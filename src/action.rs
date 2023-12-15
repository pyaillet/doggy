use std::fmt::Display;

use crate::components::Component;

pub enum Action {
    Refresh,
    Down,
    Up,
    Right,
    Left,
    Quit,
    All,
    Inspect,
    Screen(Box<dyn Component>),
    PreviousScreen,
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Refresh => f.write_str("Refresh"),
            Action::Up => f.write_str("Up"),
            Action::Down => f.write_str("Down"),
            Action::Left => f.write_str("Left"),
            Action::Right => f.write_str("Right"),
            Action::Quit => f.write_str("Quit"),
            Action::All => f.write_str("All"),
            Action::Inspect => f.write_str("Inspect"),
            Action::Screen(c) => f.write_fmt(format_args!("Screen({})", c.get_name())),
            Action::PreviousScreen => f.write_str("PreviousScreen"),
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
