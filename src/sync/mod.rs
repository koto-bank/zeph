use hyper::client::Client;
use hyper::header::UserAgent;
use hyper::Error as HyperError;

use std::io::Read;
use std::path::Path;
use std::str::FromStr;

use {DB,CONFIG};
use db::ImageBuilder;
use utils::*;

use std::sync::mpsc::{Receiver,TryRecvError};

use serde_json::Value;

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
pub mod gelbooru;

/// Do the prepare/download/add-to-db chores on an image list
/// and stop if console sends `kill` signal
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
                log(format!("DONE {}", im.name));
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

/// Download image and add it to DB
fn download(client: &Client, im: &Image) -> Result<(), HyperError> {
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

    save_image(Path::new(config!("images-directory")), &im.name, &body);

    Ok(())
}

/// Get and parse JSON
fn req_and_parse(client: &Client, url: &str) -> Result<Value, HyperError> {
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

    Ok(Value::from_str(&body).unwrap())
}
