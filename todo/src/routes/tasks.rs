use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};

pub fn routes() -> Router {
    // Beispiel-Daten initialisieren
    let todos = Arc::new(Mutex::new(vec![
        ToDo::new(1, "Learn Rust".to_string(), None, false),
        ToDo::new(2, "Write Axum app".to_string(), None, false),
    ]));

    let app_state = AppState {
        todos,
        next_id: Arc::new(Mutex::new(3)),
    };

    Router::new()
        .route("/", get(tasks))
        .route("/tick", post(tick_task))
        .route("/new_task", post(new_task))
        .with_state(app_state) // AppState hier binden
}

async fn tasks(State(state): State<AppState>) -> Html<String> {
    let todos = state.todos.lock().unwrap(); // Zugriff auf die gemeinsam genutzte ToDo-Liste

    // Tera-Instanz erstellen
    let tera = Tera::new("src/templates/**/*").unwrap();

    // Kontext erstellen und Daten hinzufügen
    let mut context = Context::new();
    context.insert("tasks", &*todos);

    // Template rendern
    let rendered = tera.render("tasks.html", &context).unwrap();
    Html(rendered)
}

// Form-Daten
#[derive(Deserialize)]
struct TickForm {
    task_id: u32,
}

#[derive(Deserialize)]
struct ChangeTaskForm {
    task_title: String,
    task_description: Option<String>,
    // task_completed: bool,
}

// POST-Handler für /tick
async fn tick_task(State(state): State<AppState>, Form(input): Form<TickForm>) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    if let Some(task) = todos.iter_mut().find(|t| t.id == input.task_id) {
        task.tick();
    }

    Redirect::to("/")
}

async fn new_task(State(state): State<AppState>, Form(input): Form<ChangeTaskForm>) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    let new_todo: ToDo = ToDo::new(
        state.generate_id(),
        input.task_title,
        input.task_description,
        false,
    );
    println!(
        "ID: {}\nTitel: {}",
        new_todo.id, new_todo.title
    );
    todos.insert(0, new_todo);
    Redirect::to("/")
}

// ToDo-Struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToDo {
    id: u32,
    title: String,
    description: Option<String>,
    due_date: Option<DateTime<Local>>, // Fälligkeitsdatum
    created_at: DateTime<Local>,       // Erstellungsdatum
    completed: bool,
}

impl ToDo {
    fn new(id: u32, title: String, description: Option<String>, completed: bool) -> Self {
        ToDo {
            id,
            title,
            description,
            due_date: None,
            created_at: Local::now(),
            completed,
        }
    }

    fn tick(&mut self) -> &ToDo {
        self.completed = !self.completed;
        self
    }
}

#[derive(Clone)]
struct AppState {
    todos: Arc<Mutex<Vec<ToDo>>>,
    next_id: Arc<Mutex<u32>>,
}

#[allow(dead_code)]
impl AppState {
    fn new() -> Self {
        AppState {
            todos: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    fn generate_id(&self) -> u32 {
        let mut id = self.next_id.lock().unwrap();
        let current_id = *id;
        *id += 1;
        current_id
    }
}
