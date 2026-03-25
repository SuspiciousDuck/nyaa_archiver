use actix_files::{self as fs, NamedFile};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use chrono::prelude::*;
use derive_more::derive::{Display, Error};
use diesel::expression::expression_types::NotSelectable;
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;
use lava_torrent::torrent::v1::Torrent as LavaTorrent;
use magnet_url::MagnetBuilder;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracker_lib::establish_connection;
use tracker_lib::models::{self, Category};
use tracker_lib::schema::torrents::dsl as tor_dsl;
use tracker_lib::schema::torrents::table as tor_table;
use tracker_lib::util::search_torrent;

const LINK: &str = "https://nyaa.si/"; // change later?
const ANNOUNCE: &str = "http://xyz.b32.i2p:10999";

#[derive(Debug, Display, Error)]
enum FuckingError {
    #[display("An internal error occurred. Please try again later.")]
    InternalError,
}
impl actix_web::error::ResponseError for FuckingError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }
    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            FuckingError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
struct RssGuid {
    #[serde(rename = "@isPermaLink")]
    permalink: bool,
    #[serde(rename = "#text")]
    contents: String,
}

#[derive(Serialize, Clone, Debug)]
struct RssItem {
    title: String,
    link: String,
    guid: RssGuid,
    #[serde(rename = "pubDate")]
    pub_date: String,
    #[serde(rename = "nyaa:seeders")]
    seeders: usize,
    #[serde(rename = "nyaa:leechers")]
    leechers: usize,
    #[serde(rename = "nyaa:downloads")]
    downloads: usize,
    #[serde(rename = "nyaa:infoHash")]
    info_hash: String,
    #[serde(rename = "nyaa:categoryId")]
    category_id: String,
    #[serde(rename = "nyaa:category")]
    category: String,
    #[serde(rename = "nyaa:size")]
    size: String,
    #[serde(rename = "nyaa:comments")]
    comments: usize,
    #[serde(rename = "nyaa:trusted")]
    trusted: String,
    #[serde(rename = "nyaa:remake")]
    remake: String,
    description: String,
}
impl TryFrom<&models::Torrent> for RssItem {
    type Error = String;
    fn try_from(value: &models::Torrent) -> Result<Self, Self::Error> {
        if value.partial {
            return Err("Cannot create RssItem from a partial Torrent!".to_string());
        }
        let date = DateTime::from_timestamp(value.date as i64, 0)
            .ok_or("Failed to get DateTime from UNIX timestamp!")?;
        let torrent_path = PathBuf::from(format!("./torrents/{}.torrent", value.id));
        let torrent = LavaTorrent::read_from_file(&torrent_path)
            .map_err(|e| format!("lava_torrent failed to read torrent file!: {e}"))?;
        let size = byte_unit::Byte::from_u64(torrent.length as u64)
            .get_appropriate_unit(byte_unit::UnitType::Binary);
        let hash = torrent.info_hash();
        let mut category_id = value.category.to_string();
        category_id.insert(1, '_');
        let category = Category::from_u8(value.category as u8).unwrap();
        Ok(Self {
            title: value.title.clone(),
            link: format!("{LINK}download/{}.torrent", value.id),
            guid: RssGuid {
                permalink: true,
                contents: format!("{LINK}view/{}", value.id),
            },
            pub_date: date.format("%a, %d %B %Y %H:%M:%S -0000").to_string(),
            seeders: value.seeders as usize,
            leechers: value.leechers as usize,
            downloads: value.completed as usize,
            info_hash: hash.clone(),
            category_id: (category_id),
            category: category.fancy(),
            size: format!("{size:.1}"),
            comments: value.comments as usize,
            trusted: if value.trusted { "Yes" } else { "No" }.to_string(),
            remake: if value.remake { "Yes" } else { "No" }.to_string(),
            description: format!(
                "<a href=\"{LINK}view/{}\">#1974102 | {}</a> | {size:.1} | {} | {}",
                value.id,
                value.title,
                category.fancy(),
                hash.to_uppercase()
            ),
        })
    }
}

#[derive(Serialize, Clone, Debug)]
struct AtomLink {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@rel")]
    rel: String,
    #[serde(rename = "@type")]
    type_: String,
}

#[derive(Serialize, Clone, Debug)]
struct RssChannel {
    title: String,
    description: String,
    link: String,
    #[serde(rename = "atom:link")]
    atom_link: AtomLink,
    #[serde(rename = "item")]
    items: Vec<RssItem>,
}

