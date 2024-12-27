mod routes;

use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = routes::app_router();

    // Server starten
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server läuft auf https://{}", addr);
    hyper::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
