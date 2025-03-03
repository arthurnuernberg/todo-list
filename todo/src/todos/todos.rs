use axum::extract::Query;
use axum::{
    extract::State,
    response::{Html, Redirect},
    routing::{get, post},
    Form, Json, Router,
};
use chrono::NaiveDateTime;
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::thread::current;
use tera::{Context, Tera};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::todos;
use todos::filter::*;
use todos::forms::*;
use todos::todo::*;

pub type TodoListId = String;
#[allow(dead_code)]
pub type TagId = String;
#[allow(dead_code)]
pub type TodoId = String;

#[derive(Clone)]
pub struct AppState {
    pub current_list_id: Arc<Mutex<TodoListId>>,
    pub lists: Arc<Mutex<HashMap<TodoListId, TodoList>>>,
    pub tags: Arc<RwLock<HashMap<String, Arc<Mutex<Tag>>>>>,
}

#[derive(Debug, Clone)]
pub struct TodoList {
    pub id: Arc<Mutex<TodoListId>>,
    pub todos: Arc<Mutex<Vec<Todo>>>,
    pub title: Arc<Mutex<String>>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub creation_date: DateTime<Local>,
}

impl AppState {
    async fn new() -> Self {
        let mut map: HashMap<TodoListId, TodoList> = HashMap::new();
        let todo_list = TodoList::new(String::from("To-dos"));
        map.insert(todo_list.id.lock().await.clone(), todo_list.clone());
        AppState {
            current_list_id: todo_list.id,
            lists: Arc::new(Mutex::new(map)),
            tags: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_todo(&self, todo_id: TodoId) -> Option<Todo> {
        for list in self.lists.lock().await.values() {
            let todos = list.todos.lock().await;
            if let Some(todo) = todos.iter().find(|todo| todo.id == todo_id) {
                return Some(todo.clone());
            }
        }
        None
    }

    pub async fn exists_same_todo_name(&self, todo_name: &str) -> bool {
        let current_todos = self.get_current_list().await.todos.lock().await.clone();
        for todo in current_todos {
            if todo.title.eq(todo_name) {
                return true;
            };
        }
        false
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

    pub async fn get_current_list(&self) -> TodoList {
        let current_id = self.current_list_id.lock().await.clone();
        self.lists
            .lock()
            .await
            .get(&current_id)
            .expect("Aktuelle Liste nicht gefunden")
            .clone()
    }
    
    /*pub async fn get_current_list_mut(&mut self) -> &mut TodoList {
        let current_id = self.current_list_id.lock().await.clone();
        lists = self.lists.lock().await.get_mut(&current_id).expect("Aktuelle Liste mutable nicht gefunden")
    }*/

    pub async fn get_titles(&self) -> Vec<String> {
        let mut titles: Vec<String> = Vec::new();
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

    pub async fn get_todos_for_tag(&self, tag: TagId, list: Option<&TodoList>) -> Vec<Todo> {
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
    
    pub async fn change_title(&mut self, new_title: String) {
        if !new_title.is_empty() {
            let mut title = self.title.lock().await;
            *title = new_title;
        }
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

async fn update_todo(
    State(state): State<AppState>,
    Json(payload): Json<ChangeTodoForm>,
) -> Redirect {
    let current_todos = state.get_current_list().await; // Aktuelle Liste holen
    let mut todos = current_todos.todos.lock().await; // To-dos aus aktueller Liste holen
    if let Some(todo) = todos.iter_mut().find(|t| t.id.eq(&payload.todo_id)) {
        todo.title = {
            if !payload.todo_title.is_empty() {
                payload.todo_title
            } else {
                todo.title.clone()
            }
        };
        todo.description = {
            if payload.todo_description.is_empty() {
                None
            } else {
                Some(payload.todo_description)
            }
        };
        update_tags(State(&state), &payload.tags, &mut todo.tags).await;
    }
    Redirect::to("/")
}

pub async fn update_tags(
    State(state): State<&AppState>,
    new_tag_strings: &Vec<String>,
    todo_tags: &mut HashSet<TagId>,
) {
    let mut new_tags = HashMap::<TagId, Arc<Mutex<Tag>>>::new();

    for new_tag_name in new_tag_strings {
        match state.get_tag_from_name(new_tag_name.clone()).await {
            Some(tag) => {
                new_tags.insert(
                    tag.lock().await.id.clone(),
                    Arc::new(Mutex::new(tag.lock().await.clone())),
                );
            }
            None => {
                let created_tag = Tag::new(new_tag_name.clone(), Local::now());
                new_tags.insert(created_tag.clone().id, Arc::new(Mutex::new(created_tag)));
            }
        }
    }
    state.tags.write().await.extend(new_tags.clone());
    todo_tags.extend(new_tags.keys().cloned());
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
    let mut lists = state.lists.lock().await;
    let current_todo_list = lists.get_mut(&current_list_id).unwrap();
    current_todo_list.change_title(payload.title).await;
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

// Neue Version des Tag rename
pub async fn rename_tag(
    State(state): State<AppState>,
    Json(payload): Json<RenameTagForm>,
) -> Redirect {
    let tags = state.tags.write().await;
    if let Some(found_tag) = tags.get(&payload.tag_id) {
        let mut unwrapped_value = found_tag.lock().await;
        if !payload.tag_name.is_empty() {
            unwrapped_value.name = payload.tag_name;
        }
    }
    Redirect::to("/")
}

pub async fn remove_tag(
    State(state): State<AppState>,
    Json(payload): Json<RemoveTagForm>,
) -> Redirect {
    state.tags.write().await.remove(&payload.tag_id);
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
    if !filters.list.is_empty() {
        let current_id = state.get_list_id(filters.list.clone()).await.unwrap();
        *state.current_list_id.lock().await = current_id;
    }
    // To-dos der aktuellen Liste holen und Fälligkeiten prüfen
    let current_list = state.get_current_list().await;
    let mut updated_todos = {
        let todos_lock = current_list.todos.lock().await;
        todos_lock.clone()
    };
    // To-do-Fälligkeiten prüfen
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

    // HTML parsen
    let tera = Tera::new("src/templates/**/*").unwrap();
    let mut context = Context::new();
    context.insert("title", &title);
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
        .route("/tick", post(tick_todo))
        .route("/new_todo", post(new_todo))
        .route("/delete_todo", post(delete_todo))
        .route("/update_list_title", post(update_list_title))
        .route("/update_date", post(update_due_date))
        .route("/update_todo", post(update_todo))
        .route("/change_list", post(change_list))
        .route("/create_list", post(create_list))
        .route("/rename_tag", post(rename_tag))
        .route("/add_tag", post(add_tag))
        .route("/remove_tag", post(remove_tag))
        .with_state(app_state)
}

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
