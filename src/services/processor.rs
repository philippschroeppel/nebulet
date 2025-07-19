use anyhow::Result;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use std::sync::{Arc, Mutex};
use tokio::time::Duration;
use tracing::{error, info, warn};

use crate::api::handlers::AppState;
use crate::models::v1::container::{
    ActiveModel as ContainerActiveModel, Entity as ContainerEntity, Model as ContainerModel,
};

pub struct ProcessorService {
    app_state: AppState,
    shutdown_signal: Arc<Mutex<bool>>,
}

impl ProcessorService {
    pub async fn new(processor_name: String, app_state: AppState) -> Result<Self> {
        let shutdown_signal = Arc::new(Mutex::new(false));

        info!("Processor service initialized: {}", processor_name);

        Ok(Self {
            app_state,
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
        let containers = ContainerEntity::find().all(&self.app_state.db).await?;

        for container in containers {
            if let Err(e) = self.process_single_container(&container).await {
                error!("Error processing container {}: {}", container.id, e);
            }
        }

        Ok(())
    }

    async fn process_single_container(&self, container: &ContainerModel) -> Result<()> {
        match container.status.as_str() {
            "Created" => {
                // Container is created but not started
                if let Some(docker_id) = &container.docker_id {
                    if let Err(e) = self.app_state.docker.start_container(docker_id).await {
                        error!("Failed to start container {}: {}", docker_id, e);
                        self.update_container_status(&container.id, "Failed")
                            .await?;
                    } else {
                        self.update_container_status(&container.id, "Running")
                            .await?;
                    }
                }
            }
            "Running" => {
                // Check if container is still running
                if let Some(docker_id) = &container.docker_id {
                    let status = self
                        .app_state
                        .docker
                        .get_container_status(docker_id)
                        .await?;
                    if status != "running" {
                        self.update_container_status(&container.id, &status).await?;
                    }
                }
            }
            "Stopped" | "Failed" => {
                // Clean up stopped/failed containers
                if let Some(docker_id) = &container.docker_id {
                    if let Err(e) = self.app_state.docker.remove_container(docker_id).await {
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

    async fn update_container_status(&self, container_id: &str, status: &str) -> Result<()> {
        let container = ContainerEntity::find_by_id(container_id.to_string())
            .one(&self.app_state.db)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Container not found"))?;

        let mut active_model: ContainerActiveModel = container.into();
        active_model.status = Set(status.to_string());
        active_model.updated_at = Set(Utc::now().to_rfc3339());

        active_model.update(&self.app_state.db).await?;

        info!("Updated container {} status to {}", container_id, status);
        Ok(())
    }

    pub fn shutdown(&self) {
        if let Ok(mut signal) = self.shutdown_signal.lock() {
            *signal = true;
        }
    }
}
