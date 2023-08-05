#[derive(Clone, Debug)]
pub enum Selected {
    Feeds,
    Entries,
    Entry(crate::rss::EntryMeta),
    References(crate::rss::EntryMeta),
    None,
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
