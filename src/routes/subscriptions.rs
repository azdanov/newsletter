use axum::{Form, extract::State, http::StatusCode};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::NewSubscriber;

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

pub async fn create_subscription(
    State(pool): State<PgPool>,
    Form(form): Form<FormData>,
) -> StatusCode {
    let subscriber: NewSubscriber = match form.try_into() {
        Ok(form) => form,
        Err(e) => {
            tracing::error!("Failed to parse form data: {}", e);
            return StatusCode::BAD_REQUEST;
        }
    };

    match sqlx::query!(
        "INSERT INTO subscriptions (id, email, name, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')",
        Uuid::now_v7(),
        subscriber.email.as_ref(),
        subscriber.name.as_ref(),
        Utc::now()
    )
    .execute(&pool)
    .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("Failed to execute query: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
