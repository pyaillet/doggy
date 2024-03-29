use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::action::Action;
use crate::components::composes::Composes;
use crate::components::containers::Containers;
use crate::components::images::Images;
use crate::components::networks::Networks;
use crate::components::volumes::Volumes;
use crate::components::Component;
use crate::runtime::{
    get_suggestions, RuntimeSummary, COMPOSES, CONTAINERS, IMAGES, NETWORKS, VOLUMES,
};
use crate::tui;
use crate::utils::{default_layout, help_screen, toast};

enum InputMode {
    None,
    Change,
    Filter,
}

const DEFAULT_TOAST_DELAY: usize = 8;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Popup {
    None,
    Error {
        msg: String,
        timeout: usize,
        ttl: usize,
    },
    Help,
}

pub struct App {
    should_quit: bool,
    should_suspend: bool,
    input: String,
    input_mode: InputMode,
    cursor_position: usize,
    suggestion: Option<&'static str>,
    version: &'static str,
    frame_rate: f64,
    tick_rate: f64,
    show_popup: Popup,
    runtime_info: Option<RuntimeSummary>,
}

impl App {
    pub fn new(version: &'static str, tick_rate: f64, frame_rate: f64) -> Self {
        App {
            should_quit: false,
            should_suspend: false,
            input: "".to_string(),
            input_mode: InputMode::None,
            suggestion: None,
            cursor_position: 0,
            version,
            frame_rate,
            tick_rate,
            show_popup: Popup::None,
            runtime_info: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();

        let mut tui = tui::Tui::new()?;
        tui.tick_rate(self.tick_rate);
        tui.frame_rate(self.frame_rate);
        tui.enter()?;

        let mut main: Component = Component::Containers(Containers::new(Default::default()));
        main.register_action_handler(action_tx.clone());

        let info = crate::runtime::get_runtime_info().await?;
        self.runtime_info = Some(info);

        loop {
            if let Some(event) = tui.next().await {
                match event {
                    tui::Event::Tick => action_tx.send(Action::Tick)?,
                    tui::Event::Render => action_tx.send(Action::Render)?,
                    tui::Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                    tui::Event::Key(kevent) => match self.input_mode {
                        InputMode::Change | InputMode::Filter => {
                            self.handle_input(kevent, action_tx.clone()).await?;
                        }
                        InputMode::None => {
                            if let Some(kevent) = main.handle_input(kevent)? {
                                self.handle_key(&main, kevent, action_tx.clone())?;
                            }
                        }
                    },
                    _ => {}
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                match action {
                    Action::Quit => self.should_quit = true,
                    Action::Suspend => self.should_suspend = true,
                    Action::Resume => {
                        self.should_suspend = false;
                        tui.resume()?;
                    }
                    Action::Resize(w, h) => {
                        tui.resize(Rect::new(0, 0, w, h))?;

                        self.draw(&mut tui, &mut main)?;
                    }
                    Action::Render => {
                        self.draw(&mut tui, &mut main)?;
                    }
                    Action::Tick => {
                        if let Popup::Error { ttl, .. } = &mut self.show_popup {
                            if *ttl > 0 {
                                *ttl = ttl.saturating_sub(1);
                            } else {
                                self.show_popup = Popup::None;
                            }
                        }
                    }
                    Action::Screen(ref screen) => {
                        let mut new_main = screen.clone();
                        new_main.register_action_handler(action_tx.clone());
                        new_main.setup(&mut tui)?;
                        main.teardown(&mut tui)?;
                        main = new_main;
                    }
                    Action::Change => {
                        self.input_mode = InputMode::Change;
                    }
                    Action::Filter => {
                        self.input_mode = InputMode::Filter;
                    }
                    Action::Help => {
                        self.show_popup = Popup::Help;
                    }
                    Action::PreviousScreen => {
                        if let InputMode::Change = self.input_mode {
                            self.reset_input();
                        }
                        match self.show_popup {
                            Popup::Error { .. } | Popup::Help => {
                                self.show_popup = Popup::None;
                            }
                            Popup::None => {}
                        }
                    }
                    Action::Error(ref msg) => {
                        self.show_popup = Popup::Error {
                            msg: msg.to_string(),
                            timeout: DEFAULT_TOAST_DELAY,
                            ttl: DEFAULT_TOAST_DELAY,
                        };
                    }
                    _ => {}
                };
                if let InputMode::None = self.input_mode {
                    main.update(action.clone()).await?;
                }
            }
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                tui = tui::Tui::new()?;
                tui.tick_rate(self.tick_rate);
                tui.frame_rate(self.frame_rate);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    fn draw(
        &mut self,
        tui: &mut tui::Tui,
        main_component: &mut Component,
    ) -> Result<(), color_eyre::eyre::Error> {
        let main_layout = default_layout(tui.get_frame().size());
        tui.draw(|f| {
            self.draw_header(f, main_layout[0]);
            main_component.draw(f, main_layout[1]);
            self.draw_popup(f, main_component);
            self.draw_status(f, main_layout[2]);
        })?;
        Ok(())
    }

    fn draw_header(&self, f: &mut ratatui::prelude::Frame<'_>, rect: ratatui::prelude::Rect) {
        match self.input_mode {
            InputMode::None => {
                let text = if let Some(info) = &self.runtime_info {
                    vec![
                        Line::from(format!(
                            "Welcome to Doggy - Using {}@{}",
                            info.name, info.version
                        )),
                        Line::from(format!(
                            "Connected to: {}",
                            info.config
                                .as_ref()
                                .map(|c| c.to_string())
                                .unwrap_or("<Unknown>".to_string())
                        )),
                    ]
                } else {
                    vec![Line::from("Welcome to Doggy")]
                };
                let p = Paragraph::new(text);
                f.render_widget(p, rect)
            }
            InputMode::Change => {
                let mut spans = vec![
                    Span::styled("> ", Style::default().bold()),
                    Span::styled(self.input.to_string(), Style::default().gray()),
                ];
                if let Some(suggestion) = self.suggestion {
                    spans.push(Span::styled(
                        suggestion[self.cursor_position..].to_string(),
                        Style::default().dark_gray(),
                    ));
                }

                let input = Paragraph::new(Line::from(spans))
                    .block(Block::default().borders(Borders::ALL).title("Input"));
                f.render_widget(input, rect);
            }
            InputMode::Filter => {
                let input = Paragraph::new(Line::from(vec![
                    Span::styled("/ ", Style::default().bold()),
                    Span::styled(self.input.to_string(), Style::default().gray()),
                ]))
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

    async fn handle_input(
        &mut self,
        kevent: event::KeyEvent,
        action_tx: UnboundedSender<Action>,
    ) -> Result<()> {
        if kevent.kind == KeyEventKind::Press {
            match kevent.code {
                KeyCode::Enter => match self.submit_input() {
                    Some(action) => {
                        action_tx.send(action)?;
                    }
                    None => {
                        action_tx.send(Action::Error("No resource found".to_string()))?;
                    }
                },
                KeyCode::Char(to_insert) => {
                    self.enter_char(to_insert);
                    self.suggestion = self.update_suggestion().await;
                }
                KeyCode::Backspace => {
                    self.delete_char();
                }
                KeyCode::Left => {
                    self.move_cursor_left();
                }
                KeyCode::Right => {
                    self.move_cursor_right();
                }
                KeyCode::Esc => {
                    self.input = "".to_string();
                    self.input_mode = InputMode::None;
                    self.reset_cursor();
                }
                _ => {}
            }
        };
        Ok(())
    }

    fn submit_input(&mut self) -> Option<Action> {
        if let InputMode::Change = self.input_mode {
            match self.suggestion {
                Some(CONTAINERS) => {
                    self.reset_input();
                    Some(Action::Screen(Component::Containers(Containers::new(
                        Default::default(),
                    ))))
                }
                Some(COMPOSES) => {
                    self.reset_input();
                    Some(Action::Screen(Component::Composes(Composes::new())))
                }
                Some(IMAGES) => {
                    self.reset_input();
                    Some(Action::Screen(Component::Images(Images::new())))
                }
                Some(VOLUMES) => {
                    self.reset_input();
                    Some(Action::Screen(Component::Volumes(Volumes::new(
                        Default::default(),
                    ))))
                }
                Some(NETWORKS) => {
                    self.reset_input();
                    Some(Action::Screen(Component::Networks(Networks::new(
                        Default::default(),
                    ))))
                }
                _ => None,
            }
        } else {
            let input = self.input.clone();
            self.reset_input();
            if input.is_empty() {
                Some(Action::SetFilter(None))
            } else {
                Some(Action::SetFilter(Some(input.clone())))
            }
        }
    }

    fn reset_input(&mut self) {
        self.input = "".to_string();
        self.cursor_position = 0;
        self.input_mode = InputMode::None;
    }

    async fn update_suggestion(&self) -> Option<&'static str> {
        get_suggestions()
            .await
            .iter()
            .find(|searched| searched.starts_with(&self.input))
            .copied()
    }

    fn handle_key(
        &self,
        main: &Component,
        kevent: event::KeyEvent,
        action_tx: UnboundedSender<Action>,
    ) -> Result<()> {
        let action = if self.show_popup == Popup::None {
            main.get_action(&kevent)
        } else {
            None
        };
        let action = action.or(match kevent.code {
            KeyCode::Char('a') => Some(Action::All),
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char(':') => Some(Action::Change),
            KeyCode::Char('/') => {
                if main.has_filter() {
                    Some(Action::Filter)
                } else {
                    None
                }
            }
            KeyCode::Char('j') | KeyCode::Down => Some(Action::Down),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::Up),
            KeyCode::Char('?') => Some(Action::Help),
            KeyCode::F(n) => Some(Action::SortColumn(n)),
            KeyCode::PageUp => Some(Action::PageUp),
            KeyCode::PageDown => Some(Action::PageDown),
            KeyCode::Esc => Some(Action::PreviousScreen),
            KeyCode::Enter => Some(Action::Ok),
            KeyCode::Char('d') => {
                if let KeyModifiers::CONTROL = kevent.modifiers {
                    Some(Action::Delete)
                } else {
                    None
                }
            }
            _ => None,
        });
        if let Some(action) = action {
            action_tx.send(action)?;
        }

        Ok(())
    }

    fn draw_popup(&mut self, f: &mut Frame<'_>, main_component: &Component) {
        match &mut self.show_popup {
            Popup::Error { msg, timeout, ttl } => {
                let title = Span::styled("Error", Style::new().red());
                toast(f, title, msg, *timeout, *ttl);
            }
            Popup::Help => {
                help_screen(f, main_component);
            }
            Popup::None => {}
        }
    }
}
