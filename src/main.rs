mod api;
mod auth;
mod billing;
mod cache;
mod config;
mod database;
mod inference;
mod models;
mod monitoring;
mod web;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if it exists
    if let Err(e) = dotenvy::dotenv() {
        println!("No .env file found, using environment variables: {}", e);
    }

    // Initialize tracing
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

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
        // Web UI routes (root domain)
        .route("/", get(web::home))
        .route("/login", get(web::auth::login_page))
        .route("/login", post(web::auth::login_submit))
        .route("/register", get(web::auth::register_page))
        .route("/register", post(web::auth::register_submit))
        .route("/dashboard", get(web::dashboard::show))
        // API routes (will be moved to api. subdomain later)
        // Embedding API (CWT token authentication)
        .route("/v1/embed", post(api::create_embedding_handler))
        // User authentication (admin token required)
        .route("/v1/auth/register", post(api::users::register_handler))
        .route("/v1/auth/login", post(api::users::login_handler))
        // User profile (JWT session required)
        .route("/v1/users/me", get(api::users::get_profile_handler))
        // Organization management (JWT session required)
        .route(
            "/v1/organizations",
            post(api::organizations::create_organization_handler),
        )
        .route(
            "/v1/organizations",
            get(api::organizations::list_organizations_handler),
        )
        .route(
            "/v1/organizations/:org_id",
            get(api::organizations::get_organization_handler),
        )
        .route(
            "/v1/organizations/:org_id/members",
            post(api::organizations::invite_member_handler),
        )
        // API key management (JWT session required)
        .route(
            "/v1/organizations/:org_id/keys",
            post(api::api_keys::create_api_key_handler),
        )
        .route(
            "/v1/organizations/:org_id/keys",
            get(api::api_keys::list_api_keys_handler),
        )
        .route(
            "/v1/organizations/:org_id/keys/:key_id",
            axum::routing::delete(api::api_keys::revoke_api_key_handler),
        )
        // Health and metrics
        .route("/health", get(api::health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/api", get(api::root_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
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
