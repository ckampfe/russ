#![forbid(unsafe_code)]

use crate::modes::*;
use anyhow::{Context, Result};
use app::App;
use crossterm::{
    event::{self, Event as CEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rayon::prelude::*;
use std::{
    io::{stdout, Write},
    path::PathBuf,
    sync::mpsc,
    thread, time,
};
use structopt::*;
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod modes;
mod rss;
mod ui;
mod util;

const RUSS_VERSION: &str = env!("RUSS_VERSION");

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "russ", version = crate::RUSS_VERSION)]
pub struct Options {
    /// feed database path
    #[structopt(short, long)]
    database_path: PathBuf,
    /// time in ms between two ticks
    #[structopt(short, long, default_value = "250")]
    tick_rate: u64,
    /// maximum line length for entries
    #[structopt(short, long, default_value = "90")]
    line_length: usize,
    /// number of seconds to show the flash message before clearing it
    #[structopt(short, long, default_value = "4", parse(try_from_str = parse_seconds))]
    flash_display_duration_seconds: time::Duration,
    /// RSS/Atom network request timeout in seconds
    #[structopt(short, long, default_value = "5", parse(try_from_str = parse_seconds))]
    network_timeout: time::Duration,
}

fn parse_seconds(s: &str) -> Result<time::Duration, std::num::ParseIntError> {
    let as_u64 = u64::from_str_radix(s, 10)?;
    Ok(time::Duration::from_secs(as_u64))
}

enum IOCommand {
    Break,
    RefreshFeed(crate::rss::FeedId),
    RefreshFeeds(Vec<crate::rss::FeedId>),
    SubscribeToFeed(String),
    ClearFlash,
}

fn start_async_io(
    app: App,
    sx: &mpsc::Sender<IOCommand>,
    rx: mpsc::Receiver<IOCommand>,
    options: &Options,
) -> Result<()> {
    use IOCommand::*;

    let manager = r2d2_sqlite::SqliteConnectionManager::file(&options.database_path);
    let pool = r2d2::Pool::new(manager)?;

    while let Ok(event) = rx.recv() {
        match event {
            Break => break,
            RefreshFeed(feed_id) => {
                let now = std::time::Instant::now();

                app.set_flash("Refreshing feed...".to_string());

                let conn = pool.get()?;

                if let Err(e) = crate::rss::refresh_feed(&app.http_client(), &conn, feed_id)
                    .with_context(|| {
                        let feed_url =
                            crate::rss::get_feed_url(&conn, feed_id).unwrap_or_else(|_| {
                                panic!("Unable to get feed URL for feed_id {}", feed_id)
                            });

                        format!("Failed to fetch and refresh feed {}", feed_url)
                    })
                {
                    app.push_error_flash(e);
                } else {
                    app.update_current_feed_and_entries()?;
                    let elapsed = now.elapsed();
                    app.set_flash(format!("Refreshed feed in {:?}", elapsed));

                    clear_flash_after(&sx, &options.flash_display_duration_seconds);
                };
            }
            RefreshFeeds(feed_ids) => {
                let now = std::time::Instant::now();

                app.set_flash("Refreshing all feeds...".to_string());

                feed_ids
                    .into_par_iter()
                    .for_each(|feed_id| match pool.get() {
                        Ok(conn) => {
                            if let Err(e) =
                                crate::rss::refresh_feed(&app.http_client(), &conn, feed_id)
                                    .with_context(|| {
                                        let feed_url = crate::rss::get_feed_url(&conn, feed_id)
                                            .unwrap_or_else(|_| {
                                                panic!(
                                                    "Unable to get feed URL for feed_id {}",
                                                    feed_id
                                                )
                                            });

                                        format!("Failed to fetch and refresh feed {}", feed_url)
                                    })
                            {
                                app.push_error_flash(e);
                            }
                        }
                        Err(e) => {
                            app.push_error_flash(e.into());
                        }
                    });

                {
                    app.update_current_feed_and_entries()?;

                    let elapsed = now.elapsed();
                    app.set_flash(format!("Refreshed all feeds in {:?}", elapsed));
                }

                clear_flash_after(&sx, &options.flash_display_duration_seconds);
            }
            SubscribeToFeed(feed_subscription_input) => {
                let now = std::time::Instant::now();

                app.set_flash("Subscribing to feed...".to_string());

                let conn = pool.get()?;

                if let Err(e) = crate::rss::subscribe_to_feed(
                    &app.http_client(),
                    &conn,
                    &feed_subscription_input,
                ) {
                    app.push_error_flash(e);
                } else {
                    match crate::rss::get_feeds(&conn) {
                        Ok(feeds) => {
                            {
                                app.reset_feed_subscription_input();
                                app.set_feeds(feeds);
                                app.select_feeds();
                                app.update_current_feed_and_entries()?;

                                let elapsed = now.elapsed();
                                app.set_flash(format!("Subscribed in {:?}", elapsed));
                            }

                            clear_flash_after(&sx, &options.flash_display_duration_seconds);
                        }
                        Err(e) => {
                            app.push_error_flash(e);
                        }
                    };
                }
            }
            ClearFlash => {
                app.clear_flash();
            }
        }
    }

    Ok(())
}

