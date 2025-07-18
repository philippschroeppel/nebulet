use anyhow::Result;
use sea_orm::{DatabaseConnection, Statement, ConnectionTrait};
use tracing::info;

pub async fn run_migrations(db: &DatabaseConnection) -> Result<()> {
    info!("Running database migrations...");
    
    let create_containers_table = Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        r#"
        CREATE TABLE IF NOT EXISTS containers (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            image TEXT NOT NULL,
            status TEXT NOT NULL,
            docker_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#.to_string(),
    );
    
    db.execute(create_containers_table).await?;
    
    info!("Database migrations completed successfully");
    Ok(())
} 