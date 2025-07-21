use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sea_orm::{DatabaseConnection, EntityTrait, Set, ActiveModelTrait};
use serde_json::json;
use tracing::{error, info};

use crate::models::v1::container::{
    ContainerResponse, CreateContainerRequest, Entity as ContainerEntity, Model as ContainerModel,
};

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
}

pub async fn health_check() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "status": "healthy" })))
}

pub async fn create_container(
    State(state): State<AppState>,
    Json(request): Json<CreateContainerRequest>,
) -> Result<(StatusCode, Json<ContainerResponse>), (StatusCode, Json<serde_json::Value>)> {
    info!("Creating container: {}", request.name);

    let mut container_model: ContainerModel = request.clone().into();
    
    // Set initial status to "Pending" - processor will handle Docker creation
    container_model.status = "Pending".to_string();
    container_model.docker_id = None;

    let container_active_model = container_model.clone().into_active_model();
    
    ContainerEntity::insert(container_active_model)
        .exec(&state.db)
        .await
        .map_err(|e| {
            error!("Failed to create container in database: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Database error" })),
            )
        })?;

    let response: ContainerResponse = container_model.into();

    info!("Container record created successfully: {}", response.id);
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn list_containers(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Vec<ContainerResponse>>), (StatusCode, Json<serde_json::Value>)> {
    let containers = ContainerEntity::find().all(&state.db).await.map_err(|e| {
        error!("Failed to fetch containers: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "Database error" })),
        )
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Database error" })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Container not found" })),
            )
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
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Database error" })),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "Container not found" })),
            )
        })?;
    
    // Mark container for removal - processor will handle actual Docker operations
    let mut active_model = container.into_active_model();
    active_model.status = Set("Removing".to_string());
    active_model.updated_at = Set(chrono::Utc::now().to_rfc3339());
    
    active_model.update(&state.db).await
        .map_err(|e| {
            error!("Failed to mark container for removal: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Database error" })))
        })?;
    
    info!("Container marked for removal: {}", container_id);
    Ok((StatusCode::OK, Json(json!({ "message": "Container marked for removal" }))))
}
