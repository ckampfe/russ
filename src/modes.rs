#[derive(Clone, Debug, PartialEq)]
pub enum Selected {
    Feeds,
    Entries,
    Entry(crate::rss::Entry),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Editing,
    Normal,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReadMode {
    ShowRead,
    ShowUnread,
    All,
}
