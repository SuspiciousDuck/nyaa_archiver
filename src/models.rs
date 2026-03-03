use crate::schema::*;
use diesel::prelude::*;
use num_derive::FromPrimitive as from_num;
use num_traits::FromPrimitive;
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug)]
pub enum Item {
    User,
    Torrent,
    Comment,
}
impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::User => "User",
            Self::Torrent => "Torrent",
            Self::Comment => "Comment",
        })
    }
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("{0} {1} already exists!")]
    Exists(Item, String), // Item, Identifier
    #[error("{0} {1} doesn't exist!")]
    NotExists(Item, String), // Item, Identifier
    #[error("Required fields weren't filled!: {0:?}")]
    MissingFields(Vec<String>), // List of missing fields
    #[error("Failed to create {0}!: {1}")]
    Create(Item, diesel::result::Error), // Item
    #[error("Failed to update {0} {1}!: {2}")]
    Update(Item, String, diesel::result::Error), // Item, Identifier
    #[error("Failed to delete {0} {1}!: {2}")]
    Delete(Item, String, diesel::result::Error),
    #[error("Failed to search for {0}!: {1}")]
    Search(Item, diesel::result::Error),
    #[error("Failed to hash password!")]
    Hash,
}

#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug)]
#[diesel[table_name = users, treat_none_as_null = true]]
#[diesel[check_for_backend(diesel::sqlite::Sqlite)]]
pub struct User {
    pub username: String,
    pub password: Option<String>,
    pub salt: Option<String>,
    pub email: Option<String>,
    pub nyaa: bool,
    pub trusted: bool,
    pub banned: bool,
    pub last_updated: Option<i32>,
}
impl User {
    pub fn insert(&self, conn: &mut SqliteConnection, replace: bool) -> Result<(), DatabaseError> {
        let exists = crate::user_exists(conn, &self.username);
        if exists && !replace {
            return Err(DatabaseError::Exists(Item::User, self.username.clone()));
        }
        let password = self.password.as_ref();
        let salt = self.password.as_ref();
        let email = self.password.as_ref();
        let mut missing = vec![];
        if password.is_none() {
            missing.push("password".to_string());
        }
        if salt.is_none() {
            missing.push("salt".to_string());
        }
        if email.is_none() {
            missing.push("email".to_string());
        }
        if !self.nyaa && !missing.is_empty() {
            return Err(DatabaseError::MissingFields(missing));
        }

        if replace && exists {
            diesel::update(users::table)
                .filter(users::username.eq(self.username.clone()))
                .set(self)
                .returning(User::as_returning())
                .get_result(conn)
                .map(|_| ())
                .map_err(|e| DatabaseError::Update(Item::User, self.username.clone(), e))
        } else {
            diesel::insert_into(users::table)
                .values(self)
                .returning(User::as_returning())
                .get_result(conn)
                .map(|_| ())
                .map_err(|e| DatabaseError::Create(Item::User, e))
        }
    }

    pub fn hash(&mut self) -> Result<(), DatabaseError> {
        let password = self
            .password
            .as_ref()
            .ok_or(DatabaseError::MissingFields(vec!["password".to_string()]))?;
        let (hash, salt) = crate::util::hash_password(password).map_err(|_| DatabaseError::Hash)?;
        self.password = Some(hash);
        self.salt = Some(salt);
        Ok(())
    }
}

