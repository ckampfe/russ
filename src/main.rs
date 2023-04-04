#![forbid(unsafe_code)]

use crate::modes::{Mode, Selected};
use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::event;
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
mod rss;
mod ui;
mod util;

pub enum Event<I> {
    Input(I),
    Tick,
}

// Only used to take input at the boundary.
// Turned into `Options` with `to_options()`.
/// A TUI RSS reader with vim-like controls and a local-first, offline-first focus
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, name = "russ")]
struct CliOptions {
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
}

impl CliOptions {
    fn to_options(&self) -> std::io::Result<Options> {
        let database_path = get_database_path(self)?;

        Ok(Options {
            database_path,
            tick_rate: self.tick_rate,
            flash_display_duration_seconds: self.flash_display_duration_seconds,
            network_timeout: self.network_timeout,
        })
    }
}

fn parse_seconds(s: &str) -> Result<time::Duration, std::num::ParseIntError> {
    let as_u64 = s.parse::<u64>()?;
    Ok(time::Duration::from_secs(as_u64))
}

/// internal, validated options
#[derive(Clone, Debug)]
pub struct Options {
    /// feed database path
    database_path: PathBuf,
    /// time in ms between two ticks
    tick_rate: u64,
    /// number of seconds to show the flash message before clearing it
    flash_display_duration_seconds: time::Duration,
    /// RSS/Atom network request timeout in seconds
    network_timeout: time::Duration,
}

fn get_database_path(cli_options: &CliOptions) -> std::io::Result<PathBuf> {
    let database_path = if let Some(database_path) = cli_options.database_path.as_ref() {
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
    sx: mpsc::Sender<IoCommand>,
    rx: mpsc::Receiver<IoCommand>,
    options: &Options,
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
                clear_flash_after(sx.clone(), options.flash_display_duration_seconds);
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

                clear_flash_after(sx.clone(), options.flash_display_duration_seconds);
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

                        clear_flash_after(sx.clone(), options.flash_display_duration_seconds);
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

fn clear_flash_after(sx: mpsc::Sender<IoCommand>, duration: time::Duration) {
    thread::spawn(move || {
        thread::sleep(duration);
        sx.send(IoCommand::ClearFlash)
            .expect("Unable to send IOCommand::ClearFlash");
    });
}

fn main() -> Result<()> {
    let cli_options: CliOptions = CliOptions::parse();

    let options = cli_options.to_options()?;

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup input handling
    let (tx, rx) = mpsc::channel();
    let tx_clone = tx.clone();

    let tick_rate = time::Duration::from_millis(options.tick_rate);
    thread::spawn(move || {
        let mut last_tick = time::Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            if event::poll(tick_rate - last_tick.elapsed())
                .expect("Unable to poll for Crossterm event")
            {
                if let CEvent::Key(key) = event::read().expect("Unable to read Crossterm event") {
                    tx.send(Event::Input(key))
                        .expect("Unable to send Crossterm Key input event");
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).expect("Unable to send tick");
                last_tick = time::Instant::now();
            }
        }
    });

    let options_clone = options.clone();

    let app = App::new(options, tx_clone)?;

    let cloned_app = app.clone();

    terminal.clear()?;

    let (io_s, io_r) = mpsc::channel();

    let io_s_clone = io_s.clone();

    // spawn this thread to handle receiving messages to performing blocking network and db IO
    let io_thread = thread::spawn(move || -> Result<()> {
        io_loop(cloned_app, io_s_clone, io_r, &options_clone)
    });

    // MAIN THREAD IS DRAW THREAD
    loop {
        let mode = {
            app.draw(&mut terminal)?;
            app.mode()
        };

        match mode {
            Mode::Normal => match rx.recv()? {
                Event::Input(event) => match (event.code, event.modifiers) {
                    // These first few keycodes are handled inline
                    // because they talk to either the IO thread or the terminal.
                    // All other keycodes are handled in the final `on_key`
                    // wildcard pattern, as they do neither.
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL)
                    | (KeyCode::Esc, _) => {
                        if !app.error_flash_is_empty() {
                            app.clear_error_flash();
                        } else {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            io_s.send(IoCommand::Break)?;
                            break;
                        }
                    }
                    (KeyCode::Char('r'), KeyModifiers::NONE) => match &app.selected() {
                        Selected::Feeds => {
                            let feed_id = app.selected_feed_id();
                            io_s.send(IoCommand::RefreshFeed(feed_id))?;
                        }
                        _ => app.toggle_read()?,
                    },
                    (KeyCode::Char('x'), KeyModifiers::NONE) => {
                        let feed_ids = app.feed_ids()?;
                        io_s.send(IoCommand::RefreshFeeds(feed_ids))?;
                    }
                    // handle all other normal-mode keycodes here
                    (keycode, modifiers) => {
                        // Manually match out the on_key result here
                        // and show errors in the error flash,
                        // because these on_key actions can fail
                        // in such a way that the app can continue.
                        if let Err(e) = app.on_key(keycode, modifiers) {
                            app.push_error_flash(e);
                        }
                    }
                },
                Event::Tick => (),
            },
            Mode::Editing => match rx.recv()? {
                Event::Input(event) => match event.code {
                    KeyCode::Enter => {
                        let feed_subscription_input = { app.feed_subscription_input() };
                        io_s.send(IoCommand::SubscribeToFeed(feed_subscription_input))?;
                    }
                    KeyCode::Char(c) => {
                        app.push_feed_subscription_input(c);
                    }
                    KeyCode::Backspace => app.pop_feed_subscription_input(),
                    KeyCode::Delete => {
                        app.delete_feed()?;
                    }
                    KeyCode::Esc => {
                        app.set_mode(Mode::Normal);
                    }
                    _ => {}
                },
                Event::Tick => (),
            },
        }
    }

    io_thread
        .join()
        .expect("Unable to join IO thread to main thread")?;

    Ok(())
}
