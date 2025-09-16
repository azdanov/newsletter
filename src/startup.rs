use crate::routes::{check_health, create_subscription};
use axum::{
    Router,
    http::{HeaderName, Request},
    routing::{get, post},
    serve::Serve,
};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info_span};

const REQUEST_ID_HEADER: &str = "x-request-id";

pub async fn serve(
    listener: TcpListener,
    pool: PgPool,
) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health", get(check_health))
        .route("/subscriptions", post(create_subscription))
        .with_state(pool);

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
