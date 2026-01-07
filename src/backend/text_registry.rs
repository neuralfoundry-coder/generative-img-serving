//! Text backend registry for managing text generation backends

use std::sync::Arc;
use dashmap::DashMap;
use tracing::{info, warn};

use crate::backend::text_backend::{TextBackend, TextBackendStatus, create_text_backend};
use crate::config::{BackendConfig, BackendType};
use crate::error::{AppError, Result};

/// Registry for text generation backends
pub struct TextBackendRegistry {
    backends: DashMap<String, Arc<dyn TextBackend>>,
    model_to_backend: DashMap<String, String>,
}

impl TextBackendRegistry {
    /// Create a new text backend registry
    pub fn new() -> Self {
        Self {
            backends: DashMap::new(),
            model_to_backend: DashMap::new(),
        }
    }

    /// Add a backend from configuration
    pub async fn add_backend(&self, config: BackendConfig) -> Result<()> {
        if config.backend_type != BackendType::Text {
            return Err(AppError::Internal(format!(
                "Backend '{}' is not a text backend",
                config.name
            )));
        }

        let backend = create_text_backend(&config)?;
        let name = config.name.clone();
        
        // Register model mappings
        for model in &config.models {
            self.model_to_backend.insert(model.clone(), name.clone());
        }
        
        self.backends.insert(name.clone(), backend);
        info!(name = %name, "Text backend registered");
        
        Ok(())
    }

    /// Remove a backend
    pub async fn remove_backend(&self, name: &str) -> Result<()> {
        if self.backends.remove(name).is_none() {
            return Err(AppError::BackendNotFound(name.to_string()));
        }
        
        // Remove model mappings for this backend
        self.model_to_backend.retain(|_, v| v != name);
        
        info!(name = %name, "Text backend removed");
        Ok(())
    }

    /// Get a backend by name
    pub async fn get_backend(&self, name: &str) -> Option<Arc<dyn TextBackend>> {
        self.backends.get(name).map(|b| b.value().clone())
    }

    /// Get a backend for a specific model
    pub async fn get_backend_for_model(
        &self,
        model: &str,
        preferred_backend: Option<&str>,
    ) -> Result<Arc<dyn TextBackend>> {
        // If preferred backend specified, use it
        if let Some(backend_name) = preferred_backend {
            if let Some(backend) = self.backends.get(backend_name) {
                return Ok(backend.value().clone());
            }
            warn!(backend = %backend_name, "Preferred backend not found, falling back");
        }

        // Try to find backend by model mapping
        if let Some(backend_name) = self.model_to_backend.get(model) {
            if let Some(backend) = self.backends.get(backend_name.value()) {
                let b = backend.value().clone();
                if b.is_enabled() {
                    return Ok(b);
                }
            }
        }

        // Fall back to first enabled backend that has the model
        for entry in self.backends.iter() {
            let backend = entry.value();
            if backend.is_enabled() && backend.models().contains(&model.to_string()) {
                return Ok(backend.clone());
            }
        }

        // Fall back to any enabled backend
        for entry in self.backends.iter() {
            let backend = entry.value();
            if backend.is_enabled() {
                return Ok(backend.clone());
            }
        }

        Err(AppError::NoHealthyBackends(format!(
            "No available backend for model '{}'",
            model
        )))
    }

    /// List all backends with status
    pub async fn list_backends(&self) -> Vec<TextBackendStatus> {
        self.backends
            .iter()
            .map(|entry| entry.value().status())
            .collect()
    }

    /// Get all backends
    pub fn get_all_backends(&self) -> Vec<Arc<dyn TextBackend>> {
        self.backends
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Run health checks on all backends
    pub async fn health_check_all(&self) -> (usize, usize, usize) {
        let mut total = 0;
        let mut healthy = 0;
        let mut unhealthy = 0;

        for entry in self.backends.iter() {
            total += 1;
            if entry.value().health_check().await {
                healthy += 1;
            } else {
                unhealthy += 1;
            }
        }

        (total, healthy, unhealthy)
    }
}

impl Default for TextBackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

