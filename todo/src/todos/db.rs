use std::mem::offset_of;
use crate::todos::todo::Todo;
use chrono::{DateTime, TimeZone, Timelike, Utc};
use serde_json::Value::Null;
use sqlx::types::time::{OffsetDateTime, UtcOffset};
use sqlx::PgPool;

type TodoId = String;

fn to_utc(offset_dt: OffsetDateTime) -> DateTime<Utc> {
    let year = offset_dt.year();
    let month = offset_dt.month() as u32;
    let day = offset_dt.day();
    let hour = offset_dt.hour();
    let minute = offset_dt.minute();
    let second = offset_dt.second();
    let nanosecond = offset_dt.nanosecond();

    Utc.with_ymd_and_hms(
        year,
        month,
        day as u32,
        hour as u32,
        minute as u32,
        second as u32,
    )
    .single()
    .map(|dt| dt.with_nanosecond(nanosecond).unwrap_or(dt))
    .unwrap_or_else(|| Utc::now())
}

fn to_offset(utc_dt: DateTime<Utc>) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(utc_dt.timestamp_nanos_opt().unwrap() as i128).expect("Fehler beim Parsen von DateTime<Utc> zu OffsetDateTime.").to_offset(UtcOffset::UTC)
}

pub async fn insert_todo(pool: &PgPool, todo: &Todo) -> Result<(), sqlx::Error> {
    
    sqlx::query!(
        r#"
        INSERT INTO todos_db.public.todos (
            id,
            title,
            description,
            due_date,
            created_at,
            completed,
            is_overdue
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7
        )
        "#,
        todo.id,
        todo.title,
        todo.description,
        todo.due_date.map(to_offset),
        to_offset(todo.created_at),
        todo.completed,
        todo.is_overdue,
    )
    .execute(pool)
    .await?;

    // Tags separat einfügen
    for tag_id in &todo.tags {
        sqlx::query!(
            r#"
            INSERT INTO todos_db.public.todo_tags (todo_id, tag_id)
            VALUES ($1, $2)
            "#,
            todo.id,
            tag_id
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn tick_todo_db(pool: &PgPool, todo_id: TodoId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            UPDATE todos_db.public.todos
            SET completed = NOT completed
            WHERE id = $1
        "#, todo_id
    ).execute(pool).await?;

    Ok(())
}

pub async fn remove_todo(pool: &PgPool, todo_id: TodoId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            DELETE FROM todos_db.public.todos
            WHERE id = $1
        "#, todo_id
    ).execute(pool).await?;

    Ok(())
}

pub async fn get_todos(pool: &PgPool) -> Result<Vec<Todo>, sqlx::Error> {
    // Basis-Informationen der To-dos abrufen
    let base_todos = sqlx::query!(
        r#"
        SELECT
            id,
            title,
            description,
            due_date,
            created_at,
            completed
        FROM todos_db.public.todos
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut todos = Vec::new();

    for row in base_todos {
        // Tags für das jeweilige Todo laden
        let tags: Vec<String> = sqlx::query!(
            r#"
            SELECT tag_id
            FROM todos_db.public.todo_tags
            WHERE todo_id = $1
            "#,
            row.id.to_string()
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|t| t.tag_id)
        .collect();

        let created_at = to_utc(row.created_at);

        let mut due_date: Option<DateTime<Utc>> = None;
        if let Some(date) = row.due_date {
            due_date = Some(to_utc(date));
        }

        let mut todo = Todo {
            id: row.id.to_string(),
            title: row.title,
            description: row.description,
            due_date,
            created_at,
            completed: row.completed,
            is_overdue: false,
            tags,
        };

        todo.check_overdue();

        todos.push(todo);
    }

    Ok(todos)
}
