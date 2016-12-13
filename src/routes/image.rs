//! Routes to work with individual image (and `/show` page)

use {DB,CONFIG,save_image,json,VoteImageError};

use std::fs::{File,remove_file};
use std::io::Read;
use std::path::Path;

use iron::prelude::*;
use iron::status;
use iron::modifiers::RedirectRaw as Redirect;
use iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};


use urlencoded::UrlEncodedQuery;
use router::Router;
use session::SessionRequestExt;

use multipart::server::{Multipart, SaveResult};

use Login;

/// Show an image
pub fn show(req: &mut Request) -> IronResult<Response> {
    let id = req.extensions.get::<Router>().and_then(|x| x.find("id")).and_then(|x| x.parse::<i32>().ok()).unwrap();
    let image = match DB.lock().unwrap().get_image(id).unwrap() {
        Some(x) => x,
        None    => return Ok(Response::with(status::NotFound))
    };

    let page = html!{
        meta charset="utf-8" /
        link rel="stylesheet" href="/assets/css/milligram.min.css" /
        link rel="stylesheet" href="/assets/css/main.css" /
        link rel="icon" type="image/jpeg" href="/assets/favicon.jpg" /
        script src="/assets/js/show.js" {}
        title { "Zeph - " (image.tags.join(" ")) }
        meta property="og:title" content="Zeph" /
        meta property="og:description" content=(image.tags.join(" ")) /
        meta property="og:url" content={ "https://zeph.kotobank.ch/show/" (image.id) } /
        meta property="og:image" content={"https://zeph.kotobank.ch/images/preview/" (image.name)} /

        div style="width:100%;" {
            div.tags-search {
                a href="/" title="Boop!" {
                    img#nano-logo src="/assets/logo.jpg" /
                    h3 style="display: inline; vertical-align: 50%" "Zeph"
                }
                form#tag-search-form action="/search" {
                    input#tag-search-field placeholder="Search" name="q" type="text" /
                }
                div#id { "#" (image.id) }
                div#tags {
                    div#image-info {
                        @if let Some(original_link) = image.original_link {
                            a#original-link href=(original_link) "Original page" br /
                        }
                        @if let Some(rating) = image.rating {
                            a#rating href={ "/search?q=rating:" (rating) } value=(rating) { "rating:" (rating) } br /
                        }
                        @if let Some(got_from) = image.got_from {
                            a#got_from href={ "/search?q=from:" (got_from) } value=(got_from) { "from:" (got_from) } br /
                        }
                        @if let Some(uploader) = image.uploader {
                            a#uploader href={ "/search?q=uploader:" (uploader) } value=(uploader) { "uploader:" (uploader) } br /
                        }
                        div#vote-area div#score value=(image.score) { "Score: " (image.score) } br /
                        @for tag in image.tags {
                            a href={ "/search?q=" (tag) } { (tag) } br /
                        }
                    }
                }
                a href="/about" style="opacity: 0.5;" "About Zeph & Help"
            }
            div style="margin-left: 15%;" {
                a href={ "/images/" (image.name) } {
                    img#image-block style="display: block; margin: 0 auto;" src={ "/images/" (image.name) } /
                }
                h4 style="margin-top: 2%;" { "Similiar images" } br /
                div#similiar {} // Similiar w/ JS
            }
        }
    };

    Ok(Response::with((status::Ok, page)))
}

/// Remove an image
pub fn delete(req: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();

    let id = req.extensions.get::<Router>().and_then(|x| x.find("id")).and_then(|x| x.parse::<i32>().ok()).unwrap();
    let image = match DB.lock().unwrap().get_image(id).unwrap() {
        Some(image) => image,
        None    => return Ok(Response::with(status::NotFound))
    };

    Ok(match req.session().get::<Login>()? {
        Some(username) => if Some(username.0) == image.uploader {
            let name = DB.lock().unwrap().delete_image(id).unwrap();
            remove_file(format!("{}/{}", config!("images-directory"), name)).unwrap();
            remove_file(format!("{}/preview/{}", config!("images-directory"), name)).unwrap();
            response
                .set_mut(Redirect("/".to_string()))
                .set_mut(status::Found);
            response
        } else {
            Response::with((status::Forbidden,"You are not an uploader of this picture"))
        },
        None    => Response::with((status::Forbidden,"Not logged in"))
    })
}

