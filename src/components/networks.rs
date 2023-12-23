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
use crate::runtime::{delete_network, get_network, list_networks};
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

#[derive(Clone, Debug)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub enum SortColumn {
    Id(SortOrder),
    Name(SortOrder),
    Driver(SortOrder),
    Age(SortOrder),
}

pub struct Networks {
    state: TableState,
    networks: Vec<[String; 4]>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
    sort_by: SortColumn,
}

impl Networks {
    pub fn new() -> Self {
        Networks {
            state: Default::default(),
            networks: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
            sort_by: SortColumn::Name(SortOrder::Asc),
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

    fn get_selected_network_info(&self) -> Option<(String, String)> {
        self.state
            .selected()
            .and_then(|i| self.networks.get(i))
            .and_then(|n| match (n.first(), n.get(1)) {
                (Some(id), Some(name)) => Some((id.to_string(), name.to_string())),
                _ => None,
            })
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

    fn sort(&mut self) {
        self.networks.sort_by(|a, b| {
            let (idx, o) = match &self.sort_by {
                SortColumn::Id(o) => (0, o),
                SortColumn::Name(o) => (1, o),
                SortColumn::Driver(o) => (2, o),
                SortColumn::Age(o) => (3, o),
            };
            match o {
                SortOrder::Asc => a[idx].cmp(&b[idx]),
                SortOrder::Desc => b[idx].cmp(&a[idx]),
            }
        });
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
        let tx = self.action_tx.clone().expect("No action sender available");
        match action {
            Action::Tick => match block_on(list_networks()) {
                Ok(networks) => {
                    self.networks = networks;
                    self.sort();
                }
                Err(e) => self
                    .action_tx
                    .clone()
                    .expect("No action sender availabel")
                    .send(Action::Error(format!("Unable to list networks:\n{}", e)))?,
            },
            Action::Down => {
                self.next();
            }
            Action::Up => {
                self.previous();
            }
            Action::Inspect => {
                if let Some(info) = self.get_selected_network_info() {
                    let id = info.0.to_string();
                    let name = info.1.to_string();
                    let action = match block_on(get_network(&name)) {
                        Ok(details) => {
                            Action::Screen(super::ComponentInit::NetworkInspect(id, name, details))
                        }
                        Err(e) => Action::Error(format!(
                            "Unable to get network \"{}\" details:\n{}",
                            name, e
                        )),
                    };
                    tx.send(action)?;
                };
            }
            Action::Delete => {
                if let Some((id, _)) = self.get_selected_network_info() {
                    self.show_popup = Popup::Delete(id);
                }
            }
            Action::Ok => {
                if let Popup::Delete(id) = &self.show_popup {
                    if let Err(e) = block_on(delete_network(id)) {
                        tx.send(Action::Error(format!(
                            "Unable to delete network \"{}\":\n{}",
                            id, e
                        )))?;
                    }
                    self.show_popup = Popup::None;
                    tx.send(Action::Tick)?;
                }
            }
            Action::PreviousScreen => {
                self.show_popup = Popup::None;
            }
            Action::SortColumn(n) => {
                self.sort_by = match (n, &self.sort_by) {
                    (1, SortColumn::Id(SortOrder::Asc)) => SortColumn::Id(SortOrder::Desc),
                    (1, _) => SortColumn::Id(SortOrder::Asc),
                    (2, SortColumn::Name(SortOrder::Asc)) => SortColumn::Age(SortOrder::Desc),
                    (2, _) => SortColumn::Name(SortOrder::Asc),
                    (3, SortColumn::Driver(SortOrder::Asc)) => SortColumn::Driver(SortOrder::Desc),
                    (3, _) => SortColumn::Driver(SortOrder::Asc),
                    (4, SortColumn::Age(SortOrder::Asc)) => SortColumn::Age(SortOrder::Desc),
                    (4, _) => SortColumn::Age(SortOrder::Asc),
                    _ => self.sort_by.clone(),
                }
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
            ["Id", "Name", "Driver", "Age"],
            self.networks.clone(),
            &NETWORK_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }
}
