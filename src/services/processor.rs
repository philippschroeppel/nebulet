use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use std::sync::{Arc, Mutex};
use tokio::time::Duration;
use tracing::{error, info, warn};

use crate::models::v1::container::{Entity as ContainerEntity, Model as ContainerModel, ActiveModel as ContainerActiveModel};
use crate::services::docker::DockerService;

pub struct ProcessorService {
    db: sea_orm::DatabaseConnection,
    docker: DockerService,
    shutdown_signal: Arc<Mutex<bool>>,
}

impl ProcessorService {
    pub async fn new(processor_name: String, db: sea_orm::DatabaseConnection) -> Result<Self> {
        let shutdown_signal = Arc::new(Mutex::new(false));
        
        // Create Docker service internally since it's private to this module
        let docker = DockerService::new().await?;
        
        info!("Processor service initialized: {}", processor_name);

        Ok(Self {
            db,
            docker,
            shutdown_signal,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting processor service...");

        self.run_main_loop().await?;

        Ok(())
    }

    async fn run_main_loop(&mut self) -> Result<()> {
        info!("Starting main processing loop");

        let mut interval = tokio::time::interval(Duration::from_secs(10));

        loop {
            if *self.shutdown_signal.lock().unwrap() {
                info!("Shutdown signal received, stopping processor");
                break;
            }

            interval.tick().await;

            if let Err(e) = self.process_containers().await {
                error!("Error in main processing loop: {}", e);
            }
        }

        Ok(())
    }

    async fn process_containers(&self) -> Result<()> {
        let containers = ContainerEntity::find().all(&self.db).await?;
        
        for container in containers {
            if let Err(e) = self.process_single_container(&container).await {
                error!("Error processing container {}: {}", container.id, e);
            }
        }

        Ok(())
    }

    async fn process_single_container(&self, container: &ContainerModel) -> Result<()> {
        match container.status.as_str() {
            "Pending" => {
                // Container is pending creation - create it in Docker
                info!("Creating container in Docker: {}", container.name);
                match self.docker.create_container(&crate::models::v1::container::CreateContainerRequest {
                    name: container.name.clone(),
                    image: container.image.clone(),
                }).await {
                    Ok(docker_id) => {
                        self.update_container_status(&container.id, "Created", Some(docker_id)).await?;
                        info!("Container created successfully: {}", container.id);
                    }
                    Err(e) => {
                        error!("Failed to create container {}: {}", container.id, e);
                        self.update_container_status(&container.id, "Failed", None).await?;
                    }
                }
            }
            "Created" => {
                // Container is created but not started
                if let Some(docker_id) = &container.docker_id {
                    info!("Starting container: {}", docker_id);
                    if let Err(e) = self.docker.start_container(docker_id).await {
                        error!("Failed to start container {}: {}", docker_id, e);
                        self.update_container_status(&container.id, "Failed", None).await?;
                    } else {
                        self.update_container_status(&container.id, "Running", None).await?;
                        info!("Container started successfully: {}", container.id);
                    }
                }
            }
            "Running" => {
                // Check if container is still running
                if let Some(docker_id) = &container.docker_id {
                    let status = self.docker.get_container_status(docker_id).await?;
                    if status != "running" {
                        self.update_container_status(&container.id, &status, None).await?;
                    }
                }
            }
            "Removing" => {
                // Container is marked for removal
                if let Some(docker_id) = &container.docker_id {
                    info!("Removing container: {}", docker_id);
                    if let Err(e) = self.docker.stop_container(docker_id).await {
                        warn!("Failed to stop container {}: {}", docker_id, e);
                    }
                    
                    if let Err(e) = self.docker.remove_container(docker_id).await {
                        warn!("Failed to remove container {}: {}", docker_id, e);
                    }
                }
                
                // Delete from database
                ContainerEntity::delete_by_id(container.id.clone())
                    .exec(&self.db)
                    .await?;
                info!("Container removed from database: {}", container.id);
            }
            "Stopped" | "Failed" => {
                // Clean up stopped/failed containers
                if let Some(docker_id) = &container.docker_id {
                    if let Err(e) = self.docker.remove_container(docker_id).await {
                        warn!("Failed to remove container {}: {}", docker_id, e);
                    }
                }
            }
            _ => {
                // Unknown status, log warning
                warn!("Unknown container status: {}", container.status);
            }
        }

        Ok(())
    }

    async fn update_container_status(&self, container_id: &str, status: &str, docker_id: Option<String>) -> Result<()> {
        let container = ContainerEntity::find_by_id(container_id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Container not found"))?;

        let mut active_model: ContainerActiveModel = container.into();
        active_model.status = Set(status.to_string());
        active_model.updated_at = Set(Utc::now().to_rfc3339());
        
        if let Some(docker_id) = docker_id {
            active_model.docker_id = Set(Some(docker_id));
        }
        
        active_model.update(&self.db).await?;
        
        info!("Updated container {} status to {}", container_id, status);
        Ok(())
    }

    pub fn shutdown(&self) {
        if let Ok(mut signal) = self.shutdown_signal.lock() {
            *signal = true;
        }
    }
}
