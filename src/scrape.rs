use crate::models::{DatabaseError, NewComment, NewTorrent, User};
use crate::{comment_exists, get_torrent, get_torrent_comments, mark_torrent_deleted, user_exists};
use arti_ureq::arti_client::{TorClient, TorClientConfig};
use arti_ureq::tor_rtcompat::tokio::TokioRustlsRuntime;
use arti_ureq::ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use arti_ureq::ureq::Agent;
use diesel::SqliteConnection;
use flate2::read::GzDecoder;
use lava_torrent::torrent::v1::Torrent as LavaTorrent;
use lava_torrent::LavaTorrentError;
use magnet_url::{Magnet, MagnetError};
use scraper::{ElementRef, Html, Selector};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, SystemTime, SystemTimeError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScrapeError {
    #[error("Failed to create ipc channel!: {0}")]
    CreateIpcChannel(std::io::Error),
    #[error("Failed to create Tokio Rustls runtime!: {0}")]
    CreateRuntime(std::io::Error),
    #[error("Failed to create TorClient!: {0}")]
    CreateTorClient(arti_ureq::arti_client::Error),
    #[error("SystemTime encountered an unexpected error!: {0}")]
    Clock(SystemTimeError), // error string
    #[error("Arti client encountered an unexpected error!: {0}")]
    Arti(arti_ureq::Error), // error string
    #[error("Failed to send request to {0:?}!: {1}")]
    Request(String, arti_ureq::ureq::Error), // url, error string
    #[error("Response from {0:?} received a {1} status code!")]
    Response(String, u16), // url, response status
    #[error("Failed to read the bytes of the response from {0:?}!: {1}")]
    ResponseIO(String, arti_ureq::ureq::Error),
    #[error("Failed to gzip decode response!: {0}")]
    Gzip(std::io::Error), // error string
    #[error("Failed to parse {0:?}'s {1} response!")]
    BadResponse(String, u16), // url, response status
    #[error("lava_torrent failed to parse {0:#?}!: {1}")]
    LavaTorrent(PathBuf, LavaTorrentError), // file path, error string
    #[error("magnet_url failed to parse magnet link!: {0}")]
    MagnetError(MagnetError),
    #[error("The torrent {0} has been deleted from nyaa!")]
    TorrentDeleted(usize),
    #[error("The torrent {0} has no .torrent file uploaded yet!")]
    TorrentMissing(usize),
    #[error("Selector {1:?} had no matches on {0:?}")]
    Selector(String, String), // url, selector
    #[error("Attribute {0:?} is missing!")]
    MissingAttribute(String), // attribute name
    #[error("Failed to parse attribute {0:?}!: {1:?}")]
    BadAttribute(String, String), // attribute name, error string
    #[error("Database encountered an unexpected error!: {0}")]
    Database(DatabaseError), // error string
    #[error("Failed to create file at {0:#?}!: {1}")]
    CreateFile(PathBuf, std::io::Error), // file path
    #[error("Failed to create file at {0:#?}!: {1}")]
    CreateDirectory(PathBuf, std::io::Error), // dir path
    #[error("Failed to open file {0:#?}!: {1}")]
    WriteFile(PathBuf, std::io::Error), // file path
    #[error("Failed to write to file {0:#?}!: {1}")]
    OpenFile(PathBuf, std::io::Error), // file path
    #[error("Failed to get info hash for the torrent {0}!")]
    InfoHash(usize),
    #[error("Failed to get size for the torrent {0}!")]
    Size(usize),
    #[error("Failed to parse the avatar id from its url {0:?}!")]
    AvatarUrl(String),
}
impl From<SystemTimeError> for ScrapeError {
    fn from(value: SystemTimeError) -> Self {
        Self::Clock(value)
    }
}
impl From<arti_ureq::Error> for ScrapeError {
    fn from(value: arti_ureq::Error) -> Self {
        Self::Arti(value)
    }
}
impl From<MagnetError> for ScrapeError {
    fn from(value: MagnetError) -> Self {
        Self::MagnetError(value)
    }
}
impl From<DatabaseError> for ScrapeError {
    fn from(value: DatabaseError) -> Self {
        Self::Database(value)
    }
}

