//! Routes that don't really belong to anything else

use {DB,CONFIG};

use iron::prelude::*;
use iron::status;
use iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};

use urlencoded::UrlEncodedQuery;

use serde_json::to_value;

pub mod image;
pub mod user;
pub mod admin;

pub use image::*;
pub use user::*;
pub use admin::*;

/// Main page & search
pub fn index_n_search(_req: &mut Request) -> IronResult<Response> {
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

/// Load more pictures, used from JS
pub fn more(req: &mut Request) -> IronResult<Response> {
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
        .set_mut(to_value(&images).to_string())
        .set_mut(status::Ok);
    Ok(response)
}

/// `/about` page
pub fn about(_: &mut Request) -> IronResult<Response> {
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
