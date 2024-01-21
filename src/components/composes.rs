use color_eyre::Result;
use crossterm::event::{self, KeyCode};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    widgets::TableState,
    Frame,
};

use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    runtime::{list_compose_projects, Compose, Filter},
    utils::table,
};

use super::{
    compose_view::ComposeView, containers::Containers, networks::Networks, volumes::Volumes,
    Component,
};

const COMPOSES_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Min(20),
    Constraint::Max(12),
    Constraint::Max(12),
    Constraint::Max(12),
];

#[derive(Clone, Debug)]
pub struct Composes {
    composes: Vec<Compose>,
    action_tx: Option<UnboundedSender<Action>>,
    state: TableState,
}

impl Composes {
    pub fn new() -> Self {
        Composes {
            composes: Vec::new(),
            action_tx: None,
            state: TableState::default(),
        }
    }

    fn previous(&mut self) {
        if !self.composes.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.composes.len() - 1
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
        if !self.composes.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.composes.len() - 1 {
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

    fn get_selected_compose_info(&self) -> Option<Compose> {
        self.state
            .selected()
            .and_then(|i| self.composes.get(i).cloned())
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "Compose projects"
    }

    pub(crate) fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        let tx = self
            .action_tx
            .clone()
            .expect("Action tx queue not initialized");
        match action {
            Action::Tick => {
                self.composes = match list_compose_projects().await {
                    Ok(composes) => composes,
                    Err(e) => {
                        tx.send(Action::Error(format!(
                            "Error getting container list: {}",
                            e
                        )))?;
                        Vec::new()
                    }
                };
                self.composes.sort_by(|a, b| a.project.cmp(&b.project));
                if self.state.selected().is_none() {
                    self.state.select(Some(0));
                }
            }
            Action::Down => {
                self.next();
            }
            Action::Up => {
                self.previous();
            }
            Action::Ok => {}
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            self.get_name().to_string(),
            ["Project", "Containers", "Volumes", "Networks"],
            self.composes.iter().map(|c| c.into()).collect(),
            &COMPOSES_CONSTRAINTS,
            Some(Style::new().gray()),
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        Some(&[
            ("Enter", "Compose details"),
            ("c", "Containers"),
            ("v", "Volumes"),
            ("n", "Networks"),
        ])
    }

    pub(crate) fn get_action(&self, k: &event::KeyEvent) -> Option<Action> {
        if let Some(compose) = self.get_selected_compose_info() {
            let filter = Filter::default().compose_project(compose.project.clone());
            match k.code {
                KeyCode::Enter => Some(Action::Screen(Component::ComposeView(ComposeView::new(
                    compose,
                )))),
                KeyCode::Char('c') => Some(Action::Screen(Component::Containers(Containers::new(
                    filter,
                )))),
                KeyCode::Char('v') => {
                    Some(Action::Screen(Component::Volumes(Volumes::new(filter))))
                }
                KeyCode::Char('n') => {
                    Some(Action::Screen(Component::Networks(Networks::new(filter))))
                }
                _ => None,
            }
        } else {
            None
        }
    }
}
