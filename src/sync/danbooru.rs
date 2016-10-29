extern crate hyper;

use self::hyper::client::Client;

use std::thread;
use std::time::Duration;

use ::db::Db;
use super::{Image,download,req_and_parse};

use std::sync::mpsc::Receiver;

pub fn main(rc: &Receiver<()>) {
    let db = Db::new();
    let client = Client::new();
    let images_c = db.get_images(None,0).unwrap();
    let mut url_string = "http://danbooru.donmai.us/posts.json".to_string();
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
            let tags = image["tag_string"].as_string().unwrap().split_whitespace().map(String::from).collect::<Vec<_>>();
            let rating = image["rating"].as_string().unwrap().chars().nth(0).unwrap();

            if let Some(ext) = image.get("file_ext") {
                let ext = ext.as_string().unwrap();

                if ext != "webm" && ext != "swf" && ext != "mp4" {
                    let url = format!("http://danbooru.donmai.us{}", image["file_url"].as_string().unwrap().to_string());
                    let name = url.clone();
                    let name = name.split('/').collect::<Vec<_>>();
                    let name = name[name.len()-1];

                    let id = image["id"].as_i64().unwrap();

                    acc.push(Image{
                        name: name.to_string(),
                        got_from: "danbooru".to_string(),
                        url: url,
                        tags: tags,
                        rating: rating,
                        post_url: format!("http://danbooru.donmai.us/posts/{}", id)
                    });
                }
            }
            acc
        });

        for im in images {
            if !images_c.iter().any(|x| x.name == im.name ) {
                if let Err(_) = download(&Client::new(), &im, rc) { //Вот это бред какой-то, но danbooru не даёт качать две картинки подряд
                    break 'main
                }
            }
        }

        page += 1;

        url_string = format!("http://danbooru.donmai.us/posts.json?page={}", page);
    }
}
