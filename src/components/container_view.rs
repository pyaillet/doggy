use color_eyre::Result;

use crossterm::event;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, TableState};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::{containers::Containers, Component};
use crate::runtime::{get_container_details, ContainerDetails};
use crate::utils::table;

const CONTAINER_PROCESSES_CONSTRAINTS: [Constraint; 3] = [
    Constraint::Min(10),
    Constraint::Min(10),
    Constraint::Min(20),
];

#[derive(Clone, Debug)]
pub struct ContainerView {
    id: String,
    details: Option<ContainerDetails>,
    action_tx: Option<UnboundedSender<Action>>,
    state: TableState,
}

impl ContainerView {
    pub fn new(id: String) -> Self {
        ContainerView {
            id,
            details: None,
            action_tx: None,
            state: TableState::new(),
        }
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "ContainerView"
    }

    pub(crate) fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        let tx = self.action_tx.clone().expect("No action sender");
        match action {
            Action::PreviousScreen => {
                tx.send(Action::Screen(Component::Containers(Containers::new(None))))?;
            }
            Action::Tick => match get_container_details(&self.id).await {
                Ok(details) => self.details = Some(details),
                Err(e) => {
                    tx.send(Action::Error(e.to_string()))?;
                    self.details = None;
                }
            },
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn draw(
        &mut self,
        f: &mut ratatui::prelude::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        let nb_processes = self
            .details
            .as_ref()
            .map(|d| d.processes.len())
            .unwrap_or_default();

        let (detail_area, ps_area) = if nb_processes > 0 {
            let details_constraints: [Constraint; 2] = [
                Constraint::Min(20),
                Constraint::Max((nb_processes + 4) as u16),
            ];

            let rects = Layout::default()
                .direction(Direction::Vertical)
                .constraints(details_constraints)
                .split(area);
            (rects[0], rects[1])
        } else {
            (area, area)
        };

        let text: Vec<Line> = self
            .details
            .as_ref()
            .map(|d| d.into())
            .unwrap_or(vec![Line::from("Unable to get container details")]);

        let details = Paragraph::new(Text::from(text)).block(
            Block::default().borders(Borders::ALL).title(Span::styled(
                format!(
                    "Inspecting container: \"{}/{}\" (press 'ESC' to previous screen, 'q' to quit)",
                    &self.id[0..12],
                    self.details
                        .clone()
                        .map(|d| d.name)
                        .unwrap_or(String::from("<UNKNOWN>"))
                ),
                Style::default().add_modifier(Modifier::BOLD),
            )),
        );
        f.render_widget(details, detail_area);

        if nb_processes > 0 {
            let t = table(
                "Processes".into(),
                ["UID", "HOST_PID", "PROCESS"],
                self.details
                    .as_ref()
                    .map(|details| {
                        details
                            .processes
                            .iter()
                            .map(|(uid, pid, cmd)| {
                                Row::new(vec![uid.to_string(), pid.to_string(), cmd.to_string()])
                                    .style(Style::default().gray())
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                &CONTAINER_PROCESSES_CONSTRAINTS,
                Some(Style::new().gray()),
            );
            f.render_stateful_widget(t, ps_area, &mut self.state);
        }
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        Some(&[])
    }

    pub(crate) fn get_action(&self, _k: &event::KeyEvent) -> Option<Action> {
        None
    }
}
