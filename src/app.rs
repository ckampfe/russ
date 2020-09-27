use crate::error::Error;
use crate::modes::{Mode, ReadMode, Selected};
use crate::util;
use copypasta::{ClipboardContext, ClipboardProvider};
use std::sync::{Arc, Mutex};
use tui::{backend::CrosstermBackend, Terminal};

#[derive(Clone, Debug)]
pub struct App {
    inner: Arc<Mutex<AppImpl>>,
}

impl App {
    pub fn new(options: crate::Options) -> Result<App, Error> {
        Ok(App {
            inner: Arc::new(Mutex::new(AppImpl::new(options)?)),
        })
    }

    pub fn draw(
        &self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> std::io::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        terminal.draw(|mut f| crate::ui::draw(&mut f, &mut inner))
    }

    pub fn mode(&self) -> Mode {
        self.inner.lock().unwrap().mode
    }

    pub fn on_key(&self, c: char) -> Result<(), Error> {
        match c {
            // vim-style movement
            'h' => self.on_left(),
            'j' => self.on_down(),
            'k' => self.on_up(),
            'l' => self.on_right(),
            'a' => self.toggle_read_mode(),
            'e' | 'i' => {
                let mut inner = self.inner.lock().unwrap();
                inner.mode = Mode::Editing;
                Ok(())
            }
            'c' => self.put_current_link_in_clipboard(),
            _ => Ok(()),
        }
    }

    pub fn on_up(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();

        match inner.selected {
            Selected::Feeds => {
                inner.feeds.previous();
                inner.update_current_feed_and_entries()?;
            }
            Selected::Entries => {
                if !inner.entries.items.is_empty() {
                    inner.entries.previous();
                    inner.entry_selection_position = inner.entries.state.selected().unwrap();
                    if let Some(entry) = inner.get_selected_entry() {
                        let entry = entry?;
                        inner.current_entry = Some(entry);
                    }
                }
            }
            Selected::Entry(_) => {
                if let Some(n) = inner.entry_scroll_position.checked_sub(1) {
                    inner.entry_scroll_position = n
                };
            }
        }

        Ok(())
    }

    pub fn on_down(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();

        match inner.selected {
            Selected::Feeds => {
                inner.feeds.next();
                inner.update_current_feed_and_entries()?;
            }
            Selected::Entries => {
                if !inner.entries.items.is_empty() {
                    inner.entries.next();
                    inner.entry_selection_position = inner.entries.state.selected().unwrap();
                    if let Some(entry) = inner.get_selected_entry() {
                        let entry = entry?;
                        inner.current_entry = Some(entry);
                    }
                }
            }
            Selected::Entry(_) => {
                if let Some(n) = inner.entry_scroll_position.checked_add(1) {
                    inner.entry_scroll_position = n
                };
            }
        }

        Ok(())
    }

    pub fn on_right(&self) -> Result<(), Error> {
        let selected = self.inner.lock().unwrap().selected.clone();

        let mut inner = self.inner.lock().unwrap();

        match selected {
            Selected::Feeds => {
                if !inner.entries.items.is_empty() {
                    inner.selected = Selected::Entries;
                    inner.entries.state.select(Some(0));
                    if let Some(entry) = inner.get_selected_entry() {
                        let entry = entry?;
                        inner.current_entry = Some(entry);
                    }
                }
                Ok(())
            }
            Selected::Entries => inner.on_enter(),
            Selected::Entry(_) => Ok(()),
        }
    }

    pub fn on_left(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();

        match inner.selected {
            Selected::Feeds => (),
            Selected::Entries => inner.selected = Selected::Feeds,
            Selected::Entry(_) => {
                inner.entry_scroll_position = 0;
                inner.selected = {
                    inner.current_entry_text = String::new();
                    Selected::Entries
                }
            }
        }

        Ok(())
    }

    pub fn on_enter(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        inner.on_enter()
    }

    fn put_current_link_in_clipboard(&self) -> Result<(), Error> {
        let mut ctx = ClipboardContext::new().unwrap();

        let inner = self.inner.lock().unwrap();

        let clipboard_result = match &inner.selected {
            Selected::Feeds => {
                let feed = inner.current_feed.clone().unwrap();
                let link = feed.link.clone().unwrap_or_else(|| feed.feed_link.unwrap());
                ctx.set_contents(link)
            }
            Selected::Entries => {
                let idx = inner.entry_selection_position;
                let entry = &inner.entries.items[idx];
                let link = entry.link.clone().unwrap_or_else(|| "".to_string());
                ctx.set_contents(link)
            }
            Selected::Entry(e) => {
                let link = e.link.clone().unwrap_or_else(|| "".to_string());
                ctx.set_contents(link)
            }
        };

        clipboard_result.map_err(|_e| crate::error::ClipboardSetError.into())
    }

