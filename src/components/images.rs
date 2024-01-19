use color_eyre::Result;

use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, TableState, Wrap};
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::runtime::{delete_image, get_image, list_images, Filter, ImageSummary};

use crate::components::{containers::Containers, image_inspect::ImageInspect, Component};
use crate::utils::{centered_rect, table};

const IMAGE_CONSTRAINTS: [Constraint; 4] = [
    Constraint::Max(15),
    Constraint::Min(35),
    Constraint::Max(10),
    Constraint::Max(20),
];

#[derive(Clone, Debug)]
enum Popup {
    None,
    Delete(String, String),
}

#[derive(Clone, Debug)]
pub struct Images {
    state: TableState,
    images: Vec<ImageSummary>,
    show_popup: Popup,
    action_tx: Option<UnboundedSender<Action>>,
    sort_by: SortColumn,
    filter: Option<String>,
}

#[derive(Clone, Debug)]
pub enum SortOrder {
    Asc,
    Desc,
}

#[derive(Clone, Debug)]
pub enum SortColumn {
    Id(SortOrder),
    Name(SortOrder),
    Size(SortOrder),
    Age(SortOrder),
}

impl Images {
    pub fn new() -> Self {
        Images {
            state: Default::default(),
            images: Vec::new(),
            show_popup: Popup::None,
            action_tx: None,
            sort_by: SortColumn::Age(SortOrder::Asc),
            filter: None,
        }
    }

    fn previous(&mut self) {
        if !self.images.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.images.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    fn next(&mut self) {
        if !self.images.is_empty() {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.images.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
    }

    fn get_selected_image_info(&self) -> Option<(String, String)> {
        self.state
            .selected()
            .and_then(|i| self.images.get(i).cloned())
            .map(|c| (c.id, c.name))
    }

    fn draw_popup(&self, f: &mut Frame<'_>) {
        if let Popup::Delete(_id, tag) = &self.show_popup {
            let text = vec![
                Line::from(vec![
                    Span::raw("Are you sure you want to delete image: \""),
                    Span::styled(tag, Style::new().gray()),
                    Span::raw("\"?"),
                ]),
                Line::from(""),
                Line::from(vec![
                    "ESC".bold(),
                    " to Cancel, ".into(),
                    "Enter".bold(),
                    " to Confirm".into(),
                ]),
            ];
            let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });

            let block = Block::default()
                .title("Confirmation".bold())
                .padding(Padding::new(1, 1, 1, 1))
                .borders(Borders::ALL);
            let area = centered_rect(50, 8, f.size());
            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(paragraph.block(block), area);
        }
    }

    fn sort(&mut self) {
        self.images.sort_by(|a, b| {
            let (cmp_result, o) = match &self.sort_by {
                SortColumn::Id(o) => (a.id.cmp(&b.id), o),
                SortColumn::Name(o) => (a.name.cmp(&b.name), o),
                SortColumn::Size(o) => (a.size.cmp(&b.size), o),
                SortColumn::Age(o) => (a.created.cmp(&b.created), o),
            };
            match o {
                SortOrder::Asc => cmp_result,
                SortOrder::Desc => cmp_result.reverse(),
            }
        });
    }

    pub(crate) fn get_name(&self) -> &'static str {
        "Images"
    }

    pub(crate) fn register_action_handler(&mut self, action_tx: UnboundedSender<Action>) {
        self.action_tx = Some(action_tx);
    }

    pub(crate) async fn update(&mut self, action: Action) -> Result<()> {
        let tx = self.action_tx.clone().expect("No action sender available");
        match action {
            Action::Tick => {
                self.images = list_images(&self.filter).await?;
                self.sort();
                if self.state.selected().is_none() {
                    self.state.select(Some(0));
                }
            }
            Action::Down => {
                self.next();
            }
            Action::Up => {
                self.previous();
            }
            Action::Inspect => {
                if let Some(info) = self.get_selected_image_info() {
                    let id = info.0.to_string();
                    let name = info.1.to_string();
                    let action = match get_image(&id).await {
                        Ok(details) => Action::Screen(Component::ImageInspect(ImageInspect::new(
                            id, name, details,
                        ))),
                        Err(e) => Action::Error(format!(
                            "Unable to get image \"{}\" details:\n{}",
                            name, e
                        )),
                    };
                    tx.send(action)?;
                };
            }
            Action::SetFilter(filter) => {
                self.filter = filter;
            }
            Action::Delete => {
                if let Some((id, tag)) = self.get_selected_image_info() {
                    self.show_popup = Popup::Delete(id, tag);
                }
            }
            Action::Ok => {
                if let Popup::Delete(id, _) = &self.show_popup.clone() {
                    if let Err(e) = delete_image(id).await {
                        tx.send(Action::Error(format!(
                            "Unable to delete container \"{}\" {}",
                            id, e
                        )))?;
                    } else {
                        self.show_popup = Popup::None;
                        tx.send(Action::Tick)?;
                    }
                };
            }
            Action::PreviousScreen => {
                self.show_popup = Popup::None;
            }
            Action::SortColumn(n) => {
                self.sort_by = match (n, &self.sort_by) {
                    (1, SortColumn::Id(SortOrder::Asc)) => SortColumn::Id(SortOrder::Desc),
                    (1, _) => SortColumn::Id(SortOrder::Asc),
                    (2, SortColumn::Name(SortOrder::Asc)) => SortColumn::Name(SortOrder::Desc),
                    (2, _) => SortColumn::Name(SortOrder::Asc),
                    (3, SortColumn::Size(SortOrder::Asc)) => SortColumn::Size(SortOrder::Desc),
                    (3, _) => SortColumn::Size(SortOrder::Asc),
                    (4, SortColumn::Age(SortOrder::Asc)) => SortColumn::Age(SortOrder::Desc),
                    (4, _) => SortColumn::Age(SortOrder::Asc),
                    _ => self.sort_by.clone(),
                }
            }
            _ => {}
        };
        Ok(())
    }

    pub(crate) fn draw(&mut self, f: &mut Frame<'_>, area: Rect) {
        let rects = Layout::default()
            .constraints([Constraint::Percentage(100)])
            .split(area);
        let t = table(
            format!(
                "{}{}",
                self.get_name(),
                match &self.filter {
                    Some(f) => format!(" - Filter: {}", f),
                    None => "".to_string(),
                }
            ),
            ["Id", "Name", "Size", "Age"],
            self.images.iter().map(|i| i.into()).collect(),
            &IMAGE_CONSTRAINTS,
            Some(Style::new().gray()),
        );
        f.render_stateful_widget(t, rects[0], &mut self.state);

        self.draw_popup(f);
    }

    pub(crate) fn get_bindings(&self) -> Option<&[(&str, &str)]> {
        Some(&[
            ("ctrl+d", "Delete"),
            ("i", "Inspect/View details"),
            ("c", "Show containers"),
            ("F1", "Sort by image id"),
            ("F2", "Sort by image name"),
            ("F3", "Sort by image size"),
            ("F4", "Sort by image age"),
        ])
    }

    pub(crate) fn get_action(&self, k: &crossterm::event::KeyEvent) -> Option<Action> {
        if let KeyCode::Char('c') = k.code {
            if let Some((id, _)) = self.get_selected_image_info() {
                Some(Action::Screen(Component::Containers(Containers::new(
                    Filter::default().filter("ancestor".to_string(), id),
                ))))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(crate) fn has_filter(&self) -> bool {
        true
    }
}
