# Changelog

I welcome contributions to Russ. If you have an idea for something you would like to contribute, open an issue and we can talk about it!

## Unreleased

- Russ now creates a feeds database by default. It will be at`$XDG_DATA_HOME/russ/feeds.db` or `$HOME/.local/share/russ/feeds.db` on Linux, and `$HOME/Library/Application Support/russ/feeds.db` on Mac. See the [directories](https://github.com/dirs-dev/directories-rs#projectdirs) docs for more information on how this location is computed. Note that this is a non-breaking change and the `-d` CLI flag will continue to override the default database path, so you can continue to use a different path if you want.
- Reimplement the feed refresh functionality to use regular threads instead of `tokio` and `futures-util`.
- Remove `tokio` and `futures-util`.
- Group operations that mutate the database into meaningful transactions
- Tidy up the [README](README.md)
- Bump `html2text` to `0.4`
- Add Github Issue templates, thank you @NickLarsenNZ ([#8](https://github.com/ckampfe/russ/pull/8))
- Fix first-run panics (and improve first-run experience), thank you @NickLarsenNZ for reporting ([#7](https://github.com/ckampfe/russ/issues/7))
- README improvements, thank you @toastal ([#10](https://github.com/ckampfe/russ/pull/10))
- Update dependencies, fixes build on android, thank you @phanirithvij ([#14](https://github.com/ckampfe/russ/pull/14))
- Update clap to version 4
- Remove CircleCI from build (only Github Actions now)
- Fix a clippy warning in test
- CI: use `apt-get` instead of `apt`

## 0.4.0

- Bring `webbrowser` dependency up to mainline, thank you @amodm ([#4](https://github.com/ckampfe/russ/pull/4))
- Add ability to delete a feed and its entries, thank you @Funami580 ([#3](https://github.com/ckampfe/russ/pull/3))
- Use `clap` and its derive instead of `structopt`
- Bump `tui` to `0.18`
- Bump `crossterm` to `0.23.2`
- Bump `rusqlite` to `0.27` and `r2d2_sqlite` to `0.20`
- Bump `html2text` to `0.3`
- Bump `webbrowser` to `0.7`
- Fix clippys/formatting
- Bump a lot of transitive dependencies

## 0.3.0

- `russ --version` now reports the numeric version (e.x.: `0.3.0`) rather than a git commit hash.
- Bump `tui`, `crossterm`, `ureq`, and `copypasta` and some transitive dependencies

## 0.2.0

- You can now press `o` to open the current link in your default browser (thanks [@Funami580](https://github.com/Funami580)) ([#2](https://github.com/ckampfe/russ/pull/2))
- Use Alacritty's fork of Copypasta which includes a Macos memory leak fix
- Bump versions of many dependencies
- Add this changelog! Prior to this version there was no changelog for development
