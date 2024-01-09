use std::fmt::Display;

use crate::components::Component;

#[derive(Clone, Debug)]
pub(crate) enum Action {
    Down,
    Up,
    PageUp,
    PageDown,
    Quit,
    All,
    Inspect,
    Logs,
    Shell,
    Delete,
    Screen(Component),
    Ok,
    PreviousScreen,
    Change,
    Filter,
    SetFilter(Option<String>),
    Tick,
    Render,
    Error(String),
    Resize(u16, u16),
    Resume,
    Suspend,
    CustomShell,
    SortColumn(u8),
    Help,
    AutoScroll,
    Since(u16),
    LineWrap,
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}
