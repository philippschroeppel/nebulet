use crate::models::CreateContainerRequest;
use anyhow::Result;
use bollard::container::{Config, CreateContainerOptions, ListContainersOptions};
use bollard::Docker;
use std::default::Default;
use tracing::{error, info};

#[derive(Clone)]
pub struct DockerService {
    _docker: Docker,
}

impl DockerService {
    #[tracing::instrument]
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        let version = docker.version().await?;
        info!(
            version = version.version,
            "Docker service initialized successfully"
        );
        Ok(Self { _docker: docker })
    }

    pub async fn create_container(&self, request: &CreateContainerRequest) -> Result<String> {
        info!("Creating container: {}", request.name);

        let options = Some(CreateContainerOptions {
            name: &request.name,
            platform: None,
        });
        let config = Config {
            image: Some(request.image.clone()),
            ..Default::default()
        };

        let container_id = match self._docker.create_container(options, config).await {
            Ok(response) => response.id,
            Err(e) => {
                error!("Failed to create container: {}", e);
                return Err(e.into());
            }
        };
        info!("Container created successfully: {}", container_id);
        Ok(container_id)
    }

    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        info!("Starting container: {}", container_id);
        // TODO: Implement actual container start
        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        info!("Stopping container: {}", container_id);
        // TODO: Implement actual container stop
        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<()> {
        info!("Removing container: {}", container_id);
        // TODO: Implement actual container removal
        Ok(())
    }

    pub async fn get_container_status(&self, _container_id: &str) -> Result<String> {
        // TODO: Implement actual status check
        Ok("running".to_string())
    }

    pub async fn _list_containers(&self) -> Result<Vec<String>> {
        info!("Listing all containers");
        let options = Some(ListContainersOptions::<&str> {
            all: true,
            ..Default::default()
        });
        let containers = match self._docker.list_containers(options).await {
            Ok(containers) => containers.iter().filter_map(|c| c.id.clone()).collect(),
            Err(e) => {
                error!("Failed to list containers: {}", e);
                return Err(e.into());
            }
        };
        Ok(containers)
    }
}
