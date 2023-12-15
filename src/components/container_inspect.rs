use bollard::{container::InspectContainerOptions, Docker};
use color_eyre::Result;

use futures::executor::block_on;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style, Stylize},
    text::Span,
    widgets::{Block, Borders, Paragraph, ScrollbarState},
    Frame,
};

use crate::{
    action::Action,
    components::{containers::Containers, Component},
};

pub struct ContainerDetails {
    cid: String,
    details: String,
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl ContainerDetails {
    pub fn new(cid: String) -> Self {
        let details = block_on(async {
            let docker_cli =
                Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
            let container_details = docker_cli
                .inspect_container(&cid, Some(InspectContainerOptions { size: false }))
                .await
                .expect("Unable to get container description");
            serde_json::to_string_pretty(&container_details)
                .expect("Unable to serialize container_details")
        });

        ContainerDetails {
            cid,
            details,
            vertical_scroll_state: Default::default(),
            vertical_scroll: 0,
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
}

impl Component for ContainerDetails {
    fn get_name(&self) -> &'static str {
        "ContainerDetails"
    }

    fn update(
        &mut self,
        action: Option<crate::action::Action>,
    ) -> Result<Option<crate::action::Action>> {
        let action = match action {
            Some(Action::PreviousScreen) => Some(Action::Screen(Box::new(Containers::new()))),
            Some(Action::Up) => {
                self.up(1);
                None
            }
            Some(Action::Down) => {
                self.down(1);
                None
            }
            Some(Action::PageUp) => {
                self.up(15);
                None
            }
            Some(Action::PageDown) => {
                self.down(15);
                None
            }
            Some(action) => Some(action),
            _ => None,
        };
        Ok(action)
    }

    fn draw(&mut self, f: &mut Frame<'_>, _area: Rect) -> Result<()> {
        let container_details = Paragraph::new(self.details.clone())
            .gray()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .gray()
                    .title(Span::styled(
                        format!("Inspecting container: \"{}\" (press 'ESC' to previous screen, 'q' to quit)", self.cid),
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
            )
            .scroll((self.vertical_scroll as u16, 0));

        f.render_widget(container_details, f.size());
        Ok(())
    }
}
