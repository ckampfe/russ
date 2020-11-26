#[derive(Clone, Debug)]
pub enum Selected {
    Feeds,
    Entries,
    Entry(crate::rss::EntryMeta),
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
