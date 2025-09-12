use newsletter::{config::get_config, startup::serve};
use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = get_config().expect("failed to read configuration");
    let db_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("failed to connect to postgres");
    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.app_port)).await?;

    println!("listening on http://{} ", listener.local_addr()?);

    serve(listener, db_pool).await?.await
}
