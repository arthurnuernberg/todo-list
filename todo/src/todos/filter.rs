use crate::todos::forms::AllFilters;
use crate::todos::todo::Todo;
use crate::todos::todos::Tag;
use chrono::NaiveDateTime;
use sorting::*;

pub fn filter_by_date_range(
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

pub fn filter_by_tags(todos: &Vec<Todo>, tags: &Vec<Tag>) -> Vec<Todo> {
    todos
        .iter()
        .filter(|todo| {
            todo.tags
                .iter()
                .any(|tag| tags.iter().any(|t| t.id.eq(tag)))
        })
        .cloned()
        .collect()
}

pub fn search_tasks(tasks: &[Todo], query: &str) -> Vec<Todo> {
    tasks
        .iter()
        .filter(|todo| todo.matches_query(query))
        .cloned()
        .collect()
}

pub fn filter_by_status(todos: &Vec<Todo>, status: bool) -> Vec<Todo> {
    todos
        .iter()
        .filter(|todo| todo.completed == status)
        .cloned()
        .collect()
}

pub fn filter_by_due(todos: &Vec<Todo>, is_due: bool) -> Vec<Todo> {
    todos
        .iter()
        .filter(|todo| todo.is_overdue == is_due)
        .cloned()
        .collect()
}

pub fn filter(
    filters: &AllFilters,
    todos: &Vec<Todo>,
    tags: &Vec<Tag>,
) -> Vec<Todo> {
    let mut filtered_todos = todos.to_vec();
    // Start- und Enddatum
    if let (Some(start), Some(end)) = (filters.start_date, filters.end_date) {
        filtered_todos = filter_by_date_range(&filtered_todos, Some(start), Some(end));
    }
    // Such-query
    if let Some(query) = &filters.query {
        filtered_todos = search_tasks(filtered_todos.as_slice(), query.as_str());
    }
    // Status
    if let Some(status) = filters.completed {
        filtered_todos = filter_by_status(&filtered_todos, status);
    }
    // Fälligkeit
    if let Some(is_due) = filters.is_due {
        filtered_todos = filter_by_due(&filtered_todos, is_due);
    }
    // TODO: Tag-Filter überprüfen
    if !filters.tags.is_empty() {
        filtered_todos = filter_by_tags(&filtered_todos, tags);
    }
    filtered_todos = match filters.sort.as_deref() {
        Some("title_asc")      => sort_by_title(&filtered_todos),
        Some("title_desc")     => sort_by_title_descending(&filtered_todos),
        Some("date_asc")       => sort_by_date(&filtered_todos),
        Some("date_desc")      => sort_by_date_descending(&filtered_todos),
        Some("uncompleted")    => uncompleted_first(&filtered_todos),
        Some("completed")      => completed_first(&filtered_todos),
        Some("overdue")        => overdue_first(&filtered_todos),
        Some("undue")          => undue_first(&filtered_todos),
        Some("last_created")   => last_created(&filtered_todos),
        Some("first_created")  => first_created(&filtered_todos),
        None                   => filtered_todos,
        _ => filtered_todos
    };
    filtered_todos.clone()
}

pub mod sorting {
    use std::cmp::Reverse;
    use crate::todos::todo::Todo;

    pub fn sort_by_title(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (todo.title.clone(), todo.description.clone().unwrap_or_default()));
        sorted_todos
    }

    pub fn sort_by_title_descending(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (Reverse(todo.title.clone()), Reverse(todo.description.clone().unwrap_or_default())));
        sorted_todos
    }

    pub fn sort_by_date(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| todo.due_date.unwrap_or_default());
        sorted_todos
    }


    pub fn sort_by_date_descending(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| Reverse(todo.due_date.unwrap_or_default()));
        sorted_todos
    }

    pub fn uncompleted_first(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (todo.completed, todo.title.clone()));
        sorted_todos
    }

    pub fn completed_first(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (!todo.completed, todo.title.clone()));
        sorted_todos
    }

    pub fn overdue_first(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (!todo.is_overdue, todo.due_date.unwrap_or_default().clone()));
        sorted_todos
    }

    pub fn undue_first(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (todo.is_overdue, todo.due_date.unwrap_or_default().clone()));
        sorted_todos
    }
    
    pub fn last_created(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (Reverse(todo.created_at), todo.title.clone()));
        sorted_todos
    }

    pub fn first_created(todos: &Vec<Todo>) -> Vec<Todo> {
        let mut sorted_todos = todos.clone();
        sorted_todos.sort_by_key(|todo| (todo.created_at, todo.title.clone()));
        sorted_todos
    }
}