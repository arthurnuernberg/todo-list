use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use axum::{
    routing::{get, post},
    Form, Json, Router,
};
use axum_sessions::extractors::ReadableSession;
use axum_sessions::{async_session::MemoryStore, SessionLayer};
use chrono::{DateTime, NaiveDateTime, Utc};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::env;
use std::hash::Hash;
use tera::{Context, Result as TeraResult, Tera, Value};
use uuid::Uuid;

use crate::todos;
use crate::todos::db::{FrontendList, TodoDatabaseExt};
use crate::todos::filter::filter;
use todos::forms::*;
use todos::todo::*;

pub type TodoListId = String;
pub type TagId = String;
pub type TodoId = String;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub current_list_id: TodoListId,
}

#[derive(Debug, Clone)]
pub struct TodoList {
    pub id: TodoListId,
    pub title: String,
    pub todos: Vec<Todo>,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Debug, Clone)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl AppState {
    fn new(db_pool: PgPool, current_list_id: String) -> Self {
        Self {
            db_pool,
            current_list_id,
        }
    }
}

impl TodoList {
    fn new(name: &str) -> Self {
        Self {
            id: String::from(Uuid::new_v4()),
            title: String::from(name),
            todos: Vec::new(),
        }
    }
}

impl Tag {
    pub fn new(name: String, creation_date: DateTime<Utc>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            created_at: creation_date,
        }
    }
}

/*pub async fn get_todos_with_tags(todos: Vec<Todo>, tags: Vec<Tag>) -> Vec<FrontendTodo> {
    let mut frontend_todos = Vec::new();
    for todo in todos.iter().cloned() {
        let mut todo_tags = Vec::new();
        for tag_id in todo.tags.iter() {
            let found_tag =
                tags
                .get(tag_id)
                .unwrap()
                .lock()
                .await
                .clone();
            todo_tags.push(found_tag);
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
}*/

pub async fn new_todo(State(state): State<AppState>, Json(payload): Json<NewTodoForm>) -> Redirect {
    if payload.todo_title.is_empty() {
        return Redirect::to("/");
    }

    let new_todo: Todo = Todo::new(Uuid::new_v4(), payload.todo_title.as_str(), None, false);
    if let Err(e) = state
        .db_pool
        .add_todo(&new_todo, state.current_list_id)
        .await
    {
        eprintln!(
            "Fehler beim Hinzufügen eines To-dos in die Datenbank:\n{:?}",
            e
        );
    }
    Redirect::to("/")
}

pub async fn add_sub_todo(State(state): State<AppState>, Json(payload): Json<NewSubTodoForm>) -> Redirect {
    if payload.sub_todo_title.is_empty() {
        return Redirect::to("/");
    }

    let new_sub_todo = Todo::new(Uuid::new_v4(), payload.sub_todo_title.as_str(), None, false);
    if let Err(e) = state.db_pool.add_subtodo(&new_sub_todo, payload.todo_id, state.current_list_id).await {
        eprintln!(
            "Fehler beim Hinzufügen eines Sub-To-dos in die Datenbank:\n{:?}",
            e
        );
    }
    Redirect::to("/")
}

pub async fn tick_todo(State(state): State<AppState>, Json(payload): Json<TickForm>) -> Redirect {
    if let Err(e) = &state
        .db_pool
        .tick_todo(payload.todo_id, state.current_list_id)
        .await
    {
        eprintln!("Fehler beim Tick-Todo in der Datenbank:\n{:?}", e);
    }
    Redirect::to("/")
}

pub async fn delete_todo(State(state): State<AppState>, Json(payload): Json<RemoveTodoForm>) -> Redirect {
    if let Err(e) = &state
        .db_pool
        .remove_todo(state.current_list_id, payload.todo_id)
        .await
    {
        eprintln!("Fehler beim Delete-Todo in der Datenbank:\n{:?}", e);
    }
    Redirect::to("/")
}

pub async fn update_todo_name(
    State(state): State<AppState>,
    Json(payload): Json<UpdateTodoNameForm>,
) -> Redirect {
    if let Err(e) = &state
        .db_pool
        .update_todo_name(payload.todo_name, payload.todo_id, state.current_list_id)
        .await
    {
        eprintln!("Fehler beim Update-Name-Todo in der Datenbank:\n{:?}", e);
    }
    Redirect::to("/")
}

