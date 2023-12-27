use std::env;

use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
        RemoveContainerOptions,
    },
    image::{ListImagesOptions, RemoveImageOptions},
    network::{InspectNetworkOptions, ListNetworksOptions},
    service::{Network, Volume},
    volume::{ListVolumesOptions, RemoveVolumeOptions},
    Docker,
};
use chrono::{DateTime, Utc};
use color_eyre::Result;
use futures::{Stream, StreamExt};
use humansize::{FormatSizeI, BINARY};

use tracing::instrument;

use crate::utils::get_or_not_found;

pub fn get_docker_connection() -> Result<Docker> {
    let docker_host = env::var("DOCKER_HOST");
    let docker_cert = env::var("DOCKER_CERT_PATH");
    match (docker_host, docker_cert) {
        (Ok(_host), Ok(_certs)) => {
            log::debug!("Connect with ssl");
            Ok(Docker::connect_with_ssl_defaults()?)
        }
        (Ok(_host), Err(_)) => {
            log::debug!("Connect with http");
            Ok(Docker::connect_with_http_defaults()?)
        }
        _ => {
            log::debug!("Connect with socket");
            Ok(Docker::connect_with_socket_defaults()?)
        }
    }
}

#[derive(Clone, Debug)]
pub struct VolumeSummary {
    pub id: String,
    pub driver: String,
    pub size: i64,
    pub created: String,
}

impl From<VolumeSummary> for [String; 4] {
    fn from(value: VolumeSummary) -> Self {
        let VolumeSummary {
            id,
            driver,
            size,
            created,
        } = value.clone();
        [id, driver, size.format_size_i(BINARY), created]
    }
}

pub(crate) async fn list_volumes() -> Result<Vec<VolumeSummary>> {
    let options: ListVolumesOptions<String> = Default::default();
    let docker_cli = get_docker_connection()?;
    let result = docker_cli.list_volumes(Some(options)).await?;
    let volumes = result
        .volumes
        .unwrap_or(vec![])
        .iter()
        .map(|v: &Volume| VolumeSummary {
            id: v.name.to_owned(),
            driver: v.driver.to_owned(),
            size: v
                .usage_data
                .to_owned()
                .map(|usage| usage.size)
                .unwrap_or_default(),
            created: v.created_at.to_owned().unwrap_or("<Unknown>".to_string()),
        })
        .collect();
    Ok(volumes)
}

#[allow(dead_code)]
pub(crate) async fn get_volume(id: &str) -> Result<String> {
    let docker_cli = get_docker_connection()?;
    let volume = docker_cli.inspect_volume(id).await?;
    Ok(serde_json::to_string_pretty(&volume)?)
}

