use axum::extract::Query;
use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::{get, post},
    Form, Json, Router,
};
use chrono::NaiveDateTime;
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};
use uuid::Uuid;

pub type TodoListId = String;

#[derive(Clone)]
pub struct AppState {
    pub current_list_id: Arc<Mutex<TodoListId>>,
    pub lists: Arc<Mutex<HashMap<TodoListId, TodoList>>>,
    pub tags: Arc<Mutex<HashSet<Tag>>>,
}

#[derive(Debug, Clone)]
pub struct TodoList {
    pub id: Arc<Mutex<TodoListId>>,
    pub todos: Arc<Mutex<Vec<Todo>>>,
    pub next_id: Arc<Mutex<u32>>,
    pub title: Arc<Mutex<String>>,
}

// ToDo-Struct
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Todo {
    id: u32,
    title: String,
    description: Option<String>,
    due_date: Option<DateTime<Local>>, // Fälligkeitsdatum
    created_at: DateTime<Local>,       // Erstellungsdatum
    completed: bool,
    is_overdue: bool,
    tags: HashSet<Tag>,
}

#[derive(Debug, Deserialize)]
pub struct AllFilters {
    #[serde(deserialize_with = "deserialize_optional_bool")]
    #[serde(default)]
    completed: Option<bool>,

    #[serde(deserialize_with = "deserialize_optional_bool")]
    #[serde(default)]
    is_due: Option<bool>,

    #[serde(deserialize_with = "deserialize_optional_datetime")]
    #[serde(default)]
    start_date: Option<NaiveDateTime>,

    #[serde(deserialize_with = "deserialize_optional_datetime")]
    #[serde(default)]
    end_date: Option<NaiveDateTime>,

    #[serde(default)]
    query: Option<String>,

    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone)]
pub struct Tag {
    name: String,
    creation_date: DateTime<Local>,
}

// Form-Daten
#[derive(Deserialize)]
struct TickForm {
    task_id: u32,
}

