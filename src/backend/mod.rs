//! Backend module - Traits, HTTP/gRPC clients, and registry

pub mod grpc_backend;
pub mod http_backend;
pub mod proto;
pub mod registry;
pub mod text_backend;
pub mod text_registry;
pub mod traits;

// Re-export text backend types for convenience
pub use text_backend::{
    TextBackend, TextBackendStatus,
    ChatMessage, ChatCompletionRequest, ChatCompletionResponse, ChatChoice,
    TextCompletionRequest, TextCompletionResponse, TextChoice,
    Usage, ModelInfo, ModelsResponse,
    create_text_backend,
};

pub use text_registry::TextBackendRegistry;

