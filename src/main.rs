#[macro_use] extern crate nickel;
#[macro_use] extern crate lazy_static;

extern crate rustc_serialize;
extern crate multipart;

use nickel::{Nickel, Request, Response, MiddlewareResult, HttpRouter, StaticFilesHandler, QueryString, MediaType};
use nickel::extensions::Redirect;

use std::fs::File;
use std::path::Path;
use std::io::Read;
use std::thread;

use multipart::server::{Multipart, SaveResult};

use rustc_serialize::json;

mod db;
mod sync;
mod commands;

use db::Db;
use sync::save_image;

fn index_n_search<'a, D>(_request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    response.render("src/templates/index.html", &[0]) // Вот тут и ниже так надо, чтобы не пересобирать программу при изменении HTML
}

fn show<'a, D>(_request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    response.render("src/templates/show.html", &[0])
}

use std::sync::Mutex;

lazy_static! {
    pub static ref DB : Mutex<Db> = Mutex::new(Db::new());
}

fn upload_image<'mw>(req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    if let Ok(mut multipart) = Multipart::from_request(req) {
            match multipart.save_all() {
                SaveResult::Full(entries) | SaveResult::Partial(entries, _)  => {
                    if let Some(savedfile) = entries.files.get("image") {
                        if let Some(ref filename) = savedfile.filename {
                            if let Some(tags) = entries.fields.get("tags") {
                                let tags = tags.split_whitespace().map(String::from).collect::<Vec<_>>();
                                let mut body = Vec::new();
                                let _ = File::open(&savedfile.path).unwrap().read_to_end(&mut body);
                                let name = DB.lock().unwrap().add_with_tags_name(&tags, filename.split('.').collect::<Vec<_>>()[1]).unwrap();

                                save_image(Path::new("assets/images"), &name, &body);

                                res.redirect("/")

                            } else { res.send("No tags found") }
                        } else { res.send("Can't get filename") }
                    } else { res.send("Can't load file") }
                },

                SaveResult::Error(e) =>  res.send(format!("Server could not handle multipart POST! {:?}", e))
            }
    } else {
        res.set(nickel::status::StatusCode::BadRequest);
        res.send("Not a multipart request")
    }
}

fn get_image<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let id = request.param("id").unwrap().parse::<i32>().unwrap();
    response.set(MediaType::Json);
    response.send(json::encode(&DB.lock().unwrap().get_image(id).unwrap()).unwrap())
}

fn more<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let offset = request.query().get("offset").unwrap().parse::<usize>().unwrap();

    let images = match request.query().get("q") {
        Some(x) =>  DB.lock().unwrap().by_tags(25, offset, &x.to_lowercase().split_whitespace().map(String::from).collect::<Vec<_>>()).unwrap(),
        None    =>  DB.lock().unwrap().get_images(25, offset).unwrap()
    };

    response.set(MediaType::Json);
    response.send(json::encode(&images).unwrap())
}

#[derive(RustcEncodable)]
struct UserStatus {
    logined: bool,
    name: Option<String>
}

fn user_status<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    response.set(MediaType::Json);

    response.send(json::encode(&UserStatus{
        logined: false,
        name: None
    }).unwrap())
}

macro_rules! routes(
    { $serv:ident, $($method:ident $($path:expr),+ => $fun:ident),+ } => {
        {
            $($(
                    $serv.$method($path, $fun);
               )+)+
        }
     };
);

fn main() {
    let mut server = Nickel::new();

    server.utilize(StaticFilesHandler::new("assets"));

    routes!{server,
        get "/","/search" => index_n_search,
        get "/show/:id" => show,
        get "/more" => more,
        get "/get_image/:id" => get_image,
        get "/user_status" => user_status,

        post "/upload_image" => upload_image
    };

    thread::spawn(commands::main);

    let _server = server.listen("127.0.0.1:3000");
}
