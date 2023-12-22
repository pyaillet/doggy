use std::fmt::Display;

use crate::components::ComponentInit;

#[derive(Clone, Debug)]
pub(crate) enum Action {
    Down,
    Up,
    Right,
    Left,
    PageUp,
    PageDown,
    Quit,
    All,
    Inspect,
    Shell,
    Delete,
    Screen(ComponentInit),
    Ok,
    PreviousScreen,
    Change,
    //TODO: Filter,
    Tick,
    Render,
    Error(String),
    Resize(u16, u16),
    Resume,
    Suspend,
    CustomShell,
}

/*
impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Refresh => f.write_str("Refresh"),
            Action::Up => f.write_str("Up"),
            Action::Down => f.write_str("Down"),
            Action::Left => f.write_str("Left"),
            Action::Right => f.write_str("Right"),
            Action::PageUp => f.write_str("PageUp"),
            Action::PageDown => f.write_str("PageDown"),
            Action::Quit => f.write_str("Quit"),
            Action::All => f.write_str("All"),
            Action::Inspect => f.write_str("Inspect"),
            Action::Shell => f.write_str("Shell"),
            Action::Delete => f.write_str("Delete"),
            Action::Screen(c) => f.write_fmt(format_args!("Screen({:?})", c)),
            Action::Ok => f.write_str("Ok"),
            Action::PreviousScreen => f.write_str("PreviousScreen"),
            Action::Change => f.write_str("Change"),
            Action::Filter => f.write_str("Filter"),
            Action::Tick => f.write_str("Tick"),
            Action::Render => f.write_str("Render"),
            Action::Error(e) => f.write_fmt(format_args!("Error({})", e)),
            Action::Resize(x, y) => f.write_fmt(format_args!("Resize({},{})", x, y)),
            Action::Resume => f.write_str("Resume"),
        }
    }
}
*/

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