pub async fn update_todo_description(
    State(state): State<AppState>,
    Json(payload): Json<UpdateTodoDescriptionForm>,
) -> Redirect {
    if let Err(e) = &state
        .db_pool
        .update_todo_description(
            Some(payload.todo_description),
            payload.todo_id,
            state.current_list_id,
        )
        .await
    {
        eprintln!(
            "Fehler beim Update-Description-Todo in der Datenbank:\n{:?}",
            e
        );
    }
    Redirect::to("/")
}

pub async fn update_due_date(
    State(state): State<AppState>,
    Form(input): Form<DueDateForm>,
) -> Redirect {
    let naive_dt = NaiveDateTime::parse_from_str(&input.due_date, "%Y-%m-%dT%H:%M");
    if let Ok(dt) = naive_dt {
        let utc_dt = dt.and_utc();
        if let Err(e) = &state
            .db_pool
            .update_due_date(utc_dt, input.todo_id, state.current_list_id)
            .await
        {
            eprintln!(
                "Fehler beim update-todo-due_date in der Datenbank:\n{:?}",
                e
            );
        }
    }
    Redirect::to("/")
}


pub async fn remove_due_date(
    State(state): State<AppState>,
    Json(payload): Json<RemoveDueDateForm>
) -> Redirect {
    if let Err(e) = state.db_pool.remove_due_date(payload.todo_id).await {
        eprintln!("Fehler beim remove-todo-due_date in der Datenbank:\n{:?}",
        e)
    }
    Redirect::to("/")
}

pub async fn update_list_title(
    State(state): State<AppState>,
    Json(payload): Json<TitleUpdateForm>,
) -> Redirect {
    if let Err(e) = &state
        .db_pool
        .update_list_title(state.current_list_id, payload.title)
        .await
    {
        eprintln!("Fehler beim Update-List-Title in der Datenbank:\n{:?}", e);
    }
    Redirect::to("/")
}

// TODO: db integration
pub async fn switch_list(
    State(mut state): State<AppState>,
    Json(payload): Json<SwitchListForm>,
) -> Redirect {
    // let list = state.db_pool.get_list_by_string(payload.list_name).await.unwrap_or(None);
    let list_id = state
        .db_pool
        .get_list_id_by_name(payload.list_name)
        .await
        .unwrap_or(None);
    {
        if let Some(id) = list_id {
            state.current_list_id = id;
        }
    }

    Redirect::to("/")
}

#[allow(dead_code)]
pub async fn get_current_list(session: ReadableSession) -> Option<String> {
    session.get::<String>("current_list_id")
}

pub async fn add_list(
    State(state): State<AppState>,
    Json(payload): Json<CreateListForm>,
) -> Redirect {
    let list = FrontendList::new(payload.list_name);
    if let Err(e) = &state.db_pool.add_list(list.id, list.title).await {
        eprintln!("Fehler beim list-add in der Datenbank:\n{:?}", e);
    }
    Redirect::to("/")
}

pub async fn delete_list(
    State(mut state): State<AppState>,
    Json(payload): Json<RemoveListForm>,
) -> Redirect {
    if let Err(e) = &state.db_pool.remove_list(payload.list_id).await {
        eprintln!("Fehler beim list-remove in der Datenbank:\n{:?}", e);
    } else {
        state.current_list_id = state
            .db_pool
            .get_lists()
            .await
            .unwrap()
            .first()
            .unwrap()
            .id
            .clone();
    }
    Redirect::to("/")
}

pub async fn rename_tag(
    State(state): State<AppState>,
    Json(payload): Json<RenameTagForm>,
) -> Redirect {
    if let Err(e) = &state
        .db_pool
        .rename_tag(payload.tag_id, payload.tag_name)
        .await
    {
        eprintln!("Fehler beim Rename-Tag in der Datenbank:\n{:?}", e);
    }
    Redirect::to("/")
}

#[allow(dead_code)]
pub async fn remove_entire_tag(
    State(state): State<AppState>,
    Json(payload): Json<RemoveTagForm>,
) -> Redirect {
    if let Err(e) = state.db_pool.remove_tag(payload.tag_id).await {
        eprintln!("Fehler beim Löschen eines Tags:\n{:?}", e);
    }
    Redirect::to("/")
}

