use crate::modes::*;
use app::App;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::path::PathBuf;
use std::{
    error::Error,
    io::{stdout, Write},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use structopt::*;
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod error;
mod modes;
mod rss;
mod ui;
mod util;

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Debug, StructOpt)]
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
}

pub enum IOCommand {
    Break,
    OnKey(char),
    OnEnter,
    OnEsc,
    OnLeft,
    OnRight,
    OnUp,
    OnDown,
    SubscribeToFeed,
    SelectFeeds,
    UpdateCurrentFeedAndEntries,
}

#[tokio::main]
async fn start_async_io(
    app: Arc<Mutex<App>>,
    rx: crossbeam_channel::Receiver<IOCommand>,
    // conn: &rusqlite::Connection,
) -> Result<(), crate::error::Error> {
    use IOCommand::*;
    while let Ok(event) = rx.recv() {
        match event {
            Break => break,
            OnKey(c) => {
                let mut app = app.lock().unwrap();
                app.on_key(c).await;
            }
            SubscribeToFeed => {
                let mut app = app.lock().unwrap();
                app.subscribe_to_feed().await?;
                app.feed_subscription_input = String::new();
            }
            SelectFeeds => {
                let mut app = app.lock().unwrap();
                app.select_feeds().await;
            }
            UpdateCurrentFeedAndEntries => {
                let mut app = app.lock().unwrap();
                app.update_current_feed_and_entries()?;
            }
            OnLeft => {
                let mut app = app.lock().unwrap();
                app.on_left().await?;
            }
            OnRight => {
                let mut app = app.lock().unwrap();
                app.on_right().await?;
            }
            OnUp => {
                let mut app = app.lock().unwrap();
                app.on_up().await?;
            }
            OnDown => {
                let mut app = app.lock().unwrap();
                app.on_down().await?;
            }
            OnEnter => {
                let mut app = app.lock().unwrap();
                app.on_enter().await?;
            }
            OnEsc => {
                let mut app = app.lock().unwrap();
                app.on_esc();
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options: Options = Options::from_args();

    enable_raw_mode()?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);

    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    // Setup input handling
    let (tx, rx) = crossbeam_channel::unbounded();

    let tick_rate = Duration::from_millis(options.tick_rate);
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            // poll for tick rate duration, if no events, sent tick event.
            if event::poll(tick_rate - last_tick.elapsed()).unwrap() {
                if let CEvent::Key(key) = event::read().unwrap() {
                    tx.send(Event::Input(key)).unwrap();
                }
            }
            if last_tick.elapsed() >= tick_rate {
                tx.send(Event::Tick).unwrap();
                last_tick = Instant::now();
            }
        }
    });

    // let conn = rusqlite::Connection::open(&options.database_path)?;

    let app = Arc::new(Mutex::new(App::new(options)?));

    let cloned_app = Arc::clone(&app);

    terminal.clear()?;

    let (io_s, io_r) = crossbeam_channel::unbounded();

    // this thread is for async IO
    let io_thread = thread::spawn(move || -> Result<(), crate::error::Error> {
        start_async_io(app, io_r)?;
        Ok(())
    });

    // MAIN THREAD IS DRAW THREAD
    loop {
        {
            let mut app = cloned_app.lock().unwrap();
            terminal.draw(|mut f| ui::draw(&mut f, &mut app))?;
        }
        // match app.mode {
        //     Mode::Normal => {
        match rx.recv()? {
            Event::Input(event) => match event.code {
                KeyCode::Char('q') => {
                    let mut app = cloned_app.lock().unwrap();
                    if app.error_flash.is_some() {
                        app.error_flash = None;
                    } else {
                        disable_raw_mode()?;
                        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                        terminal.show_cursor()?;
                        io_s.send(IOCommand::Break)?;
                        break;
                    }
                }
                KeyCode::Char(c) => {
                    io_s.send(IOCommand::OnKey(c))?;
                    // app.on_key(c).await
                }
                KeyCode::Left => {
                    io_s.send(IOCommand::OnLeft)?;
                    // app.on_left()
                }
                KeyCode::Up => {
                    io_s.send(IOCommand::OnUp)?;
                    // app.on_up()?
                }
                KeyCode::Right => {
                    io_s.send(IOCommand::OnRight)?;
                    // app.on_right()?
                }
                KeyCode::Down => {
                    io_s.send(IOCommand::OnDown)?;
                    // app.on_down()?
                }
                KeyCode::Enter => {
                    io_s.send(IOCommand::OnEnter)?;
                    // app.on_enter()?
                }
                KeyCode::Esc => {
                    io_s.send(IOCommand::OnEsc)?;
                    // app.on_esc(),
                }
                _ => {}
            },
            Event::Tick => (),
        }
        // if app.should_quit {
        //     break;
        // }
        // }
        // Mode::Editing => {
        //     match rx.recv()? {
        //         Event::Input(event) => match event.code {
        //             KeyCode::Enter => {
        //                 io_s.send(SubscribeToFeed)?;
        //                 io_s.send(SelectFeeds)?;
        //                 io_s.send(UpdateCurrentFeedAndEntries)?;
        //             }
        //             KeyCode::Char(c) => {
        //                 app.feed_subscription_input.push(c);
        //             }
        //             KeyCode::Backspace => {
        //                 app.feed_subscription_input.pop();
        //             }
        //             KeyCode::Esc => {
        //                 app.mode = Mode::Normal;
        //             }
        //             _ => {}
        //         },
        //         Event::Tick => (),
        //     }
        //     if app.should_quit {
        //         break;
        //     }
        // }
        // }
    }

    io_thread.join().unwrap()?;

    Ok(())
}
