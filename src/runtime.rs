use bollard::{
    container::{
        InspectContainerOptions, ListContainersOptions, LogOutput, LogsOptions,
        RemoveContainerOptions,
    },
    image::{ListImagesOptions, RemoveImageOptions},
    network::{InspectNetworkOptions, ListNetworksOptions},
    service::{ImageSummary, Network, Volume},
    volume::{ListVolumesOptions, RemoveVolumeOptions},
    Docker,
};
use chrono::{DateTime, Utc};
use color_eyre::Result;
use futures::{Stream, StreamExt};
use humansize::{FormatSizeI, BINARY};

use tracing::instrument;

use crate::utils::get_or_not_found;

pub(crate) async fn list_volumes() -> Result<Vec<[String; 4]>> {
    let options: ListVolumesOptions<String> = Default::default();
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let result = docker_cli.list_volumes(Some(options)).await?;
    let volumes = result
        .volumes
        .unwrap_or(vec![])
        .iter()
        .map(|v: &Volume| {
            [
                v.name.to_owned(),
                v.driver.to_owned(),
                v.usage_data
                    .to_owned()
                    .map(|usage| usage.size)
                    .map(|s| s.format_size_i(BINARY))
                    .unwrap_or("<Unknown>".to_owned()),
                v.created_at.to_owned().unwrap_or("<Unknown>".to_string()),
            ]
        })
        .collect();
    Ok(volumes)
}

#[allow(dead_code)]
pub(crate) async fn get_volume(id: &str) -> Result<String> {
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let volume = docker_cli.inspect_volume(id).await?;
    Ok(serde_json::to_string_pretty(&volume)?)
}

pub(crate) async fn delete_volume(id: &str) -> Result<()> {
    let options = RemoveVolumeOptions { force: true };
    let docker_cli = Docker::connect_with_socket_defaults()?;
    docker_cli.remove_volume(id, Some(options)).await?;
    Ok(())
}

pub(crate) async fn list_networks() -> Result<Vec<[String; 4]>> {
    let options: ListNetworksOptions<String> = Default::default();
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let networks = docker_cli.list_networks(Some(options)).await?;
    let networks = networks
        .iter()
        .map(|n: &Network| {
            [
                n.id.to_owned().unwrap_or("<Unknown>".to_owned()),
                n.name.to_owned().unwrap_or("<Unknown>".to_owned()),
                n.driver.to_owned().unwrap_or("<Unknown>".to_owned()),
                n.created.to_owned().unwrap_or("<Unknown>".to_owned()),
            ]
        })
        .collect();
    Ok(networks)
}

#[allow(dead_code)]
pub(crate) async fn get_network(id: &str) -> Result<String> {
    let docker_cli = Docker::connect_with_socket_defaults()?;
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
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let _ = docker_cli.remove_network(id).await;
    Ok(())
}
pub(crate) async fn list_images() -> Result<Vec<[String; 4]>> {
    let options: ListImagesOptions<String> = Default::default();
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let images = docker_cli.list_images(Some(options)).await?;
    let images = images
        .iter()
        .map(|i: &ImageSummary| {
            [
                i.id.to_string().split(':').last().unwrap()[0..12].to_string(),
                get_or_not_found!(i.repo_tags.first()),
                i.size.format_size_i(BINARY),
                DateTime::<Utc>::from_timestamp(i.created, 0)
                    .expect("Unable to parse timestamp")
                    .to_string(),
            ]
        })
        .collect();
    Ok(images)
}

pub(crate) async fn get_image(id: &str) -> Result<String> {
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let image = docker_cli.inspect_image(id).await?;
    Ok(serde_json::to_string_pretty(&image)?)
}

pub(crate) async fn delete_image(id: &str) -> Result<()> {
    let options = RemoveImageOptions {
        force: true,
        ..Default::default()
    };
    let docker_cli = Docker::connect_with_socket_defaults()?;
    docker_cli.remove_image(id, Some(options), None).await?;
    Ok(())
}

#[instrument(name = "containers::delete_container")]
pub(crate) async fn delete_container(cid: &str) -> Result<()> {
    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    let docker_cli = Docker::connect_with_socket_defaults()?;
    docker_cli.remove_container(cid, Some(options)).await?;
    Ok(())
}

#[instrument(name = "containers::list_containers")]
pub(crate) async fn list_containers(all: bool) -> Result<Vec<[String; 4]>> {
    let options: ListContainersOptions<String> = ListContainersOptions {
        all,
        ..Default::default()
    };
    let docker_cli = Docker::connect_with_socket_defaults().expect("Unable to connect to docker");
    let containers = docker_cli
        .list_containers(Some(options))
        .await
        .expect("Unable to get container list");
    let containers = containers
        .iter()
        .map(|c| {
            [
                get_or_not_found!(c.id),
                get_or_not_found!(c.names, |c| c.first().and_then(|s| s.split('/').last())),
                get_or_not_found!(c.image, |i| i.split('@').next()),
                get_or_not_found!(c.state),
            ]
        })
        .collect();
    Ok(containers)
}

pub(crate) async fn get_container(cid: &str) -> Result<String> {
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let container_details = docker_cli
        .inspect_container(cid, Some(InspectContainerOptions { size: false }))
        .await?;
    Ok(serde_json::to_string_pretty(&container_details)?)
}

pub(crate) fn get_container_logs(
    cid: &str,
    options: LogsOptions<String>,
) -> Result<impl Stream<Item = Result<LogOutput>>> {
    let docker_cli = Docker::connect_with_socket_defaults()?;
    let stream = docker_cli.logs(cid, Some(options));
    Ok(stream.map(|item| match item {
        Err(e) => Err(color_eyre::Report::from(e)),
        Ok(other) => Ok(other),
    }))
}
