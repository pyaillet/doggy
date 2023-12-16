use ratatui::{layout::Rect, Frame};

use color_eyre::Result;

use crate::action::Action;

pub mod container_inspect;
pub mod containers;
pub mod images;

pub(crate) trait Component {
    fn get_name(&self) -> &'static str;

    fn update(&mut self, action: Option<Action>) -> Result<Option<Action>>;

    /// Render the component on the screen. (REQUIRED)
    ///
    /// # Arguments
    ///
    /// * `f` - A frame used for rendering.
    /// * `area` - The area in which the component should be drawn.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect);
}
