use newsletter::{config::get_config, startup::serve};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_tracing();

    info!("Starting server");

    let config = get_config().expect("failed to read configuration");
    let db_pool = PgPool::connect_with(config.db.connect_options())
        .await
        .expect("failed to connect to postgres");
    let listener = TcpListener::bind(config.app.address()).await?;

    info!("listening on http://{} ", listener.local_addr()?);

    serve(listener, db_pool).await?.await
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::builder()
                .parse("newsletter=info,tower_http=trace,axum::rejection=trace")
                .expect("failed to parse default tracing filter")
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
