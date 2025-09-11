use axum::{Router, http::StatusCode, routing::get};

pub fn create_router() -> Router {
    Router::new().route("/health", get(check_health))
}

async fn check_health() -> StatusCode {
    StatusCode::OK
}
