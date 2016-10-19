extern crate lmdb_rs;
#[macro_use] extern crate nickel;
#[macro_use] extern crate log;
extern crate rustc_serialize;
extern crate multipart;

use nickel::{Nickel, Request, Response, MiddlewareResult, HttpRouter, StaticFilesHandler, QueryString, MediaType};
use nickel::extensions::Redirect;

use std::fs::{copy,read_dir,create_dir};
use std::path::Path;

use multipart::server::{Multipart, SaveResult};

use std::collections::HashMap;

use rustc_serialize::json;

mod db;
mod logger;
mod sync;

use db::Db;
use logger::ZephLogger;

fn index_n_search<'a, D>(_request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    response.send(include_str!("templates/index.html"))
}

fn upload_image<'mw>(req: &mut Request, mut res: Response<'mw>) -> MiddlewareResult<'mw> {
    if let Ok(mut multipart) = Multipart::from_request(req) {
            match multipart.save_all() {
                SaveResult::Full(entries) | SaveResult::Partial(entries, _)  => {
                    if let Some(savedfile) = entries.files.get("image") {
                        if let Some(ref filename) = savedfile.filename {
                            if let Some(tags) = entries.fields.get("tags") {
                                let db = Db::new();
                                let tags = tags.split_whitespace().map(|x| x.to_string()).collect::<Vec<_>>();
                                let ext = Path::new(&filename).extension().unwrap().to_str().unwrap();

                                if let Err(_) = read_dir("assets/images") {
                                    error!("Images directory does not exist, creating..");
                                    create_dir("assets/images").unwrap();
                                }

                                let name = db.add_image(&tags,ext).unwrap();
                                match copy(&savedfile.path,format!("assets/images/{}",name)) {
                                    Ok(_)   => info!("Saved {}", name),
                                    Err(x)  => error!("Can't save image: {}", x)
                                }

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

fn show<'a, D>(request: &mut Request<D>, response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let name = request.param("name").unwrap().replace("_OPENQ_","(").replace("_CLOSEQ_",")");
    let ext = request.param("ext").unwrap();

    info!("Showing {}.{}", name, ext);

    let name = format!("{}.{}",name, ext);
    let mut data = HashMap::new();
    let db = Db::new();
    data.insert("image", db.get_image(&name).unwrap());
    response.render("src/templates/show.html", &data)
}

fn more<'a, D>(request: &mut Request<D>, mut response: Response<'a, D>) -> MiddlewareResult<'a, D> {
    let db = Db::new();
    let offset = request.query().get("offset").unwrap().parse::<usize>().unwrap();

    info!("Requested more with offset {}", offset);

    let images = match request.query().get("q") {
        Some(x) =>  db.by_tags(25, offset, &x.to_lowercase().split_whitespace().map(|x| x.to_string()).collect::<Vec<_>>()).unwrap(),
        None    =>  db.get_images(25, offset).unwrap()
    };

    response.set(MediaType::Json);
    response.send(json::encode(&images).unwrap())
}

fn main() {
    if std::env::args().any(|x| x == "log") {
        ZephLogger::init().unwrap();
    }

    if std::env::args().any(|x| x == "sync") {
        sync::e621();
    }

    let mut server = Nickel::new();

    server.utilize(StaticFilesHandler::new("assets"));
    server.get("/", index_n_search);
    server.get("/search", index_n_search);
    server.get("/show/:name.:ext", show);
    server.get("/more", more);

    server.post("/upload_image", upload_image);

    let _server = server.listen("127.0.0.1:3000");
}