pub async fn remove_tag_from_todo(
    State(state): State<AppState>,
    Json(payload): Json<RemoveTagForm>,
) -> Redirect {
    if let Err(e) = state
        .db_pool
        .remove_link(payload.todo_id, payload.tag_id)
        .await
    {
        eprintln!("Fehler beim Entfernen eines Tags von einemTodo:\n{:?}", e);
    }

    Redirect::to("/")
}

pub async fn add_tag(State(state): State<AppState>, Json(payload): Json<AddTagForm>) -> Redirect {
    let trimmed_string = payload.tag_name.trim().to_lowercase();
    if trimmed_string.is_empty() {
        return Redirect::to("/");
    };
    let tag_id: TagId = if let Some(tag) = state
        .db_pool
        .get_tag_by_name(trimmed_string.clone())
        .await
        .unwrap()
    {
        tag.id
    } else {
        let new_tag = Tag::new(trimmed_string, Utc::now());
        if let Err(e) = state.db_pool.add_tag_uniq(&new_tag).await {
            eprintln!("Fehler beim Hinzufügen eines Tags:\n{:?}", e);
        }
        new_tag.id
    };

    if let Err(e) = state.db_pool.link_todo_tag(payload.todo_id, tag_id).await {
        eprintln!("Fehler beim Linken von Todo und Tag:\n{:?}", e);
    }

    Redirect::to("/")
}

pub async fn todos(
    State(state): State<AppState>,
    Query(filters): Query<AllFilters>,
) -> Html<String> {
    let current_list_id = state.current_list_id;

    let mut todos = state
        .db_pool
        .get_main_todos(current_list_id.clone())
        .await
        .unwrap_or(Vec::new());

    for todo in &mut todos {
        todo.check_overdue();
    }

    let tags = state.db_pool.get_tags().await.unwrap();
    let title = state
        .db_pool
        .get_list(current_list_id.clone())
        .await
        .unwrap_or_default();
    let lists = state.db_pool.get_lists_string().await.unwrap_or_default();

    let filtered_todos = filter(&filters, &todos, &tags);
    // TODO wirft einen Fehler wegen Option Unwrap (wahrscheinlich wegen Tags)
    let frontend_todos = state
        .db_pool
        .get_frontend_todos(filtered_todos)
        .await
        .unwrap();

    // HTML parsen
    let mut tera = Tera::new("src/templates/**/*").unwrap();
    tera.register_filter("json_encode_single", json_encode_single);
    let mut context = Context::new();
    context.insert("title", &title);
    context.insert("list_id", &current_list_id);
    context.insert("lists", &lists);
    context.insert("todos", &frontend_todos);
    context.insert("tags", &tags);

    let rendered = tera.render("todos.html", &context).unwrap();
    Html(rendered)
}

pub fn json_encode_single(value: &Value, _args: &HashMap<String, Value>) -> TeraResult<Value> {
    let json_str = serde_json::to_string(value).map_err(|e| tera::Error::msg(e.to_string()))?;
    let single_quoted = json_str.replace("\"", "'");
    Ok(Value::String(single_quoted))
}

