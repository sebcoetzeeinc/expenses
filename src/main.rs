mod args;
mod db;
mod domain;
mod handlers;
mod jobs;
mod logging;
mod model;
mod monzo;

use std::sync::Arc;

use args::parse_args;
use axum::{
    Router,
    routing::{get, post},
};
use db::create_pool;
use handlers::{authorise, callback, get_transactions, monzo_callback};
use jobs::{account_poll_task, token_refresh_task};
use logging::setup_logging;
use sqlx::PgPool;

pub struct AppState {
    base_url: String,
    client_id: String,
    client_secret: String,
    pool: PgPool,
    token_refresh_interval: u64,
    token_refresh_threshold: u64,
    account_poll_interval: u64,
}

#[tokio::main]
async fn main() {
    let args = parse_args();

    setup_logging(&args.base_log_dir);

    let pool = create_pool(&args.database_url)
        .await
        .expect("Failed to create PostgreSQL pool");

    let app_state = Arc::new(AppState {
        base_url: args.base_url,
        client_id: args.client_id,
        client_secret: args.client_secret,
        pool,
        token_refresh_interval: args.token_refresh_interval,
        token_refresh_threshold: args.token_refresh_threshold,
        account_poll_interval: args.account_poll_interval,
    });

    tracing::info!("Spawning background tasks...");
    tokio::spawn(token_refresh_task(app_state.clone()));
    tokio::spawn(account_poll_task(app_state.clone()));

    // build our application with a single route
    let app = Router::new()
        .route("/api/transactions/{user_id}", get(get_transactions))
        .route("/api/monzo-callback", post(monzo_callback))
        .route("/authorise", get(authorise))
        .route("/oauth/callback", get(callback))
        .route("/", get(|| async { "Hello, World!" }))
        .with_state(app_state);

    let bind_address = format! {"0.0.0.0:{}", args.port};
    tracing::info!("Server listening on {}...", bind_address);

    let listener = tokio::net::TcpListener::bind(bind_address).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
