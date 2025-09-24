use std::{env, sync::LazyLock};

use newsletter::{config::DbConfig, startup::serve};
use reqwest::Client;
use secrecy::SecretString;
use sqlx::PgPool;
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres;
use tokio::{net::TcpListener, task::JoinHandle};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let log_tests = env::var("LOG_TESTS").unwrap_or_default() == "true";
    let filter = if log_tests {
        "newsletter=info,tower_http=trace,axum::rejection=trace".to_string()
    } else {
        "off".to_string()
    };

    tracing_subscriber::registry()
        .with(EnvFilter::new(filter))
        .with(tracing_subscriber::fmt::layer())
        .init();
});

pub struct TestApp {
    pub url: String,
    pub db_pool: PgPool,
    _server: JoinHandle<()>,
}

async fn init() -> TestApp {
    LazyLock::force(&TRACING);
    let container = postgres::Postgres::default()
        .with_db_name("newsletter")
        .with_tag("17.6")
        .start()
        .await
        .unwrap();
    let db_config = DbConfig {
        host: "127.0.0.1".to_string(),
        port: container.get_host_port_ipv4(5432).await.unwrap(),
        username: "postgres".to_string(),
        password: SecretString::from("postgres"),
        db_name: "newsletter".to_string(),
        require_ssl: false,
    };
    let pool = PgPool::connect_with(db_config.connect_options())
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_future = serve(listener, pool.clone()).await.unwrap().into_future();
    let handle = tokio::spawn(async move {
        let _container = container;
        if let Err(e) = server_future.await {
            panic!("Server failed: {}", e);
        }
    });

    TestApp {
        url: format!("http://127.0.0.1:{}", port),
        db_pool: pool,
        _server: handle,
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self._server.abort();
    }
}

#[tokio::test]
async fn health_returns_a_200() {
    // Arrange
    let app = init().await;
    let client = Client::new();

    // Act
    let response = client
        .get(format!("{}/health", &app.url))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = init().await;
    let client = Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Act
    let response = client
        .post(format!("{}/subscriptions", &app.url))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request");

    // Assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_422_when_data_is_missing() {
    // Arrange
    let app = init().await;
    let client = Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(format!("{}/subscriptions", &app.url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("failed to execute request");

        // Assert
        assert_eq!(
            422,
            response.status().as_u16(),
            "{} should return a 422 Unprocessable Entity",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = init().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = client
            .post(format!("{}/subscriptions", &app.url))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "{} should return 400 Bad Request",
            description
        );
    }
}
