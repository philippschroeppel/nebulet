mod api;
mod config;
mod db;
mod models;
mod services;

use anyhow::Result;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::signal;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::api::handlers::AppState;
use crate::api::routes::create_router;
use crate::config::Config;
use crate::db::{establish_connection, run_migrations};
use crate::services::{DockerService, ProcessorService};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env();
    
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::from_str(&config.log_level).unwrap_or(Level::INFO))
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting Nebulet container service...");
    info!("Configuration: {:?}", config);
    
    // Initialize database
    let db = establish_connection(&config).await?;
    run_migrations(&db).await?;
    info!("Database initialized successfully");
    
    // Initialize Docker service
    let docker = DockerService::new().await?;
    info!("Docker service initialized successfully");
    
    // Initialize processor service
    let mut processor = ProcessorService::new(config.processor_name.clone(), AppState { db: db.clone(), docker: docker.clone() }).await?;
    info!("Processor service initialized successfully");
    
    // Create application state
    let state = AppState { db: db.clone(), docker };
    
    // Create router
    let app = create_router(state);
    
    // Start HTTP server and processor service concurrently
    let addr = format!("{}:{}", config.server_host, config.server_port)
        .parse::<SocketAddr>()?;
    
    info!("Starting HTTP server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    // Run both services concurrently
    tokio::select! {
        result = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()) => {
            if let Err(e) = result {
                error!("HTTP server error: {}", e);
            }
        }
        result = processor.start() => {
            if let Err(e) = result {
                error!("Processor service error: {}", e);
            }
        }
    }
    
    processor.shutdown();
    
    info!("Nebulet service stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };
    
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };
    
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    
    info!("Shutdown signal received");
}
