//! Application settings and configuration management

use crate::error::{AppError, Result};
use config::{Config, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Root configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub backends: Vec<BackendConfig>,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub api_keys: Vec<String>,
    #[serde(default)]
    pub bypass_paths: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_rps")]
    pub requests_per_second: u32,
    #[serde(default = "default_burst")]
    pub burst_size: u32,
}

fn default_rps() -> u32 {
    100
}

fn default_burst() -> u32 {
    200
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    #[serde(default = "default_storage_path")]
    pub base_path: String,
    #[serde(default = "default_url_prefix")]
    pub url_prefix: String,
}

fn default_storage_path() -> String {
    "./generated".to_string()
}

fn default_url_prefix() -> String {
    "http://localhost:8080/files".to_string()
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

/// Backend type enum
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    Image,
    Text,
    Multi, // For backends that support both
}

impl Default for BackendType {
    fn default() -> Self {
        BackendType::Image
    }
}

/// Protocol type enum
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolType {
    Http,
    Grpc,
    OpenAI,
    Anthropic,
    Tgi, // Text Generation Inference
}

impl Default for ProtocolType {
    fn default() -> Self {
        ProtocolType::Http
    }
}

impl std::fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolType::Http => write!(f, "http"),
            ProtocolType::Grpc => write!(f, "grpc"),
            ProtocolType::OpenAI => write!(f, "openai"),
            ProtocolType::Anthropic => write!(f, "anthropic"),
            ProtocolType::Tgi => write!(f, "tgi"),
        }
    }
}

