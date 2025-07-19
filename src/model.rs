use std::{collections::HashMap, error::Error, sync::Arc};

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use futures::stream::{self, StreamExt};

use crate::{
    AppState,
    db::{query_account_ids, upsert_account, upsert_transaction},
    domain::{Account, Token, Transaction},
    monzo::{
        WebhookResponse, delete_webhook, list_accounts, list_all_transactions, list_webhooks,
        register_webhook as register_webhook_with_monzo,
    },
};

pub async fn list_and_update_accounts(pool: &PgPool, token: &Token) -> Result<(), Box<dyn Error>> {
    let result = list_accounts(&token.access_token).await?;
    tracing::info!("Found accounts: {}", result.len());
    for account_response in result.iter() {
        let account = Account {
            id: account_response.id.clone(),
            user_id: token.user_id.clone(),
            description: account_response.description.clone(),
            created: DateTime::parse_from_rfc3339(&account_response.created)
                .inspect_err(|err| tracing::error!("Error parsing date: {}", err))
                .unwrap()
                .to_utc(),
        };
        tracing::info!(
            "Upserting account id={} for user_id={}",
            &account.id,
            &token.user_id
        );
        let _ = upsert_account(pool, &account).await;
    }
    Ok(())
}

pub async fn register_webhook(
    access_token: &str,
    account_id: &str,
    url: &str,
) -> Result<(), Box<dyn Error>> {
    let webhook_response = list_webhooks(access_token, account_id).await?;

    let webhooks: HashMap<String, WebhookResponse> = webhook_response
        .into_iter()
        .map(|webhook| (webhook.account_id.clone(), webhook))
        .collect();

    match webhooks.get(account_id) {
        Some(WebhookResponse {
            id,
            url: existing_url,
            account_id,
        }) => {
            tracing::info!(
                "Existing webhook found for account_id={}, id={}, url={}",
                &account_id,
                &id,
                &existing_url
            );
            if url != existing_url {
                delete_webhook(access_token, id).await?;
                register_webhook_with_monzo(access_token, account_id, url).await?;
            }
        }
        None => {
            register_webhook_with_monzo(access_token, account_id, url).await?;
        }
    };

    Ok(())
}

pub fn parse_monzo_date(date_str: &str) -> Option<DateTime<Utc>> {
    if date_str == "" {
        None
    } else {
        Some(
            DateTime::parse_from_rfc3339(date_str)
                .inspect_err(|err| tracing::error!("Error parsing date: {}", err))
                .unwrap()
                .to_utc(),
        )
    }
}

pub async fn list_and_update_transactions(
    pool: &PgPool,
    token: &Token,
) -> Result<(), Box<dyn Error>> {
    let account_ids = query_account_ids(pool, &token.user_id).await?;

    tracing::info!("Listing transactions for {} account_ids", account_ids.len());

    let results: Vec<_> = stream::iter(account_ids)
        .map(async |account_id| {
            (
                account_id.clone(),
                list_all_transactions(&token.access_token, &account_id).await,
            )
        })
        .buffered(1)
        .collect()
        .await;

    results.iter().for_each(|(account_id, responses)| {
        tracing::info!(
            "Retrieved {} transactions for account_id={}",
            responses.as_ref().map(|val| val.len()).unwrap_or(0),
            account_id
        )
    });

    let transactions: Vec<_> = results
        .into_iter()
        .filter(|(_, res)| res.is_ok())
        .map(|(account_id, res)| (account_id, res.unwrap()))
        .map(|(account_id, responses)| -> Vec<_> {
            responses
                .iter()
                .map(|res| Transaction {
                    id: res.id.clone(),
                    account_id: account_id.clone(),
                    amount: res.amount.clone(),
                    currency: res.currency.clone(),
                    description: res.description.clone(),
                    notes: res.notes.clone(),
                    merchant: res.merchant.clone(),
                    category: res.category.clone(),
                    created: parse_monzo_date(&res.created).unwrap(),
                    settled: parse_monzo_date(&res.settled),
                })
                .collect()
        })
        .flatten()
        .collect();

    tracing::info!("Upserting {} transactions...", transactions.len());

    stream::iter(transactions)
        .map(async |transaction| upsert_transaction(pool, &transaction).await)
        .buffered(1)
        .collect::<Vec<_>>()
        .await;
    Ok(())
}

pub async fn initial_load_data(state: Arc<AppState>, token: Token) -> () {
    tracing::info!("Loading initial data for user_id={}", &token.user_id);
    let _ = list_and_update_accounts(&state.pool, &token).await;
}
