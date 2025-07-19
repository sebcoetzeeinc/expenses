use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row, postgres::PgQueryResult};

use crate::domain::{Account, Token, Transaction};

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPool::connect(database_url).await
}

pub async fn upsert_token(pool: &PgPool, token: &Token) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query(
        "
            INSERT INTO tokens (
                user_id,
                expiry_time,
                token_type,
                access_token,
                refresh_token
            ) VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id)
            DO UPDATE SET
                expiry_time = EXCLUDED.expiry_time,
                token_type = EXCLUDED.token_type,
                access_token = EXCLUDED.access_token,
                refresh_token = EXCLUDED.refresh_token
        ",
    )
    .bind(&token.user_id)
    .bind(&token.expiry_time)
    .bind(&token.token_type)
    .bind(&token.access_token)
    .bind(&token.refresh_token)
    .execute(pool)
    .await
}

pub async fn upsert_account(
    pool: &PgPool,
    account: &Account,
) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query(
        "
            INSERT INTO accounts (
                id,
                user_id,
                description,
                created
            ) VALUES ($1, $2, $3, $4)
            ON CONFLICT (id)
            DO UPDATE SET
                user_id = EXCLUDED.user_id,
                description = EXCLUDED.description,
                created = EXCLUDED.created
        ",
    )
    .bind(&account.id)
    .bind(&account.user_id)
    .bind(&account.description)
    .bind(&account.created)
    .execute(pool)
    .await
}

pub async fn upsert_transaction(
    pool: &PgPool,
    transaction: &Transaction,
) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query(
        "
            INSERT INTO transactions (
                id,
                account_id,
                amount,
                currency,
                description,
                notes,
                merchant,
                category,
                created,
                settled
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id)
            DO UPDATE SET
                account_id = EXCLUDED.account_id,
                amount = EXCLUDED.amount,
                currency = EXCLUDED.currency,
                description = EXCLUDED.description,
                notes = EXCLUDED.notes,
                merchant = EXCLUDED.merchant,
                category = EXCLUDED.category,
                created = EXCLUDED.created,
                settled = EXCLUDED.settled
        ",
    )
    .bind(&transaction.id)
    .bind(&transaction.account_id)
    .bind(&transaction.amount)
    .bind(&transaction.currency)
    .bind(&transaction.description)
    .bind(&transaction.notes)
    .bind(&transaction.merchant)
    .bind(&transaction.category)
    .bind(&transaction.created)
    .bind(&transaction.settled)
    .execute(pool)
    .await
    .inspect_err(|err| {
        tracing::error!(
            "Failed to upsert transaction id={} account_id={}: {}",
            &transaction.id,
            &transaction.account_id,
            err
        );
    })
}

pub async fn query_account_ids(pool: &PgPool, user_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let account_ids: Vec<_> = sqlx::query(
        "
            SELECT id FROM accounts
                WHERE user_id = $1
        ",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get::<String, &str>("id"))
    .collect();

    Ok(account_ids)
}

pub async fn query_all_tokens(pool: &PgPool) -> Result<Vec<Token>, sqlx::Error> {
    sqlx::query_as::<_, Token>(
        "
            SELECT * FROM tokens
        ",
    )
    .fetch_all(pool)
    .await
}

pub async fn query_tokens_expiring_before(
    pool: &PgPool,
    expiry_time: DateTime<Utc>,
) -> Result<Vec<Token>, sqlx::Error> {
    sqlx::query_as::<_, Token>(
        "
            SELECT * FROM tokens
            WHERE expiry_time < $1
        ",
    )
    .bind(&expiry_time)
    .fetch_all(pool)
    .await
}

pub async fn query_transactions(
    pool: &PgPool,
    account_ids: &Vec<String>,
) -> Result<Vec<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(
        "
            SELECT * FROM transactions 
            WHERE account_id = ANY($1)
            ORDER BY created DESC
        ",
    )
    .bind(&account_ids)
    .fetch_all(pool)
    .await
}
