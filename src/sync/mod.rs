extern crate hyper;
extern crate image;

use self::hyper::client::Client;
use self::hyper::header::UserAgent;

use std::io::{Read,Write};
use std::fs::{File,OpenOptions,read_dir,create_dir};
use std::path::Path;

pub use super::DB;

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
pub mod konachan;

/// Сохраняет картинку & создаёт к ней превью
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

    f.write(file).unwrap();
    prev.save(&mut prevf, image::JPEG).unwrap();
}

fn process_downloads(client: &Client, images: &[Image], recv: &Receiver<()>) -> Result<(),()> {
    let images_c = DB.get_images(None,0).unwrap();
    let mut outf = OpenOptions::new().append(true).create(true).open("OUTPUT").unwrap();

    for im in images {
        match recv.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                return Err(());
            }
            Err(TryRecvError::Empty) => {}
        }

        if !images_c.iter().any(|x| x.name == im.name ) {
            if im.got_from == "konachan" || im.got_from == "danbooru" {
                download(&Client::new(), im)?;
            } else {
                download(client, im)?;
            }
        } else {
            let mut m_tags = images_c.iter().find(|x| x.name == im.name ).unwrap().tags.clone();
            let mut curr_tags = im.tags.clone();
            m_tags.dedup();
            curr_tags.dedup();
            if m_tags != curr_tags {
                DB.add_image(&im.name, &im.tags, im.got_from.as_str(), im.post_url.as_str(), im.rating).unwrap();
                writeln!(&mut outf, "UPDATE tags on {}", im.name).unwrap();
            }
        }
    }
    Ok(())
}

/// Качает картинку, прерываясь, если из консоли поступил kill
fn download(client: &Client, im: &Image) -> Result<(),()> {
    let mut outf = OpenOptions::new().append(true).create(true).open("OUTPUT").unwrap();

    let mut res = client.get(&im.url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send().unwrap();
    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();

    DB.add_image(&im.name, &im.tags, im.got_from.as_str(), im.post_url.as_str(), im.rating).unwrap();

    save_image(Path::new("assets/images"), &im.name, &body);

    writeln!(&mut outf, "DONE {}", im.name).unwrap();

    Ok(())
}

/// Запросить и распарсить JSON
fn req_and_parse(client: &Client, url: &str) -> Result<Json, hyper::Error> {
    let mut outf = OpenOptions::new().append(true).create(true).open("OUTPUT").unwrap();
    let mut res = match client.get(url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send() {
            Ok(x)   => x,
            Err(x)  => {
                writeln!(&mut outf, "ERROR: {}", x).unwrap();
                return Err(x)
            }
        };

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    Ok(Json::from_str(&body).unwrap())
}
