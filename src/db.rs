#![allow(warnings)]

extern crate time;
extern crate rusqlite;

use lmdb_rs::core::*;
use std::path::Path;
use std::fs::remove_file;

pub struct Db {
    env: Environment,
    handle: DbHandle
}

#[derive(Debug,Clone,RustcEncodable)]
pub struct Image {
    pub name: String,
    pub link: String,
    pub tags: Vec<String>
}

#[derive(Debug,Clone,RustcEncodable)]
pub struct SImage {
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
    AnyWith(AnyWith)
}

impl Db {
    pub fn new() -> Db {
        let env = EnvBuilder::new().open(Path::new("database"), 0o700).unwrap();
        Db{
            env:  env.clone(),
            handle: env.get_default_db(DbFlags::empty()).unwrap()
        }
    }

    pub fn add_image_with_name(&self, name: &str, tags: &[String]) -> MdbResult<()> {
        let tags = tags.join(" ");

        let trans = try!(self.env.new_transaction());
        {
            let db = trans.bind(&self.handle);
            try!(db.set(&name.to_string(), &tags));
        }

        trans.commit()
    }

    pub fn add_image(&self, tags: &[String], ext: &str) -> MdbResult<String> {
        let name = tags.iter().take(10).map(|x| x.as_str()).collect::<Vec<_>>();
        let name = format!("{}_{}.{}", name.join("_"), time::now().to_timespec().sec, ext);
        let tags = tags.join(" ");

        let trans = try!(self.env.new_transaction());
        {
            let db = trans.bind(&self.handle);
            try!(db.set(&name.to_string(), &tags));
        }

        try!(trans.commit());
        Ok(name)
    }

    pub fn del_image(&self, name: &str) -> MdbResult<()> {
        let trans = try!(self.env.new_transaction());
        {
            let db = trans.bind(&self.handle);
            try!(db.del(&name));
        }
        let _ = remove_file(Path::new(&format!("/assets/images/{}", name)));
        trans.commit()
    }

    pub fn get_image(&self, name: &str) -> Result<Image,MdbError> {
        let trans = try!(self.env.get_reader());
        let db = trans.bind(&self.handle);
        let tags = try!(db.get::<&str>(&name));
        let tags = tags.split_whitespace().map(|x| x.to_string()).collect::<Vec<_>>();
        let link = name.replace("(","_OPENQ_").replace(")","_CLOSEQ_");

        Ok(Image{
            name: name.to_string(),
            link: link,
            tags: tags
        })
    }

    pub fn get_images<T: Into<Option<usize>>>(&self, take: T, skip: usize) -> MdbResult<Vec<Image>> {
        let trans = try!(self.env.get_reader());
        let db = trans.bind(&self.handle);

        let take = match take.into() {
            Some(x) => x,
            None    => 0
        };
        let iter = try!(db.iter()).skip(skip).take(take);

        Ok(iter.fold(Vec::new(), |mut acc, x| {
            acc.push(Image{
                name: x.get_key(),
                link: x.get_key::<String>().replace("(","_OPENQ_").replace(")","_CLOSEQ_"),
                tags: x.get_value::<&str>().split(' ').map(|x| x.to_string()).collect::<Vec<_>>()
            });
            acc
        }))
    }

    fn parse_tag(&self, tag: &str) -> Tag {
        if tag.starts_with("rating") {
            let tag = tag.split("rating:").collect::<Vec<_>>()[1];
            Tag::Rating(tag.split(',').map(|x| x.to_string()).collect::<Vec<_>>())
        } else if tag.starts_with('-') {
            Tag::Exclude(tag[1..].to_string())
        } else if tag.starts_with('*') {
            Tag::AnyWith(AnyWith::After(tag[1..].to_string()))
        } else if tag.ends_with("*") {
            let mut n = tag.to_string();
            n.pop();
            Tag::AnyWith(AnyWith::Before(n))
        } else {
            Tag::Include(tag.to_string())
        }
    }

