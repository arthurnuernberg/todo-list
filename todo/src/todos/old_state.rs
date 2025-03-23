#![allow(dead_code)]
#![deprecated]
use crate::todos::db::TodoDatabaseExt;
use crate::todos::filter::filter;
use crate::todos::forms::{
    AddTagForm, AllFilters, CreateListForm, DueDateForm, NewTodoForm, RemoveListForm,
    RemoveTagForm, RenameTagForm, SwitchListForm, TickForm, TitleUpdateForm,
    UpdateTodoDescriptionForm, UpdateTodoNameForm,
};
use crate::todos::todo::Todo;
use crate::todos::todos::{json_encode_single, AppState, Tag, TodoList};
use axum::extract::{Query, State};
use axum::response::{Html, Redirect};
use axum::{Form, Json};
use axum_sessions::extractors::ReadableSession;
use chrono::{NaiveDateTime, Utc};
use futures::future::join_all;
use std::sync::Arc;
use tera::{Context, Tera};
use tokio::sync::Mutex;
use uuid::Uuid;

async fn new_todo(State(state): State<AppState>, Json(payload): Json<NewTodoForm>) -> Redirect {
    let current_todos = state.get_current_list().await;
    let mut todos = current_todos.todos.lock().await;
    if payload.todo_title.is_empty() {
        return Redirect::to("/");
    }

    let new_todo: Todo = Todo::new(Uuid::new_v4(), payload.todo_title.as_str(), None, false);
    todos.insert(0, new_todo.clone());
    let list_id = state.get_current_list().await.id.lock().await.clone();
    let result = state.db_pool.add_todo(&new_todo, list_id).await;
    if let Err(res) = result {
        eprintln!(
            "Fehler beim Hinzufügen eines To-dos in die Datenbank:\n{:?}",
            res
        );
    }
    Redirect::to("/")
}

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
        let utc_dt = dt.and_utc();

        let current_todos = state.get_current_list().await;
        let mut todos = current_todos.todos.lock().await;
        if let Some(todo) = todos.iter_mut().find(|t| t.id == input.todo_id) {
            todo.due_date = Some(utc_dt);
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

// TODO: db integration
pub async fn switch_list(
    State(state): State<AppState>,
    Json(payload): Json<SwitchListForm>,
) -> Redirect {
    if let Some(new_id) = state.find_list_id_by_name(payload.list_name.as_str()).await {
        *state.current_list_id.lock().await = new_id;
    }
    Redirect::to("/")
}

pub async fn get_current_list(session: ReadableSession) -> Option<String> {
    session.get::<String>("current_list_id")
}

pub async fn add_list(
    State(state): State<AppState>,
    Json(payload): Json<CreateListForm>,
) -> Redirect {
    if !state.get_titles().await.contains(&payload.list_name) {
        let new_list = TodoList::new(payload.list_name.as_str());
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
        eprintln!(
            "Der folgende Tag konnte nicht umbenannt werden: {}",
            payload.tag_id
        );
    }
    Redirect::to("/")
}

pub async fn remove_tag_from_todo(
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
        let new_tag = Tag::new(payload.tag_name.clone(), Utc::now());
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

    // Für Datenbank Error fixen
    /*let current_list_clone = current_list.clone();
    let error = add_list_uniq(
        &state.db_pool,
        current_list_clone.id.lock().await.clone(),
        current_list_clone.title.lock().await.clone(),
    )
    .await;
    if let Err(e) = error {
        eprintln!("Fehler beim Hinzufügen einer Liste in die Datenbank:\n{:?}", e);
    }*/

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
    let mut tera = Tera::new("src/templates/**/*").unwrap();
    tera.register_filter("json_encode_single", json_encode_single);
    let mut context = Context::new();
    context.insert("title", &title);
    context.insert("list_id", &current_list_id);
    context.insert("lists", &todo_lists);
    context.insert("todos", &frontend_todos);
    context.insert("tags", &tags);

    let rendered = tera.render("todos.html", &context).unwrap();
    Html(rendered)
}
