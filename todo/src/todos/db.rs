use crate::todos::forms::FrontendTodo;
use crate::todos::todo::Todo;
use crate::todos::todos::{Tag, TagId, TodoId, TodoList, TodoListId};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::future::join_all;
use serde::Deserialize;
use serde::Serialize;
use sqlx::{Error, PgPool};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct DBList {
    id: String,
    title: String,
}

#[async_trait]
pub trait TodoDatabaseExt {
    async fn clear_all_tables(&self) -> Result<(), Error>;
    async fn add_todo(&self, todo: &Todo, list_id: TodoListId) -> Result<(), Error>;
    async fn tick_todo(&self, list_id: TodoListId, todo_id: TodoId) -> Result<(), Error>;
    async fn remove_todo(&self, list_id: TodoListId, todo_id: TodoId) -> Result<(), Error>;
    async fn update_todo_name(
        &self,
        todo_name: String,
        todo_id: TodoId,
        list_id: TodoListId,
    ) -> Result<(), Error>;
    async fn update_todo_description(
        &self,
        description: Option<String>,
        todo_id: TodoId,
        list_id: TodoListId,
    ) -> Result<(), Error>;
    async fn update_due_date(
        &self,
        due_date: DateTime<Utc>,
        todo_id: TodoId,
        list_id: TodoListId,
    ) -> Result<(), Error>;
    async fn rename_tag(&self, tag_name: String, tag_id: TagId) -> Result<(), Error>;
    async fn remove_tag(&self, tag_id: TagId) -> Result<(), Error>;
    async fn add_tag(&self, tag: &Tag) -> Result<(), Error>;
    async fn add_tag_uniq(&self, tag: &Tag) -> Result<(), Error>;
    async fn link_todo_tag(&self, todo_id: TodoId, tag_id: TagId) -> Result<(), Error>;
    async fn remove_link(&self, todo_id: TodoId, tag_id: TagId) -> Result<(), Error>;
    async fn update_list_title(&self, list_id: TodoListId, list_name: String) -> Result<(), Error>;
    async fn add_list(&self, list_id: TodoListId, list_name: String) -> Result<(), Error>;
    async fn add_list_uniq(&self, list_id: TodoListId, list_name: String) -> Result<(), Error>;
    async fn remove_list(&self, list_id: TodoListId) -> Result<(), Error>;
    async fn get_todos(&self, list_id: TodoListId) -> Result<Vec<Todo>, Error>;
    async fn get_frontend_todos(&self, list_id: TodoListId) -> Result<Vec<FrontendTodo>, Error>;
    async fn get_todo(&self, todo_id: TodoId) -> Result<Todo, Error>;
    async fn get_tag(&self, tag_id: TagId) -> Result<Option<Tag>, Error>;
    async fn get_tag_by_name(&self, tag_name: String) -> Result<Option<Tag>, Error>;
    async fn get_tags(&self) -> Result<Vec<Tag>, Error>;
    async fn get_list(&self, list_id: TodoListId) -> Result<String, Error>;
    async fn get_list_by_string(&self, list_name: String) -> Result<Option<FrontendList>, Error>;
    async fn get_list_id_by_name(&self, list_name: String) -> Result<Option<String>, Error>;
    async fn get_lists(&self) -> Result<Vec<FrontendList>, Error>;
    async fn get_lists_string(&self) -> Result<Vec<String>, Error>;
}

#[async_trait]
impl TodoDatabaseExt for PgPool {
    async fn clear_all_tables(&self) -> Result<(), Error> {
        sqlx::query!("TRUNCATE TABLE todos_db.public.todo_tags RESTART IDENTITY CASCADE")
            .execute(self)
            .await?;
        sqlx::query!("TRUNCATE TABLE todos_db.public.todos RESTART IDENTITY CASCADE")
            .execute(self)
            .await?;
        sqlx::query!("TRUNCATE TABLE todos_db.public.todo_lists RESTART IDENTITY CASCADE")
            .execute(self)
            .await?;
        sqlx::query!("TRUNCATE TABLE todos_db.public.tags RESTART IDENTITY CASCADE")
            .execute(self)
            .await?;
        Ok(())
    }

