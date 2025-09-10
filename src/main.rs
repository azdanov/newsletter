use axum::{Router, extract::Path, response::IntoResponse, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler))
        .route("/{name}", get(handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000")
        .await
        .unwrap();

    println!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler(name: Option<Path<String>>) -> impl IntoResponse {
    let name = match name {
        Some(Path(name)) => name,
        None => "World".to_string(),
    };
    format!("Hello {}!", name)
}
