use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, Paragraph, Text},
    Frame,
};

use crate::app::{App, Mode, Selected};

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
            let title = entry.title.as_ref().unwrap_or_else(|| &default_title);
            draw_entry(f, chunks[1], app.scroll, &app.current_entry_text, title);
        }
    }
}

fn draw_feeds<B>(f: &mut Frame<B>, area: Rect, app: &mut App)
where
    B: Backend,
{
    let chunks = Layout::default()
        .constraints(
            [
                Constraint::Percentage(60),
                Constraint::Percentage(30),
                Constraint::Percentage(3),
                Constraint::Percentage(5),
            ]
            .as_ref(),
        )
        .direction(Direction::Vertical)
        .split(area);
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

        f.render_stateful_widget(feeds, chunks[0], &mut app.feed_titles.state);

        // STATUS WIDGET
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

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Info")
            .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD));
        let paragraph = Paragraph::new(text.iter()).block(block).wrap(true);
        f.render_widget(paragraph, chunks[1]);

        // HELP SECTION
        let msg = match app.mode {
            Mode::Normal => "Press q to exit, i to start editing.",
            Mode::Editing => "Press Esc to stop editing, Enter to record the message",
        };
        let text = [Text::raw(msg)];
        let help_message = Paragraph::new(text.iter());
        f.render_widget(help_message, chunks[2]);

        // INPUT SECTION
        let text = [Text::raw(&app.input)];
        let input = Paragraph::new(text.iter())
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Add a feed")
                    .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD)),
            );
        f.render_widget(input, chunks[3]);
    }
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

    f.render_stateful_widget(entries_titles, area, &mut app.entries.state);
}

fn draw_entry<B>(
    f: &mut Frame<B>,
    area: Rect,
    scroll: u16,
    current_entry_text: &[Text],
    title: &str,
) where
    B: Backend,
{
    let text = current_entry_text;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        // .title("Entry")
        .title_style(Style::default().fg(Color::Cyan).modifier(Modifier::BOLD));
    let paragraph = Paragraph::new(text.iter())
        .block(block)
        .wrap(true)
        .scroll(scroll);
    f.render_widget(paragraph, area);
}