impl Default for RssChannel {
    fn default() -> Self {
        Self {
            title: "Nyaa - Home - Torrent File RSS".to_string(),
            description: "RSS Feed for Home".to_string(),
            link: LINK.to_string(),
            atom_link: AtomLink {
                href: format!("{LINK}?page=rss"),
                rel: "self".to_string(),
                type_: "application/rss+xml".to_string(),
            },
            items: Vec::default(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename = "rss")]
struct Rss {
    #[serde(rename = "@version")]
    version: String,
    channel: RssChannel,
}

#[derive(Deserialize, Default, Clone, Debug, PartialEq)]
enum Order {
    Ascending,
    #[default]
    Descending,
}

#[derive(Deserialize, Default, Clone, Debug)]
enum Sort {
    #[default]
    Date,
    Size,
    Comments,
    Seeders,
    Leechers,
    Downloads,
}
impl Sort {
    fn expr(
        &self,
        order: &Order,
    ) -> Box<dyn BoxableExpression<tor_table, Sqlite, SqlType = NotSelectable>> {
        match self {
            Self::Date => {
                if order == &Order::Ascending {
                    Box::new(tor_dsl::date.asc())
                } else {
                    Box::new(tor_dsl::date.desc())
                }
            }
            Self::Size => {
                if order == &Order::Ascending {
                    Box::new(tor_dsl::size.asc())
                } else {
                    Box::new(tor_dsl::size.desc())
                }
            }
            Self::Comments => {
                if order == &Order::Ascending {
                    Box::new(tor_dsl::comments.asc())
                } else {
                    Box::new(tor_dsl::comments.desc())
                }
            }
            Self::Seeders => {
                if order == &Order::Ascending {
                    Box::new(tor_dsl::seeders.asc())
                } else {
                    Box::new(tor_dsl::seeders.desc())
                }
            }
            Self::Leechers => {
                if order == &Order::Ascending {
                    Box::new(tor_dsl::leechers.asc())
                } else {
                    Box::new(tor_dsl::leechers.desc())
                }
            }
            Self::Downloads => {
                if order == &Order::Ascending {
                    Box::new(tor_dsl::completed.asc())
                } else {
                    Box::new(tor_dsl::completed.desc())
                }
            }
        }
    }
}

#[derive(Deserialize, Default, Clone, Debug)]
enum Filter {
    #[default]
    None,
    NoRemakes,
    Trusted,
}
impl Filter {
    fn expr(&self) -> Box<dyn BoxableExpression<tor_table, Sqlite, SqlType = Bool>> {
        match self {
            Self::None => Box::new(<bool as AsExpression<Bool>>::as_expression(true)),
            Self::NoRemakes => Box::new(tor_dsl::remake.eq(false)),
            Self::Trusted => Box::new(tor_dsl::trusted.eq(true)),
        }
    }
}

fn cat_to_expr(cat: &usize) -> Box<dyn BoxableExpression<tor_table, Sqlite, SqlType = Bool>> {
    if cat == &0 {
        Box::new(<bool as AsExpression<Bool>>::as_expression(true))
    } else if cat.is_multiple_of(10) {
        Box::new(
            tor_dsl::category
                .gt(*cat as i32)
                .and(tor_dsl::category.lt(*cat as i32 + 10)),
        )
    } else {
        Box::new(tor_dsl::category.eq(*cat as i32))
    }
}

#[derive(Deserialize, Debug)]
struct TorrentRequest {
    id: usize,
}

#[derive(Deserialize, Debug)]
struct Page {
    page: usize,
    sort: Option<Sort>,
    order: Option<Order>,
    query: Option<String>,
    filter: Option<Filter>,
    category: Option<usize>,
}

#[derive(Deserialize, Debug)]
struct Pages {
    query: Option<String>,
    filter: Option<Filter>,
    category: Option<usize>,
}

#[derive(Serialize, Debug)]
struct NumPages {
    pages: usize,
    results: usize,
}

#[derive(Serialize, Clone, Debug)]
struct BasicTorrent {
    id: usize,
    category: u8,
    title: String,
    torrent: Option<PathBuf>,
    magnet: Option<String>,
    size: String,
    date: usize,
    seeders: usize,
    leechers: usize,
    completed: usize,
    comments: usize,
    remake: bool,
    trusted: bool,
}
impl TryFrom<&models::Torrent> for BasicTorrent {
    type Error = String;
    fn try_from(value: &models::Torrent) -> Result<Self, String> {
        let (torrent, magnet, size) = if value.partial {
            let size = byte_unit::Byte::from_u64(value.size as u64)
                .get_appropriate_unit(byte_unit::UnitType::Binary);
            let magnet = value.info_hash.as_ref().map(|hash| {
                MagnetBuilder::new()
                    .hash(hash)
                    .hash_type("btih")
                    .add_tracker(ANNOUNCE)
                    .length(value.size as u64)
                    .display_name(&value.title)
                    .build()
                    .to_string()
            });
            (None, magnet, size)
        } else {
            let torrent_path = PathBuf::from(format!("./torrents/{}.torrent", value.id));
            let torrent = LavaTorrent::read_from_file(&torrent_path)
                .map_err(|e| format!("lava_torrent failed to read torrent file!: {e}"))?;
            let magnet = torrent
                .magnet_link()
                .map_err(|e| format!("lava_torrent failed to create magnet link!: {e}"))?;
            let size = byte_unit::Byte::from_u64(torrent.length as u64)
                .get_appropriate_unit(byte_unit::UnitType::Binary);
            let torrent = PathBuf::from(format!("/download/{}.torrent", value.id));
            (Some(torrent), Some(magnet), size)
        };
        Ok(Self {
            id: value.id as usize,
            category: value.category as u8,
            title: value.title.clone(),
            torrent: (torrent),
            magnet: (magnet),
            size: format!("{size:.1}"),
            date: value.date as usize,
            seeders: value.seeders as usize,
            leechers: value.leechers as usize,
            completed: value.completed as usize,
            comments: value.comments as usize,
            remake: value.remake,
            trusted: value.trusted,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct File {
    pub length: String,
    pub path: PathBuf,
    pub parts: Vec<String>,
}
impl From<&lava_torrent::torrent::v1::File> for File {
    fn from(value: &lava_torrent::torrent::v1::File) -> Self {
        let size = byte_unit::Byte::from_u64(value.length as u64)
            .get_appropriate_unit(byte_unit::UnitType::Binary);
        log::info!("{:#?}", value.path.ancestors());
        Self {
            length: format!("{size:.1}"),
            path: value.path.clone(),
            parts: value
                .path
                .ancestors()
                .filter(|p| p.file_name().is_some() && p.is_dir())
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .collect(),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct Torrent {
    pub id: usize,
    pub info_hash: Option<String>,
    pub name: Option<String>,
    pub files: Vec<File>,
    pub seeders: usize,
    pub leechers: usize,
    pub completed: usize,
    pub title: String,
    pub torrent: Option<PathBuf>,
    pub magnet: Option<String>,
    pub category: u8,
    pub submitter: String,
    pub information: String,
    pub size: String,
    pub date: usize,
    pub description: String,
    pub comments: usize,
    pub remake: bool,
    pub trusted: bool,
    pub anonymous: bool,
    pub partial: bool,
}
impl TryFrom<&models::Torrent> for Torrent {
    type Error = String;
    fn try_from(value: &models::Torrent) -> Result<Self, String> {
        let (torrent, magnet, hash, size, files, name) = if value.partial {
            let size = byte_unit::Byte::from_u64(value.size as u64)
                .get_appropriate_unit(byte_unit::UnitType::Binary);
            let (magnet, hash) = if let Some(hash) = &value.info_hash {
                let magnet = MagnetBuilder::new()
                    .hash(hash)
                    .hash_type("btih")
                    .add_tracker(ANNOUNCE)
                    .length(value.size as u64)
                    .display_name(&value.title)
                    .build();
                (
                    Some(magnet.to_string()),
                    magnet.hash().map(|s| s.to_string()),
                )
            } else {
                (None, None)
            };
            (None, magnet, hash, size, Vec::new(), None)
        } else {
            let torrent_path = PathBuf::from(format!("./torrents/{}.torrent", value.id));
            let torrent = LavaTorrent::read_from_file(&torrent_path)
                .map_err(|e| format!("lava_torrent failed to read torrent file!: {e}"))?;
            let size = byte_unit::Byte::from_u64(torrent.length as u64)
                .get_appropriate_unit(byte_unit::UnitType::Binary);
            let magnet = torrent
                .magnet_link()
                .map_err(|e| format!("lava_torrent failed to create magnet link!: {e}"))?;
            let hash = torrent.info_hash();
            let name = torrent.name.clone();
            let files = torrent
                .files
                .clone()
                .map(|f| f.iter().map(|f| f.into()).collect())
                .unwrap_or_default();
            let torrent = PathBuf::from(format!("/download/{}.torrent", value.id));
            (
                Some(torrent),
                Some(magnet),
                Some(hash),
                size,
                files,
                Some(name),
            )
        };

        Ok(Self {
            id: value.id as usize,
            info_hash: hash,
            name: (name),
            files: (files),
            seeders: value.seeders as usize,
            leechers: value.leechers as usize,
            completed: value.completed as usize,
            title: value.title.clone(),
            torrent: (torrent),
            magnet: (magnet),
            category: value.category as u8,
            submitter: value.submitter.clone().unwrap_or("Anonymous".to_string()),
            information: value.information.clone().unwrap_or_default(),
            size: format!("{size:.1}"),
            date: value.date as usize,
            description: value.description.clone().unwrap_or_default(),
            comments: value.comments as usize,
            remake: value.remake,
            trusted: value.trusted,
            anonymous: value.anonymous,
            partial: value.partial,
        })
    }
}

#[derive(Debug, serde::Serialize)]
struct CommentObj {
    id: i32,
    torrent_id: i32,
    submitter: String,
    date_created: i32,
    date_edited: Option<i32>,
    text: String,
    default_pfp: bool,
}

#[get("/api/torrent")]
async fn get_torrent(id: web::Query<TorrentRequest>) -> Result<web::Json<Torrent>, FuckingError> {
    let id = id.id;
    let connection = &mut establish_connection();
    let torrent = tracker_lib::get_torrent(connection, &id);
    if torrent.as_ref().is_none() {
        log::error!("Torrent id {id} not found!");
        return Err(FuckingError::InternalError);
    }
    let torrent = &torrent.unwrap();
    let torrent = torrent.try_into();
    if torrent.as_ref().is_err() {
        log::error!("{}", torrent.err().unwrap());
        return Err(FuckingError::InternalError);
    }
    Ok(web::Json(torrent.unwrap()))
}

// 75 torrents per page
#[get("/api/")]
async fn get_page(page: web::Query<Page>) -> Result<web::Json<Vec<BasicTorrent>>, FuckingError> {
    let page_num = page.page as i64;
    let sort = page.sort.to_owned().unwrap_or_default();
    let order = page.order.to_owned().unwrap_or_default();
    let query = page.query.as_ref();
    let filter = page.filter.to_owned().unwrap_or_default();
    let category = page.category.unwrap_or(0);
    let conn = &mut establish_connection();
    let results = if query.as_ref().is_some() {
        search_torrent(query.unwrap())
    } else {
        tor_dsl::torrents.into_boxed()
    };
    let expr = sort.expr(&order);
    let results = results
        .offset(page_num * 75)
        .limit(75)
        .filter(filter.expr())
        .filter(cat_to_expr(&category))
        .order(expr)
        .select(models::Torrent::as_select())
        .load(conn);

    if results.is_err() {
        log::error!("Error loading torrents!: {}", results.err().unwrap());
        return Err(FuckingError::InternalError);
    }
    let results = results.unwrap();
    let results: Vec<Result<BasicTorrent, String>> =
        results.iter().map(BasicTorrent::try_from).collect();
    let mut errored = results.iter().filter(|t| t.is_err());
    let err = errored.next();
    if let Some(err) = err {
        let err = err.clone().err().unwrap();
        log::error!("Failed to get information from one or more torrents!: {err}");
        Err(FuckingError::InternalError)
    } else {
        Ok(web::Json(
            results.iter().map(|t| t.clone().unwrap()).collect(),
        ))
    }
}

// number of pages
#[get("/api/pages")]
async fn get_pages(query: web::Query<Pages>) -> Result<web::Json<NumPages>, FuckingError> {
    let conn = &mut establish_connection();
    let filter = query.filter.to_owned().unwrap_or_default();
    let category = query.category.unwrap_or(0);
    let query = &query.query;
    let results = query
        .as_ref()
        .map(|q| search_torrent(q))
        .unwrap_or(tor_dsl::torrents.into_boxed())
        .limit(1000)
        .filter(filter.expr())
        .filter(cat_to_expr(&category))
        .count()
        .get_result(conn);
    if results.is_err() {
        log::error!("Error counting torrents!: {}", results.err().unwrap());
        return Err(FuckingError::InternalError);
    }
    let results: i64 = results.unwrap();
    let pages = (results as f64 / 75f64).floor() + if results % 75 == 0 { 0f64 } else { 1f64 };
    Ok(web::Json(NumPages {
        pages: pages as usize,
        results: results as usize,
    }))
}

#[get("/api/comments")]
async fn get_comments(
    query: web::Query<TorrentRequest>,
) -> Result<web::Json<Vec<CommentObj>>, FuckingError> {
    let conn = &mut establish_connection();
    let id = query.id;
    let results = tracker_lib::get_torrent_comments(conn, &id);
    let results = results
        .iter()
        .map(|result| CommentObj {
            id: result.id,
            torrent_id: result.torrent_id,
            submitter: result.submitter.clone(),
            date_created: result.date_created,
            date_edited: result.date_edited,
            text: result.text.clone(),
            default_pfp: !PathBuf::from(format!("./pfps/{}.png", &result.submitter))
                .try_exists()
                .is_ok_and(|b| b),
        })
        .collect::<Vec<CommentObj>>();
    Ok(web::Json(results))
}

#[derive(Deserialize)]
struct RssPage {
    page: Option<String>,
}

#[get("/")]
async fn serve_rss(_rq: HttpRequest, page: web::Query<RssPage>) -> impl Responder {
    let page = &page.page;
    if page.as_ref().is_none_or(|p| p != "rss") {
        let path = PathBuf::from("./frontend/index.html");
        return NamedFile::open(path).unwrap().into_response(&_rq);
    }
    let conn = &mut establish_connection();
    let results = tor_dsl::torrents
        .order(tor_dsl::date.desc())
        .limit(75)
        .filter(tor_dsl::partial.eq(false))
        .select(models::Torrent::as_select())
        .load(conn)
        .unwrap();
    let results = results
        .iter()
        .map(|t| RssItem::try_from(t).unwrap())
        .collect();
    let xml = Rss {
        version: "2.0".to_string(),
        channel: RssChannel {
            items: results,
            ..Default::default()
        },
    };
    let config = serde_xml_rs::SerdeXml::new()
        .namespace("atom", "http://www.w3.org/2005/Atom")
        .namespace("nyaa", &format!("{LINK}xmlns/nyaa"));
    HttpResponse::Ok()
        .content_type("application/xml")
        .body(config.to_string(&xml).unwrap())
}

async fn serve_view(_rq: HttpRequest, _num: web::Path<usize>) -> Result<NamedFile> {
    let path = PathBuf::from("./frontend/torrent.html");
    Ok(NamedFile::open(path)?)
}

async fn rss_namespace(_rq: HttpRequest) -> Result<NamedFile> {
    let path = PathBuf::from("./frontend/rss.html");
    Ok(NamedFile::open(path)?)
}

async fn rules(_rq: HttpRequest) -> Result<NamedFile> {
    let path = PathBuf::from("./frontend/rules.html");
    Ok(NamedFile::open(path)?)
}

async fn help(_rq: HttpRequest) -> Result<NamedFile> {
    let path = PathBuf::from("./frontend/help.html");
    Ok(NamedFile::open(path)?)
}

async fn register(_rq: HttpRequest) -> Result<NamedFile> {
    let path = PathBuf::from("./frontend/register.html");
    Ok(NamedFile::open(path)?)
}

async fn login(_rq: HttpRequest) -> Result<NamedFile> {
    let path = PathBuf::from("./frontend/login.html");
    Ok(NamedFile::open(path)?)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    unsafe {
        std::env::set_var("RUST_LOG", "info");
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    env_logger::init();

    HttpServer::new(|| {
        let logger = Logger::default();

        App::new()
            .wrap(logger)
            .wrap(actix_web::middleware::Compress::default())
            .service(get_torrent)
            .service(get_comments)
            .service(get_page)
            .service(get_pages)
            .service(serve_rss)
            .route("/view/{number}", web::get().to(serve_view))
            .route("/rules", web::get().to(rules))
            .route("/help", web::get().to(help))
            .route("/register", web::get().to(register))
            .route("/login", web::get().to(login))
            .route("/xmlns/nyaa", web::get().to(rss_namespace))
            .service(fs::Files::new("/pfps/", "./pfps"))
            .service(fs::Files::new("/download/", "./torrents"))
            .service(fs::Files::new("/", "./frontend").index_file("index.html"))
    })
    .bind(("0.0.0.0", 11000))?
    .run()
    .await
}
