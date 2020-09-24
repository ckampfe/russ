use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::AppImpl;
use crate::modes::{Mode, ReadMode, Selected};
use crate::rss::Entry;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut AppImpl) {
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
            let default_entry_title = "No entry title".to_string();
            let default_feed_title = "No feed title".to_string();

            let entry_title = entry.title.as_ref().unwrap_or_else(|| &default_entry_title);

            let current_feed_title = app
                .current_feed
                .as_ref()
                .and_then(|feed| feed.title.as_ref())
                .unwrap_or_else(|| &default_feed_title);

            draw_entry(
                f,
                chunks[1],
                app.entry_scroll_position,
                &app.current_entry_text,
                entry_title,
                &app.error_flash,
                &current_feed_title,
            );
        }
    }
}

fn draw_info_column<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let constraints = match &app.mode {
        Mode::Normal => [
            Constraint::Percentage(70),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
        ]
        .as_ref(),
        Mode::Editing => [
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
        ]
        .as_ref(),
    };
    let chunks = Layout::default()
        .constraints(constraints)
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
        if let Mode::Editing = &app.mode {
            draw_new_feed_input(f, chunks[3], app)
        }
    }
}

fn draw_entry_info<B>(f: &mut Frame<B>, area: Rect, entry: &Entry)
where
    B: Backend,
{
    let mut text = String::new();
    if let Some(item) = &entry.title {
        text.push_str("Title: ");
        text.push_str(item.to_string().as_str());
        text.push_str("\n");
    };

    if let Some(item) = &entry.link {
        text.push_str("Link: ");
        text.push_str(item);
        text.push_str("\n");
    }

    if let Some(pub_date) = &entry.pub_date {
        text.push_str("Pub. date: ");
        text.push_str(pub_date.to_string().as_str());
        text.push_str("\n");
    } else {
        // TODO this should probably pull the <updated> tag
        // and use that
        let inserted_at = entry.inserted_at;
        text.push_str("Pulled date: ");
        text.push_str(inserted_at.to_string().as_str());
        text.push_str("\n");
    }

    if let Some(read_at) = &entry.read_at {
        text.push_str("Read at: ");
        text.push_str(read_at.to_string().as_str());
        text.push_str("\n");
    }

    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Info",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    let paragraph = Paragraph::new(Text::from(text.as_str()))
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_feeds<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let feeds = app
        .feeds
        .items
        .iter()
        .flat_map(|feed| feed.title.as_ref())
        .map(Span::raw)
        .map(ListItem::new)
        .collect::<Vec<ListItem>>();

    let default_title = String::from("Feeds");
    let title = app.flash.as_ref().unwrap_or_else(|| &default_title);

    let feeds = List::new(feeds).block(
        Block::default().borders(Borders::ALL).title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    );

    let feeds = match app.selected {
        Selected::Feeds => feeds
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(255, 150, 167))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> "),
        _ => feeds,
    };

    f.render_stateful_widget(feeds, area, &mut app.feeds.state);
}

fn draw_feed_info<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let mut text = String::new();
    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.title.as_ref())
    {
        text.push_str("Title: ");
        text.push_str(item.to_owned().to_string().as_str());
        text.push_str("\n");
    }

    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.link.as_ref())
    {
        text.push_str("Link: ");
        text.push_str(item.to_owned().to_string().as_str());
        text.push_str("\n");
    }

    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.feed_link.as_ref())
    {
        text.push_str("Feed link: ");
        text.push_str(item.to_owned().to_string().as_str());
        text.push_str("\n");
    }

    if let Some(item) = app.entries.items.get(0) {
        if let Some(pub_date) = &item.pub_date {
            text.push_str("Most recent entry at: ");
            text.push_str(pub_date.to_string().as_str());
            text.push_str("\n");
        }
    }

    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.refreshed_at)
        .map(|timestamp| timestamp.to_owned().to_string())
        .or_else(|| Some("Never refreshed".to_string()))
    {
        text.push_str("Refreshed at: ");
        text.push_str(item.as_str());
        text.push_str("\n");
    }

    match app.read_mode {
        ReadMode::ShowUnread => text.push_str("Unread entries: "),
        ReadMode::ShowRead => text.push_str("Read entries: "),
        ReadMode::All => unreachable!("ReadMode::All should never be possible from the UI!"),
    }
    text.push_str(app.entries.items.len().to_string().as_str());
    text.push_str("\n");

    if let Some(feed_kind) = app.current_feed.as_ref().map(|feed| feed.feed_kind) {
        text.push_str("Feed kind: ");
        text.push_str(&feed_kind.to_string());
        text.push_str("\n");
    }

    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        "Info",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));

    let paragraph = Paragraph::new(Text::from(text.as_str()))
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_help<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let mut text = String::new();
    match app.selected {
        Selected::Feeds => text.push_str("r - refresh selected feed; x - refresh all feeds\n"),
        _ => text.push_str("r - mark entry read/un; a - toggle view read/un\n"),
    }
    match app.mode {
        Mode::Normal => text.push_str("i - edit mode; q - exit"),
        Mode::Editing => text.push_str("esc - normal mode; enter - fetch feed"),
    }

    let help_message =
        Paragraph::new(Text::from(text.as_str())).block(Block::default().borders(Borders::ALL));
    f.render_widget(help_message, area);
}

