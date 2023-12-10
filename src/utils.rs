use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Row, Table},
};

pub const NOT_FOUND: &str = "<NOT_FOUND>";

macro_rules! get_or_not_found {
    ($property:expr) => {
        $property
            .as_ref()
            .and_then(|s| Some(s.as_str()))
            .unwrap_or(crate::utils::NOT_FOUND)
            .to_string()
    };
    ($property:expr, $extractor:expr) => {
        $property
            .as_ref()
            .and_then($extractor)
            .and_then(|s| Some(s.as_str()))
            .unwrap_or(crate::utils::NOT_FOUND)
            .to_string()
    };
}
pub(crate) use get_or_not_found;

pub(crate) fn table<'a, const SIZE: usize>(
    headers: [&'a str; SIZE],
    items: Vec<[String; SIZE]>,
    constraints: &'static [Constraint; SIZE],
) -> Table<'a> {
    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default();
    let header_cells = headers
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().bold()));
    let header = ratatui::widgets::Row::new(header_cells)
        .style(normal_style)
        .height(1);
    let rows = items.iter().map(|c| {
        let cells = c
            .iter()
            .map(|c| Cell::from(c.to_string()).style(normal_style));
        Row::new(cells).style(normal_style).height(1)
    });
    Table::new(rows)
        .widths(constraints)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Containers"))
        .highlight_style(selected_style)
}
