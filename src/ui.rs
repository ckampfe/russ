use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Borders, LineGauge, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;
use std::rc::Rc;

use crate::app::AppImpl;
use crate::modes::{Mode, ReadMode, Selected};
use crate::rss::EntryMeta;

const PINK: Color = Color::Rgb(255, 150, 167);

pub fn predraw<B: Backend>(f: &Frame<B>) -> Rc<[Rect]> {
    Layout::default()
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .direction(Direction::Horizontal)
        .split(f.size())
}

pub fn draw<B: Backend>(f: &mut Frame<B>, chunks: Rc<[Rect]>, app: &mut AppImpl) {
    draw_info_column(f, chunks[0], app);

    match &app.selected {
        Selected::Feeds | Selected::Entries => {
            draw_entries(f, chunks[1], app);
        }
        Selected::Entry(_entry_meta) => {
            draw_entry(f, chunks[1], app);
        }
        Selected::None => draw_entries(f, chunks[1], app),
    }
}

fn draw_info_column<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let mut constraints = match &app.mode {
        Mode::Normal => vec![Constraint::Percentage(70), Constraint::Percentage(20)],
        Mode::Editing => vec![
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
        ],
    };

    if app.show_help {
        constraints.push(Constraint::Percentage(10));
    }

    let chunks = Layout::default()
        .constraints(constraints)
        .direction(Direction::Vertical)
        .split(area);
    {
        // FEEDS
        draw_feeds(f, chunks[0], app);

        // INFO
        match &app.selected {
            Selected::Entry(entry) => draw_entry_info(f, chunks[1], entry),
            Selected::Entries => {
                if let Some(entry_meta) = &app.current_entry_meta {
                    draw_entry_info(f, chunks[1], entry_meta);
                } else {
                    draw_feed_info(f, chunks[1], app);
                }
            }
            Selected::None => draw_first_run_helper(f, chunks[1]),
            _ => {
                if app.current_feed.is_some() {
                    draw_feed_info(f, chunks[1], app);
                }
            }
        }

        match (app.mode, app.show_help) {
            (Mode::Editing, true) => {
                draw_new_feed_input(f, chunks[2], app);
                draw_help(f, chunks[3], app);
            }
            (Mode::Editing, false) => {
                draw_new_feed_input(f, chunks[2], app);
            }
            (_, true) => {
                draw_help(f, chunks[2], app);
            }
            _ => (),
        }
    }
}

