//! Gen Serving Gateway
//!
//! A Rust-based gateway for serving multiple AI model backends (image and text generation)
//! through a unified API with load balancing, health checking, and more.

pub mod api;
pub mod backend;
pub mod config;
pub mod error;
pub mod gateway;
pub mod middleware;
pub mod queue;
pub mod response;

pub use error::{AppError, Result};

use std::sync::Arc;
use tokio::sync::RwLock;

use backend::registry::BackendRegistry;
use backend::TextBackendRegistry;
use gateway::{health_check::HealthCheckManager, load_balancer::LoadBalancer};
use queue::request_queue::RequestQueue;

/// Application state shared across all handlers
pub struct AppState {
    pub settings: Arc<RwLock<config::Settings>>,
    pub backend_registry: Arc<BackendRegistry>,
    pub text_registry: Arc<TextBackendRegistry>,
    pub load_balancer: Arc<LoadBalancer>,
    pub health_manager: Arc<HealthCheckManager>,
    pub request_queue: Arc<RequestQueue>,
}