/// Authentication type for backend
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BackendAuth {
    #[serde(rename = "type", default = "default_auth_type")]
    pub auth_type: String,
    #[serde(default)]
    pub token_env: Option<String>,
    #[serde(default)]
    pub header_name: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_auth_type() -> String {
    "none".to_string()
}

/// Health check configuration for backend
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BackendHealthCheck {
    #[serde(default = "default_health_check_path")]
    pub path: String,
    #[serde(default = "default_health_check_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_health_timeout")]
    pub timeout_secs: u64,
}

fn default_health_timeout() -> u64 {
    5
}

/// Load balancer configuration for backend
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BackendLoadBalancer {
    #[serde(default = "default_lb_strategy")]
    pub strategy: String,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_lb_strategy() -> String {
    "round_robin".to_string()
}

/// Backend configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackendConfig {
    pub name: String,
    
    #[serde(rename = "type", default)]
    pub backend_type: BackendType,
    
    #[serde(default)]
    pub protocol: ProtocolType,
    
    pub endpoints: Vec<String>,
    
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    #[serde(default)]
    pub auth: BackendAuth,
    
    #[serde(default)]
    pub health_check: BackendHealthCheck,
    
    #[serde(default)]
    pub load_balancer: BackendLoadBalancer,
    
    #[serde(default)]
    pub models: Vec<String>,
    
    #[serde(default)]
    pub capabilities: Vec<String>,
    
    // Legacy fields for backward compatibility
    #[serde(default = "default_health_check_path")]
    pub health_check_path: String,
    
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
    
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_health_check_path() -> String {
    "/health".to_string()
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_timeout() -> u64 {
    60000
}

fn default_weight() -> u32 {
    1
}

/// YAML Backends configuration file structure
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BackendsConfig {
    #[serde(default)]
    pub version: String,
    
    #[serde(default)]
    pub defaults: BackendsDefaults,
    
    #[serde(default)]
    pub backends: BackendGroups,
    
    #[serde(default)]
    pub routing: RoutingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BackendsDefaults {
    #[serde(default)]
    pub health_check: BackendHealthCheck,
    
    #[serde(default)]
    pub connection: ConnectionDefaults,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ConnectionDefaults {
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
}

fn default_retry_count() -> u32 {
    3
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BackendGroups {
    #[serde(default)]
    pub image: Vec<BackendConfig>,
    
    #[serde(default)]
    pub text: Vec<BackendConfig>,
    
    #[serde(default)]
    pub grpc: Vec<BackendConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RoutingConfig {
    #[serde(default = "default_lb_strategy")]
    pub default_strategy: String,
    
    #[serde(default)]
    pub model_mappings: HashMap<String, String>,
    
    #[serde(default)]
    pub fallbacks: HashMap<String, Vec<String>>,
}

impl Settings {
    /// Load settings from configuration files and environment variables
    pub fn load() -> Result<Self> {
        Self::load_from_paths("config/gateway.yaml", Some("config/backends.yaml"))
    }

    /// Load settings from a specific configuration file path (TOML - legacy)
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_str().unwrap_or("config/default");
        
        let config = Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("auth.enabled", true)?
            .set_default("rate_limit.enabled", true)?
            .set_default("rate_limit.requests_per_second", 100)?
            .set_default("rate_limit.burst_size", 200)?
            .add_source(File::with_name(path_str).required(false))
            .add_source(
                Environment::with_prefix("GEN_GATEWAY")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }

    /// Load settings from YAML configuration files
    pub fn load_from_paths<P: AsRef<Path>>(
        gateway_config: P,
        backends_config: Option<P>,
    ) -> Result<Self> {
        let gateway_path = gateway_config.as_ref();
        
        // Determine file format
        let format = if gateway_path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
            FileFormat::Yaml
        } else {
            FileFormat::Toml
        };
        
        let mut config_builder = Config::builder()
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("auth.enabled", true)?
            .set_default("auth.bypass_paths", Vec::<String>::new())?
            .set_default("rate_limit.enabled", true)?
            .set_default("rate_limit.requests_per_second", 100)?
            .set_default("rate_limit.burst_size", 200)?
            .set_default("storage.base_path", "./generated")?
            .set_default("storage.url_prefix", "http://localhost:8080/files")?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "json")?;
        
        // Add gateway config if exists
        if gateway_path.exists() {
            config_builder = config_builder.add_source(
                File::from(gateway_path).format(format)
            );
        }
        
        // Add environment overrides
        config_builder = config_builder.add_source(
            Environment::with_prefix("GEN_GATEWAY")
                .separator("__")
                .try_parsing(true),
        );
        
        let config = config_builder.build()?;
        let mut settings: Settings = config.try_deserialize()?;
        
        // Load backends from separate file if provided
        if let Some(backends_path) = backends_config {
            let backends_path = backends_path.as_ref();
            if backends_path.exists() {
                let backends_config = Self::load_backends_config(backends_path)?;
                settings.backends = Self::flatten_backends(backends_config);
            }
        }
        
        Ok(settings)
    }
    
    /// Load backends configuration from YAML file
    pub fn load_backends_config<P: AsRef<Path>>(path: P) -> Result<BackendsConfig> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| AppError::Config(config::ConfigError::Message(
                format!("Failed to read backends config: {}", e)
            )))?;
        
        let config: BackendsConfig = serde_yaml::from_str(&content)
            .map_err(|e| AppError::Config(config::ConfigError::Message(
                format!("Failed to parse backends config: {}", e)
            )))?;
        
        Ok(config)
    }
    
    /// Save backends configuration to YAML file
    pub fn save_backends_config<P: AsRef<Path>>(path: P, config: &BackendsConfig) -> Result<()> {
        let content = serde_yaml::to_string(config)
            .map_err(|e| AppError::Config(config::ConfigError::Message(
                format!("Failed to serialize backends config: {}", e)
            )))?;
        
        std::fs::write(path.as_ref(), content)
            .map_err(|e| AppError::Config(config::ConfigError::Message(
                format!("Failed to write backends config: {}", e)
            )))?;
        
        Ok(())
    }
    
    /// Flatten backend groups into a single list
    fn flatten_backends(config: BackendsConfig) -> Vec<BackendConfig> {
        let mut backends = Vec::new();
        
        // Add image backends with type set
        for mut backend in config.backends.image {
            backend.backend_type = BackendType::Image;
            backends.push(backend);
        }
        
        // Add text backends with type set
        for mut backend in config.backends.text {
            backend.backend_type = BackendType::Text;
            backends.push(backend);
        }
        
        // Add gRPC backends
        for backend in config.backends.grpc {
            backends.push(backend);
        }
        
        backends
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(AppError::Config(config::ConfigError::Message(
                "Server port cannot be 0".to_string(),
            )));
        }

        // Validate backends
        for backend in &self.backends {
            if backend.name.is_empty() {
                return Err(AppError::Config(config::ConfigError::Message(
                    "Backend name cannot be empty".to_string(),
                )));
            }
            if backend.endpoints.is_empty() {
                return Err(AppError::Config(config::ConfigError::Message(
                    format!("Backend '{}' must have at least one endpoint", backend.name),
                )));
            }
        }

        Ok(())
    }
    
    /// Get backends by type
    pub fn get_backends_by_type(&self, backend_type: BackendType) -> Vec<&BackendConfig> {
        self.backends
            .iter()
            .filter(|b| b.backend_type == backend_type && b.enabled)
            .collect()
    }
    
    /// Get enabled backends
    pub fn get_enabled_backends(&self) -> Vec<&BackendConfig> {
        self.backends
            .iter()
            .filter(|b| b.enabled)
            .collect()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
            },
            auth: AuthConfig {
                enabled: true,
                api_keys: vec![],
                bypass_paths: vec!["/health".to_string()],
            },
            rate_limit: RateLimitConfig {
                enabled: true,
                requests_per_second: default_rps(),
                burst_size: default_burst(),
            },
            storage: StorageConfig {
                base_path: default_storage_path(),
                url_prefix: default_url_prefix(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                format: default_log_format(),
            },
            backends: vec![],
        }
    }
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            backend_type: BackendType::default(),
            protocol: ProtocolType::default(),
            endpoints: vec![],
            enabled: true,
            auth: BackendAuth::default(),
            health_check: BackendHealthCheck::default(),
            load_balancer: BackendLoadBalancer::default(),
            models: vec![],
            capabilities: vec![],
            health_check_path: default_health_check_path(),
            health_check_interval_secs: default_health_check_interval(),
            timeout_ms: default_timeout(),
            weight: default_weight(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.server.port, 8080);
        assert!(settings.auth.enabled);
        assert!(settings.rate_limit.enabled);
    }
    
    #[test]
    fn test_backend_type_serialization() {
        let backend = BackendConfig {
            name: "test".to_string(),
            backend_type: BackendType::Text,
            ..Default::default()
        };
        
        let yaml = serde_yaml::to_string(&backend).unwrap();
        assert!(yaml.contains("type: text"));
    }
}
