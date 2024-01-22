use bollard::container::StatsOptions;
use color_eyre::Result;

use crossterm::event::{self, KeyCode, KeyEventKind};
use futures::{executor::block_on, future::join_all, StreamExt};
use humansize::{format_size, FormatSizeOptions, BINARY};

use std::{collections::HashMap, sync::Arc, time::Duration};

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Padding, Paragraph, Row, TableState, Wrap},
    Frame,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::{select, spawn};
use tokio::{sync::mpsc::UnboundedSender, time::sleep};
use tokio_util::sync::CancellationToken;

use crate::{action::Action, utils::centered_rect};
use crate::{runtime::ContainerSummary, utils::table};
use crate::{
    runtime::{
        delete_container,
        docker::{compute_cpu, compute_mem},
        get_container, get_container_stats, list_containers, validate_container_filters,
        ContainerMetrics, Filter,
    },
    tui,
};

use crate::components::{
    container_exec::ContainerExec, container_inspect::ContainerDetails,
    container_logs::ContainerLogs, container_view::ContainerView, Component,
};

const CONTAINER_CONSTRAINTS: [Constraint; 7] = [
    Constraint::Percentage(20),
    Constraint::Percentage(20),
    Constraint::Percentage(20),
    Constraint::Percentage(20),
    Constraint::Max(4),
    Constraint::Max(5),
    Constraint::Max(9),
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
    Age(SortOrder),
}

#[derive(Clone, Debug)]
pub struct Containers {
    all: bool,
    state: TableState,
    containers: Vec<ContainerSummary>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
    sort_by: SortColumn,
    filter: Filter,
    metrics: Arc<Mutex<HashMap<String, ContainerMetrics>>>,
    task: Arc<JoinHandle<Result<()>>>,
    cancellation_token: CancellationToken,
}

async fn run_setup_task(
    metrics: Arc<Mutex<HashMap<String, ContainerMetrics>>>,
    cancel: CancellationToken,
) -> Result<()> {
    let mut should_stop = false;
    while !should_stop {
        select!(
        _ = update_metrics(Arc::clone(&metrics)) => {},
        _ = cancel.cancelled() => {
            should_stop = true;
        }
        );
    }
    Ok(())
}

async fn update_metrics(metrics: Arc<Mutex<HashMap<String, ContainerMetrics>>>) -> Result<()> {
    let container_list = list_containers(false, &Filter::default()).await?;
    let options = Some(StatsOptions {
        stream: false,
        one_shot: false,
    });
    let stats_futures = join_all(container_list.iter().map(|c| async {
        match get_container_stats(&c.id, options).await {
            Ok(mut stats) => match stats.next().await {
                Some(Ok(stats)) => Some((c.id.clone(), compute_cpu(&stats), compute_mem(&stats))),
                _ => None,
            },
            Err(_) => None,
        }
    }))
    .await;

    let mut map_lock = metrics.lock().await;
    for cid_stats in stats_futures.into_iter().filter(|s| s.is_some()) {
        let (cid, cpu_usage, mem_usage) = cid_stats.expect("Already checked and filtered out None");
        let entry = map_lock.get_mut(&cid);
        match entry {
            Some(entry) => entry.push_metrics(cpu_usage, mem_usage),
            None => {
                let mut cm = ContainerMetrics::new(cid.clone(), 20);
                cm.push_metrics(cpu_usage, mem_usage);
                map_lock.insert(cid, cm);
            }
        }
    }
    drop(map_lock);

    sleep(Duration::from_millis(1000)).await;

    Ok(())
}

impl Containers {
    pub fn new(filter: Filter) -> Self {
        let metrics = Arc::new(Mutex::new(HashMap::new()));
        let cancel = CancellationToken::new();
        let _cancel = cancel.clone();

        let _metrics = Arc::clone(&metrics);

        let task = Arc::new(spawn(run_setup_task(_metrics, _cancel)));

        Containers {
            all: false,
            state: Default::default(),
            containers: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
            sort_by: SortColumn::Name(SortOrder::Asc),
            filter,
            metrics,
            task: Arc::clone(&task),
            cancellation_token: cancel,
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
                SortColumn::Age(o) => (a.age.cmp(&b.age), o),
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
                        self.filter = filter.into();
                    } else {
                        tx.send(Action::Error(format!("Invalid filter: {}", filter)))?;
                    }
                } else {
                    self.filter = Default::default();
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
            (Action::Ok, Popup::None) => {
                if let Some((cid, _)) = self.get_selected_container_info() {
                    let cid = cid.to_string();
                    tx.send(Action::Screen(Component::ContainerView(
                        ContainerView::new(cid),
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
                    (5, SortColumn::Age(SortOrder::Asc)) => SortColumn::Age(SortOrder::Desc),
                    (5, _) => SortColumn::Age(SortOrder::Asc),
                    _ => self.sort_by.clone(),
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let stats = block_on(async { self.metrics.lock().await.clone() });
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            format!(
                "{} ({}{})",
                self.get_name(),
                if self.all { "All" } else { "Running" },
                self.filter.format()
            ),
            ["Id", "Name", "Image", "Status", "Age", "CPU", "MEM"],
            self.containers
                .iter()
                .map(|c| {
                    let mut cells: Vec<Cell> = c.into();
                    if let Some(stats) = stats.get(&c.id) {
                        if let Some(cpu) = stats.cpu_data().next() {
                            cells.push(Cell::new(format!("{:.1}%", cpu)));
                        } else {
                            cells.push(Cell::new("-".to_string()));
                        }
                        if let Some(mem) = stats.mem_data().next() {
                            let format = FormatSizeOptions::from(BINARY).decimal_places(1);
                            cells.push(Cell::new(format_size(*mem, format)));
                        } else {
                            cells.push(Cell::new("-".to_string()));
                        }
                    } else {
                        cells.push(Cell::new("-".to_string()));
                        cells.push(Cell::new("-".to_string()));
                    }
                    Row::new(cells)
                })
                .collect(),
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

    fn cancel(&mut self) -> Result<()> {
        self.cancellation_token.cancel();
        self.task.abort();
        Ok(())
    }

    pub(crate) fn teardown(&mut self, _t: &mut tui::Tui) -> Result<()> {
        self.cancel()?;
        Ok(())
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        Some(&[
            ("Enter", "Container view"),
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
            KeyCode::Enter => Some(Action::Ok),
            _ => None,
        }
    }

    pub(crate) fn has_filter(&self) -> bool {
        true
    }
}
