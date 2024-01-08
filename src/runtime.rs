use std::{error::Error, fmt::Display};

use tokio::sync::Mutex;

use lazy_static::lazy_static;

use bollard::container::{LogOutput, LogsOptions};
use color_eyre::Result;

#[cfg(feature = "cri")]
pub mod cri;
#[cfg(feature = "docker")]
pub mod docker;
pub mod model;

use futures::Stream;
pub use model::*;

lazy_static! {
    static ref CLIENT: Mutex<Option<Connection>> = Mutex::new(None);
}

pub const CONTAINERS: &str = "containers";
pub const IMAGES: &str = "images";
pub const NETWORKS: &str = "networks";
pub const VOLUMES: &str = "volumes";

pub(crate) async fn get_suggestions() -> &'static [&'static str] {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(_) => &[CONTAINERS, IMAGES, NETWORKS, VOLUMES],
            #[cfg(feature = "cri")]
            Client::Cri(_) => &[CONTAINERS, IMAGES],
        },
        _ => unimplemented!(),
    }
}

#[derive(Clone, Debug)]
struct NotInitialized {}

impl Display for NotInitialized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Not initialized")
    }
}

impl Error for NotInitialized {}

#[derive(Clone, Debug)]
struct NoConfigurationFound {}

impl Display for NoConfigurationFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("No configuration found for runtime")
    }
}

impl Error for NoConfigurationFound {}

pub enum ConnectionConfig {
    #[cfg(feature = "docker")]
    Docker(docker::ConnectionConfig),
    #[cfg(feature = "cri")]
    Cri(cri::ConnectionConfig),
}

#[allow(dead_code)]
struct Connection {
    config: ConnectionConfig,
    client: Client,
}

#[allow(dead_code)]
pub enum Client {
    #[cfg(feature = "docker")]
    Docker(docker::Client),
    #[cfg(feature = "cri")]
    Cri(cri::Client),
}

#[cfg(feature = "docker")]
async fn init_docker(config: docker::ConnectionConfig) -> Result<()> {
    let client = docker::connect(&config)?;

    let config = ConnectionConfig::Docker(config);

    let connection = Connection {
        config,
        client: Client::Docker(client),
    };

    let mut client = CLIENT.lock().await;
    *client = Some(connection);
    Ok(())
}

#[cfg(feature = "cri")]
async fn init_cri(config: cri::ConnectionConfig) -> Result<()> {
    let client = cri::connect(&config).await?;

    let config = ConnectionConfig::Cri(config);

    let connection = Connection {
        config,
        client: Client::Cri(client),
    };

    let mut client = CLIENT.lock().await;
    *client = Some(connection);
    Ok(())
}

pub async fn init(config: Option<ConnectionConfig>) -> Result<()> {
    #[cfg(feature = "cri")]
    {
        let config = config
            .or_else(|| docker::detect_connection_config().map(ConnectionConfig::Docker))
            .or_else(|| cri::detect_connection_config().map(ConnectionConfig::Cri));

        match config {
            Some(ConnectionConfig::Cri(c)) => init_cri(c).await,
            Some(ConnectionConfig::Docker(c)) => init_docker(c).await,
            None => Err(NoConfigurationFound {}.into()),
        }
    }

    #[cfg(not(feature = "cri"))]
    {
        let config =
            config.or_else(|| docker::detect_connection_config().map(ConnectionConfig::Docker));

        match config {
            Some(ConnectionConfig::Docker(c)) => init_docker(c).await,
            None => Err(NoConfigurationFound {}.into()),
        }
    }
}

pub(crate) async fn list_volumes() -> Result<Vec<VolumeSummary>> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.list_volumes().await,
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

#[allow(dead_code)]
pub(crate) async fn get_volume(id: &str) -> Result<String> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.get_volume(id).await,
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_volume(id: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.delete_volume(id).await,
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn list_networks(filter: &Option<String>) -> Result<Vec<NetworkSummary>> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.list_networks(filter).await,
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_network(id: &str) -> Result<String> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.get_network(id).await,
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_network(id: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.delete_network(id).await,
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn list_images(filter: &Option<String>) -> Result<Vec<ImageSummary>> {
    let mut client = CLIENT.lock().await;
    match *client {
        Some(ref mut conn) => match &mut conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.list_images(filter).await,
            #[cfg(feature = "cri")]
            Client::Cri(ref mut client) => client.list_images(filter).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_image(id: &str) -> Result<String> {
    let mut client = CLIENT.lock().await;
    match *client {
        Some(ref mut conn) => match &mut conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.get_image(id).await,
            #[cfg(feature = "cri")]
            Client::Cri(ref mut client) => client.get_image(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_image(id: &str) -> Result<()> {
    let mut client = CLIENT.lock().await;
    match *client {
        Some(ref mut conn) => match &mut conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.delete_image(id).await,
            #[cfg(feature = "cri")]
            Client::Cri(client) => client.delete_image(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_container(cid: &str) -> Result<()> {
    let mut client = CLIENT.lock().await;
    match *client {
        Some(ref mut conn) => match &mut conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.delete_container(cid).await,
            #[cfg(feature = "cri")]
            Client::Cri(client) => client.delete_container(cid).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn list_containers(
    all: bool,
    filter: &Option<String>,
) -> Result<Vec<ContainerSummary>> {
    let mut client = CLIENT.lock().await;
    match *client {
        Some(ref mut conn) => match &mut conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.list_containers(all, filter).await,
            #[cfg(feature = "cri")]
            Client::Cri(client) => client.list_containers(all, filter).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_container(cid: &str) -> Result<String> {
    let mut client = CLIENT.lock().await;
    match *client {
        Some(ref mut conn) => match &mut conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.get_container(cid).await,
            #[cfg(feature = "cri")]
            Client::Cri(client) => client.get_container(cid).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_container_logs(
    cid: &str,
    options: LogsOptions<String>,
) -> Result<impl Stream<Item = Result<LogOutput>>> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.get_container_logs(cid, options),
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn container_exec(cid: &str, cmd: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.container_exec(cid, cmd).await,
            #[cfg(feature = "cri")]
            _ => unimplemented!(),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_runtime_info() -> Result<RuntimeSummary> {
    let mut client = CLIENT.lock().await;
    match *client {
        Some(ref mut conn) => match &mut conn.client {
            #[cfg(feature = "docker")]
            Client::Docker(client) => client.info().await,
            #[cfg(feature = "cri")]
            Client::Cri(client) => client.info().await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}
