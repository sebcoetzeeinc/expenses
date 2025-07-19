use std::sync::Arc;

use crate::{
    AppState,
    db::{query_account_ids, query_transactions, upsert_token, upsert_transaction},
    domain::{Token, Transaction},
    model::{initial_load_data, parse_monzo_date},
    monzo::{TransactionRequest, exchange_auth_code},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::form_urlencoded;

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DataResponse<T> {
    pub data: T,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    ReqwestError(reqwest::Error),
    SqlxError,
    NotFound,
    InternalServerError,
    BadRequest(String),
}

impl From<sqlx::Error> for AppError {
    fn from(_err: sqlx::Error) -> Self {
        AppError::SqlxError
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::ReqwestError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ReqwestError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!(
                    "Internal request failed with status_code={}",
                    err.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                ),
            ),
            AppError::SqlxError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            ),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Todo not found".to_string()),
            AppError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
            AppError::BadRequest(msg) => {
                tracing::info!("Bad request: {}", &msg);
                (StatusCode::BAD_REQUEST, msg)
            }
        };

        (
            status,
            Json(serde_json::json!({ "message": error_message })),
        )
            .into_response()
    }
}

fn oauth_redirect_url(base_url: &String) -> String {
    format!("{}/oauth/callback", base_url)
}

pub async fn authorise(State(state): State<Arc<AppState>>) -> Redirect {
    let redirect_url = form_urlencoded::Serializer::new(String::from("https://auth.monzo.com/?"))
        .append_pair("client_id", &state.client_id)
        .append_pair("redirect_uri", &oauth_redirect_url(&state.base_url))
        .append_pair("response_type", "code")
        .append_pair("state", "state")
        .finish();

    tracing::info!("Redirecting to {}", &redirect_url);

    Redirect::to(&redirect_url)
}

fn future_datetime_from_seconds(duration_in_seconds: u32) -> DateTime<Utc> {
    Utc::now() + Duration::seconds(duration_in_seconds.into())
}

pub async fn callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<CallbackParams>,
) -> Result<Redirect, AppError> {
    let code = params.code.ok_or(AppError::BadRequest(String::from(
        "No authorisation code received",
    )))?;

    if code.is_empty() {
        return Err(AppError::BadRequest(String::from(
            "Empty authorisation code received",
        )));
    }

    let request_state = params.state.unwrap_or(String::from(""));

    tracing::info!("Received code={} state={}", &code, &request_state);

    let token_response = exchange_auth_code(
        &state.client_id,
        &state.client_secret,
        &oauth_redirect_url(&state.base_url),
        &code,
    )
    .await?;

    tracing::info!("Received token response: {:#?}", &token_response);

    let token = Token {
        user_id: token_response.user_id,
        expiry_time: future_datetime_from_seconds(token_response.expires_in),
        token_type: token_response.token_type,
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
    };

    upsert_token(&state.pool, &token)
        .await
        .expect("An error occurred while inserting the token");

    tokio::spawn(initial_load_data(state.clone(), token.clone()));

    let redirect_url = format!("{}/", &state.base_url);

    tracing::info!("Redirecting to {}", &redirect_url);

    Ok(Redirect::to(&redirect_url))
}

#[axum::debug_handler]
pub async fn get_transactions(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<DataResponse<Vec<Transaction>>>, AppError> {
    let account_ids = query_account_ids(&state.pool, &user_id)
        .await
        .inspect_err(|err| {
            tracing::error!("Error querying account IDs in get_transactions: {:#?}", err)
        })?;

    let transactions = query_transactions(&state.pool, &account_ids)
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Error querying transactions in get_transactions: {:#?}",
                err
            )
        })?;

    Ok(Json(DataResponse { data: transactions }))
}

#[axum::debug_handler]
pub async fn monzo_callback(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Result<StatusCode, AppError> {
    tracing::info!(
        "Monzo callback was called with payload={}",
        serde_json::to_string_pretty(&payload).unwrap_or(String::from("<invalid json>"))
    );

    let payload_type = payload.get("type");
    if payload_type.is_none() {
        return Err(AppError::BadRequest(String::from(
            "Request body should have type 'transaction.created'",
        )));
    }

    let payload_type = payload_type.unwrap().as_str().unwrap_or("");
    if payload_type != "transaction.created" && payload_type != "transaction.updated" {
        return Err(AppError::BadRequest(String::from(
            "Request body should have type 'transaction.created|updated'",
        )));
    }

    let data = payload.get("data");
    if data.is_none() {
        return Err(AppError::BadRequest(String::from(
            "Unable to parse data payload to transaction request",
        )));
    }

    let transaction =
        serde_json::from_value::<TransactionRequest>(data.unwrap().clone()).map_err(|_err| {
            AppError::BadRequest(String::from(
                "Unable to parse data payload to transaction request",
            ))
        })?;

    upsert_transaction(
        &state.pool,
        &Transaction {
            id: transaction.id,
            account_id: transaction.account_id,
            amount: transaction.amount,
            currency: transaction.currency,
            description: transaction.description,
            notes: transaction.notes,
            merchant: transaction.merchant.map(|merchant| merchant.id),
            category: transaction.category,
            created: parse_monzo_date(&transaction.created).unwrap(),
            settled: parse_monzo_date(&transaction.settled),
        },
    )
    .await
    .map_err(|_err| AppError::SqlxError)?;

    Ok(StatusCode::CREATED)
}
