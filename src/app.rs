use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::action::Action;
use crate::components::containers::Containers;
use crate::components::images::Images;
use crate::components::Component;
use crate::DoggyTerminal;

enum InputMode {
    None,
    Change,
    //TODO Filter
}

const CONTAINERS: &str = "containers";
const IMAGES: &str = "images";

const SUGGESTIONS: [&str; 2] = [CONTAINERS, IMAGES];

pub(crate) struct App<'a> {
    should_quit: bool,
    main: Box<dyn Component>,
    input: String,
    input_mode: InputMode,
    cursor_position: usize,
    suggestion: Option<&'static str>,
    version: &'a str,
}

impl<'a> App<'a> {
    pub fn new(version: &'a str) -> Self {
        App {
            should_quit: false,
            main: Box::new(Containers::new()),
            input: "".to_string(),
            input_mode: InputMode::None,
            suggestion: None,
            cursor_position: 0,
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
                self.main.update(Some(Action::Refresh))
            }
            Some(Action::Change) => {
                self.input_mode = InputMode::Change;
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    pub fn run_app(&mut self, terminal: &mut DoggyTerminal) -> Result<()> {
        log::debug!("Updating component: {}", self.main.get_name());
        self.main.update(Some(Action::Refresh))?;
        while !self.should_quit {
            let main_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Max(3), Constraint::Min(5), Constraint::Max(1)])
                .split(terminal.get_frame().size());

            log::debug!("{:?}", main_layout);

            log::debug!("Drawing component: {}", self.main.get_name());
            terminal.draw(|f| {
                self.draw_header(f, main_layout[0]);
                self.main.draw(f, main_layout[1]);
                self.draw_status(f, main_layout[2]);
            })?;

            let mut action = match self.input_mode {
                InputMode::None => handle_event()?,
                InputMode::Change => self.handle_input()?,
                // InputMode::Filter => self.handle_input()?,
            };

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
        match self.input_mode {
            InputMode::None => {
                let p = Paragraph::new("Welcome to Doggy!");
                f.render_widget(p, rect)
            }
            InputMode::Change => {
                let input = Paragraph::new(Line::from(if let Some(suggestion) = self.suggestion {
                    vec![
                        Span::styled(self.input.to_string(), Style::default().gray()),
                        Span::styled(
                            suggestion[self.cursor_position..].to_string(),
                            Style::default().dark_gray(),
                        ),
                    ]
                } else {
                    vec![Span::styled(
                        self.input.to_string(),
                        Style::default().gray(),
                    )]
                }))
                .block(Block::default().borders(Borders::ALL).title("Input"));
                f.render_widget(input, rect);
            }
        }
    }

    fn draw_status(&self, f: &mut ratatui::prelude::Frame<'_>, rect: ratatui::prelude::Rect) {
        let p = Paragraph::new(format!("Doggy version {}", self.version).dark_gray());
        f.render_widget(p, rect)
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn enter_char(&mut self, new_char: char) {
        self.input.insert(self.cursor_position, new_char);

        self.move_cursor_right();
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.len())
    }

    fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    fn handle_input(&mut self) -> Result<Option<Action>> {
        let action = if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Enter => self.submit_input(),
                    KeyCode::Char(to_insert) => {
                        self.enter_char(to_insert);
                        self.suggestion = self.update_suggestion();
                        None
                    }
                    KeyCode::Backspace => {
                        self.delete_char();
                        None
                    }
                    KeyCode::Left => {
                        self.move_cursor_left();
                        None
                    }
                    KeyCode::Right => {
                        self.move_cursor_right();
                        None
                    }
                    KeyCode::Esc => {
                        self.input = "".to_string();
                        self.input_mode = InputMode::None;
                        self.reset_cursor();
                        None
                    }
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

    fn submit_input(&mut self) -> Option<Action> {
        match self.suggestion {
            Some(CONTAINERS) => {
                self.reset_input();
                Some(Action::Screen(Box::new(Containers::new())))
            }
            Some(IMAGES) => {
                self.reset_input();
                Some(Action::Screen(Box::new(Images::new())))
            }
            _ => None,
        }
    }

    fn reset_input(&mut self) {
        self.input = "".to_string();
        self.cursor_position = 0;
        self.input_mode = InputMode::None;
    }

    fn update_suggestion(&self) -> Option<&'static str> {
        SUGGESTIONS
            .into_iter()
            .find(|searched| searched.starts_with(&self.input))
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
                    (KeyCode::Char(':'), KeyModifiers::NONE) => Some(Action::Change),
                    (KeyCode::Char('/'), KeyModifiers::NONE) => Some(Action::Filter),
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
