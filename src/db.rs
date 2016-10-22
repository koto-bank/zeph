extern crate time;
extern crate rusqlite;

pub struct Db {
    db: rusqlite::Connection
}

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

use self::rusqlite::{Result as SQLResult, Row};

impl Db {
    pub fn new() -> Db {
        use self::rusqlite::Connection;
        use std::path::Path;

        let conn = Connection::open(Path::new("db.db")).unwrap();
        conn.execute("CREATE TABLE IF NOT EXISTS images(
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    tags TEXT NOT NULL,

                    got_from TEXT,
                    original_link TEXT,
                    rating CHAR);",&[]).unwrap();
        Db{
            db: conn
        }
    }

    pub fn add_with_tags_name(&self, tags: &[String], ext: &str) -> SQLResult<String> {
        let mut name = tags.join("_").replace("'","''");
        name.push_str(ext);
        try!(self.add_image(&name, tags, None, None, None));
        Ok(name)
    }

    pub fn add_image<'a, T1: Into<Option<&'a str>>,
    T2: Into<Option<&'a str>>,
    C: Into<Option<char>>>(&self, name: &str, tags: &[String], got_from: T1, original_link: T2, rating: C) -> SQLResult<()> {
        let mut fields = "INSERT INTO images (name, tags".to_string();
        let mut values = format!("VALUES('{}', '{}'", name, format!(",{},",tags.join(",").replace("'","''")));
        if let Some(x) = got_from.into() {
            fields.push_str(", got_from");
            values.push_str(&format!(", '{}'", x));
        }
        if let Some(x) = original_link.into() {
            fields.push_str(", original_link");
            values.push_str(&format!(", '{}'", x));
        }
        if let Some(x) = rating.into() {
            fields.push_str(", rating");
            values.push_str(&format!(", '{}'", x));
        }

        fields.push_str(")");
        values.push_str(")");

        let q = format!("{} {}", fields, values);
        self.db.execute(&q, &[]).unwrap();
        Ok(())
    }

    fn extract_all(row: Row) -> Image {
        let id = row.get(0);
        let name = row.get(1);
        let mut tags = row.get::<i32,String>(2).split(',').map(|x| x.to_string()).collect::<Vec<_>>();
        let l = tags.len()-2;
        tags.remove(0); tags.remove(l);

        let got_from = row.get::<i32, Option<String>>(3).unwrap_or(" ".to_string());
        let original_link = row.get::<i32,Option<String>>(4).unwrap_or(" ".to_string());
        let rating = row.get::<i32,Option<String>>(5).unwrap_or(" ".to_string()).chars().nth(0).unwrap_or(' ');

        Image{
            id: id,
            name: name,
            tags: tags,
            got_from: got_from,
            original_link: original_link,
            rating: rating
        }
    }

    fn extract_all_ref(row: &Row) -> Image {
        let id = row.get(0);
        let name = row.get(1);
        let mut tags = row.get::<i32,String>(2).split(',').map(|x| x.to_string()).collect::<Vec<_>>();
        let l = tags.len()-2;
        tags.remove(0); tags.remove(l);

        let got_from = row.get::<i32, Option<String>>(3).unwrap_or(" ".to_string());
        let original_link = row.get::<i32,Option<String>>(4).unwrap_or(" ".to_string());
        let rating = row.get::<i32,Option<String>>(5).unwrap_or(" ".to_string()).chars().nth(0).unwrap_or(' ');

        Image{
            id: id,
            name: name,
            tags: tags,
            got_from: got_from,
            original_link: original_link,
            rating: rating
        }
    }

    pub fn get_image(&self, id: i32) -> SQLResult<Image> {
        self.db.query_row("SELECT * FROM images WHERE id = ?", &[&id], Db::extract_all)
    }

    pub fn get_images<T: Into<Option<i32>>>(&self, take: T, skip: usize) -> SQLResult<Vec<Image>>{
        let take = match take.into() {
            Some(x) => x,
            None    => -1
        };

        let mut st = try!(self.db.prepare(&format!("SELECT * FROM images LIMIT {} OFFSET {}", take, skip)));
        let st = try!(st.query_map(&[], Db::extract_all_ref)).map(|x| x.unwrap());
        Ok(st.collect::<Vec<_>>())
    }

    pub fn by_tags<T: Into<Option<i32>>>(&self, take: T, skip: usize, tags: &[String]) -> SQLResult<Vec<Image>> {
        let tags = tags.iter().map(|x| parse_tag(&x.replace("_",r"\_").replace("%", r"\%").replace("'", "''"))).collect::<Vec<_>>();

        let q = tags.iter().map(|t| {
            match *t {
                Tag::Include(ref incl) => format!(r"tags LIKE '%,{},%' ESCAPE '\'", incl),
                Tag::Exclude(ref excl) => format!(r"tags NOT LIKE '%,{},%' ESCAPE '\'", excl),
                Tag::AnyWith(ref x) => match *x {
                    AnyWith::Before(ref bef) => format!(r"tags LIKE '%,{}%,%' ESCAPE '\'", bef),
                    AnyWith::After(ref aft) => format!(r"tags LIKE '%,%{},%' ESCAPE '\'", aft),
                },
                Tag::Rating(ref r) => {
                    let mut s = "(".to_string();
                    for tg in r {
                        s.push_str(&format!("rating = '{}' OR ", tg))
                    }
                    let _ = (0..4).inspect(|_| {s.pop(); }).collect::<Vec<_>>();
                    s.push_str(")");

                    s
                },

                Tag::From(ref f) => {
                    let mut s = "(".to_string();
                    for tg in f {
                        s.push_str(&format!("got_from = '{}' OR ", tg))
                    }
                    let _ = (0..4).inspect(|_| {s.pop(); }).collect::<Vec<_>>();
                    s.push_str(")");

                    s
                }
            }
        }).collect::<Vec<_>>().join(" AND ");


        let take = match take.into() {
            Some(x) => x,
            None    => -1
        };

        let mut st = try!(self.db.prepare(&format!("SELECT * FROM images WHERE {} LIMIT {} OFFSET {}", q, take, skip)));
        let st = try!(st.query_map(&[], Db::extract_all_ref)).map(|x| x.unwrap());
        Ok(st.collect::<Vec<_>>())
    }
}

fn parse_tag(tag: &str) -> Tag {
    if tag.starts_with("rating") {
        let tag = tag.split("rating:").collect::<Vec<_>>()[1];
        Tag::Rating(tag.split(',').map(|x| x.to_string()).collect::<Vec<_>>())
    } else if tag.starts_with("from") {
        let tag = tag.split("from:").collect::<Vec<_>>()[1];
        Tag::From(tag.split(',').map(|x| x.to_string()).collect::<Vec<_>>())
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
