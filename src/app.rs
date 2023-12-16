use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::Stylize;
use ratatui::widgets::Paragraph;

use crate::action::Action;
use crate::components::containers::Containers;
use crate::components::Component;
use crate::DoggyTerminal;

pub(crate) struct App<'a> {
    should_quit: bool,
    main: Box<dyn Component>,
    version: &'a str,
}

impl<'a> App<'a> {
    pub fn new(version: &'a str) -> Self {
        App {
            should_quit: false,
            main: Box::new(Containers::new()),
            version,
        }
    }

    pub fn update(&mut self, action: Option<Action>) -> Result<Option<Action>> {
        match action {
            Some(Action::Quit) => {
                self.should_quit = true;
                Ok(None)
            }
            Some(Action::Screen(screen)) => {
                self.main = screen;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    pub fn run_app(&mut self, terminal: &mut DoggyTerminal) -> Result<()> {
        while !self.should_quit {
            log::debug!("Updating component: {}", self.main.get_name());
            self.main.update(Some(Action::Refresh))?;

            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Max(4), Constraint::Min(5), Constraint::Max(1)])
                .split(terminal.get_frame().size());

            log::debug!("{:?}", main_layout);

            log::debug!("Drawing component: {}", self.main.get_name());
            terminal.draw(|f| {
                self.draw_header(f, main_layout[0]);
                self.main.draw(f, main_layout[1]);
                self.draw_status(f, main_layout[2]);
            })?;

            let mut action = handle_event()?;

            log::debug!("Received action: {:?}", action);
            action = self.main.update(action)?;

            log::debug!("Action after component processing: {:?}", action);
            if let Some(ignored_action) = self.update(action)? {
                log::warn!("Ignored action: {}", ignored_action);
            }
        }
        Ok(())
    }

    fn draw_header(&self, f: &mut ratatui::prelude::Frame<'_>, rect: ratatui::prelude::Rect) {
        let p = Paragraph::new("TODO".red());
        f.render_widget(p, rect)
    }

    fn draw_status(&self, f: &mut ratatui::prelude::Frame<'_>, rect: ratatui::prelude::Rect) {
        let p = Paragraph::new(format!("Doggy version {}", self.version).dark_gray());
        f.render_widget(p, rect)
    }
}

fn handle_event() -> Result<Option<Action>, color_eyre::eyre::Error> {
    let action =
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::NONE) => Some(Action::Quit),
                    (KeyCode::Char('j'), KeyModifiers::NONE)
                    | (KeyCode::Down, KeyModifiers::NONE) => Some(Action::Down),
                    (KeyCode::Char('k'), KeyModifiers::NONE)
                    | (KeyCode::Up, KeyModifiers::NONE) => Some(Action::Up),
                    (KeyCode::Char('h'), KeyModifiers::NONE)
                    | (KeyCode::Left, KeyModifiers::NONE) => Some(Action::Left),
                    (KeyCode::Char('l'), KeyModifiers::NONE)
                    | (KeyCode::Right, KeyModifiers::NONE) => Some(Action::Right),
                    (KeyCode::Char('J'), KeyModifiers::NONE)
                    | (KeyCode::PageUp, KeyModifiers::NONE) => Some(Action::PageUp),
                    (KeyCode::Char('K'), KeyModifiers::NONE)
                    | (KeyCode::PageDown, KeyModifiers::NONE) => Some(Action::PageDown),
                    (KeyCode::Char('a'), KeyModifiers::NONE) => Some(Action::All),
                    (KeyCode::Char('i'), KeyModifiers::NONE) => Some(Action::Inspect),
                    (KeyCode::Esc, KeyModifiers::NONE) => Some(Action::PreviousScreen),
                    (KeyCode::Enter, KeyModifiers::NONE) => Some(Action::Ok),
                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => Some(Action::Delete),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };
    Ok(action)
}