fn clear_flash_after(sx: &mpsc::Sender<IOCommand>, duration: &time::Duration) {
    let sx = sx.clone();
    let duration = *duration;
    thread::spawn(move || {
        thread::sleep(duration);
        sx.send(IOCommand::ClearFlash)
            .expect("Unable to send IOCommand::ClearFlash");
    });
}

fn main() -> Result<()> {
    let options: Options = Options::from_args();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup input handling
    let (tx, rx) = mpsc::channel();

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

    let mut app = App::new(options)?;

    let cloned_app = app.clone();

    terminal.clear()?;

    let (io_s, io_r) = mpsc::channel();

    let io_s_clone = io_s.clone();

    // this thread is for async IO
    let io_thread = thread::spawn(move || -> Result<()> {
        start_async_io(cloned_app, &io_s_clone, io_r, &options_clone)?;
        Ok(())
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
                    (KeyCode::Char('q'), _)
                    | (KeyCode::Char('c'), KeyModifiers::CONTROL)
                    | (KeyCode::Esc, _) => {
                        if !app.error_flash_is_empty() {
                            app.clear_error_flash();
                        } else {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            io_s.send(IOCommand::Break)?;
                            break;
                        }
                    }
                    (KeyCode::Char('r'), KeyModifiers::NONE) => match &app.selected() {
                        Selected::Feeds => {
                            let feed_id = app.selected_feed_id();
                            io_s.send(IOCommand::RefreshFeed(feed_id))?;
                        }
                        _ => app.toggle_read()?,
                    },
                    (KeyCode::Char('x'), KeyModifiers::NONE) => {
                        let feed_ids = app.feed_ids()?;
                        io_s.send(IOCommand::RefreshFeeds(feed_ids))?;
                    }
                    (KeyCode::Char('?'), _) => app.toggle_help()?,
                    (KeyCode::Char(c), KeyModifiers::NONE) => app.on_key(c)?,
                    (KeyCode::Left, _) => app.on_left()?,
                    (KeyCode::Up, _) => app.on_up()?,
                    (KeyCode::Right, _) => app.on_right()?,
                    (KeyCode::Down, _) => app.on_down()?,
                    (KeyCode::Enter, _) => app.on_enter()?,
                    _ => {}
                },
                Event::Tick => (),
            },
            Mode::Editing => match rx.recv()? {
                Event::Input(event) => match event.code {
                    KeyCode::Enter => {
                        let feed_subscription_input = { app.feed_subscription_input() };
                        io_s.send(IOCommand::SubscribeToFeed(feed_subscription_input))?;
                    }
                    KeyCode::Char(c) => {
                        app.push_feed_subscription_input(c);
                    }
                    KeyCode::Backspace => app.pop_feed_subscription_input(),
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
