use newsletter::{config::DbConfig, startup::serve};
use reqwest::Client;
use sqlx::PgPool;
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::{self, Postgres};
use tokio::net::TcpListener;

pub struct TestApp {
    pub url: String,
    pub db_pool: PgPool,
    pub container: ContainerAsync<Postgres>,
}

async fn init() -> TestApp {
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
        password: "postgres".to_string(),
        db_name: "newsletter".to_string(),
    };
    let pool = PgPool::connect(&db_config.connection_string())
        .await
        .unwrap();

    sqlx::migrate!().run(&pool).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server = serve(listener, pool.clone()).await.unwrap().into_future();
    tokio::spawn(server);

    TestApp {
        url: format!("http://127.0.0.1:{}", port),
        db_pool: pool,
        container,
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

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
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
            "when the body is {}",
            error_message
        );
    }
}
