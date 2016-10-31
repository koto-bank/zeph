extern crate hyper;

use self::hyper::client::Client;

use std::thread;
use std::time::Duration;

use super::{DB,Image,download,req_and_parse};

use std::sync::mpsc::Receiver;

pub fn main(rc: &Receiver<()>) {
    let client = Client::new();
    let images_c = DB.get_images(None,0).unwrap();
    let mut url_string = "https://konachan.com/post.json".to_string();
    let mut page = 1;

    'main: loop {
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

        for im in images {
            if !images_c.iter().any(|x| x.name == im.name ) {
                if let Err(_) = download(&Client::new(), &im, rc) { // Видимо, всё основанное на Danbooru не пускает с одного клиента
                    break 'main
                }
            }
        }

        page += 1;

        url_string = format!("https://konachan.com/post.json?page={}", page);
    }
}
