use crate::{domain::NewSubscriber, email_client::EmailClient, startup::AppState};
use axum::{Form, extract::State, http::StatusCode};
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
) -> StatusCode {
    let subscriber: NewSubscriber = match form.try_into() {
        Ok(form) => form,
        Err(e) => {
            tracing::error!("Failed to parse form data: {}", e);
            return StatusCode::BAD_REQUEST;
        }
    };
    let mut tx = match db_pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let subscriber_id = match insert_subscriber(&mut tx, &subscriber).await {
        Ok(subscriber_id) => subscriber_id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };
    let subscription_token = generate_subscription_token();
    if store_token(&mut tx, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if tx.commit().await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if send_confirmation_email(&email_client, subscriber, &base_url.0, &subscription_token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    StatusCode::OK
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
) -> Result<(), anyhow::Error> {
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
) -> Result<Uuid, anyhow::Error> {
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
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(subscriber_id)
}

pub async fn store_token(
    tx: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
    INSERT INTO subscription_tokens (subscription_token, subscriber_id)
    VALUES ($1, $2)
        "#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
