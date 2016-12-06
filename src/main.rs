#![feature(plugin)]
#![plugin(maud_macros)]

#[macro_use] extern crate router;
#[macro_use] extern crate lazy_static;

extern crate maud;
extern crate rustc_serialize;
extern crate hyper;
extern crate time;
extern crate multipart;
extern crate toml;

extern crate iron;
extern crate staticfile;
extern crate mount;
extern crate urlencoded;
extern crate iron_sessionstorage as session;

use iron::prelude::*;
use iron::status;
use iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};
use iron::modifiers::RedirectRaw as Redirect;

use multipart::server::{Multipart, SaveResult};

use staticfile::Static;
use mount::Mount;
use urlencoded::{UrlEncodedQuery,UrlEncodedBody};
use router::Router;
use session::{SessionStorage,SessionRequestExt,Value as SessionValue};
use session::backends::SignedCookieBackend;

use std::path::Path;
use std::fs::{File,remove_file};
use std::io::Read;
use std::sync::Mutex;

use rustc_serialize::json;

pub use toml::{Table,Parser};

// Macros

#[macro_export]
macro_rules! config{
    {? $name:expr} => {
        CONFIG.get($name).and_then(|x| x.as_str())
    };

    {$name:expr} => {
        CONFIG[$name].as_str().unwrap()
    };
}

macro_rules! query{
    ($q:ident, $name:expr) => {
        $q.get($name).unwrap_or(&Vec::new()).get(0)
    }
}

// Modules

mod db;
mod sync;
mod utils;

use db::{Db,VoteImageError};
use utils::{save_image,open_config,exec_command};

lazy_static! {
    pub static ref DB : Mutex<Db> = Mutex::new(Db::new());
    pub static ref CONFIG : Table = open_config();
    /// Used in utils
    pub static ref LOG : Mutex<Vec<String>> = Mutex::new(Vec::new());
}

struct Login(String);
impl SessionValue for Login {
    fn get_key() -> &'static str { "username" }
    fn into_raw(self) -> String { self.0 }
    fn from_raw(value: String) -> Option<Self> {
        if value.is_empty() {
            None
        } else {
            Some(Login(value))
        }
    }
}

fn index_n_search(_req: &mut Request) -> IronResult<Response> {
    let page = html! {
        meta charset="utf-8" /
        link rel="stylesheet" href="/assets/css/milligram.min.css" /
        link rel="stylesheet" href="/assets/css/main.css" /
        link rel="icon" type="image/jpeg" href="/assets/favicon.jpg" /
        title "Zeph"
        script src="/assets/js/main.js" {}

        div style="width:100%;" {
            div.tags-search {
                a href="/" title="Boop!" {
                    img#nano-logo src="/assets/logo.jpg"
                    h3 style="display: inline; vertical-align: 50%" "Zeph"
                }
                form#tag-search-form action="/search" {
                    input#tag-search-field placeholder="Search" name="q" type="text" /
                }
                div#tags {} // Tags w/ JS
                a href="/about" style="opacity: 0.5;" "About Zeph & Help"
            }
            div#images {} // Pics w/ JS
            button#upload-button onclick="showUploadOrLogin()" "Login"
            div#login-or-upload-form / // Form w/ JS
        }
    };
    Ok(Response::with((status::Ok, page)))
}

fn more(req: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();

    let q = match req.get_ref::<UrlEncodedQuery>() {
        Ok(hashmap) => hashmap,
        Err(_) => return Ok(Response::with((status::BadRequest, "No parameters")))
    };

    let offset = query!(q,"offset").unwrap_or(&"0".to_string()).parse::<usize>().unwrap();
    let images = match query!(q,"q") {
        Some(x) =>  DB.lock().unwrap().by_tags(25, offset, &x.to_lowercase().split_whitespace().map(String::from).collect::<Vec<_>>()).unwrap(),
        None    =>  DB.lock().unwrap().get_images(25, offset).unwrap()
    };

    response
        .set_mut(Mime(TopLevel::Application, SubLevel::Json,
                      vec![(Attr::Charset, Value::Utf8)]))
        .set_mut(json::encode(&images).unwrap())
        .set_mut(status::Ok);
    Ok(response)
}

