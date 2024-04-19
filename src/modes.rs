//! Russ is modal, and these are the modes it can be in.

/// what type of object is currently selected
#[derive(Clone, Debug)]
pub enum Selected {
    Feeds,
    Entries,
    Entry(crate::rss::EntryMetadata),
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
