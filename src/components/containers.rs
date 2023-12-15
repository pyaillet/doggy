use std::collections::HashMap;

use bollard::{container::ListContainersOptions, Docker};
use color_eyre::Result;

use futures::executor::block_on;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::TableState,
    Frame,
};

use crate::action::Action;
use crate::components::container_inspect::ContainerDetails;
use crate::components::Component;
use crate::utils::get_or_not_found;

const CONTAINER_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Min(14),
    Constraint::Max(30),
    Constraint::Percentage(50),
    Constraint::Min(14),
];

pub struct Containers {
    all: bool,
    filters: HashMap<String, Vec<String>>,
    state: TableState,
    containers: Vec<[String; 4]>,
}

impl Containers {
    pub fn new() -> Self {
        Containers {
            all: false,
            filters: HashMap::new(),
            state: Default::default(),
            containers: Vec::new(),
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
                                get_or_not_found!(c.names, |c| c.first()),
                                get_or_not_found!(c.image),
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
                    .state
                    .selected()
                    .and_then(|i| self.containers.get(i))
                    .and_then(|c| c.first())
                    .map(|cid| Action::Screen(Box::new(ContainerDetails::new(cid.to_string()))));
                Ok(cid)
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
        Ok(())
    }
}