struct ParsedTorrent {
    title: String,
    category: usize,
    submitter: Option<String>,
    info: Option<String>,
    date: usize,
    desc: Option<String>,
    remake: bool,
    trusted: bool,
    anonymous: bool,
    hidden: bool,
}

#[derive(Clone, Debug)]
pub struct ParsedPartialTorrent {
    pub id: usize,
    pub title: String,
    pub category: usize,
    pub magnet: Magnet,
    pub size: usize,
    pub date: usize,
    pub comments: usize,
    pub remake: bool,
    pub trusted: bool,
}

struct ParsedCommentUser {
    username: String,
    trusted: bool,
    banned: bool,
    admin: bool,
    moderator: bool,
    avatar: Option<String>,
}

struct ParsedComment {
    comment_id: usize,
    torrent_id: usize,
    submitter: ParsedCommentUser,
    date: usize,
    content: String,
}

/// returns next_update, update_frequency
/// args: new, differs, date, update_frequency
type UpdateAction = fn(bool, bool, i64, Option<i32>) -> (i64, i32);

#[derive(Clone)]
pub struct Client {
    pub latest_request: SystemTime,
    pub runtime: TokioRustlsRuntime,
    pub agent: Agent,
}
impl Client {
    pub fn new() -> Result<Self, ScrapeError> {
        let runtime = TokioRustlsRuntime::create().map_err(ScrapeError::CreateRuntime)?;
        let agent = Client::generate(runtime.clone())?;
        Ok(Self {
            latest_request: SystemTime::UNIX_EPOCH,
            runtime: (runtime),
            agent: (agent),
        })
    }

    pub fn generate(runtime: TokioRustlsRuntime) -> Result<Agent, ScrapeError> {
        println!("Creating new circuit...");
        let client = TorClient::with_runtime(runtime.clone())
            .config(TorClientConfig::default())
            .create_unbootstrapped()
            .map_err(ScrapeError::CreateTorClient)?;
        let connector = arti_ureq::ConnectorBuilder::with_runtime(runtime)?
            .tor_client(client)
            .tls_provider(TlsProvider::Rustls);
        let tls_config = TlsConfig::builder()
            .root_certs(RootCerts::WebPki)
            .provider(TlsProvider::Rustls)
            .build();
        let config = arti_ureq::ureq::Agent::config_builder()
            .tls_config(tls_config)
            .http_status_as_error(false)
            .accept_encoding("gzip")
            .user_agent("Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0")
            .build();
        let agent = connector.build()?.agent_with_ureq_config(config)?;
        Ok(agent)
    }

    pub fn get(&mut self, url: &str, utf8: bool) -> Result<Vec<u8>, ScrapeError> {
        let since = SystemTime::now().duration_since(self.latest_request)?;
        if since.as_secs() < 5 {
            // according to robots.txt
            let duration = 5000 - since.as_millis() as u64;
            eprintln!("Sent requests too quickly! Waiting {duration} milliseconds.");
            sleep(Duration::from_millis(duration));
        }
        println!("Making request to: {url}");
        let request = self.agent.get(url).call();
        if let Some(arti_ureq::ureq::Error::Io(error)) = request.as_ref().err() {
            eprintln!("Request resulted in IO error!: {error}\nRetrying...");
            self.agent = Client::generate(self.runtime.clone())?;
            return self.get(url, utf8);
        }
        let mut request = request.map_err(|e| ScrapeError::Request(url.to_string(), e))?;
        let gzip = request
            .headers()
            .get("content-encoding")
            .is_some_and(|s| s == "gzip");
        let status = request.status().as_u16();
        if request.status().is_success() {
            let response = request
                .body_mut()
                .with_config()
                .limit(50 * 1024 * 1024)
                .read_to_vec()
                .map_err(|e| ScrapeError::ResponseIO(url.to_string(), e))?;
            let response = if gzip {
                let mut buf = Vec::new();
                GzDecoder::new(response.as_slice())
                    .read_to_end(&mut buf)
                    .map_err(ScrapeError::Gzip)?;
                buf
            } else {
                response
            };
            if !(utf8 && String::from_utf8(response.clone()).is_err()) {
                self.latest_request = SystemTime::now();
                return Ok(response);
            }
            return Err(ScrapeError::BadResponse(url.to_string(), status));
        }
        match status {
            429 | 504 => {
                eprintln!("Recieved status {status}! Retrying...");
                self.agent = Client::generate(self.runtime.clone())?;
                self.get(url, utf8)
            }
            _ => Err(ScrapeError::Response(url.to_string(), status)),
        }
    }

