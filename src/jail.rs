use crate::backend::{create_backend, JailBackend};
use crate::config::JailConfig;
use crate::error::Result;
use std::sync::Arc;

/// Jail manager that handles jail lifecycle
pub struct JailManager {
    config: JailConfig,
    backend: Arc<Box<dyn JailBackend>>,
}

impl JailManager {
    pub fn new(config: JailConfig) -> Self {
        let backend = Arc::new(create_backend(&config));
        Self { config, backend }
    }

    /// Create a new jail
    pub async fn create(&self) -> Result<()> {
        self.backend.create(&self.config).await
    }

    /// Start the jail
    pub async fn start(&self) -> Result<()> {
        self.backend.start(&self.config.name).await
    }

    /// Stop the jail
    pub async fn stop(&self) -> Result<()> {
        self.backend.stop(&self.config.name).await
    }

    /// Remove the jail
    pub async fn remove(&self) -> Result<()> {
        self.backend.remove(&self.config.name).await
    }

    /// Execute a command in the jail
    pub async fn exec(&self, command: &[String], interactive: bool) -> Result<String> {
        self.backend
            .exec(&self.config.name, command, interactive)
            .await
    }

    /// Check if jail exists
    pub async fn exists(&self) -> Result<bool> {
        self.backend.exists(&self.config.name).await
    }

    /// Get jail configuration
    pub fn config(&self) -> &JailConfig {
        &self.config
    }
}

/// Builder for creating jail configurations
pub struct JailBuilder {
    config: JailConfig,
}

impl JailBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: JailConfig {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    pub fn backend(mut self, backend: crate::config::BackendType) -> Self {
        self.config.backend = backend;
        self
    }

    pub fn base_image(mut self, image: impl Into<String>) -> Self {
        self.config.base_image = image.into();
        self
    }

    pub fn bind_mount(
        mut self,
        source: impl Into<std::path::PathBuf>,
        target: impl Into<std::path::PathBuf>,
        readonly: bool,
    ) -> Self {
        self.config.bind_mounts.push(crate::config::BindMount {
            source: source.into(),
            target: target.into(),
            readonly,
        });
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.environment.push((key.into(), value.into()));
        self
    }

    pub fn network(mut self, enabled: bool, private: bool) -> Self {
        self.config.network.enabled = enabled;
        self.config.network.private = private;
        self
    }

    pub fn memory_limit(mut self, mb: u64) -> Self {
        self.config.limits.memory_mb = Some(mb);
        self
    }

    pub fn cpu_quota(mut self, percent: u32) -> Self {
        self.config.limits.cpu_quota = Some(percent);
        self
    }

    pub fn build(self) -> JailManager {
        JailManager::new(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jail_builder() {
        let manager = JailBuilder::new("test-jail")
            .backend(crate::config::BackendType::Podman)
            .base_image("alpine:latest")
            .env("TEST", "value")
            .memory_limit(1024)
            .cpu_quota(75)
            .build();

        let config = manager.config();
        assert_eq!(config.name, "test-jail");
        assert_eq!(config.backend, crate::config::BackendType::Podman);
        assert_eq!(config.base_image, "alpine:latest");
        assert_eq!(config.limits.memory_mb, Some(1024));
        assert_eq!(config.limits.cpu_quota, Some(75));
        assert_eq!(config.environment.len(), 1);
    }
}
