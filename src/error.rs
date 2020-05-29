use atom_syndication as atom;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    NetworkError(reqwest::Error),
    AtomError(atom::Error),
    RssError(rss::Error),
    DatabaseError(rusqlite::Error),
    FeedKindError(String),
    ChannelError(crossbeam_channel::RecvError),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        Error::NetworkError(error)
    }
}

impl From<atom::Error> for Error {
    fn from(error: atom::Error) -> Error {
        Error::AtomError(error)
    }
}

impl From<rss::Error> for Error {
    fn from(error: rss::Error) -> Error {
        Error::RssError(error)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Error {
        Error::DatabaseError(error)
    }
}

impl From<crossbeam_channel::RecvError> for Error {
    fn from(error: crossbeam_channel::RecvError) -> Error {
        Error::ChannelError(error)
    }
}