#[derive(Deserialize)]
struct ChangeTaskForm {
    task_id: u32,
    task_title: String,
    task_description: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct NewTaskForm {
    task_title: String,
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

#[derive(Deserialize)]
struct ChangeListForm {
    list_id: String,
}

#[allow(dead_code)]
impl AppState {
    fn new() -> Self {
        let mut map: HashMap<TodoListId, TodoList> = HashMap::new();
        let todo_list = TodoList::new();
        map.insert(todo_list.id.lock().unwrap().clone(), todo_list.clone());
        AppState {
            current_list_id: todo_list.id,
            lists: Arc::new(Mutex::new(map)),
            tags: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn get_current_list(&self) -> TodoList {
        let current_id = self.current_list_id.lock().unwrap().clone();
        self.lists
            .lock()
            .unwrap()
            .get(&current_id)
            .expect("Aktuelle Liste nicht gefunden")
            .clone()
    }
}

impl Hash for TodoList {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.title.lock().unwrap().hash(state);
    }
}

impl PartialEq<Self> for TodoList {
    fn eq(&self, other: &Self) -> bool {
        self.title.lock().unwrap().eq(&*other.title.lock().unwrap())
    }
}

impl Eq for TodoList {}

impl Todo {
    fn new(id: u32, title: String, description: Option<String>, completed: bool) -> Self {
        Todo {
            id,
            title,
            description,
            due_date: None,
            created_at: Local::now(),
            completed,
            is_overdue: false,
            tags: HashSet::new(),
        }
    }

    fn tick(&mut self) -> &Todo {
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

impl TodoList {
    pub fn new() -> TodoList {
        Self {
            id: Arc::new(Mutex::new(Uuid::new_v4().to_string())),
            title: Arc::new(Mutex::new(String::from("To-dos"))),
            next_id: Arc::new(Mutex::new(1)),
            todos: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn generate_id(&self) -> u32 {
        let mut id = self.next_id.lock().unwrap();
        let current_id = *id;
        *id += 1;
        current_id
    }
}

fn deserialize_optional_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s.as_deref() {
        Some("true") => Ok(Some(true)),
        Some("false") => Ok(Some(false)),
        Some("") | None => Ok(None),
        _ => Err(de::Error::custom(
            "Invalid boolean value, expected 'true' or 'false'",
        )),
    }
}

fn deserialize_optional_datetime<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        if s.is_empty() {
            return Ok(None);
        }
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M")
            .map(Some)
            .map_err(de::Error::custom)
    } else {
        Ok(None)
    }
}

fn deserialize_comma_separated<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        Ok(s.split(',')
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect())
    } else {
        Ok(Vec::new())
    }
}

fn filter_by_date_range(
    todos: &Vec<Todo>,
    start_date: Option<NaiveDateTime>,
    end_date: Option<NaiveDateTime>,
) -> Vec<Todo> {
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

#[allow(dead_code)]
fn filter_by_tags(todos: &Vec<Todo>, tags: &HashSet<Tag>) -> Vec<Todo> {
    todos
        .iter()
        .filter(|todo| {
            todo.tags
                .iter()
                .any(|tag| tags.iter().any(|t| t.name == tag.name))
        })
        .cloned()
        .collect()
}

async fn new_task(State(state): State<AppState>, Json(payload): Json<NewTaskForm>) -> Redirect {
    let current_todos = state.get_current_list();
    let mut todos = current_todos.todos.lock().unwrap();
    if payload.task_title.is_empty() {
        return Redirect::to("/");
    }

    let new_todo: Todo = Todo::new(current_todos.generate_id(), payload.task_title, None, false);
    todos.insert(0, new_todo);
    Redirect::to("/")
}

// POST-Handler für /tick
async fn tick_task(State(state): State<AppState>, Json(payload): Json<TickForm>) -> Redirect {
    let current_todos = state.get_current_list();
    let mut todos = current_todos.todos.lock().unwrap();
    if let Some(task) = todos.iter_mut().find(|t| t.id == payload.task_id) {
        task.tick();
    }

    Redirect::to("/")
}

async fn delete_task(State(state): State<AppState>, Json(payload): Json<TickForm>) -> Redirect {
    let current_todos = state.get_current_list();
    let mut todos = current_todos.todos.lock().unwrap();
    if let Some(pos) = todos.iter().position(|todo| todo.id == payload.task_id) {
        todos.remove(pos);
    }
    Redirect::to("/")
}

async fn update_task(
    State(state): State<AppState>,
    Json(payload): Json<ChangeTaskForm>,
) -> Redirect {
    let current_todos = state.get_current_list();
    let mut todos = current_todos.todos.lock().unwrap();
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
        let old_tags = task.tags.clone();
        task.tags = payload
            .tags
            .iter()
            .map(|tag_name| {
                if let Some(existing) = old_tags.iter().find(|t| t.name == *tag_name) {
                    Tag {
                        name: tag_name.to_string(),
                        creation_date: existing.creation_date,
                    }
                } else {
                    Tag {
                        name: tag_name.to_string(),
                        creation_date: Local::now(),
                    }
                }
            })
            .collect();
    }
    let mut global_tags = state.tags.lock().unwrap();
    let old_global_tags = global_tags.clone();
    global_tags.clear();
    global_tags.extend(payload.tags.iter().map(|tag_name| {
        if let Some(existing) = old_global_tags.iter().find(|t| t.name == *tag_name) {
            Tag {
                name: tag_name.to_string(),
                creation_date: existing.creation_date,
            }
        } else {
            Tag {
                name: tag_name.to_string(),
                creation_date: Local::now(),
            }
        }
    }));
    Redirect::to("/")
}

async fn update_due_date(
    State(state): State<AppState>,
    Form(input): Form<DueDateUpdate>,
) -> Redirect {
    let naive_dt = NaiveDateTime::parse_from_str(&input.due_date, "%Y-%m-%dT%H:%M");
    if let Ok(dt) = naive_dt {
        let local_dt = Local.from_local_datetime(&dt).unwrap();

        let current_todos = state.get_current_list();
        let mut todos = current_todos.todos.lock().unwrap();
        if let Some(task) = todos.iter_mut().find(|t| t.id == input.task_id) {
            task.due_date = Some(local_dt);
        }
    }
    Redirect::to("/")
}

async fn update_list_title(
    State(state): State<AppState>,
    Json(payload): Json<TitleUpdate>,
) -> Redirect {
    let current_todos = state.get_current_list();
    let mut title = current_todos.title.lock().unwrap();
    if !payload.title.is_empty() {
        *title = payload.title;
    }
    Redirect::to("/")
}

async fn change_list(
    State(state): State<AppState>,
    Json(payload): Json<ChangeListForm>,
) -> Redirect {
    let lists = state.lists.lock().unwrap();
    if let Some(new_list) = lists.get(&payload.list_id) {
        *state.current_list_id.lock().unwrap() = new_list.id.lock().unwrap().clone();
    }
    Redirect::to("/")
}

pub async fn tasks(
    State(state): State<AppState>,
    Query(filters): Query<AllFilters>,
) -> Html<String> {
    let current_todos = state.get_current_list();
    let mut todos = current_todos.todos.lock().unwrap();
    todos.iter_mut().for_each(|todo| {
        todo.check_overdue();
    });
    let title = current_todos.title.lock().unwrap();
    let tags = state.tags.lock().unwrap();
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

    if !&filters.tags.is_empty() {
        filtered_todos = filter_by_tags(&filtered_todos, &tags);
    }

    let tera = Tera::new("src/templates/**/*").unwrap();
    let mut context = Context::new();

    context.insert("tasks", &*filtered_todos);
    context.insert("title", &*title);
    context.insert("now_string", &now_string);
    context.insert("tags", &*tags);

    let rendered = tera.render("tasks.html", &context).unwrap();
    Html(rendered)
}

pub fn routes() -> Router {
    // Beispiel-Daten
    Arc::new(Mutex::new(vec![
        Todo::new(1, "X aufkaufen".to_string(), None, true),
        Todo::new(
            2,
            "Kaffee kochen".to_string(),
            Some("Filterkaffee...".to_string()),
            false,
        ),
    ]));

    let app_state = AppState::new();
    let current_list_id = {
        let current_id_lock = app_state.current_list_id.lock().unwrap();
        current_id_lock.clone()
    };

    {
        let mut lists = app_state.lists.lock().unwrap();
        if let Some(list) = lists.get_mut(&current_list_id) {
            let mut todos = list.todos.lock().unwrap();
            todos.extend(vec![
                Todo::new(1, "X aufkaufen".to_string(), None, true),
                Todo::new(
                    2,
                    "Kaffee kochen".to_string(),
                    Some("Filterkaffee...".to_string()),
                    false,
                ),
            ]);
        }
    }
    
    Router::new()
        .route("/", get(tasks))
        .route("/tick", post(tick_task))
        .route("/new_task", post(new_task))
        .route("/delete_task", post(delete_task))
        .route("/update_list_title", post(update_list_title))
        .route("/update_date", post(update_due_date))
        .route("/update_task", post(update_task))
        .route("/change_list", post(change_list))
        .with_state(app_state) // AppState hier binden
}
