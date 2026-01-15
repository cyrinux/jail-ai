use crate::error::{JailError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct State {
    pub last_weekly_upgrade_check: Option<chrono::DateTime<chrono::Utc>>,
}

impl State {
    pub fn load() -> Result<Self> {
        let path = get_state_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
            JailError::Config(format!(
                "Failed to read state file {}: {}",
                path.display(),
                e
            ))
        })?;
        let state = serde_json::from_str(&content).map_err(|e| {
            JailError::Config(format!(
                "Failed to parse state file {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(state)
    }

    pub fn save(&self) -> Result<()> {
        let path = get_state_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                JailError::Config(format!(
                    "Failed to create state directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| JailError::Config(format!("Failed to serialize state: {}", e)))?;
        std::fs::write(&path, content).map_err(|e| {
            JailError::Config(format!(
                "Failed to write state file {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(())
    }
}

fn get_state_path() -> Result<PathBuf> {
    let home_dir = std::env::var("HOME")
        .map_err(|_| JailError::Config("HOME environment variable not set".to_string()))?;
    Ok(PathBuf::from(home_dir).join(".config/jail-ai/state.json"))
}
