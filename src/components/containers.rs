use color_eyre::Result;

use crossterm::event::{self, KeyCode, KeyEventKind};
use futures::executor::block_on;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Padding, Paragraph, TableState, Wrap},
    Frame,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::utils::table;
use crate::{action::Action, utils::centered_rect};
use crate::{
    components::Component,
    runtime::{delete_container, get_container, list_containers},
};

use super::ComponentInit;

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

pub struct Containers {
    all: bool,
    state: TableState,
    containers: Vec<[String; 4]>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
    sort_by: SortColumn,
}

impl Containers {
    pub fn new() -> Self {
        Containers {
            all: false,
            state: Default::default(),
            containers: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
            sort_by: SortColumn::Name(SortOrder::Asc),
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
            let (idx, o) = match &self.sort_by {
                SortColumn::Id(o) => (0, o),
                SortColumn::Name(o) => (1, o),
                SortColumn::Image(o) => (2, o),
                SortColumn::Status(o) => (3, o),
            };
            match o {
                SortOrder::Asc => a[idx].cmp(&b[idx]),
                SortOrder::Desc => b[idx].cmp(&a[idx]),
            }
        });
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
                self.containers = match block_on(list_containers(self.all)) {
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
                if let Some(cinfo) = self.get_selected_container_info() {
                    let cid = cinfo.0.to_string();
                    let cname = cinfo.1.to_string();
                    let action = match block_on(get_container(&cid)) {
                        Ok(details) => {
                            Action::Screen(ComponentInit::ContainerInspect(cid, cname, details))
                        }
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
                    tx.send(Action::Screen(ComponentInit::ContainerLogs(cid, cname)))?;
                }
            }
            (Action::Shell, Popup::None) => {
                if let Some(action) = self.get_selected_container_info().map(|cinfo| {
                    Action::Screen(ComponentInit::ContainerExec(cinfo.0, cinfo.1, None))
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
                block_on(async {
                    if let Err(e) = block_on(delete_container(&cid)) {
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
            (Action::Ok, Popup::Shell(shell)) => {
                let action = Action::Screen(ComponentInit::ContainerExec(
                    shell.cid,
                    shell.cname,
                    Some(shell.input),
                ));
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

    fn handle_input(&mut self, kevent: event::KeyEvent) -> Result<()> {
        if let Popup::Shell(ref mut _shell_popup) = self.show_popup {
            if kevent.kind == KeyEventKind::Press {
                match kevent.code {
                    KeyCode::Char(to_insert) => {
                        self.enter_char(to_insert);
                    }
                    KeyCode::Backspace => {
                        self.delete_char();
                    }
                    KeyCode::Left => {
                        self.move_cursor_left();
                    }
                    KeyCode::Right => {
                        self.move_cursor_right();
                    }
                    KeyCode::Esc => {
                        self.show_popup = Popup::None;
                    }
                    _ => {}
                }
            };
        }
        Ok(())
    }

    fn get_bindings(&self) -> Option<&[(&str, &str)]> {
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

    fn get_action(&self, k: &event::KeyEvent) -> Option<Action> {
        match k.code {
            KeyCode::Char('i') => Some(Action::Inspect),
            KeyCode::Char('l') => Some(Action::Logs),
            KeyCode::Char('s') => Some(Action::Shell),
            KeyCode::Char('S') => Some(Action::CustomShell),
            _ => None,
        }
    }
}
