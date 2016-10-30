extern crate hyper;

use self::hyper::client::Client;

use std::thread;
use std::time::Duration;

use super::{DB,Image,download,req_and_parse};

use std::sync::mpsc::Receiver;

pub fn main(rc: &Receiver<()>) {
    let client = Client::new();
    let images_c = DB.get_images(None,0).unwrap();
    let mut url_string = "https://derpibooru.org/search.json?q=score.gt:0&filter_id=56027".to_string();
    let mut page = 1;

    'main: loop {
        let res = match req_and_parse(&client, &url_string) {
            Ok(x) => x,
            Err(_) => {
                thread::sleep(Duration::new(3,0));
                continue
            }
        };

        let images = res.as_object().unwrap()["search"].as_array().unwrap();
        if images.is_empty() { break }

        let images = images.iter().fold(Vec::new(), |mut acc, x| {
            let image = x.as_object().unwrap();
            let mut rating = String::new();

            let tags = image["tags"].as_string().unwrap().split(',').map(|x| x.trim().replace(" ", "_")).filter_map(|x| {
                if x.starts_with("artist:") {
                    Some(x.split(':').collect::<Vec<_>>()[1].to_string())
                } else if x == "safe" || x == "semi-grimdark" {
                    rating = "s".to_string();
                    None
                } else if x == "explicit" || x == "grimdark" || x == "grotesque" {
                    rating = "e".to_string();
                    None
                } else if x == "questionable" || x == "suggestive" {
                    rating = "q".to_string();
                    None
                } else {
                    Some(x.to_string())
                }}).collect::<Vec<_>>();
            let rating = rating.chars().collect::<Vec<_>>()[0];
            let url = format!("https:{}", image["image"].as_string().unwrap());
            let name = image["file_name"].as_string().unwrap();
            let id = image["id"].as_string().unwrap().parse::<u64>().unwrap();

            acc.push(Image{
                    name: format!("{}_{}",id,name),
                    got_from: "derpi".to_string(),
                    url: url,
                    tags: tags,
                    rating: rating,
                    post_url: format!("https://derpibooru.org/{}", id)
                });
            acc
        });

        for im in images {
            if !images_c.iter().any(|x| x.name == im.name ) {
                if let Err(_) = download(&client, &im, rc) {
                    break 'main
                }
            }
        }

        page += 1;

        url_string = format!("https://derpibooru.org/search.json?q=score.gt:0&filter_id=56027&page={}", page);
    }
}
