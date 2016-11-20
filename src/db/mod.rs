#[derive(Debug,Clone,RustcEncodable)]
pub struct Image {
    pub id: i32,
    pub name: String,
    pub tags: Vec<String>,
    pub got_from: Option<String>,
    pub original_link: Option<String>,
    pub uploader: Option<String>,
    pub rating: Option<char>,
    pub score: i32
}

/// Image voting error
pub enum VoteImageError {
    /// Voting up/down twice
    Already,
    /// No such image
    NoImage
}

/// Partial tags, e.g. `Some*`
#[derive(Debug,Clone)]
enum AnyWith {
    After(String),
    Before(String)
}

/// Ascending/descending sort
#[derive(Debug,Clone)]
enum AscDesc {
    Asc,
    Desc
}

/// Sorting, e.g. `sort:asc/desc:id/score`
#[derive(Debug,Clone)]
enum OrderBy {
    Id,
    Score
}

#[derive(Debug,Clone)]
enum Tag {
    /// Just a tag
    Include(String),
    /// Exlude tag
    Exclude(String),
    /// Rating, e.g. rating:s for safe
    Rating(Vec<String>),
    /// Partial tag
    AnyWith(AnyWith),
    /// from:derpibooru,konachan etc.
    From(Vec<String>),
    /// Uploader search
    Uploader(Vec<String>),
    /// Sorting
    OrderBy(OrderBy, AscDesc)
}

/// `pub` is used to switch DBs, though only postgres works TODO: fix sqlite sometime
pub mod postgres;
mod sqlite;

#[cfg(feature = "sqlite")]
pub use self::sqlite::Db;

#[cfg(feature = "postgresql")]
pub use self::postgres::Db;

/// Tag parsing, many copy-paste code and kinda ugly, probably could be better
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
    } else if tag.starts_with("sort:") {
        let t = tag.split(":").collect::<Vec<_>>();
        let s = t[1];
        let aod = match s {
            "asc" => AscDesc::Asc, // - -> +
            "desc" => AscDesc::Desc, // + -> -
            _   => AscDesc::Desc
        };
        let by = match t[2] {
            "id" => OrderBy::Id,
            "score" => OrderBy::Score,
            _   => OrderBy::Id
        };

        Tag::OrderBy(by, aod)
    } else {
        Tag::Include(tag.to_string())
    }
}

/// Strcture to ease image addition
pub struct ImageBuilder {
    name: String,
    tags: Vec<String>,
    got_from: Option<String>,
    original_link: Option<String>,
    uploader: Option<String>,
    score: i32,
    rating: Option<char>
}

impl ImageBuilder {
    pub fn new(name: &str, tags: &[String]) -> Self {
        ImageBuilder{
            name: name.to_string(),
            tags: tags.to_owned(),
            got_from: None,
            original_link: None,
            uploader: None,
            score: 0,
            rating: None
        }
    }

    pub fn got_from(mut self, got_from: &str) -> Self {
        self.got_from = Some(got_from.to_string());
        self
    }

    pub fn original_link(mut self, original_link: &str) -> Self {
        self.original_link = Some(original_link.to_string());
        self
    }

    pub fn uploader(mut self, uploader: &str) -> Self {
        self.uploader = Some(uploader.to_string());
        self
    }

    pub fn score(mut self, score: i32) -> Self {
        self.score = score;
        self
    }

    pub fn rating(mut self, rating: char) -> Self {
        self.rating = Some(rating);
        self
    }

    pub fn finalize(self) -> Self { self }
}
