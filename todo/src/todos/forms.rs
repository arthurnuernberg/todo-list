use crate::todos::todos::{Tag, TodoId};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{de, Deserialize, Deserializer, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrontendTodo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub completed: bool,
    pub is_overdue: bool,
    pub tags: Vec<Tag>,
    pub parent_id: Option<TodoId>,
    pub subtasks: Vec<FrontendTodo>
}

impl Hash for FrontendTodo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
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

    #[serde(default, deserialize_with = "deserialize_comma_separated")]
    pub tags: Vec<String>,

    #[serde(default)]
    pub list: String,
    
    #[serde(default)]
    pub sort: Option<String>,
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

// Form-Daten
#[derive(Deserialize)]
pub struct TickForm {
    pub todo_id: String,
}

#[derive(Deserialize)]
pub struct RemoveTodoForm {
    pub todo_id: String,
}

#[derive(Deserialize)]
pub struct UpdateTodoNameForm {
    pub todo_id: String,
    pub todo_name: String,
}

#[derive(Deserialize)]
pub struct UpdateTodoDescriptionForm {
    pub todo_id: String,
    pub todo_description: String,
}
#[derive(Deserialize)]
pub struct NewTodoForm {
    pub todo_title: String,
}

#[derive(Deserialize)]
pub struct NewSubTodoForm {
    pub sub_todo_title: String,
    pub todo_id: String,
}

#[derive(Deserialize)]
pub struct TitleUpdateForm {
    pub title: String,
}

#[derive(Deserialize)]
pub struct DueDateForm {
    pub todo_id: String,
    pub due_date: String,
}

#[derive(Deserialize)]
pub struct SwitchListForm {
    pub list_name: String,
}

#[derive(Deserialize)]
pub struct CreateListForm {
    pub list_name: String,
}

#[derive(Deserialize)]
pub struct RemoveListForm {
    pub list_id: String,
}

#[derive(Deserialize)]
pub struct RenameTagForm {
    pub tag_id: String,
    pub tag_name: String,
}

#[derive(Deserialize)]
pub struct RemoveTagForm {
    pub todo_id: String,
    pub tag_id: String,
}
#[derive(Deserialize)]
pub struct AddTagForm {
    pub todo_id: String,
    pub tag_name: String,
}

#[derive(Deserialize)]
pub struct RemoveDueDateForm {
    pub todo_id: TodoId,
}
