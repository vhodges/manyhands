use super::schema::*;
use crate::canonical::ItemId;
use time::OffsetDateTime;

fn id(text: &str) -> ItemId {
    text.parse().expect("item id")
}

fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

fn target(kind: ItemKind, comment: Option<CommentTarget>) -> DraftTarget {
    DraftTarget {
        repository: "/home/u/work/project".to_owned(),
        item_kind: kind,
        item_id: id("01M5CS4MJ0G7B0MPJD0KN0QMYA"),
        comment,
    }
}

fn ticket_fields(title: &str, deps: Vec<String>) -> DraftFields {
    DraftFields::Ticket {
        title: title.to_owned(),
        ticket_type: "task".to_owned(),
        status: "open".to_owned(),
        project: None,
        team: Some("core".to_owned()),
        deps,
        parent: None,
        body: "Line one\nLine \"two\"\n".to_owned(),
    }
}

fn draft_id() -> DraftId {
    "01J9ZQ6V0Y8K3N4M5P6Q7R8S9T".parse().expect("draft id")
}

fn ticket_draft() -> Draft {
    Draft {
        id: draft_id(),
        revision: 7,
        created_at: at(1_792_000_000),
        flushed_at: at(1_792_000_329),
        target: target(ItemKind::Ticket, None),
        base: Some(DraftBase {
            observation: Some("obs-1".to_owned()),
            path: ".manyhands/tickets/01M5CS4MJ0G7B0MPJD0KN0QMYA/ticket.md".to_owned(),
            context: BaseContext {
                kind: ContextKind::Primary,
                branch: Some("main".to_owned()),
                worktree: "/home/u/work/project".to_owned(),
                head_oid: Some("abc123".to_owned()),
            },
            source: Some("---\ntitle: Old\n---\n".to_owned()),
            fields: ticket_fields("Old", vec![]),
        }),
        current: ticket_fields("New", vec!["01M5B0Z6PNP7X4WH4JV3N0VQJ6".to_owned()]),
    }
}

fn document_draft() -> Draft {
    Draft {
        id: draft_id(),
        revision: 1,
        created_at: at(1_792_000_000),
        flushed_at: at(1_792_000_000),
        target: target(ItemKind::Document, None),
        base: Some(DraftBase {
            observation: None,
            path: ".manyhands/docs/a.md".to_owned(),
            context: BaseContext {
                kind: ContextKind::Unverified,
                branch: None,
                worktree: "/home/u/work/project".to_owned(),
                head_oid: None,
            },
            source: None,
            fields: DraftFields::Document {
                title: "T".to_owned(),
                body: "b".to_owned(),
            },
        }),
        current: DraftFields::Document {
            title: "T2".to_owned(),
            body: "b2 \u{1F600}".to_owned(),
        },
    }
}

fn comment_draft() -> Draft {
    Draft {
        id: draft_id(),
        revision: 3,
        created_at: at(1_792_000_000),
        flushed_at: at(1_792_000_100),
        target: target(
            ItemKind::Ticket,
            Some(CommentTarget {
                comment_id: id("01M5CS4MJ0G7B0MPJD0KN0QMYB"),
                parent_id: Some(id("01M5CS4MJ0G7B0MPJD0KN0QMYC")),
            }),
        ),
        base: None,
        current: DraftFields::Comment {
            body: "hello".to_owned(),
        },
    }
}

fn round_trip(draft: &Draft) {
    let bytes = encode(draft);
    assert!(bytes.ends_with(b"\n"));
    let decoded = match decode(&bytes) {
        Ok(decoded) => decoded,
        Err(_) => panic!("encoded draft must decode"),
    };
    assert!(&decoded == draft);
    assert!(encode(&decoded) == bytes);
}

#[test]
fn ticket_document_and_comment_drafts_round_trip() {
    round_trip(&ticket_draft());
    round_trip(&document_draft());
    round_trip(&comment_draft());
}

#[test]
fn keys_are_written_sorted_at_every_level() {
    let bytes = encode(&ticket_draft());
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    fn sorted(value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                let keys: Vec<&String> = map.keys().collect();
                let mut expected = keys.clone();
                expected.sort();
                assert_eq!(keys, expected);
                map.values().for_each(sorted);
            }
            serde_json::Value::Array(items) => items.iter().for_each(sorted),
            _ => {}
        }
    }
    sorted(&value);
}

#[test]
fn draft_ids_use_the_ulid_text() {
    assert_eq!(draft_id().to_string(), "01J9ZQ6V0Y8K3N4M5P6Q7R8S9T");
    assert!("nope".parse::<DraftId>().is_err());
}

#[test]
fn a_greater_version_is_newer_format() {
    let mut value: serde_json::Value = serde_json::from_slice(&encode(&ticket_draft())).unwrap();
    value["version"] = serde_json::json!(9);
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        decode(&bytes).err(),
        Some(UnreadableReason::NewerFormat { version: 9 })
    );
}

#[test]
fn an_unknown_kind_or_bad_id_is_damaged() {
    let text = String::from_utf8(encode(&comment_draft())).unwrap();
    for bad in [
        text.replace("\"kind\": \"comment\"", "\"kind\": \"memo\""),
        text.replace("01J9ZQ6V0Y8K3N4M5P6Q7R8S9T", "short"),
        text.replace("\"item_kind\": \"ticket\"", "\"item_kind\": \"epic\""),
        text.replace("2026-", "20x6-"),
    ] {
        assert_eq!(
            decode(bad.as_bytes()).err(),
            Some(UnreadableReason::Damaged)
        );
    }
}
