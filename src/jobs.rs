use std::sync::Arc;

use chrono::{Duration, Utc};

use crate::{
    AppState,
    db::{query_account_ids, query_all_tokens, query_tokens_expiring_before, upsert_token},
    domain::Token,
    model::{list_and_update_accounts, list_and_update_transactions, register_webhook},
    monzo::refresh_tokens,
};

pub async fn token_refresh_task(state: Arc<AppState>) {
    // Create a Tokio interval. The first tick fires immediately.
    let mut interval =
        tokio::time::interval(std::time::Duration::from_secs(state.token_refresh_interval));

    loop {
        // Wait for the next interval tick
        interval.tick().await;
        tracing::info!("Running token_refresh_task...");

        let tokens = match query_tokens_expiring_before(
            &state.pool,
            Utc::now() + Duration::seconds(state.token_refresh_threshold as i64),
        )
        .await
        {
            Ok(tokens) => tokens,
            Err(err) => {
                tracing::error!(
                    "An error occurred while querying expiring tokens: {:#?}",
                    err
                );
                continue;
            }
        };

        tracing::info!("Found {} tokens to refresh", tokens.len());

        let token_responses = refresh_tokens(tokens, &state.client_id, &state.client_secret).await;

        for token_response in token_responses.into_iter() {
            let token = Token {
                user_id: token_response.user_id,
                expiry_time: Utc::now() + Duration::seconds(token_response.expires_in.into()),
                token_type: token_response.token_type,
                access_token: token_response.access_token,
                refresh_token: token_response.refresh_token,
            };
            match upsert_token(&state.pool, &token).await {
                Ok(_) => {
                    tracing::info!("Successfully updated token for user_id={}", &token.user_id);
                }
                Err(err) => {
                    tracing::error!(
                        "An error occurred while updating a token in the database: {:#?}",
                        err
                    );
                }
            }
        }

        tracing::info!("Finished running token_refresh_task...");
    }
}

pub async fn account_poll_task(state: Arc<AppState>) {
    // Create a Tokio interval. The first tick fires immediately.
    let mut interval =
        tokio::time::interval(std::time::Duration::from_secs(state.account_poll_interval));

    loop {
        // Wait for the next interval tick
        interval.tick().await;
        tracing::info!("Running account_poll_task...");

        let tokens = match query_all_tokens(&state.pool).await {
            Ok(tokens) => tokens,
            Err(err) => {
                tracing::error!("An error occurred while querying tokens: {:#?}", err);
                continue;
            }
        };

        tracing::info!("Found {} tokens to poll accounts for", tokens.len());

        for token in tokens.iter() {
            let _ = list_and_update_accounts(&state.pool, &token).await;
            let _ = list_and_update_transactions(&state.pool, &token).await;
            for account_id in query_account_ids(&state.pool, &token.user_id)
                .await
                .unwrap()
                .iter()
            {
                let _ = register_webhook(
                    &token.access_token,
                    account_id,
                    "https://expenses.sebastiancoetzee.com/api/monzo-callback",
                )
                .await;
            }
        }

        tracing::info!("Finished running account_poll_task...");
    }
}
