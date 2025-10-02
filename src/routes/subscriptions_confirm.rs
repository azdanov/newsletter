use crate::startup::AppState;
use anyhow::Context;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfirmationError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("There is no subscriber associated with the provided token.")]
    UnknownToken,
}

impl IntoResponse for ConfirmationError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            ConfirmationError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ConfirmationError::UnknownToken => StatusCode::UNAUTHORIZED,
        };
        tracing::error!("Failed to confirm subscription: {}", self);
        (status_code, self.to_string()).into_response()
    }
}

pub async fn confirm(
    Query(params): Query<Parameters>,
    State(AppState { db_pool, .. }): State<AppState>,
) -> Result<StatusCode, ConfirmationError> {
    let subscriber_id = get_subscriber_id_from_token(&db_pool, &params.subscription_token)
        .await
        .context("Failed to retrieve subscriber id from token")?
        .ok_or(ConfirmationError::UnknownToken)?;

    confirm_subscriber(&db_pool, subscriber_id)
        .await
        .context("Failed to confirm subscriber")?;

    Ok(StatusCode::OK)
}

async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = 'confirmed' WHERE id = $1"#,
        subscriber_id,
    )
    .execute(db_pool)
    .await?;

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
    .await?;

    Ok(result.map(|r| r.subscriber_id))
}
