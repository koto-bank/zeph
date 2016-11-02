#![cfg(feature = "postgresql")]

extern crate postgres;

use self::postgres::{Connection, TlsMode, Result as SQLResult};
use self::postgres::rows::Row;

pub struct Db(Connection);

use super::{Image,Tag,AnyWith,parse_tag};

impl Default for Db { // Чтобы Clippy не жаловался
    fn default() -> Self {
        Self::new()
    }
}

static POSTGRES_LOGIN : &'static str = "easy";
static POSTGRES_PASS : &'static str = "";

impl Db {
    pub fn new() -> Self {
        let conn = Connection::connect(format!("postgres://{}:{}@localhost", POSTGRES_LOGIN, POSTGRES_PASS), TlsMode::None).unwrap();
        conn.execute("CREATE TABLE IF NOT EXISTS images(
                     id SERIAL PRIMARY KEY,
                     name TEXT NOT NULL UNIQUE,
                     tags TEXT[] NOT NULL,

                     got_from TEXT,
                     original_link TEXT,
                     rating CHAR
                    );", &[]).unwrap();
        Db(conn)
    }

    /// Сохранить картинку, сгенерировава имя из тэгов
    pub fn add_with_tags_name(&self, tags: &[String], ext: &str) -> SQLResult<String> {
        let lastnum = self.0.query("SELECT id FROM images ORDER BY id DESC LIMIT 1", &[])?.get(0).get::<_, i32>("id");

        let name = format!("{}_{}.{}", lastnum + 1  , tags.join("_").replace("'","''"),ext);
        self.add_image(&name, tags, None, None, None)?;
        Ok(name)
    }

    pub fn add_image<'a, T1: Into<Option<&'a str>>,
    T2: Into<Option<&'a str>>,
    C: Into<Option<char>>>(&self, name: &str, tags: &[String], got_from: T1, original_link: T2, rating: C) -> SQLResult<()> {
        self.0.execute("INSERT into images (name,tags,got_from,original_link,rating) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (name) DO UPDATE SET tags = $2",
        &[&name,&tags,&got_from.into(), &original_link.into(),&rating.into().map(|x| x.to_string())]).unwrap();
        Ok(())
    }

    pub fn get_image(&self, id: i32) -> SQLResult<Image> {
        let row = self.0.query("SELECT * FROM images WHERE id = $1", &[&id])?;
        Ok(Db::extract_image(row.get(0)))
    }

    pub fn get_images<T: Into<Option<i32>>>(&self, take: T, skip: usize) -> SQLResult<Vec<Image>>{
        let take = match take.into() {
            Some(x) => x.to_string(),
            None    => "ALL".to_string()
        };

        Ok(self.0.query(&format!("SELECT * FROM images ORDER BY id DESC LIMIT {} OFFSET {}", take, skip as i32),&[])?
           .iter().fold(Vec::new(), |mut acc, row| {
               acc.push(Db::extract_image(row));
               acc
           }))
    }

    pub fn by_tags<T: Into<Option<i32>>>(&self, take: T, skip: usize, tags: &[String]) -> SQLResult<Vec<Image>> {
        let tags = tags.iter().map(|x| parse_tag(x)).collect::<Vec<_>>();

        let q = tags.iter().map(|t| {
            match *t {
                Tag::Include(ref incl) => format!(r"tags @> ARRAY['{}']", incl),
                Tag::Exclude(ref excl) => format!(r"NOT tags @> ARRAY['{}']", excl),
                Tag::AnyWith(ref x) => match *x {
                    AnyWith::Before(ref bef) => format!(r"(SELECT bool_or(tag ~ '^{}') FROM unnest(tags) t (tag))", bef),
                    AnyWith::After(ref aft) => format!(r"(SELECT bool_or(tag ~ '{}$') FROM unnest(tags) t (tag))", aft),
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
            Some(x) => x.to_string(),
            None    => "ALL".to_string()
        };

        Ok(self.0.query(&format!("SELECT * FROM images WHERE {} ORDER BY id DESC LIMIT {} OFFSET {}", q, take, skip),&[])?
           .iter().fold(Vec::new(), |mut acc, row| {
               acc.push(Db::extract_image(row));
               acc
           }))
    }

    fn extract_image(row: Row) -> Image {
        Image{
            id: row.get("id"),
            name: row.get("name"),
            tags: row.get("tags"),
            got_from: row.get::<_, Option<String>>("got_from").unwrap_or(" ".to_string()),
            original_link: row.get::<_, Option<String>>("original_link").unwrap_or(" ".to_string()),
            rating: row.get::<_,Option<String>>("rating").unwrap_or(" ".to_string()).chars().nth(0).unwrap_or(' ')
        }
    }
}
