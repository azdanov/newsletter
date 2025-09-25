use newsletter::{config::get_config, email_client::EmailClient, startup::serve};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_tracing();

    info!("Starting server");

    let config = get_config()?;
    let db_pool = PgPool::connect_with(config.db.connect_options()).await?;
    let sender_email = config.email.sender()?;
    let timeout = config.email.timeout();
    let base_url = config.email.base_url;
    let email_client = EmailClient::new(
        base_url.0.clone(),
        sender_email,
        config.email.authorization_token,
        timeout,
    );
    let listener = TcpListener::bind(config.app.address()).await?;

    info!("listening on http://{} ", listener.local_addr()?);

    Ok(serve(listener, db_pool, email_client, base_url)
        .await?
        .await?)
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