    pub fn toggle_read_mode(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();

        match (&inner.read_mode, &inner.selected) {
            (ReadMode::ShowRead, Selected::Feeds) | (ReadMode::ShowRead, Selected::Entries) => {
                inner.entry_selection_position = 0;
                inner.read_mode = ReadMode::ShowUnread
            }
            (ReadMode::ShowUnread, Selected::Feeds) | (ReadMode::ShowUnread, Selected::Entries) => {
                inner.entry_selection_position = 0;
                inner.read_mode = ReadMode::ShowRead
            }
            _ => (),
        }
        inner.update_current_entries()?;

        if !inner.entries.items.is_empty() {
            inner.entries.state.select(Some(0));
        } else {
            inner.entries.state.select(None);
        }

        if let Some(entry) = inner.get_selected_entry() {
            let entry = entry?;
            inner.current_entry = Some(entry);
        }

        Ok(())
    }

    pub fn set_flash(&self, flash: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.flash = Some(flash)
    }

    pub fn clear_flash(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.flash = None
    }

    pub fn error_flash_is_empty(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.error_flash.is_empty()
    }

    pub fn push_error_flash(&self, e: crate::error::Error) {
        let mut inner = self.inner.lock().unwrap();
        inner.error_flash.push(e);
    }

    pub fn clear_error_flash(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.error_flash = vec![];
    }

    pub fn set_mode(&self, mode: Mode) {
        let mut inner = self.inner.lock().unwrap();
        inner.mode = mode;
    }

    pub fn feed_subscription_input(&self) -> String {
        let inner = self.inner.lock().unwrap();
        inner.feed_subscription_input.clone()
    }

    pub fn set_feed_subscription_input(&self, feed_subscription_input: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.feed_subscription_input = feed_subscription_input;
    }

    pub fn push_feed_subscription_input(&self, input: char) {
        let mut inner = self.inner.lock().unwrap();
        inner.feed_subscription_input.push(input);
    }

    pub fn pop_feed_subscription_input(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.feed_subscription_input.pop();
    }

    pub fn set_feeds(&self, feeds: Vec<crate::rss::Feed>) {
        let mut inner = self.inner.lock().unwrap();
        let feeds = feeds.into();
        inner.feeds = feeds;
    }

    pub fn update_current_feed_and_entries(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        inner.update_current_feed()?;
        inner.update_current_entries()?;
        Ok(())
    }

    pub fn select_feeds(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.selected = Selected::Feeds;
    }

    pub fn selected(&self) -> Selected {
        let inner = self.inner.lock().unwrap();
        inner.selected.clone()
    }

    pub fn selected_feed_id(&self) -> crate::rss::FeedId {
        let feeds = &self.inner.lock().unwrap().feeds;
        let selected_idx = feeds.state.selected().unwrap();
        feeds.items[selected_idx].id
    }

    pub fn feed_ids(&self) -> Result<Vec<crate::rss::FeedId>, crate::error::Error> {
        let inner = self.inner.lock().unwrap();

        let ids = crate::rss::get_feeds(&inner.conn)?
            .iter()
            .map(|feed| feed.id)
            .collect::<Vec<_>>();

        Ok(ids)
    }

