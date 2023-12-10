use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind, self, Event};

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

    pub fn run_app(&mut self, terminal: &mut DoggyTerminal) -> Result<()> {
        while !self.should_quit {
            for c in self.components.iter_mut() {
                c.update()?;
            }

            for c in self.components.iter_mut() {
                terminal.draw(|f| { let _ = c.draw(f, f.size()); })?;
            }
            
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => self.should_quit = true,
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }
}
