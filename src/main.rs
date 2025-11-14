mod api;
mod auth;
mod billing;
mod cache;
mod config;
mod database;
mod inference;
mod models;
mod monitoring;

use axum::{
    http::Method,
    routing::{get, post},
    Router,
};
use prometheus::{Encoder, TextEncoder};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if it exists
    if let Err(e) = dotenvy::dotenv() {
        println!("No .env file found, using environment variables: {}", e);
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting Smally API...");

    let settings = config::get_settings();

    // Initialize database
    info!("Initializing database...");
    database::init_db().await?;
    info!("Database connection pool initialized");

    // Load ONNX model
    info!("Loading ONNX model...");
    inference::init_model()?;
    info!("Model loaded: {}", settings.model_name);

    // Connect to Redis cache
    info!("Connecting to Redis...");
    cache::init_cache().await?;
    info!("Redis connected");

    // Initialize Redis connection for billing (rate limiting)
    info!("Initializing Redis connection for billing...");
    billing::init_redis().await?;
    info!("Redis connection for billing initialized");

    // Initialize token validator
    info!("Initializing token validator...");
    auth::init_token_validator().await?;
    info!("Token validator initialized");

    // Initialize usage buffer with background flush task
    info!("Initializing usage buffer...");
    billing::init_usage_buffer(database::get_db())?;
    info!("Usage buffer initialized with 5-second flush interval");

    // Setup CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            hyper::header::CONTENT_TYPE,
            hyper::header::AUTHORIZATION,
            hyper::header::ACCEPT,
        ])
        .allow_credentials(false);

    // Setup routes
    let app = Router::new()
        .route("/v1/embed", post(api::create_embedding_handler))
        .route("/health", get(api::health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/", get(api::root_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
        )
        .layer(cors);

    // Create server address
    let addr: SocketAddr = settings.address().parse()?;

    info!("Smally API started on http://{}", addr);

    // Start server with graceful shutdown
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Shutdown complete");

    Ok(())
}

async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Shutting down Smally API...");
        },
        _ = terminate => {
            info!("Shutting down Smally API...");
        },
    }
}
