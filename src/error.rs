use atom_syndication as atom;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    AtomError(atom::Error),
    ClipboardSetError(ClipboardSetError),
    DatabaseConnectionPoolError(r2d2::Error),
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

impl From<r2d2::Error> for Error {
    fn from(error: r2d2::Error) -> Error {
        Error::DatabaseConnectionPoolError(error)
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

impl From<ClipboardSetError> for Error {
    fn from(error: ClipboardSetError) -> Error {
        Error::ClipboardSetError(error)
    }
}

#[derive(Debug)]
pub struct ClipboardSetError;

impl std::error::Error for ClipboardSetError {}

impl fmt::Display for ClipboardSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
