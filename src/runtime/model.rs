use std::{collections::HashMap, fmt::Display};

use bollard::service::ContainerStateStatusEnum;
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
            "label".to_string(),
            "com.docker.compose.project".to_string(),
        )
    }

    pub fn compose_project(self, project: String) -> Self {
        self.filter(
            "label".to_string(),
            format!("{}={}", "com.docker.compose.project", project).to_string(),
        )
    }

    pub fn format(&self) -> String {
        if self.filter.is_empty() {
            String::new()
        } else {
            format!(" - Filters: {}", self)
        }
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

impl From<Filter> for String {
    fn from(value: Filter) -> Self {
        value
            .filter
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join("&")
    }
}

impl Display for Filter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .filter
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join("&");
        f.write_str(&s)
    }
}

impl From<Option<String>> for Filter {
    fn from(value: Option<String>) -> Self {
        match value {
            None => Filter::default(),
            Some(s) => s.into(),
        }
    }
}

impl From<String> for Filter {
    fn from(value: String) -> Self {
        match value.split_once('=') {
            Some((k, "")) => Filter::default().name(k.to_string()),
            Some((k, v)) => Filter::default().filter(k.to_string(), v.to_string()),
            None => Filter::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeSummary {
    pub name: String,
    pub version: String,
    pub config: Option<ConnectionConfig>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VolumeSummary {
    pub id: String,
    pub driver: String,
    pub created: i64,
    pub labels: HashMap<String, String>,
}

impl<'a> From<&VolumeSummary> for Row<'a> {
    fn from(value: &VolumeSummary) -> Row<'a> {
        let VolumeSummary {
            id,
            driver,
            created,
            ..
        } = value.clone();
        Row::new(vec![id.gray(), driver.gray(), created.age().gray()])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkSummary {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub created: i64,
    pub labels: HashMap<String, String>,
}

impl<'a> From<&NetworkSummary> for Row<'a> {
    fn from(value: &NetworkSummary) -> Row<'a> {
        let NetworkSummary {
            id,
            name,
            driver,
            created,
            ..
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
pub enum ContainerHealth {
    Unknown,
    Healthy,
    Unhealthy,
    Starting,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ContainerStatus {
    Created,
    Running(ContainerHealth),
    Paused,
    Restarting,
    Removing,
    Exited,
    Dead,
    Unknown,
}

impl From<String> for ContainerStatus {
    fn from(value: String) -> Self {
        match value.as_ref() {
            "created" => ContainerStatus::Created,
            "running" => ContainerStatus::Running(ContainerHealth::Unknown),
            "paused" => ContainerStatus::Paused,
            "restarting" => ContainerStatus::Restarting,
            "removing" => ContainerStatus::Removing,
            "exited" => ContainerStatus::Exited,
            "dead" => ContainerStatus::Dead,
            _ => ContainerStatus::Unknown,
        }
    }
}

impl From<ContainerStateStatusEnum> for ContainerStatus {
    fn from(value: ContainerStateStatusEnum) -> Self {
        match value {
            ContainerStateStatusEnum::DEAD => ContainerStatus::Dead,
            ContainerStateStatusEnum::EMPTY => ContainerStatus::Unknown,
            ContainerStateStatusEnum::EXITED => ContainerStatus::Exited,
            ContainerStateStatusEnum::CREATED => ContainerStatus::Created,
            ContainerStateStatusEnum::PAUSED => ContainerStatus::Paused,
            ContainerStateStatusEnum::RUNNING => ContainerStatus::Running(ContainerHealth::Unknown),
            ContainerStateStatusEnum::REMOVING => ContainerStatus::Removing,
            ContainerStateStatusEnum::RESTARTING => ContainerStatus::Restarting,
        }
    }
}

impl From<ContainerStatus> for String {
    fn from(value: ContainerStatus) -> Self {
        match value {
            ContainerStatus::Created => "created".into(),
            ContainerStatus::Running(h) => match h {
                ContainerHealth::Unknown => "running".into(),
                ContainerHealth::Healthy => "running (healthy)".into(),
                ContainerHealth::Unhealthy => "running (unhealthy)".into(),
                ContainerHealth::Starting => "running (starting)".into(),
            },
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
            ContainerStatus::Running(h) => match h {
                ContainerHealth::Unknown => Span::styled("running", Style::new().green()),
                ContainerHealth::Healthy => Span::styled("running (healthy)", Style::new().green()),
                ContainerHealth::Unhealthy => {
                    Span::styled("running (unhealthy)", Style::new().yellow())
                }
                ContainerHealth::Starting => {
                    Span::styled("running (starting)", Style::new().green())
                }
            },
            ContainerStatus::Paused => Span::styled("paused", Style::new().dark_gray()),
            ContainerStatus::Restarting => Span::styled("restarting", Style::new().yellow()),
            ContainerStatus::Removing => Span::styled("removing", Style::new().red()),
            ContainerStatus::Exited => Span::styled("exited", Style::new().red()),
            ContainerStatus::Dead => Span::styled("dead", Style::new().red()),
            ContainerStatus::Unknown => "unknown".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_id: String,
    pub labels: HashMap<String, String>,
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

#[derive(Clone, Debug, PartialEq)]
pub struct ContainerDetails {
    pub id: String,
    pub name: String,
    pub image: Option<String>,
    pub image_id: Option<String>,
    pub labels: HashMap<String, String>,
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
                        None => vec![Line::styled(format!("  - Name: {}", n), style)],
                        Some(ip) if ip.is_empty() => {
                            vec![Line::styled(format!("  - Name: {}", n), style)]
                        }
                        Some(ip) => vec![
                            Line::styled(format!("  - Name: {}", n), style),
                            Line::styled(format!("    IPAddress: {}", ip), style),
                        ],
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

#[derive(Clone, Debug, PartialEq)]
pub struct Compose {
    pub project: String,
    pub config_file: Option<String>,
    pub working_dir: Option<String>,
    pub environment_files: Option<String>,
    pub services: HashMap<(String, String), ContainerDetails>,
    pub volumes: HashMap<String, VolumeSummary>,
    pub networks: HashMap<String, NetworkSummary>,
}

impl Compose {
    pub fn new(
        project: String,
        config_file: Option<String>,
        working_dir: Option<String>,
        environment_files: Option<String>,
    ) -> Self {
        Compose {
            project,
            config_file,
            working_dir,
            environment_files,
            services: HashMap::new(),
            volumes: HashMap::new(),
            networks: HashMap::new(),
        }
    }
}

impl<'a> From<&Compose> for Row<'a> {
    fn from(value: &Compose) -> Row<'a> {
        Row::new(vec![
            value.project.to_string(),
            value.services.len().to_string(),
            value.volumes.len().to_string(),
            value.networks.len().to_string(),
        ])
    }
}
