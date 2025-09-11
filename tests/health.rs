use newsletter::make_service;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    start_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get("http://127.0.0.1:8000/health")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

async fn start_app() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    tokio::spawn(async {
        axum::serve(listener, make_service()).await.unwrap();
    });
}
