mod todos;
mod lib;

use std::net::SocketAddr;
use axum::http::StatusCode;
use axum::routing::get_service;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let app = todos::app_router().await.nest_service(
        "/static",
        get_service(ServeDir::new("src/templates/static")).handle_error(|error: std::io::Error| async move {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Something went wrong: {}", error),
            )
        }),
    );

    // Server starten
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server läuft");
    hyper::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
