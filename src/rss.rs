use crate::modes::ReadMode;
use anyhow::Result;
use atom_syndication as atom;
use chrono::prelude::*;
use rss::Channel;
use rusqlite::{params, ToSql, NO_PARAMS};
use std::collections::HashSet;
use std::fmt::Display;
use std::str::FromStr;

type EntryId = i64;
pub type FeedId = i64;

#[derive(Clone, Copy, Debug)]
pub enum FeedKind {
    Atom,
    RSS,
}

impl rusqlite::types::FromSql for FeedKind {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let s = value.as_str()?;
        match FeedKind::from_str(s) {
            Ok(feed_kind) => Ok(feed_kind),
            Err(e) => Err(rusqlite::types::FromSqlError::Other(e.into())),
        }
    }
}

impl Display for FeedKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            FeedKind::Atom => "Atom",
            FeedKind::RSS => "RSS",
        };

        write!(f, "{}", out)
    }
}

impl FromStr for FeedKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Atom" => Ok(FeedKind::Atom),
            "RSS" => Ok(FeedKind::RSS),
            _ => Err(anyhow::anyhow!(format!("{} is not a valid FeedKind", s))),
        }
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: EntryId,
    pub feed_id: FeedId,
    pub title: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<chrono::DateTime<Utc>>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub link: Option<String>,
    pub read_at: Option<chrono::DateTime<Utc>>,
    pub inserted_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl From<&atom::Entry> for Entry {
    fn from(entry: &atom::Entry) -> Self {
        Self {
            id: -1,
            feed_id: -1,
            title: Some(entry.title().to_string()),
            author: entry.authors().get(0).map(|author| author.name.to_owned()),
            pub_date: entry.published().map(|date| date.with_timezone(&Utc)),
            description: None,
            content: entry.content().and_then(|content| content.value.to_owned()),
            link: entry.links().get(0).map(|link| link.href().to_string()),
            read_at: None,
            inserted_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl From<&rss::Item> for Entry {
    fn from(entry: &rss::Item) -> Self {
        Self {
            id: -1,
            feed_id: -1,
            title: entry.title().map(|title| title.to_owned()),
            author: entry.author().map(|author| author.to_owned()),
            pub_date: entry.pub_date().and_then(parse_datetime),
            description: entry
                .description()
                .map(|description| description.to_owned()),
            content: entry.content().map(|content| content.to_owned()),
            link: entry.link().map(|link| link.to_owned()),
            read_at: None,
            inserted_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EntryMeta {
    pub id: EntryId,
    pub feed_id: FeedId,
    pub title: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<chrono::DateTime<Utc>>,
    pub link: Option<String>,
    pub read_at: Option<chrono::DateTime<Utc>>,
    pub inserted_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

impl EntryMeta {
    pub fn toggle_read(&self, conn: &rusqlite::Connection) -> Result<()> {
        if self.read_at.is_none() {
            self.mark_as_read(&conn)
        } else {
            self.mark_as_unread(conn)
        }
    }

    fn mark_as_read(&self, conn: &rusqlite::Connection) -> Result<()> {
        let mut statement = conn.prepare("UPDATE entries SET read_at = ?2 WHERE id = ?1")?;
        statement.execute(params![self.id, Utc::now()])?;
        Ok(())
    }

    fn mark_as_unread(&self, conn: &rusqlite::Connection) -> Result<()> {
        let mut statement = conn.prepare("UPDATE entries SET read_at = NULL WHERE id = ?1")?;
        statement.execute(params![self.id])?;
        Ok(())
    }
}

pub struct EntryContent {
    pub content: Option<String>,
    pub description: Option<String>,
}

fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
    diligent_date_parser::parse_date(s).map(|dt| dt.with_timezone(&Utc))
}

struct FeedAndEntries {
    pub feed: Feed,
    pub entries: Vec<Entry>,
}

impl FeedAndEntries {
    pub fn set_feed_link(&mut self, url: &str) {
        self.feed.feed_link = Some(url.to_owned());
    }
}

impl FromStr for FeedAndEntries {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match atom::Feed::from_str(s) {
            Ok(atom_feed) => {
                let feed = Feed {
                    id: 0,
                    title: Some(atom_feed.title.clone()),
                    feed_link: None,
                    link: atom_feed.links.get(0).map(|link| link.href().to_string()),
                    feed_kind: FeedKind::Atom,
                    refreshed_at: None,
                    inserted_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                let entries = atom_feed
                    .entries()
                    .iter()
                    .map(|entry| entry.into())
                    .collect::<Vec<_>>();

                Ok(FeedAndEntries { feed, entries })
            }

            Err(_e) => match Channel::from_str(s) {
                Ok(channel) => {
                    let feed = Feed {
                        id: 0,
                        title: Some(channel.title().to_string()),
                        feed_link: None,
                        link: Some(channel.link().to_string()),
                        feed_kind: FeedKind::RSS,
                        refreshed_at: None,
                        inserted_at: Utc::now(),
                        updated_at: Utc::now(),
                    };

                    let entries = channel
                        .items()
                        .iter()
                        .map(|item| item.into())
                        .collect::<Vec<_>>();

                    Ok(FeedAndEntries { feed, entries })
                }
                Err(e) => Err(e.into()),
            },
        }
    }
}

pub fn subscribe_to_feed(
    http_client: &reqwest::blocking::Client,
    conn: &rusqlite::Connection,
    url: &str,
) -> Result<FeedId> {
    let feed_and_entries: FeedAndEntries = fetch_feed(http_client, url)?;
    let feed_id = create_feed(conn, &feed_and_entries.feed)?;
    add_entries_to_feed(conn, feed_id, &feed_and_entries.entries)?;

    Ok(feed_id)
}

fn fetch_feed(http_client: &reqwest::blocking::Client, url: &str) -> Result<FeedAndEntries> {
    let resp = http_client.get(url).send()?.text()?;
    let mut feed = FeedAndEntries::from_str(&resp)?;
    feed.set_feed_link(url);

    Ok(feed)
}

/// fetches the feed and stores the new entries
/// uses the link as the uniqueness key.
/// TODO hash the content to see if anything changed, and update that way.
pub fn refresh_feed(
    client: &reqwest::blocking::Client,
    conn: &rusqlite::Connection,
    feed_id: FeedId,
) -> Result<()> {
    let feed_url = get_feed_url(conn, feed_id)?;
    let remote_feed: FeedAndEntries = fetch_feed(client, &feed_url)?;
    let remote_items = remote_feed.entries;
    let remote_items_links = remote_items
        .iter()
        .flat_map(|item| &item.link)
        .cloned()
        .collect::<HashSet<String>>();

    let local_entries_links = get_entries_links(conn, &ReadMode::All, feed_id)?
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>();

    let difference = remote_items_links
        .difference(&local_entries_links)
        .cloned()
        .collect::<HashSet<_>>();

    let items_to_add = remote_items
        .into_iter()
        .filter(|item| match &item.link {
            Some(link) => difference.contains(link.as_str()),
            None => false,
        })
        .collect::<Vec<_>>();

    add_entries_to_feed(conn, feed_id, &items_to_add)?;

    update_feed_refreshed_at(&conn, feed_id)?;

    Ok(())
}

pub fn initialize_db(conn: &rusqlite::Connection) -> Result<()> {
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
        read_at TIMESTAMP,
        inserted_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
        NO_PARAMS,
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS entries_feed_id_and_pub_date_and_inserted_at_index 
        ON entries (feed_id, pub_date, inserted_at)",
        NO_PARAMS,
    )?;

    Ok(())
}

fn create_feed(conn: &rusqlite::Connection, feed: &Feed) -> Result<FeedId> {
    conn.execute(
        "INSERT INTO feeds (title, link, feed_link, feed_kind)
        VALUES (?1, ?2, ?3, ?4)",
        params![
            feed.title,
            feed.link,
            feed.feed_link,
            feed.feed_kind.to_string()
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

fn add_entries_to_feed(
    conn: &rusqlite::Connection,
    feed_id: FeedId,
    entries: &[Entry],
) -> Result<()> {
    if !entries.is_empty() {
        let now = Utc::now();

        let mut entries_values = vec![];

        for entry in entries {
            entries_values.append(&mut vec![
                feed_id.to_sql()?,
                entry.title.to_sql()?,
                entry.author.to_sql()?,
                entry.pub_date.to_sql()?,
                entry.description.to_sql()?,
                entry.content.to_sql()?,
                entry.link.to_sql()?,
                now.to_sql()?,
            ]);
        }

        // this seems cool but probably not the right fit:
        // https://stackoverflow.com/questions/54177438/how-to-programmatically-get-the-number-of-fields-of-a-struct
        let query = build_bulk_insert_query(
            "entries",
            &[
                "feed_id",
                "title",
                "author",
                "pub_date",
                "description",
                "content",
                "link",
                "updated_at",
            ],
            &entries_values,
            entries_values.len() / entries.len(),
        );

        conn.execute(&query, entries_values)?;
    }

    Ok(())
}

fn build_bulk_insert_query<T>(
    table: &str,
    columns: &[&str],
    entries_values: &[T],
    entry_fields_len: usize,
) -> String {
    let idxs = (1..entries_values.len() + 1).collect::<Vec<_>>();
    let mut parameter_groups_strings = Vec::with_capacity(entries_values.len());
    let mut parameters_values = Vec::with_capacity(entry_fields_len);

    for chunk in idxs.chunks(entry_fields_len) {
        parameters_values.clear();
        let mut parameter_group_string = "(".to_string();
        for i in chunk {
            parameters_values.push(format!("?{}", i))
        }
        let parameter_values_string = parameters_values.join(", ");
        parameter_group_string.push_str(&parameter_values_string);
        parameter_group_string.push(')');
        parameter_groups_strings.push(parameter_group_string);
    }

    let paramter_groups_string = parameter_groups_strings.join(", ");

    let mut columns_string = "".to_string();
    columns_string.push('(');
    columns_string.push_str(&columns.join(", "));
    columns_string.push_str(") ");

    let mut query = String::new();
    query.push_str("INSERT INTO ");
    query.push_str(table);
    query.push_str(&columns_string);
    query.push_str("VALUES ");
    query.push_str(&paramter_groups_string);

    query
}

pub fn get_feed(conn: &rusqlite::Connection, feed_id: FeedId) -> Result<Feed> {
    let s = conn.query_row(
        "SELECT id, title, feed_link, link, feed_kind, refreshed_at, inserted_at, updated_at FROM feeds WHERE id=?1",
        params![feed_id],
        |row| {
            let feed_kind_str: String = row.get(4)?;
            let feed_kind: FeedKind = FeedKind::from_str(&feed_kind_str)
                .expect(&format!("FeedKind must be Atom or RSS, got {}", feed_kind_str));

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

fn update_feed_refreshed_at(conn: &rusqlite::Connection, feed_id: FeedId) -> Result<()> {
    conn.execute(
        "UPDATE feeds SET refreshed_at = ?2 WHERE id = ?1",
        params![feed_id, Utc::now()],
    )?;

    Ok(())
}

pub fn get_feed_url(conn: &rusqlite::Connection, feed_id: FeedId) -> Result<String> {
    let s: String = conn.query_row(
        "SELECT feed_link FROM feeds WHERE id=?1",
        params![feed_id],
        |row| row.get(0),
    )?;

    Ok(s)
}

pub fn get_feeds(conn: &rusqlite::Connection) -> Result<Vec<Feed>> {
    let mut statement = conn.prepare(
        "SELECT 
          id, 
          title, 
          feed_link, 
          link, 
          feed_kind, 
          refreshed_at, 
          inserted_at, 
          updated_at 
        FROM feeds ORDER BY lower(title) ASC",
    )?;
    let mut feeds = vec![];
    for feed in statement.query_map(NO_PARAMS, |row| {
        Ok(Feed {
            id: row.get(0)?,
            title: row.get(1)?,
            feed_link: row.get(2)?,
            link: row.get(3)?,
            feed_kind: row.get(4)?,
            refreshed_at: row.get(5)?,
            inserted_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })? {
        feeds.push(feed?)
    }

    Ok(feeds)
}

pub fn get_feed_ids(conn: &rusqlite::Connection) -> Result<Vec<FeedId>> {
    let mut statement = conn.prepare("SELECT id FROM feeds ORDER BY lower(title) ASC")?;
    let mut ids = vec![];
    for id in statement.query_map(NO_PARAMS, |row| Ok(row.get(0)?))? {
        ids.push(id?)
    }

    Ok(ids)
}

pub fn get_entry_meta(conn: &rusqlite::Connection, entry_id: EntryId) -> Result<EntryMeta> {
    let result = conn.query_row(
        "SELECT 
          id, 
          feed_id, 
          title, 
          author, 
          pub_date, 
          link, 
          read_at, 
          inserted_at, 
          updated_at 
        FROM entries WHERE id=?1",
        params![entry_id],
        |row| {
            Ok(EntryMeta {
                id: row.get(0)?,
                feed_id: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                pub_date: row.get(4)?,
                link: row.get(5)?,
                read_at: row.get(6)?,
                inserted_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        },
    )?;

    Ok(result)
}

pub fn get_entry_content(conn: &rusqlite::Connection, entry_id: EntryId) -> Result<EntryContent> {
    let result = conn.query_row(
        "SELECT content, description FROM entries WHERE id=?1",
        params![entry_id],
        |row| {
            Ok(EntryContent {
                content: row.get(0)?,
                description: row.get(1)?,
            })
        },
    )?;

    Ok(result)
}

pub fn get_entries_metas(
    conn: &rusqlite::Connection,
    read_mode: &ReadMode,
    feed_id: FeedId,
) -> Result<Vec<EntryMeta>> {
    let read_at_predicate = match read_mode {
        ReadMode::ShowUnread => "\nAND read_at IS NULL",
        ReadMode::ShowRead => "\nAND read_at IS NOT NULL",
        ReadMode::All => "\n",
    };

    // we get weird pubDate formats from feeds,
    // so sort by inserted at as this as a stable order at least
    let mut query = "SELECT 
        id, 
        feed_id, 
        title, 
        author, 
        pub_date, 
        link, 
        read_at, 
        inserted_at, 
        updated_at 
        FROM entries 
        WHERE feed_id=?1"
        .to_string();

    query.push_str(read_at_predicate);
    query.push_str("\nORDER BY pub_date DESC, inserted_at DESC");

    let mut statement = conn.prepare(&query)?;
    let mut entries = vec![];
    for entry in statement.query_map(params![feed_id], |row| {
        Ok(EntryMeta {
            id: row.get(0)?,
            feed_id: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            pub_date: row.get(4)?,
            link: row.get(5)?,
            read_at: row.get(6)?,
            inserted_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    })? {
        entries.push(entry?)
    }

    Ok(entries)
}

pub fn get_entries_links(
    conn: &rusqlite::Connection,
    read_mode: &ReadMode,
    feed_id: FeedId,
) -> Result<Vec<Option<String>>> {
    let read_at_predicate = match read_mode {
        ReadMode::ShowUnread => "\nAND read_at IS NULL",
        ReadMode::ShowRead => "\nAND read_at IS NOT NULL",
        ReadMode::All => "\n",
    };

    // we get weird pubDate formats from feeds,
    // so sort by inserted at as this as a stable order at least
    let mut query = "SELECT link FROM entries WHERE feed_id=?1".to_string();

    query.push_str(read_at_predicate);
    query.push_str("\nORDER BY pub_date DESC, inserted_at DESC");

    let mut links = vec![];
    let mut statement = conn.prepare(&query)?;

    for link in statement.query_map(params![feed_id], |row| Ok(row.get(0)?))? {
        links.push(link?);
    }

    Ok(links)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ZCT: &str = "https://zeroclarkthirty.com/feed";

    #[test]
    fn it_fetches() {
        let http_client = reqwest::blocking::ClientBuilder::default()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let feed_and_entries = fetch_feed(&http_client, ZCT).unwrap();
        assert!(feed_and_entries.entries.len() > 0)
    }

    #[test]
    fn it_subscribes_to_a_feed() {
        let http_client = reqwest::blocking::ClientBuilder::default()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        initialize_db(&conn).unwrap();
        subscribe_to_feed(&http_client, &conn, ZCT).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM entries", NO_PARAMS, |row| row.get(0))
            .unwrap();

        assert!(count > 50)
    }

    #[test]
    fn refresh_feed_does_not_add_any_items_if_there_are_no_new_items() {
        let http_client = reqwest::blocking::ClientBuilder::default()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        initialize_db(&conn).unwrap();
        subscribe_to_feed(&http_client, &conn, ZCT).unwrap();
        let feed_id = 1;
        let old_entries = get_entries_metas(&conn, &ReadMode::ShowUnread, feed_id).unwrap();
        refresh_feed(&http_client, &conn, feed_id).unwrap();
        let e = get_entry_meta(&conn, 1).unwrap();
        e.mark_as_read(&conn).unwrap();
        let new_entries = get_entries_metas(&conn, &ReadMode::ShowUnread, feed_id).unwrap();

        assert_eq!(new_entries.len(), old_entries.len() - 1);
    }
}
