use std::collections::HashMap;

use humansize::{FormatSizeI, BINARY};

use ratatui::{
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Row,
};

use crate::utils::Age;

use super::ConnectionConfig;

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct Filter {
    filter: HashMap<String, String>,
}

#[allow(dead_code)]
impl Filter {
    pub fn filter(mut self, key: String, value: String) -> Self {
        self.filter.insert(key, value);
        self
    }

    pub fn name(self, name: String) -> Self {
        self.filter("name".to_string(), name)
    }

    pub fn image(self, image: String) -> Self {
        self.filter("ancestor".to_string(), image)
    }

    pub fn compose(self) -> Self {
        self.filter(
            "labels".to_string(),
            "com.docker.compose.project".to_string(),
        )
    }

    pub fn compose_project(self, project: String) -> Self {
        self.filter(
            "labels".to_string(),
            format!("{}={}", "com.docker.compose.project", project).to_string(),
        )
    }
}

impl From<Filter> for HashMap<String, Vec<String>> {
    fn from(value: Filter) -> Self {
        value
            .filter
            .into_iter()
            .map(|(k, v)| (k, vec![v]))
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeSummary {
    pub name: String,
    pub version: String,
    pub config: Option<ConnectionConfig>,
}

#[derive(Clone, Debug)]
pub struct VolumeSummary {
    pub id: String,
    pub driver: String,
    pub created: i64,
}

impl<'a> From<&VolumeSummary> for Row<'a> {
    fn from(value: &VolumeSummary) -> Row<'a> {
        let VolumeSummary {
            id,
            driver,
            created,
        } = value.clone();
        Row::new(vec![id.gray(), driver.gray(), created.age().gray()])
    }
}

#[derive(Clone, Debug)]
pub struct NetworkSummary {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub created: i64,
}

impl<'a> From<&NetworkSummary> for Row<'a> {
    fn from(value: &NetworkSummary) -> Row<'a> {
        let NetworkSummary {
            id,
            name,
            driver,
            created,
        } = value.clone();
        Row::new(vec![
            id.gray(),
            name.gray(),
            driver.gray(),
            created.age().gray(),
        ])
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
            created.age().gray(),
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
            age,
            ..
        } = value.clone();
        Row::new(vec![
            id.gray(),
            name.gray(),
            image.gray(),
            status.format(),
            age.age().gray(),
        ])
    }
}

#[derive(Clone, Debug)]
pub struct ContainerDetails {
    pub id: String,
    pub name: String,
    pub image: Option<String>,
    pub image_id: Option<String>,
    pub status: ContainerStatus,
    pub age: Option<i64>,
    pub ports: Vec<(String, String)>,
    pub volumes: Vec<(String, String)>,
    pub env: Vec<(String, String)>,
    pub entrypoint: Option<Vec<String>>,
    pub command: Option<Vec<String>>,
    pub network: Vec<(String, Option<String>)>,
    pub processes: Vec<(String, String, String)>,
}

impl<'a> From<&ContainerDetails> for Vec<Line<'a>> {
    fn from(val: &ContainerDetails) -> Self {
        let style = Style::default().gray();
        let mut text: Vec<Line> = vec![
            Line::styled(format!("Id: {}", &val.id[0..12]).to_string(), style),
            Line::styled(format!("Name: {}", val.name).to_string(), style),
            Line::from(vec![Span::styled("Status: ", style), val.status.format()]),
        ];
        if let Some(age) = val.age {
            text.push(Line::styled(
                format!("Created: {}", age.age()).to_string(),
                style,
            ));
        }
        match (val.image.as_ref(), val.image_id.as_ref()) {
            (Some(image), Some(_image_id)) => {
                text.push(Line::styled(format!("Image: {}", image).to_string(), style))
            }
            (Some(image), None) => {
                text.push(Line::styled(format!("Image: {}", image).to_string(), style))
            }
            (None, Some(image_id)) => text.push(Line::styled(
                format!("Image: {}", image_id).to_string(),
                style,
            )),
            (None, None) => {}
        }
        if let Some(entrypoint) = &val.entrypoint {
            if !entrypoint.is_empty() {
                text.push(Line::styled("Entrypoint:", style));
                text.append(
                    &mut entrypoint
                        .iter()
                        .map(|entry| Line::styled(format!("  - {}", entry).to_string(), style))
                        .collect(),
                );
            }
        }
        if let Some(command) = &val.command {
            if !command.is_empty() {
                text.push(Line::styled("Command:".to_string(), style));
                text.append(
                    &mut command
                        .iter()
                        .map(|cmd| Line::styled(format!("  - {}", cmd).to_string(), style))
                        .collect(),
                );
            }
        }
        if !val.env.is_empty() {
            text.push(Line::styled("Environment:".to_string(), style));
            text.append(
                &mut val
                    .env
                    .iter()
                    .map(|(k, v)| Line::styled(format!("  {}: {}", k, v), style))
                    .collect(),
            );
        }
        if !val.volumes.is_empty() {
            text.push(Line::styled("Volumes:".to_string(), style));
            text.append(
                &mut val
                    .volumes
                    .iter()
                    .map(|(s, d)| Line::styled(format!("  - {}:{}", s, d), style))
                    .collect(),
            );
        }
        if !val.network.is_empty() {
            text.push(Line::styled("Networks:".to_string(), style));
            text.append(
                &mut val
                    .network
                    .iter()
                    .flat_map(|(n, ip)| match ip {
                        Some(ip) => vec![
                            Line::styled(format!("  - Name: {}", n), style),
                            Line::styled(format!("    IPAddress: {}", ip), style),
                        ],
                        None => vec![Line::styled(format!("  - Name: {}", n), style)],
                    })
                    .collect(),
            );
        }
        if !val.ports.is_empty() {
            text.push(Line::styled("Ports:".to_string(), style));
            text.append(
                &mut val
                    .ports
                    .iter()
                    .map(|(h, c)| Line::styled(format!("  - {}:{}", h, c), style))
                    .collect(),
            );
        }
        text
    }
}
