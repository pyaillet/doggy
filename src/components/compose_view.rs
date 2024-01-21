use color_eyre::Result;

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, ScrollbarState},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{action::Action, runtime::Compose};

use super::{composes::Composes, Component};

#[derive(Clone, Debug)]
pub struct ComposeView {
    compose: Compose,
    action_tx: Option<UnboundedSender<Action>>,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl ComposeView {
    pub fn new(compose: Compose) -> Self {
        ComposeView {
            compose,
            action_tx: None,
            vertical_scroll_state: Default::default(),
            vertical_scroll: 0,
        }
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "ComposeView"
    }

    pub(crate) fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    fn down(&mut self, qty: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_add(qty);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    fn up(&mut self, qty: usize) {
        self.vertical_scroll = self.vertical_scroll.saturating_sub(qty);
        self.vertical_scroll_state = self.vertical_scroll_state.position(self.vertical_scroll);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        let tx = self.action_tx.clone().expect("No action sender");
        match action {
            Action::PreviousScreen => {
                tx.send(Action::Screen(Component::Composes(Composes::new())))?;
            }
            Action::Up => {
                self.up(1);
            }
            Action::Down => {
                self.down(1);
            }
            Action::PageUp => {
                self.up(15);
            }
            Action::PageDown => {
                self.down(15);
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
        let text: Vec<Line> = (&self.compose).into();
        let details = Paragraph::new(Text::from(text)).block(
            Block::default().borders(Borders::ALL).title(Span::styled(
                format!(
                    "Inspecting compose project: \"{}\" (press 'ESC' to previous screen, 'q' to quit)",
                    self.compose.project
                ),
                Style::default().add_modifier(Modifier::BOLD),
            )),
        )
        .scroll((self.vertical_scroll as u16, 0));

        f.render_widget(details, area);
    }
}
