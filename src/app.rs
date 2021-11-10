use crate::modes::{Mode, ReadMode, Selected};
use crate::util;
use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::{Arc, Mutex};
use tui::{backend::CrosstermBackend, Terminal};

macro_rules! delegate_to_locked_inner {
    ($(($fn_name:ident, $t:ty)),* $(,)? ) => {
        $(
            pub fn $fn_name(&self) -> $t {
                let inner = self.inner.lock().unwrap();
                inner.$fn_name()
            }
        )*
    };
}

macro_rules! delegate_to_locked_mut_inner {
    ($(($fn_name:ident, $t:ty)),* $(,)?) => {
        $(
            pub fn $fn_name(&self) -> $t {
                let mut inner = self.inner.lock().unwrap();
                inner.$fn_name()
            }
        )*
    };
}

#[derive(Clone, Debug)]
pub struct App {
    inner: Arc<Mutex<AppImpl>>,
}

impl App {
    delegate_to_locked_inner![
        (error_flash_is_empty, bool),
        (feed_ids, Result<Vec<crate::rss::FeedId>>),
        (feed_subscription_input, String),
        (force_redraw, Result<()>),
        (http_client, ureq::Agent),
        (mode, Mode),
        (selected, Selected),
        (selected_feed_id, crate::rss::FeedId),
        (open_link_in_browser, Result<()>),
    ];

    delegate_to_locked_mut_inner![
        (clear_error_flash, ()),
        (clear_flash, ()),
        (on_down, Result<()>),
        (on_enter, Result<()>),
        (on_left, Result<()>),
        (on_right, Result<()>),
        (on_up, Result<()>),
        (page_up, ()),
        (page_down, ()),
        (pop_feed_subscription_input, ()),
        (put_current_link_in_clipboard, Result<()>),
        (reset_feed_subscription_input, ()),
        (select_feeds, ()),
        (toggle_help, Result<()>),
        (toggle_read, Result<()>),
        (toggle_read_mode, Result<()>),
        (update_current_feed_and_entries, Result<()>),
    ];

    pub fn new(
        options: crate::Options,
        event_s: std::sync::mpsc::Sender<crate::Event<crossterm::event::KeyEvent>>,
    ) -> Result<App> {
        Ok(App {
            inner: Arc::new(Mutex::new(AppImpl::new(options, event_s)?)),
        })
    }

    pub fn draw(&self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();

        terminal.draw(|mut f| {
            let chunks = crate::ui::predraw(f);

            assert!(
                chunks.len() >= 2,
                "There must be at least two chunks in order to draw two columns"
            );

            let new_width = chunks[1].width;

            if inner.entry_column_width != new_width {
                inner.entry_column_width = new_width;
                inner.on_enter().unwrap_or_else(|e| {
                    inner.error_flash = vec![e];
                })
            }

            inner.entry_column_width = chunks[1].width;

            crate::ui::draw(&mut f, chunks, &mut inner);
        })?;

        Ok(())
    }

    pub fn on_key(&self, keycode: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match (keycode, modifiers) {
            // movement
            (KeyCode::Left, _) | (KeyCode::Char('h'), _) => self.on_left(),
            (KeyCode::Down, _) | (KeyCode::Char('j'), _) => self.on_down(),
            (KeyCode::Up, _) | (KeyCode::Char('k'), _) => self.on_up(),
            (KeyCode::Right, _) | (KeyCode::Char('l'), _) => self.on_right(),
            (KeyCode::PageUp, _) => {
                self.page_up();
                Ok(())
            }
            (KeyCode::PageDown, _) => {
                self.page_down();
                Ok(())
            }
            // modes, selections, editing, etc.
            (KeyCode::Enter, _) => self.on_enter(),
            (KeyCode::Char('?'), _) => self.toggle_help(),
            (KeyCode::Char('a'), _) => self.toggle_read_mode(),
            (KeyCode::Char('e'), _) | (KeyCode::Char('i'), _) => {
                let mut inner = self.inner.lock().unwrap();
                inner.mode = Mode::Editing;
                Ok(())
            }
            (KeyCode::Char('c'), _) => self.put_current_link_in_clipboard(),
            (KeyCode::Char('o'), _) => self.open_link_in_browser(),
            _ => Ok(()),
        }
    }

    pub fn set_flash(&self, flash: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.flash = Some(flash)
    }

    pub fn push_error_flash(&self, e: anyhow::Error) {
        let mut inner = self.inner.lock().unwrap();
        inner.error_flash.push(e);
    }

    pub fn set_mode(&self, mode: Mode) {
        let mut inner = self.inner.lock().unwrap();
        inner.mode = mode;
    }

