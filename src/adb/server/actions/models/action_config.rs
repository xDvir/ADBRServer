use tracing::{info, error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::adb::server::actions::models::device_action::DeviceActions;
use crate::adb::server::actions::models::global_actions::GlobalActions;

const SERVER_DIR: &str = "adbr-server";
const ACTIONS_FILE: &str = "actions.yml";

#[derive(Debug, Deserialize, Serialize)]
pub struct ActionConfig {
    pub global: GlobalActions,
    pub devices: HashMap<String, DeviceActions>,
}

impl ActionConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = if cfg!(target_os = "windows") {
            dirs::data_local_dir()
                .ok_or("Failed to get local data directory")?
                .join(SERVER_DIR)
                .join(ACTIONS_FILE)
        } else if cfg!(target_os = "macos") {
            dirs::data_dir()
                .ok_or("Failed to get data directory")?
                .join(SERVER_DIR)
                .join(ACTIONS_FILE)
        } else {
            dirs::config_dir()
                .ok_or("Failed to get config directory")?
                .join(SERVER_DIR)
                .join(ACTIONS_FILE)
        };

        info!("Loading actions config from: {:?}", config_path);

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| {
                    error!("Failed to create config directory: {}", e);
                    e
                })?;
        }

        let contents = match std::fs::read_to_string(&config_path) {
            Ok(contents) => contents,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                info!("No config file found, using default configuration");
                return Ok(Self::default());
            }
            Err(e) => {
                error!("Failed to read config file: {}", e);
                return Err(e.into());
            }
        };

        let config: ActionConfig = serde_yaml::from_str(&contents)
            .map_err(|e| {
                error!("Failed to parse config file: {}", e);
                e
            })?;

        Ok(config)
    }
}

impl Default for ActionConfig {
    fn default() -> Self {
        ActionConfig {
            global: GlobalActions {
                connect: Vec::new(),
                disconnect: Vec::new(),
            },
            devices: HashMap::new(),
        }
    }
}