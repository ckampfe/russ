# russ

Russ is a really simple RSS reader built for my own use.
If you happen to like it, that's great!
At this time I cannot guarantee any kind of stability in either the software itself or the data format, or a timeframe for achieving stability. Caveat emptor.

<img src="entries.png"></img>
<img src="entry.png"></img>

## install/use

```
$ git clone
$ cd russ
$ cargo install --path .
$ russ -d"your_db_name.db"
```

```
$ russ -h
russ 0.1.0

USAGE:
    russ [OPTIONS] --database-path <database-path>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --database-path <database-path>    feed database path
    -l, --line-length <line-length>        maximum line length for entries [default: 90]
    -t, --tick-rate <tick-rate>            time in ms between two ticks [default: 250]
```

## goals

1. An RSS client in the terminal, not in a webpage
1. Controls how I like them (vim-style navigation)
1. Layout how I like it (columnar layout with higher-level info on the left, content on the right)
1. Fully offline operation - network used only for resyncing feeds
1. All application data in SQLite
1. Fast - all interactions should take place in under 100ms
1. As few configuration/options as possible
1. As little resource usage as possible
1. As few system dependencies as possible - Rust and `sqlite3`

## nongoals

1. Separate client/server processes
1. Browser interaction/support
1. Analysis/metrics - use SQLite itself
1. Sharing/social/posting to Twitter/etc
1. Emacs integration
1. Mobile support

## design

Russ is a [tui](https://crates.io/crates/tui) app that uses [crossterm](https://crates.io/crates/crossterm), so it should (???) work on Windows (I do not use Windows so I cannot verify this, but feel free to open an issue with an experience report)

It stores all application data in a SQLite database file at a location of your choosing. There is no default file. You need to specify this file when you start Russ (see above).

## todo

- [x] rss support
- [x] atom support
- [x] vim-style hjkl navigation
- [x] subscribe to a feed
- [x] refresh a feed
- [x] mark entries as read
- [x] mark entries as unread
- [x] view only unread entries
- [x] view only read entries
- [x] entry reading/scrolling
- [x] error handling/display
- [x] display entry info
- [x] display feed info
- [x] configurable word wrapping line length
- [ ] profiling mode that shows speed of UI interaction
- [ ] stabilize the database schema
- [ ] migration process for database changes
- [x] nonblocking IO (inspiration: https://keliris.dev/improving-spotify-tui/)
- [ ] automatically fetch entries that only provide a link field
- [ ] debug view (show app state)
- [ ] deleting feeds
- [x] refresh all feeds
- [ ] some kind of search