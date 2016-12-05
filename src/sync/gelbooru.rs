extern crate hyper;

use self::hyper::client::Client;

use std::thread;
use std::time::Duration;

use super::{Image,process_downloads,req_and_parse};

use std::sync::mpsc::Receiver;

pub fn main(rc: &Receiver<()>) {
    let client = Client::new();
    let mut url_string = "http://gelbooru.com/index.php?page=dapi&s=post&q=index&json=1".to_string();
    let mut page = 1;

    loop {
        let res = match req_and_parse(&client, &url_string) {
            Ok(x) => x,
            Err(_) => {
                thread::sleep(Duration::new(3,0));
                continue
            }
        };

        let images = res.as_array().unwrap();
        if images.is_empty() { break }

        let images = images.iter().fold(Vec::new(), |mut acc, x| {
            let image = x.as_object().unwrap();
            let tags = image["tags"].as_string().unwrap().split_whitespace().map(String::from).collect::<Vec<_>>();
            let rating = image["rating"].as_string().unwrap().chars().nth(0).unwrap();

            let url = image["file_url"].as_string().unwrap().to_string();
                            
            let ext = image["image"].as_string().unwrap();
            let ext = ext.split('.').collect::<Vec<_>>();
            let ext = ext.last().unwrap();

            if *ext != "webm" && *ext != "swf" && *ext != "mp4" {
                let id = image["id"].as_i64().unwrap();
                let name = format!("gelbooru_{}.{}", id, ext);
                let score = image["score"].as_i64().unwrap();

                acc.push(Image{
                    name: name.to_string(),
                    got_from: "gelbooru".to_string(),
                    url: url,
                    tags: tags,
                    rating: rating,
                    post_url: format!("http://gelbooru.com/index.php?page=post&s=view&id={}", id),
                    score: score as i32
                });
                acc
            } else {
                acc
            }
        });

        if process_downloads(&client, &images, rc).is_err() { break }

        page += 1;

        url_string = format!("http://gelbooru.com//index.php?page=dapi&s=post&q=index&json=1&pid={}", page);
    }
}
