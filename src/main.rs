#![forbid(unsafe_code)]

use crate::modes::{Mode, Selected};
use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use crossterm::event::{self, KeyEvent};
use crossterm::event::{Event as CEvent, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::mpsc;
use std::{thread, time};

mod app;
mod modes;
mod opml;
mod rss;
mod ui;
mod util;

fn main() -> Result<()> {
    let options = Options::parse();

    let validated_options = options.subcommand.validate()?;

    match validated_options {
        ValidatedOptions::Import(options) => crate::opml::import(options),
        ValidatedOptions::Read(options) => run_reader(options),
    }
}

pub enum Event<I> {
    Input(I),
    Tick,
}

/// A TUI RSS reader with vim-like controls and a local-first, offline-first focus
#[derive(Debug, Parser)]
#[command(author, version, about, name = "russ")]
struct Options {
    #[command(subcommand)]
    subcommand: Command,
}

/// Only used to take input at the boundary.
/// Turned into `ValidatedOptions` with `validate()`.
#[derive(Debug, Subcommand)]
enum Command {
    /// Read your feeds
    Read {
        /// Override where `russ` stores and reads feeds.
        /// By default, the feeds database on Linux this will be at `XDG_DATA_HOME/russ/feeds.db` or `$HOME/.local/share/russ/feeds.db`.
        /// On MacOS it will be at `$HOME/Library/Application Support/russ/feeds.db`.
        /// On Windows it will be at `{FOLDERID_LocalAppData}/russ/data/feeds.db`.
        #[arg(short, long)]
        database_path: Option<PathBuf>,
        /// time in ms between two ticks
        #[arg(short, long, default_value = "250")]
        tick_rate: u64,
        /// number of seconds to show the flash message before clearing it
        #[arg(short, long, default_value = "4", value_parser = parse_seconds)]
        flash_display_duration_seconds: time::Duration,
        /// RSS/Atom network request timeout in seconds
        #[arg(short, long, default_value = "5", value_parser = parse_seconds)]
        network_timeout: time::Duration,
    },
    /// Import feeds from an OPML document
    Import {
        /// Override where `russ` stores and reads feeds.
        /// By default, the feeds database on Linux this will be at `XDG_DATA_HOME/russ/feeds.db` or `$HOME/.local/share/russ/feeds.db`.
        /// On MacOS it will be at `$HOME/Library/Application Support/russ/feeds.db`.
        /// On Windows it will be at `{FOLDERID_LocalAppData}/russ/data/feeds.db`.
        #[arg(short, long)]
        database_path: Option<PathBuf>,
        #[arg(short, long)]
        opml_path: PathBuf,
        /// RSS/Atom network request timeout in seconds
        #[arg(short, long, default_value = "5", value_parser = parse_seconds)]
        network_timeout: time::Duration,
    },
}

impl Command {
    fn validate(&self) -> std::io::Result<ValidatedOptions> {
        match self {
            Command::Read {
                database_path,
                tick_rate,
                flash_display_duration_seconds,
                network_timeout,
            } => {
                let database_path = get_database_path(database_path)?;

                Ok(ValidatedOptions::Read(ReadOptions {
                    database_path,
                    tick_rate: *tick_rate,
                    flash_display_duration_seconds: *flash_display_duration_seconds,
                    network_timeout: *network_timeout,
                }))
            }
            Command::Import {
                database_path,
                opml_path,
                network_timeout,
            } => {
                let database_path = get_database_path(database_path)?;
                Ok(ValidatedOptions::Import(ImportOptions {
                    database_path,
                    opml_path: opml_path.to_owned(),
                    network_timeout: *network_timeout,
                }))
            }
        }
    }
}

fn parse_seconds(s: &str) -> Result<time::Duration, std::num::ParseIntError> {
    let as_u64 = s.parse::<u64>()?;
    Ok(time::Duration::from_secs(as_u64))
}

/// internal, validated options for the normal reader mode
#[derive(Debug)]
enum ValidatedOptions {
    Read(ReadOptions),
    Import(ImportOptions),
}

#[derive(Clone, Debug)]
struct ReadOptions {
    database_path: PathBuf,
    tick_rate: u64,
    flash_display_duration_seconds: time::Duration,
    network_timeout: time::Duration,
}

#[derive(Debug)]
struct ImportOptions {
    database_path: PathBuf,
    opml_path: PathBuf,
    network_timeout: time::Duration,
}

fn get_database_path(database_path: &Option<PathBuf>) -> std::io::Result<PathBuf> {
    let database_path = if let Some(database_path) = database_path {
        database_path.to_owned()
    } else {
        let mut database_path = directories::ProjectDirs::from("", "", "russ")
            .expect("unable to find home directory. if you like, you can provide a database path directly by passing the -d option.")
            .data_local_dir()
            .to_path_buf();

        std::fs::create_dir_all(&database_path)?;

        database_path.push("feeds.db");

        database_path
    };

    Ok(database_path)
}

enum IoCommand {
    Break,
    RefreshFeed(crate::rss::FeedId),
    RefreshFeeds(Vec<crate::rss::FeedId>),
    SubscribeToFeed(String),
    ClearFlash,
}

fn io_loop(
    app: App,
    tx: mpsc::Sender<IoCommand>,
    rx: mpsc::Receiver<IoCommand>,
    options: &ReadOptions,
) -> Result<()> {
    use IoCommand::*;

    let manager = r2d2_sqlite::SqliteConnectionManager::file(&options.database_path);
    let connection_pool = r2d2::Pool::new(manager)?;

    while let Ok(event) = rx.recv() {
        match event {
            Break => break,
            RefreshFeed(feed_id) => {
                let now = std::time::Instant::now();

                app.set_flash("Refreshing feed...".to_string());
                app.force_redraw()?;

                refresh_feeds(&app, &connection_pool, &[feed_id], |_app, fetch_result| {
                    if let Err(e) = fetch_result {
                        app.push_error_flash(e)
                    }
                })?;

                app.update_current_feed_and_entries()?;
                let elapsed = now.elapsed();
                app.set_flash(format!("Refreshed feed in {elapsed:?}"));
                app.force_redraw()?;
                clear_flash_after(tx.clone(), options.flash_display_duration_seconds);
            }
            RefreshFeeds(feed_ids) => {
                let now = std::time::Instant::now();

                app.set_flash("Refreshing all feeds...".to_string());
                app.force_redraw()?;

                let all_feeds_len = feed_ids.len();
                let mut successfully_refreshed_len = 0usize;

                refresh_feeds(&app, &connection_pool, &feed_ids, |app, fetch_result| {
                    match fetch_result {
                        Ok(_) => successfully_refreshed_len += 1,
                        Err(e) => app.push_error_flash(e),
                    }
                })?;

                {
                    app.update_current_feed_and_entries()?;

                    let elapsed = now.elapsed();
                    app.set_flash(format!(
                        "Refreshed {successfully_refreshed_len}/{all_feeds_len} feeds in {elapsed:?}"
                    ));
                    app.force_redraw()?;
                }

                clear_flash_after(tx.clone(), options.flash_display_duration_seconds);
            }
            SubscribeToFeed(feed_subscription_input) => {
                let now = std::time::Instant::now();

                app.set_flash("Subscribing to feed...".to_string());
                app.force_redraw()?;

                let mut conn = connection_pool.get()?;
                let r = crate::rss::subscribe_to_feed(
                    &app.http_client(),
                    &mut conn,
                    &feed_subscription_input,
                );

                if let Err(e) = r {
                    app.push_error_flash(e);
                    continue;
                }

                match crate::rss::get_feeds(&conn) {
                    Ok(feeds) => {
                        {
                            app.reset_feed_subscription_input();
                            app.set_feeds(feeds);
                            app.select_feeds();
                            app.update_current_feed_and_entries()?;

                            let elapsed = now.elapsed();
                            app.set_flash(format!("Subscribed in {elapsed:?}"));
                            app.set_mode(Mode::Normal);
                            app.force_redraw()?;
                        }

                        clear_flash_after(tx.clone(), options.flash_display_duration_seconds);
                    }
                    Err(e) => {
                        app.push_error_flash(e);
                    }
                }
            }
            ClearFlash => {
                app.clear_flash();
            }
        }
    }

    Ok(())
}

fn refresh_feeds<F>(
    app: &App,
    connection_pool: &r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
    feed_ids: &[crate::rss::FeedId],
    mut refresh_result_handler: F,
) -> Result<()>
where
    F: FnMut(&App, anyhow::Result<()>),
{
    let min_number_of_threads = num_cpus::get() * 2;
    let chunk_size = feed_ids.len() / min_number_of_threads;
    // due to usize floor division, it's possible chunk_size would be 0,
    // so ensure it is at least 1
    let chunk_size = chunk_size.max(1);
    let chunks = feed_ids.chunks(chunk_size);

    let join_handles: Vec<_> = chunks
        .map(|chunk_feed_ids| {
            let pool_get_result = connection_pool.get();
            let http = app.http_client();
            let chunk_feed_ids = chunk_feed_ids.to_owned();

            thread::spawn(move || -> Result<Vec<Result<(), anyhow::Error>>> {
                let mut results = vec![];
                let mut conn = pool_get_result?;

                for feed_id in chunk_feed_ids.into_iter() {
                    results.push(crate::rss::refresh_feed(&http, &mut conn, feed_id))
                }

                Ok::<Vec<Result<(), anyhow::Error>>, anyhow::Error>(results)
            })
        })
        .collect();

    for join_handle in join_handles {
        let chunk_results = join_handle
            .join()
            .expect("unable to join worker thread to io thread");
        for chunk_result in chunk_results? {
            refresh_result_handler(app, chunk_result)
        }
    }

    Ok(())
}

fn clear_flash_after(tx: mpsc::Sender<IoCommand>, duration: time::Duration) {
    thread::spawn(move || {
        thread::sleep(duration);
        tx.send(IoCommand::ClearFlash)
            .expect("Unable to send IOCommand::ClearFlash");
    });
}

fn run_reader(options: ReadOptions) -> Result<()> {
    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup input handling
    let (event_tx, event_rx) = mpsc::channel();

    let event_tx_clone = event_tx.clone();

    let tick_rate = time::Duration::from_millis(options.tick_rate);

    thread::spawn(move || {
        let mut last_tick = time::Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            if event::poll(tick_rate - last_tick.elapsed())
                .expect("Unable to poll for Crossterm event")
            {
                if let CEvent::Key(key) = event::read().expect("Unable to read Crossterm event") {
                    event_tx
                        .send(Event::Input(key))
                        .expect("Unable to send Crossterm Key input event");
                }
            }
            if last_tick.elapsed() >= tick_rate {
                event_tx.send(Event::Tick).expect("Unable to send tick");
                last_tick = time::Instant::now();
            }
        }
    });

    let options_clone = options.clone();

    let (io_tx, io_rx) = mpsc::channel();

    let io_tx_clone = io_tx.clone();

    let mut app = App::new(options, event_tx_clone, io_tx)?;

    let cloned_app = app.clone();

    terminal.clear()?;

    // spawn this thread to handle receiving messages to performing blocking network and db IO
    let io_thread = thread::spawn(move || -> Result<()> {
        io_loop(cloned_app, io_tx_clone, io_rx, &options_clone)
    });

    // this is basically "the Elm Architecture".
    //
    // more or less:
    // ui <- current_state
    // action <- current_state + event
    // new_state <- current_state + action
    loop {
        app.draw(&mut terminal)?;

        let event = event_rx.recv()?;

        let action = get_action(&app, event);

        if let Some(action) = action {
            update(&mut app, action)?;
        }

        if app.should_quit() {
            app.break_io_thread()?;
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;
            break;
        }
    }

    io_thread
        .join()
        .expect("Unable to join IO thread to main thread")?;

    Ok(())
}

