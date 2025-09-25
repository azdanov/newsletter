use newsletter::{
    config::{AppBaseUrl, DbConfig},
    email_client::EmailClient,
    startup::serve,
};
use secrecy::SecretString;
use sqlx::PgPool;
use std::{env, sync::LazyLock};
use testcontainers::{ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres;
use tokio::{net::TcpListener, task::JoinHandle};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use wiremock::MockServer;

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
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    _server: JoinHandle<()>,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();

            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
}

pub async fn init() -> TestApp {
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

    let email_server = MockServer::start().await;
    let email_config = newsletter::config::EmailConfig {
        base_url: AppBaseUrl(email_server.uri()),
        sender_email: "user@example.com".to_string(),
        authorization_token: SecretString::from("test_token"),
        timeout_milliseconds: 2000,
    };

    let sender_email = email_config.sender().unwrap();
    let timeout = email_config.timeout();
    let email_client = EmailClient::new(
        email_config.base_url.0.clone(),
        sender_email,
        email_config.authorization_token,
        timeout,
    );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_future = serve(listener, pool.clone(), email_client, email_config.base_url)
        .await
        .unwrap()
        .into_future();
    let handle = tokio::spawn(async move {
        let _container = container;
        if let Err(e) = server_future.await {
            panic!("Server failed: {}", e);
        }
    });

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        port,
        db_pool: pool,
        email_server,
        _server: handle,
    }
}
