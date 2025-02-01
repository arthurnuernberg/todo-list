use axum::extract::Query;
use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDateTime;
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
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
        .route("/update_task", post(update_task))
        .with_state(app_state) // AppState hier binden
}

#[derive(Debug, Deserialize)]
pub struct AllFilters {
    #[serde(deserialize_with = "deserialize_optional_bool")]
    #[serde(default)]
    pub completed: Option<bool>,

    #[serde(deserialize_with = "deserialize_optional_bool")]
    #[serde(default)]
    pub is_due: Option<bool>,

    #[serde(deserialize_with = "deserialize_optional_datetime")]
    #[serde(default)]
    pub start_date: Option<NaiveDateTime>,

    #[serde(deserialize_with = "deserialize_optional_datetime")]
    #[serde(default)]
    pub end_date: Option<NaiveDateTime>,

    #[serde(default)]
    pub query: Option<String>,
}

fn deserialize_optional_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s.as_deref() {
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some("") | None => Ok(None),
        _ => Err(serde::de::Error::custom(
            "Invalid boolean value, expected 'true' or 'false'",
        )),
    }
}

fn deserialize_optional_datetime<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        if s.is_empty() {
            return Ok(None);
        }
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M")
            .map(Some)
            .map_err(serde::de::Error::custom)
    } else {
        Ok(None)
    }
}

async fn tasks(State(state): State<AppState>, Query(filters): Query<AllFilters>) -> Html<String> {
    let mut todos = state.todos.lock().unwrap(); // Zugriff auf die gemeinsam genutzte ToDo-Liste
    todos.iter_mut().for_each(|todo| {
        todo.check_overdue();
    });
    let title = state.title.lock().unwrap();
    let now_string = {
        let now = Local::now();
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
        )
    };

    let mut filtered_todos = todos.clone();

    if let Some(completed) = filters.completed {
        filtered_todos = filtered_todos
            .into_iter()
            .filter(|todo| todo.completed == completed)
            .collect();
    }

    if let (Some(start), Some(end)) = (filters.start_date, filters.end_date) {
        filtered_todos = filter_by_date_range(&filtered_todos, Some(start), Some(end));
    }

    if let Some(query) = &filters.query {
        filtered_todos = filtered_todos
            .into_iter()
            .filter(|todo| todo.title.to_lowercase().contains(&query.to_lowercase()))
            .collect();
    }

    if let Some(true) = filters.is_due {
        filtered_todos = filtered_todos
            .into_iter()
            .filter(|todo| todo.due_date.map(|d| d <= Local::now()).unwrap_or(false))
            .collect();
    }

    // Tera-Instanz erstellen
    let tera = Tera::new("src/templates/**/*").unwrap();

    // Kontext erstellen und Daten hinzufügen
    let mut context = Context::new();

    context.insert("tasks", &*filtered_todos);
    context.insert("title", &*title);
    context.insert("now_string", &now_string);

    // Template rendern
    let rendered = tera.render("tasks.html", &context).unwrap();
    Html(rendered)
}

fn filter_by_date_range(
    todos: &Vec<ToDo>,
    start_date: Option<NaiveDateTime>,
    end_date: Option<NaiveDateTime>,
) -> Vec<ToDo> {
    todos
        .iter()
        .filter(|todo| {
            if let Some(start) = start_date {
                if let Some(end) = end_date {
                    if let Some(due_date) = todo.due_date {
                        return due_date.naive_local() >= start && due_date.naive_local() <= end;
                    }
                }
            }
            false
        })
        .cloned()
        .collect()
}

// Form-Daten
#[derive(Deserialize)]
struct TaskForm {
    task_id: u32,
}

#[derive(Deserialize)]
struct ChangeTaskForm {
    task_id: u32,
    task_title: String,
    task_description: String,
}

#[derive(Deserialize)]
struct TitleUpdate {
    title: String,
}

#[derive(Deserialize)]
struct DueDateUpdate {
    task_id: u32,
    due_date: String,
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
    let mut description: Option<String> = None;
    if !input.task_description.is_empty() {
        description = Some(input.task_description)
    };

    let new_todo: ToDo = ToDo::new(state.generate_id(), input.task_title, description, false);
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
    if !payload.title.is_empty() {
        *title = payload.title;
    }
    Redirect::to("/")
}

async fn update_task(
    State(state): State<AppState>,
    Json(payload): Json<ChangeTaskForm>,
) -> Redirect {
    let mut todos = state.todos.lock().unwrap();
    if let Some(task) = todos.iter_mut().find(|t| t.id == payload.task_id) {
        task.title = {
            if !payload.task_title.is_empty() {
                payload.task_title
            } else {
                task.title.clone()
            }
        };
        task.description = {
            if payload.task_description.is_empty() {
                None
            } else {
                Some(payload.task_description)
            }
        };
    }
    Redirect::to("/")
}

async fn update_due_date(
    State(state): State<AppState>,
    Form(input): Form<DueDateUpdate>,
) -> Redirect {
    // Versuch manuelles Parsen
    use chrono::{Local, NaiveDateTime};

    let naive_dt = NaiveDateTime::parse_from_str(&input.due_date, "%Y-%m-%dT%H:%M");
    if let Ok(dt) = naive_dt {
        let local_dt = Local.from_local_datetime(&dt).unwrap();

        let mut todos = state.todos.lock().unwrap();
        if let Some(task) = todos.iter_mut().find(|t| t.id == input.task_id) {
            task.due_date = Some(local_dt);
        }
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
    is_overdue: bool,
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
            is_overdue: false,
        }
    }

    fn tick(&mut self) -> &ToDo {
        self.completed = !self.completed;
        self
    }

    fn check_overdue(&mut self) {
        self.is_overdue = match self.due_date {
            Some(d) => d <= Local::now(),
            None => false,
        }
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
