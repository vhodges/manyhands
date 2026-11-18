use super::export::markdown;
use super::schema::*;
use crate::canonical::ItemId;
use time::OffsetDateTime;

fn id(text: &str) -> ItemId {
    text.parse().expect("item id")
}

fn draft(item_kind: ItemKind, current: DraftFields) -> Draft {
    let now = OffsetDateTime::from_unix_timestamp(1_792_000_000).unwrap();
    Draft {
        id: "01J9ZQ6V0Y8K3N4M5P6Q7R8S9T".parse().expect("draft id"),
        revision: 2,
        created_at: now,
        flushed_at: now,
        target: DraftTarget {
            repository: "/home/u/work/project".to_owned(),
            item_kind,
            item_id: id("01M5CS4MJ0G7B0MPJD0KN0QMYA"),
            comment: None,
        },
        base: None,
        current,
    }
}

/// The front matter of `text` as YAML, and the body after it.
fn split(text: &str) -> (serde_yaml::Mapping, &str) {
    let rest = text.strip_prefix("---\n").expect("front matter opens");
    let end = rest.find("\n---\n").expect("front matter closes");
    let mapping = serde_yaml::from_str(&rest[..=end]).expect("front matter is YAML");
    (mapping, &rest[end + 5..])
}

fn key(mapping: &serde_yaml::Mapping, name: &str) -> serde_yaml::Value {
    mapping
        .get(name)
        .unwrap_or_else(|| panic!("{name} is in the front matter"))
        .clone()
}

#[test]
fn a_ticket_exports_its_named_fields_without_an_id_then_its_body() {
    let text = markdown(&draft(
        ItemKind::Ticket,
        DraftFields::Ticket {
            title: "A title: with \"quotes\"\n---".to_owned(),
            ticket_type: "bug".to_owned(),
            status: "open".to_owned(),
            project: Some("p".to_owned()),
            team: None,
            deps: vec!["01M5B0Z6PNP7X4WH4JV3N0VQJ6".to_owned()],
            parent: None,
            body: "Line one\n---\nLine two\n".to_owned(),
        },
    ));
    let (front, body) = split(&text);
    assert_eq!(body, "Line one\n---\nLine two\n");
    assert_eq!(
        key(&front, "title"),
        serde_yaml::Value::from("A title: with \"quotes\"\n---")
    );
    assert_eq!(key(&front, "type"), serde_yaml::Value::from("bug"));
    assert_eq!(key(&front, "status"), serde_yaml::Value::from("open"));
    assert_eq!(key(&front, "project"), serde_yaml::Value::from("p"));
    assert_eq!(key(&front, "team"), serde_yaml::Value::Null);
    assert_eq!(key(&front, "parent"), serde_yaml::Value::Null);
    assert_eq!(
        key(&front, "deps"),
        serde_yaml::Value::Sequence(vec!["01M5B0Z6PNP7X4WH4JV3N0VQJ6".into()])
    );
    assert_eq!(front.len(), 7, "no other key: {text}");
    for absent in ["id", "manyhands_managed", "manyhands_kind", "body"] {
        assert!(front.get(absent).is_none(), "{absent} is not exported");
    }
}

#[test]
fn a_document_exports_its_title_then_its_body() {
    let text = markdown(&draft(
        ItemKind::Document,
        DraftFields::Document {
            title: "Guide".to_owned(),
            body: "# Guide\n".to_owned(),
        },
    ));
    assert_eq!(text, "---\ntitle: Guide\n---\n# Guide\n");
}

#[test]
fn a_comment_exports_its_body_only() {
    let mut comment = draft(
        ItemKind::Ticket,
        DraftFields::Comment {
            body: "Just the reply.\n".to_owned(),
        },
    );
    comment.target.comment = Some(CommentTarget {
        comment_id: id("01M4CC0VMZ21Z79B09YQRA0P7J"),
        parent_id: None,
    });
    assert_eq!(markdown(&comment), "Just the reply.\n");
}
