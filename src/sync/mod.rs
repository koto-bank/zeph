extern crate hyper;
extern crate rustc_serialize;
extern crate log;
extern crate ansi_term;

use self::ansi_term::Color::Green;

use self::hyper::client::Client;
use self::hyper::header::UserAgent;

use rustc_serialize::json::Json;

use std::thread;
use std::time::Duration;
use std::io::{Read,Write};
use std::fs::{File,read_dir,create_dir};
use std::path::Path;

use ::db::Db;
use ::logger::ZephLogger;

#[derive(Debug)]
struct Image {
    url: String,
    tags: Vec<String>,
    id: i64,
    rating: char
}

fn download(client: &Client, im: &Image) {

    let name = im.url.split('/').collect::<Vec<_>>();
    let name = name[name.len()-1];

    let mut res = client.get(&im.url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send().unwrap();
    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();

    let db = Db::new();
    db.add_image(name, &im.tags, "e621", &*format!("https://e621.net/post/show/{}", im.id), im.rating).unwrap();

    if let Err(_) = read_dir("assets/images") {
        create_dir("assets/images").unwrap();
    }

    let mut f = File::create(Path::new(&format!("assets/images/{}", name))).unwrap();
    f.write(&body).unwrap();
    info!(r"{} {}", name, Green.paint("done"));
}

pub fn e621() {
    ZephLogger::init().unwrap();

    let db = Db::new();
    let client = Client::new();
    let images_c = db.get_images(None,0).unwrap();
    let mut url_string = "https://e621.net/post/index.json".to_string();

    loop {
        let mut res = match client.get(&url_string)
            .header(UserAgent("Zeph/1.0".to_owned()))
            .send() {
                Ok(x)   => x,
                Err(x)  => {
                    error!("{}",x);
                    thread::sleep(Duration::new(5,0));
                    continue
                }
            };

        let mut body = String::new();
        res.read_to_string(&mut body).unwrap();

        let body = Json::from_str(&body).unwrap();
        let images = body.as_array().unwrap();
        if images.is_empty() { break }

        let before_id = images[images.len()-1]
            .as_object()
            .map(|x| &x["id"])
            .and_then(|x| x.as_u64())
            .unwrap();

        let images = images.iter().fold(Vec::new(), |mut acc, x| {
            let image = x.as_object().unwrap();
            let tags = image["tags"].as_string().unwrap().split_whitespace().map(|x| x.to_string()).collect::<Vec<_>>();
            let rating = image["rating"].as_string().unwrap().chars().nth(0).unwrap();

            let ext = image["file_ext"].as_string().unwrap();
            if ext != "webm" && ext != "swf" && ext != "mp4" {
                acc.push(Image{
                    url: image["file_url"].as_string().unwrap().to_string(),
                    tags: tags,
                    id: image["id"].as_i64().unwrap(),
                    rating: rating
                });
                acc
            } else {
                acc
            }
        });

        for im in images {
            let name = im.url.split('/').collect::<Vec<_>>();
            let name = name[name.len()-1];

            let mut was = false;
            for im_c in &images_c {
                if name == im_c.name {
                    was = true;
                }
            }

            if !was {
                download(&client, &im);
            }
        }
        url_string = format!("https://e621.net/post/index.json?before_id={}", before_id);
    }
}
