use atom_syndication as atom;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    AtomError(atom::Error),
    DatabaseError(rusqlite::Error),
    FeedKindError(String),
    FromSqlError(rusqlite::types::FromSqlError),
    NetworkError(reqwest::Error),
    RssError(rss::Error),
    ThreadJoinError(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<atom::Error> for Error {
    fn from(error: atom::Error) -> Error {
        Error::AtomError(error)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Error {
        Error::DatabaseError(error)
    }
}

impl From<rusqlite::types::FromSqlError> for Error {
    fn from(error: rusqlite::types::FromSqlError) -> Error {
        Error::FromSqlError(error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        Error::NetworkError(error)
    }
}

impl From<rss::Error> for Error {
    fn from(error: rss::Error) -> Error {
        Error::RssError(error)
    }
}

impl From<Box<dyn std::any::Any + Send + 'static>> for Error {
    fn from(error: Box<dyn std::any::Any + Send + 'static>) -> Error {
        Error::ThreadJoinError(format!("{:?}", error))
    }
}