fn draw_new_feed_input<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let text = &app.feed_subscription_input;
    let text = Text::from(text.as_str());
    let input = Paragraph::new(text)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default().borders(Borders::ALL).title(Span::styled(
                "Add a feed",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
        );
    f.render_widget(input, area);
}

fn draw_entries<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let entries = app
        .entries
        .items
        .iter()
        .map(|entry| ListItem::new(Span::raw(entry.title.as_ref().unwrap())))
        .collect::<Vec<ListItem>>();

    let default_title = "Entries".to_string();

    let title = app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.title.as_ref())
        .unwrap_or_else(|| &default_title);

    let entries_titles = List::new(entries).block(
        Block::default().borders(Borders::ALL).title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    );

    let entries_titles = match app.selected {
        Selected::Entries => entries_titles
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(255, 150, 167))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> "),
        _ => entries_titles,
    };

    if !&app.error_flash.is_empty() {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(60), Constraint::Percentage(30)].as_ref())
            .direction(Direction::Vertical)
            .split(area);
        {
            let error_text = app
                .error_flash
                .iter()
                .flat_map(|error| {
                    let mut e = format!("{:?}", error)
                        .split('\n')
                        .map(|line| {
                            let mut s = String::with_capacity(line.len() + 1);
                            s.push_str(line);
                            s.push_str("\n");
                            Span::raw(s)
                        })
                        .collect::<Vec<_>>();
                    e.push(Span::raw("\n"));
                    e
                })
                .collect::<Vec<_>>();

            let block = Block::default().borders(Borders::ALL).title(Span::styled(
                "Error - press 'q' to close",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));

            let error_widget = Paragraph::new(Spans::from(error_text))
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((0, 0));

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
    current_entry_text: &str,
    entry_title: &str,
    error_flash: &[crate::error::Error],
    feed_title: &str,
) where
    B: Backend,
{
    let text = current_entry_text;
    let mut title = entry_title.to_string();
    title.push_str(" - ");
    title.push_str(feed_title);

    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        &title,
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Cyan),
    ));

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    if !error_flash.is_empty() {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(60), Constraint::Percentage(30)].as_ref())
            .direction(Direction::Vertical)
            .split(area);
        {
            // see https://github.com/dtolnay/indoc/blob/902c3eba53a7c1206ee43262768e6dff6f882f29/unindent/src/lib.rs#L123-L130
            // for a function that can count leading spaces.
            let error_text = error_flash
                .iter()
                .flat_map(|error| {
                    let mut e = format!("{:?}", error)
                        .split('\n')
                        .map(|line| {
                            let mut s = String::with_capacity(line.len() + 1);
                            s.push_str(line);
                            s.push_str("\n");
                            Span::from(s)
                        })
                        .collect::<Vec<_>>();
                    e.push(Span::raw("\n"));
                    e
                })
                .collect::<Vec<_>>();

            let block = Block::default().borders(Borders::ALL).title(Span::styled(
                "Error - press 'q' to close",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            ));

            let error_widget = Paragraph::new(Spans::from(error_text))
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((0, 0));

            f.render_widget(paragraph, chunks[0]);
            f.render_widget(error_widget, chunks[1]);
        }
    } else {
        f.render_widget(paragraph, area);
    }
}
