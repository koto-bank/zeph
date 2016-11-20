extern crate hyper;

use self::hyper::client::Client;

use std::thread;
use std::time::Duration;

use super::{Image,process_downloads,req_and_parse};

use std::sync::mpsc::Receiver;

pub fn main(rc: &Receiver<()>) {
    let client = Client::new();
    let mut url_string = "https://e621.net/post/index.json?limit=300".to_string(); // limit is actually 320

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

        let before_id = images[images.len()-1]
            .as_object()
            .map(|x| &x["id"])
            .and_then(|x| x.as_u64())
            .unwrap();

        let images = images.iter().fold(Vec::new(), |mut acc, x| {
            let image = x.as_object().unwrap();
            let tags = image["tags"].as_string().unwrap().split_whitespace().map(String::from).collect::<Vec<_>>();
            let rating = image["rating"].as_string().unwrap().chars().nth(0).unwrap();

            let ext = image["file_ext"].as_string().unwrap();
            if ext != "webm" && ext != "swf" && ext != "mp4" {
                let url = image["file_url"].as_string().unwrap().to_string();
                let id = image["id"].as_i64().unwrap();
                let name = format!("e621_{}.{}", id, ext);
                let score = image["score"].as_i64().unwrap();

                acc.push(Image{
                    name: name.to_string(),
                    got_from: "e621".to_string(),
                    url: url,
                    tags: tags,
                    rating: rating,
                    post_url: format!("https://e621.net/post/show/{}", id),
                    score: score as i32
                });
                acc
            } else {
                acc
            }
        });

        if process_downloads(&client, &images, rc).is_err() { break }

        url_string = format!("https://e621.net/post/index.json?before_id={}&limit=300", before_id);
    }
}
