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

use crate::components::Component;
use crate::utils::get_or_not_found;
use crate::{action::Action, utils::centered_rect};
use crate::{components::container_inspect::ContainerDetails, utils::table};

use super::container_exec::ContainerExec;

const CONTAINER_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Min(14),
    Constraint::Max(30),
    Constraint::Percentage(50),
    Constraint::Min(14),
];

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
                Ok(self.update(Some(Action::Refresh))?)
            }
            Some(Action::Inspect) => {
                let action = self.get_selected_container_info().map(|cinfo| {
                    Action::Screen(Box::new(ContainerDetails::new(
                        cinfo.0.to_string(),
                        cinfo.1.to_string(),
                    )))
                });
                Ok(action)
            }
            Some(Action::Shell) => {
                let action = self.get_selected_container_info().map(|cinfo| {
                    Action::Screen(Box::new(ContainerExec::new_with_default(cinfo.0)))
                });
                Ok(action)
            }
            Some(Action::Delete) => {
                if let Some((cid, cname)) = self.get_selected_container_info() {
                    self.show_popup = Popup::Delete(cid, cname);
                    Ok(Some(Action::Refresh))
                } else {
                    Ok(None)
                }
            }
            Some(Action::Ok) => {
                let show_popup = &self.show_popup;
                match show_popup {
                    Popup::Delete(cid, _) => {
                        delete_container(cid)?;
                        self.show_popup = Popup::None;
                        self.update(Some(Action::Refresh))
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

    fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            self.get_name(),
            ["Id", "Name", "Image", "Status"],
            self.containers.clone(),
            &CONTAINER_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }
}

fn delete_container(cid: &str) -> Result<()> {
    let options = RemoveContainerOptions {
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
