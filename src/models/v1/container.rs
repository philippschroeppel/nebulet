use chrono::{DateTime, Utc};
use sea_orm::{entity::prelude::*, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateContainerRequest {
    pub name: String,
    pub image: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerResponse {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ContainerStatus {
    Pending,
    Created,
    Running,
    Stopped,
    Failed,
    Removing,
}

impl ContainerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContainerStatus::Pending => "Pending",
            ContainerStatus::Created => "Created",
            ContainerStatus::Running => "Running",
            ContainerStatus::Stopped => "Stopped",
            ContainerStatus::Failed => "Failed",
            ContainerStatus::Removing => "Removing",
        }
    }
}

// Database Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "containers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub docker_id: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub created_at: String,
    #[sea_orm(column_type = "Text")]
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

// Using standard From trait for API -> Database conversion
impl From<CreateContainerRequest> for Model {
    fn from(api_model: CreateContainerRequest) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            name: api_model.name,
            image: api_model.image,
            status: ContainerStatus::Pending.as_str().to_string(),
            docker_id: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl From<Model> for ContainerResponse {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            image: model.image,
            status: match model.status.as_str() {
                "Pending" => ContainerStatus::Pending,
                "Created" => ContainerStatus::Created,
                "Running" => ContainerStatus::Running,
                "Stopped" => ContainerStatus::Stopped,
                "Failed" => ContainerStatus::Failed,
                "Removing" => ContainerStatus::Removing,
                _ => ContainerStatus::Pending,
            },
            created_at: DateTime::parse_from_rfc3339(&model.created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&model.updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}

// Convenience methods for the Model
impl Model {
    pub fn new(name: String, image: String) -> Self {
        CreateContainerRequest { name, image }.into()
    }

    // You can still keep this method for explicit conversion
    pub fn into_response(self) -> ContainerResponse {
        self.into()
    }

    pub fn update_status(&mut self, status: ContainerStatus) {
        self.status = status.as_str().to_string();
        self.updated_at = Utc::now().to_rfc3339();
    }

    pub fn set_docker_id(&mut self, docker_id: String) {
        self.docker_id = Some(docker_id);
        self.updated_at = Utc::now().to_rfc3339();
    }

    // Helper method to create an ActiveModel for insertion
    pub fn into_active_model(self) -> ActiveModel {
        ActiveModel {
            id: Set(self.id),
            name: Set(self.name),
            image: Set(self.image),
            status: Set(self.status),
            docker_id: Set(self.docker_id),
            created_at: Set(self.created_at),
            updated_at: Set(self.updated_at),
        }
    }
}
