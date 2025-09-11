use newsletter::create_router;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let url = start_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health", &url))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

async fn start_app() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    // No cleanup is necessary, since tokio will do it once runtime is done.
    tokio::spawn(async {
        axum::serve(listener, create_router()).await.unwrap();
    });

    format!("http://127.0.1:{}", port)
}
