use crate::{domain::NewSubscriber, email_client::EmailClient, startup::AppState};
use anyhow::Context;
use axum::{Form, extract::State, http::StatusCode, response::IntoResponse};
use chrono::Utc;
use rand::Rng;
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[derive(thiserror::Error, Debug)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for SubscribeError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::UnexpectedError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        tracing::error!("Failed to subscribe: {}", self);
        (status_code, self.to_string()).into_response()
    }
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        Self::new(form.email, form.name).map_err(|e| e.to_string())
    }
}

pub async fn subscribe(
    State(AppState {
        db_pool,
        email_client,
        base_url,
    }): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<StatusCode, SubscribeError> {
    let subscriber: NewSubscriber = form.try_into().map_err(SubscribeError::ValidationError)?;

    let mut tx = db_pool
        .begin()
        .await
        .context("Failed to begin a transaction")?;

    let subscriber_id = insert_subscriber(&mut tx, &subscriber)
        .await
        .context("Failed to insert new subscriber")?;
    let subscription_token = generate_subscription_token();
    store_token(&mut tx, subscriber_id, &subscription_token)
        .await
        .context("Failed to store subscription token")?;

    tx.commit().await.context("Failed to commit transaction")?;

    send_confirmation_email(&email_client, subscriber, &base_url.0, &subscription_token)
        .await
        .context("Failed to send confirmation email")?;

    Ok(StatusCode::OK)
}

fn generate_subscription_token() -> String {
    let mut rng = rand::rng();
    std::iter::repeat_with(|| rng.sample(rand::distr::Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br />Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
}

pub async fn insert_subscriber(
    tx: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();

    sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, 'pending_confirmation')
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(&mut **tx)
    .await?;

    Ok(subscriber_id)
}

pub async fn store_token(
    tx: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
