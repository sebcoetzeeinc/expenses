mod db;
mod domain;
mod handlers;
mod jobs;
mod logging;
mod model;
mod monzo;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use clap::Parser;
use db::create_pool;
use handlers::{authorise, callback, get_transactions, monzo_callback};
use jobs::{account_poll_task, token_refresh_task};
use logging::setup_logging;
use sqlx::PgPool;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Expenses web application", long_about = None)]
struct Args {
    #[arg(long, default_value_t = String::from(""))]
    base_log_dir: String,

    #[arg(long)]
    base_url: String,

    #[arg(long, env = "CLIENT_ID")]
    client_id: String,

    #[arg(long, env = "CLIENT_SECRET")]
    client_secret: String,

    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    #[arg(long)]
    port: u32,

    #[arg(
        long,
        default_value_t = 300u64,
        help = "Interval in seconds for checking which tokens to refresh"
    )]
    token_refresh_interval: u64,

    #[arg(
        long,
        default_value_t = 3600u64,
        help = "Time remaining before expiry when a refresh will be triggered"
    )]
    token_refresh_threshold: u64,

    #[arg(
        long,
        default_value_t = 3600u64,
        help = "Interval in seconds for polling accounts"
    )]
    account_poll_interval: u64,
}

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
    let args = Args::parse();

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
