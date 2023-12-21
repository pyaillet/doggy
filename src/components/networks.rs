use std::collections::HashMap;

use bollard::network::ListNetworksOptions;
use bollard::service::Network;
use color_eyre::Result;

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

const NETWORK_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Max(15),
    Constraint::Min(35),
    Constraint::Max(10),
    Constraint::Max(20),
];

enum Popup {
    None,
    Delete(String),
}

pub struct Networks {
    filters: HashMap<String, Vec<String>>,
    state: TableState,
    networks: Vec<[String; 4]>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Networks {
    pub fn new() -> Self {
        Networks {
            filters: HashMap::new(),
            state: Default::default(),
            networks: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
        }
    }

    fn previous(&mut self) {
        if !self.networks.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.networks.len() - 1
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
        if !self.networks.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.networks.len() - 1 {
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

    fn get_selected_network_info(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|i| self.networks.get(i))
            .and_then(|n| n.first())
            .cloned()
    }

    fn draw_popup(&self, f: &mut Frame<'_>) {
        if let Popup::Delete(id) = &self.show_popup {
            let text = vec![
                Line::from(vec![
                    Span::raw("Are you sure you want to delete network: \""),
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

impl Component for Networks {
    fn get_name(&self) -> &'static str {
        "Networks"
    }

    fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    fn update(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Tick => {
                let options = ListNetworksOptions {
                    filters: self.filters.clone(),
                };

                self.networks = block_on(async {
                    let docker_cli = Docker::connect_with_socket_defaults()
                        .expect("Unable to connect to docker");
                    let networks = docker_cli
                        .list_networks(Some(options))
                        .await
                        .expect("Unable to list networks");
                    networks
                        .iter()
                        .map(|n: &Network| {
                            [
                                n.id.to_owned().unwrap_or("<Unknown>".to_owned()),
                                n.name.to_owned().unwrap_or("<Unknown>".to_owned()),
                                n.driver.to_owned().unwrap_or("<Unknown>".to_owned()),
                                n.created.to_owned().unwrap_or("<Unknown>".to_owned()),
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
                if let Some(id) = self.get_selected_network_info() {
                    self.show_popup = Popup::Delete(id);
                }
            }
            Action::Ok => {
                if let Popup::Delete(id) = &self.show_popup {
                    delete_network(id)?;
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
            ["Id", "Name", "Size", "Age"],
            self.networks.clone(),
            &NETWORK_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }
}

fn delete_network(id: &str) -> Result<()> {
    block_on(async {
        let docker_cli =
            Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
        // TODO: Handle error correctly
        let _ = docker_cli.remove_network(id).await;
    });
    Ok(())
}
