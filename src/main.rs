//! Main entry point for the Gen Serving Gateway

use gen_serving_gateway::{
    api,
    backend::registry::BackendRegistry,
    backend::TextBackendRegistry,
    config::{Settings, BackendType},
    gateway::{health_check::HealthCheckManager, load_balancer::LoadBalancer},
    queue::request_queue::RequestQueue,
    AppState,
};
use rand::Rng;
use std::sync::Arc;
use std::path::Path;
use std::io::Write;
use tokio::sync::RwLock;
use tracing::{info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Generate a random 32-character ASCII API key
fn generate_api_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Load or generate API key from .env file
fn load_or_generate_api_key() -> Option<String> {
    // Check if API key is already set in environment
    if let Ok(key) = std::env::var("GEN_GATEWAY_API_KEY") {
        if !key.is_empty() {
            info!("Using API key from environment variable");
            return Some(key);
        }
    }
    
    let env_path = Path::new(".env");
    
    // Try to read existing .env file
    if env_path.exists() {
        if let Ok(contents) = std::fs::read_to_string(env_path) {
            for line in contents.lines() {
                if line.starts_with("GEN_GATEWAY_API_KEY=") {
                    let key = line.trim_start_matches("GEN_GATEWAY_API_KEY=").trim();
                    if !key.is_empty() {
                        // Set in environment for this process
                        std::env::set_var("GEN_GATEWAY_API_KEY", key);
                        info!("Loaded API key from .env file");
                        return Some(key.to_string());
                    }
                }
            }
        }
    }
    
    // Generate new API key
    let new_key = generate_api_key();
    info!("Generated new API key");
    
    // Append to .env file
    let env_entry = format!("\n# Gateway API Key (auto-generated)\nGEN_GATEWAY_API_KEY={}\n", new_key);
    
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(env_path)
    {
        Ok(mut file) => {
            if let Err(e) = file.write_all(env_entry.as_bytes()) {
                warn!("Failed to write API key to .env: {}", e);
            } else {
                info!("API key saved to .env file");
            }
        }
        Err(e) => {
            warn!("Failed to open .env file: {}", e);
        }
    }
    
    // Set in environment for this process
    std::env::set_var("GEN_GATEWAY_API_KEY", &new_key);
    
    Some(new_key)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file first
    let _ = dotenvy::dotenv();
    
    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .init();

    info!("Starting Gen Serving Gateway");

    // Load or generate API key
    let api_key = load_or_generate_api_key();
    
    // Load configuration
    let mut settings = Settings::load()?;
    
    // If API key was loaded/generated and auth is enabled but no keys configured, add it
    if let Some(key) = api_key {
        if settings.auth.enabled && settings.auth.api_keys.is_empty() {
            settings.auth.api_keys.push(key.clone());
            info!("Using auto-configured API key for authentication");
        } else if !settings.auth.api_keys.is_empty() {
            // Add the env key to existing keys if not already present
            if !settings.auth.api_keys.contains(&key) {
                settings.auth.api_keys.push(key);
            }
        }
    }
    
    info!(
        "Loaded configuration: server={}:{}, auth_enabled={}, api_keys_count={}",
        settings.server.host, settings.server.port,
        settings.auth.enabled, settings.auth.api_keys.len()
    );

    let settings = Arc::new(RwLock::new(settings));
    
    // Initialize backend registries
    let backend_registry = Arc::new(BackendRegistry::new());
    let text_registry = Arc::new(TextBackendRegistry::new());
    
    // Register backends from configuration
    {
        let config = settings.read().await;
        
        // Register image backends
        let image_backends: Vec<_> = config.backends.iter()
            .filter(|b| b.backend_type == BackendType::Image)
            .cloned()
            .collect();
        backend_registry.initialize_from_config(&image_backends).await?;
        info!("Registered {} image backends", image_backends.len());
        
        // Register text backends
        for backend_config in config.backends.iter()
            .filter(|b| b.backend_type == BackendType::Text)
        {
            if let Err(e) = text_registry.add_backend(backend_config.clone()).await {
                warn!(
                    backend = %backend_config.name,
                    error = %e,
                    "Failed to register text backend"
                );
            }
        }
        info!("Registered {} text backends", text_registry.list_backends().await.len());
    }
    
    // Initialize load balancer
    let load_balancer = Arc::new(LoadBalancer::new(backend_registry.clone()));
    
    // Initialize health check manager
    let health_manager = Arc::new(HealthCheckManager::new(backend_registry.clone()));
    
    // Start health check background task
    {
        let config = settings.read().await;
        health_manager.start(config.backends.iter()
            .map(|b| b.health_check_interval_secs)
            .min()
            .unwrap_or(30))
            .await;
    }
    
    // Initialize request queue
    let request_queue = Arc::new(RequestQueue::new(load_balancer.clone()));
    
    // Create application state
    let app_state = Arc::new(AppState {
        settings: settings.clone(),
        backend_registry,
        text_registry,
        load_balancer,
        health_manager,
        request_queue,
    });

    // Build the router
    let app = api::routes::create_router(app_state.clone()).await;

    // Get server address and print startup info
    let addr = {
        let config = settings.read().await;
        
        // Print API key info for first-time setup
        if !config.auth.api_keys.is_empty() {
            println!("\n╔════════════════════════════════════════════════════════════╗");
            println!("║  Gen Serving Gateway - Authentication                       ║");
            println!("╠════════════════════════════════════════════════════════════╣");
            println!("║  API Key: {}...  ║", &config.auth.api_keys[0][..16]);
            println!("║  (Full key in .env file as GEN_GATEWAY_API_KEY)             ║");
            println!("╚════════════════════════════════════════════════════════════╝\n");
        }
        
        format!("{}:{}", config.server.host, config.server.port)
    };
    
    info!("Server listening on {}", addr);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
