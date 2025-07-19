use anyhow::Result;
use sea_orm::{Database, DatabaseConnection};
use tracing::info;

use crate::config::Config;

pub async fn establish_connection(config: &Config) -> Result<DatabaseConnection> {
    info!("Connecting to database: {}", config.database_url);

    let db = Database::connect(&config.database_url).await?;

    info!("Database connection established successfully");
    Ok(db)
}
