//! Text generation backend implementation for LLM models
//! Supports OpenAI API compatible endpoints (OpenAI, Ollama, vLLM, etc.)

use async_trait::async_trait;
use parking_lot::RwLock;
use reqwest::{Client, header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE}};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn, error};

use crate::config::{BackendConfig, ProtocolType};
use crate::error::{AppError, Result};

/// Chat message for completion requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Chat completion request (OpenAI compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Text completion request (OpenAI compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextCompletionRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Chat choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Text completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<TextChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Text choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChoice {
    pub index: u32,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_by: Option<String>,
}

/// Models list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}

/// Text backend endpoint status
#[derive(Debug, Clone)]
pub struct TextEndpoint {
    pub url: String,
    pub healthy: bool,
    pub last_check: Option<std::time::Instant>,
    pub consecutive_failures: u32,
}

impl TextEndpoint {
    pub fn new(url: String) -> Self {
        Self {
            url,
            healthy: true,
            last_check: None,
            consecutive_failures: 0,
        }
    }

    pub fn mark_healthy(&mut self) {
        self.healthy = true;
        self.last_check = Some(std::time::Instant::now());
        self.consecutive_failures = 0;
    }

    pub fn mark_unhealthy(&mut self) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= 3 {
            self.healthy = false;
        }
        self.last_check = Some(std::time::Instant::now());
    }
}

/// Text backend status
#[derive(Debug, Clone)]
pub struct TextBackendStatus {
    pub name: String,
    pub protocol: String,
    pub endpoints: Vec<String>,
    pub healthy: bool,
    pub models: Vec<String>,
    pub capabilities: Vec<String>,
    pub enabled: bool,
}

/// Trait for text generation backends
#[async_trait]
pub trait TextBackend: Send + Sync {
    /// Get the backend name
    fn name(&self) -> &str;
    
    /// Get the backend protocol
    fn protocol(&self) -> &str;
    
    /// Get available models
    fn models(&self) -> Vec<String>;
    
    /// Get supported capabilities
    fn capabilities(&self) -> Vec<String>;
    
    /// Chat completion
    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse>;
    
    /// Text completion
    async fn text_completion(&self, request: TextCompletionRequest) -> Result<TextCompletionResponse>;
    
    /// List available models from the backend
    async fn list_models(&self) -> Result<ModelsResponse>;
    
    /// Health check
    async fn health_check(&self) -> bool;
    
    /// Check if enabled
    fn is_enabled(&self) -> bool;
    
    /// Get status
    fn status(&self) -> TextBackendStatus;
}

/// OpenAI API compatible text backend
pub struct OpenAICompatibleBackend {
    name: String,
    protocol: ProtocolType,
    client: Client,
    endpoints: Arc<RwLock<Vec<TextEndpoint>>>,
    health_check_path: String,
    models: Vec<String>,
    capabilities: Vec<String>,
    enabled: bool,
    current_endpoint_index: Arc<RwLock<usize>>,
    auth_token: Option<String>,
    auth_header_name: Option<String>,
}

impl OpenAICompatibleBackend {
    /// Create a new OpenAI compatible backend
    pub fn new(config: &BackendConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let endpoints: Vec<TextEndpoint> = config
            .endpoints
            .iter()
            .map(|url| TextEndpoint::new(url.clone()))
            .collect();

        // Get auth token from environment if specified
        let auth_token = if let Some(token_env) = &config.auth.token_env {
            std::env::var(token_env).ok()
        } else {
            config.auth.api_key.clone()
        };

        let auth_header_name = config.auth.header_name.clone();

        Ok(Self {
            name: config.name.clone(),
            protocol: config.protocol.clone(),
            client,
            endpoints: Arc::new(RwLock::new(endpoints)),
            health_check_path: config.health_check.path.clone(),
            models: config.models.clone(),
            capabilities: config.capabilities.clone(),
            enabled: config.enabled,
            current_endpoint_index: Arc::new(RwLock::new(0)),
            auth_token,
            auth_header_name,
        })
    }

