use color_eyre::Result;

use crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, TableState, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::runtime::{
    delete_container, get_container, list_containers, validate_container_filters,
};
use crate::{action::Action, utils::centered_rect};
use crate::{runtime::ContainerSummary, utils::table};

use crate::components::{
    container_exec::ContainerExec, container_inspect::ContainerDetails,
    container_logs::ContainerLogs, Component,
};

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
    Shell(ShellPopup),
}

#[derive(Clone, Debug, Default)]
struct ShellPopup {
    cid: String,
    cname: String,
    input: String,
    cursor_position: usize,
}

impl ShellPopup {
    fn new(cid: String, cname: String) -> Self {
        ShellPopup {
            cid,
            cname,
            ..Default::default()
        }
    }
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
    Image(SortOrder),
    Status(SortOrder),
}

#[derive(Clone, Debug)]
pub struct Containers {
    all: bool,
    state: TableState,
    containers: Vec<ContainerSummary>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
    sort_by: SortColumn,
    filter: Option<String>,
}

impl Containers {
    pub fn new(filter: Option<String>) -> Self {
        Containers {
            all: false,
            state: Default::default(),
            containers: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
            sort_by: SortColumn::Name(SortOrder::Asc),
            filter,
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
            .and_then(|i| self.containers.get(i).cloned())
            .map(|c| (c.id, c.name))
    }

    fn draw_popup(&self, f: &mut Frame<'_>) {
        match &self.show_popup {
            Popup::Delete(_cid, cname) => {
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
            Popup::Shell(shell_popup) => {
                let text = vec![
                    Line::from(vec![Span::raw(
                        "You will launch the following command in the container:",
                    )]),
                    Line::from(""),
                    Line::from(format!("> {}", shell_popup.input.clone())),
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
                    .title("Launch command".bold())
                    .padding(Padding::new(1, 1, 1, 1))
                    .borders(Borders::ALL);
                let area = centered_rect(50, 10, f.size());
                f.render_widget(Clear, area); //this clears out the background
                f.render_widget(paragraph.block(block), area);
            }
            _ => {}
        }
    }

    fn delete_char(&mut self) {
        if let Popup::Shell(ref mut shell_popup) = self.show_popup {
            let is_not_cursor_leftmost = shell_popup.cursor_position != 0;
            if is_not_cursor_leftmost {
                // Method "remove" is not used on the saved text for deleting the selected char.
                // Reason: Using remove on String works on bytes instead of the chars.
                // Using remove would require special care because of char boundaries.

                let current_index = shell_popup.cursor_position;
                let from_left_to_current_index = current_index - 1;

                // Getting all characters before the selected character.
                let before_char_to_delete =
                    shell_popup.input.chars().take(from_left_to_current_index);
                // Getting all characters after selected character.
                let after_char_to_delete = shell_popup.input.chars().skip(current_index);

                // Put all characters together except the selected one.
                // By leaving the selected one out, it is forgotten and therefore deleted.
                shell_popup.input = before_char_to_delete.chain(after_char_to_delete).collect();
                self.move_cursor_left();
            }
        }
    }

    fn enter_char(&mut self, new_char: char) {
        if let Popup::Shell(ref mut shell_popup) = self.show_popup {
            shell_popup
                .input
                .insert(shell_popup.cursor_position, new_char);

            self.move_cursor_right();
        }
    }

    fn move_cursor_left(&mut self) {
        if let Popup::Shell(ref mut shell_popup) = self.show_popup {
            let cursor_moved_left = shell_popup.cursor_position.saturating_sub(1);
            let length = shell_popup.input.len();
            shell_popup.cursor_position = cursor_moved_left.clamp(0, length);
        }
    }

    fn move_cursor_right(&mut self) {
        if let Popup::Shell(ref mut shell_popup) = self.show_popup {
            let cursor_moved_right = shell_popup.cursor_position.saturating_add(1);
            let length = shell_popup.input.len();
            shell_popup.cursor_position = cursor_moved_right.clamp(0, length);
        }
    }

    fn sort(&mut self) {
        self.containers.sort_by(|a, b| {
            let (cmp_result, o) = match &self.sort_by {
                SortColumn::Id(o) => (a.id.cmp(&b.id), o),
                SortColumn::Name(o) => (a.name.cmp(&b.name), o),
                SortColumn::Image(o) => (a.image.cmp(&b.image), o),
                SortColumn::Status(o) => (a.status.cmp(&b.status), o),
            };
            match o {
                SortOrder::Asc => cmp_result,
                SortOrder::Desc => cmp_result.reverse(),
            }
        });
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "Containers"
    }

    pub(crate) fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        let tx = self
            .action_tx
            .clone()
            .expect("Action tx queue not initialized");
        match (action, self.show_popup.clone()) {
            (Action::Tick, Popup::None) => {
                self.containers = match list_containers(self.all, &self.filter).await {
                    Ok(containers) => containers,
                    Err(e) => {
                        tx.send(Action::Error(format!(
                            "Error getting container list: {}",
                            e
                        )))?;
                        vec![]
                    }
                };
                self.sort();
                if self.state.selected().is_none() {
                    self.state.select(Some(0));
                }
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
            (Action::SetFilter(filter), Popup::None) => {
                if let Some(filter) = filter {
                    if validate_container_filters(&filter).await {
                        self.filter = Some(filter);
                    } else {
                        tx.send(Action::Error(format!("Invalid filter: {}", filter)))?;
                    }
                } else {
                    self.filter = filter;
                }
            }
            (Action::Inspect, Popup::None) => {
                if let Some(cinfo) = self.get_selected_container_info() {
                    let cid = cinfo.0.to_string();
                    let cname = cinfo.1.to_string();
                    let action = match get_container(&cid).await {
                        Ok(details) => Action::Screen(Component::ContainerInspect(
                            ContainerDetails::new(cid, cname, details),
                        )),
                        Err(e) => Action::Error(format!(
                            "Unable to get container \"{}\" details:\n{}",
                            cname, e
                        )),
                    };
                    tx.send(action)?;
                };
            }
            (Action::Logs, Popup::None) => {
                if let Some(cinfo) = self.get_selected_container_info() {
                    let cid = cinfo.0.to_string();
                    let cname = cinfo.1.to_string();
                    tx.send(Action::Screen(Component::ContainerLogs(
                        ContainerLogs::new(cid, cname),
                    )))?;
                }
            }
            (Action::Shell, Popup::None) => {
                if let Some(action) = self.get_selected_container_info().map(|cinfo| {
                    Action::Screen(Component::ContainerExec(ContainerExec::new(
                        cinfo.0, cinfo.1, None,
                    )))
                }) {
                    tx.send(Action::Suspend)?;
                    tx.send(action)?;
                }
            }
            (Action::CustomShell, Popup::None) => {
                if let Some((cid, cname)) = self.get_selected_container_info() {
                    self.show_popup = Popup::Shell(ShellPopup::new(cid, cname));
                }
            }
            (Action::Delete, Popup::None) => {
                if let Some((cid, cname)) = self.get_selected_container_info() {
                    self.show_popup = Popup::Delete(cid, cname);
                }
            }
            (Action::Ok, Popup::Delete(cid, _)) => {
                if let Err(e) = delete_container(&cid).await {
                    tx.send(Action::Error(format!(
                        "Unable to delete container \"{}\" {}",
                        cid, e
                    )))
                    .expect("Unable to send error");
                } else {
                    self.show_popup = Popup::None;
                }
            }
            (Action::Ok, Popup::Shell(shell)) => {
                let action = Action::Screen(Component::ContainerExec(ContainerExec::new(
                    shell.cid,
                    shell.cname,
                    Some(shell.input),
                )));
                tx.send(Action::Suspend)?;
                tx.send(action)?;
            }
            (Action::PreviousScreen, Popup::Delete(_, _))
            | (Action::PreviousScreen, Popup::Shell(_)) => {
                self.show_popup = Popup::None;
            }
            (Action::SortColumn(n), Popup::None) => {
                self.sort_by = match (n, &self.sort_by) {
                    (1, SortColumn::Id(SortOrder::Asc)) => SortColumn::Id(SortOrder::Desc),
                    (1, _) => SortColumn::Id(SortOrder::Asc),
                    (2, SortColumn::Name(SortOrder::Asc)) => SortColumn::Name(SortOrder::Desc),
                    (2, _) => SortColumn::Name(SortOrder::Asc),
                    (3, SortColumn::Image(SortOrder::Asc)) => SortColumn::Image(SortOrder::Desc),
                    (3, _) => SortColumn::Image(SortOrder::Asc),
                    (4, SortColumn::Status(SortOrder::Asc)) => SortColumn::Status(SortOrder::Desc),
                    (4, _) => SortColumn::Status(SortOrder::Asc),
                    _ => self.sort_by.clone(),
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            format!(
                "{} ({}{})",
                self.get_name(),
                if self.all { "All" } else { "Running" },
                if let Some(filter) = &self.filter {
                    format!(" - Filter: {}", filter)
                } else {
                    "".to_string()
                }
            ),
            ["Id", "Name", "Image", "Status"],
            self.containers.iter().map(|c| c.into()).collect(),
            &CONTAINER_CONSTRAINTS,
            Some(Style::new().gray()),
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }

    pub(crate) fn handle_input(
        &mut self,
        kevent: event::KeyEvent,
    ) -> Result<Option<event::KeyEvent>> {
        if let Popup::Shell(ref mut _shell_popup) = self.show_popup {
            if kevent.kind == KeyEventKind::Press {
                match kevent.code {
                    KeyCode::Char(to_insert) => {
                        self.enter_char(to_insert);
                        Ok(None)
                    }
                    KeyCode::Backspace => {
                        self.delete_char();
                        Ok(None)
                    }
                    KeyCode::Left => {
                        self.move_cursor_left();
                        Ok(None)
                    }
                    KeyCode::Right => {
                        self.move_cursor_right();
                        Ok(None)
                    }
                    KeyCode::Esc => {
                        self.show_popup = Popup::None;
                        Ok(None)
                    }
                    _ => Ok(Some(kevent)),
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(Some(kevent))
        }
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        Some(&[
            ("ctrl+d", "Delete"),
            ("i", "Inspect"),
            ("l", "Logs"),
            ("s", "Execute '/bin/bash' in container"),
            ("S", "Execute custom command"),
            ("F1", "Sort by container id"),
            ("F2", "Sort by container name"),
            ("F3", "Sort by image name"),
            ("F4", "Sort by status"),
        ])
    }

    pub(crate) fn get_action(&self, k: &event::KeyEvent) -> Option<Action> {
        match k.code {
            KeyCode::Char('i') => Some(Action::Inspect),
            KeyCode::Char('l') => Some(Action::Logs),
            KeyCode::Char('s') => Some(Action::Shell),
            KeyCode::Char('S') => Some(Action::CustomShell),
            _ => None,
        }
    }

    pub(crate) fn has_filter(&self) -> bool {
        true
    }
}
