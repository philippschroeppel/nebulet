use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::api::handlers::{
    create_container, delete_container, get_container, health_check, list_containers, AppState,
};

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::permissive();

    let v1_routes = Router::new()
        .route("/health", get(health_check))
        .route("/containers", get(list_containers))
        .route("/containers", post(create_container))
        .route("/containers/:id", get(get_container))
        .route("/containers/:id", delete(delete_container));

    Router::new()
        .nest("/v1", v1_routes)
        .layer(cors)
        .with_state(state)
}