    pub fn push_feed_subscription_input(&self, input: char) {
        let mut inner = self.inner.lock().unwrap();
        inner.feed_subscription_input.push(input);
    }

    pub fn set_feeds(&self, feeds: Vec<crate::rss::Feed>) {
        let mut inner = self.inner.lock().unwrap();
        let feeds = feeds.into();
        inner.feeds = feeds;
    }
}

#[derive(Debug)]
pub struct AppImpl {
    // database stuff
    pub conn: rusqlite::Connection,
    // network stuff
    pub http_client: ureq::Agent,
    // feed stuff
    pub current_feed: Option<crate::rss::Feed>,
    pub feeds: util::StatefulList<crate::rss::Feed>,
    // entry stuff
    pub current_entry_meta: Option<crate::rss::EntryMeta>,
    pub entries: util::StatefulList<crate::rss::EntryMeta>,
    pub entry_selection_position: usize,
    pub current_entry_text: String,
    pub entry_scroll_position: u16,
    pub entry_lines_len: usize,
    pub entry_lines_rendered_len: u16,
    pub entry_column_width: u16,
    // modes
    pub should_quit: bool,
    pub selected: Selected,
    pub mode: Mode,
    pub read_mode: ReadMode,
    pub show_help: bool,
    // misc
    pub error_flash: Vec<anyhow::Error>,
    pub feed_subscription_input: String,
    pub flash: Option<String>,
    event_s: std::sync::mpsc::Sender<crate::Event<crossterm::event::KeyEvent>>,
    is_wsl: Option<bool>,
}

impl AppImpl {
    pub fn new(
        options: crate::Options,
        event_s: std::sync::mpsc::Sender<crate::Event<crossterm::event::KeyEvent>>,
    ) -> Result<AppImpl> {
        let conn = rusqlite::Connection::open(&options.database_path)?;

        let http_client = ureq::AgentBuilder::new()
            .timeout_read(options.network_timeout)
            .build();

        crate::rss::initialize_db(&conn)?;
        let feeds: util::StatefulList<crate::rss::Feed> = vec![].into();
        let entries: util::StatefulList<crate::rss::EntryMeta> = vec![].into();
        let selected = Selected::Feeds;
        let initial_current_feed = None;

        let mut app = AppImpl {
            conn,
            http_client,
            should_quit: false,
            error_flash: vec![],
            feeds,
            entries,
            selected,
            entry_scroll_position: 0,
            entry_lines_len: 0,
            entry_lines_rendered_len: 0,
            entry_column_width: 0,
            current_entry_meta: None,
            current_entry_text: String::new(),
            current_feed: initial_current_feed,
            feed_subscription_input: String::new(),
            mode: Mode::Normal,
            read_mode: ReadMode::ShowUnread,
            show_help: true,
            entry_selection_position: 0,
            flash: None,
            event_s,
            is_wsl: None,
        };

        app.update_feeds()?;
        app.update_current_feed_and_entries()?;

        Ok(app)
    }

    pub fn update_feeds(&mut self) -> Result<()> {
        let feeds = crate::rss::get_feeds(&self.conn)?.into();
        self.feeds = feeds;
        Ok(())
    }

    pub fn update_current_feed_and_entries(&mut self) -> Result<()> {
        self.update_current_feed()?;
        self.update_current_entries()?;
        Ok(())
    }

    fn update_current_feed(&mut self) -> Result<()> {
        let current_feed = if self.feeds.items.is_empty() {
            None
        } else {
            let selected_idx = match self.feeds.state.selected() {
                Some(idx) => idx,
                None => {
                    self.feeds.reset();
                    0
                }
            };
            let feed_id = self.feeds.items[selected_idx].id;
            Some(crate::rss::get_feed(&self.conn, feed_id)?)
        };

        self.current_feed = current_feed;

        Ok(())
    }

