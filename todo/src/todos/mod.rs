use axum::Router;

pub mod todos;
pub mod users;
mod todo;
mod forms;
mod lib;
mod filter;

#[allow(dead_code)]
pub async fn app_router() -> Router {
    Router::new()
        .merge(todos::routes().await) // Routen für Aufgaben
        .merge(users::routes()) // Routen für Nutzer
}
