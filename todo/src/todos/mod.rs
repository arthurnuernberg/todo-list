use axum::Router;

pub mod tasks;
pub mod users;
pub mod todo;
pub mod forms;
pub mod filter;
pub mod db;
mod old_state;
mod export;

#[allow(dead_code)]
pub async fn app_router() -> Router {
    Router::new()
        .merge(tasks::routes().await) // Routen für Aufgaben
        .merge(users::routes()) // Routen für Nutzer
}
