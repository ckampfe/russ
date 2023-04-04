use ratatui::widgets::ListState;

#[derive(Debug)]
pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn reset(&mut self) {
        self.state.select(Some(0));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

impl<T> From<Vec<T>> for StatefulList<T> {
    fn from(other: Vec<T>) -> Self {
        StatefulList::with_items(other)
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn set_wsl_clipboard_contents(s: &str) -> anyhow::Result<()> {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    // it looks like this on the CLI:
    // `echo "foo" | clip.exe`
    let mut clipboard = Command::new("clip.exe").stdin(Stdio::piped()).spawn()?;

    let mut clipboard_stdin = clipboard
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("Unable to get stdin handle for clip.exe"))?;

    clipboard_stdin.write_all(s.as_bytes())?;

    Ok(())
}