pub(crate) async fn delete_volume(id: &str) -> Result<()> {
    let options = RemoveVolumeOptions { force: true };
    let docker_cli = get_docker_connection()?;
    docker_cli.remove_volume(id, Some(options)).await?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct NetworkSummary {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub created: String,
}

impl From<NetworkSummary> for [String; 4] {
    fn from(value: NetworkSummary) -> Self {
        let NetworkSummary {
            id,
            name,
            driver,
            created,
        } = value.clone();
        [id, name, driver, created]
    }
}

pub(crate) async fn list_networks(filter: &Option<String>) -> Result<Vec<NetworkSummary>> {
    let options: ListNetworksOptions<String> = Default::default();
    let docker_cli = get_docker_connection()?;
    let networks = docker_cli.list_networks(Some(options)).await?;
    let networks = networks
        .iter()
        .map(|n: &Network| NetworkSummary {
            id: n.id.to_owned().unwrap_or("<Unknown>".to_string()),
            name: n.name.to_owned().unwrap_or("<Unknown>".to_string()),
            driver: n.driver.to_owned().unwrap_or("<Unknown>".to_string()),
            created: n.created.to_owned().unwrap_or("<Unknown>".to_string()),
        })
        .filter(|n| match filter {
            Some(f) => n.name.contains(f),
            None => true,
        })
        .collect();
    Ok(networks)
}

#[allow(dead_code)]
pub(crate) async fn get_network(id: &str) -> Result<String> {
    let docker_cli = get_docker_connection()?;
    let network = docker_cli
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

pub(crate) async fn delete_network(id: &str) -> Result<()> {
    let docker_cli = get_docker_connection()?;
    let _ = docker_cli.remove_network(id).await;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ImageSummary {
    pub id: String,
    pub name: String,
    pub size: i64,
    pub created: i64,
}

impl From<ImageSummary> for [String; 4] {
    fn from(value: ImageSummary) -> Self {
        let ImageSummary {
            id,
            name,
            size,
            created,
        } = value.clone();
        [
            id,
            name,
            size.format_size_i(BINARY),
            DateTime::<Utc>::from_timestamp(created, 0)
                .expect("Unable to parse timestamp")
                .to_string(),
        ]
    }
}

pub(crate) async fn list_images(filter: &Option<String>) -> Result<Vec<ImageSummary>> {
    let options: ListImagesOptions<String> = Default::default();
    let docker_cli = get_docker_connection()?;
    let images = docker_cli.list_images(Some(options)).await?;
    let images = images
        .iter()
        .map(|i: &bollard::service::ImageSummary| ImageSummary {
            id: i
                .id
                .to_string()
                .split(':')
                .last()
                .unwrap_or("NOT_FOUND")
                .to_string(),
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

pub(crate) async fn get_image(id: &str) -> Result<String> {
    let docker_cli = get_docker_connection()?;
    let image = docker_cli.inspect_image(id).await?;
    Ok(serde_json::to_string_pretty(&image)?)
}

pub(crate) async fn delete_image(id: &str) -> Result<()> {
    let options = RemoveImageOptions {
        force: true,
        ..Default::default()
    };
    let docker_cli = get_docker_connection()?;
    docker_cli.remove_image(id, Some(options), None).await?;
    Ok(())
}

#[instrument(name = "containers::delete_container")]
pub(crate) async fn delete_container(cid: &str) -> Result<()> {
    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    let docker_cli = get_docker_connection()?;
    docker_cli.remove_container(cid, Some(options)).await?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct ContainerSummary {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_id: String,
    pub status: String,
    pub age: i64,
}

impl From<ContainerSummary> for [String; 4] {
    fn from(val: ContainerSummary) -> Self {
        let ContainerSummary {
            id,
            name,
            image,
            status,
            ..
        } = val.clone();
        [id, name, image, status]
    }
}

#[instrument(name = "containers::list_containers")]
pub(crate) async fn list_containers(
    all: bool,
    filter: &Option<String>,
) -> Result<Vec<ContainerSummary>> {
    let options: ListContainersOptions<String> = ListContainersOptions {
        all,
        ..Default::default()
    };
    let docker_cli = get_docker_connection()?;
    let containers = docker_cli
        .list_containers(Some(options))
        .await
        .expect("Unable to get container list");
    let containers = containers
        .iter()
        .map(|c| ContainerSummary {
            id: get_or_not_found!(c.id),
            name: get_or_not_found!(c.names, |c| c.first().and_then(|s| s.split('/').last())),
            image: get_or_not_found!(c.image, |i| i.split('@').next()),
            image_id: get_or_not_found!(c.image_id),
            status: get_or_not_found!(c.state),
            age: c.created.unwrap_or_default(),
        })
        .filter(|c| match filter {
            Some(f) => c.name.contains(f) || c.image.contains(f) || c.image_id.contains(f),
            None => true,
        })
        .collect();
    Ok(containers)
}

pub(crate) async fn get_container(cid: &str) -> Result<String> {
    let docker_cli = get_docker_connection()?;
    let container_details = docker_cli
        .inspect_container(cid, Some(InspectContainerOptions { size: false }))
        .await?;
    Ok(serde_json::to_string_pretty(&container_details)?)
}

pub(crate) fn get_container_logs(
    cid: &str,
    options: LogsOptions<String>,
) -> Result<impl Stream<Item = Result<LogOutput>>> {
    let docker_cli = get_docker_connection()?;
    let stream = docker_cli.logs(cid, Some(options));
    Ok(stream.map(|item| match item {
        Err(e) => Err(color_eyre::Report::from(e)),
        Ok(other) => Ok(other),
    }))
}
