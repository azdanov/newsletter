use axum::{
    Form, Router,
    http::StatusCode,
    routing::{get, post},
    serve::Serve,
};
use serde::Deserialize;
use tokio::net::TcpListener;

pub fn serve(listener: TcpListener) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health", get(check_health))
        .route("/subscriptions", post(subscribe));

    Ok(axum::serve(listener, app))
}

async fn check_health() -> StatusCode {
    StatusCode::OK
}

#[derive(Deserialize)]
pub struct Subscription {
    email: String,
    name: String,
}

async fn subscribe(Form(form): Form<Subscription>) -> StatusCode {
    println!("Received subscription: {} <{}>", form.name, form.email);
    StatusCode::OK
}
