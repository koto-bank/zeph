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

use staticfile::Static;
use mount::Mount;
use session::{SessionStorage,Value as SessionValue};
use session::backends::SignedCookieBackend;

use std::path::Path;
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
mod routes;

use db::{Db,VoteImageError};
use utils::{save_image,open_config,exec_command};
use routes::*;

lazy_static! {
    pub static ref DB : Mutex<Db> = Mutex::new(Db::new());
    pub static ref CONFIG : Table = open_config();
    /// Used in utils
    pub static ref LOG : Mutex<Vec<String>> = Mutex::new(Vec::new());
}

/// Structure to get login from session
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
