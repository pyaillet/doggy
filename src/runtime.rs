use std::{error::Error, fmt::Display};

use tokio::sync::Mutex;

use lazy_static::lazy_static;

use bollard::container::{LogOutput, LogsOptions};
use color_eyre::Result;

mod docker;
mod model;

use futures::Stream;
pub use model::*;

lazy_static! {
    static ref CLIENT: Mutex<Option<Connection>> = Mutex::new(None);
}

#[derive(Clone, Debug)]
struct NotInitialized {}

impl Display for NotInitialized {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Not initialized")
    }
}

impl Error for NotInitialized {}

pub enum ConnectionConfig {
    Docker(docker::ConnectionConfig),
}

#[allow(dead_code)]
struct Connection {
    config: ConnectionConfig,
    client: Client,
}

pub enum Client {
    Docker(docker::Client),
}

pub async fn init() -> Result<()> {
    let config = docker::detect_connection_config();
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

pub(crate) async fn list_volumes() -> Result<Vec<VolumeSummary>> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.list_volumes().await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

#[allow(dead_code)]
pub(crate) async fn get_volume(id: &str) -> Result<String> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.get_volume(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_volume(id: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.delete_volume(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn list_networks(filter: &Option<String>) -> Result<Vec<NetworkSummary>> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.list_networks(filter).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_network(id: &str) -> Result<String> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.get_network(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_network(id: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.delete_network(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn list_images(filter: &Option<String>) -> Result<Vec<ImageSummary>> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.list_images(filter).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_image(id: &str) -> Result<String> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.get_image(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_image(id: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.delete_image(id).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn delete_container(cid: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.delete_container(cid).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn list_containers(
    all: bool,
    filter: &Option<String>,
) -> Result<Vec<ContainerSummary>> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.list_containers(all, filter).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn get_container(cid: &str) -> Result<String> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.get_container(cid).await,
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
            Client::Docker(client) => client.get_container_logs(cid, options),
        },
        _ => Err(NotInitialized {}.into()),
    }
}

pub(crate) async fn container_exec(cid: &str, cmd: &str) -> Result<()> {
    let client = CLIENT.lock().await;
    match *client {
        Some(ref conn) => match &conn.client {
            Client::Docker(client) => client.container_exec(cid, cmd).await,
        },
        _ => Err(NotInitialized {}.into()),
    }
}
