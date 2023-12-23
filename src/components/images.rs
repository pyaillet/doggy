use color_eyre::Result;

use futures::executor::block_on;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, TableState, Wrap};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;
use crate::runtime::{delete_image, list_images};
use crate::utils::{centered_rect, table};

const IMAGE_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Max(15),
    Constraint::Min(35),
    Constraint::Max(10),
    Constraint::Max(20),
];

#[derive(Clone, Debug)]
enum Popup {
    None,
    Delete(String, String),
}

pub struct Images {
    state: TableState,
    images: Vec<[String; 4]>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Images {
    pub fn new() -> Self {
        Images {
            state: Default::default(),
            images: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
        }
    }

    fn previous(&mut self) {
        if !self.images.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.images.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    fn next(&mut self) {
        if !self.images.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.images.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    fn get_selected_image_info(&self) -> Option<(String, String)> {
        self.state
            .selected()
            .and_then(|i| self.images.get(i))
            .and_then(|c| {
                c.first()
                    .and_then(|id| c.get(1).map(|tag| (id.to_owned(), tag.to_owned())))
            })
    }

    fn draw_popup(&self, f: &mut Frame<'_>) {
        if let Popup::Delete(_id, tag) = &self.show_popup {
            let text = vec![
                Line::from(vec![
                    Span::raw("Are you sure you want to delete image: \""),
                    Span::styled(tag, Style::new().gray()),
                    Span::raw("\"?"),
                ]),
                Line::from(""),
                Line::from(vec![
                    "ESC".bold(),
                    " to Cancel, ".into(),
                    "Enter".bold(),
                    " to Confirm".into(),
                ]),
            ];
            let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });

            let block = Block::default()
                .title("Confirmation".bold())
                .padding(Padding::new(1, 1, 1, 1))
                .borders(Borders::ALL);
            let area = centered_rect(50, 8, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(paragraph.block(block), area);
        }
    }
}

impl Component for Images {
    fn get_name(&self) -> &'static str {
        "Images"
    }

    fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    fn update(&mut self, action: Action) -> Result<()> {
        let tx = self.action_tx.clone().expect("No action sender available");
        match action {
            Action::Tick => {
                self.images = block_on(list_images())?;
            }
            Action::Down => {
                self.next();
            }
            Action::Up => {
                self.previous();
            }
            Action::Delete => {
                if let Some((id, tag)) = self.get_selected_image_info() {
                    self.show_popup = Popup::Delete(id, tag);
                }
            }
            Action::Ok => {
                if let Popup::Delete(id, _) = &self.show_popup.clone() {
                    if let Err(e) = block_on(delete_image(id)) {
                        tx.send(Action::Error(format!(
                            "Unable to delete container \"{}\" {}",
                            id, e
                        )))?;
                    } else {
                        self.show_popup = Popup::None;
                        tx.send(Action::Tick)?;
                    }
                };
            }
            Action::PreviousScreen => {
                self.show_popup = Popup::None;
            }
            _ => {}
        };
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            self.get_name().to_string(),
            ["Id", "Name", "Size", "Age"],
            self.images.clone(),
            &IMAGE_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }
}