#[derive(Queryable, Selectable, Insertable, Debug)]
#[diesel(table_name = deleted_torrents)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct DeletedTorrent {
    pub id: i32,
}
impl DeletedTorrent {
    pub fn insert(&self, conn: &mut SqliteConnection) -> Result<(), DatabaseError> {
        diesel::insert_into(deleted_torrents::table)
            .values(self)
            .execute(conn)
            .map(|_| ())
            .map_err(|e| DatabaseError::Create(Item::Torrent, e))
    }
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = torrents, treat_none_as_null = true)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Torrent {
    pub id: i32,
    pub info_hash: Option<String>,
    pub seeders: i32,
    pub leechers: i32,
    pub completed: i32,
    pub title: String,
    pub category: i32,
    pub submitter: Option<String>,
    pub information: Option<String>,
    pub size: i64,
    pub date: i32,
    pub description: Option<String>,
    pub comments: i32,
    pub remake: bool,
    pub trusted: bool,
    pub partial: bool,
    pub anonymous: bool,
    pub deleted: bool,
    pub last_updated: Option<i32>,
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = torrents, treat_none_as_null = true)]
pub struct NewTorrent {
    pub id: Option<i32>,
    pub info_hash: Option<String>,
    pub title: String,
    pub category: i32,
    pub submitter: Option<String>,
    pub information: Option<String>,
    pub size: i64,
    pub date: i32,
    pub description: Option<String>,
    pub comments: i32,
    pub remake: bool,
    pub trusted: bool,
    pub partial: bool,
    pub anonymous: bool,
    pub deleted: bool,
    pub last_updated: Option<i32>,
}
impl NewTorrent {
    pub fn insert(
        &self,
        conn: &mut SqliteConnection,
        replace: bool,
    ) -> Result<Torrent, DatabaseError> {
        let id = self.id.as_ref().map(|id| *id as usize);
        let exists = id.is_some() && crate::torrent_exists(conn, &id.unwrap());
        if exists && !replace {
            return Err(DatabaseError::Exists(
                Item::Torrent,
                id.unwrap().to_string(),
            ));
        }

        if let (true, Some(id)) = (replace && exists, id) {
            diesel::update(torrents::table)
                .filter(torrents::id.eq(id as i32))
                .set(self)
                .returning(Torrent::as_returning())
                .get_result(conn)
                .map_err(|e| DatabaseError::Update(Item::Torrent, id.to_string(), e))
        } else {
            diesel::insert_into(torrents::table)
                .values(self)
                .returning(Torrent::as_returning())
                .get_result(conn)
                .map_err(|e| DatabaseError::Create(Item::Torrent, e))
        }
    }
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = comments, treat_none_as_null = true)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Comment {
    pub id: i32,
    pub torrent_id: i32,
    pub submitter: String,
    pub date_created: i32,
    pub date_edited: Option<i32>,
    pub text: String,
}

#[derive(Insertable, AsChangeset, Debug)]
#[diesel(table_name = comments, treat_none_as_null = true)]
pub struct NewComment {
    pub id: Option<i32>,
    pub torrent_id: i32,
    pub submitter: String,
    pub date_created: i32,
    pub date_edited: Option<i32>,
    pub text: String,
}
impl NewComment {
    pub fn insert(
        &self,
        conn: &mut SqliteConnection,
        replace: bool,
    ) -> Result<Comment, DatabaseError> {
        use torrents::dsl;
        if !crate::torrent_exists(conn, &(self.torrent_id as usize)) {
            return Err(DatabaseError::NotExists(
                Item::Torrent,
                self.torrent_id.to_string(),
            ));
        }
        let id = self.id.as_ref().map(|id| *id as usize);
        let exists = id.is_some() && crate::comment_exists(conn, &id.unwrap());
        if exists && !replace {
            return Err(DatabaseError::Exists(
                Item::Comment,
                id.unwrap().to_string(),
            ));
        }

        if let (true, Some(id)) = (replace && exists, id) {
            diesel::update(comments::table)
                .filter(comments::id.eq(id as i32))
                .set(self)
                .returning(Comment::as_returning())
                .get_result(conn)
                .map_err(|e| DatabaseError::Update(Item::Comment, id.to_string(), e))
        } else {
            let result = diesel::insert_into(comments::table)
                .values(self)
                .returning(Comment::as_returning())
                .get_result(conn)
                .map_err(|e| DatabaseError::Create(Item::Comment, e))?;
            diesel::update(dsl::torrents)
                .filter(dsl::id.eq(self.torrent_id))
                .set(dsl::comments.eq(dsl::comments + 1))
                .execute(conn)
                .map_err(|e| {
                    DatabaseError::Update(Item::Torrent, self.torrent_id.to_string(), e)
                })?;
            Ok(result)
        }
    }
}

