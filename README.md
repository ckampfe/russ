# russ

Russ is a really simple RSS/Atom reader with vim-like controls and an offline-first focus.

See the [license](LICENSE) if you're curious about that kind of thing.

[![CircleCI](https://circleci.com/gh/ckampfe/russ.svg?style=svg)](https://circleci.com/gh/ckampfe/russ)

---

<img src="entries.png"></img>
<img src="entry.png"></img>

## install

```
$ git clone
$ cd russ
$ cargo install --path .
$ russ -d"your_db_name.db"
```

I do not currently publish binary releases, but that may change if someone is interested in that.

## use

Russ has few controls, that mostly follow a small subset of vim's controls.
If you know vim, Russ should feel natural.
The only controls are `hjkl` (or arrow keys), `i`, `r`, `a`, `x`, `q`, `Esc`, and `Enter`.

### insert mode

To subscribe to your first feed, you will need to be in `insert` mode.
Press `i` to enter `insert` mode, where you can type the URL of and RSS or Atom feed you want to subscribe to.
Press  `Enter` to subscribe to a feed and fetch all entries.
If this operation is successful, title of the feed will appear in the left column, and its unread entries on the right.
Press `Esc` to exit `insert` mode and return to `normal` mode.
This is how you subscribe to RSS/Atom feeds in Russ.

### normal mode

`Normal` mode is where you spend most of your time using Russ.
It is where you read RSS/Atom entries and refresh feeds.

Navigation in `normal` mode is spatial.
Navigating right takes you in a more specific direction (all feeds -> a single feed's entries -> a single entry),
and navigating left takes you in a more general direction (a single entry -> a single feed's entries -> all feeds).
Use `hjkl` or the arrow keys to navigate between the left (context) column and the right (reading) column.
The cursor indicates where you are.

You can scroll down/up in a list or an entry with `j`/`k` or `down`/`up`.
To mark a selected entry as read, press `r`.
By default, Russ will only show unread entries, so any entries marked read will disappear from the entry list.
To view entries you have marked read, press `a`. You can mark them unread by pressing `r` on a selected entry.
To view entries that are unread (the default state), press `a` again.

To refresh a single feed, press `r` when you are in the most general context (all the way to the left) and that feed is highlighted.
To refresh all feeds, press `x` when in the most general context.
Press `q` or `Esc` to quit Russ.

### quick reference

`hjkl`/arrows - move
`q` - quit
`Esc` - quit (in normal mode)
`i` - insert mode
`Enter` - refresh the currently input feed (insert mode)
`Enter` - read selected entry
`r` - refresh single feed (context dependent)
`r` - mark entry as read (context dependent)
`a` - view read/unread entries
`x` - refresh all feeds
`Esc` - go from insert mode to normal mode

## help/options/config

```
$ russ -h
russ ccf8f7a

USAGE:
    russ [OPTIONS] --database-path <database-path>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --database-path <database-path>                                      feed database path
    -f, --flash-display-duration-seconds <flash-display-duration-seconds>
            number of seconds to show the flash message before clearing it [default: 4]

    -l, --line-length <line-length>
            maximum line length for entries [default: 90]

    -t, --tick-rate <tick-rate>                                              time in ms between two ticks [default: 250]
```

## design

By design, Russ is non-eager. It will not automaticlly refresh your subscriptions on a timer, it will not automatically mark entries as read. It will do these things when you tell it to.
Russ is designed such that it should be possible to use it 100% offline. You should be able to load it up with new feeds and entries and fly to Australia, and not have Russ complain when the plane's Wifi fails. As long as you have a copy of Russ and a SQLite database of your RSS/Atom feeds, it should work.

Russ is a [tui](https://crates.io/crates/tui) app that uses [crossterm](https://crates.io/crates/crossterm), so it should (???) work on Windows (I do not use Windows so I cannot verify this, but feel free to open an issue with an experience report)

## stability

At this time, I cannot guarantee any kind of stability of interfaces or database schema.
I reserve the right to change Russ or its database format at any time.
That said: Russ is generally stable! I use it every day to read my feeds, and I don't believe I've broken either the config or the database schema in quite a while. It works pretty well at this point.
I have no major features planned that would require breaking schema or interface changes.
I will do my best not to break any data contracts, and will change this text if I believe that Russ has stabilized enough to be considered "stable" or "1.0".

## todo

This is not a strict feature list. Unchecked items are ideas to explore rather than features that are going to be built.

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
- [x] parse and store proper `chrono::DateTime<Utc>` for `pub_date`
- [x] sort entries by `pub_date` descending, fall back to `inserted_at` if no `pub_date`
- [ ] profiling mode that shows speed of UI interaction
- [ ] stabilize the database schema
- [ ] migration process for database changes
- [x] nonblocking IO (inspiration: https://keliris.dev/improving-spotify-tui/)
- [ ] automatically fetch entries that only provide a link field
- [ ] debug view (show app state)
- [ ] deleting feeds
- [x] refresh all feeds
- [x] refresh all feeds in parallel (multithreaded IO)
- [x] use a database connection pool when refreshing feeds
- [x] show refresh time for single feed and all feeds
- [ ] mark entries as "favorite"
- [ ] some kind of search
- [x] fix N+1 queries on feed/entry creation
- [x] set up CI

## license

See the [license.](LICENSE)