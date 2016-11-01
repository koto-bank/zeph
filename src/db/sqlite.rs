#![cfg(feature = "sqlite")]

extern crate rusqlite;

use super::{Image,Tag,AnyWith,parse_tag};

use self::rusqlite::{Result as SQLResult, Row, Connection};
use std::path::Path;

pub struct Db(Connection);

impl Default for Db { // Чтобы Clippy не жаловался
    fn default() -> Self {
        Self::new()
    }
}

impl Db {
    pub fn new() -> Self {
        let conn = Connection::open(Path::new("db.db")).unwrap();
        conn.execute("CREATE TABLE IF NOT EXISTS images(
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL UNIQUE,
                    tags TEXT NOT NULL,

                    got_from TEXT,
                    original_link TEXT,
                    rating CHAR);",&[]).unwrap();
        Db(conn)
    }

    /// Сохранить картинку, сгенерировава имя из тэгов
    pub fn add_with_tags_name(&self, tags: &[String], ext: &str) -> SQLResult<String> {
        let lastnum = self.0.query_row("SELECT id FROM images ORDER BY id DESC LIMIT 1", &[], |row| {
            row.get::<i32,i32>(0)
        }).unwrap();

        let name = format!("{}_{}.{}", lastnum + 1  , tags.join("_").replace("'","''"),ext);
        self.add_image(&name, tags, None, None, None)?;
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
        self.0.execute(&q, &[]).unwrap();
        Ok(())
    }

    pub fn get_image(&self, id: i32) -> SQLResult<Image> {
        self.0.query_row("SELECT * FROM images WHERE id = ?", &[&id], Db::extract_all)
    }

    pub fn get_images<T: Into<Option<i32>>>(&self, take: T, skip: usize) -> SQLResult<Vec<Image>>{
        let take = match take.into() {
            Some(x) => x,
            None    => -1
        };

        let mut st = self.0.prepare(&format!("SELECT * FROM images ORDER BY id DESC LIMIT {} OFFSET {}", take, skip))?;
        let st = st.query_map(&[], Db::extract_all_ref)?.map(|x| x.unwrap());
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

        let mut st = self.0.prepare(&format!("SELECT * FROM images WHERE {} ORDER BY id DESC LIMIT {} OFFSET {}", q, take, skip))?;
        let st = st.query_map(&[], Db::extract_all_ref)?.map(|x| x.unwrap());
        Ok(st.collect::<Vec<_>>())
    }

    fn extract_all(row: Row) -> Image {
        let id = row.get(0);
        let name = row.get(1);
        let mut tags = row.get::<i32,String>(2).split(',').map(String::from).collect::<Vec<_>>();
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
        let mut tags = row.get::<i32,String>(2).split(',').map(String::from).collect::<Vec<_>>();
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
}
