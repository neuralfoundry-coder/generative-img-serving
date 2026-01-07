//! HTTP route definitions

use crate::api::handlers;
use crate::api::models::*;
use crate::api::text_handlers::{self, *};
use crate::backend::{
    ChatMessage, ChatCompletionResponse, ChatChoice,
    TextCompletionResponse, TextChoice, Usage,
    ModelsResponse, ModelInfo,
};
use crate::middleware::{auth::AuthLayer, rate_limit::RateLimitLayer};
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Gen Serving Gateway API",
        version = "0.3.2",
        description = "Unified AI model serving gateway for image and text generation. OpenAI API compatible.",
        license(name = "MIT"),
    ),
    servers(
        (url = "http://localhost:15115", description = "Local development server")
    ),
    paths(
        handlers::generate_image,
        handlers::list_backends,
        handlers::add_backend,
        handlers::remove_backend,
        handlers::health_check,
        text_handlers::chat_completion,
        text_handlers::text_completion,
        text_handlers::list_models,
        text_handlers::list_text_backends,
    ),
    components(schemas(
        GenerateImageRequest,
        GenerateImageResponse,
        ImageData,
        BackendInfo,
        BackendListResponse,
        AddBackendRequest,
        HealthResponse,
        BackendHealthSummary,
        SuccessResponse,
        ApiChatCompletionRequest,
        ApiTextCompletionRequest,
        TextBackendInfo,
        TextBackendListResponse,
        ChatMessage,
        ChatCompletionResponse,
        ChatChoice,
        TextCompletionResponse,
        TextChoice,
        Usage,
        ModelsResponse,
        ModelInfo,
    )),
    tags(
        (name = "Images", description = "Image generation endpoints"),
        (name = "Chat", description = "Chat completion endpoints"),
        (name = "Text", description = "Text completion endpoints"),
        (name = "Models", description = "Model management endpoints"),
        (name = "Backends", description = "Backend management endpoints"),
        (name = "Health", description = "Health and monitoring endpoints"),
    )
)]
pub struct ApiDoc;

/// Create the main application router
pub async fn create_router(state: Arc<crate::AppState>) -> Router {
    // Get configuration for middleware
    let (auth_enabled, api_keys, rate_limit_enabled, rps, burst) = {
        let config = state.settings.read().await;
        (
            config.auth.enabled,
            config.auth.api_keys.clone(),
            config.rate_limit.enabled,
            config.rate_limit.requests_per_second,
            config.rate_limit.burst_size,
        )
    };

    // Build the API routes that require authentication and rate limiting
    let api_routes = Router::new()
        // Image generation endpoint (OpenAI compatible)
        .route("/images/generations", post(handlers::generate_image))
        // Text/Chat completion endpoints (OpenAI compatible)
        .route("/chat/completions", post(text_handlers::chat_completion))
        .route("/completions", post(text_handlers::text_completion))
        // Models endpoint
        .route("/models", get(text_handlers::list_models))
        // Backend management endpoints
        .route("/backends", get(handlers::list_backends))
        .route("/backends", post(handlers::add_backend))
        .route("/backends/:name", delete(handlers::remove_backend))
        .route("/backends/text", get(text_handlers::list_text_backends));

    // Apply middleware conditionally
    let api_routes = if rate_limit_enabled {
        api_routes.layer(RateLimitLayer::new(rps, burst))
    } else {
        api_routes
    };

    let api_routes = if auth_enabled {
        api_routes.layer(AuthLayer::new(api_keys))
    } else {
        api_routes
    };

    // Build the full router
    Router::new()
        // Health check endpoint (no auth required)
        .route("/health", get(handlers::health_check))
        // Metrics endpoint (no auth required)
        .route("/metrics", get(handlers::metrics))
        // Swagger UI
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Static file serving for generated images
        .nest_service("/images", tower_http::services::ServeDir::new("generated_images"))
        // Static file serving for generated content
        .nest_service("/files", tower_http::services::ServeDir::new("generated"))
        // API routes under /v1 prefix
        .nest("/v1", api_routes)
        // Add shared state
        .with_state(state)
        // Add tracing layer
        .layer(TraceLayer::new_for_http())
}

