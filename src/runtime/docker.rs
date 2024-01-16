use std::{collections::HashMap, env, fmt::Display, fs, io::Write, path::PathBuf};

use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
        RemoveContainerOptions,
    },
    exec::{CreateExecOptions, ResizeExecOptions, StartExecResults},
    image::{ListImagesOptions, RemoveImageOptions},
    network::{InspectNetworkOptions, ListNetworksOptions},
    service::{Network, Volume},
    volume::{ListVolumesOptions, RemoveVolumeOptions},
    Docker,
};
use chrono::DateTime;
use color_eyre::Result;
use crossterm::{
    cursor::{self, MoveTo},
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use eyre::eyre;
use futures::{Stream, StreamExt};
use tokio::{
    io::{stdin, AsyncReadExt, AsyncWriteExt},
    select, spawn,
};
use tokio_util::sync::CancellationToken;

use crate::utils::get_or_not_found;

use super::{
    ContainerDetails, ContainerSummary, Filter, ImageSummary, NetworkSummary, VolumeSummary,
};

const DEFAULT_TIMEOUT: u64 = 120;
const DEFAULT_DOCKER_SOCKET_PATH: &str = "/var/run/docker.sock";

#[cfg(target_os = "macos")]
const DEFAULT_RANCHER_DESKTOP_SOCKET_PATH: &str = ".rd/docker.sock";
#[cfg(target_os = "macos")]
const DEFAULT_PODMAN_DESKTOP_SOCKET_PATH: &str =
    ".local/share/containers/podman/machine/podman.sock";
#[cfg(target_os = "macos")]
const DEFAULT_ORBSTACK_DESKTOP_SOCKET_PATH: &str = ".orbstack/run/docker.sock";

const AVAILABLE_CONTAINER_FILTERS: [&str; 14] = [
    "ancestor", "before", "expose", "exited", "health", "id", "is-task", "label", "name",
    "network", "publish", "since", "status", "volume",
];

#[derive(Clone, Debug)]
pub enum ConnectionConfig {
    Ssl(String, String),
    Http(String),
    Socket(Option<String>),
}

#[allow(dead_code)]
impl ConnectionConfig {
    pub fn default_socket() -> Self {
        ConnectionConfig::Socket(None)
    }

    pub fn socket(path: String) -> Self {
        ConnectionConfig::Socket(Some(path))
    }

    pub fn http(address: String) -> Self {
        ConnectionConfig::Http(address)
    }

    pub fn ssl(address: String, certs_path: String) -> Self {
        ConnectionConfig::Ssl(address, certs_path)
    }
}

impl Display for ConnectionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionConfig::Ssl(host, _certs_path) => f.write_str(host),
            ConnectionConfig::Http(host) => f.write_str(host),
            ConnectionConfig::Socket(Some(socket_path)) => {
                f.write_fmt(format_args!("unix://{}", socket_path))
            }
            ConnectionConfig::Socket(None) => {
                f.write_fmt(format_args!("unix://{}", DEFAULT_DOCKER_SOCKET_PATH))
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn test_other_default_socket(relative_path: &str) -> Result<ConnectionConfig> {
    use eyre::eyre;
    use std::path::Path;

    let home_dir = env!("HOME");
    let socket_path = Path::new(home_dir).join(relative_path);
    let socket_path = socket_path
        .into_os_string()
        .into_string()
        .map_err(|_| eyre!("Unable to convert path to string"))?;
    fs::metadata(&socket_path).map(|_| Ok(ConnectionConfig::Socket(Some(socket_path))))?
}

#[cfg(target_os = "macos")]
pub fn detect_connection_config() -> Option<ConnectionConfig> {
    let docker_host = env::var("DOCKER_HOST");
    let docker_cert = env::var("DOCKER_CERT_PATH");
    match (docker_host, docker_cert) {
        (Ok(host), Ok(certs)) => {
            log::debug!("Connect with ssl");
            Some(ConnectionConfig::Ssl(host, certs))
        }
        (Ok(host), Err(_)) => {
            log::debug!("Connect with http");
            Some(ConnectionConfig::Http(host))
        }
        _ => {
            log::debug!("Connect with socket");
            fs::metadata(DEFAULT_DOCKER_SOCKET_PATH)
                .map(|_| ConnectionConfig::Socket(Some(DEFAULT_DOCKER_SOCKET_PATH.to_string())))
                .or_else(|_| test_other_default_socket(DEFAULT_RANCHER_DESKTOP_SOCKET_PATH))
                .or_else(|_| test_other_default_socket(DEFAULT_PODMAN_DESKTOP_SOCKET_PATH))
                .or_else(|_| test_other_default_socket(DEFAULT_ORBSTACK_DESKTOP_SOCKET_PATH))
                .ok()
        }
    }
}

#[cfg(target_os = "linux")]
pub fn detect_connection_config() -> Option<ConnectionConfig> {
    let docker_host = env::var("DOCKER_HOST");
    let docker_cert = env::var("DOCKER_CERT_PATH");
    match (docker_host, docker_cert) {
        (Ok(host), Ok(certs)) => {
            log::debug!("Connect with ssl");
            Some(ConnectionConfig::Ssl(host, certs))
        }
        (Ok(host), Err(_)) => {
            log::debug!("Connect with http");
            Some(ConnectionConfig::Http(host))
        }
        _ => {
            log::debug!("Connect with socket");
            match fs::metadata(DEFAULT_DOCKER_SOCKET_PATH) {
                Ok(_) => Some(ConnectionConfig::default_socket()),
                Err(_) => None,
            }
        }
    }
}

pub struct Client {
    client: Docker,
}

impl Client {
    pub(crate) async fn list_volumes(&self) -> Result<Vec<VolumeSummary>> {
        let options: ListVolumesOptions<String> = Default::default();
        let result = self.client.list_volumes(Some(options)).await?;
        let volumes = result
            .volumes
            .unwrap_or_default()
            .iter()
            .map(|v: &Volume| VolumeSummary {
                id: v.name.to_owned(),
                driver: v.driver.to_owned(),
                created: v
                    .created_at
                    .as_ref()
                    .map(|d| {
                        DateTime::parse_from_rfc3339(d)
                            .unwrap_or_default()
                            .timestamp()
                    })
                    .unwrap_or_default(),
            })
            .collect();
        Ok(volumes)
    }

    #[allow(dead_code)]
    pub(crate) async fn get_volume(&self, id: &str) -> Result<String> {
        let volume = self.client.inspect_volume(id).await?;
        Ok(serde_json::to_string_pretty(&volume)?)
    }

    pub(crate) async fn delete_volume(&self, id: &str) -> Result<()> {
        let options = RemoveVolumeOptions { force: true };
        self.client.remove_volume(id, Some(options)).await?;
        Ok(())
    }

    pub(crate) async fn list_networks(
        &self,
        filter: &Option<String>,
    ) -> Result<Vec<NetworkSummary>> {
        let options: ListNetworksOptions<String> = Default::default();
        let networks = self.client.list_networks(Some(options)).await?;
        let networks = networks
            .iter()
            .map(|n: &Network| NetworkSummary {
                id: n.id.to_owned().unwrap_or("<Unknown>".to_string()),
                name: n.name.to_owned().unwrap_or("<Unknown>".to_string()),
                driver: n.driver.to_owned().unwrap_or("<Unknown>".to_string()),
                created: n
                    .created
                    .as_ref()
                    .map(|d| {
                        DateTime::parse_from_rfc3339(d)
                            .unwrap_or_default()
                            .timestamp()
                    })
                    .unwrap_or_default(),
            })
            .filter(|n| match filter {
                Some(f) => n.name.contains(f),
                None => true,
            })
            .collect();
        Ok(networks)
    }

    #[allow(dead_code)]
    pub(crate) async fn get_network(&self, id: &str) -> Result<String> {
        let network = self
            .client
            .inspect_network(
                id,
                Some(InspectNetworkOptions::<String> {
                    verbose: true,
                    ..Default::default()
                }),
            )
            .await?;
        Ok(serde_json::to_string_pretty(&network)?)
    }

    pub(crate) async fn delete_network(&self, id: &str) -> Result<()> {
        self.client.remove_network(id).await?;
        Ok(())
    }

    pub(crate) async fn list_images(&self, filter: &Option<String>) -> Result<Vec<ImageSummary>> {
        let options: ListImagesOptions<String> = Default::default();
        let images = self.client.list_images(Some(options)).await?;
        let images = images
            .iter()
            .map(|i: &bollard::service::ImageSummary| ImageSummary {
                id: i.id.split(':').last().unwrap_or("NOT_FOUND").to_string(),
                name: get_or_not_found!(i.repo_tags.first()),
                size: i.size,
                created: i.created,
            })
            .filter(|i| match filter {
                Some(f) => i.name.contains(f),
                None => true,
            })
            .collect();
        Ok(images)
    }

    pub(crate) async fn get_image(&self, id: &str) -> Result<String> {
        let image = self.client.inspect_image(id).await?;
        Ok(serde_json::to_string_pretty(&image)?)
    }

    pub(crate) async fn delete_image(&self, id: &str) -> Result<()> {
        let options = RemoveImageOptions {
            force: true,
            ..Default::default()
        };
        self.client.remove_image(id, Some(options), None).await?;
        Ok(())
    }

    pub(crate) async fn delete_container(&self, cid: &str) -> Result<()> {
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };
        self.client.remove_container(cid, Some(options)).await?;
        Ok(())
    }

    pub(crate) async fn list_containers(
        &self,
        all: bool,
        filter: &Option<String>,
    ) -> Result<Vec<ContainerSummary>> {
        let filters = container_filters(filter);
        let options: ListContainersOptions<String> = ListContainersOptions {
            all,
            filters,
            ..Default::default()
        };
        let containers = self.client.list_containers(Some(options)).await?;
        let containers = containers
            .iter()
            .map(|c| ContainerSummary {
                id: get_or_not_found!(c.id),
                name: get_or_not_found!(c.names, |c| c.first().and_then(|s| s.split('/').last())),
                image: get_or_not_found!(c.image, |i| i.split('@').next()),
                image_id: get_or_not_found!(c.image_id),
                status: c.state.clone().unwrap_or("unknown".into()).into(),
                age: c.created.unwrap_or_default(),
            })
            .collect();
        Ok(containers)
    }

    pub(crate) async fn get_container(&self, cid: &str) -> Result<String> {
        let container_details = self
            .client
            .inspect_container(cid, Some(InspectContainerOptions { size: false }))
            .await?;
        Ok(serde_json::to_string_pretty(&container_details)?)
    }

    pub(crate) async fn get_container_details(&self, cid: &str) -> Result<ContainerDetails> {
        let container_details = self
            .client
            .inspect_container(cid, Some(InspectContainerOptions { size: false }))
            .await?;
        let config = container_details
            .config
            .ok_or(eyre!("No container configuration"))?;
        let status = parse_state(container_details.state);
        let container_top = match status {
            super::ContainerStatus::Running => self
                .client
                .top_processes(cid, Some(bollard::container::TopOptions { ps_args: "aux" }))
                .await
                .ok(),
            _ => None,
        };
        Ok(ContainerDetails {
            id: cid.to_string(),
            name: parse_name(container_details.name),
            age: parse_created(container_details.created),
            image: config.image,
            image_id: container_details.image,
            entrypoint: config.entrypoint,
            command: config.cmd,
            status,
            env: parse_env(config.env),
            ports: parse_ports(config.exposed_ports),
            network: parse_networks(container_details.network_settings),
            volumes: parse_mounts(container_details.mounts),
            processes: parse_processes(container_top.and_then(|t| t.processes)),
        })
    }

    pub(crate) fn get_container_logs(
        &self,
        cid: &str,
        options: LogsOptions<String>,
    ) -> Result<impl Stream<Item = Result<LogOutput>>> {
        let stream = self.client.logs(cid, Some(options));
        Ok(stream.map(|item| match item {
            Err(e) => Err(color_eyre::Report::from(e)),
            Ok(other) => Ok(other),
        }))
    }

    pub(crate) async fn container_exec(&self, cid: &str, cmd: &str) -> Result<()> {
        let cancellation_token = CancellationToken::new();
        let _cancellation_token = cancellation_token.clone();
        let tty_size = crossterm::terminal::size()?;
        let mut stdout = std::io::stdout();

        let exec = self
            .client
            .create_exec(
                cid,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    attach_stdin: Some(true),
                    tty: Some(true),
                    cmd: Some(vec![cmd]),
                    ..Default::default()
                },
            )
            .await?
            .id;

        if let StartExecResults::Attached {
            mut output,
            mut input,
        } = self.client.start_exec(&exec, None).await?
        {
            // pipe stdin into the docker exec stream input
            let handle = spawn(async move {
                let mut buf: [u8; 1] = [0];
                let mut should_stop = false;
                let mut stdin = stdin();
                while !should_stop {
                    select!(
                        _ = _cancellation_token.cancelled() => { should_stop = true; },
                        _ = stdin.read(&mut buf) => { input.write(&buf).await.ok(); }
                    );
                }
            });

            stdout.execute(MoveTo(0, 0))?;
            stdout.execute(Clear(ClearType::All))?;
            stdout.execute(cursor::Show)?;

            self.client
                .resize_exec(
                    &exec,
                    ResizeExecOptions {
                        height: tty_size.1,
                        width: tty_size.0,
                    },
                )
                .await?;

            // pipe docker exec output into stdout
            while let Some(Ok(output)) = output.next().await {
                stdout.write_all(output.into_bytes().as_ref())?;
                stdout.flush()?;
                log::debug!("========================== FLUSH");
            }

            log::debug!("Closing terminal");
            cancellation_token.cancel();
            handle.await?;
        }
        Ok(())
    }

    pub(crate) async fn info(&self) -> Result<(String, String)> {
        let info = self.client.info().await?;
        let version = info.server_version.unwrap_or("Unknown".to_string());
        let name = "docker".to_string();
        Ok((name, version))
    }

    pub(crate) fn validate_container_filters(&self, filter: &str) -> bool {
        let mut split = filter.split('=');
        match (split.next(), split.next()) {
            (Some(s), Some(_)) => AVAILABLE_CONTAINER_FILTERS.contains(&s),
            (None, Some(_)) => false,
            (Some(_), None) | (None, None) => true,
        }
    }
}

