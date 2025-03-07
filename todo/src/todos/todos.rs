use axum::extract::Query;
use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::{get, post},
    Form, Json, Router,
};
use chrono::NaiveDateTime;
use chrono::{DateTime, Local, TimeZone};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::todos;
use todos::filter::*;
use todos::forms::*;
use todos::todo::*;

pub type TodoListId = String;
pub type TagId = String;
pub type TodoId = String;

#[derive(Clone)]
pub struct AppState {
    pub current_list_id: Arc<Mutex<TodoListId>>,
    pub lists: Arc<Mutex<HashMap<TodoListId, Arc<TodoList>>>>,
    pub tags: Arc<RwLock<HashMap<String, Arc<Mutex<Tag>>>>>,
}

#[derive(Debug, Clone)]
pub struct TodoList {
    pub id: Arc<Mutex<TodoListId>>,
    pub title: Arc<Mutex<String>>,
    pub todos: Arc<Mutex<Vec<Todo>>>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub creation_date: DateTime<Local>,
}

#[allow(dead_code)]
impl AppState {
    async fn new() -> Self {
        let mut map: HashMap<TodoListId, Arc<TodoList>> = HashMap::new();
        let todo_list = TodoList::new(String::from("To-dos"));
        map.insert(
            todo_list.id.lock().await.clone(),
            Arc::new(todo_list.clone()),
        );
        AppState {
            current_list_id: todo_list.id,
            lists: Arc::new(Mutex::new(map)),
            tags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn find_list_id_by_name(&self, name: &str) -> Option<TodoListId> {
        let lists = self.lists.lock().await;
        for (id, list) in lists.iter() {
            let title = list.title.lock().await;
            if *title == name {
                return Some(id.clone());
            }
        }
        None
    }

    pub async fn with_todo_mut<F, R>(&self, todo_id: TodoId, mut f: F) -> Option<R>
    where
        F: FnMut(&mut Todo) -> R,
    {
        let lists = self.lists.lock().await;
        for list in lists.values() {
            let mut todos = list.todos.lock().await;
            if let Some(todo) = todos.iter_mut().find(|todo| todo.id == todo_id) {
                return Some(f(todo));
            }
        }
        None
    }

    pub async fn get_current_list(&self) -> Arc<TodoList> {
        let current_id = self.current_list_id.lock().await.clone();
        self.lists
            .lock()
            .await
            .get(&current_id)
            .expect("Aktuelle Liste nicht gefunden")
            .clone()
    }

    // TODO Methode hat einen Fehler irgendwo
    pub async fn get_list(&self, list: String) -> Option<Arc<TodoList>> {
        let lists = self.lists.lock().await;
        for todo_list in lists.values() {
            let title = todo_list.title.lock().await.clone();
            if *title == list {
                return Some(todo_list.clone());
            }
        }
        None
    }

    pub async fn get_titles(&self) -> Vec<String> {
        let mut titles: Vec<String> = Vec::new();
        for list in self.lists.lock().await.values() {
            let title_lock = list.title.lock().await;
            titles.push(title_lock.clone());
        }
        titles
    }

    pub async fn get_todos_for_tag(&self, tag: TagId, list: Option<Arc<TodoList>>) -> Vec<Todo> {
        let mut todos_with_tag: Vec<Todo> = Vec::new();
        let lists = match list {
            Some(list) => vec![list.clone()],
            None => self
                .lists
                .lock()
                .await
                .values()
                .cloned()
                .collect::<Vec<Arc<TodoList>>>(),
        };

        for list in lists {
            let todos = list.todos.lock().await;
            for todo in todos.iter() {
                if todo.tags.contains(&tag) {
                    todos_with_tag.push(todo.clone());
                }
            }
        }
        todos_with_tag
    }

    pub async fn get_todos_with_tags(&self, todos: Vec<Todo>) -> Vec<FrontendTodo> {
        let mut frontend_todos = Vec::new();
        for todo in todos.iter().cloned() {
            let mut todo_tags = HashSet::new();
            for tag_id in todo.tags.iter() {
                let found_tag = self
                    .tags
                    .read()
                    .await
                    .get(tag_id)
                    .unwrap()
                    .lock()
                    .await
                    .clone();
                todo_tags.insert(found_tag);
            }
            frontend_todos.push(FrontendTodo {
                id: todo.id,
                title: todo.title,
                due_date: todo.due_date,
                description: todo.description,
                created_at: todo.created_at,
                completed: todo.completed,
                is_overdue: todo.is_overdue,
                tags: todo_tags,
            });
        }
        frontend_todos
    }

    pub async fn get_all_tags(&self) -> Vec<Tag> {
        let tags_map = self.tags.read().await;
        let mut result = Vec::new();
        for tag_arc in tags_map.values() {
            let tag = tag_arc.lock().await.clone();
            result.push(tag);
        }
        result
    }

    pub async fn get_tag(&self, tag_id: TagId) -> Option<Arc<Mutex<Tag>>> {
        self.tags.read().await.get(&tag_id).cloned()
    }

    pub async fn get_tag_from_name(&self, tag_name: String) -> Option<Arc<Mutex<Tag>>> {
        let tags = self.tags.read().await;
        for tag in tags.values() {
            let locked_tag = tag.lock().await;
            if tag_name == locked_tag.name {
                return Some(tag.clone());
            }
        }
        None
    }

    pub async fn get_tags_from_strings(&self, tag_strings: Vec<String>) -> HashSet<Tag> {
        let mut found_tags = HashSet::new();
        let tags_lock = self.tags.read().await;

        for tag_string in tag_strings {
            for tag in tags_lock.values() {
                if tag.lock().await.name.eq(&tag_string) {
                    found_tags.insert(tag.lock().await.clone());
                }
            }
        }
        found_tags
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

impl TodoList {
    pub fn new(name: String) -> TodoList {
        Self {
            id: Arc::new(Mutex::new(Uuid::new_v4().to_string())),
            title: Arc::new(Mutex::new(String::from(name))),
            todos: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn change_title(&self, new_title: String) {
        let mut title = self.title.lock().await;
        *title = new_title;
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
    
    pub fn change_name(&mut self, name: String) {
        self.name = name;
    }
}

async fn new_todo(State(state): State<AppState>, Json(payload): Json<NewTodoForm>) -> Redirect {
    let current_todos = state.get_current_list().await;
    let mut todos = current_todos.todos.lock().await;
    if payload.todo_title.is_empty() {
        return Redirect::to("/");
    }

    let new_todo: Todo = Todo::new(Uuid::new_v4().to_string(), payload.todo_title, None, false);
    todos.insert(0, new_todo);
    Redirect::to("/")
}

// POST-Handler für /tick
async fn tick_todo(State(state): State<AppState>, Json(payload): Json<TickForm>) -> Redirect {
    let current_id = {
        let id_lock = state.current_list_id.lock().await;
        id_lock.clone()
    };

    let mut lists = state.lists.lock().await;
    let list = lists
        .get_mut(&current_id)
        .expect("Aktuelle Liste nicht gefunden");
    let mut todos = list.todos.lock().await;

    for todo in todos.iter_mut() {
        if todo.id == payload.todo_id {
            todo.tick();
            break;
        }
    }
    Redirect::to("/")
}

async fn delete_todo(State(state): State<AppState>, Json(payload): Json<TickForm>) -> Redirect {
    let current_todos = state.get_current_list().await;
    let mut todos = current_todos.todos.lock().await;
    if let Some(pos) = todos.iter().position(|todo| todo.id.eq(&payload.todo_id)) {
        todos.remove(pos);
    }
    Redirect::to("/")
}

pub async fn update_todo_name(
    State(state): State<AppState>,
    Json(payload): Json<UpdateTodoNameForm>,
) -> Redirect {
    let current_todos = state.get_current_list().await; // Aktuelle Liste holen
    let mut todos = current_todos.todos.lock().await; // To-dos aus aktueller Liste holen
    if let Some(todo) = todos.iter_mut().find(|t| t.id.eq(&payload.todo_id)) {
        todo.update_title(payload.todo_name);
    }
    Redirect::to("/")
}

pub async fn update_todo_description(
    State(state): State<AppState>,
    Json(payload): Json<UpdateTodoDescriptionForm>,
) -> Redirect {
    let current_todos = state.get_current_list().await; // Aktuelle Liste holen
    let mut todos = current_todos.todos.lock().await; // To-dos aus aktueller Liste holen
    if let Some(todo) = todos.iter_mut().find(|t| t.id.eq(&payload.todo_id)) {
        todo.update_description(payload.todo_description);
    }
    Redirect::to("/")
}

async fn update_due_date(
    State(state): State<AppState>,
    Form(input): Form<DueDateForm>,
) -> Redirect {
    let naive_dt = NaiveDateTime::parse_from_str(&input.due_date, "%Y-%m-%dT%H:%M");
    if let Ok(dt) = naive_dt {
        let local_dt = Local.from_local_datetime(&dt).single().unwrap();

        let current_todos = state.get_current_list().await;
        let mut todos = current_todos.todos.lock().await;
        if let Some(todo) = todos.iter_mut().find(|t| t.id == input.todo_id) {
            todo.due_date = Some(local_dt);
        }
    }
    Redirect::to("/")
}

async fn update_list_title(
    State(state): State<AppState>,
    Json(payload): Json<TitleUpdateForm>,
) -> Redirect {
    let current_list_id = state.current_list_id.lock().await.clone();
    let lists = state.lists.lock().await;
    if let Some(current_todo_list) = lists.get(&current_list_id) {
        current_todo_list.change_title(payload.title).await;
    } else {
        println!("Liste mit ID '{}' nicht gefunden!", current_list_id);
    }

    Redirect::to("/")
}

pub async fn switch_list(
    State(state): State<AppState>,
    Json(payload): Json<SwitchListForm>,
) -> Redirect {
    if let Some(new_id) = state.find_list_id_by_name(payload.list_name.as_str()).await {
        *state.current_list_id.lock().await = new_id;
    }
    Redirect::to("/")
}

pub async fn add_list(
    State(state): State<AppState>,
    Json(payload): Json<CreateListForm>,
) -> Redirect {
    if !state.get_titles().await.contains(&payload.list_name) {
        let new_list = TodoList::new(payload.list_name);
        state.lists.lock().await.insert(
            new_list.clone().id.lock().await.to_string(),
            Arc::new(new_list),
        );
    }
    Redirect::to("/")
}

pub async fn delete_list(
    State(state): State<AppState>,
    Json(payload): Json<RemoveListForm>,
) -> Redirect {
    let mut lists = state.lists.lock().await;
    if lists.values().len() >= 2 {
        lists.remove(&payload.list_id);
        let new_list_id = lists.keys().next().unwrap().clone();
        drop(lists);

        let mut current_list_id = state.current_list_id.lock().await;
        *current_list_id = new_list_id;
    }
    Redirect::to("/")
}

pub async fn rename_tag(
    State(state): State<AppState>,
    Json(payload): Json<RenameTagForm>,
) -> Redirect {
    if let Some(tag) = state.tags.write().await.get_mut(&payload.tag_id) {
        tag.lock().await.change_name(payload.tag_name);
    } else {
        eprintln!("Der folgende Tag konnte nicht umbenannt werden: {}", payload.tag_id);
    }
    Redirect::to("/")
}

pub async fn remove_tag(
    State(state): State<AppState>,
    Json(payload): Json<RemoveTagForm>,
) -> Redirect {
    {
        let todos_with_tag = state.get_todos_for_tag(payload.tag_id.clone(), None).await;
        if todos_with_tag.len().lt(&2) {
            state.tags.write().await.remove(&payload.tag_id);
        }
    }
    let current_list = state.get_current_list().await;
    let mut todos = current_list.todos.lock().await;
    for todo in todos.iter_mut() {
        if todo.id == payload.todo_id && todo.tags.contains(&payload.tag_id) {
            todo.remove_tag(&payload.tag_id);
        }
    }
    Redirect::to("/")
}

pub async fn add_tag(State(state): State<AppState>, Json(payload): Json<AddTagForm>) -> Redirect {
    if payload.tag_name.is_empty() {
        return Redirect::to("/");
    };
    let searched_tag = state.get_tag_from_name(payload.tag_name.clone()).await;

    // Wenn Tag nicht existiert, neuen Tag erstellen und in den globalen Tag-Manager einfügen
    let tag_id = if let None = searched_tag {
        let new_tag = Tag::new(payload.tag_name.clone(), Local::now());
        {
            let mut tags = state.tags.write().await;
            tags.insert(new_tag.id.clone(), Arc::new(Mutex::new(new_tag.clone())));
        }
        new_tag.id
    } else {
        searched_tag.unwrap().lock().await.id.clone()
    };

    state
        .with_todo_mut(payload.todo_id, |todo| {
            todo.add_tag(tag_id.clone());
        })
        .await;

    Redirect::to("/")
}

pub async fn todos(
    State(state): State<AppState>,
    Query(filters): Query<AllFilters>,
) -> Html<String> {
    /*if !filters.list.is_empty() {
        let current_id = state.get_list_id(filters.list.clone()).await.unwrap_or_default();
        if state.lists.lock().await.keys().collect::<Vec<&TodoListId>>().contains(&&current_id) {
            *state.current_list_id.lock().await = current_id;
        }
    }*/
    
    let current_list = state.get_current_list().await;
    let mut updated_todos = current_list.todos.lock().await.clone();

    for todo in &mut updated_todos {
        todo.check_overdue();
    }

    let tags: Vec<Tag> = {
        let tags_guard = state.tags.read().await;
        let tag_futures = tags_guard
            .values()
            .map(|tag| async { tag.lock().await.clone() });
        let tags_vec = join_all(tag_futures).await;
        tags_vec.into_iter().collect()
    };

    let filtered_todos = filter(&filters, &updated_todos, &tags);
    let frontend_todos = state.get_todos_with_tags(filtered_todos).await;

    let title = {
        let title_lock = current_list.title.lock().await;
        title_lock.clone()
    };
    let todo_lists = state.get_titles().await;
    let current_list_id = state.current_list_id.lock().await.clone();

    // HTML parsen
    let tera = Tera::new("src/templates/**/*").unwrap();
    let mut context = Context::new();
    context.insert("title", &title);
    context.insert("list_id", &current_list_id);
    context.insert("lists", &todo_lists);
    context.insert("todos", &frontend_todos);
    context.insert("tags", &tags);

    let rendered = tera.render("todos.html", &context).unwrap();
    Html(rendered)
}

pub async fn routes() -> Router {
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
                Todo::new(
                    Uuid::new_v4().to_string(),
                    "X aufkaufen".to_string(),
                    None,
                    true,
                ),
                Todo::new(
                    Uuid::new_v4().to_string(),
                    "Kaffee kochen".to_string(),
                    Some("Filterkaffee...".to_string()),
                    false,
                ),
            ]);
        }
    }
    .await;

    Router::new()
        .route("/", get(todos))
        .route("/add_todo", post(new_todo))
        .route("/delete_todo", post(delete_todo))
        .route("/tick_todo", post(tick_todo))
        .route("/update_todo_name", post(update_todo_name))
        .route("/update_todo_date", post(update_due_date))
        .route("/update_todo_description", post(update_todo_description))
        .route("/add_list", post(add_list))
        .route("/delete_list", post(delete_list))
        .route("/switch_list", post(switch_list))
        .route("/update_list_title", post(update_list_title))
        .route("/add_tag", post(add_tag))
        .route("/remove_tag", post(remove_tag))
        .route("/rename_tag", post(rename_tag))
        .with_state(app_state)
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;

    async fn todos_setup() -> TodoList {
        let list = TodoList::new("New List".to_string());
        let first_todo = Todo::new(
            "1".to_string(),
            "First Todo".to_string(),
            Some("This is the description of the first todo".to_string()),
            false,
        );
        let second_todo = Todo::new(
            "2".to_string(),
            "Second Todo".to_string(),
            Some("This is the description of the second todo".to_string()),
            true,
        );

        list.todos.lock().await.insert(0, first_todo);
        list.todos.lock().await.insert(1, second_todo);
        list
    }

    fn todos_vec_setup() -> Vec<Todo> {
        let first_tag = Tag::new(
            "Wonderful Tag Name".to_string(),
            Local
                .from_local_datetime(
                    &NaiveDateTime::parse_from_str("2022-04-13 00:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                )
                .unwrap(),
        );

        let mut list = Vec::new();
        let first_todo = Todo::new(
            "1".to_string(),
            "First Todo".to_string(),
            Some("This is the description of the first todo".to_string()),
            false,
        );
        let mut second_todo = Todo::new(
            "2".to_string(),
            "Second Todo".to_string(),
            Some("This is the description of the second todo".to_string()),
            true,
        );
        second_todo.tags.insert(first_tag.id);
        list.extend(vec![first_todo, second_todo.clone()]);
        list
    }

    #[test]
    fn test_filter_by_tags() {
        let first_tag = Tag::new(
            "Wonderful Tag Name".to_string(),
            Local
                .from_local_datetime(
                    &NaiveDateTime::parse_from_str("2022-04-13 00:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                )
                .unwrap(),
        );
        let second_tag = Tag::new(
            "Amazing second Tag Name".to_string(),
            Local
                .from_local_datetime(
                    &NaiveDateTime::parse_from_str("2025-04-16 00:00:00", "%Y-%m-%d %H:%M:%S")
                        .unwrap(),
                )
                .unwrap(),
        );
        let available_tags = vec![first_tag.clone(), second_tag].into_iter().collect();

        // To-dos vorbereiten
        let mut list = Vec::new();
        let first_todo = Todo::new(
            "1".to_string(),
            "First Todo".to_string(),
            Some("This is the description of the first todo".to_string()),
            false,
        );
        let mut second_todo = Todo::new(
            "2".to_string(),
            "Second Todo".to_string(),
            Some("This is the description of the second todo".to_string()),
            true,
        );
        second_todo.tags.insert(first_tag.id);
        list.extend(vec![first_todo, second_todo.clone()]);

        // Filterfunktion testen
        let mut second_todo_vec = Vec::new();
        second_todo_vec.push(second_todo);
        assert_eq!(filter_by_tags(&list, &available_tags), second_todo_vec);
    }
}
