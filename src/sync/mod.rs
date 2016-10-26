extern crate hyper;
extern crate rustc_serialize;
extern crate log;
extern crate ansi_term;

use self::ansi_term::Color::Green;

use self::hyper::client::Client;
use self::hyper::header::UserAgent;

use std::io::{Read,Write};
use std::fs::{File,read_dir,create_dir};
use std::path::Path;

use ::db::Db;
use ::logger::ZephLogger;

use rustc_serialize::json::Json;

#[derive(Debug)]
pub struct Image {
    name: String,
    tags: Vec<String>,
    got_from: String,
    url: String,
    rating: char,
    post_url: String
}

mod e621;
mod derpy;

fn download(client: &Client, im: &Image) {
    let mut res = client.get(&im.url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send().unwrap();
    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();

    let db = Db::new();
    db.add_image(&im.name, &im.tags, im.got_from.as_str(), im.post_url.as_str(), im.rating).unwrap();

    if let Err(_) = read_dir("assets/images") {
        create_dir("assets/images").unwrap();
    }

    let mut f = File::create(Path::new(&format!("assets/images/{}", im.name))).unwrap();
    f.write(&body).unwrap();
    info!(r"{} {}", im.name, Green.paint("done"));
}

fn req_and_parse(client: &Client, url: &str) -> Result<Json, hyper::Error> {
    let mut res = match client.get(url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send() {
            Ok(x)   => x,
            Err(x)  => {
                error!("{}", x);
                return Err(x)
            }
        };

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    Ok(Json::from_str(&body).unwrap())
}

pub fn main() {
    ZephLogger::init().unwrap();
    //e621::main();
    derpy::main();
}
