use std::{
    io,
    ops::{Deref, DerefMut},
};

use color_eyre::Result;

use crossterm::{
    cursor, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::backend::CrosstermBackend as Backend;
use ratatui::prelude::*;

pub struct Tui {
    pub terminal: ratatui::Terminal<Backend<std::io::Stderr>>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let mut stderr = io::stderr();
        execute!(stderr, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stderr);
        let terminal = Terminal::new(backend)?;
        Ok(Tui { terminal })
    }

    pub fn exit(&mut self) -> Result<()> {
        if crossterm::terminal::is_raw_mode_enabled()? {
            crossterm::execute!(std::io::stderr(), LeaveAlternateScreen, cursor::Show)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }
}

impl Deref for Tui {
    type Target = ratatui::Terminal<Backend<std::io::Stderr>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.exit().unwrap();
    }
}
