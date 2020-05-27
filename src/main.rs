use app::{App, Mode};
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::path::PathBuf;
use std::{
    error::Error,
    io::{stdout, Write},
    thread,
    time::{Duration, Instant},
};
use structopt::*;
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod error;
mod rss;
mod ui;
mod util;

enum Event<I> {
    Input(I),
    Tick,
}

#[derive(Debug, StructOpt)]
struct Options {
    /// feed database path
    #[structopt(short, long)]
    database_path: PathBuf,
    /// time in ms between two ticks.
    #[structopt(short, long, default_value = "250")]
    tick_rate: u64,
    /// whether unicode symbols are used to improve the overall look of the app
    /// defaults to true
    #[structopt(short, long)]
    enhanced_graphics: Option<bool>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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

    let mut app = App::new(
        "Russ",
        options.database_path,
        options.enhanced_graphics.unwrap_or_else(|| true),
    )?;

    terminal.clear()?;

    loop {
        terminal.draw(|mut f| ui::draw(&mut f, &mut app))?;
        match app.mode {
            Mode::Normal => {
                match rx.recv()? {
                    Event::Input(event) => match event.code {
                        KeyCode::Char('q') => {
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            terminal.show_cursor()?;
                            break;
                        }
                        KeyCode::Char(c) => app.on_key(c).await?,
                        KeyCode::Left => app.on_left(),
                        KeyCode::Up => app.on_up(),
                        KeyCode::Right => app.on_right().unwrap(),
                        KeyCode::Down => app.on_down(),
                        KeyCode::Enter => app.on_enter().unwrap(),
                        KeyCode::Esc => app.on_esc(),
                        _ => {}
                    },
                    Event::Tick => {
                        app.on_tick();
                    }
                }
                if app.should_quit {
                    break;
                }
            }
            Mode::Editing => {
                match rx.recv()? {
                    Event::Input(event) => match event.code {
                        KeyCode::Enter => {
                            app.subscribe_to_feed().await?;
                            app.input = String::new();
                            app.select_feeds().await;
                            app.update_current_feed_and_entries()?;
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Normal;
                            // events.enable_exit_key();
                        }
                        _ => {}
                    },
                    Event::Tick => {
                        app.on_tick();
                    }
                }
                if app.should_quit {
                    break;
                }
            }
        }
    }

    Ok(())
}