    async fn add_todo(&self, todo: &Todo, list_id: TodoListId) -> Result<(), Error> {
        sqlx::query!(
            r#"
        INSERT INTO todos_db.public.todos (
            id,
            list_id,                                           
            title,
            description,
            due_date,
            created_at,
            completed,
            is_overdue
        )
        VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8
        )
        "#,
            todo.id,
            list_id,
            todo.title,
            todo.description,
            todo.due_date.map(conversions::to_offset),
            conversions::to_offset(todo.created_at),
            todo.completed,
            todo.is_overdue,
        )
        .execute(self)
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
            .execute(self)
            .await?;
        }

        Ok(())
    }

    async fn tick_todo(&self, todo_id: TodoId, list_id: TodoListId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE todos_db.public.todos
            SET completed = NOT completed
            WHERE id = $1 AND list_id = $2
        "#,
            todo_id,
            list_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn remove_todo(&self, list_id: TodoListId, todo_id: TodoId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            DELETE FROM todos_db.public.todos
            WHERE id = $1 AND list_id = $2
        "#,
            todo_id,
            list_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn update_todo_name(
        &self,
        todo_name: String,
        todo_id: TodoId,
        list_id: TodoListId,
    ) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE todos_db.public.todos
            SET title = $1
            WHERE id = $2 AND list_id = $3
        "#,
            todo_name,
            todo_id,
            list_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn update_todo_description(
        &self,
        description: Option<String>,
        todo_id: TodoId,
        list_id: TodoListId,
    ) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE todos_db.public.todos
            SET description = $1
            WHERE id = $2 AND list_id = $3
        "#,
            description,
            todo_id,
            list_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn update_due_date(
        &self,
        due_date: DateTime<Utc>,
        todo_id: TodoId,
        list_id: TodoListId,
    ) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE todos_db.public.todos
            SET due_date = $1
            WHERE id = $2 AND list_id = $3
        "#,
            conversions::to_offset(due_date),
            todo_id,
            list_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn rename_tag(&self, tag_id: TagId, tag_name: String) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE todos_db.public.tags
            SET name = $1
            WHERE id = $2
        "#,
            tag_name,
            tag_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn remove_tag(&self, tag_id: TagId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            DELETE FROM tags
            WHERE id = $1
        "#,
            tag_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    // TODO Fügt nur den Tag der globalen Liste hinzu, nicht aber einem spezifischen To-do
    async fn add_tag(&self, tag: &Tag) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO todos_db.public.tags (
            id,
          name, created_at
        )
        VALUES (
            $1, $2, $3
        )
        "#,
            tag.id,
            tag.name,
            conversions::to_offset(tag.created_at)
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn add_tag_uniq(&self, tag: &Tag) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO todos_db.public.tags (id, name, created_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (name) DO NOTHING
        "#,
            tag.id,
            tag.name,
            conversions::to_offset(tag.created_at)
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn link_todo_tag(&self, todo_id: TodoId, tag_id: TagId) -> Result<(), Error> {
        sqlx::query!(
            r#"
                INSERT INTO todo_tags
                (todo_id, tag_id)
                VALUES ($1, $2)
            "#,
            todo_id,
            tag_id
        )
        .execute(self)
        .await?;
        Ok(())
    }

    async fn remove_link(&self, todo_id: TodoId, tag_id: TagId) -> Result<(), Error> {
        sqlx::query!(
            r#"
                DELETE FROM todo_tags
                WHERE todo_id = $1 AND tag_id = $2
            "#,
            todo_id,
            tag_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn update_list_title(&self, list_id: TodoListId, list_name: String) -> Result<(), Error> {
        sqlx::query!(
            r#"
            UPDATE todo_lists
            SET title = $1
            WHERE id = $2
        "#,
            list_name,
            list_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn add_list(&self, list_id: TodoListId, list_name: String) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO todos_db.public.todo_lists (id, title)
            VALUES ($1, $2)
        "#,
            list_id,
            list_name
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn add_list_uniq(&self, list_id: TodoListId, list_name: String) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO todos_db.public.todo_lists (id, title)
            VALUES ($1, $2)
            ON CONFLICT (id) DO NOTHING
        "#,
            list_id,
            list_name
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn remove_list(&self, list_id: TodoListId) -> Result<(), Error> {
        sqlx::query!(
            r#"
            DELETE FROM todos_db.public.todo_lists
            WHERE id = $1
        "#,
            list_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    async fn get_todos(&self, list_id: TodoListId) -> Result<Vec<Todo>, Error> {
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
        WHERE list_id = $1
        ORDER BY created_at DESC
        "#,
            list_id
        )
        .fetch_all(self)
        .await?;

        let mut todos = Vec::new();

        for row in base_todos {
            // Tags für das jeweilige To-do laden
            let tags: Vec<String> = sqlx::query!(
                r#"
            SELECT tag_id
            FROM todos_db.public.todo_tags
            WHERE todo_id = $1
            "#,
                row.id.to_string()
            )
            .fetch_all(self)
            .await?
            .into_iter()
            .map(|t| t.tag_id)
            .collect();

            let created_at = conversions::to_utc(row.created_at);

            let mut due_date: Option<DateTime<Utc>> = None;
            if let Some(date) = row.due_date {
                due_date = Some(conversions::to_utc(date));
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

    async fn get_frontend_todos(&self, list_id: TodoListId) -> Result<Vec<FrontendTodo>, Error> {
        let todos = self.get_todos(list_id).await?;
        let mut frontend = Vec::with_capacity(todos.len());
        for todo in todos {
            let tag_futures = todo
                .tags
                .into_iter()
                .map(|tag_id| self.get_tag(tag_id))
                .collect::<Vec<_>>();
            let tag_results = join_all(tag_futures).await;
            let mut tags = Vec::new();
            for res in tag_results {
                if let Some(tag) = res? {
                    tags.push(tag);
                }
            }
            frontend.push(FrontendTodo {
                id: todo.id,
                title: todo.title,
                description: todo.description,
                due_date: todo.due_date,
                created_at: todo.created_at,
                completed: todo.completed,
                is_overdue: todo.is_overdue,
                tags,
            });
        }
        Ok(frontend)
    }

    async fn get_todo(&self, todo_id: TodoId) -> Result<Todo, Error> {
        let record = sqlx::query!(
            r#"
                SELECT * FROM todos
                WHERE id = $1
            "#,
            todo_id
        )
        .fetch_one(self)
        .await?;
        let tags = sqlx::query!(
            r#"
                SELECT * FROM tags
                WHERE id = (SELECT tag_id FROM todo_tags
                WHERE todo_id = $1)
            "#,
            todo_id
        )
        .fetch_all(self)
        .await?;

        let tag_list: Vec<TodoId> = tags.iter().map(|rec| rec.id.clone()).collect();
        let todo = Todo {
            id: record.id,
            title: record.title,
            description: record.description,
            due_date: record.due_date.map(conversions::to_utc),
            created_at: conversions::to_utc(record.created_at),
            completed: record.completed,
            is_overdue: false,
            tags: tag_list,
        };
        Ok(todo)
    }

    async fn get_tag(&self, tag_id: TagId) -> Result<Option<Tag>, Error> {
        let record = sqlx::query!(
            r#"
        SELECT * FROM tags
        WHERE id = $1
        "#,
            tag_id
        )
        .fetch_optional(self)
        .await?;
        Ok(record.map(|r| Tag {
            id: r.id,
            name: r.name,
            created_at: conversions::to_utc(r.created_at),
        }))
    }

    async fn get_tag_by_name(&self, tag_name: String) -> Result<Option<Tag>, Error> {
        let record = sqlx::query!(
            r#"
        SELECT * FROM tags
        WHERE name = $1
        "#,
            tag_name.trim().to_lowercase()
        )
        .fetch_optional(self)
        .await?;
        Ok(record.map(|r| Tag {
            id: r.id,
            name: r.name,
            created_at: conversions::to_utc(r.created_at),
        }))
    }

    async fn get_tags(&self) -> Result<Vec<Tag>, Error> {
        let records = sqlx::query!(
            r#"
                SELECT * FROM tags
            "#
        )
        .fetch_all(self)
        .await?;
        let tags: Vec<Tag> = records
            .iter()
            .map(|rec| {
                return Tag {
                    id: rec.id.clone(),
                    name: rec.name.clone(),
                    created_at: conversions::to_utc(rec.created_at),
                };
            })
            .collect();
        Ok(tags)
    }

    async fn get_list(&self, list_id: TodoListId) -> Result<String, Error> {
        sqlx::query!(
            r#"
                SELECT * FROM todo_lists
                WHERE id = $1
            "#,
            list_id
        )
        .fetch_one(self)
        .await
        .map(|rec| rec.title)
    }

    async fn get_list_by_string(&self, list_name: String) -> Result<Option<FrontendList>, Error> {
        let list = sqlx::query!(
            r#"
                SELECT * FROM todo_lists
                WHERE title = $1
            "#,
            list_name
        )
        .fetch_optional(self)
        .await?;
        // .map(|r| {let todos = self.get_todos(r.unwrap().id).await?;FrontendList {r.id.clone(), r.title.clone(), todos})
        if let Some(r) = list {
            let todos = self.get_todos(r.id.clone()).await?;
            return Ok(Some(FrontendList {
                id: r.id,
                title: r.title.clone(),
                todos,
            }));
        };
        Ok(None)
    }

    async fn get_list_id_by_name(&self, list_name: String) -> Result<Option<String>, Error> {
        Ok(sqlx::query!(
            r#"
                SELECT * FROM todo_lists
                WHERE title = $1
            "#,
            list_name
        )
        .fetch_optional(self)
        .await?
        .map(|r| r.id))
    }

    async fn get_lists(&self) -> Result<Vec<FrontendList>, Error> {
        let rows = sqlx::query!(
            r#"
        SELECT id, title FROM todo_lists AS list;
        "#,
        )
        .fetch_all(self)
        .await?;
        let mut lists = Vec::with_capacity(rows.len());
        for row in rows {
            let todos = self.get_todos(row.id.clone()).await?;
            lists.push(FrontendList {
                id: row.id,
                title: row.title,
                todos,
            });
        }
        Ok(lists)
    }

    async fn get_lists_string(&self) -> Result<Vec<String>, Error> {
        let list_strings = sqlx::query!(
            r#"
                SELECT title FROM todo_lists
            "#
        )
        .fetch_all(self)
        .await
        .unwrap_or_default();
        let lists: Vec<TodoListId> = list_strings.iter().map(|rec| rec.title.clone()).collect();
        Ok(lists)
    }
}

#[derive(Serialize, Deserialize)]
pub struct FrontendList {
    pub id: String,
    pub title: String,
    pub todos: Vec<Todo>,
}

impl FrontendList {
    pub fn new(name: String) -> Self {
        Self {
            id: String::from(Uuid::new_v4()),
            title: String::from(name),
            todos: Vec::new(),
        }
    }
}

/// # Trennung hier von Implementation
mod conversions {
    use chrono::{DateTime, TimeZone, Timelike, Utc};
    use sqlx::types::time::{OffsetDateTime, UtcOffset};

    pub fn to_utc(offset_dt: OffsetDateTime) -> DateTime<Utc> {
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

    pub fn to_offset(utc_dt: DateTime<Utc>) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp_nanos(utc_dt.timestamp_nanos_opt().unwrap() as i128)
            .expect("Fehler beim Parsen von DateTime<Utc> zu OffsetDateTime.")
            .to_offset(UtcOffset::UTC)
    }
}
