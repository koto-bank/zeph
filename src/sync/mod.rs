extern crate hyper;
extern crate rustc_serialize;
extern crate log;
extern crate ansi_term;

use self::ansi_term::Color::{Green,Red};

use self::hyper::client::Client;
use self::hyper::header::UserAgent;

use rustc_serialize::json::Json;

use std::thread;
use std::time::Duration;
use std::io::{Read,Write};
use std::fs::{File,read_dir,create_dir};
use std::path::Path;

use ::db::Db;

use log::{LogRecord, LogLevel, LogMetadata, LogLevelFilter, SetLoggerError};

struct SyncLogger;

impl ::log::Log for SyncLogger {
    fn enabled(&self, _: &LogMetadata) -> bool { true }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            match record.level() {
                LogLevel::Info  => println!(r"[{}] {}", Green.paint("INFO"), record.args()),
                LogLevel::Error => println!(r"[{}] {}", Red.paint("ERROR"), record.args()),
                _               => println!(r"[{}] {}", record.level(), record.args())
            }
        }
    }
}

impl SyncLogger {
    fn init() -> Result<(), SetLoggerError> {
        log::set_logger(|max_log_level| {
            max_log_level.set(LogLevelFilter::Info);
            Box::new(SyncLogger)
        })
    }
}

#[derive(Debug)]
struct Image {
    url: String,
    tags: Vec<String>
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
    db.add_image_with_name(name, &im.tags).unwrap();

    if let Err(_) = read_dir("assets/images") {
        create_dir("assets/images").unwrap();
    }

    let mut f = File::create(Path::new(&format!("assets/images/{}", name))).unwrap();
    f.write(&body).unwrap();
    info!(r"{} {}", name, Green.paint("done"));
}

pub fn e621() {
    SyncLogger::init().unwrap();

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
            let mut tags = image["tags"].as_string().unwrap().split_whitespace().map(|x| x.to_string()).collect::<Vec<_>>();
            let rating = format!("rating:{}",image["rating"].as_string().unwrap());

            tags.push("e621".to_string());
            tags.push(rating);

            acc.push(Image{
                url: image["file_url"].as_string().unwrap().to_string(),
                tags: tags,
            });
            acc
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
