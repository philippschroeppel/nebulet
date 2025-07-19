use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_port: u16,
    pub server_host: String,
    pub processor_name: String,
    pub log_level: String,
    pub log_json: bool,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://./nebulet.db?mode=rwc".to_string());

        Self {
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            processor_name: env::var("PROCESSOR_NAME")
                .unwrap_or_else(|_| "nebulet-processor".to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            log_json: env::var("LOG_JSON").is_ok(),
            database_url,
        }
    }
}