/// Upload an image w/ multipart/form-data
pub fn upload_image(req: &mut Request) -> IronResult<Response> {
    let username = match req.session().get::<Login>()? {
        Some(u) => u.0,
        None    => return Ok(Response::with((status::Forbidden,"Not logged in")))
    };
    let mut multipart = match Multipart::from_request(req) {
        Ok(m)   => m,
        Err(e)  => return Ok(Response::with((status::BadRequest, format!("Not a multipart request? {:#?}", e))))
    };

    match multipart.save_all() {
        SaveResult::Full(entries) | SaveResult::Partial(entries, _)  => {
            let savedfile = match entries.files.get("image") {
                Some(s) => s,
                None    => return Ok(Response::with((status::BadRequest,"Can't load file")))
            };
            let filename = match savedfile.filename {
                Some(ref f) => f,
                None    => return Ok(Response::with((status::BadRequest,"No filename"))) // Is this even possible?
            };
            let tags = match entries.fields.get("tags") {
                Some(t) => t.split_whitespace().map(String::from).collect::<Vec<_>>(),
                None    => return Ok(Response::with((status::BadRequest,"No tags found")))
            };

            let mut body = Vec::new();
            let _ = File::open(&savedfile.path).unwrap().read_to_end(&mut body);
            let name = DB.lock().unwrap().add_with_tags_name(&tags, filename.split('.').collect::<Vec<_>>()[1], &username).unwrap();

            save_image(Path::new(config!("images-directory")), &name, &body);

            let mut response = Response::new();
            response
                .set_mut(Redirect("/".to_string()))
                .set_mut(status::Found);
            Ok(response)
        },

        SaveResult::Error(e) =>  Ok(Response::with((status::BadRequest,format!("Server could not handle multipart POST! {:?}", e))))
    }
}

// Vota an image
pub fn vote_image(req: &mut Request) -> IronResult<Response> {
    let q = match req.get::<UrlEncodedQuery>() {
        Ok(hashmap) => hashmap,
        Err(_) => return Ok(Response::with((status::BadRequest, "No parameters")))
    };

    Ok(if let (Some(id), Some(vote)) = (query!(q,"id"),query!(q,"vote")) {
        if let Some(name) = req.session().get::<Login>()? {
            let name = name.0;
            if let (Ok(vote),Ok(id)) = (vote.parse::<bool>(),id.parse::<i32>()) {
                match DB.lock().unwrap().vote_image(&name, id, vote).unwrap() {
                    Ok(newv)                        => Response::with((status::Ok,newv.to_string())),
                    Err(VoteImageError::Already)    => Response::with((status::Ok,"Already voted that")),
                    Err(VoteImageError::NoImage)    => Response::with((status::Ok,"No such image"))
                }
            } else {
                Response::with((status::BadRequest,"Invalid data"))
            }
        } else {
            Response::with((status::Forbidden,"Not logged in"))
        }
    } else {
        Response::with((status::BadRequest,"No data"))
    })
}

/// Find similiar images by tags
pub fn similiar(req: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();

    let q = match req.get_ref::<UrlEncodedQuery>() {
        Ok(hashmap) => hashmap,
        Err(_) => return Ok(Response::with((status::BadRequest, "No parameters")))
    };

    let offset = query!(q,"offset").unwrap_or(&"0".to_string()).parse::<usize>().unwrap();
    let id = query!(q,"id").unwrap().parse::<i32>().unwrap();
    let images = DB.lock().unwrap().similiar(id, 25, offset).unwrap();

    response
        .set_mut(Mime(TopLevel::Application, SubLevel::Json,
                      vec![(Attr::Charset, Value::Utf8)]))
        .set_mut(json::encode(&images).unwrap())
        .set_mut(status::Ok);
    Ok(response)
}
