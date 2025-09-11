use axum::{Router, http::StatusCode, routing::get, serve::Serve};
use tokio::net::TcpListener;

pub fn serve(listener: TcpListener) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new().route("/health", get(check_health));

    Ok(axum::serve(listener, app))
}

async fn check_health() -> StatusCode {
    StatusCode::OK
}
