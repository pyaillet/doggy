use std::collections::HashMap;

use bollard::service::Volume;
use bollard::volume::{ListVolumesOptions, RemoveVolumeOptions};
use color_eyre::Result;

use humansize::{FormatSizeI, BINARY};

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
use crate::utils::{centered_rect, table};

const VOLUME_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Max(15),
    Constraint::Min(35),
    Constraint::Max(10),
    Constraint::Max(20),
];

enum Popup {
    None,
    Delete(String),
}

pub struct Volumes {
    filters: HashMap<String, Vec<String>>,
    state: TableState,
    volumes: Vec<[String; 4]>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Volumes {
    pub fn new() -> Self {
        Volumes {
            filters: HashMap::new(),
            state: Default::default(),
            volumes: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
        }
    }

    fn previous(&mut self) {
        if !self.volumes.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.volumes.len() - 1
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
        if !self.volumes.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.volumes.len() - 1 {
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

    fn get_selected_volume_info(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|i| self.volumes.get(i))
            .and_then(|i| i.first())
            .cloned()
    }

    fn draw_popup(&self, f: &mut Frame<'_>) {
        if let Popup::Delete(id) = &self.show_popup {
            let text = vec![
                Line::from(vec![
                    Span::raw("Are you sure you want to delete volume: \""),
                    Span::styled(id, Style::new().gray()),
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

impl Component for Volumes {
    fn get_name(&self) -> &'static str {
        "Volumes"
    }

    fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    fn update(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Tick => {
                let options = ListVolumesOptions {
                    filters: self.filters.clone(),
                };

                self.volumes = block_on(async {
                    let docker_cli = Docker::connect_with_socket_defaults()
                        .expect("Unable to connect to docker");
                    let result = docker_cli
                        .list_volumes(Some(options))
                        .await
                        .expect("Unable to list volumes");
                    result
                        .volumes
                        .unwrap_or(vec![])
                        .iter()
                        .map(|v: &Volume| {
                            [
                                v.name.to_owned(),
                                v.driver.to_owned(),
                                v.usage_data
                                    .to_owned()
                                    .map(|usage| usage.size)
                                    .map(|s| s.format_size_i(BINARY))
                                    .unwrap_or("<Unknown>".to_owned()),
                                v.created_at.to_owned().unwrap_or("<Unknown>".to_string()),
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
                if let Some(id) = self.get_selected_volume_info() {
                    self.show_popup = Popup::Delete(id);
                }
            }
            Action::Ok => {
                if let Popup::Delete(id) = &self.show_popup {
                    delete_volume(id)?;
                    self.show_popup = Popup::None;
                    if let Some(tx) = self.action_tx.clone() {
                        tx.send(Action::Tick)?;
                    }
                }
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
            ["Name", "Driver", "Size", "Age"],
            self.volumes.clone(),
            &VOLUME_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }
}

fn delete_volume(id: &str) -> Result<()> {
    let options = RemoveVolumeOptions { force: true };
    block_on(async {
        let docker_cli =
            Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
        // TODO: Handle error correctly
        let _ = docker_cli.remove_volume(id, Some(options)).await;
    });
    Ok(())
}
