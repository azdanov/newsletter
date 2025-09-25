use crate::api::helpers::init;

#[tokio::test]
async fn health_works() {
    // Arrange
    let app = init().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
