# russ

Russ is a TUI RSS/Atom reader with vim-like controls and a local-first, offline-first focus.

[![CircleCI](https://circleci.com/gh/ckampfe/russ.svg?style=svg)](https://circleci.com/gh/ckampfe/russ)
[![Rust](https://github.com/ckampfe/russ/actions/workflows/rust.yml/badge.svg)](https://github.com/ckampfe/russ/actions/workflows/rust.yml)

---

![](entries.png)
![](entry.png)

## install

```console
$ cargo install russ --git https://github.com/ckampfe/russ

  note that on linux, you will need these system dependencies as well, for example:
$ sudo apt update && sudo apt install libxcb-shape0-dev libxcb-xfixes0-dev
$ russ
```

**Note** that on its first run with no arguments, `russ` creates a SQLite database file called `feeds.db` to store RSS/Atom feeds in a location of its choosing. If you wish to override this, you can pass a path with the `-d` option, like `russ -d /your/database/location/my_feeds.db`. If you use a custom database location, you will need to pass the `-d` option every time you invoke `russ`. See the help with `russ -h` for more information about where `russ` will store the `feeds.db` database by default on your platform.

I do not currently publish binary releases, but that may change if someone is interested in that.

## use

Russ is modal, like vim. If you are comfortable with vim, or know of vim, you are probably going to be immediately comfortable with Russ. If you don't know vim, don't be afraid! If you read the following controls section and tinker a bit, you'll have no trouble using Russ.

There are two modes: normal mode and insert mode.

In normal mode, you read your RSS entries, navigate between entries, navigate between feeds, refresh feeds, and a few other things. This is where you spend 99% of your time when using Russ.

When you want to start following a new feed, you enter insert mode.
In insert mode, you enter the URL of a feed you wish to begin following, and Russ will download that feed for you.

That's basically it!

### controls - normal mode

Some normal mode controls vary based on whether you are currently selecting a feed or an entry.

- `q`/`Esc` - quit Russ
- `hjkl`/arrows - move up/down/left/right between feeds and entries, scroll up/down on an entry
- `Enter` - read selected entry
- `r` - refresh the selected feed
- `r` - mark the selected entry as read
- `x` - refresh all feeds
- `i` - change to insert mode
- `a` - toggle between read/unread entries
- `c` - copy the selected link to the clipboard (feed or entry)
- `o` - open the selected link in your browser (feed or entry)

### controls - insert mode

- `Esc` - go back to normal mode
- `Enter` - subscribe to the feed you just typed in the input box
- `Del` - delete the selected feed.

## help/options/config

```console
$ russ -h
russ 0.4.0
Clark Kampfe <clark.kampfe@gmail.com>
A TUI RSS reader with vim-like controls and a local-first, offline-first focus

USAGE:
    russ [OPTIONS]

OPTIONS:
    -d, --database-path <DATABASE_PATH>
            Override where `russ` stores and reads feeds. By default, the feeds database on Linux
            this will be at `XDG_DATA_HOME/russ/feeds.db` or `$HOME/.local/share/russ/feeds.db`. On
            MacOS it will be at `$HOME/Library/Application Support/russ/feeds.db`. On Windows it
            will be at `{FOLDERID_LocalAppData}/russ/data/feeds.db`

    -f, --flash-display-duration-seconds <FLASH_DISPLAY_DURATION_SECONDS>
            number of seconds to show the flash message before clearing it [default: 4]

    -h, --help
            Print help information

    -n, --network-timeout <NETWORK_TIMEOUT>
            RSS/Atom network request timeout in seconds [default: 5]

    -t, --tick-rate <TICK_RATE>
            time in ms between two ticks [default: 250]

    -V, --version
            Print version information

```

## design

Russ stores all application data in a SQLite database. Additionally, Russ is non-eager. It will not automaticlly refresh your feeds on a timer, it will not automatically mark entries as read. Russ will only do these things when you tell it to. This is intentional, as Russ has been designed to be 100% usable offline, with no internet connection. You should be able to load it up with new feeds and entries and fly to Australia, and not have Russ complain when the plane's Wifi fails. As long as you have a copy of Russ and a SQLite database of your RSS/Atom feeds, you will be able to read your RSS/Atom feeds.

Russ is a [tui](https://crates.io/crates/tui) app that uses [crossterm](https://crates.io/crates/crossterm). I develop and use Russ primarily on a Mac, but I have run it successfully on Linux and WSL. It should be possible to use Russ on Windows, but I have not personally used Russ on Windows, so I cannot verify this. If you use Russ on Windows or have tried to use Russ on Windows, please open an issue and let me know!

## stability

The application-level and database-level contracts encoded in Russ are stable. I can't remember the last time I broke one. That said, I still reserve the right to break application or database contracts to fix things, but I have no reason to believe this will happen. I use Russ every day, and it basically "just works". If you use Russ and this is not the case for you, please open an issue and let me know.

## SQL

Despite being a useful RSS reader for me and a few others, Russ cannot possibly provide all of
the functionality everyone might want from an RSS reader.

However, Russ uses a regular SQLite database to store RSS feeds (more detail below),
which means that if a feature you want isn't in Russ itself, you can probably accomplish
what you want to do with regular SQL.

This is especially true for one-off tasks like running analysis of your RSS feeds,
removing duplicates when a feed changes its link scheme, etc.

If there's something you want to do with your RSS feeds and Russ doesn't do it,
consider opening a Github issue and asking if anyone knows how to make it happen with SQL.

## features/todo

This is not a strict feature list, and it is not a roadmap. Unchecked items are ideas to explore rather than features that are going to be built. If you have an idea for a feature that you would enjoy, open an issue and we can talk about it.

- [ ] visual indicator for which feeds have new/unacknowledged entries
- [ ] profiling mode that shows speed of UI interaction
- [ ] stabilize the database schema
- [ ] migration process for database changes
- [ ] automatically fetch entries that only provide a link field
- [ ] debug view (show app state)
- [x] rss support
- [x] atom support
- [x] vim-style hjkl navigation
- [x] subscribe to a feed
- [x] refresh a feed
- [x] delete a feed
- [x] mark entries as read
- [x] mark entries as unread
- [x] view only unread entries
- [x] view only read entries
- [x] entry reading/scrolling
- [x] error handling/display
- [x] display entry info
- [x] display feed info
- [x] configurable word wrapping line length
- [x] parse and store proper `chrono::DateTime<Utc>` for `pub_date`
- [x] sort entries by `pub_date` descending, fall back to `inserted_at` if no `pub_date`
- [x] nonblocking IO (inspiration: https://keliris.dev/improving-spotify-tui/)
- [x] refresh all feeds
- [x] refresh all feeds in parallel (multithreaded IO)
- [x] use a database connection pool when refreshing feeds
- [x] show refresh time for single feed and all feeds
- [x] fix N+1 queries on feed/entry creation
- [x] set up CI
- [x] copy feed and entry links to clipboard
- [x] add a network timeout for fetching new rss/atom entries (default: 5 seconds)
- [x] show scroll progress for an entry
- [x] show/hide help with `?`
- [x] page-down/page-up entry scrolling
- [x] automatic line length for wrapping
- [x] ability to open the current link in your default browser
- [x] create a feeds database by default (overridable with `-d` CLI option)

## Minimum Supported Rust Version (MSRV) policy

Russ targets the latest stable version of the Rust compiler. Older Rust versions may work, but building Russ against non-latest stable versions is not a project goal and is not supported.
Likewise, Russ may build with a nightly Rust compiler, but this is not a project goal.

## SQLite version

`russ` compiles and bundles its own embedded SQLite via the [Rusqlite](https://github.com/rusqlite/rusqlite) project, which is version 3.39.2.

If you prefer to use the version of SQLite on your system, edit `Cargo.toml` to
remove the `"bundled"` feature from the `rusqlite` dependency and recompile `russ`.

**Please note** that while `russ` may run just fine with whatever version of SQLite you happen to have on your system, I do not test `russ` with a system SQLite, **and running `russ` with a system SQLite is not officially supported.**

## contributing

I welcome contributions to Russ. If you have an idea for something you would like to contribute, open an issue and we can talk about it!

## license

See the [license.](LICENSE)
