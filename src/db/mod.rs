#[derive(Debug,Clone,RustcEncodable)]
pub struct Image {
    pub id: i32,
    pub name: String,
    pub tags: Vec<String>,
    pub got_from: String,
    pub original_link: String,
    pub rating: char
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
    From(Vec<String>)
}

mod sqlite;

pub use self::sqlite::Db;

unsafe impl Sync for Db {}

fn parse_tag(tag: &str) -> Tag {
    if tag.starts_with("rating") {
        let tag = tag.split("rating:").collect::<Vec<_>>()[1];
        Tag::Rating(tag.split(',').map(String::from).collect::<Vec<_>>())
    } else if tag.starts_with("from") {
        let tag = tag.split("from:").collect::<Vec<_>>()[1];
        Tag::From(tag.split(',').map(String::from).collect::<Vec<_>>())
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
