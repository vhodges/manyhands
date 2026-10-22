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

/// Times the parser accepts but that leave the range a draft file can hold
/// once taken to UTC: one past year 9999 (which would panic the
/// conversion) and one before year 0 (which could never be written again).
const OUT_OF_RANGE_TIMES: [&str; 2] = ["9999-12-31T23:59:59-01:00", "0000-01-01T00:30:00+01:00"];

#[test]
fn a_time_out_of_range_in_utc_is_damaged() {
    let text = String::from_utf8(fixture("v1-document.json")).unwrap();
    for time in OUT_OF_RANGE_TIMES {
        for field in ["created_at", "flushed_at"] {
            let bytes = text
                .replacen(
                    &format!("\"{field}\": \"2026-10-14T17:46:40Z\""),
                    &format!("\"{field}\": \"{time}\""),
                    1,
                )
                .into_bytes();
            assert!(bytes != text.as_bytes(), "{field} must be replaced");
            assert_eq!(reason(&bytes), UnreadableReason::Damaged, "{field} {time}");
        }
    }
}

/// The same times in a file in the folder list as damaged, and the file
/// stays as it was.
#[test]
fn a_file_with_a_time_out_of_range_is_listed_damaged_and_kept() {
    use crate::support::{drafts, list, store};
    use manyhands::drafts::Entry;

    let text = String::from_utf8(fixture("v1-document.json")).unwrap();
    for time in OUT_OF_RANGE_TIMES {
        let data = tempfile::tempdir().unwrap();
        let store = store(data.path());
        let bytes = text
            .replacen("\"2026-10-14T17:46:40Z\"", &format!("\"{time}\""), 1)
            .into_bytes();
        let id = match codec::decode(&fixture("v1-document.json")) {
            Ok(draft) => draft.id,
            Err(_) => panic!("the fixture decodes"),
        };
        let path = drafts(data.path()).join(format!("{id}.json"));
        std::fs::write(&path, &bytes).unwrap();
        let listed = list(&store);
        assert_eq!(listed.len(), 1, "{time}");
        assert!(
            matches!(
                listed[0].entry,
                Entry::Unreadable(UnreadableReason::Damaged)
            ),
            "{time}"
        );
        assert!(
            std::fs::read(&path).unwrap() == bytes,
            "{time}: kept byte for byte"
        );
    }
}