fn parse_processes(processes: Option<Vec<Vec<String>>>) -> Vec<(String, String, String)> {
    processes
        .map(|ps| {
            ps.into_iter()
                .map(|p| {
                    (
                        p.first().cloned().unwrap_or_default(),
                        p.get(1).cloned().unwrap_or_default(),
                        p.get(10).cloned().unwrap_or_default(),
                    )
                })
                .collect::<Vec<(String, String, String)>>()
        })
        .unwrap_or_default()
}

fn parse_mounts(mounts: Option<Vec<bollard::service::MountPoint>>) -> Vec<(String, String)> {
    let mut mounts: Vec<(String, String)> = mounts
        .map(|m| {
            m.into_iter()
                .map(|m| {
                    (
                        m.source.unwrap_or_default(),
                        m.destination.unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    mounts.sort();
    mounts
}

fn parse_networks(
    network_settings: Option<bollard::service::NetworkSettings>,
) -> Vec<(String, Option<String>)> {
    let mut nets: Vec<(String, Option<String>)> = network_settings
        .and_then(|n| {
            n.networks
                .map(|n| n.into_iter().map(|(k, v)| (k, v.ip_address)).collect())
        })
        .unwrap_or_default();
    nets.sort();
    nets
}

fn parse_ports(exposed_ports: Option<HashMap<String, HashMap<(), ()>>>) -> Vec<(String, String)> {
    let mut ports: Vec<(String, String)> = exposed_ports
        .map(|ports| ports.keys().cloned().map(|p| (p, String::new())).collect())
        .unwrap_or_default();
    ports.sort();
    ports
}

fn parse_state(state: Option<bollard::service::ContainerState>) -> super::ContainerStatus {
    match state {
        Some(state) => state
            .status
            .map_or(super::ContainerStatus::Unknown, |s| s.into()),
        None => super::ContainerStatus::Unknown,
    }
}

fn parse_created(created: Option<String>) -> Option<i64> {
    created
        .as_ref()
        .and_then(|c| DateTime::parse_from_rfc3339(c).ok())
        .map(|d| d.timestamp())
}

fn parse_name(name: Option<String>) -> String {
    name.and_then(|s| s.split('/').last().map(String::from))
        .unwrap_or("<UNKNOWN>".to_string())
}

fn parse_env(env: Option<Vec<String>>) -> Vec<(String, String)> {
    let mut envs: Vec<(String, String)> = env
        .map(|env| {
            env.iter()
                .map(|env| {
                    let mut var = env.split('=');
                    (
                        var.next().unwrap_or("<NONE>").to_string(),
                        var.next().unwrap_or("").to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    envs.sort();
    envs
}

pub fn container_filters(filter: &Option<String>) -> HashMap<String, Vec<String>> {
    match filter {
        None => HashMap::new(),
        Some(s) => {
            let mut split = s.split('=');
            match (split.next(), split.next()) {
                (Some(k), Some(v)) => Filter::default()
                    .filter(k.to_string(), v.to_string())
                    .into(),
                (None, Some(_)) => Filter::default().into(),
                (Some(_), None) | (None, None) => Filter::default().name(s.to_string()).into(),
            }
        }
    }
}

pub(crate) fn connect(config: &ConnectionConfig) -> Result<Client> {
    let docker = match config {
        ConnectionConfig::Ssl(host, certs_path) => {
            let mut ca = PathBuf::from(certs_path);

            let mut key = ca.clone();
            key.push("key");
            key.set_extension("pem");
            let mut cert = ca.clone();
            cert.push("cert");
            cert.set_extension("pem");

            ca.push("ca");
            ca.set_extension("pem");

            Docker::connect_with_ssl(
                host,
                &key,
                &cert,
                &ca,
                DEFAULT_TIMEOUT,
                bollard::API_DEFAULT_VERSION,
            )?
        }
        ConnectionConfig::Http(host) => {
            Docker::connect_with_http(host, DEFAULT_TIMEOUT, bollard::API_DEFAULT_VERSION)?
        }
        ConnectionConfig::Socket(None) => Docker::connect_with_socket_defaults()?,
        ConnectionConfig::Socket(Some(path)) => {
            Docker::connect_with_socket(path, DEFAULT_TIMEOUT, bollard::API_DEFAULT_VERSION)?
        }
    };
    Ok(Client { client: docker })
}
