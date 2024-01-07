use chrono::{DateTime, Utc};

use humansize::{FormatSizeI, BINARY};

use ratatui::{
    style::{Style, Stylize},
    text::Span,
    widgets::Row,
};
#[derive(Clone, Debug)]
pub struct VolumeSummary {
    pub id: String,
    pub driver: String,
    pub size: i64,
    pub created: String,
}

impl<'a> From<&VolumeSummary> for Row<'a> {
    fn from(value: &VolumeSummary) -> Row<'a> {
        let VolumeSummary {
            id,
            driver,
            size,
            created,
        } = value.clone();
        Row::new(vec![
            id.gray(),
            driver.gray(),
            size.format_size_i(BINARY).gray(),
            created.gray(),
        ])
    }
}

#[derive(Clone, Debug)]
pub struct NetworkSummary {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub created: String,
}

impl<'a> From<&NetworkSummary> for Row<'a> {
    fn from(value: &NetworkSummary) -> Row<'a> {
        let NetworkSummary {
            id,
            name,
            driver,
            created,
        } = value.clone();
        Row::new(vec![id.gray(), name.gray(), driver.gray(), created.gray()])
    }
}

#[derive(Clone, Debug)]
pub struct ImageSummary {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub created: i64,
}

impl<'a> From<&ImageSummary> for Row<'a> {
    fn from(value: &ImageSummary) -> Row<'a> {
        let ImageSummary {
            id,
            name,
            size,
            created,
        } = value.clone();
        Row::new(vec![
            id.gray(),
            name.gray(),
            size.format_size_i(BINARY).gray(),
            DateTime::<Utc>::from_timestamp(created, 0)
                .expect("Unable to parse timestamp")
                .to_string()
                .gray(),
        ])
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ContainerStatus {
    Created,
    Running,
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
    Unknown,
}

impl<T> From<T> for ContainerStatus
where
    T: AsRef<str>,
{
    fn from(value: T) -> Self {
        match value.as_ref() {
            "created" => ContainerStatus::Created,
            "running" => ContainerStatus::Running,
            "paused" => ContainerStatus::Paused,
            "restarting" => ContainerStatus::Restarting,
            "removing" => ContainerStatus::Removing,
            "exited" => ContainerStatus::Exited,
            "dead" => ContainerStatus::Dead,
            _ => ContainerStatus::Unknown,
        }
    }
}

impl From<ContainerStatus> for String {
    fn from(value: ContainerStatus) -> Self {
        match value {
            ContainerStatus::Created => "created".into(),
            ContainerStatus::Running => "running".into(),
            ContainerStatus::Paused => "paused".into(),
            ContainerStatus::Restarting => "restarting".into(),
            ContainerStatus::Removing => "removing".into(),
            ContainerStatus::Exited => "exited".into(),
            ContainerStatus::Dead => "dead".into(),
            ContainerStatus::Unknown => "unknown".into(),
        }
    }
}

impl ContainerStatus {
    fn format(&self) -> Span<'static> {
        match self {
            ContainerStatus::Created => Span::styled("created", Style::new().dark_gray()),
            ContainerStatus::Running => Span::styled("running", Style::new().green()),
            ContainerStatus::Paused => Span::styled("paused", Style::new().dark_gray()),
            ContainerStatus::Restarting => Span::styled("restarting", Style::new().yellow()),
            ContainerStatus::Removing => Span::styled("removing", Style::new().red()),
            ContainerStatus::Exited => Span::styled("exited", Style::new().red()),
            ContainerStatus::Dead => Span::styled("dead", Style::new().red()),
            ContainerStatus::Unknown => "unknown".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_id: String,
    pub status: ContainerStatus,
    pub age: i64,
}

impl<'a> From<&ContainerSummary> for Row<'a> {
    fn from(value: &ContainerSummary) -> Row<'a> {
        let ContainerSummary {
            id,
            name,
            image,
            status,
            ..
        } = value.clone();
        Row::new(vec![id.gray(), name.gray(), image.gray(), status.format()])
    }
}