    /// Get headers with authentication
    fn get_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(token) = &self.auth_token {
            match &self.auth_header_name {
                Some(header_name) => {
                    // Custom header (e.g., x-api-key for Anthropic)
                    if let Ok(value) = HeaderValue::from_str(token) {
                        if let Ok(name) = reqwest::header::HeaderName::from_bytes(header_name.as_bytes()) {
                            headers.insert(name, value);
                        }
                    }
                }
                None => {
                    // Default: Bearer token
                    if let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", token)) {
                        headers.insert(AUTHORIZATION, value);
                    }
                }
            }
        }

        headers
    }

    /// Get the next healthy endpoint
    fn get_next_endpoint(&self) -> Option<String> {
        let endpoints = self.endpoints.read();
        let healthy_endpoints: Vec<_> = endpoints
            .iter()
            .filter(|e| e.healthy)
            .collect();

        if healthy_endpoints.is_empty() {
            return None;
        }

        let mut index = self.current_endpoint_index.write();
        *index = (*index + 1) % healthy_endpoints.len();
        Some(healthy_endpoints[*index].url.clone())
    }

    fn mark_endpoint_healthy(&self, url: &str) {
        let mut endpoints = self.endpoints.write();
        if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
            endpoint.mark_healthy();
            debug!(backend = %self.name, url = %url, "Marked endpoint as healthy");
        }
    }

    fn mark_endpoint_unhealthy(&self, url: &str) {
        let mut endpoints = self.endpoints.write();
        if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
            endpoint.mark_unhealthy();
            warn!(backend = %self.name, url = %url, "Marked endpoint as unhealthy");
        }
    }
}

#[async_trait]
impl TextBackend for OpenAICompatibleBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn protocol(&self) -> &str {
        match self.protocol {
            ProtocolType::OpenAI => "openai",
            ProtocolType::Anthropic => "anthropic",
            ProtocolType::Tgi => "tgi",
            ProtocolType::Http => "http",
            ProtocolType::Grpc => "grpc",
        }
    }

    fn models(&self) -> Vec<String> {
        self.models.clone()
    }

    fn capabilities(&self) -> Vec<String> {
        self.capabilities.clone()
    }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        let endpoint = self
            .get_next_endpoint()
            .ok_or_else(|| AppError::NoHealthyBackends(self.name.clone()))?;

        debug!(backend = %self.name, endpoint = %endpoint, model = %request.model, "Sending chat completion request");

        let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
        
        let response = self
            .client
            .post(&url)
            .headers(self.get_headers())
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                self.mark_endpoint_unhealthy(&endpoint);
                AppError::HttpClient(e)
            })?;

        if response.status().is_success() {
            let result = response.json::<ChatCompletionResponse>().await.map_err(|e| {
                error!(backend = %self.name, error = %e, "Failed to parse chat completion response");
                AppError::BackendError(format!("Failed to parse response: {}", e))
            })?;
            
            self.mark_endpoint_healthy(&endpoint);
            Ok(result)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            
            if status.as_u16() >= 500 {
                self.mark_endpoint_unhealthy(&endpoint);
            }
            
            Err(AppError::BackendError(format!(
                "Backend returned {}: {}",
                status, body
            )))
        }
    }

    async fn text_completion(&self, request: TextCompletionRequest) -> Result<TextCompletionResponse> {
        let endpoint = self
            .get_next_endpoint()
            .ok_or_else(|| AppError::NoHealthyBackends(self.name.clone()))?;

        debug!(backend = %self.name, endpoint = %endpoint, model = %request.model, "Sending text completion request");

        let url = format!("{}/completions", endpoint.trim_end_matches('/'));
        
        let response = self
            .client
            .post(&url)
            .headers(self.get_headers())
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                self.mark_endpoint_unhealthy(&endpoint);
                AppError::HttpClient(e)
            })?;

        if response.status().is_success() {
            let result = response.json::<TextCompletionResponse>().await.map_err(|e| {
                error!(backend = %self.name, error = %e, "Failed to parse text completion response");
                AppError::BackendError(format!("Failed to parse response: {}", e))
            })?;
            
            self.mark_endpoint_healthy(&endpoint);
            Ok(result)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            
            if status.as_u16() >= 500 {
                self.mark_endpoint_unhealthy(&endpoint);
            }
            
            Err(AppError::BackendError(format!(
                "Backend returned {}: {}",
                status, body
            )))
        }
    }

    async fn list_models(&self) -> Result<ModelsResponse> {
        let endpoint = self
            .get_next_endpoint()
            .ok_or_else(|| AppError::NoHealthyBackends(self.name.clone()))?;

        let url = format!("{}/models", endpoint.trim_end_matches('/'));
        
        let response = self
            .client
            .get(&url)
            .headers(self.get_headers())
            .send()
            .await
            .map_err(|e| {
                self.mark_endpoint_unhealthy(&endpoint);
                AppError::HttpClient(e)
            })?;

        if response.status().is_success() {
            let result = response.json::<ModelsResponse>().await.map_err(|e| {
                // If we can't parse the response, return configured models instead
                warn!(backend = %self.name, error = %e, "Failed to parse models response, using configured models");
                AppError::BackendError(format!("Failed to parse response: {}", e))
            })?;
            
            self.mark_endpoint_healthy(&endpoint);
            Ok(result)
        } else {
            // Return configured models if the endpoint doesn't support /models
            Ok(ModelsResponse {
                object: "list".to_string(),
                data: self.models.iter().map(|id| ModelInfo {
                    id: id.clone(),
                    object: "model".to_string(),
                    created: None,
                    owned_by: Some(self.name.clone()),
                }).collect(),
            })
        }
    }

    async fn health_check(&self) -> bool {
        let endpoints = self.endpoints.read().clone();
        let mut any_healthy = false;

        for endpoint in &endpoints {
            let url = format!("{}{}", endpoint.url.trim_end_matches('/'), self.health_check_path);
            
            let result = self
                .client
                .get(&url)
                .headers(self.get_headers())
                .send()
                .await;

            match result {
                Ok(response) if response.status().is_success() || response.status().as_u16() == 401 => {
                    // 401 means server is up but needs auth - still healthy
                    self.mark_endpoint_healthy(&endpoint.url);
                    any_healthy = true;
                    debug!(
                        backend = %self.name,
                        endpoint = %endpoint.url,
                        "Health check passed"
                    );
                }
                Ok(response) => {
                    self.mark_endpoint_unhealthy(&endpoint.url);
                    debug!(
                        backend = %self.name,
                        endpoint = %endpoint.url,
                        status = %response.status(),
                        "Health check failed"
                    );
                }
                Err(e) => {
                    self.mark_endpoint_unhealthy(&endpoint.url);
                    debug!(
                        backend = %self.name,
                        endpoint = %endpoint.url,
                        error = %e,
                        "Health check failed"
                    );
                }
            }
        }

        any_healthy
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn status(&self) -> TextBackendStatus {
        let endpoints = self.endpoints.read();
        let any_healthy = endpoints.iter().any(|e| e.healthy);
        
        TextBackendStatus {
            name: self.name.clone(),
            protocol: self.protocol().to_string(),
            endpoints: endpoints.iter().map(|e| e.url.clone()).collect(),
            healthy: any_healthy,
            models: self.models.clone(),
            capabilities: self.capabilities.clone(),
            enabled: self.enabled,
        }
    }
}

