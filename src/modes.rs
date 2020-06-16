#[derive(Clone, Debug)]
pub enum Selected {
    Feeds,
    Entries,
    Entry(crate::rss::Entry),
}

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Editing,
    Normal,
}

#[derive(Clone, Debug)]
pub enum ReadMode {
    ShowRead,
    ShowUnread,
    All,
}