    pub fn toggle_read(&mut self) -> Result<(), Error> {
        let mut inner = self.inner.lock().unwrap();
        let selected = inner.selected.clone();
        match selected {
            Selected::Entry(entry) => {
                entry.toggle_read(&inner.conn)?;
                inner.selected = Selected::Entries;
                inner.update_current_entries()?;

                if let Some(entry) = inner.get_selected_entry() {
                    let entry = entry?;
                    inner.current_entry = Some(entry);
                }

                inner.entry_scroll_position = 0;
            }
            Selected::Entries => {
                if let Some(entry) = &inner.current_entry {
                    entry.toggle_read(&inner.conn)?;
                    inner.update_current_entries()?;
                    if let Some(entry) = inner.get_selected_entry() {
                        let entry = entry?;
                        inner.current_entry = Some(entry);
                    }
                }
            }
            Selected::Feeds => (),
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct AppImpl {
    // database stuff
    pub conn: rusqlite::Connection,
    // feed stuff
    pub current_feed: Option<crate::rss::Feed>,
    pub feeds: util::StatefulList<crate::rss::Feed>,
    // entry stuff
    pub current_entry: Option<crate::rss::Entry>,
    pub entries: util::StatefulList<crate::rss::Entry>,
    pub line_length: usize,
    pub entry_selection_position: usize,
    pub current_entry_text: String,
    pub entry_scroll_position: u16,
    // modes
    pub should_quit: bool,
    pub selected: Selected,
    pub mode: Mode,
    pub read_mode: ReadMode,
    // misc
    pub error_flash: Vec<Error>,
    pub feed_subscription_input: String,
    pub flash: Option<String>,
}

impl AppImpl {
    pub fn new(options: crate::Options) -> Result<AppImpl, Error> {
        let conn = rusqlite::Connection::open(&options.database_path)?;
        crate::rss::initialize_db(&conn)?;
        let initial_feed_titles = vec![].into();
        let selected = Selected::Feeds;
        let initial_current_feed = None;
        let initial_entries = vec![].into();

        let mut app = AppImpl {
            conn,
            line_length: options.line_length,
            should_quit: false,
            error_flash: vec![],
            feeds: initial_feed_titles,
            entries: initial_entries,
            selected,
            entry_scroll_position: 0,
            current_entry: None,
            current_entry_text: String::new(),
            current_feed: initial_current_feed,
            feed_subscription_input: String::new(),
            mode: Mode::Normal,
            read_mode: ReadMode::ShowUnread,
            entry_selection_position: 0,
            flash: None,
        };

        app.update_feeds()?;
        app.update_current_feed_and_entries()?;

        Ok(app)
    }

    pub fn update_feeds(&mut self) -> Result<(), Error> {
        let feeds = crate::rss::get_feeds(&self.conn)?.into();
        self.feeds = feeds;
        Ok(())
    }

    pub fn update_current_feed_and_entries(&mut self) -> Result<(), Error> {
        self.update_current_feed()?;
        self.update_current_entries()?;
        Ok(())
    }

    fn update_current_feed(&mut self) -> Result<(), Error> {
        let current_feed = if self.feeds.items.is_empty() {
            None
        } else {
            let selected_idx = match self.feeds.state.selected() {
                Some(idx) => idx,
                None => {
                    self.feeds.state.select(Some(0));
                    0
                }
            };
            let feed_id = self.feeds.items[selected_idx].id;
            Some(crate::rss::get_feed(&self.conn, feed_id)?)
        };

        self.current_feed = current_feed;

        Ok(())
    }

    fn update_current_entries(&mut self) -> Result<(), Error> {
        let entries = if let Some(feed) = &self.current_feed {
            crate::rss::get_entries(&self.conn, &self.read_mode, feed.id)?
                .into_iter()
                .collect::<Vec<_>>()
                .into()
        } else {
            vec![].into()
        };

        self.entries = entries;
        if self.entry_selection_position < self.entries.items.len() {
            self.entries
                .state
                .select(Some(self.entry_selection_position))
        } else {
            match self.entries.items.len().checked_sub(1) {
                Some(n) => self.entries.state.select(Some(n)),
                None => self.entries.state.select(Some(0)),
            }
        }
        Ok(())
    }

    fn get_selected_entry(&self) -> Option<Result<crate::rss::Entry, Error>> {
        if let Some(selected_idx) = self.entries.state.selected() {
            if let Some(entry_id) = self.entries.items.get(selected_idx).map(|item| item.id) {
                Some(crate::rss::get_entry(&self.conn, entry_id))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn on_enter(&mut self) -> Result<(), Error> {
        match self.selected {
            Selected::Entries => {
                if !self.entries.items.is_empty() {
                    if let Some(entry) = &self.current_entry {
                        let empty_string = String::from("No content or description tag provided.");

                        // try content tag first,
                        // if there is not content tag,
                        // go to description tag,
                        // if no description tag,
                        // use empty string.
                        // TODO figure out what to actually do if there are neither
                        let entry_html = &entry
                            .content
                            .as_ref()
                            .or_else(|| entry.description.as_ref())
                            .or_else(|| Some(&empty_string));

                        if let Some(html) = entry_html {
                            let text = html2text::from_read(html.as_bytes(), self.line_length);
                            self.current_entry_text = text;
                        } else {
                            self.current_entry_text = String::new();
                        }

                        self.selected = Selected::Entry(entry.clone());
                    }
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
