use axum::{response::Html, routing::get, Router};

pub fn routes() -> Router {
    Router::new().route("/users", get(list_users)) // Nutzer anzeigen
}

async fn list_users() -> Html<&'static str> {
    Html("<h2>Nutzerübersicht</h2>")
}
