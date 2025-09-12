use crate::routes::{check_health, create_subscription};
use axum::{
    Router,
    routing::{get, post},
    serve::Serve,
};
use sqlx::PgPool;
use tokio::net::TcpListener;

pub async fn serve(
    listener: TcpListener,
    pool: PgPool,
) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health", get(check_health))
        .route("/subscriptions", post(create_subscription))
        .with_state(pool);

    Ok(axum::serve(listener, app))
}
