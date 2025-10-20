use std::fs;

use org_core::AgendaItem;
use orgize::{
    ParseConfig,
    export::{Container, Event, Traverser},
};

fn main() {
    let content = fs::read_to_string("org-core/examples/refile.org").expect("Failed to read file");

    let config = ParseConfig {
        todo_keywords: (
            vec!["TODO".to_string(), "PROGRESS".to_string()],
            vec!["DONE".to_string()],
        ),
        ..Default::default()
    };
    let org = config.parse(content);

    let mut todos = TodoList { tasks: Vec::new() };

    org.traverse(&mut todos);

    println!("Got {} unfinished tasks", todos.tasks.len());
}

struct TodoList {
    tasks: Vec<AgendaItem>,
}

impl Traverser for TodoList {
    fn event(&mut self, event: Event, _ctx: &mut orgize::export::TraversalContext) {
        if let Event::Enter(container) = event
            && let Container::Headline(headline) = container
            && headline.is_todo()
        {
            let task = AgendaItem {
                file_path: "notes.org".to_string(),
                heading: headline.title_raw(),
                level: headline.level(),
                todo_state: Some("TODO".to_string()),
                priority: Some("A".to_string()),
                deadline: headline.deadline().map(|d| d.raw()),
                scheduled: headline.scheduled().map(|d| d.raw()),
                tags: vec![],
                line_number: Some(8),
            };
            self.tasks.push(task);
        }
    }
}
