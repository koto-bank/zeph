extern crate hyper;
extern crate image;

use self::hyper::client::Client;
use self::hyper::header::UserAgent;

use std::io::{Read,Write};
use std::fs::{File,read_dir,create_dir};
use std::path::Path;
use std::fmt::Display;

pub use super::{DB,OUTF};
use super::db::ImageBuilder;

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
    post_url: String,
    score: i32
}

pub mod e621;
pub mod derpy;
pub mod danbooru;
pub mod konachan;

pub fn log<T: Display>(s: T) {
    let outf = OUTF.lock().unwrap(); 
    let mut outf = outf.borrow_mut();
    writeln!(outf, "{}", s).unwrap();
}

/// Сохраняет картинку & создаёт к ней превью
pub fn save_image(dir: &Path, name: &str, file: &[u8]) {
    if read_dir("assets/images").is_err() { create_dir("assets/images").unwrap(); }
    if read_dir("assets/images/preview").is_err() { create_dir("assets/images/preview").unwrap(); }

    let prev = match image::load_from_memory(file) {
        Ok(x) => x.resize(500, 500, FilterType::Nearest),
        Err(x)  => {
            log(x);
            return
        }
    };

    let mut f = File::create(dir.join(name)).unwrap();
    let mut prevf = File::create(dir.join("preview").join(name)).unwrap();

    f.write(file).unwrap();
    prev.save(&mut prevf, image::JPEG).unwrap();
}

fn arr_eq<T: PartialEq>(first: &mut Vec<T>, second: &mut Vec<T>) -> bool {
    first.dedup();
    second.dedup();
    first == second
}


// Включает ли второй массив первый
fn includes<T: PartialEq>(first: &[T], second: &[T]) -> bool {
    let r = first.len();
    let mut c = 0;
    for f in first {
        if second.iter().any(|x| x == f) {
            c += 1;
        }
    }

    r == c
}

fn process_downloads(client: &Client, images: &[Image], recv: &Receiver<()>) -> Result<(),()> {
    let images_c = DB.lock().unwrap().get_images(None,0).unwrap();

    let printed = if includes(&images.iter().map(|x| x.name.clone()).collect::<Vec<_>>(), &images_c.iter().map(|x| x.name.clone()).collect::<Vec<_>>()) {
        log(format!("ALREADY DONE {} ~ {}", images.first().unwrap().name, images.last().unwrap().name));
        true
    } else { false };

    for im in images {
        match recv.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                return Err(());
            }
            Err(TryRecvError::Empty) => {}
        }

        if !images_c.iter().any(|x| x.name == im.name ) {
            if let Err(er) = if im.got_from == "konachan" || im.got_from == "danbooru" {
                download(&Client::new(), im)
            } else {
                download(client, im)
            } {
                log(format!("ERROR: {}; SKIP", er));
                continue
            } else {
                log(format!( "DONE {}", im.name));
            }
        } else {
            let m_image = images_c.iter().find(|x| x.name == im.name ).unwrap();
            let mut m_tags = m_image.tags.clone();

            if !arr_eq(&mut m_tags, &mut im.tags.clone()) || im.score != m_image.score {
                let imb = ImageBuilder::new(&im.name, &im.tags)
                    .got_from(&im.got_from)
                    .original_link(&im.post_url)
                    .uploader("sync")
                    .rating(im.rating)
                    .score(im.score)
                    .finalize();
                DB.lock().unwrap().add_image(&imb).unwrap();
                log(format!("UPDATE tags / score on {}", im.name));
            } else if !printed {
                log(format!("ALREADY DONE {}", im.name));
            }
        }
    }
    Ok(())
}

/// Качает картинку, прерываясь, если из консоли поступил kill
fn download(client: &Client, im: &Image) -> Result<(), hyper::Error> {
    let mut res = client.get(&im.url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send()?;

    let mut body = Vec::new();
    res.read_to_end(&mut body).unwrap();
    let imb = ImageBuilder::new(&im.name, &im.tags)
        .got_from(&im.got_from)
        .original_link(&im.post_url)
        .uploader("sync")
        .rating(im.rating)
        .score(im.score)
        .finalize();
    DB.lock().unwrap().add_image(&imb).unwrap();

    save_image(Path::new("assets/images"), &im.name, &body);

    Ok(())
}

/// Запросить и распарсить JSON
fn req_and_parse(client: &Client, url: &str) -> Result<Json, hyper::Error> {

    let mut res = match client.get(url)
        .header(UserAgent("Zeph/1.0".to_owned()))
        .send() {
            Ok(x)   => x,
            Err(x)  => {
                log(format!("ERROR: {}", x));
                return Err(x)
            }
        };

    let mut body = String::new();
    res.read_to_string(&mut body).unwrap();

    Ok(Json::from_str(&body).unwrap())
}
