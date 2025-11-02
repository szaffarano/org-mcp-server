use rowan::ast::AstNode;
use std::fs;

use orgize::{
    Org,
    ast::{Headline, Timestamp},
    export::{Container, Event, from_fn},
};

fn main() {
    let content = fs::read_to_string("org-core/examples/simple.org").expect("Failed to read file");

    let org = Org::parse(content);

    let mut handler = from_fn(|event| {
        if let Event::Enter(Container::Headline(h)) = event {
            let title = h.title_raw();
            let scheduled = h.scheduled();
            let deadline = h.deadline();

            let timestamps = h
                .syntax()
                .children()
                .filter(|c| !Headline::can_cast(c.kind()))
                .flat_map(|node| node.descendants().filter_map(Timestamp::cast))
                .filter(|ts| ts.is_active())
                .filter(|ts| {
                    h.scheduled().map(|s| &s != ts).unwrap_or(true)
                        && h.deadline().map(|s| &s != ts).unwrap_or(true)
                })
                .map(|ts| ts.raw())
                .collect::<Vec<_>>();

            let indent = "  ".repeat(h.level() - 1);
            println!("{indent}Found headline: {title}");
            println!("{indent}  Scheduled: {:?}", scheduled.map(|d| d.raw()));
            println!("{indent}  Deadline: {:?}", deadline.map(|d| d.raw()));
            println!("{indent}  Timestamps: {:?}", timestamps);
            println!("--------------------------");
        }
    });

    org.traverse(&mut handler);
}
