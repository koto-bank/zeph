extern crate time;

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

#[derive(Debug,Clone)]
enum AnyWith {
    After(String),
    Before(String)
}

#[derive(Debug,Clone)]
enum Tag {
    Include(String),
    Exclude(String),
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

    fn parse_tag(&self, tag: &str) -> Vec<Tag> {
        if tag.starts_with("rating") {
            let tag = tag.split("rating:").collect::<Vec<_>>()[1];
            let ratings = tag.split(',');
            ratings.map(|x| Tag::Include(format!("rating:{}",x))).collect::<Vec<_>>()
        } else if tag.starts_with('-') {
            vec![Tag::Exclude(tag[1..].to_string())]
        } else if tag.starts_with('*') {
            vec![Tag::AnyWith(AnyWith::After(tag[1..].to_string()))]
        } else if tag.ends_with("*") {
            let mut n = tag.to_string();
            n.pop();
            vec![Tag::AnyWith(AnyWith::Before(n))]
        } else {
            vec![Tag::Include(tag.to_string())]
        }
    }

    pub fn by_tags<T: Into<Option<usize>>>(&self, take: T, skip: usize, tags: &[String]) -> MdbResult<Vec<Image>> {
        let tags = tags.iter().map(|x| self.parse_tag(x)).collect::<Vec<_>>();
        let needed_len = tags
            .iter()
            .flat_map(|x| x)
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
                for tag in &tags.iter().flat_map(|x| x).collect::<Vec<_>>() {
                    match **tag {
                        Tag::Include(ref incl)  => if incl == x_tag { is_all_tags = is_all_tags.map(|x| x + 1 ) },
                        Tag::Exclude(ref excl)  => if excl == x_tag { is_all_tags = None; break 'up },
                        Tag::AnyWith(ref x)    => match *x {
                            AnyWith::Before(ref bef)    => if x_tag.starts_with(bef) { is_all_tags = is_all_tags.map(|x| x + 1 ) },
                            AnyWith::After(ref aft)        => if x_tag.ends_with(aft) { is_all_tags = is_all_tags.map(|x| x + 1 ) }
                        }
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