#[repr(u8)]
pub enum Category {
    Anime(AnimeSubCategory) = 10,
    Audio(AudioSubCategory) = 20,
    Literature(LiteratureSubCategory) = 30,
    LiveAction(LiveActionSubCategory) = 40,
    Pictures(PicturesSubCategory) = 50,
    Software(SoftwareSubCategory) = 60,
}
impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Anime(_) => "Anime",
            Self::Audio(_) => "Audio",
            Self::Literature(_) => "Literature",
            Self::LiveAction(_) => "Live Action",
            Self::Pictures(_) => "Pictures",
            Self::Software(_) => "Software",
        })
    }
}
impl Category {
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            11..=14 => Some(Self::Anime(AnimeSubCategory::from_u8(n)?)),
            21..=22 => Some(Self::Audio(AudioSubCategory::from_u8(n)?)),
            31..=33 => Some(Self::Literature(LiteratureSubCategory::from_u8(n)?)),
            41..=44 => Some(Self::LiveAction(LiveActionSubCategory::from_u8(n)?)),
            51..=52 => Some(Self::Pictures(PicturesSubCategory::from_u8(n)?)),
            61..=62 => Some(Self::Software(SoftwareSubCategory::from_u8(n)?)),
            _ => None,
        }
    }
    pub fn fancy(&self) -> String {
        match self {
            Self::Anime(c) => format!("{self} - {}", c.fancy()),
            Self::Audio(c) => format!("{self} - {}", c.fancy()),
            Self::Literature(c) => format!("{self} - {}", c.fancy()),
            Self::LiveAction(c) => format!("{self} - {}", c.fancy()),
            Self::Pictures(c) => format!("{self} - {}", c.fancy()),
            Self::Software(c) => format!("{self} - {}", c.fancy()),
        }
    }
    pub fn normal(&self) -> String {
        match self {
            Self::Anime(c) => format!("{self} - {c}"),
            Self::Audio(c) => format!("{self} - {c}"),
            Self::Literature(c) => format!("{self} - {c}"),
            Self::LiveAction(c) => format!("{self} - {c}"),
            Self::Pictures(c) => format!("{self} - {c}"),
            Self::Software(c) => format!("{self} - {c}"),
        }
    }
}

pub trait FancySubCategory {
    fn fancy(&self) -> String;
}

#[derive(from_num)]
pub enum AnimeSubCategory {
    AnimeMusicVideo = 11,
    EnglishTranslated = 12,
    NonEnglishTranslated = 13,
    Raw = 14,
}
impl Display for AnimeSubCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AnimeMusicVideo => "AMV",
            Self::EnglishTranslated => "English",
            Self::NonEnglishTranslated => "Non-English",
            Self::Raw => "Raw",
        })
    }
}
impl FancySubCategory for AnimeSubCategory {
    fn fancy(&self) -> String {
        match self {
            Self::AnimeMusicVideo => "Anime Music Video",
            Self::EnglishTranslated => "English-translated",
            Self::NonEnglishTranslated => "Non-English-translated",
            Self::Raw => "Raw",
        }
        .to_string()
    }
}

#[derive(from_num)]
pub enum AudioSubCategory {
    Lossless = 21,
    Lossy = 22,
}
impl Display for AudioSubCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Lossless => "Lossless",
            Self::Lossy => "Lossy",
        })
    }
}
impl FancySubCategory for AudioSubCategory {
    fn fancy(&self) -> String {
        self.to_string()
    }
}

#[derive(from_num)]
pub enum LiteratureSubCategory {
    EnglishTranslated = 31,
    NonEnglishTranslated = 32,
    Raw = 33,
}
impl Display for LiteratureSubCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EnglishTranslated => "English",
            Self::NonEnglishTranslated => "Non-English",
            Self::Raw => "Raw",
        })
    }
}
impl FancySubCategory for LiteratureSubCategory {
    fn fancy(&self) -> String {
        match self {
            Self::EnglishTranslated => "English-translated",
            Self::NonEnglishTranslated => "Non-English-translated",
            Self::Raw => "Raw",
        }
        .to_string()
    }
}

#[derive(from_num)]
pub enum LiveActionSubCategory {
    EnglishTranslated = 41,
    IdolPromotionalVideo = 42,
    NonEnglishTranslated = 43,
    Raw = 44,
}
impl Display for LiveActionSubCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EnglishTranslated => "English",
            Self::IdolPromotionalVideo => "Idol/PV",
            Self::NonEnglishTranslated => "Non-English",
            Self::Raw => "Raw",
        })
    }
}
impl FancySubCategory for LiveActionSubCategory {
    fn fancy(&self) -> String {
        match self {
            Self::EnglishTranslated => "English-translated",
            Self::IdolPromotionalVideo => "Idol/Promotional Video",
            Self::NonEnglishTranslated => "Non-English-translated",
            Self::Raw => "Raw",
        }
        .to_string()
    }
}

#[derive(from_num)]
pub enum PicturesSubCategory {
    Graphics = 51,
    Photos = 52,
}
impl Display for PicturesSubCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Graphics => "Graphics",
            Self::Photos => "Photos",
        })
    }
}
impl FancySubCategory for PicturesSubCategory {
    fn fancy(&self) -> String {
        self.to_string()
    }
}

#[derive(from_num)]
pub enum SoftwareSubCategory {
    Applications = 61,
    Games = 62,
}
impl Display for SoftwareSubCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Applications => "Apps",
            Self::Games => "Games",
        })
    }
}
impl FancySubCategory for SoftwareSubCategory {
    fn fancy(&self) -> String {
        match self {
            Self::Applications => "Applications",
            Self::Games => "Games",
        }
        .to_string()
    }
}
