use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, Paragraph, Text},
    Frame,
};

use crate::app::App;
use crate::modes::{Mode, Selected};
use crate::rss::Entry;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .direction(Direction::Horizontal)
        .split(f.size());

    draw_info_column(f, chunks[0], app);

    match &app.selected {
        Selected::Feeds => {
            draw_entries(f, chunks[1], app);
        }
        Selected::Entries => {
            draw_entries(f, chunks[1], app);
        }
        Selected::Entry(entry) => {
            let default_title = String::from("Entry");
            let title = entry.title.as_ref().unwrap_or_else(|| &default_title);
            draw_entry(
                f,
                chunks[1],
                app.entry_scroll_position,
                &app.current_entry_text,
                title,
                &app.error_flash,
            );
        }
    }
}

fn draw_info_column<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Percentage(60),
                Constraint::Percentage(30),
                Constraint::Percentage(4),
                Constraint::Percentage(5),
            ]
            .as_ref(),
        )
        .direction(Direction::Vertical)
        .split(area);
    {
        //FEEDS
        draw_feeds(f, chunks[0], app);

        // INFO
        match &app.selected {
            Selected::Entry(entry) => draw_entry_info(f, chunks[1], entry),
            Selected::Entries => {
                if let Some(entry) = &app.current_entry {
                    draw_entry_info(f, chunks[1], entry);
                } else {
                    draw_feed_info(f, chunks[1], app);
                }
            }
            _ => {
                if app.current_feed.is_some() {
                    draw_feed_info(f, chunks[1], app);
                }
            }
        }

        // HELP SECTION
        draw_help(f, chunks[2], app);

        // INPUT SECTION
        draw_new_feed_input(f, chunks[3], app);
    }
}

fn draw_entry_info<B>(f: &mut Frame<B>, area: Rect, entry: &Entry)
where
    B: Backend,
{
    let mut text = vec![];
    if let Some(item) = &entry.title {
        text.push({
            let mut s = String::new();
            s.push_str("Title: ");
            s.push_str(item.to_string().as_str());
            s.push_str("\n");
            Text::raw(s)
        });
    }

    if let Some(item) = &entry.link {
        text.push({
            let mut s = String::new();
            s.push_str("Link: ");
            s.push_str(item);
            s.push_str("\n");
            Text::raw(s)
        })
    }

    if let Some(pub_date) = &entry.pub_date {
        text.push({
            let mut s = String::new();
            s.push_str("Pub. date: ");
            s.push_str(pub_date.to_string().as_str());
            s.push_str("\n");
            Text::raw(s)
        })
    } else {
        // TODO this should probably pull the <updated> tag
        // and use that
        let inserted_at = entry.inserted_at;
        text.push({
            let mut s = String::new();
            s.push_str("Pulled date: ");
            s.push_str(inserted_at.to_string().as_str());
            s.push_str("\n");
            Text::raw(s)
        })
    }

    if let Some(read_at) = &entry.read_at {
        text.push({
            let mut s = String::new();
            s.push_str("Read at: ");
            s.push_str(read_at.to_string().as_str());
            s.push_str("\n");
            Text::raw(s)
        })
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Info")
        .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD));

    let paragraph = Paragraph::new(text.iter()).block(block).wrap(true);

    f.render_widget(paragraph, area);
}

fn draw_feeds<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let feeds = app
        .feeds
        .items
        .iter()
        .flat_map(|feed| feed.title.as_ref())
        .map(Text::raw);
    let feeds = List::new(feeds).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Feeds")
            .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD)),
    );

    let feeds = if app.selected == Selected::Feeds {
        feeds
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(255, 150, 167))
                    .modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ")
    } else {
        feeds
    };

    f.render_stateful_widget(feeds, area, &mut app.feeds.state);
}

