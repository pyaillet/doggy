use std::{collections::HashMap, fmt::Display, fs};

use color_eyre::Result;

use k8s_cri::v1::{
    image_service_client::ImageServiceClient, runtime_service_client::RuntimeServiceClient,
    ContainerStatusRequest, ImageSpec, ImageStatusRequest, ListContainersRequest,
    ListImagesRequest, RemoveContainerRequest, RemoveImageRequest, VersionRequest,
};

use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

use super::{ContainerSummary, ImageSummary};

const DEFAULT_SOCKET_PATH: &str = "/run/containerd/containerd.sock";

enum ContainerState {
    Created = 0,
    Running = 1,
    Exited = 2,
    Unknown = 3,
}

impl From<i32> for ContainerState {
    fn from(value: i32) -> Self {
        match value {
            0 => ContainerState::Created,
            1 => ContainerState::Running,
            2 => ContainerState::Exited,
            _ => ContainerState::Unknown,
        }
    }
}

impl From<ContainerState> for super::ContainerStatus {
    fn from(value: ContainerState) -> Self {
        match value {
            ContainerState::Created => super::ContainerStatus::Created,
            ContainerState::Running => super::ContainerStatus::Running,
            ContainerState::Exited => super::ContainerStatus::Exited,
            _ => super::ContainerStatus::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ConnectionConfig {
    Socket(Option<String>),
}

impl ConnectionConfig {
    pub fn default_socket() -> Self {
        ConnectionConfig::Socket(None)
    }

    pub fn socket(path: String) -> Self {
        ConnectionConfig::Socket(Some(path))
    }
}

impl Display for ConnectionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionConfig::Socket(Some(socket_path)) => {
                f.write_fmt(format_args!("unix://{}", socket_path))
            }
            ConnectionConfig::Socket(None) => {
                f.write_fmt(format_args!("unix://{}", DEFAULT_SOCKET_PATH))
            }
        }
    }
}

pub struct Client {
    image_client: ImageServiceClient<Channel>,
    runtime_client: RuntimeServiceClient<Channel>,
}

pub fn detect_connection_config() -> Option<ConnectionConfig> {
    match fs::metadata(DEFAULT_SOCKET_PATH) {
        Ok(_) => Some(ConnectionConfig::default_socket()),
        Err(_) => None,
    }
}

pub(crate) async fn connect(config: &ConnectionConfig) -> Result<Client> {
    let socket_path = match config {
        ConnectionConfig::Socket(None) => DEFAULT_SOCKET_PATH.to_string(),
        ConnectionConfig::Socket(Some(path)) => path.to_string(),
    };

    let channel = Endpoint::try_from("http://[::]")
        .unwrap()
        .connect_with_connector(service_fn(move |_: Uri| {
            let socket_path = socket_path.to_string();
            UnixStream::connect(socket_path)
        }))
        .await
        .expect("Could not create client.");

    let runtime_client = RuntimeServiceClient::new(channel.clone());
    let image_client = ImageServiceClient::new(channel);

    Ok(Client {
        image_client,
        runtime_client,
    })
}

impl Client {
    pub(crate) async fn list_images(
        &mut self,
        _filter: &Option<String>,
    ) -> Result<Vec<ImageSummary>> {
        let request = tonic::Request::new(ListImagesRequest { filter: None });
        let response = self.image_client.list_images(request).await?;
        let images = response
            .get_ref()
            .images
            .iter()
            .map(|i| ImageSummary {
                id: i.id.split(':').nth(1).unwrap_or("<Unknown>").to_string(),
                name: i
                    .repo_tags
                    .first()
                    .cloned()
                    .unwrap_or("<Unknown>".to_string()),
                size: i.size as i64,
                created: 0,
            })
            .collect();
        Ok(images)
    }

    pub(crate) async fn get_image(&mut self, id: &str) -> Result<String> {
        let spec = ImageSpec {
            image: id.to_string(),
            annotations: HashMap::new(),
        };
        let request = tonic::Request::new(ImageStatusRequest {
            image: Some(spec),
            verbose: true,
        });
        let response = self.image_client.image_status(request).await?;
        let image_status = response.get_ref();
        Ok(format!("{:?}", image_status))
    }

    pub(crate) async fn delete_image(&mut self, id: &str) -> Result<()> {
        let spec = ImageSpec {
            image: id.to_string(),
            annotations: HashMap::new(),
        };
        let request = tonic::Request::new(RemoveImageRequest { image: Some(spec) });
        let _response = self.image_client.remove_image(request).await?;
        Ok(())
    }

    pub(crate) async fn delete_container(&mut self, cid: &str) -> Result<()> {
        let request = tonic::Request::new(RemoveContainerRequest {
            container_id: cid.to_string(),
        });
        let _response = self.runtime_client.remove_container(request).await?;
        Ok(())
    }

    pub(crate) async fn list_containers(
        &mut self,
        _all: bool,
        _filter: &Option<String>,
    ) -> Result<Vec<ContainerSummary>> {
        let request = tonic::Request::new(ListContainersRequest { filter: None });
        let response = self.runtime_client.list_containers(request).await?;
        let containers = response
            .get_ref()
            .containers
            .iter()
            .map(|c| {
                let state: ContainerState = c.state.into();
                ContainerSummary {
                    id: c.id.to_string(),
                    name: c
                        .metadata
                        .clone()
                        .map(|m| m.name)
                        .unwrap_or("<Unknown>".to_string()),
                    image: c
                        .image
                        .clone()
                        .map(|i| i.image)
                        .unwrap_or("<Unknown>".to_string()),
                    image_id: c.image_ref.to_string(),
                    age: c.created_at,
                    status: state.into(),
                }
            })
            .collect();
        Ok(containers)
    }

    pub(crate) async fn get_container(&mut self, cid: &str) -> Result<String> {
        let request = tonic::Request::new(ContainerStatusRequest {
            container_id: cid.to_string(),
            verbose: true,
        });
        let response = self.runtime_client.container_status(request).await?;
        let container_status = response.get_ref();
        Ok(format!("{:?}", container_status))
    }

    /*
    pub(crate) fn get_container_logs(
        &self,
        cid: &str,
        options: LogsOptions<String>,
    ) -> Result<impl Stream<Item = Result<LogOutput>>> {
        unimplemented!(self, cid, options)
    }

    pub(crate) async fn container_exec(&self, _cid: &str, _cmd: &str) -> Result<()> {
        unimplemented!()
    }
    */

    pub(crate) async fn info(&mut self) -> Result<(String, String)> {
        let request = tonic::Request::new(VersionRequest {
            version: "v1".to_string(),
        });
        let version = self.runtime_client.version(request).await?;
        // let request = tonic::Request::new(StatusRequest { verbose: true });
        // let status = self.runtime_client.status(request).await?.get_ref();
        let name = version.get_ref().runtime_name.to_string();
        let version = version.get_ref().runtime_version.to_string();
        Ok((name, version))
    }

    pub(crate) fn validate_container_filters(&self, _name: &str) -> bool {
        true
    }
}