    fn update_current_entries(&mut self) -> Result<()> {
        let entries = if let Some(feed) = &self.current_feed {
            crate::rss::get_entries_metas(&self.conn, &self.read_mode, feed.id)?
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
                None => self.entries.reset(),
            }
        }
        Ok(())
    }

    fn update_entry_selection_position(&mut self) {
        if self.entries.items.is_empty() {
            self.entry_selection_position = 0
        } else if self.entry_selection_position > self.entries.items.len() - 1 {
            self.entry_selection_position = self.entries.items.len() - 1
        };
    }

    fn get_selected_entry(&self) -> Option<Result<crate::rss::EntryContent>> {
        self.entries.state.selected().and_then(|selected_idx| {
            self.entries
                .items
                .get(selected_idx)
                .map(|item| item.id)
                .map(|entry_id| crate::rss::get_entry_content(&self.conn, entry_id))
        })
    }

    fn get_selected_entry_meta(&self) -> Option<Result<crate::rss::EntryMeta>> {
        self.entries.state.selected().and_then(|selected_idx| {
            self.entries
                .items
                .get(selected_idx)
                .map(|item| item.id)
                .map(|entry_id| crate::rss::get_entry_meta(&self.conn, entry_id))
        })
    }

    fn update_current_entry_meta(&mut self) -> Result<()> {
        if let Some(entry_meta) = self.get_selected_entry_meta() {
            let entry_meta = entry_meta?;
            self.current_entry_meta = Some(entry_meta);
        }
        Ok(())
    }

    fn page_up(&mut self) {
        if matches!(self.selected, Selected::Entry(_)) {
            self.entry_scroll_position = if let Some(position) = self
                .entry_scroll_position
                .checked_sub(self.entry_lines_rendered_len)
            {
                position
            } else {
                0
            };
        };
    }

    fn page_down(&mut self) {
        if matches!(self.selected, Selected::Entry(_)) {
            self.entry_scroll_position = if self.entry_scroll_position
                + self.entry_lines_rendered_len
                >= self.entry_lines_len as u16
            {
                self.entry_lines_len as u16
            } else {
                self.entry_scroll_position + self.entry_lines_rendered_len
            };
        }
    }

    pub fn on_enter(&mut self) -> Result<()> {
        match self.selected {
            Selected::Entries | Selected::Entry(_) => {
                if !self.entries.items.is_empty() {
                    if let Some(entry_meta) = &self.current_entry_meta {
                        if let Some(entry) = self.get_selected_entry() {
                            let entry = entry?;
                            let empty_string =
                                String::from("No content or description tag provided.");

                            // try content tag first,
                            // if there is not content tag,
                            // go to description tag,
                            // if no description tag,
                            // use empty string.
                            // TODO figure out what to actually do if there are neither
                            let entry_html = entry
                                .content
                                .as_ref()
                                .or_else(|| entry.description.as_ref())
                                .or(Some(&empty_string));

                            // minimum is 1
                            let line_length = if self.entry_column_width >= 5 {
                                self.entry_column_width - 4
                            } else {
                                1
                            };

                            if let Some(html) = entry_html {
                                let text =
                                    html2text::from_read(html.as_bytes(), line_length.into());
                                self.entry_lines_len = text.matches('\n').count();
                                self.current_entry_text = text;
                            } else {
                                self.current_entry_text = String::new();
                            }
                        }

                        self.selected = Selected::Entry(entry_meta.clone());
                    }
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn toggle_help(&mut self) -> Result<()> {
        self.show_help = !self.show_help;
        Ok(())
    }

    pub fn clear_error_flash(&mut self) {
        self.error_flash = vec![];
    }

    pub fn reset_feed_subscription_input(&mut self) {
        self.feed_subscription_input.clear();
    }

    pub fn pop_feed_subscription_input(&mut self) {
        self.feed_subscription_input.pop();
    }

    pub fn feed_subscription_input(&self) -> String {
        self.feed_subscription_input.clone()
    }

    pub fn error_flash_is_empty(&self) -> bool {
        self.error_flash.is_empty()
    }

    pub fn clear_flash(&mut self) {
        self.flash = None
    }

    pub fn select_feeds(&mut self) {
        self.selected = Selected::Feeds;
    }

    pub fn selected(&self) -> Selected {
        self.selected.clone()
    }

    pub fn selected_feed_id(&self) -> crate::rss::FeedId {
        let selected_idx = self.feeds.state.selected().unwrap();
        self.feeds.items[selected_idx].id
    }

    pub fn feed_ids(&self) -> Result<Vec<crate::rss::FeedId>> {
        let ids = crate::rss::get_feed_ids(&self.conn)?;
        Ok(ids)
    }

    pub fn toggle_read(&mut self) -> Result<()> {
        let selected = self.selected.clone();
        match selected {
            Selected::Entry(entry) => {
                entry.toggle_read(&self.conn)?;
                self.selected = Selected::Entries;
                self.update_current_entries()?;
                self.update_current_entry_meta()?;
                self.entry_scroll_position = 0;
            }
            Selected::Entries => {
                if let Some(entry_meta) = &self.current_entry_meta {
                    entry_meta.toggle_read(&self.conn)?;
                    self.update_current_entries()?;
                    self.update_current_entry_meta()?;
                    self.update_entry_selection_position();
                }
            }
            Selected::Feeds => (),
        }

        Ok(())
    }

    pub fn http_client(&self) -> ureq::Agent {
        // this is cheap because it only clones a struct containing two Arcs
        self.http_client.clone()
    }

    pub fn toggle_read_mode(&mut self) -> Result<()> {
        match (&self.read_mode, &self.selected) {
            (ReadMode::ShowRead, Selected::Feeds) | (ReadMode::ShowRead, Selected::Entries) => {
                self.entry_selection_position = 0;
                self.read_mode = ReadMode::ShowUnread
            }
            (ReadMode::ShowUnread, Selected::Feeds) | (ReadMode::ShowUnread, Selected::Entries) => {
                self.entry_selection_position = 0;
                self.read_mode = ReadMode::ShowRead
            }
            _ => (),
        }
        self.update_current_entries()?;

        if !self.entries.items.is_empty() {
            self.entries.reset();
        } else {
            self.entries.unselect();
        }

        self.update_current_entry_meta()?;

        Ok(())
    }

    fn get_current_link(&self) -> String {
        match &self.selected {
            Selected::Feeds => {
                let feed = self.current_feed.clone().unwrap();
                feed.link.clone().unwrap_or_else(|| feed.feed_link.unwrap())
            }
            Selected::Entries => {
                if let Some(entry) = self.entries.items.get(self.entry_selection_position) {
                    entry.link.clone().unwrap_or_else(|| "".to_string())
                } else {
                    "".to_string()
                }
            }
            Selected::Entry(e) => e.link.clone().unwrap_or_else(|| "".to_string()),
        }
    }

    fn put_current_link_in_clipboard(&mut self) -> Result<()> {
        let current_link = self.get_current_link();

        if self.is_wsl() {
            #[cfg(target_os = "linux")]
            {
                util::set_wsl_clipboard_contents(&current_link)
            }

            #[cfg(not(target_os = "linux"))]
            {
                unreachable!("This should never happen. This code should only be reachable if the target OS is WSL.")
            }
        } else {
            let mut ctx = ClipboardContext::new().map_err(|e| anyhow::anyhow!(e))?;
            ctx.set_contents(current_link)
                .map_err(|e| anyhow::anyhow!(e))
        }
    }

    fn open_link_in_browser(&self) -> Result<()> {
        webbrowser::open_browser_with_options(webbrowser::BrowserOptions::create_with_suppressed_output(
            &self.get_current_link(),
        ))
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!(e))
    }

    fn is_wsl(&mut self) -> bool {
        if let Some(is_wsl) = self.is_wsl {
            is_wsl
        } else {
            self.is_wsl = Some(wsl::is_wsl());
            self.is_wsl.unwrap()
        }
    }

    pub fn on_left(&mut self) -> Result<()> {
        match self.selected {
            Selected::Feeds => (),
            Selected::Entries => {
                self.entry_selection_position = 0;
                self.selected = Selected::Feeds
            }
            Selected::Entry(_) => {
                self.entry_scroll_position = 0;
                self.selected = {
                    self.current_entry_text = String::new();
                    Selected::Entries
                }
            }
        }

        Ok(())
    }

    pub fn on_up(&mut self) -> Result<()> {
        match self.selected {
            Selected::Feeds => {
                self.feeds.previous();
                self.update_current_feed_and_entries()?;
            }
            Selected::Entries => {
                if !self.entries.items.is_empty() {
                    self.entries.previous();
                    self.entry_selection_position = self.entries.state.selected().unwrap();
                    self.update_current_entry_meta()?;
                }
            }
            Selected::Entry(_) => {
                if let Some(n) = self.entry_scroll_position.checked_sub(1) {
                    self.entry_scroll_position = n
                };
            }
        }

        Ok(())
    }

    pub fn on_right(&mut self) -> Result<()> {
        match self.selected {
            Selected::Feeds => {
                if !self.entries.items.is_empty() {
                    self.selected = Selected::Entries;
                    self.entries.reset();
                    self.update_current_entry_meta()?;
                }
                Ok(())
            }
            Selected::Entries => self.on_enter(),
            Selected::Entry(_) => Ok(()),
        }
    }

    pub fn on_down(&mut self) -> Result<()> {
        match self.selected {
            Selected::Feeds => {
                self.feeds.next();
                self.update_current_feed_and_entries()?;
            }
            Selected::Entries => {
                if !self.entries.items.is_empty() {
                    self.entries.next();
                    self.entry_selection_position = self.entries.state.selected().unwrap();
                    self.update_current_entry_meta()?;
                }
            }
            Selected::Entry(_) => {
                if let Some(n) = self.entry_scroll_position.checked_add(1) {
                    self.entry_scroll_position = n
                };
            }
        }

        Ok(())
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn force_redraw(&self) -> Result<()> {
        self.event_s.send(crate::Event::Tick).map_err(|e| e.into())
    }
}
