use crate::todos::tasks::{Tag, TodoList};
use crate::todos::todo::*;
use serde_json::Result as JsonResult;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct JsonExport {
    pub lists: Vec<TodoList>,
    pub tags: Vec<Tag>,
}

pub fn get_todos_json(todos: &Vec<Todo>) -> JsonResult<String> {
    serde_json::to_string_pretty(todos)
}

pub fn print_todos(todos: &Vec<Todo>) {
    print!("Todos:\n{}", get_todos_json(todos).unwrap());
}

pub fn get_lists_json(lists: &Vec<TodoList>) -> JsonResult<String> {
    serde_json::to_string_pretty(lists)
}

pub fn print_lists(lists: &Vec<TodoList>) {
    print!("Listen:\n{}", get_lists_json(lists).unwrap());
}

pub fn get_tags_json(tags: &Vec<Tag>) -> JsonResult<String> {
    serde_json::to_string_pretty(tags)
}

pub fn print_tags(tags: &Vec<Tag>) {
    print!("Tags:\n{}", get_tags_json(tags).unwrap())
}

pub fn get_json(lists: &Vec<TodoList>, tags: &Vec<Tag>) -> JsonResult<String> {
    let new_export = JsonExport {
        lists: lists.clone(),
        tags: tags.clone(),
    };
    serde_json::to_string_pretty(&new_export)
}

pub fn print_json(lists: &Vec<TodoList>, tags: &Vec<Tag>) {
    print!("JSON:\n{}", get_json(lists, tags).unwrap())
}

pub fn export_file(lists: &Vec<TodoList>, tags: &Vec<Tag>) {
    let now = Utc::now();
    let filename = now.format("%Y%m%d_%H%M%S_todos_export.json").to_string();
    let content = get_json(lists, tags).unwrap();
    fs::write(&filename, content.as_str()).unwrap();
}