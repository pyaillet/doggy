use std::collections::HashMap;

use bollard::{container::ListContainersOptions, Docker};
use color_eyre::Result;

use futures::executor::block_on;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Clear, TableState},
    Frame,
};

use crate::components::container_inspect::ContainerDetails;
use crate::components::Component;
use crate::utils::get_or_not_found;
use crate::{action::Action, utils::centered_rect};

const CONTAINER_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Min(14),
    Constraint::Max(30),
    Constraint::Percentage(50),
    Constraint::Min(14),
];

enum Popup {
    None,
    Delete(String),
}

pub struct Containers {
    all: bool,
    filters: HashMap<String, Vec<String>>,
    state: TableState,
    containers: Vec<[String; 4]>,
    show_popup: Popup,
}

impl Containers {
    pub fn new() -> Self {
        Containers {
            all: false,
            filters: HashMap::new(),
            state: Default::default(),
            containers: Vec::new(),
            show_popup: Popup::None,
        }
    }

    fn previous(&mut self) {
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

    fn next(&mut self) {
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

    fn get_selected_container_id(&self) -> Option<String> {
        self.state
            .selected()
            .and_then(|i| self.containers.get(i))
            .and_then(|c| c.first())
            .cloned()
    }

    fn draw_popup(&self, f: &mut Frame<'_>) {
        if let Popup::Delete(_cid) = &self.show_popup {
            let block = Block::default().title("Confirmation").borders(Borders::ALL);
            let area = centered_rect(60, 20, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(block, area);
        }
    }
}

impl Component for Containers {
    fn get_name(&self) -> &'static str {
        "Containers"
    }

    fn update(&mut self, action: Option<Action>) -> Result<Option<Action>> {
        match action {
            Some(Action::Refresh) => {
                let options = ListContainersOptions {
                    all: self.all,
                    filters: self.filters.clone(),
                    ..Default::default()
                };

                self.containers = block_on(async {
                    let docker_cli = Docker::connect_with_socket_defaults()
                        .expect("Unable to connect to docker");
                    let containers = docker_cli
                        .list_containers(Some(options))
                        .await
                        .expect("Unable to get container list");
                    containers
                        .iter()
                        .map(|c| {
                            [
                                get_or_not_found!(c.id),
                                get_or_not_found!(c.names, |c| c
                                    .first()
                                    .and_then(|s| s.split('/').last())),
                                get_or_not_found!(c.image, |i| i.split('@').nth(0)),
                                get_or_not_found!(c.state),
                            ]
                        })
                        .collect()
                });
                Ok(None)
            }
            Some(Action::Down) => {
                self.next();
                Ok(None)
            }
            Some(Action::Up) => {
                self.previous();
                Ok(None)
            }
            Some(Action::All) => {
                self.all = !self.all;
                Ok(None)
            }
            Some(Action::Inspect) => {
                let cid = self
                    .get_selected_container_id()
                    .map(|cid| Action::Screen(Box::new(ContainerDetails::new(cid.to_string()))));
                Ok(cid)
            }
            Some(Action::Delete) => {
                if let Some(cid) = self.get_selected_container_id() {
                    self.show_popup = Popup::Delete(cid);
                    Ok(Some(Action::Refresh))
                } else {
                    Ok(None)
                }
            }
            Some(Action::Ok) => {
                let show_popup = &self.show_popup;
                match show_popup {
                    Popup::Delete(cid) => {
                        delete_container(cid)?;
                        self.show_popup = Popup::None;
                        Ok(Some(Action::Refresh))
                    }
                    _ => Ok(None),
                }
            }
            Some(Action::PreviousScreen) => {
                self.show_popup = Popup::None;
                Ok(Some(Action::Refresh))
            }
            _ => Ok(action),
        }
    }

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> Result<()> {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = crate::utils::table(
            ["Id", "Name", "Image", "Status"],
            self.containers.clone(),
            &CONTAINER_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
        Ok(())
    }
}

fn delete_container(cid: &str) -> Result<()> {
    let options = bollard::container::RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    block_on(async {
        let docker_cli =
            Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
        docker_cli
            .remove_container(cid, Some(options))
            .await
            .expect("Unable to remove container");
    });
    Ok(())
}
