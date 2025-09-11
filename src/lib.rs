use axum::{Router, http::StatusCode, routing::get};

pub fn make_service() -> Router {
    Router::new().route("/health", get(check_health))
}

async fn check_health() -> StatusCode {
    StatusCode::OK
}