/// Anthropic-specific backend (Claude API)
pub struct AnthropicBackend {
    inner: OpenAICompatibleBackend,
}

impl AnthropicBackend {
    pub fn new(config: &BackendConfig) -> Result<Self> {
        Ok(Self {
            inner: OpenAICompatibleBackend::new(config)?,
        })
    }
}

#[async_trait]
impl TextBackend for AnthropicBackend {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn protocol(&self) -> &str {
        "anthropic"
    }

    fn models(&self) -> Vec<String> {
        self.inner.models()
    }

    fn capabilities(&self) -> Vec<String> {
        self.inner.capabilities()
    }

    async fn chat_completion(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        // Convert to Anthropic format
        // For now, use the OpenAI compatible endpoint
        // TODO: Implement native Anthropic API format
        self.inner.chat_completion(request).await
    }

    async fn text_completion(&self, request: TextCompletionRequest) -> Result<TextCompletionResponse> {
        self.inner.text_completion(request).await
    }

    async fn list_models(&self) -> Result<ModelsResponse> {
        // Anthropic doesn't have a models endpoint, return configured models
        Ok(ModelsResponse {
            object: "list".to_string(),
            data: self.inner.models().iter().map(|id| ModelInfo {
                id: id.clone(),
                object: "model".to_string(),
                created: None,
                owned_by: Some("anthropic".to_string()),
            }).collect(),
        })
    }

    async fn health_check(&self) -> bool {
        self.inner.health_check().await
    }

    fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }

    fn status(&self) -> TextBackendStatus {
        let mut status = self.inner.status();
        status.protocol = "anthropic".to_string();
        status
    }
}

/// Create appropriate text backend based on configuration
pub fn create_text_backend(config: &BackendConfig) -> Result<Arc<dyn TextBackend>> {
    match config.protocol {
        ProtocolType::Anthropic => {
            Ok(Arc::new(AnthropicBackend::new(config)?))
        }
        ProtocolType::OpenAI | ProtocolType::Http | ProtocolType::Tgi => {
            Ok(Arc::new(OpenAICompatibleBackend::new(config)?))
        }
        ProtocolType::Grpc => {
            Err(AppError::Internal("gRPC text backends not yet supported".to_string()))
        }
    }
}

