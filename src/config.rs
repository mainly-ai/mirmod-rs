use crate::debug_println;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MirandaConfig {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
}

#[derive(Clone, Debug)]
pub struct PartialMirandaConfig {
    pub host: Option<String>,
    pub port: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub database: Option<String>,
}

impl MirandaConfig {
    pub fn new_from_file(path: &str) -> Result<MirandaConfig, Box<dyn std::error::Error>> {
        let config = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&config)?)
    }

    pub fn new_from_default() -> Result<MirandaConfig, Box<dyn std::error::Error>> {
        if let Ok(env_config) = std::env::var("MIRANDA_CONFIG_JSON") {
            debug_println!("[cfg] Loading config from MIRANDA_CONFIG_JSON");
            return Ok(serde_json::from_str(&env_config)?);
        }

        // check if config.json exists in home directory
        let home_dir = dirs::home_dir().unwrap();
        let home_config = home_dir.join("config.json");
        if home_config.exists() {
            debug_println!("[cfg] Loading config from {}", home_config.display());
            let config = std::fs::read_to_string(home_config)?;
            return Ok(serde_json::from_str(&config)?);
        }

        // check if config.json exists in /etc/miranda
        let etc_config = std::path::Path::new("/etc/miranda/config.json");
        if etc_config.exists() {
            debug_println!("[cfg] Loading config from {}", etc_config.display());
            let config = std::fs::read_to_string(etc_config)?;
            return Ok(serde_json::from_str(&config)?);
        }

        // return error if config.json does not exist
        debug_println!("[cfg] config.json not found");
        Err("config.json not found".into())
    }

    pub fn merge_into_new(&mut self, other: PartialMirandaConfig) -> Result<MirandaConfig, ()> {
        let mut new_config = self.clone();
        if let Some(host) = other.host {
            new_config.host = host;
        }
        if let Some(port) = other.port {
            new_config.port = port;
        }
        if let Some(user) = other.user {
            new_config.user = user;
        }
        if let Some(password) = other.password {
            new_config.password = password;
        }
        if let Some(database) = other.database {
            new_config.database = database;
        }
        Ok(new_config)
    }
}

impl PartialMirandaConfig {
    pub fn new() -> PartialMirandaConfig {
        PartialMirandaConfig {
            host: None,
            port: None,
            user: None,
            password: None,
            database: None,
        }
    }

    pub fn new_from_user(user: String, password: String) -> PartialMirandaConfig {
        PartialMirandaConfig {
            user: Some(user),
            password: Some(password),
            host: None,
            port: None,
            database: None,
        }
    }

    pub fn new_from_token_string(
        token: String,
    ) -> Result<PartialMirandaConfig, Box<dyn std::error::Error>> {
        // pxy.username.password
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid token".into());
        }
        let user = "pxy.".to_owned() + parts[1];
        let password = parts[2].to_string();
        Ok(PartialMirandaConfig::new_from_user(user, password))
    }

    pub fn merge_into_new(
        &mut self,
        other: PartialMirandaConfig,
    ) -> Result<PartialMirandaConfig, ()> {
        let mut new_config = self.clone();
        if let Some(host) = other.host {
            new_config.host = Some(host);
        }
        if let Some(port) = other.port {
            new_config.port = Some(port);
        }
        if let Some(user) = other.user {
            new_config.user = Some(user);
        }
        if let Some(password) = other.password {
            new_config.password = Some(password);
        }
        if let Some(database) = other.database {
            new_config.database = Some(database);
        }
        Ok(new_config)
    }
}
