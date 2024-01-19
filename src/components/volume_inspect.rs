use color_eyre::Result;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, ScrollbarState},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    components::{volumes::Volumes, Component},
};

#[derive(Clone, Debug)]
pub struct VolumeInspect {
    id: String,
    details: String,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
    action_tx: Option<UnboundedSender<Action>>,
}

impl VolumeInspect {
    pub fn new(id: String, details: String) -> Self {
        VolumeInspect {
            id,
            details,
            vertical_scroll_state: Default::default(),
            vertical_scroll: 0,
            action_tx: None,
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

    pub(crate) fn get_name(&self) -> &'static str {
        "VolumeInspect"
    }

    pub(crate) fn register_action_handler(&mut self, tx: UnboundedSender<Action>) {
        self.action_tx = Some(tx);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        match action {
            Action::PreviousScreen => {
                if let Some(tx) = self.action_tx.clone() {
                    tx.send(Action::Screen(Component::Volumes(Volumes::new(
                        Default::default(),
                    ))))?;
                }
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
        };
        Ok(())
    }

    pub(crate) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let volume_details = Paragraph::new(self.details.clone())
            .gray()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .gray()
                    .title(Span::styled(
                        format!(
                        "Inspecting volume: \"{}\" (press 'ESC' to previous screen, 'q' to quit)",
                        &self.id[0..12]
                    ),
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
            )
            .scroll((self.vertical_scroll as u16, 0));

        f.render_widget(volume_details, area);
    }
}
