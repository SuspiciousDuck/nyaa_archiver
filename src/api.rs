use std::path::PathBuf;

use actix_files as fs;
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, HttpServer, Result};

use diesel::expression::expression_types::NotSelectable;
use diesel::expression::AsExpression;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel::sqlite::Sqlite;

use chrono::prelude::*;

use askama::Template;
use magnet_url::MagnetBuilder;
use serde::Deserialize;
use thiserror::Error;

use tracker_lib::schema::torrents::{dsl as tor_dsl, table as tor_table};
use tracker_lib::util::search_torrent;
use tracker_lib::{establish_connection, models};

type BoolExpr = Box<dyn BoxableExpression<tor_table, Sqlite, SqlType = Bool>>;

const SITE_NAME: &str = "nyaa_archiver";

#[derive(Template)]
#[template(path = "error.html", blocks = ["title"])]
struct ErrorTemplate;

#[derive(Debug, Error)]
enum ServerError {
    #[error("Askama encountered an error!: {0}")]
    Askama(askama::Error),
    #[error("An invalid parameter was supplied!: {0}")]
    InvalidParam(String),
    #[error("An error occurred in diesel!: {0}")]
    Diesel(diesel::result::Error),
    #[error("Failed to get DateTime from UNIX timestamp for torrent {0}!")]
    DateTime(usize),
}
impl actix_web::error::ResponseError for ServerError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let template = ErrorTemplate;
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(template.render().unwrap())
    }

    fn status_code(&self) -> actix_web::http::StatusCode {
        match *self {
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
impl From<askama::Error> for ServerError {
    fn from(value: askama::Error) -> Self {
        Self::Askama(value)
    }
}
impl From<diesel::result::Error> for ServerError {
    fn from(value: diesel::result::Error) -> Self {
        Self::Diesel(value)
    }
}

#[derive(Template)]
#[template(path = "404.html", blocks = ["title"])]
struct NotFoundTemplate;

async fn not_found() -> Result<web::Html, ServerError> {
    let test = NotFoundTemplate;
    Ok(web::Html::new(test.render()?))
}

#[derive(Clone, Default, Deserialize)]
enum Order {
    Ascending,
    #[default]
    Descending,
}
impl Order {
    fn nick(&self) -> String {
        match self {
            Self::Ascending => "asc",
            Self::Descending => "desc",
        }
        .to_string()
    }

    fn flip(&self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

#[derive(Clone, Default, Deserialize, PartialEq)]
enum Sort {
    #[default]
    Date,
    Size,
    Comments,
    Seeders,
    Leechers,
    Completed,
}
impl Sort {
    fn expr(
        &self,
        order: &Order,
    ) -> Box<dyn BoxableExpression<tor_table, Sqlite, SqlType = NotSelectable>> {
        match (self, order) {
            (Self::Date, Order::Ascending) => Box::new(tor_dsl::date.asc()),
            (Self::Date, Order::Descending) => Box::new(tor_dsl::date.desc()),
            (Self::Size, Order::Ascending) => Box::new(tor_dsl::size.asc()),
            (Self::Size, Order::Descending) => Box::new(tor_dsl::size.desc()),
            (Self::Comments, Order::Ascending) => Box::new(tor_dsl::comments.asc()),
            (Self::Comments, Order::Descending) => Box::new(tor_dsl::comments.desc()),
            (Self::Seeders, Order::Ascending) => Box::new(tor_dsl::seeders.asc()),
            (Self::Seeders, Order::Descending) => Box::new(tor_dsl::seeders.desc()),
            (Self::Leechers, Order::Ascending) => Box::new(tor_dsl::leechers.asc()),
            (Self::Leechers, Order::Descending) => Box::new(tor_dsl::leechers.desc()),
            (Self::Completed, Order::Ascending) => Box::new(tor_dsl::completed.asc()),
            (Self::Completed, Order::Descending) => Box::new(tor_dsl::completed.desc()),
        }
    }
}

#[derive(Clone, Default, Deserialize)]
enum Filter {
    #[default]
    None,
    NoRemakes,
    Trusted,
}
impl Filter {
    fn expr(&self) -> BoolExpr {
        match self {
            Self::None => Box::new(<bool as AsExpression<Bool>>::as_expression(true)),
            Self::NoRemakes => Box::new(tor_dsl::remake.eq(false)),
            Self::Trusted => Box::new(tor_dsl::trusted.eq(true)),
        }
    }
}

fn category_to_expr(category: u8) -> BoolExpr {
    let cat = category as i32;
    if category == 0 {
        Box::new(<bool as AsExpression<Bool>>::as_expression(true))
    } else if category.is_multiple_of(10) {
        Box::new(
            tor_dsl::category
                .gt(cat)
                .and(tor_dsl::category.lt(cat + 10)),
        )
    } else {
        Box::new(tor_dsl::category.eq(cat))
    }
}

#[derive(Clone)]
struct BasicTorrent {
    id: usize,
    category: u8,
    title: String,
    torrent: Option<PathBuf>,
    magnet: Option<String>,
    size: String,
    date: usize,
    time: String,
    seeders: usize,
    leechers: usize,
    completed: usize,
    comments: usize,
    remake: bool,
    trusted: bool,
}
impl TryFrom<&models::Torrent> for BasicTorrent {
    type Error = ServerError;

    fn try_from(value: &models::Torrent) -> std::result::Result<Self, Self::Error> {
        let size = byte_unit::Byte::from_u64(value.size as u64)
            .get_appropriate_unit(byte_unit::UnitType::Binary); // size attribute is planned to be
                                                                // validated and accurate
        let torrent_exists = PathBuf::from(format!("./torrents/{}.torrent", value.id))
            .try_exists()
            .is_ok_and(|e| e);
        let torrent = if torrent_exists {
            Some(PathBuf::from(format!("/download/{}.torrent", value.id)))
        } else {
            None
        };
        // TODO: make info_hash not optional in database
        let magnet = value.info_hash.as_ref().map(|hash| {
            MagnetBuilder::new()
                .hash(hash)
                .hash_type("btih")
                .length(value.size as u64)
                .display_name(&value.title)
                .build()
                .to_string()
        });

        Ok(Self {
            id: value.id as usize,
            category: value.category as u8,
            title: value.title.clone(),
            torrent,
            magnet,
            size: format!("{size:.1}"),
            date: value.date as usize,
            time: DateTime::from_timestamp_secs(value.date as i64)
                .ok_or(ServerError::DateTime(value.id as usize))?
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            seeders: value.seeders as usize,
            leechers: value.leechers as usize,
            completed: value.completed as usize,
            comments: value.comments as usize,
            remake: value.remake,
            trusted: value.trusted,
        })
    }
}
impl BasicTorrent {
    fn row_class(&self) -> String {
        if self.remake {
            "danger"
        } else if self.trusted {
            "success"
        } else {
            "default"
        }
        .to_string()
    }
}

struct SearchResults {
    torrents: Vec<BasicTorrent>,
    total: usize,
    pages: usize,
    query: String,
    sort: Sort,
    order: Order,
    filter: Filter,
}
impl SearchResults {
    fn header_classes(&self, base_class: &str, target_sort: Option<Sort>, center: bool) -> String {
        let mut classes = vec![base_class.to_string()];

        if target_sort.as_ref().is_some_and(|s| self.sort == *s) {
            classes.push(format!("sorting_{}", self.order.nick()));
        } else if target_sort.is_some() {
            classes.push("sorting".to_string());
        }

        if center {
            classes.push("text-center".to_string());
        }

        classes.join(" ")
    }

    fn next_order(&self, target_sort: Sort) -> String {
        if self.sort == target_sort {
            self.order.flip().nick()
        } else {
            Order::default().nick()
        }
    }
}

#[derive(Template)]
#[template(path = "home.html", blocks = ["title"])]
struct HomeTemplate {
    results: SearchResults,
}

#[derive(Deserialize)]
struct SearchParams {
    page: Option<String>,
    query: Option<String>,
    category: Option<u8>,
    filter: Option<Filter>,
    sort: Option<Sort>,
    order: Option<Order>,
}
impl SearchParams {
    fn get_results(&self) -> Result<Vec<BasicTorrent>, ServerError> {
        let page = self.page.clone().unwrap_or("1".to_string());
        let page = if page == "rss" {
            1
        } else {
            page.parse::<i64>()
                .map_err(|_| ServerError::InvalidParam("page param not a number".to_string()))?
        };
        let filter = self.filter.clone().unwrap_or_default();
        let sort = self.sort.clone().unwrap_or_default();
        let order = self.order.clone().unwrap_or_default();
        let category = self.category.unwrap_or(0);

        let conn = &mut establish_connection();
        let expr = sort.expr(&order);
        let results = self
            .query
            .as_ref()
            .map(|q| search_torrent(q))
            .unwrap_or(tor_dsl::torrents.into_boxed())
            .offset((page - 1) * 75)
            .limit(75)
            .filter(filter.expr())
            .filter(category_to_expr(category))
            .order(expr)
            .select(models::Torrent::as_select())
            .load(conn)?;

        let (torrents, errors): (Vec<_>, _) = results
            .iter()
            .map(BasicTorrent::try_from)
            .partition(|x| x.as_ref().is_ok());

        if let Some(error) = errors.into_iter().next() {
            Err(error.err().unwrap())
        } else {
            Ok(torrents.into_iter().map(Result::unwrap).collect())
        }
    }

    fn get_stats(&self) -> Result<(usize, usize), ServerError> {
        let filter = self.filter.clone().unwrap_or_default();
        let category = self.category.unwrap_or(0);

        let conn = &mut establish_connection();
        let results: i64 = self
            .query
            .as_ref()
            .map(|q| search_torrent(q))
            .unwrap_or(tor_dsl::torrents.into_boxed())
            .limit(1000)
            .filter(filter.expr())
            .filter(category_to_expr(category))
            .count()
            .get_result(conn)?;

        let pages = (results as usize).div_ceil(75);
        Ok((results as usize, pages))
    }
}

#[get("/")]
async fn home(params: web::Query<SearchParams>) -> Result<web::Html, ServerError> {
    let torrents = params.get_results()?;
    let (total, pages) = params.get_stats()?;

    let test = HomeTemplate {
        results: SearchResults {
            torrents,
            total,
            pages,
            query: params.query.clone().unwrap_or_default(),
            sort: Sort::default(),
            order: Order::default(),
            filter: Filter::default(),
        },
    };
    Ok(web::Html::new(test.render()?))
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
            .service(home)
            .service(fs::Files::new("/", "./frontend"))
            .service(fs::Files::new("/download", "./torrents"))
            .default_service(web::route().to(not_found))
    })
    .bind(("0.0.0.0", 11000))?
    .run()
    .await
}