enum Action {
    Quit,
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    PageUp,
    PageDown,
    RefreshAll,
    RefreshFeed,
    ToggleHelp,
    ToggleReadMode,
    EnterEditingMode,
    OpenLinkInBrowser,
    CopyLinkToClipboard,
    Tick,
    SubscribeToFeed,
    PushInputChar(char),
    DeleteInputChar,
    DeleteFeed,
    EnterNormalMode,
    ClearErrorFlash,
    SelectAndShowCurrentEntry,
    ToggleReadStatus,
}

fn get_action(app: &App, event: Event<KeyEvent>) -> Option<Action> {
    match app.mode() {
        Mode::Normal => match event {
            Event::Input(keypress) => match (keypress.code, keypress.modifiers) {
                (KeyCode::Char('q'), _)
                | (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Esc, _) => {
                    if !app.error_flash_is_empty() {
                        Some(Action::ClearErrorFlash)
                    } else {
                        Some(Action::Quit)
                    }
                }
                (KeyCode::Char('r'), KeyModifiers::NONE) => match app.selected() {
                    Selected::Feeds => Some(Action::RefreshFeed),
                    _ => Some(Action::ToggleReadStatus),
                },
                (KeyCode::Char('x'), KeyModifiers::NONE) => Some(Action::RefreshAll),
                (KeyCode::Left, _) | (KeyCode::Char('h'), _) => Some(Action::MoveLeft),
                (KeyCode::Right, _) | (KeyCode::Char('l'), _) => Some(Action::MoveRight),
                (KeyCode::Down, _) | (KeyCode::Char('j'), _) => Some(Action::MoveDown),
                (KeyCode::Up, _) | (KeyCode::Char('k'), _) => Some(Action::MoveUp),
                (KeyCode::PageUp, _) | (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                    Some(Action::PageUp)
                }
                (KeyCode::PageDown, _) | (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    Some(Action::PageDown)
                }
                (KeyCode::Enter, _) => match app.selected() {
                    Selected::Entries | Selected::Entry(_) => {
                        if app.has_entries() && app.has_current_entry() {
                            Some(Action::SelectAndShowCurrentEntry)
                        } else {
                            None
                        }
                    }
                    _ => None,
                },
                (KeyCode::Char('?'), _) => Some(Action::ToggleHelp),
                (KeyCode::Char('a'), _) => Some(Action::ToggleReadMode),
                (KeyCode::Char('e'), _) | (KeyCode::Char('i'), _) => Some(Action::EnterEditingMode),
                (KeyCode::Char('c'), _) => Some(Action::CopyLinkToClipboard),
                (KeyCode::Char('o'), _) => Some(Action::OpenLinkInBrowser),
                _ => None,
            },
            Event::Tick => Some(Action::Tick),
        },
        Mode::Editing => match event {
            Event::Input(keypress) => match keypress.code {
                KeyCode::Enter => {
                    if !app.feed_subscription_input_is_empty() {
                        Some(Action::SubscribeToFeed)
                    } else {
                        None
                    }
                }
                KeyCode::Char(c) => Some(Action::PushInputChar(c)),
                KeyCode::Backspace => Some(Action::DeleteInputChar),
                KeyCode::Delete => Some(Action::DeleteFeed),
                KeyCode::Esc => Some(Action::EnterNormalMode),
                _ => None,
            },
            Event::Tick => Some(Action::Tick),
        },
    }
}

