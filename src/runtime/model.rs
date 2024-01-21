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

fn volume_detail_to_lines<'a>(val: &VolumeSummary, indent: usize) -> Vec<Line<'a>> {
    vec![Line::from(format!(
        "{:indent$}Driver: {}",
        "",
        val.driver,
        indent = indent
    ))]
}

impl<'a> From<&VolumeSummary> for Vec<Line<'a>> {
    fn from(val: &VolumeSummary) -> Vec<Line<'a>> {
        volume_detail_to_lines(val, 2)
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

fn network_detail_to_lines<'a>(val: &NetworkSummary, indent: usize) -> Vec<Line<'a>> {
    vec![Line::from(format!(
        "{:indent$}Driver: {}",
        "",
        val.driver,
        indent = indent
    ))]
}

impl<'a> From<&NetworkSummary> for Vec<Line<'a>> {
    fn from(val: &NetworkSummary) -> Vec<Line<'a>> {
        network_detail_to_lines(val, 2)
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

fn details_to_lines<'a>(val: &ContainerDetails, indent: usize) -> Vec<Line<'a>> {
    let style = Style::default().gray();
    let mut text: Vec<Line> = vec![
        Line::styled(
            format!("{:indent$}Id: {}", "", &val.id[0..12], indent = indent).to_string(),
            style,
        ),
        Line::styled(
            format!("{:indent$}Name: {}", "", val.name, indent = indent).to_string(),
            style,
        ),
        Line::from(vec![
            Span::styled(format!("{:indent$}Status: ", "", indent = indent), style),
            val.status.format(),
        ]),
    ];
    if let Some(age) = val.age {
        text.push(Line::styled(
            format!("{:indent$}Created: {}", "", age.age(), indent = indent),
            style,
        ));
    }
    match (val.image.as_ref(), val.image_id.as_ref()) {
        (Some(image), Some(_image_id)) => text.push(Line::styled(
            format!("{:indent$}Image: {}", "", image, indent = indent),
            style,
        )),
        (Some(image), None) => text.push(Line::styled(
            format!("{:indent$}Image: {}", "", image, indent = indent),
            style,
        )),
        (None, Some(image_id)) => text.push(Line::styled(
            format!("{:indent$}Image: {}", "", image_id, indent = indent),
            style,
        )),
        (None, None) => {}
    }
    if let Some(entrypoint) = &val.entrypoint {
        if !entrypoint.is_empty() {
            text.push(Line::styled(
                format!("{:indent$}Entrypoint:", "", indent = indent),
                style,
            ));
            text.append(
                &mut entrypoint
                    .iter()
                    .map(|entry| {
                        Line::styled(
                            format!("{:indent$}  - {}", "", entry, indent = indent).to_string(),
                            style,
                        )
                    })
                    .collect(),
            );
        }
    }
    if let Some(command) = &val.command {
        if !command.is_empty() {
            text.push(Line::styled(
                format!("{:indent$}Command:", "", indent = indent),
                style,
            ));
            text.append(
                &mut command
                    .iter()
                    .map(|cmd| {
                        Line::styled(
                            format!("{:indent$}  - {}", "", cmd, indent = indent).to_string(),
                            style,
                        )
                    })
                    .collect(),
            );
        }
    }
    if !val.env.is_empty() {
        text.push(Line::styled(
            format!("{:indent$}Environment:", "", indent = indent),
            style,
        ));
        text.append(
            &mut val
                .env
                .iter()
                .map(|(k, v)| {
                    Line::styled(
                        format!("{:indent$}  {}: {}", "", k, v, indent = indent),
                        style,
                    )
                })
                .collect(),
        );
    }
    if !val.volumes.is_empty() {
        text.push(Line::styled(
            format!("{:indent$}Volumes:", "", indent = indent),
            style,
        ));
        text.append(
            &mut val
                .volumes
                .iter()
                .map(|(s, d)| {
                    Line::styled(
                        format!("{:indent$}  - {}:{}", "", s, d, indent = indent),
                        style,
                    )
                })
                .collect(),
        );
    }
    if !val.network.is_empty() {
        text.push(Line::styled(
            format!("{:indent$}Networks:", "", indent = indent),
            style,
        ));
        text.append(
            &mut val
                .network
                .iter()
                .flat_map(|(n, ip)| match ip {
                    None => vec![Line::styled(
                        format!("{:indent$}  - Name: {}", "", n, indent = indent),
                        style,
                    )],
                    Some(ip) if ip.is_empty() => {
                        vec![Line::styled(
                            format!("{:indent$}  - Name: {}", "", n, indent = indent),
                            style,
                        )]
                    }
                    Some(ip) => vec![
                        Line::styled(
                            format!("{:indent$}  - Name: {}", "", n, indent = indent),
                            style,
                        ),
                        Line::styled(
                            format!("{:indent$}    IPAddress: {}", "", ip, indent = indent),
                            style,
                        ),
                    ],
                })
                .collect(),
        );
    }
    if !val.ports.is_empty() {
        text.push(Line::styled(
            format!("{:indent$}Ports:", "", indent = indent),
            style,
        ));
        text.append(
            &mut val
                .ports
                .iter()
                .map(|(h, c)| {
                    Line::styled(
                        format!("{:indent$}  - {}:{}", "", h, c, indent = indent),
                        style,
                    )
                })
                .collect(),
        );
    }
    text
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
        details_to_lines(val, 0)
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

impl<'a> From<&Compose> for Vec<Line<'a>> {
    fn from(val: &Compose) -> Self {
        let mut text = vec![Line::from(format!("Compose project: {}", val.project))];
        if let Some(config_file) = &val.config_file {
            text.push(Line::from(format!("Config file: {}", config_file)));
        }
        if let Some(working_dir) = &val.working_dir {
            text.push(Line::from(format!("Working directory: {}", working_dir)));
        }
        if let Some(env_file) = &val.environment_files {
            text.push(Line::from(format!("Environment file: {}", env_file)));
        }
        if !val.services.is_empty() {
            text.push(Line::from("Services:".to_string()));
            let mut svc_text = val
                .services
                .iter()
                .flat_map(|((svc, num), c)| {
                    let mut svc_text = vec![Line::from(format!("  {} - {}", svc, num))];
                    let mut svc_content = details_to_lines(c, 4);
                    svc_text.append(&mut svc_content);
                    svc_text
                })
                .collect();
            text.append(&mut svc_text);
        }
        if !val.networks.is_empty() {
            text.push(Line::from("Networks:".to_string()));
            let mut net_text = val
                .networks
                .iter()
                .flat_map(|(name, net)| {
                    let mut net_text = vec![Line::from(format!("- Name: {}", name))];
                    let mut net_content = net.into();
                    net_text.append(&mut net_content);
                    net_text
                })
                .collect();
            text.append(&mut net_text);
        }
        if !val.volumes.is_empty() {
            text.push(Line::from("Volumes:".to_string()));
            let mut vol_text = val
                .volumes
                .iter()
                .flat_map(|(id, vol)| {
                    let mut vol_text = vec![Line::from(format!("- Id: {}", id))];
                    let mut vol_content = vol.into();
                    vol_text.append(&mut vol_content);
                    vol_text
                })
                .collect();
            text.append(&mut vol_text);
        }

        text
    }
}
