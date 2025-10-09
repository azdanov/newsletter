use crate::{
    config::AppBaseUrl,
    email_client::EmailClient,
    routes::{check_health, confirm, publish_newsletter, subscribe},
};
use axum::{
    Router,
    http::{HeaderName, Request},
    routing::{get, post},
    serve::Serve,
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info_span};

const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email_client: Arc<EmailClient>,
    pub base_url: AppBaseUrl,
}

pub async fn serve(
    listener: TcpListener,
    pool: PgPool,
    email_client: EmailClient,
    base_url: AppBaseUrl,
) -> Result<Serve<TcpListener, Router, Router>, anyhow::Error> {
    let app_state = AppState {
        db_pool: pool.clone(),
        email_client: Arc::new(email_client),
        base_url,
    };
    let app = Router::new()
        .route("/health", get(check_health))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .with_state(app_state);

    let app = add_tracing(app);

    Ok(axum::serve(listener, app))
}

pub fn add_tracing(app: Router) -> Router {
    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let request_id = request.headers().get(REQUEST_ID_HEADER);
                match request_id {
                    Some(request_id) => info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        request_id = ?request_id,
                    ),
                    None => {
                        error!("could not extract request_id");
                        info_span!(
                           "http_request",
                            method = %request.method(),
                            uri = %request.uri(),
                        )
                    }
                }
            }),
        )
        .layer(PropagateRequestIdLayer::new(x_request_id));

    app.layer(middleware)
}
