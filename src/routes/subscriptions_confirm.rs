use crate::startup::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

pub async fn confirm(
    Query(params): Query<Parameters>,
    State(AppState { db_pool, .. }): State<AppState>,
) -> StatusCode {
    let subscriber_id =
        match get_subscriber_id_from_token(&db_pool, &params.subscription_token).await {
            Ok(id) => id,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
        };
    match subscriber_id {
        None => StatusCode::UNAUTHORIZED,
        Some(subscriber_id) => {
            if confirm_subscriber(&db_pool, subscriber_id).await.is_err() {
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
            StatusCode::OK
        }
    }
}

async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, anyhow::Error> {
    let result = sqlx::query!(
        r#"SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = $1"#,
        subscription_token,
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}