//noinspection ALL
pub async fn routes() -> Router {
    dotenv().ok(); // Lade Umgebungsvariablen aus .env

    let database_url = env::var("DATABASE_URL").expect("Datenbank muss gesetzt sein");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("Konnte nicht mit der DB verbinden");

    // Debugging für Zurücksetzen
    let result = pool.clear_all_tables().await;
    if let Err(e) = result {
        eprintln!("Fehler beim Zurücksetzen der Datenbank:\n{:?}", e);
    }

    // Liste einfügen mit Werten:
    let first_list = TodoList::new("To-dos");
    let _ = pool
        .add_list(first_list.id.clone(), first_list.title)
        .await
        .unwrap();
    let app_state = AppState::new(pool, first_list.id);

    async {
        let first_todo = Todo::new(Uuid::new_v4(), "X aufkaufen", None, true);
        let second_todo = Todo::new(
            Uuid::new_v4(),
            "Kaffee kochen",
            Some("Filterkaffee..."),
            false,
        );

        if let Err(result) = &app_state
            .db_pool
            .add_todo(&first_todo, app_state.current_list_id.clone())
            .await
        {
            eprintln!(
                "Fehler beim einsetzen des To-dos zur Initialisierung{:?}",
                result
            );
        }
        if let Err(result) = &app_state
            .db_pool
            .add_todo(&second_todo, app_state.current_list_id.clone())
            .await
        {
            eprintln!(
                "Fehler beim einsetzen des To-dos zur Initialisierung{:?}",
                result
            );
        }
    }
    .await;

    let store = MemoryStore::new();
    let session_layer = SessionLayer::new(store, b"N6dXavyE/U6IIoELbOwjDG9y47cpE9opfMQAlJmc1BgW1FaJq7orcWxec2ubuDJgZLjJjzCv1IQzxkGvwaAEs++HP8qmizMfOF3L3Ao/TLI=");

    Router::new()
        .route("/", get(todos))
        .route("/add_todo", post(new_todo))
        .route("/add_sub_todo", post(add_sub_todo))
        .route("/delete_todo", post(delete_todo))
        .route("/tick_todo", post(tick_todo))
        .route("/update_todo_name", post(update_todo_name))
        .route("/update_todo_date", post(update_due_date))
        .route("/remove_todo_date", post(remove_due_date))
        .route("/update_todo_description", post(update_todo_description))
        .route("/add_list", post(add_list))
        .route("/delete_list", post(delete_list))
        .route("/switch_list", post(switch_list))
        .route("/update_list_title", post(update_list_title))
        .route("/add_tag", post(add_tag))
        .route("/remove_tag", post(remove_tag_from_todo))
        .route("/rename_tag", post(rename_tag))
        .with_state(app_state)
        .layer(session_layer)
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::todos::filter::*;
    use chrono::NaiveDateTime;

    async fn todos_setup() -> TodoList {
        let list = TodoList::new("New List");
        let first_todo = Todo::new(
            Uuid::new_v4(),
            "First Todo",
            Some("This is the description of the first todo"),
            false,
        );
        let second_todo = Todo::new(
            Uuid::new_v4(),
            "Second Todo",
            Some("This is the description of the second todo"),
            true,
        );
        list
    }

    fn todos_vec_setup() -> Vec<Todo> {
        let first_tag = Tag::new(
            "Wonderful Tag Name".to_string(),
            DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::parse_from_str("2022-04-13 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
                Utc,
            ),
        );

        let mut list = Vec::new();
        let first_todo = Todo::new(
            Uuid::new_v4(),
            "First Todo",
            Some("This is the description of the first todo"),
            false,
        );
        let mut second_todo = Todo::new(
            Uuid::new_v4(),
            "Second Todo",
            Some("This is the description of the second todo"),
            true,
        );
        second_todo.tags.push(first_tag.id);
        list.extend(vec![first_todo, second_todo.clone()]);
        list
    }

    #[test]
    fn test_filter_by_tags() {
        let first_tag = Tag::new(
            "Wonderful Tag Name".to_string(),
            DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::parse_from_str("2022-04-13 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
                Utc,
            ),
        );
        let second_tag = Tag::new(
            "Amazing second Tag Name".to_string(),
            DateTime::<Utc>::from_naive_utc_and_offset(
                NaiveDateTime::parse_from_str("2025-04-16 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
                Utc,
            ),
        );
        let available_tags = vec![first_tag.clone(), second_tag].into_iter().collect();

        // To-dos vorbereiten
        let mut list = Vec::new();
        let first_todo = Todo::new(
            Uuid::new_v4(),
            "First Todo",
            Some("This is the description of the first todo"),
            false,
        );
        let mut second_todo = Todo::new(
            Uuid::new_v4(),
            "Second Todo",
            Some("This is the description of the second todo"),
            true,
        );
        second_todo.tags.push(first_tag.id);
        list.extend(vec![first_todo, second_todo.clone()]);

        // Filterfunktion testen
        let mut second_todo_vec = Vec::new();
        second_todo_vec.push(second_todo);
        assert_eq!(filter_by_tags(&list, &available_tags), second_todo_vec);
    }
}
