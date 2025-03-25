use uuid::Uuid;
use crate::todos::todos::TagId;
use chrono::{Local, DateTime};
use serde::{Deserialize, Serialize};
use chrono::Utc;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Todo {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub completed: bool,
    pub is_overdue: bool,
    pub tags: Vec<TagId>,
    pub parent_id: Option<String>,
    pub subtasks: Vec<Todo>,
}

impl Todo {
    pub fn new(id: Uuid, title: &str, description: Option<&str>, completed: bool) -> Self {
        Todo {
            id: String::from(id),
            title: String::from(title),
            description: description.map(|v| v.to_string()),
            due_date: None,
            created_at: Utc::now(),
            completed,
            is_overdue: false,
            tags: Vec::new(),
            parent_id: None,
            subtasks: Vec::new()
        }
    }

    pub fn tick(&mut self) -> &Todo {
        self.completed = !self.completed;
        self
    }
    
    pub fn update_description(&mut self, new_description: String) {
        self.description = {
            if new_description.is_empty() {
                None
            } else {
                Some(new_description)
            }
        };
    }
    
    pub fn update_title(&mut self, new_title: String) {
        if !new_title.is_empty() {
            self.title = new_title
        }
    }
    
    pub fn check_overdue(&mut self) {
        self.is_overdue = match self.due_date {
            Some(d) => d <= Local::now(),
            None => false,
        }
    }

    pub fn add_tag(&mut self, tag_id: TagId) {
        self.tags.push(tag_id);
    }

    #[allow(dead_code)]
    pub fn remove_tag(&mut self, tag_id: &TagId) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag_id) {
            self.tags.remove(pos);
        }
    }

    pub fn matches_query(&self, query: &str) -> bool {
        self.title.to_lowercase().contains(&query.to_lowercase())
            || self
                .description
                .as_ref()
                .map_or(false, |desc| desc.to_lowercase().contains(&query))
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;

    fn new_description_todo() -> Todo {
        Todo::new(
            Uuid::new_v4(),
            "new_todo",
            Some("This is the description."),
            false,
        )
    }

    fn new_todo() -> Todo {
        Todo::new(Uuid::new_v4(), "new_todo", None, false)
    }

    #[test]
    fn test_tick() {
        let mut todo = new_description_todo();
        todo.tick();
        assert_eq!(todo.completed, true);
    }

    #[test]
    fn test_overdue_check() {
        let mut new_todo = Todo {
            id: "1234".to_string(),
            title: "New Title".to_string(),
            description: Some("New_Description".to_string()),
            due_date: Some(
                DateTime::parse_from_str("1983 Apr 13 12:09:14.274 +0000", "%Y %b %d %H:%M:%S%.3f %z")
                    .unwrap()
                    .with_timezone(&Utc)
            ),
            created_at: Utc::now(),
            completed: false,
            is_overdue: false,
            tags: Vec::new(),
            parent_id: None,
            subtasks: Vec::new()
        };
        new_todo.check_overdue();
        assert_eq!(new_todo.is_overdue, true);
    }
    
    #[test]
    fn test_update_description() {
        let mut new_todo = new_description_todo();
        new_todo.update_description("the description has changed.".to_string());
        assert_eq!("the description has changed.".to_string(), new_todo.description.unwrap())
    }
}
