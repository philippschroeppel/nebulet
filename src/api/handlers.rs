use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde_json::json;
use tracing::{error, info};

use crate::models::v1::container::{
    CreateContainerRequest, ContainerResponse, Entity as ContainerEntity,
    Model as ContainerModel,
};
use crate::services::docker::DockerService;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub docker: DockerService,
}

pub async fn health_check() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "status": "healthy" })))
}

pub async fn create_container(
    State(state): State<AppState>,
    Json(request): Json<CreateContainerRequest>,
) -> Result<(StatusCode, Json<ContainerResponse>), (StatusCode, Json<serde_json::Value>)> {
    info!("Creating container: {}", request.name);
    
    let docker_result = state.docker.create_container(&request).await;
    
    let container_model: ContainerModel = request.clone().into();
    
    let mut container_active_model = container_model.into_active_model();
    let docker_id = if let Ok(docker_id) = &docker_result {
        Some(docker_id.clone())
    } else {
        None
    };
    
    let status = if docker_result.is_ok() {
        "Created"
    } else {
        "Failed"
    };
    
    container_active_model.status = Set(status.to_string());
    container_active_model.docker_id = Set(docker_id);
    
    // Use Entity::insert().exec() instead of ActiveModel.insert() to avoid the last_insert_id issue
    let container_id = match &container_active_model.id {
        sea_orm::ActiveValue::Set(id) => id.clone(),
        _ => {
            error!("Container ID not set in ActiveModel");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" }))));
        }
    };
    ContainerEntity::insert(container_active_model)
        .exec(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to create container in database: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?;
    
    // Fetch the inserted container
    let container_model = ContainerEntity::find_by_id(container_id)
        .one(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to fetch inserted container: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?
        .ok_or_else(|| {
            error!("Inserted container not found");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?;
    
    if let Err(e) = docker_result {
        error!("Failed to create container in Docker: {}", e);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Docker error" }))));
    }
    
    let response: ContainerResponse = container_model.into();
    
    info!("Container created successfully: {}", response.id);
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_containers(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Vec<ContainerResponse>>), (StatusCode, Json<serde_json::Value>)> {
    let containers = ContainerEntity::find().all(&state.db).await
        .map_err(|e| {
            error!("Failed to fetch containers: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?;
    
    let responses: Vec<ContainerResponse> = containers
        .into_iter()
        .map(|container| container.into())
        .collect();
    
    Ok((StatusCode::OK, Json(responses)))
}

pub async fn get_container(
    State(state): State<AppState>,
    Path(container_id): Path<String>,
) -> Result<(StatusCode, Json<ContainerResponse>), (StatusCode, Json<serde_json::Value>)> {
    let container = ContainerEntity::find_by_id(container_id.clone())
        .one(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to fetch container: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "Container not found" })))
        })?;
    
    let response: ContainerResponse = container.into();
    
    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_container(
    State(state): State<AppState>,
    Path(container_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    let container = ContainerEntity::find_by_id(container_id.clone())
        .one(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to fetch container: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, Json(json!({ "error": "Container not found" })))
        })?;
    
    if let Some(docker_id) = &container.docker_id {
        if let Err(e) = state.docker.stop_container(docker_id).await {
            error!("Failed to stop container {}: {}", docker_id, e);
        }
        
        if let Err(e) = state.docker.remove_container(docker_id).await {
            error!("Failed to remove container {}: {}", docker_id, e);
        }
    }
    
    ContainerEntity::delete_by_id(container_id.clone())
        .exec(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to delete container from database: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?;
    
    info!("Container deleted successfully: {}", container_id);
    Ok((StatusCode::OK, Json(json!({ "message": "Container deleted" }))))
}