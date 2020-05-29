use crate::error::Error;
use atom_syndication as atom;
use chrono::prelude::*;
use rss::Channel;
use rusqlite::{params, NO_PARAMS};
use std::collections::HashSet;
use std::str::FromStr;

type EntryId = i64;
type FeedId = i64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FeedKind {
    Atom,
    RSS,
}

impl ToString for FeedKind {
    fn to_string(&self) -> String {
        match self {
            Self::Atom => "Atom".to_string(),
            Self::RSS => "RSS".to_string(),
        }
    }
}

impl FromStr for FeedKind {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Atom" => Ok(FeedKind::Atom),
            "RSS" => Ok(FeedKind::RSS),
            _ => Err(crate::error::Error::FeedKindError(format!(
                "{} is not a valid FeedKind",
                s
            ))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Feed {
    pub id: FeedId,
    pub title: Option<String>,
    pub feed_link: Option<String>,
    pub link: Option<String>,
    pub feed_kind: FeedKind,
    pub refreshed_at: Option<chrono::DateTime<Utc>>,
    pub inserted_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub id: EntryId,
    pub feed_id: FeedId,
    pub title: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub link: Option<String>,
    pub read_on: Option<chrono::DateTime<Utc>>,
    pub inserted_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

struct InternalFeed {
    pub id: FeedId,
    pub title: Option<String>,
    pub feed_link: Option<String>,
    pub link: Option<String>,
    pub feed_kind: FeedKind,
    pub refreshed_at: Option<chrono::DateTime<Utc>>,
    pub inserted_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub entries: Vec<InternalEntry>,
}

impl InternalFeed {
    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }
    pub fn link(&self) -> Option<String> {
        self.link.clone()
    }
    pub fn feed_link(&self) -> Option<String> {
        self.feed_link.clone()
    }
    pub fn feed_kind(&self) -> FeedKind {
        self.feed_kind
    }
    pub fn entries(&self) -> Vec<InternalEntry> {
        self.entries.clone()
    }
    pub fn set_feed_link(&mut self, url: &str) {
        self.feed_link = Some(url.to_owned());
    }
}

impl FromStr for InternalFeed {
    type Err = crate::error::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match atom::Feed::from_str(s) {
            Ok(feed) => Ok(Self {
                id: 0,
                title: Some(feed.title.clone()),
                feed_link: None,
                link: feed.links.get(0).map(|link| link.href().to_string()),
                feed_kind: FeedKind::Atom,
                refreshed_at: None,
                inserted_at: Utc::now(),
                updated_at: Utc::now(),
                entries: feed
                    .entries()
                    .into_iter()
                    .map(|entry| entry.into())
                    .collect::<Vec<_>>(),
            }),
            Err(_e) => match Channel::from_str(s) {
                Ok(channel) => Ok(Self {
                    id: 0,
                    title: Some(channel.title().to_string()),
                    feed_link: None,
                    link: Some(channel.link().to_string()),
                    feed_kind: FeedKind::RSS,
                    refreshed_at: None,
                    inserted_at: Utc::now(),
                    updated_at: Utc::now(),
                    entries: channel
                        .items()
                        .iter()
                        .map(|item| item.into())
                        .collect::<Vec<_>>(),
                }),
                Err(e) => Err(e.into()),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct InternalEntry {
    pub title: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub link: Option<String>,
    pub read_on: Option<chrono::DateTime<Utc>>,
    pub inserted_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl InternalEntry {
    pub fn link(&self) -> Option<String> {
        self.link.clone()
    }
    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }
    pub fn author(&self) -> Option<String> {
        self.author.clone()
    }
    pub fn pub_date(&self) -> Option<String> {
        self.pub_date.clone()
    }
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }
    pub fn content(&self) -> Option<String> {
        self.content.clone()
    }
}

impl From<&atom::Entry> for InternalEntry {
    fn from(entry: &atom::Entry) -> Self {
        Self {
            title: Some(entry.title().to_string()),
            author: entry.authors().get(0).map(|author| author.name.to_owned()),
            pub_date: entry.published().map(|date| date.to_string()),
            description: None,
            content: entry.content().and_then(|content| content.value.to_owned()),
            link: entry.links().get(0).map(|link| link.href().to_string()),
            read_on: None,
            inserted_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl From<&rss::Item> for InternalEntry {
    fn from(entry: &rss::Item) -> Self {
        Self {
            title: entry.title().map(|title| title.to_owned()),
            author: entry.author().map(|author| author.to_owned()),
            pub_date: entry.pub_date().map(|pub_date| pub_date.to_owned()),
            description: entry
                .description()
                .map(|description| description.to_owned()),
            content: entry.content().map(|content| content.to_owned()),
            link: entry.link().map(|link| link.to_owned()),
            read_on: None,
            inserted_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
pub(crate) async fn subscribe_to_feed(
    conn: &rusqlite::Connection,
    url: &str,
) -> Result<FeedId, Error> {
    let feed: InternalFeed = fetch_feed(url).await?;
    let feed_id = create_feed(conn, &feed)?;
    // N+1!!!! YEAH BABY
    for entry in feed.entries() {
        add_entry_to_feed(conn, feed_id, &entry)?;
    }

    Ok(feed_id)
}

async fn fetch_feed(url: &str) -> Result<InternalFeed, Error> {
    let resp = reqwest::get(url).await?.text().await?;
    let mut feed = InternalFeed::from_str(&resp)?;
    feed.set_feed_link(url);

    Ok(feed)
}

/// fetches the feed and stores the new entries
/// uses the link as the uniqueness key.
/// TODO hash the content to see if anything changed, and update that way.
pub async fn refresh_feed(
    conn: &rusqlite::Connection,
    feed_id: FeedId,
) -> Result<Vec<EntryId>, Error> {
    let feed_url = get_feed_url(conn, feed_id)?;
    let remote_feed: InternalFeed = fetch_feed(&feed_url).await?;
    let remote_items = remote_feed.entries();
    let remote_items_links = remote_items
        .iter()
        .flat_map(|item| item.link())
        .collect::<HashSet<String>>();
    let local_entries_links = get_entries_links(conn, feed_id)?;

    let difference = remote_items_links
        .difference(
            &local_entries_links
                .into_iter()
                .map(|i| i)
                .collect::<HashSet<_>>(),
        )
        .cloned()
        .collect::<HashSet<_>>();

    let mut inserted_item_ids = vec![];

    let items_to_add = remote_items.iter().filter(|item| match item.link() {
        Some(link) => difference.contains(link.as_str()),
        None => false,
    });

    for item in items_to_add {
        let item_id = add_entry_to_feed(conn, feed_id, item)?;
        inserted_item_ids.push(item_id);
    }

    update_feed_refreshed_at(&conn, feed_id)?;

    Ok(inserted_item_ids)
}

// db functions
pub(crate) fn initialize_db(conn: &rusqlite::Connection) -> Result<(), Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS feeds (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT,
        feed_link TEXT,
        link TEXT,
        feed_kind TEXT,
        refreshed_at TIMESTAMP,
        inserted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )",
        NO_PARAMS,
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS entries (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        feed_id INTEGER,
        title TEXT,
        author TEXT,
        pub_date TIMESTAMP,
        description TEXT,
        content TEXT,
        link TEXT,
        read_on TIMESTAMP,
        inserted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        NO_PARAMS,
    )?;

    Ok(())
}

fn create_feed(conn: &rusqlite::Connection, feed: &InternalFeed) -> Result<FeedId, Error> {
    conn.execute(
        "INSERT INTO feeds (title, link, feed_link, feed_kind)
        VALUES (?1, ?2, ?3, ?4)",
        params![
            feed.title(),
            feed.link(),
            feed.feed_link(),
            feed.feed_kind().to_string()
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

fn add_entry_to_feed(
    conn: &rusqlite::Connection,
    feed_id: FeedId,
    entry: &InternalEntry,
) -> Result<EntryId, Error> {
    conn.execute(
        "INSERT INTO entries (
            feed_id, 
            title, 
            author, 
            pub_date, 
            description, 
            content, 
            link, 
            updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            feed_id,
            entry.title(),
            entry.author(),
            entry.pub_date(),
            entry.description(),
            entry.content(),
            entry.link(),
            Utc::now()
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

pub fn get_feed(conn: &rusqlite::Connection, feed_id: FeedId) -> Result<Feed, Error> {
    let s = conn.query_row(
        "SELECT id, title, feed_link, link, feed_kind, refreshed_at, inserted_at, updated_at FROM feeds WHERE id=?1",
        params![feed_id],
        |row| {
            let feed_kind_str: String = row.get(4)?;
            let feed_kind: FeedKind = FeedKind::from_str(&feed_kind_str).unwrap();
            Ok(Feed {
                id: row.get(0)?,
                title: row.get(1)?,
                feed_link: row.get(2)?,
                link: row.get(3)?,
                feed_kind,
                refreshed_at: row.get(5)?,
                inserted_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )?;

    Ok(s)
}

fn update_feed_refreshed_at(conn: &rusqlite::Connection, feed_id: FeedId) -> Result<(), Error> {
    conn.execute(
        "UPDATE feeds SET refreshed_at = ?2 WHERE id = ?1",
        params![feed_id, Utc::now()],
    )?;

    Ok(())
}

fn get_feed_url(conn: &rusqlite::Connection, feed_id: FeedId) -> Result<String, Error> {
    let s: String = conn.query_row(
        "SELECT feed_link FROM feeds WHERE id=?1",
        params![feed_id],
        |row| row.get(0),
    )?;

    Ok(s)
}

pub(crate) fn get_feed_titles(conn: &rusqlite::Connection) -> Result<Vec<(FeedId, String)>, Error> {
    let mut statement = conn.prepare("SELECT id, title FROM feeds ORDER BY title ASC")?;
    let result = statement
        .query_map(NO_PARAMS, |row| Ok((row.get(0)?, row.get(1)?)))?
        .map(|s| s.unwrap())
        .collect::<Vec<(FeedId, String)>>();

    Ok(result)
}

pub fn get_entry(conn: &rusqlite::Connection, entry_id: EntryId) -> Result<Entry, Error> {
    let result = conn.query_row(
        "SELECT 
          id, 
          feed_id, 
          title, 
          author, 
          pub_date, 
          description, 
          content, 
          link, 
          read_on, 
          inserted_at, 
          updated_at 
        FROM entries WHERE id=?1",
        params![entry_id],
        |row| {
            Ok(Entry {
                id: row.get(0)?,
                feed_id: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                pub_date: row.get(4)?,
                description: row.get(5)?,
                content: row.get(6)?,
                link: row.get(7)?,
                read_on: row.get(8)?,
                inserted_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        },
    )?;

    Ok(result)
}

pub fn get_entries(conn: &rusqlite::Connection, feed_id: FeedId) -> Result<Vec<Entry>, Error> {
    // we get weird pubDate formats from feeds,
    // so sort by inserted at as this as a stable order at least
    let mut statement = conn.prepare(
        "SELECT 
        id, 
        feed_id, 
        title, 
        author, 
        pub_date, 
        description, 
        content, 
        link, 
        read_on, 
        inserted_at, 
        updated_at 
        FROM entries 
        WHERE feed_id=?1
        ORDER BY inserted_at DESC",
    )?;
    let result = statement
        .query_map(params![feed_id], |row| {
            Ok(Entry {
                id: row.get(0)?,
                feed_id: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                pub_date: row.get(4)?,
                description: row.get(5)?,
                content: row.get(6)?,
                link: row.get(7)?,
                read_on: row.get(8)?,
                inserted_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })?
        .map(|entry| entry.unwrap())
        .collect::<Vec<_>>();

    Ok(result)
}

fn get_entries_links(
    conn: &rusqlite::Connection,
    feed_id: FeedId,
) -> Result<HashSet<String>, Error> {
    let mut statement =
        conn.prepare("SELECT link FROM entries WHERE feed_id=?1 ORDER BY pub_date DESC")?;
    let result = statement
        .query_map(params![feed_id], |row| row.get(0))?
        .map(|s| s.unwrap())
        .collect::<HashSet<String>>();

    Ok(result)
}

// fn drop_db(conn: &rusqlite::Connection) -> Result<(), Error> {
//     conn.execute("DROP TABLE IF EXISTS feeds", NO_PARAMS)?;
//     conn.execute("DROP TABLE IF EXISTS entries", NO_PARAMS)?;
//     Ok(())
// }

// fn drop_and_initialize_db(location: &PathBuf) -> Result<(), Error> {
//     drop_db(location)?;
//     initialize_db(location)?;
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;
    const ZCT: &str = "https://zeroclarkthirty.com/feed";

    #[tokio::test]
    async fn it_fetches() {
        let channel = fetch_feed(ZCT).await.unwrap();

        assert!(channel.entries().len() > 0)
    }

    #[tokio::test]
    async fn it_subscribes_to_a_feed() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        initialize_db(&conn).unwrap();
        subscribe_to_feed(&conn, ZCT).await.unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", NO_PARAMS, |row| row.get(0))
            .unwrap();

        assert!(count > 50)
    }

    #[tokio::test]
    async fn refresh_feed_does_not_add_any_items_if_there_are_no_new_items() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        initialize_db(&conn).unwrap();
        subscribe_to_feed(&conn, ZCT).await.unwrap();

        let feed_id = 1;
        let new_entry_ids = refresh_feed(&conn, feed_id).await.unwrap();
        assert_eq!(new_entry_ids.len(), 0)
    }
}
