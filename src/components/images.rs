use std::collections::HashMap;

use bollard::service::ImageSummary;
use chrono::{DateTime, Utc};
use color_eyre::Result;

use humansize::{FormatSizeI, BINARY};

use bollard::image::{ListImagesOptions, RemoveImageOptions};
use bollard::Docker;
use futures::executor::block_on;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, TableState, Wrap};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::Component;
use crate::utils::{centered_rect, get_or_not_found, table};

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
    filters: HashMap<String, Vec<String>>,
    state: TableState,
    images: Vec<[String; 4]>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Images {
    pub fn new() -> Self {
        Images {
            filters: HashMap::new(),
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
        match action {
            Action::Tick => {
                let options = ListImagesOptions {
                    filters: self.filters.clone(),
                    ..Default::default()
                };

                self.images = block_on(async {
                    let docker_cli = Docker::connect_with_socket_defaults()
                        .expect("Unable to connect to docker");
                    let images = docker_cli
                        .list_images(Some(options))
                        .await
                        .expect("Unable to list images");
                    images
                        .iter()
                        .map(|i: &ImageSummary| {
                            [
                                i.id.to_string().split(':').last().unwrap()[0..12].to_string(),
                                get_or_not_found!(i.repo_tags.first()),
                                i.size.format_size_i(BINARY),
                                DateTime::<Utc>::from_timestamp(i.created, 0)
                                    .expect("Unable to parse timestamp")
                                    .to_string(),
                            ]
                        })
                        .collect()
                });
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
                    block_on(async {
                        if let Some(tx) = self.action_tx.clone() {
                            if let Err(e) = delete_image(id).await {
                                tx.send(Action::Error(format!(
                                    "Unable to delete container \"{}\" {}",
                                    id, e
                                )))
                                .expect("Unable to send error");
                            } else {
                                self.show_popup = Popup::None;
                                tx.send(Action::Tick).expect("Unable to send tick");
                            }
                        }
                    });
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

async fn delete_image(id: &str) -> Result<()> {
    let options = RemoveImageOptions {
        force: true,
        ..Default::default()
    };
    let docker_cli = Docker::connect_with_socket_defaults()?;
    // TODO: Handle error correctly
    docker_cli.remove_image(id, Some(options), None).await?;
    Ok(())
}
