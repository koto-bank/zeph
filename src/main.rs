#![feature(plugin)]
#![plugin(maud_macros)]

#[macro_use] extern crate router;
#[macro_use] extern crate lazy_static;

extern crate maud;
extern crate rustc_serialize;
extern crate hyper;

extern crate iron;
extern crate staticfile;
extern crate mount;
extern crate urlencoded;

use rustc_serialize::json;

use iron::prelude::*;
use iron::status;
use hyper::header::ContentType;
use iron::mime::{Mime, TopLevel, SubLevel, Attr, Value};

use staticfile::Static;
use mount::Mount;
use urlencoded::UrlEncodedQuery;

use std::path::Path;
use std::fs::{File,OpenOptions};

mod db;
mod sync;
mod commands;
mod utils;

use db::{Db,VoteImageError};
use utils::save_image;

use std::sync::Mutex;
use std::cell::RefCell;

lazy_static! {
    pub static ref DB : Mutex<Db> = Mutex::new(Db::new());

    /// Использование лежит в utils
    pub static ref OUTF : Mutex<RefCell<File>> = Mutex::new(RefCell::new(OpenOptions::new().append(true).create(true).open("OUTPUT").unwrap()));
}

macro_rules! query{
    {$q:ident, $name:expr} => {
        $q.get($name).unwrap_or(&Vec::new()).get(0)
    }
}

fn index_n_search(req: &mut Request) -> IronResult<Response> {
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
                div#tags {} // Тэги через JS
            }
            div#images {} // Картинки через JS
            button#more-button onclick="loadMore()" "More"
            button#upload-button onclick="showUploadOrLogin()" "Login"
            div#login-or-upload-form / // Форма через JS
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

fn main() {
    let router = router!(index: get "/" => index_n_search,
                         more: get "/more" => more,
                         search: get "/search" => index_n_search);

    let mut mount = Mount::new();
    mount.mount("/", router)
        .mount("/assets", Static::new(Path::new("assets")))
        .mount("/images", Static::new(Path::new("assets/images")));


    Iron::new(mount).http("localhost:3000").unwrap();
}
