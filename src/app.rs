use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::action::Action;
use crate::components::containers::Containers;
use crate::components::Component;
use crate::DoggyTerminal;

pub(crate) struct App {
    should_quit: bool,
    components: Vec<Box<dyn Component>>,
}

impl App {
    pub fn new() -> Self {
        App {
            should_quit: false,
            components: vec![Box::new(Containers::new())],
        }
    }

    pub fn update(&mut self, action: Option<Action>) -> Result<Option<Action>> {
        match action {
            Some(Action::Quit) => {
                self.should_quit = true;
                Ok(None)
            }
            Some(Action::Screen(screen)) => {
                self.components[0] = screen;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    pub fn run_app(&mut self, terminal: &mut DoggyTerminal) -> Result<()> {
        while !self.should_quit {
            for c in self.components.iter_mut() {
                log::debug!("Updating component: {}", c.get_name());
                c.update(Some(Action::Refresh))?;
            }

            for c in self.components.iter_mut() {
                log::debug!("Drawing component: {}", c.get_name());
                terminal.draw(|f| {
                    let _ = c.draw(f, f.size());
                })?;
            }

            let mut action = if let Event::Key(key) = event::read()? {
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

            log::debug!("Received action: {:?}", action);
            for c in self.components.iter_mut() {
                action = c.update(action)?;
            }
            log::debug!("Action after component processing: {:?}", action);
            if let Some(ignored_action) = self.update(action)? {
                log::warn!("Ignored action: {}", ignored_action);
            }
        }
        Ok(())
    }
}
