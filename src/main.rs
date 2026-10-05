use axum::{Router, routing};

mod packet;
mod bead;

#[tokio::main]
async fn main()
{
    let router = Router::new()
        .route("/", routing::get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}