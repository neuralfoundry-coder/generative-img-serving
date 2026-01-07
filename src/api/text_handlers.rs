//! Text generation API handlers (OpenAI compatible)

use crate::backend::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage,
    TextCompletionRequest, TextCompletionResponse,
    ModelsResponse, ModelInfo,
};
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::State,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// API chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub presence_penalty: Option<f32>,
    #[serde(default)]
    pub frequency_penalty: Option<f32>,
    #[serde(default)]
    pub user: Option<String>,
    /// Optional: specify backend to use
    #[serde(default)]
    pub backend: Option<String>,
}

/// API text completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTextCompletionRequest {
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub top_p: Option<f32>,
    #[serde(default)]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub stream: Option<bool>,
    /// Optional: specify backend to use
    #[serde(default)]
    pub backend: Option<String>,
}

/// Chat completion handler (OpenAI /v1/chat/completions compatible)
pub async fn chat_completion(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ApiChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, AppError> {
    info!(
        model = %request.model,
        messages = request.messages.len(),
        "Received chat completion request"
    );

    // Find appropriate backend
    let backend = state.text_registry.get_backend_for_model(&request.model, request.backend.as_deref()).await?;
    
    // Create backend request
    let backend_request = ChatCompletionRequest {
        model: request.model.clone(),
        messages: request.messages,
        temperature: request.temperature,
        top_p: request.top_p,
        max_tokens: request.max_tokens,
        stream: request.stream,
        stop: request.stop,
        presence_penalty: request.presence_penalty,
        frequency_penalty: request.frequency_penalty,
        user: request.user,
    };

    // Forward to backend
    let response = backend.chat_completion(backend_request).await?;

    info!(
        model = %response.model,
        choices = response.choices.len(),
        "Chat completion completed"
    );

    Ok(Json(response))
}

/// Text completion handler (OpenAI /v1/completions compatible)
pub async fn text_completion(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ApiTextCompletionRequest>,
) -> Result<Json<TextCompletionResponse>, AppError> {
    info!(
        model = %request.model,
        prompt_len = request.prompt.len(),
        "Received text completion request"
    );

    // Find appropriate backend
    let backend = state.text_registry.get_backend_for_model(&request.model, request.backend.as_deref()).await?;
    
    // Create backend request
    let backend_request = TextCompletionRequest {
        model: request.model.clone(),
        prompt: request.prompt,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
        top_p: request.top_p,
        stop: request.stop,
        stream: request.stream,
    };

    // Forward to backend
    let response = backend.text_completion(backend_request).await?;

    info!(
        model = %response.model,
        choices = response.choices.len(),
        "Text completion completed"
    );

    Ok(Json(response))
}

/// List models handler (OpenAI /v1/models compatible)
pub async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ModelsResponse>, AppError> {
    info!("Received list models request");

    let mut all_models: Vec<ModelInfo> = Vec::new();

    // Get models from all text backends
    let backends = state.text_registry.list_backends().await;
    for backend_status in backends {
        for model_id in backend_status.models {
            all_models.push(ModelInfo {
                id: model_id,
                object: "model".to_string(),
                created: Some(Utc::now().timestamp()),
                owned_by: Some(backend_status.name.clone()),
            });
        }
    }

    // Also include image backends' models
    let image_backends = state.backend_registry.list_backends().await;
    for backend_status in image_backends {
        all_models.push(ModelInfo {
            id: format!("image/{}", backend_status.name),
            object: "model".to_string(),
            created: Some(Utc::now().timestamp()),
            owned_by: Some(backend_status.name),
        });
    }

    Ok(Json(ModelsResponse {
        object: "list".to_string(),
        data: all_models,
    }))
}

/// Text backend info for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBackendInfo {
    pub name: String,
    pub protocol: String,
    pub endpoints: Vec<String>,
    pub healthy: bool,
    pub models: Vec<String>,
    pub capabilities: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBackendListResponse {
    pub backends: Vec<TextBackendInfo>,
}

/// List text backends
pub async fn list_text_backends(
    State(state): State<Arc<AppState>>,
) -> Result<Json<TextBackendListResponse>, AppError> {
    let backends = state.text_registry.list_backends().await;
    
    let backend_infos: Vec<TextBackendInfo> = backends
        .into_iter()
        .map(|b| TextBackendInfo {
            name: b.name,
            protocol: b.protocol,
            endpoints: b.endpoints,
            healthy: b.healthy,
            models: b.models,
            capabilities: b.capabilities,
            enabled: b.enabled,
        })
        .collect();

    Ok(Json(TextBackendListResponse {
        backends: backend_infos,
    }))
}