fn draw_first_run_helper<B>(f: &mut Frame<B>, area: Rect)
where
    B: Backend,
{
    let text = "Press 'i', then enter an RSS/Atom feed URL, then hit `Enter`!";

    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        "TO SUBSCRIBE TO YOUR FIRST FEED",
        Style::default().fg(PINK).add_modifier(Modifier::BOLD),
    ));

    let paragraph = Paragraph::new(Text::from(text))
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn draw_entry_info<B>(f: &mut Frame<B>, area: Rect, entry_meta: &EntryMeta)
where
    B: Backend,
{
    let mut text = String::new();
    if let Some(item) = &entry_meta.title {
        text.push_str("Title: ");
        text.push_str(item.to_string().as_str());
        text.push('\n');
    };

    if let Some(item) = &entry_meta.link {
        text.push_str("Link: ");
        text.push_str(item);
        text.push('\n');
    }

    if let Some(pub_date) = &entry_meta.pub_date {
        text.push_str("Pub. date: ");
        text.push_str(pub_date.to_string().as_str());
    } else {
        // TODO this should probably pull the <updated> tag
        // and use that
        let inserted_at = entry_meta.inserted_at;
        text.push_str("Pulled date: ");
        text.push_str(inserted_at.to_string().as_str());
    }
    text.push('\n');

    if let Some(read_at) = &entry_meta.read_at {
        text.push_str("Read at: ");
        text.push_str(read_at.to_string().as_str());
        text.push('\n');
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
    let title = app.flash.as_ref().unwrap_or(&default_title);

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
            .highlight_style(Style::default().fg(PINK).add_modifier(Modifier::BOLD))
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
    if let Some(item) = app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.title.as_ref())
    {
        text.push_str("Title: ");
        text.push_str(item);
        text.push('\n');
    }

    if let Some(item) = app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.link.as_ref())
    {
        text.push_str("Link: ");
        text.push_str(item);
        text.push('\n');
    }

    if let Some(item) = app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.feed_link.as_ref())
    {
        text.push_str("Feed link: ");
        text.push_str(item);
        text.push('\n');
    }

    if let Some(item) = app.entries.items.get(0) {
        if let Some(pub_date) = &item.pub_date {
            text.push_str("Most recent entry at: ");
            text.push_str(pub_date.to_string().as_str());
            text.push('\n');
        }
    }

    if let Some(item) = &app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.refreshed_at)
        .map(|timestamp| timestamp.to_string())
        .or_else(|| Some("Never refreshed".to_string()))
    {
        text.push_str("Refreshed at: ");
        text.push_str(item.as_str());
        text.push('\n');
    }

    match app.read_mode {
        ReadMode::ShowUnread => text.push_str("Unread entries: "),
        ReadMode::ShowRead => text.push_str("Read entries: "),
        ReadMode::All => unreachable!("ReadMode::All should never be possible from the UI!"),
    }
    text.push_str(app.entries.items.len().to_string().as_str());
    text.push('\n');

    if let Some(feed_kind) = app.current_feed.as_ref().map(|feed| feed.feed_kind) {
        text.push_str("Feed kind: ");
        text.push_str(&feed_kind.to_string());
        text.push('\n');
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
        Selected::Feeds => {
            text.push_str("r - refresh selected feed; x - refresh all feeds\n");
            text.push_str("c - copy link; o - open link in browser\n")
        }
        _ => {
            text.push_str("r - mark entry read/un; a - toggle view read/un\n");
            text.push_str("c - copy link; o - open link in browser\n")
        }
    }
    match app.mode {
        Mode::Normal => text.push_str("i - edit mode; q - exit\n"),
        Mode::Editing => {
            text.push_str("enter - fetch feed; del - delete feed\n");
            text.push_str("esc - normal mode\n")
        }
    }

    text.push_str("? - show/hide help");

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
        .map(|entry| {
            ListItem::new(Span::raw(entry.title.as_ref().unwrap_or_else(|| {
                panic!("Unable to get title for entry id {}", entry.id)
            })))
        })
        .collect::<Vec<ListItem>>();

    let default_title = "Entries".to_string();

    let title = app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.title.as_ref())
        .unwrap_or(&default_title);

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
            .highlight_style(Style::default().fg(PINK).add_modifier(Modifier::BOLD))
            .highlight_symbol("> "),
        _ => entries_titles,
    };

    if !&app.error_flash.is_empty() {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(60), Constraint::Percentage(30)].as_ref())
            .direction(Direction::Vertical)
            .split(area);
        {
            let error_text = error_text(&app.error_flash);

            let block = Block::default().borders(Borders::ALL).title(Span::styled(
                "Error - press 'q' to close",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));

            let error_widget = Paragraph::new(error_text)
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

fn draw_entry<B>(f: &mut Frame<B>, area: Rect, app: &mut AppImpl)
where
    B: Backend,
{
    let scroll = app.entry_scroll_position;
    let entry_meta = if let Selected::Entry(e) = &app.selected {
        e
    } else {
        panic!("draw_entry should only be called when app.selected was Selected::Entry")
    };
    let default_entry_title = "No entry title".to_string();
    let default_feed_title = "No feed title".to_string();

    let entry_title = entry_meta.title.as_ref().unwrap_or(&default_entry_title);

    let feed_title = app
        .current_feed
        .as_ref()
        .and_then(|feed| feed.title.as_ref())
        .unwrap_or(&default_feed_title);

    let mut title = entry_title.to_owned();
    title.push_str(" - ");
    title.push_str(feed_title);

    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        &title,
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Cyan),
    ));

    let paragraph = Paragraph::new(app.current_entry_text.as_str())
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    let entry_chunk_height = area.height - 2;

    let progress_gauge_chunk_percent = 3;

    let entry_percent = 100.0 - progress_gauge_chunk_percent as f32;

    let real_entry_chunk_height =
        (entry_chunk_height as f32 * (entry_percent / 100.0)).floor() as u16;

    app.entry_lines_rendered_len = real_entry_chunk_height;

    let percent = if app.entry_lines_len > 0 {
        let furthest_visible_position = app.entry_scroll_position + real_entry_chunk_height;
        let percent = ((furthest_visible_position as f32 / app.entry_lines_len as f32) * 100.0)
            .floor() as usize;

        if percent <= 100 {
            percent
        } else {
            100
        }
    } else {
        0
    };

    let label = format!("{}/100", percent);
    let ratio = percent as f64 / 100.0;
    let gauge = LineGauge::default()
        .block(Block::default().borders(Borders::NONE))
        .gauge_style(Style::default().fg(PINK))
        .ratio(ratio)
        .label(label);

    if !app.error_flash.is_empty() {
        let chunks = Layout::default()
            .constraints(
                [
                    Constraint::Percentage(57),
                    Constraint::Percentage(progress_gauge_chunk_percent),
                    Constraint::Percentage(40),
                ]
                .as_ref(),
            )
            .direction(Direction::Vertical)
            .split(area);
        {
            let error_text = error_text(&app.error_flash);
            let block = Block::default().borders(Borders::ALL).title(Span::styled(
                "Error - press 'q' to close",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            ));

            let error_widget = Paragraph::new(error_text)
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((0, 0));

            f.render_widget(paragraph, chunks[0]);
            f.render_widget(gauge, chunks[1]);
            f.render_widget(error_widget, chunks[2]);
        }
    } else {
        let chunks = Layout::default()
            .constraints(
                [
                    Constraint::Percentage(entry_percent.ceil() as u16),
                    Constraint::Percentage(progress_gauge_chunk_percent),
                ]
                .as_ref(),
            )
            .direction(Direction::Vertical)
            .split(area);

        f.render_widget(paragraph, chunks[0]);
        f.render_widget(gauge, chunks[1]);
    }
}

fn error_text(errors: &[anyhow::Error]) -> String {
    errors
        .iter()
        .flat_map(|e| {
            let mut s = format!("{:?}", e)
                .split('\n')
                .map(|s| s.to_owned())
                .collect::<Vec<String>>();
            s.push("\n".to_string());
            s
        })
        .collect::<Vec<String>>()
        .join("\n")
}
