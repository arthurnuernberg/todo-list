#[cfg(test)]
mod tests {
    use todo::todos::filter::*;
    use todo::todos::tasks::*;
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
