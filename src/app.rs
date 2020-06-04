use crate::error::Error;
use crate::modes::{Mode, ReadMode, Selected};
use crate::util;

#[derive(Debug)]
pub struct App<'app> {
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
    pub current_entry_text: Vec<tui::widgets::Text<'app>>,
    pub entry_scroll_position: u16,
    // modes
    pub should_quit: bool,
    pub selected: Selected,
    pub mode: Mode,
    pub read_mode: ReadMode,
    // misc
    pub error_flash: Option<Error>,
    pub feed_subscription_input: String,
}

impl<'app> App<'app> {
    pub fn new(options: crate::Options) -> Result<App<'app>, Error> {
        let conn = rusqlite::Connection::open(&options.database_path)?;
        crate::rss::initialize_db(&conn)?;
        let initial_feed_titles = vec![].into();
        let selected = Selected::Feeds;
        let initial_current_feed = None;
        let initial_entries = vec![].into();

        let mut app = App {
            conn,
            line_length: options.line_length,
            should_quit: false,
            error_flash: None,
            feeds: initial_feed_titles,
            entries: initial_entries,
            selected,
            entry_scroll_position: 0,
            current_entry: None,
            current_entry_text: vec![],
            current_feed: initial_current_feed,
            feed_subscription_input: String::new(),
            mode: Mode::Normal,
            read_mode: ReadMode::ShowUnread,
            entry_selection_position: 0,
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
            self.entries
                .state
                .select(Some(self.entries.items.len() - 1))
        }
        Ok(())
    }

    pub async fn select_feeds(&mut self) {
        self.selected = Selected::Feeds;
    }

    pub async fn subscribe_to_feed(&mut self) -> Result<(), Error> {
        let _feed_id =
            crate::rss::subscribe_to_feed(&self.conn, &self.feed_subscription_input).await?;
        let feeds = crate::rss::get_feeds(&self.conn)?.into();
        self.feeds = feeds;
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

    pub fn on_up(&mut self) -> Result<(), Error> {
        match self.selected {
            Selected::Feeds => {
                self.feeds.previous();
                self.update_current_feed_and_entries()?;
            }
            Selected::Entries => {
                if !self.entries.items.is_empty() {
                    self.entries.previous();
                    self.entry_selection_position = self.entries.state.selected().unwrap();
                    if let Some(entry) = self.get_selected_entry() {
                        let entry = entry?;
                        self.current_entry = Some(entry);
                    }
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

    pub fn on_down(&mut self) -> Result<(), Error> {
        match self.selected {
            Selected::Feeds => {
                self.feeds.next();
                self.update_current_feed_and_entries()?;
            }
            Selected::Entries => {
                if !self.entries.items.is_empty() {
                    self.entries.next();
                    self.entry_selection_position = self.entries.state.selected().unwrap();
                    if let Some(entry) = self.get_selected_entry() {
                        let entry = entry?;
                        self.current_entry = Some(entry);
                    }
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

    pub fn on_right(&mut self) -> Result<(), Error> {
        match self.selected {
            Selected::Feeds => {
                if !self.entries.items.is_empty() {
                    self.selected = Selected::Entries;
                    self.entries.state.select(Some(0));
                    if let Some(entry) = self.get_selected_entry() {
                        let entry = entry?;
                        self.current_entry = Some(entry);
                    }
                }
                Ok(())
            }
            Selected::Entries => self.on_enter(),
            Selected::Entry(_) => Ok(()),
        }
    }

    pub fn on_left(&mut self) {
        match self.selected {
            Selected::Feeds => (),
            Selected::Entries => self.selected = Selected::Feeds,
            Selected::Entry(_) => {
                self.entry_scroll_position = 0;
                self.selected = {
                    self.current_entry_text = vec![];
                    Selected::Entries
                }
            }
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

                            let text = text
                                .split('\n')
                                .map(|line| {
                                    tui::widgets::Text::raw({
                                        let mut owned = line.to_owned();
                                        owned.push_str("\n");
                                        owned
                                    })
                                })
                                .collect::<Vec<_>>();

                            self.current_entry_text = text;
                        } else {
                            self.current_entry_text = vec![];
                        }

                        self.selected = Selected::Entry(entry.clone());
                    }
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn on_esc(&mut self) {
        match self.selected {
            Selected::Entry(_) => self.selected = Selected::Entries,
            Selected::Entries => (),
            Selected::Feeds => (),
        }
    }

    pub async fn on_refresh(&mut self) -> Result<(), Error> {
        let selected_idx = self.feeds.state.selected().unwrap();
        let feed_id = self.feeds.items[selected_idx].id;
        let _ = crate::rss::refresh_feed(&self.conn, feed_id).await?;
        self.update_current_feed_and_entries()?;
        Ok(())
    }

    pub async fn toggle_read(&mut self) -> Result<(), Error> {
        match &self.selected {
            Selected::Entry(entry) => {
                entry.toggle_read(&self.conn).await?;
                self.update_current_entries()?;
                if let Some(entry) = self.get_selected_entry() {
                    let entry = entry?;
                    self.current_entry = Some(entry);
                }
                self.selected = Selected::Entries;
                self.entry_scroll_position = 0;
            }
            Selected::Entries => {
                if let Some(entry) = &self.current_entry {
                    entry.toggle_read(&self.conn).await?;
                    self.update_current_entries()?;
                    if let Some(entry) = self.get_selected_entry() {
                        let entry = entry?;
                        self.current_entry = Some(entry);
                    }
                }
            }
            Selected::Feeds => (),
        }

        Ok(())
    }

    pub async fn toggle_read_mode(&mut self) -> Result<(), Error> {
        match (&self.read_mode, &self.selected) {
            (ReadMode::ShowRead, Selected::Feeds) | (ReadMode::ShowRead, Selected::Entries) => {
                self.read_mode = ReadMode::ShowUnread
            }
            (ReadMode::ShowUnread, Selected::Feeds) | (ReadMode::ShowUnread, Selected::Entries) => {
                self.read_mode = ReadMode::ShowRead
            }
            _ => (),
        }
        self.update_current_entries()?;

        if !self.entries.items.is_empty() {
            self.entries.state.select(Some(0));
        } else {
            self.entries.state.select(None);
        }

        if let Some(entry) = self.get_selected_entry() {
            let entry = entry?;
            self.current_entry = Some(entry);
        }

        Ok(())
    }

    pub async fn on_key(&mut self, c: char) -> Result<(), Error> {
        match c {
            'q' => {
                self.should_quit = true;
            }
            // vim-style movement
            'h' => self.on_left(),
            'j' => self.on_down()?,
            'k' => self.on_up()?,
            'l' => self.on_right().unwrap(),
            // controls
            'r' => match self.selected {
                Selected::Feeds => return self.on_refresh().await,
                _ => return self.toggle_read().await,
            },
            'a' => self.toggle_read_mode().await?,
            'e' | 'i' => {
                self.mode = Mode::Editing;
            }
            _ => (),
        }

        Ok(())
    }
}
