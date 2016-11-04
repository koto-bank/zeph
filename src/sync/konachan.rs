extern crate hyper;

use self::hyper::client::Client;

use std::thread;
use std::time::Duration;

use super::{Image,process_downloads,req_and_parse};

use std::sync::mpsc::Receiver;

pub fn main(rc: &Receiver<()>) {
    let client = Client::new();
    let mut url_string = "https://konachan.com/post.json?limit=100".to_string();
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

            let ext = url.clone();
            let ext = ext.split('.').collect::<Vec<_>>();
            let ext = ext.last().unwrap();

            let id = image["id"].as_i64().unwrap();

            let name = format!("konachan_{}.{}", id, ext);

            acc.push(Image{
                name: name.to_string(),
                got_from: "konachan".to_string(),
                url: url,
                tags: tags,
                rating: rating,
                post_url: format!("http://konachan.com/post/show/{}", id)
            });
            acc
        });

        if process_downloads(&client, &images, rc).is_err() { break }

        page += 1;

        url_string = format!("https://konachan.com/post.json?page={}&limit=100", page);
    }
}
