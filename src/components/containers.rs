use std::collections::HashMap;

use bollard::{
    container::{ListContainersOptions, RemoveContainerOptions},
    Docker,
};
use color_eyre::Result;

use futures::executor::block_on;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, TableState, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::components::Component;
use crate::utils::get_or_not_found;
use crate::utils::table;
use crate::{action::Action, utils::centered_rect};

const CONTAINER_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Min(14),
    Constraint::Max(30),
    Constraint::Percentage(50),
    Constraint::Min(14),
];

#[derive(Clone, Debug)]
enum Popup {
    None,
    Delete(String, String),
}

pub struct Containers {
    all: bool,
    filters: HashMap<String, Vec<String>>,
    state: TableState,
    containers: Vec<[String; 4]>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Containers {
    pub fn new() -> Self {
        Containers {
            all: false,
            filters: HashMap::new(),
            state: Default::default(),
            containers: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
        }
    }

    fn previous(&mut self) {
        if !self.containers.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.containers.len() - 1
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
        if !self.containers.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.containers.len() - 1 {
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

    fn get_selected_container_info(&self) -> Option<(String, String)> {
        self.state
            .selected()
            .and_then(|i| self.containers.get(i))
            .and_then(|c| {
                c.first()
                    .and_then(|cid| c.get(1).map(|cname| (cid.to_owned(), cname.to_owned())))
            })
    }

    fn draw_popup(&self, f: &mut Frame<'_>) {
        if let Popup::Delete(_cid, cname) = &self.show_popup {
            let text = vec![
                Line::from(vec![
                    Span::raw("Are you sure you want to delete container: \""),
                    Span::styled(cname, Style::new().gray()),
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

impl Component for Containers {
    fn get_name(&self) -> &'static str {
        "Containers"
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    fn update(&mut self, action: Action) -> Result<()> {
        let tx = self
            .action_tx
            .clone()
            .expect("Action tx queue not initialized");
        match (action, self.show_popup.clone()) {
            (Action::Tick, Popup::None) => {
                let options = ListContainersOptions {
                    all: self.all,
                    filters: self.filters.clone(),
                    ..Default::default()
                };

                self.containers = block_on(async {
                    match list_containers(Some(options)).await {
                        Ok(containers) => containers,
                        Err(e) => {
                            tx.send(Action::Error(format!(
                                "Error getting container list: {}",
                                e
                            )))
                            .expect("Unable to send message");
                            vec![]
                        }
                    }
                });
            }
            (Action::Down, Popup::None) => {
                self.next();
            }
            (Action::Up, Popup::None) => {
                self.previous();
            }
            (Action::All, Popup::None) => {
                self.all = !self.all;
            }
            (Action::Inspect, Popup::None) => {
                if let Some(action) = self.get_selected_container_info().map(|cinfo| {
                    Action::Screen(super::ComponentInit::ContainerInspect(
                        cinfo.0.to_string(),
                        cinfo.1.to_string(),
                    ))
                }) {
                    tx.send(action)?;
                }
            }
            (Action::Shell, Popup::None) => {
                if let Some(action) = self
                    .get_selected_container_info()
                    .map(|cinfo| Action::Screen(super::ComponentInit::ContainerExec(cinfo.0, None)))
                {
                    tx.send(Action::Suspend)?;
                    tx.send(action)?;
                }
            }
            (Action::Delete, Popup::None) => {
                if let Some((cid, cname)) = self.get_selected_container_info() {
                    self.show_popup = Popup::Delete(cid, cname);
                }
            }
            (Action::Ok, Popup::Delete(cid, _)) => {
                block_on(async {
                    if let Err(e) = delete_container(&cid).await {
                        tx.send(Action::Error(format!(
                            "Unable to delete container \"{}\" {}",
                            cid, e
                        )))
                        .expect("Unable to send error");
                    } else {
                        self.show_popup = Popup::None;
                    }
                });
            }
            (Action::PreviousScreen, Popup::Delete(_, _)) => {
                self.show_popup = Popup::None;
            }
            _ => {}
        }
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            format!(
                "{} ({})",
                self.get_name(),
                if self.all { "All" } else { "Running" }
            ),
            ["Id", "Name", "Image", "Status"],
            self.containers.clone(),
            &CONTAINER_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }
}

async fn delete_container(cid: &str) -> Result<()> {
    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    let docker_cli = Docker::connect_with_socket_defaults()?;
    docker_cli.remove_container(cid, Some(options)).await?;
    Ok(())
}

async fn list_containers(
    options: Option<ListContainersOptions<String>>,
) -> Result<Vec<[String; 4]>> {
    let docker_cli = Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
    let containers = docker_cli
        .list_containers(options)
        .await
        .expect("Unable to get container list");
    let containers = containers
        .iter()
        .map(|c| {
            [
                get_or_not_found!(c.id),
                get_or_not_found!(c.names, |c| c.first().and_then(|s| s.split('/').last())),
                get_or_not_found!(c.image, |i| i.split('@').next()),
                get_or_not_found!(c.state),
            ]
        })
        .collect();
    Ok(containers)
}
