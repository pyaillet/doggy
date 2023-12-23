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
    SortColumn(u8),
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
