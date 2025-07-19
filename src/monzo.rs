use std::collections::HashMap;

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::Token;

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub refresh_token: String,
    pub token_type: String,
    pub user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AccountResponse {
    pub id: String,
    pub description: String,
    pub created: String,
}

#[derive(Debug, Deserialize)]
struct ListAccountsReponse {
    accounts: Vec<AccountResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Merchant {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransactionResponse {
    pub id: String,
    pub amount: i64,
    pub created: String,
    pub currency: String,
    pub description: String,
    pub notes: String,
    pub is_load: bool,
    pub settled: String,
    pub category: String,
    pub merchant: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransactionRequest {
    pub id: String,
    pub amount: i64,
    pub created: String,
    pub currency: String,
    pub description: String,
    pub notes: String,
    pub is_load: bool,
    pub settled: String,
    pub category: String,
    pub account_id: String,
    pub merchant: Option<Merchant>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ListTransactionsReponse {
    transactions: Vec<TransactionResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WebhookResponse {
    pub account_id: String,
    pub id: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ListWebhooksResponse {
    webhooks: Vec<WebhookResponse>,
}

pub async fn exchange_auth_code(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<TokenResponse, reqwest::Error> {
    let client = reqwest::Client::new();

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("client_id", &client_id);
    params.insert("client_secret", &client_secret);
    params.insert("redirect_uri", &redirect_uri);
    params.insert("code", &code);

    let res = client
        .post("https://api.monzo.com/oauth2/token")
        .form(&params)
        .send()
        .await
        .inspect_err(|err| {
            tracing::error!("error occurred in request to monzo token api: {:#?}", err);
        })?;

    let token_response = res.json::<TokenResponse>().await.inspect_err(|err| {
        tracing::error!(
            "error occurred while deserialising token response: {:#?}",
            err
        );
    })?;

    Ok(token_response)
}

pub async fn register_webhook(
    access_token: &str,
    account_id: &str,
    url: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();

    tracing::info!(
        "Registering webhook for account_id={} url={}",
        account_id,
        url
    );

    let mut params = HashMap::new();
    params.insert("account_id", account_id);
    params.insert("url", url);

    client
        .post("https://api.monzo.com/webhooks")
        .bearer_auth(access_token)
        .form(&params)
        .send()
        .await
        .inspect_err(|err| {
            tracing::error!("Error occurred in request to Monzo token API: {:#?}", err)
        })
}

pub async fn delete_webhook(
    access_token: &str,
    id: &str,
) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();

    tracing::info!("Deleting webhook id={}", id);

    client
        .delete(format!("https://api.monzo.com/webhooks/{}", id))
        .bearer_auth(access_token)
        .send()
        .await
        .inspect_err(|err| {
            tracing::error!("Error occurred in request to Monzo webhook API: {:#?}", err)
        })
}
pub async fn refresh_tokens(
    tokens: Vec<Token>,
    client_id: &str,
    client_secret: &str,
) -> Vec<TokenResponse> {
    let client = reqwest::Client::new();
    let mut token_responses = Vec::<TokenResponse>::new();

    for token in tokens.iter() {
        tracing::info!("Refreshing token for user_id={}", &token.user_id);

        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("client_id", &client_id);
        params.insert("client_secret", &client_secret);
        params.insert("refresh_token", &token.refresh_token);

        let res = match client
            .post("https://api.monzo.com/oauth2/token")
            .form(&params)
            .send()
            .await
        {
            Ok(res) => res,
            Err(err) => {
                tracing::error!("Error occurred in request to Monzo token API: {:#?}", err);
                continue;
            }
        };

        match res.json::<TokenResponse>().await {
            Ok(token_response) => {
                token_responses.push(token_response);
            }
            Err(err) => {
                tracing::error!(
                    "Error occurred while deserialising token response: {:#?}",
                    err
                )
            }
        }
    }

    return token_responses;
}

pub async fn list_webhooks(
    access_token: &str,
    account_id: &str,
) -> Result<Vec<WebhookResponse>, reqwest::Error> {
    let client = reqwest::Client::new();

    tracing::info!("Listing webhooks for account_id={}", account_id);

    let res = client
        .get("https://api.monzo.com/webhooks")
        .bearer_auth(access_token)
        .query(&[("account_id", account_id)])
        .send()
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Error occurred in request to Monzo webhooks API: {:#?}",
                err
            )
        })?;

    res.json::<ListWebhooksResponse>()
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Error occurred while deserialising webhooks response: {:#?}",
                err
            )
        })
        .map(|res| res.webhooks)
}

pub async fn list_accounts(access_token: &str) -> Result<Vec<AccountResponse>, reqwest::Error> {
    let client = reqwest::Client::new();

    tracing::info!("Listing accounts...");

    let res = client
        .get("https://api.monzo.com/accounts")
        .bearer_auth(access_token)
        .send()
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Error occurred in request to Monzo accounts API: {:#?}",
                err
            )
        })?;

    res.json::<ListAccountsReponse>()
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Error occurred while deserialising account response: {:#?}",
                err
            )
        })
        .map(|res| res.accounts)
}

pub async fn list_transactions(
    access_token: &str,
    account_id: &str,
    before: Option<&str>,
) -> Result<Vec<TransactionResponse>, reqwest::Error> {
    tracing::info!("Listing transactions for account_id={}", account_id);

    let client = reqwest::Client::new();

    let mut params = vec![("account_id", account_id), ("limit", "100")];

    if before.is_some() {
        params.push(("before", before.unwrap()));
    }

    let res = client
        .get("https://api.monzo.com/transactions")
        .bearer_auth(access_token)
        .query(&params)
        .send()
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Error occurred in request to Monzo transaction API: {:#?}",
                err
            )
        })?;

    tracing::info!("Returned code: {}", res.status());

    // For some reason Monzo returns a 403 if you request a transaction before a time that you are
    // allowed to request one for.
    if before.is_some() && res.status().as_u16() == 403 {
        return Ok(vec![]);
    }

    res.json::<ListTransactionsReponse>()
        .await
        .inspect_err(|err| {
            tracing::error!(
                "Error occurred while deserialising transactions response: {:#?}",
                err
            )
        })
        .map(|res| res.transactions)
}

pub async fn list_all_transactions(
    access_token: &str,
    account_id: &str,
) -> Result<Vec<TransactionResponse>, reqwest::Error> {
    let mut transactions = vec![];
    let mut before: Option<String> = Some((Utc::now() + Duration::days(1)).to_rfc3339());
    loop {
        let batch = list_transactions(access_token, account_id, before.as_deref()).await?;
        if let Some(transaction) = batch.get(0) {
            let created = &transaction.created;
            if created == "" {
                before = None;
            } else {
                before = Some(created.clone());
            }
        } else {
            break;
        }
        transactions.extend(batch.into_iter());
    }
    Ok(transactions)
}
