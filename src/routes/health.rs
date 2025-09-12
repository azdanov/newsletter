use axum::http::StatusCode;

pub async fn check_health() -> StatusCode {
    StatusCode::OK
}
