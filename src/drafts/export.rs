//! A draft as a Markdown file the user keeps.

use std::collections::BTreeMap;

use serde_yaml::Value;

use super::schema::{Draft, DraftFields};

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn optional(value: &Option<String>) -> Value {
    value.as_deref().map_or(Value::Null, text)
}

/// The draft as Markdown: a YAML front matter of its named fields, then
/// its body. The front matter has no `id` and no `manyhands_managed`, so
/// the file cannot be taken for an item; placed under `docs/` it is a
/// nonconforming entry. A comment draft is its body only.
pub fn markdown(draft: &Draft) -> String {
    let mut front: BTreeMap<&str, Value> = BTreeMap::new();
    let body = match &draft.current {
        DraftFields::Comment { body } => return body.clone(),
        DraftFields::Document { title, body } => {
            front.insert("title", text(title));
            body
        }
        DraftFields::Ticket {
            title,
            ticket_type,
            status,
            project,
            team,
            deps,
            parent,
            body,
        } => {
            front.insert("title", text(title));
            front.insert("type", text(ticket_type));
            front.insert("status", text(status));
            front.insert("project", optional(project));
            front.insert("team", optional(team));
            front.insert(
                "deps",
                Value::Sequence(deps.iter().map(|dep| text(dep)).collect()),
            );
            front.insert("parent", optional(parent));
            body
        }
    };
    let yaml = serde_yaml::to_string(&front).expect("a map of strings serializes");
    format!("---\n{yaml}---\n{body}")
}