    pub fn download(&mut self, url: &str, path: &PathBuf) -> Result<(), ScrapeError> {
        let response = self.get(url, false)?;
        let mut torrent =
            File::create(path).map_err(|e| ScrapeError::CreateFile(path.clone(), e))?;
        torrent
            .write_all(&response)
            .map_err(|e| ScrapeError::WriteFile(path.clone(), e))?;
        Ok(())
    }

    pub fn scrape_page(
        &mut self,
        connection: &mut SqliteConnection,
        page: usize,
        flags: &str,
        deep: bool,
        update_action: UpdateAction,
    ) -> Result<(Vec<ParsedPartialTorrent>, Vec<usize>), ScrapeError> {
        let target = format!("https://nyaa.si/?p={page}{flags}");
        let response = String::from_utf8(self.get(&target, true)?).unwrap();
        let dom = Html::parse_document(&response);
        let torrent_selector = Selector::parse(".table-responsive>table>tbody>tr").unwrap();
        let torrent_elements = dom.select(&torrent_selector).collect::<Vec<ElementRef>>();
        let mut torrents = Vec::new();
        let mut failed_torrents = Vec::new();
        let mut new_torrents = 0;

        for torrent in &torrent_elements {
            let torrent = match parse_partial_torrent(torrent, &target) {
                Err(ScrapeError::TorrentMissing(id)) => {
                    failed_torrents.push(id);
                    continue;
                }
                result => result,
            }?;
            torrents.push(torrent.clone());
            match (get_torrent(connection, torrent.id), deep) {
                (Some(_), _) => continue,
                (None, true) => {
                    match self.scrape_torrent(connection, torrent.id as usize, update_action) {
                        Err(ScrapeError::TorrentMissing(id)) => {
                            failed_torrents.push(id);
                            continue;
                        }
                        Err(ScrapeError::TorrentDeleted(id)) => {
                            mark_torrent_deleted(connection, id)?;
                            continue;
                        }
                        result => result,
                    }?;
                    continue;
                }
                _ => (),
            }
            let hash = torrent
                .magnet
                .hash()
                .ok_or(ScrapeError::InfoHash(torrent.id))?;
            let size = torrent
                .magnet
                .length()
                .ok_or(ScrapeError::Size(torrent.id))?; // we could use torrent.size but we want
                                                        // the database to be accurate
            let (next_update, update_frequency) =
                update_action(true, true, torrent.date as i64, None);
            let torrent = NewTorrent {
                id: Some(torrent.id as i32),
                info_hash: hash.to_string(),
                title: torrent.title,
                category: torrent.category as i32,
                submitter: None,
                information: None,
                size: size as i64,
                date: torrent.date as i64,
                description: None,
                comments: torrent.comments as i32,
                remake: torrent.remake,
                trusted: torrent.trusted,
                anonymous: true,
                partial: true,
                deleted: false,
                hidden: false,
                next_update: Some(next_update),
                update_count: 0,
                update_frequency: Some(update_frequency),
            };
            torrent.insert(connection, false)?;
            new_torrents += 1;
        }
        if new_torrents == 0 && !deep && !torrent_elements.is_empty() {
            eprintln!("Warning! Torrent list is empty!");
        }
        Ok((torrents, failed_torrents))
    }

