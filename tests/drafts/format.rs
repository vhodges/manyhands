//! The version 1 file format against committed golden files.

use manyhands::drafts::{UnreadableReason, codec};
use std::path::PathBuf;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/drafts")
        .join(name);
    std::fs::read(path).expect("fixture")
}

fn reason(bytes: &[u8]) -> UnreadableReason {
    match codec::decode(bytes) {
        Ok(_) => panic!("expected the bytes to be unreadable"),
        Err(reason) => reason,
    }
}

#[test]
fn golden_files_decode_and_encode_to_the_same_bytes() {
    for name in ["v1-ticket.json", "v1-document.json", "v1-comment.json"] {
        let bytes = fixture(name);
        let draft = match codec::decode(&bytes) {
            Ok(draft) => draft,
            Err(_) => panic!("{name} must decode"),
        };
        assert!(codec::encode(&draft) == bytes, "{name} must round-trip");
    }
}

#[test]
fn a_newer_version_is_reported_with_its_number() {
    assert_eq!(
        reason(&fixture("v2-ticket.json")),
        UnreadableReason::NewerFormat { version: 2 }
    );
}

#[test]
fn damaged_bytes_are_damaged() {
    let ticket = fixture("v1-ticket.json");
    let text = String::from_utf8(ticket.clone()).unwrap();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "missing format",
            text.replace("\"format\": \"manyhands-draft\",", "")
                .into_bytes(),
        ),
        (
            "other format",
            text.replace("manyhands-draft", "other").into_bytes(),
        ),
        ("invalid json", b"not json".to_vec()),
        ("truncated", ticket[..40].to_vec()),
        ("invalid utf-8", vec![0xff, 0xfe, 0xfd]),
        (
            "item id not a ulid",
            text.replacen("\"item_id\": \"", "\"item_id\": \"x", 1)
                .into_bytes(),
        ),
    ];
    for (label, bytes) in cases {
        assert_eq!(reason(&bytes), UnreadableReason::Damaged, "{label}");
    }
}