fn show(req: &mut Request) -> IronResult<Response> {
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


fn delete(req: &mut Request) -> IronResult<Response> {
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

fn user_status(req: &mut Request) -> IronResult<Response> {
    #[derive(RustcEncodable)]
    struct UserStatus {
        logined: bool,
        name: Option<String>
    }

    let (logined,name) = match req.session().get::<Login>()? {
        Some(user)  => (true, Some(user.0)),
        None        => (false, None)
    };

    let mut response = Response::new();

    response
        .set_mut(Mime(TopLevel::Application, SubLevel::Json,
                      vec![(Attr::Charset, Value::Utf8)]))
        .set_mut(json::encode(&UserStatus{logined: logined,name: name}).unwrap())
        .set_mut(status::Ok);
    Ok(response)
}

fn login(req: &mut Request) -> IronResult<Response> {
    let mut response = Response::new();

    let body = match req.get::<UrlEncodedBody>() {
        Ok(data) => data,
        Err(_)  => return Ok(Response::with(status::BadRequest))
    };

    if let (Some(login), Some(pass)) = (body.get("login"),body.get("password")) {
        match DB.lock().unwrap().check_user(&login[0], &pass[0]).unwrap() {
            Some(x) => if x {
                req.session().set(Login(login[0].clone()))?;
                response
                    .set_mut(Redirect("/".to_string()))
                    .set_mut(status::Found);
                Ok(response)
            } else {
                Ok(Response::with((status::BadRequest,"Incorrect login/pass")))
            },
            None  => Ok(Response::with((status::Ok,"No such user")))
        }
    } else {
        Ok(Response::with((status::BadRequest,"No login/pass")))
    }
}

fn upload_image(req: &mut Request) -> IronResult<Response> {
    if let Some(username) = req.session().get::<Login>()? {
        let username = username.0;
        if let Ok(mut multipart) = Multipart::from_request(req) {
            match multipart.save_all() {
                SaveResult::Full(entries) | SaveResult::Partial(entries, _)  => {
                    if let Some(savedfile) = entries.files.get("image") {
                        if let Some(ref filename) = savedfile.filename {
                            if let Some(tags) = entries.fields.get("tags") {
                                let tags = tags.split_whitespace().map(String::from).collect::<Vec<_>>();
                                let mut body = Vec::new();
                                let _ = File::open(&savedfile.path).unwrap().read_to_end(&mut body);
                                let name = DB.lock().unwrap().add_with_tags_name(&tags, filename.split('.').collect::<Vec<_>>()[1], &username).unwrap();

                                save_image(Path::new(config!("images-directory")), &name, &body);

                                let mut response = Response::new();
                                response
                                    .set_mut(Redirect("/".to_string()))
                                    .set_mut(status::Found);
                                Ok(response)

                            } else { Ok(Response::with((status::BadRequest,"No tags found"))) }
                        } else { Ok(Response::with((status::BadRequest,"No filename"))) }
                    } else { Ok(Response::with((status::BadRequest,"Can't load file"))) }
                },

                SaveResult::Error(e) =>  Ok(Response::with((status::BadRequest,format!("Server could not handle multipart POST! {:?}", e))))
            }
        } else {
            Ok(Response::with((status::BadRequest,"Not a multipart request?")))
        }
    } else {
        Ok(Response::with((status::Forbidden,"Not logged in")))
    }
}

fn adduser(req: &mut Request) -> IronResult<Response> {

    let body = match req.get::<UrlEncodedBody>() {
        Ok(data) => data,
        Err(_)  => return Ok(Response::with(status::BadRequest))
    };

    Ok(if let (Some(login), Some(pass), Some(confirm_pass)) = (body.get("login"), body.get("password"),body.get("confirm_password")) {
        let (login,pass,confirm_pass) = (login[0].clone(), pass[0].clone(), confirm_pass[0].clone());
        if pass == confirm_pass {
            if !pass.trim().is_empty() && !login.trim().is_empty() {
                if let Ok(res) = DB.lock().unwrap().add_user(&login,&pass) {
                    if res {
                        let mut response = Response::new();
                        req.session().set(Login(login))?;
                        response
                            .set_mut(Redirect("/".to_string()))
                            .set_mut(status::Found);
                        response
                    } else {
                        Response::with((status::Ok,"User already exists"))
                    }
                } else {
                    Response::with((status::InternalServerError, "Internal server error"))
                }
            } else {
                Response::with((status::BadRequest,"Empty login/pass"))
            }
        } else {
            Response::with((status::Ok,"Password and confirmation are not equeal"))
        }
    } else {
        Response::with((status::BadRequest,"No data"))
    })
}

fn vote_image(req: &mut Request) -> IronResult<Response> {
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

fn similiar(req: &mut Request) -> IronResult<Response> {
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

fn about(_: &mut Request) -> IronResult<Response> {
    let page = html! {
                meta charset="utf-8" /
        link rel="stylesheet" href="/assets/css/milligram.min.css" /
        link rel="stylesheet" href="/assets/css/main.css" /
        link rel="icon" type="image/jpeg" href="/assets/favicon.jpg" /
        title "Zeph - About"

        div style="width:100%;" {
            div.tags-search {
                a href="/" title="Boop!" {
                    img#nano-logo src="/assets/logo.jpg" /
                    h3 style="display: inline; vertical-align: 50%" "Zeph"
                }
                form#tag-search-form action="/search" {
                    input#tag-search-field placeholder="Search" name="q" type="text" /
                }
            }
        }
        div style="margin-left: 15%;" {
            {"Zeph is an open-source booru/imageboard written in " a href="https://www.rust-lang.org/" "Rust" }
            br /
            { "You can get source code to build Zeph yourself from " a href="https://github.com/koto-bank/zeph" "Github" }
            br /
            @if let Some(addr) = CONFIG.get("contact-email") {
                { "Contact e-mail adress: " a href={"mailto:" ( addr.as_str().unwrap()) } ( addr.as_str().unwrap() ) }
            }
            br
            h3 "Search options"
            table style="width: 50%;" {
                tr {
                    th "Example"
                    th "Meaning"
                }
                tr {
                    td code "1girl"
                    td "Search for a girl on her own"
                }
                tr {
                    td code "1girl -fur"
                    td "Search for a non-fluffy girl (exclude 'fur' tag)"
                }
                tr {
                    td code "rating:s,q"
                    td "Search for a safe and questionable images"
                }
                tr {
                    td {
                        code "*girls"
                        "or"
                        code "2girl*"
                    }
                    td "Search for anything that ends with 'girls' (or starts with '2girl')"
                }
                tr {
                    td code "from:konachan"
                    td "Search for images synchronized from konachan (full list in source code & easily extendable)"
                }
                tr {
                    td code "uploader:random_dude"
                    td "Images uploaded by random_dude, note that 'sync' are synchronized images"
                }
                tr {
                    td code "sort:asc:score"
                    td "Sort images by score from worst to best (ascending); desc is for descening"
                }
                tr {
                    td code "1girl | 2girls"
                    td "Search for images of girl on her own OR 2 girls"
                }
                tr {
                    td code "1girl format:jpg,gif"
                    td "Search for GIF and JPEG images"
                }
            }
        }
    };

    Ok(Response::with((status::Ok, page)))
}

fn admin_command(req: &mut Request) -> IronResult<Response> {
    if let (Some(curr_username), Some(admin_username)) = (req.session().get::<Login>()?,config!(? "admin-username")) {
        if curr_username.0.to_lowercase() == admin_username.to_lowercase() {
            let body = match req.get::<UrlEncodedBody>() {
                Ok(data) => data,
                Err(_)  => return Ok(Response::with(status::BadRequest))
            };

            if let Some(comm) = body.get("command") {
                exec_command(&comm[0]);
            }

            Ok(Response::with(status::Ok))
        } else {
            Ok(Response::with((status::Forbidden,"Not an admin")))
        }
    } else {
        Ok(Response::with((status::Forbidden,"Not logged in"))) // .. or admin account is not set
    }
}

fn admin(req: &mut Request) -> IronResult<Response> {
    if let (Some(curr_username), Some(admin_username)) = (req.session().get::<Login>()?,config!(? "admin-username")) {
        if curr_username.0.to_lowercase() == admin_username.to_lowercase() {
            let page = html!{
                script src="/assets/js/admin.js" {}

                div#log-block style="width:40%; height:50%; overflow-y: auto; border: 1px solid black;" {
                    @for l in LOG.lock().unwrap().iter() {
                        (l)
                    }
                }
                br /
                form#command-form onsubmit="sendCommand(this); return false;" {
                    input name="comm" nameplaceholder="Command" type="text" /
                    input#send-button value="Send" type="submit" /
                }
            };
            Ok(Response::with((status::Ok,page)))
        } else {
            Ok(Response::with((status::Forbidden,"Not an admin")))
        }
    } else {
        Ok(Response::with((status::Forbidden,"Not logged in"))) // .. or admin account is not set
    }
}

fn get_log(req: &mut Request) -> IronResult<Response> {
    if let (Some(curr_username), Some(admin_username)) = (req.session().get::<Login>()?,config!(? "admin-username")) {
        if curr_username.0.to_lowercase() == admin_username.to_lowercase() {
            let mut response = Response::new();
            response
                .set_mut(Mime(TopLevel::Application, SubLevel::Json,
                              vec![(Attr::Charset, Value::Utf8)]))
                .set_mut(json::encode(&*LOG.lock().unwrap()).unwrap())
                .set_mut(status::Ok);

            Ok(response)
        } else {
            Ok(Response::with((status::Forbidden,"Not an admin")))
        }
    } else {
        Ok(Response::with((status::Forbidden,"Not logged in"))) // .. or admin account is not set
    }
}

fn main() {
    let router = router!(index:     get "/" => index_n_search,
                         more:      get "/more" => more,
                         search:    get "/search" => index_n_search,
                         user_stat: get "/user_status" => user_status,
                         vote:      get "/vote_image" => vote_image,
                         about:     get "/about" => about,
                         admin:     get "/admin" => admin,
                         get_log:   get "/log"  => get_log,

                         show:      get "/show/:id" => show,
                         delete:    get "/delete/:id" => delete,
                         similiar:  get "/similiar" => similiar,

                         adm_comm:  post "/admin" => admin_command,
                         login:     post "/login" => login,
                         upload_im: post "/upload_image" => upload_image,
                         adduser:   post "/adduser" => adduser);

    let mut mount = Mount::new();
    mount.mount("/", router)
        .mount("/assets", Static::new(Path::new("assets")))
        .mount("/images", Static::new(Path::new(config!("images-directory"))));

    let mut chain = Chain::new(mount);
    chain.around(SessionStorage::new(SignedCookieBackend::new(time::now().to_timespec().sec.to_string().bytes().collect::<Vec<_>>())));

    Iron::new(chain).http("127.0.0.1:3000").unwrap();
}
