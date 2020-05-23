use crate::error::Error;
use crate::util;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Selected {
    Feeds,
    Entries,
    Entry(crate::rss::Entry),
}

#[derive(Debug)]
pub(crate) struct App<'a> {
    pub title: &'a str,
    pub database_path: PathBuf,
    pub conn: rusqlite::Connection,
    pub enhanced_graphics: bool,
    pub should_quit: bool,
    pub progress: f64,
    pub error_flash: Option<Error>,
    pub feed_titles: util::StatefulList<(i64, String)>,
    pub entries_titles: util::StatefulList<(i64, String)>,
    pub selected: Selected,
    pub scroll: u16,
    pub current_entry_text: Vec<tui::widgets::Text<'a>>,
}

impl<'a> App<'a> {
    pub(crate) async fn new(
        title: &'a str,
        database_path: PathBuf,
        enhanced_graphics: bool,
    ) -> Result<App<'a>, Error> {
        let conn = rusqlite::Connection::open(&database_path)?;
        crate::rss::initialize_db(&conn)?;
        // crate::rss::subscribe_to_feed(&conn, "https://zeroclarkthirty.com/feed").await?;
        // crate::rss::subscribe_to_feed(&conn, "https://danielmiessler.com/feed/").await?;
        let feed_titles = util::StatefulList::with_items(crate::rss::get_feed_titles(&conn)?);
        let entries_titles = util::StatefulList::with_items(vec![]);

        let selected = Selected::Feeds;

        let mut app = App {
            title,
            database_path,
            conn,
            enhanced_graphics,
            progress: 0.0,
            should_quit: false,
            error_flash: None,
            feed_titles,
            entries_titles,
            selected,
            scroll: 0,
            current_entry_text: vec![],
        };

        app.on_down();
        let selected_idx = app.feed_titles.state.selected().unwrap();
        let feed_id = app.feed_titles.items[selected_idx].0;

        let entries_titles = crate::rss::get_entries_titles(&app.conn, feed_id)?
            .into_iter()
            .collect::<Vec<_>>();

        app.entries_titles = util::StatefulList::with_items(entries_titles);

        Ok(app)
    }

    pub fn on_up(&mut self) {
        match self.selected {
            Selected::Feeds => {
                self.feed_titles.previous();

                let selected_idx = self.feed_titles.state.selected().unwrap();
                let feed_id = self.feed_titles.items[selected_idx].0;

                let entries_titles = crate::rss::get_entries_titles(&self.conn, feed_id)
                    .unwrap()
                    .into_iter()
                    .collect::<Vec<_>>();

                self.entries_titles = util::StatefulList::with_items(entries_titles);
            }
            Selected::Entries => {
                self.entries_titles.previous();
            }
            Selected::Entry(_) => {
                match self.scroll.checked_sub(1) {
                    Some(n) => self.scroll = n,
                    None => (),
                };
            }
        }
    }

    pub fn on_down(&mut self) {
        match self.selected {
            Selected::Feeds => {
                self.feed_titles.next();

                let selected_idx = self.feed_titles.state.selected().unwrap();
                let feed_id = self.feed_titles.items[selected_idx].0;

                let entries_titles = crate::rss::get_entries_titles(&self.conn, feed_id)
                    .unwrap()
                    .into_iter()
                    .collect::<Vec<_>>();

                self.entries_titles = util::StatefulList::with_items(entries_titles);
            }
            Selected::Entries => {
                self.entries_titles.next();
            }
            Selected::Entry(_) => {
                match self.scroll.checked_add(1) {
                    Some(n) => self.scroll = n,
                    None => (),
                };
            }
        }
    }

    pub fn on_right(&mut self) -> Result<(), Error> {
        match self.selected {
            Selected::Feeds => {
                self.selected = Selected::Entries;
                self.entries_titles.state.select(Some(0));
                Ok(())
            }
            Selected::Entries => self.on_enter(),
            Selected::Entry(_) => Ok(()),
        }
    }

    pub fn on_left(&mut self) {
        match self.selected {
            Selected::Feeds => {
                self.selected = Selected::Entries;
                self.entries_titles.state.select(Some(0));
            }
            Selected::Entries => self.selected = Selected::Feeds,
            Selected::Entry(_) => {
                self.scroll = 0;
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
                let selected_idx = self.entries_titles.state.selected().unwrap();
                let entry_id = self.entries_titles.items[selected_idx].0;
                let entry = crate::rss::get_entry(&self.conn, entry_id)?;

                let text = html2text::from_read(
                    entry
                        .content
                        .clone()
                        .unwrap_or_else(|| String::new())
                        .as_bytes(),
                    120,
                );

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

                self.selected = Selected::Entry(entry);
                self.current_entry_text = text;

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

    pub fn on_key(&mut self, c: char) {
        match c {
            'q' => {
                self.should_quit = true;
            }
            // vim style
            'h' => self.on_left(),
            'j' => self.on_down(),
            'k' => self.on_up(),
            'l' => self.on_right().unwrap(),
            _ => {}
        }
    }

    pub fn on_tick(&mut self) {
        // Update progress
        self.progress += 0.001;
        if self.progress > 1.0 {
            self.progress = 0.0;
        }
    }
}
