use newsletter::serve;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;

    println!("listening on http://{} ", listener.local_addr()?);

    serve(listener)?.await
}
