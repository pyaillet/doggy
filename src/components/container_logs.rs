use std::sync::Arc;

use bollard::container::LogsOptions;
use chrono::{Duration, Utc};
use color_eyre::Result;

use crossterm::event::{self, KeyCode};
use futures::StreamExt;

use futures::executor::block_on;
use ratatui::layout::{Constraint, Layout};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tokio::{select, spawn};

use ratatui::{
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, ScrollbarState},
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::components::{containers::Containers, Component};
use crate::{action::Action, runtime::get_container_logs};

#[derive(Clone, Debug)]
pub struct ContainerLogs {
    id: String,
    name: String,
    logs: Arc<Mutex<Vec<String>>>,
    task: Arc<JoinHandle<Result<()>>>,
    cancellation_token: CancellationToken,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    action_tx: Option<UnboundedSender<Action>>,
    follow: bool,
    auto_scroll: bool,
    since: i64,
}

async fn run_setup_task(
    cid: String,
    follow: bool,
    since: i64,
    logs: Arc<Mutex<Vec<String>>>,
    cancel: CancellationToken,
) -> Result<()> {
    let mut should_stop = false;
    let since = (Utc::now() - Duration::minutes(since)).timestamp();
    let options = LogsOptions {
        stdout: true,
        stderr: false,
        since,
        follow,
        ..Default::default()
    };
    let mut stream = get_container_logs(&cid, options).await?;
    while !should_stop {
        select!(
        l = stream.next() => {
            if let Some(Ok(log)) = l {
                let mut w_logs = logs.lock().await;
                w_logs.push(log.to_string());
            }
        }
        _ = cancel.cancelled() => {
            should_stop = true;
        }
        );
    }
    Ok(())
}

impl ContainerLogs {
    pub fn new(id: String, name: String) -> Self {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let cancel = CancellationToken::new();
        let _cancel = cancel.clone();

        let _logs = Arc::clone(&logs);

        let follow = true;

        let since = 15;

        let task = Arc::new(spawn(run_setup_task(
            id.clone(),
            follow,
            since,
            _logs,
            _cancel,
        )));

        ContainerLogs {
            id,
            name,
            logs,
            task: Arc::clone(&task),
            cancellation_token: cancel,
            vertical_scroll_state: Default::default(),
            vertical_scroll: 0,
            action_tx: None,
            follow: true,
            auto_scroll: true,
            since,
        }
    }

    fn down(&mut self, qty: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(qty);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    fn up(&mut self, qty: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(qty);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    fn cancel(&mut self) -> Result<()> {
        self.cancellation_token.cancel();
        self.task.abort();
        Ok(())
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "ContainerLogs"
    }

    pub(crate) fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        let tx = self.action_tx.clone().expect("No action sender");
        match action {
            Action::PreviousScreen => {
                self.cancel()?;
                tx.send(Action::Screen(Component::Containers(Containers::new(None))))?;
            }
            Action::Up => {
                self.auto_scroll = false;
                self.up(1);
            }
            Action::Down => {
                self.auto_scroll = false;
                self.down(1);
            }
            Action::PageUp => {
                self.auto_scroll = false;
                self.up(15);
            }
            Action::PageDown => {
                self.auto_scroll = false;
                self.down(15);
            }
            Action::Since(n) => {
                log::debug!("****** Since {}", n);
                self.cancel()?;
                self.logs.lock().await.clear();

                let cancel = CancellationToken::new();
                let _cancel = cancel.clone();

                let _logs = Arc::clone(&self.logs);

                let task = Arc::new(spawn(run_setup_task(
                    self.id.clone(),
                    self.follow,
                    n.into(),
                    _logs,
                    _cancel,
                )));

                self.task = Arc::clone(&task);
                self.cancellation_token = cancel;
                self.since = n as i64;
            }
            Action::AutoScroll => {
                self.auto_scroll = !self.auto_scroll;
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn draw(
        &mut self,
        f: &mut ratatui::prelude::Frame<'_>,
        area: ratatui::prelude::Rect,
    ) {
        let rects = Layout::default()
            .constraints([Constraint::Max(1), Constraint::Min(20)])
            .split(area);

        let logs = block_on(self.logs.lock());
        let first_line = Paragraph::new(Line::from(vec![
            Span::from("Autoscroll: "),
            Span::styled(
                if self.auto_scroll { "On" } else { "Off" },
                Style::new().bold(),
            ),
            Span::from(" - Since: "),
            Span::styled(format!("{}m", self.since), Style::new().bold()),
        ]))
        .block(Block::default().borders(Borders::NONE).gray());
        let mut log_paragraph = Paragraph::new(
            logs.iter()
                .map(|l| Line::from(Span::from(l)))
                .collect::<Vec<Line>>(),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .gray()
                .title(Span::styled(
                    format!(
                    "Container logs for: \"{}/{}\" (press 'ESC' to previous screen, 'q' to quit)",
                    &self.id[0..12],
                    self.name
                ),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
        );
        if self.auto_scroll {
            let lines = area.height - 2;
            self.vertical_scroll = logs.len().saturating_sub(lines.into());
        }
        log_paragraph = log_paragraph.scroll((self.vertical_scroll as u16, 0));

        f.render_widget(first_line, rects[0]);
        f.render_widget(log_paragraph, rects[1]);
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        Some(&[
            ("s", "Autoscroll"),
            ("1", "Since 1m"),
            ("2", "Since 3m"),
            ("3", "Since 5m"),
            ("4", "Since 10m"),
            ("5", "Since 15m"),
        ])
    }

    pub(crate) fn get_action(&self, k: &event::KeyEvent) -> Option<Action> {
        match k.code {
            KeyCode::Char('s') => Some(Action::AutoScroll),
            KeyCode::Char('1') => Some(Action::Since(1)),
            KeyCode::Char('2') => Some(Action::Since(3)),
            KeyCode::Char('3') => Some(Action::Since(5)),
            KeyCode::Char('4') => Some(Action::Since(10)),
            KeyCode::Char('5') => Some(Action::Since(15)),
            _ => None,
        }
    }
}
