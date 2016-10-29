extern crate hyper;
extern crate rustc_serialize;
extern crate ansi_term;
extern crate image;

use self::ansi_term::Color::{Green, Red};

use self::hyper::client::Client;
use self::hyper::header::UserAgent;

use std::io::{Read,Write};
use std::fs::{File,read_dir,create_dir};
use std::path::Path;
use std::fmt::Display;

use ::db::Db;

use rustc_serialize::json::Json;

use std::sync::mpsc::{Receiver,TryRecvError};

use self::image::FilterType;

#[derive(Debug)]
pub struct Image {
    name: String,
    tags: Vec<String>,
    got_from: String,
    url: String,
    rating: char,
    post_url: String
}

pub mod e621;
pub mod derpy;
pub mod danbooru;

fn print_success<T: Display>(name: &T) {
    println!("{} {}", name, Green.paint("done"));
}

fn print_err<T: Display>(err: &T) {
    println!("{}: {}", Red.paint("ERROR"), err);
}

pub fn save_image(dir: &Path, name: &str, file: &[u8]) {
    if let Err(_) = read_dir("assets/images") {
        create_dir("assets/images").unwrap();
    }
    if let Err(_) = read_dir("assets/images/preview") {
        create_dir("assets/images/preview").unwrap();
    }

    let prev = image::load_from_memory(file).unwrap().resize(500, 500, FilterType::Nearest);

    let mut f = File::create(dir.join(name)).unwrap();
    let mut prevf = File::create(dir.join("preview").join(name)).unwrap();

    f.write(&file).unwrap();
    prev.save(&mut prevf, image::JPEG).unwrap();
}

fn download(client: &Client, im: &Image, recv: &Receiver<()>) -> Result<(),()> {
    match recv.try_recv() {
        Ok(_) | Err(TryRecvError::Disconnected) => {
            return Err(());
        }
        Err(TryRecvError::Empty) => {}
    }

    let mut res = client.get(&im.url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send().unwrap();
    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();

    let db = Db::new();
    db.add_image(&im.name, &im.tags, im.got_from.as_str(), im.post_url.as_str(), im.rating).unwrap();

    save_image(&Path::new("assets/images"), &im.name, &body);

    print_success(&im.name);

    Ok(())
}

fn req_and_parse(client: &Client, url: &str) -> Result<Json, hyper::Error> {
    let mut res = match client.get(url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send() {
            Ok(x)   => x,
            Err(x)  => {
                print_err(&x);
                return Err(x)
            }
        };

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    Ok(Json::from_str(&body).unwrap())
}
