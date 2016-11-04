#[derive(Debug,Clone,RustcEncodable)]
pub struct Image {
    pub id: i32,
    pub name: String,
    pub tags: Vec<String>,
    pub got_from: Option<String>,
    pub original_link: Option<String>,
    pub uploader: Option<String>,
    pub rating: Option<char>,
}

#[derive(Debug,Clone)]
enum AnyWith {
    After(String),
    Before(String)
}

#[derive(Debug,Clone)]
enum Tag {
    Include(String),
    Exclude(String),
    Rating(Vec<String>),
    AnyWith(AnyWith),
    From(Vec<String>),
    Uploader(Vec<String>)
}

pub mod postgres;
mod sqlite;

#[cfg(feature = "sqlite")]
pub use self::sqlite::Db;

#[cfg(feature = "postgresql")]
pub use self::postgres::Db;

fn parse_tag(tag: &str) -> Tag {
    if tag.starts_with("rating") {
        let tag = tag.split("rating:").collect::<Vec<_>>()[1];
        Tag::Rating(tag.split(',').map(String::from).collect::<Vec<_>>())
    } else if tag.starts_with("from") {
        let tag = tag.split("from:").collect::<Vec<_>>()[1];
        Tag::From(tag.split(',').map(String::from).collect::<Vec<_>>())
    } else if tag.starts_with("uploader") {
        let tag = tag.split("uploader:").collect::<Vec<_>>()[1];
        Tag::Uploader(tag.split(',').map(String::from).collect::<Vec<_>>())
    } else if tag.starts_with('-') {
        Tag::Exclude(tag[1..].to_string())
    } else if tag.starts_with('*') {
        Tag::AnyWith(AnyWith::After(tag[1..].to_string()))
    } else if tag.ends_with('*') {
        let mut n = tag.to_string();
        n.pop();
        Tag::AnyWith(AnyWith::Before(n))
    } else {
        Tag::Include(tag.to_string())
    }
}
