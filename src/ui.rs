use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, Paragraph, Text},
    Frame,
};

use crate::app::{App, Selected};

pub(crate) fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .direction(Direction::Horizontal)
        .split(f.size());

    draw_feeds(f, chunks[0], app);

    match &app.selected {
        Selected::Feeds => {
            draw_entries(f, chunks[1], app);
        }
        Selected::Entries => {
            draw_entries(f, chunks[1], app);
        }
        Selected::Entry(entry) => {
            let default_title = String::from("Entry");
            draw_entry(
                f,
                chunks[1],
                app.scroll,
                &app.current_entry_text,
                entry,
                entry.title.as_ref().unwrap_or_else(|| &default_title),
            );
        }
    }
}

// fn draw_empty<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
// where
//     B: Backend,
// {
//     let text = [];
//     let block = Block::default()
//         .borders(Borders::ALL)
//         .title("Items")
//         .title_style(Style::default().fg(Color::Magenta).modifier(Modifier::BOLD));
//     let paragraph = Paragraph::new(text.iter()).block(block).wrap(true);
//     f.render_widget(paragraph, area);
// }

fn draw_feeds<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let feeds = app
        .feed_titles
        .items
        .iter()
        .map(|(_feed_id, title)| Text::raw(title));
    let feeds = List::new(feeds).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Feeds")
            .title_style(Style::default().fg(Color::Magenta).modifier(Modifier::BOLD)),
    );

    let feeds = if app.selected == Selected::Feeds {
        feeds
            .highlight_style(Style::default().fg(Color::Yellow).modifier(Modifier::BOLD))
            .highlight_symbol("> ")
    } else {
        feeds
    };

    f.render_stateful_widget(feeds, area, &mut app.feed_titles.state);
}

fn draw_entries<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let entries_titles = app
        .entries_titles
        .items
        .iter()
        .map(|(_id, title)| Text::raw(title));
    let entries_titles = List::new(entries_titles).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Entries")
            .title_style(Style::default().fg(Color::Magenta).modifier(Modifier::BOLD)),
    );

    let entries_titles = if app.selected == Selected::Entries {
        entries_titles
            .highlight_style(Style::default().fg(Color::Yellow).modifier(Modifier::BOLD))
            .highlight_symbol("> ")
    } else {
        entries_titles
    };

    f.render_stateful_widget(entries_titles, area, &mut app.entries_titles.state);
}

fn draw_entry<B>(
    f: &mut Frame<B>,
    area: Rect,
    scroll: u16,
    current_entry_text: &[Text],
    entry: &crate::rss::Entry,
    title: &str,
) where
    B: Backend,
{
    let text = current_entry_text;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        // .title("Entry")
        .title_style(Style::default().fg(Color::Magenta).modifier(Modifier::BOLD));
    let paragraph = Paragraph::new(text.iter())
        .block(block)
        .wrap(true)
        .scroll(scroll);
    f.render_widget(paragraph, area);
}
