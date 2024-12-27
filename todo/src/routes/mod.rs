use axum::Router;

pub mod tasks;
pub mod users;

#[allow(dead_code)]
pub fn app_router() -> Router {
    Router::new()
        .merge(tasks::routes()) // Routen für Aufgaben
        .merge(users::routes()) // Routen für Nutzer
}
