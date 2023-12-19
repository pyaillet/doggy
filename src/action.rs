use std::fmt::Display;

use crate::components::Component;

pub(crate) enum Action {
    Refresh,
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
    Screen(Box<dyn Component>, bool),
    Ok,
    PreviousScreen,
    Change,
    Filter,
}

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
            Action::Screen(c, b) => f.write_fmt(format_args!("Screen({}, {})", c.get_name(), b)),
            Action::Ok => f.write_str("Ok"),
            Action::PreviousScreen => f.write_str("PreviousScreen"),
            Action::Change => f.write_str("Change"),
            Action::Filter => f.write_str("Filter"),
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