    pub fn scrape_torrent(
        &mut self,
        connection: &mut SqliteConnection,
        id: usize,
        update_action: UpdateAction,
    ) -> Result<(), ScrapeError> {
        // HTML parsing
        let target = format!("https://nyaa.si/view/{id}");
        let response = self.get(&target, true);
        if let Some(ScrapeError::Response(_, 404)) = response.as_ref().err() {
            return Err(ScrapeError::TorrentDeleted(id));
        }
        let response = String::from_utf8(response?).unwrap();
        let dom = Html::parse_document(&response);
        let parsed = parse_torrent(&dom.root_element(), id)?;

        if !parsed.anonymous && !user_exists(connection, parsed.submitter.as_ref().unwrap()) {
            User {
                username: parsed.submitter.clone().unwrap(),
                password: None,
                salt: None,
                email: None,
                nyaa: true,
                trusted: parsed.trusted,
                banned: false, // TODO: is it even possible to tell?
                nyaa_admin: false,
                nyaa_mod: false,
                avatar: None,
            }
            .insert(connection, false)?;
        }

        // Comments
        let comments_selector = Selector::parse("#collapse-comments>div").unwrap();
        let comment_elements = dom.select(&comments_selector);
        let mut comments = Vec::new();
        for comment in comment_elements {
            let comment = parse_comment(&comment, id)?;
            comments.push(comment);
        }

        // Torrent file
        let target = format!("https://nyaa.si/download/{id}.torrent");
        let path = PathBuf::from(format!("./torrents/{id}.torrent"));
        if !path.try_exists().is_ok_and(|b| b) {
            match self.download(&target, &path) {
                Err(ScrapeError::Response(_, 404)) => Err(ScrapeError::TorrentMissing(id)),
                result => result,
            }?;
        }
        let torrent_path = PathBuf::from(format!("./torrents/{id}.torrent"));
        let torrent = LavaTorrent::read_from_file(&torrent_path)
            .map_err(|e| ScrapeError::LavaTorrent(torrent_path, e))?;
        let hash = torrent.info_hash();
        let size = torrent.length as usize;

        let comments_num = get_torrent(connection, id)
            .map(|t| {
                if t.partial {
                    0
                } else {
                    get_torrent_comments(connection, id).len() as i32 // just in case a torrent has
                                                                      // mismatched comment counts
                }
            })
            .unwrap_or(0);

        let ((next_update, update_frequency), update_count) =
            if let Some(old_torrent) = get_torrent(connection, id) {
                let changed = old_torrent.title != parsed.title
                    || old_torrent.category != parsed.category as i32
                    || old_torrent.information != parsed.info
                    || old_torrent.description != parsed.desc
                    || comments_num as usize != comments.len()
                    || old_torrent.remake != parsed.remake
                    || old_torrent.trusted != parsed.trusted
                    || old_torrent.anonymous != parsed.anonymous
                    || old_torrent.hidden != parsed.hidden;
                (
                    update_action(
                        false,
                        changed,
                        parsed.date as i64,
                        old_torrent.update_frequency,
                    ),
                    old_torrent.update_count + 1,
                )
            } else {
                (update_action(true, true, parsed.date as i64, None), 1)
            };
        NewTorrent {
            id: Some(id as i32),
            info_hash: hash,
            title: parsed.title,
            category: parsed.category as i32,
            submitter: parsed.submitter,
            information: parsed.info,
            size: size as i64,
            date: parsed.date as i64,
            description: parsed.desc,
            comments: comments_num,
            remake: parsed.remake,
            trusted: parsed.trusted,
            anonymous: parsed.anonymous,
            partial: false,
            deleted: false,
            hidden: parsed.hidden,
            next_update: Some(next_update),
            update_count,
            update_frequency: Some(update_frequency),
        }
        .insert(connection, true)?;

        for comment in &comments {
            let user = User {
                username: comment.submitter.username.clone(),
                password: None,
                salt: None,
                email: None,
                nyaa: true,
                trusted: comment.submitter.trusted,
                banned: comment.submitter.banned,
                nyaa_admin: comment.submitter.admin,
                nyaa_mod: comment.submitter.moderator,
                avatar: comment.submitter.avatar.clone(),
            };
            user.insert(connection, true)?;

            if !comment_exists(connection, comment.comment_id) {
                NewComment {
                    id: Some(comment.comment_id as i32),
                    torrent_id: comment.torrent_id as i32,
                    submitter: comment.submitter.username.clone(),
                    date_created: comment.date as i64,
                    date_edited: None,
                    text: comment.content.clone(),
                }
                .insert(connection, false)?;
            }

            if let Some(avatar) = user.avatar {
                let user_path = PathBuf::from(format!("./avatars/{}/", user.username));
                let avatar_path = PathBuf::from(format!("./avatars/{}/{avatar}", user.username));
                let avatar_url = format!("https://nyaa.si/user/{}/{avatar}", user.username);

                if !avatar_path.try_exists().is_ok_and(|e| e) {
                    create_dir_all(&user_path)
                        .map_err(|e| ScrapeError::CreateDirectory(user_path, e))?;
                    self.download(&avatar_url, &avatar_path)?;
                }
            }
        }

        Ok(())
    }
}