    pub fn by_tags<T: Into<Option<usize>>>(&self, take: T, skip: usize, tags: &[String]) -> MdbResult<Vec<Image>> {
        let tags = tags.iter().map(|x| self.parse_tag(x)).collect::<Vec<_>>();
        let needed_len = tags
            .iter()
            .filter(|x| match **x { Tag::Exclude(_) => false, _ => true })
            .collect::<Vec<_>>()
            .len();

        let trans = try!(self.env.get_reader());
        let db = trans.bind(&self.handle);

        let take = match take.into() {
            Some(x) => x,
            None    => 0
        };
        let iter = try!(db.iter());

        let res = iter.filter_map(|x| {
            let x_tags = x.get_value::<&str>().split_whitespace().map(|x| x.to_string()).collect::<Vec<_>>();

            let mut is_all_tags = Some(0);
            'up: for x_tag in &x_tags {
                for tag in &tags {
                    match *tag {
                        Tag::Include(ref incl) => if incl == x_tag { is_all_tags = is_all_tags.map(|x| x + 1 ) },
                        Tag::Exclude(ref excl) => if excl == x_tag { is_all_tags = None; break 'up },
                        Tag::AnyWith(ref x) => match *x {
                            AnyWith::Before(ref bef)    => if x_tag.starts_with(bef) { is_all_tags = is_all_tags.map(|x| x + 1 ) },
                            AnyWith::After(ref aft)        => if x_tag.ends_with(aft) { is_all_tags = is_all_tags.map(|x| x + 1 ) }
                        },
                        Tag::Rating(ref r) => {
                            for tg in r {
                                if format!("rating:{}", tg) == *x_tag { is_all_tags = is_all_tags.map(|x| x + 1 ) }
                            }}
                    }
                }
            }

            if let Some(num) = is_all_tags {
                if num == needed_len {
                    Some(Image{
                        name: x.get_key(),
                        link: x.get_key::<String>().replace("(","_OPENQ_").replace(")","_CLOSEQ_"),
                        tags: x_tags
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }).skip(skip).take(take).collect::<Vec<_>>();

        Ok(res)
    }
}

use self::rusqlite::{Result as SQLResult, Row};

pub struct DbS {
    db: rusqlite::Connection
}

impl DbS {
    pub fn new() -> DbS {
        use self::rusqlite::Connection;
        use std::path::Path;

        let conn = Connection::open(Path::new("db.db")).unwrap();
        conn.execute("CREATE TABLE IF NOT EXISTS images(
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    tags TEXT NOT NULL,

                    got_from TEXT,
                    original_link TEXT,
                    rating CHAR);",&[]);
        DbS{
            db: conn
        }
    }

    pub fn add_image<'a, T1: Into<Option<&'a str>>,
    T2: Into<Option<&'a str>>,
    C: Into<Option<char>>>(&self, name: &str, tags: &[String], got_from: T1, original_link: T2, rating: C) -> SQLResult<i32> {
        let mut fields = "INSERT INTO images (name, tags".to_string();
        let mut values = format!("VALUES('{}', '{}'", name, format!(",{},",tags.join(",")));
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
        self.db.execute(&q, &[])
    }

    fn extract_all(row: Row) -> SImage {
        let id = row.get(0);
        let name = row.get(1);
        let mut tags = row.get::<i32,String>(2).split(",").map(|x| x.to_string()).collect::<Vec<_>>();
        let l = tags.len()-2;
        tags.remove(0); tags.remove(l);

        let got_from = row.get::<i32, Option<String>>(3).unwrap_or(" ".to_string());
        let original_link = row.get::<i32,Option<String>>(4).unwrap_or(" ".to_string());
        let rating = row.get::<i32,String>(5).chars().nth(0).unwrap_or(' ');

        SImage{
            id: id,
            name: name,
            tags: tags,
            got_from: got_from,
            original_link: original_link,
            rating: rating
        }
    }

    fn extract_all_ref(row: &Row) -> SImage {
        let id = row.get(0);
        let name = row.get(1);
        let mut tags = row.get::<i32,String>(2).split(",").map(|x| x.to_string()).collect::<Vec<_>>();
        let l = tags.len()-2;
        tags.remove(0); tags.remove(l);

        let got_from = row.get::<i32, Option<String>>(3).unwrap_or(" ".to_string());
        let original_link = row.get::<i32,Option<String>>(4).unwrap_or(" ".to_string());
        let rating = row.get::<i32,String>(5).chars().nth(0).unwrap_or(' ');

        SImage{
            id: id,
            name: name,
            tags: tags,
            got_from: got_from,
            original_link: original_link,
            rating: rating
        }
    }

    pub fn get_image(&self, id: i32) -> SQLResult<SImage> {
        self.db.query_row("SELECT * FROM images WHERE id = ?", &[&id], DbS::extract_all)
    }

    pub fn get_images<T: Into<Option<usize>>>(&self, take: T, skip: usize) -> SQLResult<Vec<SImage>>{
        let take = match take.into() {
            Some(x) => format!("LIMIT {}", x),
            None    => "".to_string()
        };

        let mut st =  try!(self.db.prepare(&format!("SELECT * FROM images {} OFFSET {}", take, skip)));
        let st = try!(st.query_map(&[], DbS::extract_all_ref)).map(|x| x.unwrap());
        Ok(st.collect::<Vec<_>>())
    }

    pub fn by_tags<T: Into<Option<usize>>>(&self, take: T, skip: usize, tags: &[String]) -> SQLResult<Vec<SImage>> {
        let mut q = String::new();

        let tags = tags.iter().map(|x| parse_tag(&x.replace("_",r"\_").replace("%", r"\%"))).collect::<Vec<_>>();

        for t in tags {
            match t {
                Tag::Include(ref incl) => q.push_str(&format!(r"tags LIKE '%,{},%' ESCAPE '\' AND ", incl)),
                Tag::Exclude(ref excl) => q.push_str(&format!(r"tags NOT LIKE '%,{},%' ESCAPE '\' AND ", excl)),
                Tag::AnyWith(ref x) => match *x {
                    AnyWith::Before(ref bef) => q.push_str(&format!(r"tags LIKE '%,{}%,%' ESCAPE '\' AND ", bef)),
                    AnyWith::After(ref aft) => q.push_str(&format!(r"tags LIKE '%,%{},%' ESCAPE '\' AND ", aft)),
                },
                Tag::Rating(ref r) => {
                    for tg in r {
                        q.push_str(&format!("rating = '{}' AND ", tg))
                    }}
            }
        }


        let _ = (0..5).inspect(|_| { q.pop(); }).collect::<Vec<_>>();

        let take = match take.into() {
            Some(x) => format!("LIMIT {}", x),
            None    => "".to_string()
        };

        let mut st = try!(self.db.prepare(&format!("SELECT * FROM images WHERE {} {} OFFSET {}", q, take, skip)));
        let st = try!(st.query_map(&[], DbS::extract_all_ref)).map(|x| x.unwrap());
        Ok(st.collect::<Vec<_>>())
    }
}

fn parse_tag(tag: &str) -> Tag {
    if tag.starts_with("rating") {
        let tag = tag.split("rating:").collect::<Vec<_>>()[1];
        Tag::Rating(tag.split(',').map(|x| x.to_string()).collect::<Vec<_>>())
    } else if tag.starts_with('-') {
        Tag::Exclude(tag[1..].to_string())
    } else if tag.starts_with('*') {
        Tag::AnyWith(AnyWith::After(tag[1..].to_string()))
    } else if tag.ends_with("*") {
        let mut n = tag.to_string();
        n.pop();
        Tag::AnyWith(AnyWith::Before(n))
    } else {
        Tag::Include(tag.to_string())
    }
}
