use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(sqlx::FromRow, Clone)]
pub struct Token {
    pub user_id: String,
    pub expiry_time: DateTime<Utc>,
    pub token_type: String,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(sqlx::FromRow)]
pub struct Account {
    pub id: String,
    pub user_id: String,
    pub description: String,
    pub created: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    pub amount: i64,
    pub currency: String,
    pub description: String,
    pub notes: String,
    pub merchant: Option<String>,
    pub category: String,
    pub created: DateTime<Utc>,
    pub settled: Option<DateTime<Utc>>,
}