fn select<'a>(
    element: &ElementRef<'a>,
    selector: &str,
    url: &str,
) -> Result<ElementRef<'a>, ScrapeError> {
    let s = Selector::parse(selector).unwrap();
    element
        .select(&s)
        .next()
        .ok_or(ScrapeError::Selector(url.to_string(), selector.to_string()))
}

fn attr<'a>(element: &'a ElementRef, attr: &'a str) -> Result<&'a str, ScrapeError> {
    element
        .attr(attr)
        .ok_or(ScrapeError::MissingAttribute(attr.to_string()))
}

fn parse_partial_torrent(
    torrent: &ElementRef,
    url: &str,
) -> Result<ParsedPartialTorrent, ScrapeError> {
    let title_element = select(torrent, "td[colspan=\"2\"]>a:not(.comments)", url)?;
    let title = attr(&title_element, "title")?.to_string();
    let magnet = Magnet::new(attr(&select(torrent, "a:has(i.fa-magnet)", url)?, "href")?)?;
    let id = attr(&title_element, "href")?
        .replace("/view/", "")
        .parse::<usize>()
        .map_err(|e| ScrapeError::BadAttribute("href".to_string(), e.to_string()))?;
    let comments_element = select(torrent, ".comments", url);
    let comments = if comments_element.as_ref().is_ok() {
        comments_element
            .unwrap()
            .text()
            .collect::<String>()
            .replace("\n", "")
            .replace("\t", "")
            .parse::<usize>()
            .map_err(|e| ScrapeError::BadAttribute("textContent".to_string(), e.to_string()))?
    } else {
        0
    };
    let remake = torrent
        .value()
        .has_class("danger", scraper::CaseSensitivity::CaseSensitive);
    let trusted = torrent
        .value()
        .has_class("success", scraper::CaseSensitivity::CaseSensitive);
    let size = select(torrent, ":nth-child(4)", url)?
        .text()
        .collect::<String>()
        .parse::<byte_unit::Byte>()
        .map_err(|e| ScrapeError::BadAttribute("textContent".to_string(), e.to_string()))?;
    let date = attr(
        &select(torrent, "td[data-timestamp]", url)?,
        "data-timestamp",
    )?
    .parse::<usize>()
    .map_err(|e| ScrapeError::BadAttribute("data-timestamp".to_string(), e.to_string()))?;
    let category = attr(&select(torrent, "td>a:has(img)", url)?, "href")?
        .replace("/?c=", "")
        .replace("_", "")
        .parse::<usize>()
        .map_err(|e| ScrapeError::BadAttribute("href".to_string(), e.to_string()))?;
    Ok(ParsedPartialTorrent {
        id: (id),
        title: (title),
        category: (category),
        magnet: (magnet),
        size: size.as_u64() as usize,
        date: (date),
        comments: (comments),
        remake: (remake),
        trusted: (trusted),
    })
}