fn update(app: &mut App, action: Action) -> Result<()> {
    match action {
        Action::Tick => (),
        Action::Quit => app.set_should_quit(true),
        Action::RefreshAll => app.refresh_feeds()?,
        Action::RefreshFeed => app.refresh_feed()?,
        Action::MoveLeft => app.on_left()?,
        Action::MoveDown => app.on_down()?,
        Action::MoveUp => app.on_up()?,
        Action::MoveRight => app.on_right()?,
        Action::PageUp => app.page_up(),
        Action::PageDown => app.page_down(),
        Action::ToggleHelp => app.toggle_help()?,
        Action::ToggleReadMode => app.toggle_read_mode()?,
        Action::ToggleReadStatus => app.toggle_read()?,
        Action::EnterEditingMode => app.set_mode(Mode::Editing),
        Action::CopyLinkToClipboard => app.put_current_link_in_clipboard()?,
        Action::OpenLinkInBrowser => app.open_link_in_browser()?,
        Action::SubscribeToFeed => app.subscribe_to_feed()?,
        Action::PushInputChar(c) => app.push_feed_subscription_input(c),
        Action::DeleteInputChar => app.pop_feed_subscription_input(),
        Action::DeleteFeed => app.delete_feed()?,
        Action::EnterNormalMode => app.set_mode(Mode::Normal),
        Action::ClearErrorFlash => app.clear_error_flash(),
        Action::SelectAndShowCurrentEntry => app.select_and_show_current_entry()?,
    };

    Ok(())
}
