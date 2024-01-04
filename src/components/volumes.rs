use color_eyre::Result;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, TableState, Wrap};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::{Component, VolumeInspect};
use crate::runtime::{delete_volume, get_volume, list_volumes, VolumeSummary};
use crate::utils::{centered_rect, table};

const VOLUME_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Max(15),
    Constraint::Min(35),
    Constraint::Max(10),
    Constraint::Max(20),
];

#[derive(Clone, Debug)]
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
    Driver(SortOrder),
    Size(SortOrder),
    Age(SortOrder),
}

#[derive(Clone, Debug)]
pub struct Volumes {
    state: TableState,
    volumes: Vec<VolumeSummary>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
    sort_by: SortColumn,
    filter: Option<String>,
}

impl Volumes {
    pub fn new() -> Self {
        Volumes {
            state: Default::default(),
            volumes: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
            sort_by: SortColumn::Id(SortOrder::Asc),
            filter: None,
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
            .map(|v| v.id.to_string())
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

    fn sort(&mut self) {
        self.volumes.sort_by(|a, b| {
            let (cmp_result, o) = match &self.sort_by {
                SortColumn::Id(o) => (a.id.cmp(&b.id), o),
                SortColumn::Driver(o) => (a.driver.cmp(&b.driver), o),
                SortColumn::Size(o) => (a.size.cmp(&b.size), o),
                SortColumn::Age(o) => (a.created.cmp(&b.created), o),
            };
            match o {
                SortOrder::Asc => cmp_result,
                SortOrder::Desc => cmp_result.reverse(),
            }
        });
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "Volumes"
    }

    pub(crate) fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        let tx = self.action_tx.clone().expect("No action sender available");
        match action {
            Action::Tick => match list_volumes().await {
                Ok(volumes) => {
                    self.volumes = volumes;
                    self.sort();
                    if self.state.selected().is_none() {
                        self.state.select(Some(0));
                    }
                }
                Err(e) => tx.send(Action::Error(format!("Error listing volumes:\n{}", e)))?,
            },
            Action::Down => {
                self.next();
            }
            Action::Up => {
                self.previous();
            }
            Action::Inspect => {
                if let Some(info) = self.get_selected_volume_info() {
                    let id = info.to_string();
                    let action = match get_volume(&id).await {
                        Ok(details) => Action::Screen(Component::VolumeInspect(
                            VolumeInspect::new(id, details),
                        )),
                        Err(e) => Action::Error(format!(
                            "Unable to get network \"{}\" details:\n{}",
                            &id[0..12],
                            e
                        )),
                    };
                    tx.send(action)?;
                };
            }
            Action::SetFilter(filter) => {
                self.filter = filter;
            }
            Action::Delete => {
                if let Some(id) = self.get_selected_volume_info() {
                    self.show_popup = Popup::Delete(id);
                }
            }
            Action::Ok => {
                if let Popup::Delete(id) = &self.show_popup {
                    if let Err(e) = delete_volume(id).await {
                        tx.send(Action::Error(format!(
                            "Error deleting volume \"{}\":\n{}",
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
                    (2, SortColumn::Driver(SortOrder::Asc)) => SortColumn::Driver(SortOrder::Desc),
                    (2, _) => SortColumn::Driver(SortOrder::Asc),
                    (3, SortColumn::Size(SortOrder::Asc)) => SortColumn::Size(SortOrder::Desc),
                    (3, _) => SortColumn::Size(SortOrder::Asc),
                    (4, SortColumn::Age(SortOrder::Asc)) => SortColumn::Age(SortOrder::Desc),
                    (4, _) => SortColumn::Age(SortOrder::Asc),
                    _ => self.sort_by.clone(),
                }
            }
            _ => {}
        };
        Ok(())
    }

    pub(crate) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            format!(
                "{}{}",
                self.get_name(),
                match &self.filter {
                    Some(f) => format!(" - Filter: {}", f),
                    None => "".to_string(),
                }
            ),
            ["Id", "Driver", "Size", "Age"],
            self.volumes.iter().map(|v| (*v).clone().into()).collect(),
            &VOLUME_CONSTRAINTS,
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        Some(&[
            ("ctrl+d", "Delete"),
            ("i", "Inspect/View details"),
            ("F1", "Sort by volume id"),
            ("F2", "Sort by volume driver"),
            ("F3", "Sort by volume size"),
            ("F4", "Sort by volume age"),
        ])
    }

    pub(crate) fn has_filter(&self) -> bool {
        true
    }
}