fn parse_torrent(dom: &ElementRef, id: usize) -> Result<ParsedTorrent, ScrapeError> {
    let url = format!("https://nyaa.si/view/{id}");
    let panel_heading = select(dom, ".panel-heading", &url)?;
    let title_element = select(dom, ".panel-title", &url)?;
    let title_parent = title_element
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .value()
        .as_element()
        .unwrap();
    let remake = title_parent.has_class("panel-danger", scraper::CaseSensitivity::CaseSensitive);
    let trusted = title_parent.has_class("panel-success", scraper::CaseSensitivity::CaseSensitive);
    let hidden = panel_heading
        .attr("style")
        .is_some_and(|a| a.contains("background-color: darkgray"));
    let title = title_element
        .text()
        .collect::<String>()
        .replace("\n			", "")
        .replace("\n		", "");
    let category = attr(
        &select(
            dom,
            ".panel-body>.row:nth-child(1)>.col-md-5:nth-child(2)>a:nth-child(2)",
            &url,
        )?,
        "href",
    )?
    .replace("/?c=", "")
    .replace("_", "")
    .parse::<usize>()
    .map_err(|e| ScrapeError::BadAttribute("href".to_string(), e.to_string()))?;
    let date = attr(
        &select(dom, ".col-md-5[data-timestamp]", &url)?,
        "data-timestamp",
    )?
    .parse::<usize>()
    .map_err(|e| ScrapeError::BadAttribute("data-timestamp".to_string(), e.to_string()))?;
    let submitter = select(
        dom,
        ".panel-body>.row:nth-child(2)>.col-md-5:nth-child(2)>a",
        &url,
    );
    let submitter = if submitter.as_ref().is_ok() {
        Some(
            submitter
                .unwrap()
                .text()
                .collect::<String>()
                .replace("\n", "")
                .replace("\t", "")
                .trim_end()
                .to_string(),
        )
    } else {
        None
    };
    let anonymous = submitter.as_ref().is_none();
    let info = select(
        dom,
        ".panel-body>.row:nth-child(3)>.col-md-5:nth-child(2)",
        &url,
    )?
    .text()
    .collect::<String>()
    .replace("\n				", "")
    .replace("\n			", "");
    let info = if info == "No information." {
        None
    } else {
        Some(info)
    };
    let desc = select(dom, "#torrent-description", &url)?.inner_html();
    let desc = if desc == "#### No description." {
        None
    } else {
        Some(desc)
    };
    Ok(ParsedTorrent {
        title: (title),
        category: (category),
        submitter: (submitter),
        info: (info),
        date: (date),
        desc: (desc),
        remake: (remake),
        trusted: (trusted),
        anonymous: (anonymous),
        hidden: (hidden),
    })
}

fn parse_comment(comment: &ElementRef, id: usize) -> Result<ParsedComment, ScrapeError> {
    let url = format!("https://nyaa.si/view/{id}");
    let submitter_elem = select(comment, "p>a", &url)?;
    let submitter_title = attr(&submitter_elem, "title")?;
    let submitter = submitter_elem.text().collect::<String>();
    let banned = submitter_title.contains("BANNED");
    let trusted = submitter_title.contains("Trusted");
    let admin = submitter_title.contains("Administrator");
    let moderator = submitter_title.contains("Moderator");
    let avatar = attr(&select(comment, ".avatar", &url)?, "src")?.to_string();
    let date = attr(
        &select(comment, "small[data-timestamp]", &url)?,
        "data-timestamp",
    )?
    .parse::<usize>()
    .map_err(|e| ScrapeError::BadAttribute("data-timestamp".to_string(), e.to_string()))?;
    let content_element = select(comment, ".comment-content", &url)?;
    let content = content_element.inner_html();
    let comment_id = attr(&content_element, "id")?
        .replace("torrent-comment", "")
        .parse::<usize>()
        .map_err(|e| ScrapeError::BadAttribute("torrent-comment".to_string(), e.to_string()))?;

    Ok(ParsedComment {
        comment_id: (comment_id),
        torrent_id: id,
        submitter: ParsedCommentUser {
            username: submitter,
            trusted: (trusted),
            banned: (banned),
            admin: (admin),
            moderator: (moderator),
            avatar: if avatar == "/static/img/avatar/default.png" {
                None
            } else {
                Some(
                    avatar
                        .split('/')
                        .next_back()
                        .ok_or(ScrapeError::AvatarUrl(avatar.clone()))?
                        .split('?')
                        .next()
                        .ok_or(ScrapeError::AvatarUrl(avatar.clone()))?
                        .to_string(),
                )
            },
        },
        date: (date),
        content: (content),
    })
}