fn draw_feed_info<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let mut text = vec![];
    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.title.as_ref())
    {
        text.push({
            let mut s = String::new();
            s.push_str("Title: ");
            s.push_str(item.to_owned().to_string().as_str());
            s.push_str("\n");
            Text::raw(s)
        });
    }

    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.link.as_ref())
    {
        text.push({
            let mut s = String::new();
            s.push_str("Link: ");
            s.push_str(item.to_owned().to_string().as_str());
            s.push_str("\n");
            Text::raw(s)
        })
    }

    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.feed_link.as_ref())
    {
        text.push({
            let mut s = String::new();
            s.push_str("Feed link: ");
            s.push_str(item.to_owned().to_string().as_str());
            s.push_str("\n");
            Text::raw(s)
        })
    }

    if let Some(item) = app.entries.items.get(0) {
        if let Some(pub_date) = &item.pub_date {
            text.push({
                let mut s = String::new();
                s.push_str("Most recent entry at: ");
                s.push_str(pub_date.to_string().as_str());
                s.push_str("\n");
                Text::raw(s)
            })
        }
    }

    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.refreshed_at)
        .map(|timestamp| timestamp.to_owned().to_string())
        .or_else(|| Some("Never refreshed".to_string()))
    {
        text.push({
            let mut s = String::new();
            s.push_str("Refreshed at: ");
            s.push_str(item.as_str());
            s.push_str("\n");
            Text::raw(s)
        })
    }

    text.push({
        let mut s = String::new();
        s.push_str("Total entries: ");
        s.push_str(app.entries.items.len().to_string().as_str());
        s.push_str("\n");
        Text::raw(s)
    });

    if let Some(feed_kind) = app.current_feed.as_ref().map(|feed| feed.feed_kind) {
        text.push({
            let mut s = String::new();
            s.push_str("Feed kind: ");
            s.push_str(&feed_kind.to_string());
            s.push_str("\n");
            Text::raw(s)
        });
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Info")
        .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD));

    let paragraph = Paragraph::new(text.iter()).block(block).wrap(true);

    f.render_widget(paragraph, area);
}

fn draw_help<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let msg = match app.mode {
        Mode::Normal => "i - edit mode; q - exit",
        Mode::Editing => "esc - normal mode; enter - fetch feed",
    };
    let text = [
        Text::raw("r - mark entry read/un; a - toggle view read/un\n"),
        Text::raw(msg),
    ];
    let help_message = Paragraph::new(text.iter());
    f.render_widget(help_message, area);
}

fn draw_new_feed_input<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let text = [Text::raw(&app.feed_subscription_input)];
    let input = Paragraph::new(text.iter())
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Add a feed")
                .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD)),
        );
    f.render_widget(input, area);
}

fn draw_entries<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let entries = app
        .entries
        .items
        .iter()
        .map(|entry| Text::raw(entry.title.as_ref().unwrap()));
    let entries_titles = List::new(entries).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Entries")
            .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD)),
    );

    let entries_titles = if app.selected == Selected::Entries {
        entries_titles
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(255, 150, 167))
                    .modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ")
    } else {
        entries_titles
    };

    if let Some(error) = &app.error_flash {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(60), Constraint::Percentage(30)].as_ref())
            .direction(Direction::Vertical)
            .split(area);
        {
            let error_text = format!("{:?}", error)
                .split("\n")
                .map(|line| {
                    let mut s = String::with_capacity(line.len() + 1);
                    s.push_str(line);
                    s.push_str("\n");
                    Text::raw(s)
                })
                .collect::<Vec<_>>();

            let block = Block::default()
                .borders(Borders::ALL)
                .title("Error - press 'q' to close")
                .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD));

            let error_widget = Paragraph::new(error_text.iter())
                .block(block)
                .wrap(true)
                .scroll(0);

            f.render_stateful_widget(entries_titles, chunks[0], &mut app.entries.state);
            f.render_widget(error_widget, chunks[1]);
        }
    } else {
        f.render_stateful_widget(entries_titles, area, &mut app.entries.state);
    }
}

fn draw_entry<B>(
    f: &mut Frame<B>,
    area: Rect,
    scroll: u16,
    current_entry_text: &[Text],
    title: &str,
    error_flash: &Option<crate::error::Error>,
) where
    B: Backend,
{
    let text = current_entry_text;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD));
    let paragraph = Paragraph::new(text.iter())
        .block(block)
        .wrap(true)
        .scroll(scroll);

    if let Some(error) = error_flash {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(60), Constraint::Percentage(30)].as_ref())
            .direction(Direction::Vertical)
            .split(area);
        {
            let error_text = format!("{:?}", error)
                .split("\n")
                .map(|line| {
                    let mut s = String::with_capacity(line.len() + 1);
                    s.push_str(line);
                    s.push_str("\n");
                    Text::raw(s)
                })
                .collect::<Vec<_>>();

            let block = Block::default()
                .borders(Borders::ALL)
                .title("Error - press 'q' to close")
                .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD));

            let error_widget = Paragraph::new(error_text.iter())
                .block(block)
                .wrap(true)
                .scroll(0);

            f.render_widget(paragraph, chunks[0]);
            f.render_widget(error_widget, chunks[1]);
        }
    } else {
        f.render_widget(paragraph, area);
    }
}
