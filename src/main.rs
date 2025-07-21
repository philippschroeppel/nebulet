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

use crate::api::handlers::AppState;
use crate::api::routes::create_router;
use crate::config::Config;
use crate::db::{establish_connection, run_migrations};
use crate::services::ProcessorService;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env();

    match config.log_json {
        true => {
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(Level::from_str(&config.log_level).unwrap_or(Level::INFO))
                .json()
                .flatten_event(true)
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        false => {
            let subscriber = tracing_subscriber::fmt()
                .with_max_level(Level::from_str(&config.log_level).unwrap_or(Level::INFO))
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
    };

    info!("Starting Nebulet container service...");
    info!("Configuration: {:?}", config);

    let db = establish_connection(&config).await?;
    run_migrations(&db).await?;
    info!("Database initialized successfully");
    
    let mut processor = ProcessorService::new(config.processor_name.clone(), db.clone()).await?;
    info!("Processor service initialized successfully");
    
    let state = AppState { db };
    
    let app = create_router(state);
    let addr = format!("{}:{}", config.server_host, config.server_port).parse::<SocketAddr>()?;
    info!("Starting HTTP server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Run api and processor concurrently
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
