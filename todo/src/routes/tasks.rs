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
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::Mutex;
use uuid::Uuid;

pub type TodoListId = String;
#[allow(dead_code)]
pub type TagId = String;
#[allow(dead_code)]
pub type TaskId = String;

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

    #[serde(default)]
    list: String,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone)]
pub struct Tag {
    id: String,
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

#[derive(Deserialize)]
struct CreateListForm {
    list_name: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct SingleStringForm {
    string: String,
}

#[derive(Deserialize)]
struct RenameTagForm {
    tag_id: String,
    tag_name: String,
    task_id: String,
}

#[allow(dead_code)]
impl AppState {
    async fn new() -> Self {
        let mut map: HashMap<TodoListId, TodoList> = HashMap::new();
        let todo_list = TodoList::new(String::from("To-dos"));
        map.insert(todo_list.id.lock().await.clone(), todo_list.clone());
        AppState {
            current_list_id: todo_list.id,
            lists: Arc::new(Mutex::new(map)),
            tags: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn get_current_list(&self) -> TodoList {
        let current_id = self.current_list_id.lock().await.clone();
        self.lists
            .lock()
            .await
            .get(&current_id)
            .expect("Aktuelle Liste nicht gefunden")
            .clone()
    }

    pub async fn get_titles(&self) -> Vec<String> {
        let mut titles = Vec::new();
        for list in self.lists.lock().await.values() {
            let title_lock = list.title.lock().await;
            titles.push(title_lock.clone());
        }
        titles
    }

    pub async fn get_list_id(&self, name: String) -> Option<TodoListId> {
        let lists = self.lists.lock().await;
        for (list_id, list) in lists.iter() {
            let title = list.title.lock().await;
            if *title == name {
                return Some(list_id.clone());
            }
        }
        None
    }

    pub async fn get_todos_for_tag(&self, tag: &Tag, list: Option<&TodoList>) -> Vec<Todo> {
        let mut todos_with_tag: Vec<Todo> = Vec::new();
        let lists: Vec<TodoList> = match list {
            Some(list) => vec![list.clone()],
            None => self
                .lists
                .lock()
                .await
                .values()
                .cloned()
                .collect::<Vec<TodoList>>(),
        };

        for list in lists {
            let todos = list.todos.lock().await;
            for todo in todos.iter() {
                if todo.tags.contains(tag) {
                    todos_with_tag.push(todo.clone());
                }
            }
        }
        todos_with_tag
    }

    pub async fn get_tag_from_id(&self, tag_id: String) -> Option<&Tag> {
        let tags = self.tags.lock().await;
        for tag in tags.iter() {
            if tag.id.eq(&tag_id) {
                Some(tag);
            }
        }
        None
    }
}

impl Hash for TodoList {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let title = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.title.lock());
        title.hash(state);
    }
}

impl PartialEq<Self> for TodoList {
    fn eq(&self, other: &Self) -> bool {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { self.title.lock().await.eq(&*other.title.lock().await) })
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
    pub fn new(name: String) -> TodoList {
        Self {
            id: Arc::new(Mutex::new(Uuid::new_v4().to_string())),
            title: Arc::new(Mutex::new(String::from(name))),
            next_id: Arc::new(Mutex::new(1)),
            todos: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn generate_id(&self) -> u32 {
        let mut id = self.next_id.lock().await;
        let current_id = *id;
        *id += 1;
        current_id
    }
}

impl Tag {
    pub fn new(name: String, creation_date: DateTime<Local>) -> Self {
        Tag {
            id: Uuid::new_v4().to_string(),
            name,
            creation_date,
        }
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
    let current_todos = state.get_current_list().await;
    let mut todos = current_todos.todos.lock().await;
    if payload.task_title.is_empty() {
        return Redirect::to("/");
    }

    let new_todo: Todo = Todo::new(
        current_todos.generate_id().await,
        payload.task_title,
        None,
        false,
    );
    todos.insert(0, new_todo);
    Redirect::to("/")
}

// POST-Handler für /tick
async fn tick_task(State(state): State<AppState>, Json(payload): Json<TickForm>) -> Redirect {
    let current_todos = state.get_current_list().await;
    let mut todos = current_todos.todos.lock().await;
    if let Some(task) = todos.iter_mut().find(|t| t.id == payload.task_id) {
        task.tick();
    }

    Redirect::to("/")
}

async fn delete_task(State(state): State<AppState>, Json(payload): Json<TickForm>) -> Redirect {
    let current_todos = state.get_current_list().await;
    let mut todos = current_todos.todos.lock().await;
    if let Some(pos) = todos.iter().position(|todo| todo.id == payload.task_id) {
        todos.remove(pos);
    }
    Redirect::to("/")
}

async fn update_task(
    State(state): State<AppState>,
    Json(payload): Json<ChangeTaskForm>,
) -> Redirect {
    let current_todos = state.get_current_list().await;
    let mut todos = current_todos.todos.lock().await;
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
        task.tags = update_tags(&payload.tags, &old_tags);
    }
    let mut global_tags = state.tags.lock().await;
    let old_global_tags = global_tags.clone();
    global_tags.clear();
    global_tags.extend(update_tags(&payload.tags, &old_global_tags));
    Redirect::to("/")
}

fn update_tags(new_tags: &Vec<String>, old_tags: &HashSet<Tag>) -> HashSet<Tag> {
    new_tags
        .iter()
        .map(|tag_name| {
            if let Some(existing) = old_tags.iter().find(|t| t.name == *tag_name) {
                Tag::new(tag_name.to_string(), existing.creation_date)
            } else {
                Tag::new(tag_name.to_string(), Local::now())
            }
        })
        .collect()
}

async fn update_due_date(
    State(state): State<AppState>,
    Form(input): Form<DueDateUpdate>,
) -> Redirect {
    let naive_dt = NaiveDateTime::parse_from_str(&input.due_date, "%Y-%m-%dT%H:%M");
    if let Ok(dt) = naive_dt {
        let local_dt = Local.from_local_datetime(&dt).single().unwrap();

        let current_todos = state.get_current_list().await;
        let mut todos = current_todos.todos.lock().await;
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
    let current_todos = state.get_current_list().await;
    let mut title = current_todos.title.lock().await;
    if !payload.title.is_empty() {
        *title = payload.title;
    }
    Redirect::to("/")
}

async fn change_list(
    State(state): State<AppState>,
    Json(payload): Json<ChangeListForm>,
) -> Redirect {
    let lists = state.lists.lock().await;
    if let Some(new_list) = lists.get(&payload.list_id) {
        *state.current_list_id.lock().await = new_list.id.lock().await.clone();
    }
    Redirect::to("/")
}

async fn create_list(
    State(state): State<AppState>,
    Json(payload): Json<CreateListForm>,
) -> Redirect {
    if !state.get_titles().await.contains(&payload.list_name) {
        let new_list = TodoList::new(payload.list_name);
        state
            .lists
            .lock()
            .await
            .insert(new_list.clone().id.lock().await.to_string(), new_list);
    }
    Redirect::to("/")
}

async fn rename_tag(State(state): State<AppState>, Json(payload): Json<RenameTagForm>) -> Redirect {
    // Hole den Tag, der geändert werden soll.
    if let Some(existing_tag) = state.get_tag_from_id(payload.tag_id).await.cloned() {
        // Erstelle eine veränderte Version des Tags (kopiert) mit dem neuen Namen.
        let mut updated_tag = existing_tag;
        updated_tag.name = payload.tag_name.clone();

        // Aktualisiere den globalen Tag-Speicher.
        {
            let mut global_tags = state.tags.lock().await;
            global_tags.replace(updated_tag.clone());
        }

        // Hole die aktuelle List-ID und dann die entsprechende Liste.
        let current_id = state.current_list_id.lock().await.clone();
        let lists_guard = state.lists.lock().await;
        if let Some(current_list) = lists_guard.get(&current_id) {
            // Sperre die Todos der aktuellen Liste.
            let mut todos = current_list.todos.lock().await;
            // Suche nach der Task, deren ID mit payload.task_id übereinstimmt,
            // und ersetze in dieser Task den Tag.
            if let Some(todo) = todos.iter_mut().find(|t| t.id.to_string() == payload.task_id) {
                todo.tags.replace(updated_tag.clone());
            }
        }
    }
    let _ = tasks(
        State(state),
        Query(AllFilters {
            query: None,
            tags: vec![String::from("")],
            list: String::from(""),
            start_date: None,
            end_date: None,
            completed: None,
            is_due: None,
        }),
    )
    .await;
    Redirect::to("/")
}

pub async fn tasks(
    State(state): State<AppState>,
    Query(filters): Query<AllFilters>,
) -> Html<String> {
    if !filters.list.is_empty() {
        let current_id = state.get_list_id(filters.list).await.unwrap();
        *state.current_list_id.lock().await = current_id;
    }
    let current_list = state.get_current_list().await;

    let todos = {
        let todos_lock = current_list.todos.lock().await;
        todos_lock.clone()
    };

    let mut updated_todos = todos.clone();
    for todo in &mut updated_todos {
        todo.check_overdue();
    }

    let title = {
        let title_lock = current_list.title.lock().await;
        title_lock.clone()
    };

    let tags = {
        let tags_lock = state.tags.lock().await;
        tags_lock.clone()
    };

    let now = Local::now();
    let now_string = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute()
    );

    let mut filtered_todos = updated_todos;

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

    if !filters.tags.is_empty() {
        filtered_todos = filter_by_tags(&filtered_todos, &tags);
    }

    let todo_lists = state.get_titles().await;
    let tera = Tera::new("src/templates/**/*").unwrap();
    let mut context = Context::new();

    context.insert("lists", &todo_lists);
    context.insert("tasks", &filtered_todos);
    context.insert("title", &title);
    context.insert("now_string", &now_string);
    context.insert("tags", &tags);

    let rendered = tera.render("tasks.html", &context).unwrap();
    Html(rendered)
}

pub async fn routes() -> Router {
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

    let app_state = AppState::new().await;
    let current_list_id = async {
        let current_id_lock = app_state.current_list_id.lock().await;
        current_id_lock.clone()
    };

    async {
        let mut lists = app_state.lists.lock().await;
        if let Some(list) = lists.get_mut(&current_list_id.await) {
            let mut todos = list.todos.lock().await;
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
    .await;

    Router::new()
        .route("/", get(tasks))
        .route("/tick", post(tick_task))
        .route("/new_task", post(new_task))
        .route("/delete_task", post(delete_task))
        .route("/update_list_title", post(update_list_title))
        .route("/update_date", post(update_due_date))
        .route("/update_task", post(update_task))
        .route("/change_list", post(change_list))
        .route("/create_list", post(create_list))
        .route("/rename_tag", post(rename_tag))
        .with_state(app_state)
}
