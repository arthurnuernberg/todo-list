use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};

pub fn routes() -> Router {
    // Beispiel-Daten
    let todos = Arc::new(Mutex::new(vec![
        ToDo::new(1, "X aufkaufen".to_string(), None, true),
        ToDo::new(
            2,
            "Kaffee kochen".to_string(),
            Some("Filterkaffee...".to_string()),
            false,
        ),
    ]));

    let app_state = AppState {
        todos,
        next_id: Arc::new(Mutex::new(4)),
        title: Arc::new(Mutex::new(String::from("To-dos"))),
    };

    Router::new()
        .route("/", get(tasks))
        .route("/tick", post(tick_task))
        .route("/new_task", post(new_task))
        .route("/delete_task", post(delete_task))
        .route("/update_title", post(update_title))
        .route("/update_date", post(update_due_date))
        .with_state(app_state) // AppState hier binden
}

async fn tasks(State(state): State<AppState>) -> Html<String> {
    let todos = state.todos.lock().unwrap(); // Zugriff auf die gemeinsam genutzte ToDo-Liste
    let title = state.title.lock().unwrap();

    // Tera-Instanz erstellen
    let tera = Tera::new("src/templates/**/*").unwrap();

    // Kontext erstellen und Daten hinzufügen
    let mut context = Context::new();

    context.insert("tasks", &*todos);
    context.insert("title", &*title);

    // Template rendern
    let rendered = tera.render("tasks.html", &context).unwrap();
    Html(rendered)
}

// Form-Daten
#[derive(Deserialize)]
struct TaskForm {
    task_id: u32,
}

#[derive(Deserialize)]
struct ChangeTaskForm {
    task_title: String,
    task_description: Option<String>,
}

#[derive(Deserialize)]
struct TitleUpdate {
    title: String,
}

#[derive(Deserialize)]
struct DueDateUpdate {
    task_id: u32,
    due_date: DateTime<Local>,
}

// POST-Handler für /tick
async fn tick_task(State(state): State<AppState>, Form(input): Form<TaskForm>) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    if let Some(task) = todos.iter_mut().find(|t| t.id == input.task_id) {
        task.tick();
    }

    Redirect::to("/")
}

async fn new_task(State(state): State<AppState>, Form(input): Form<ChangeTaskForm>) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    if input.task_title.is_empty() {
        return Redirect::to("/");
    }
    let new_todo: ToDo = ToDo::new(
        state.generate_id(),
        input.task_title,
        input.task_description,
        false,
    );
    println!("ID: {}\nTitel: {}", new_todo.id, new_todo.title);
    todos.insert(0, new_todo);
    Redirect::to("/")
}

async fn delete_task(State(state): State<AppState>, form: Form<TaskForm>) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    if let Some(pos) = todos.iter().position(|todo| todo.id == form.task_id) {
        todos.remove(pos);
    }
    Redirect::to("/")
}

async fn update_title(State(state): State<AppState>, Json(payload): Json<TitleUpdate>) -> Redirect {
    let mut title = state.title.lock().unwrap();
    *title = payload.title;
    Redirect::to("/")
}

async fn update_due_date(
    State(state): State<AppState>,
    Form(input): Form<DueDateUpdate>,
) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    if let Some(task) = todos.iter_mut().find(|t| t.id == input.task_id) {
        task.due_date = Some(input.due_date);
    }
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
    title: Arc<Mutex<String>>,
}

#[allow(dead_code)]
impl AppState {
    fn new() -> Self {
        AppState {
            todos: Arc::new(Mutex::new(Vec::new())),
            next_id: Arc::new(Mutex::new(1)),
            title: Arc::new(Mutex::new(String::from("To-dos"))),
        }
    }

    fn generate_id(&self) -> u32 {
        let mut id = self.next_id.lock().unwrap();
        let current_id = *id;
        *id += 1;
        current_id
    }
}
