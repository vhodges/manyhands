use super::protect::Folder;
use super::schema::{DraftId, UnreadableReason};
use super::store::*;

fn id(text: &str) -> DraftId {
    text.parse().expect("draft id")
}

#[test]
fn the_store_and_its_claims_can_cross_threads() {
    fn send_sync_clone<T: Send + Sync + Clone>() {}
    fn send<T: Send>() {}
    send_sync_clone::<DraftStore>();
    send::<DraftClaim>();
}

#[test]
fn temporary_names_belong_to_their_draft_only() {
    let one = id("01J9ZQ6V0Y8K3N4M5P6Q7R8S9T");
    let other = id("01M5CS4MJ0G7B0MPJD0KN0QMYA");
    let name = temporary_name(one);
    assert!(name.starts_with(&format!(".{one}.")) && name.ends_with(".tmp"));
    assert_eq!(name.len(), 1 + 26 + 1 + 16 + 4);
    assert!(is_temporary_of(&name, one));
    assert!(!is_temporary_of(&name, other));
    assert_ne!(temporary_name(one), temporary_name(one));
    for stranger in [
        format!(".{one}.tmp"),
        format!(".{one}.0123456789abcdeg.tmp"),
        format!(".{one}.0123456789abcdef0.tmp"),
        format!("{one}.0123456789abcdef.tmp"),
        format!(".{one}.0123456789abcdef.json"),
    ] {
        assert!(!is_temporary_of(&stranger, one), "{stranger}");
    }
}

#[test]
fn only_canonical_ulid_json_names_are_draft_files() {
    let one = id("01J9ZQ6V0Y8K3N4M5P6Q7R8S9T");
    assert_eq!(draft_file_id("01J9ZQ6V0Y8K3N4M5P6Q7R8S9T.json"), Some(one));
    for name in [
        "01j9zq6v0y8k3n4m5p6q7r8s9t.json",
        "01J9ZQ6V0Y8K3N4M5P6Q7R8S9T.lock",
        "01J9ZQ6V0Y8K3N4M5P6Q7R8S9T.json.tmp",
        ".01J9ZQ6V0Y8K3N4M5P6Q7R8S9T.0123456789abcdef.tmp",
        "notes.json",
        ".json",
    ] {
        assert_eq!(draft_file_id(name), None, "{name}");
    }
}

#[test]
fn a_name_removed_after_listing_is_skipped_not_unreadable() {
    let data = tempfile::tempdir().unwrap();
    let folder = Folder::open(data.path()).unwrap();
    assert!(read_entry(&folder, DraftId::generate()).is_none());
}

#[test]
fn a_file_whose_id_is_not_its_name_is_damaged() {
    let data = tempfile::tempdir().unwrap();
    let folder = Folder::open(data.path()).unwrap();
    let fixture = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/drafts/v1-document.json"),
    )
    .unwrap();
    let named = id("01J9ZQ6V0Y8K3N4M5P6Q7R8S9T");
    let other = id("01M5CS4MJ0G7B0MPJD0KN0QMYA");
    let drafts = data.path().join("drafts");
    std::fs::write(drafts.join(format!("{named}.json")), &fixture).unwrap();
    std::fs::write(drafts.join(format!("{other}.json")), &fixture).unwrap();
    assert!(matches!(read_entry(&folder, named), Some(Entry::Draft(_))));
    assert!(matches!(
        read_entry(&folder, other),
        Some(Entry::Unreadable(UnreadableReason::Damaged))
    ));
}

#[test]
fn listings_sort_by_latest_flush_then_id_with_unreadable_last() {
    let fixture = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/drafts/v1-document.json"),
    )
    .unwrap();
    let base = super::schema::decode(&fixture).expect("fixture");
    let draft = |text: &str, seconds: i64| {
        let mut draft = base.clone();
        draft.id = id(text);
        draft.flushed_at = time::OffsetDateTime::from_unix_timestamp(seconds).unwrap();
        Listing {
            id: draft.id,
            entry: Entry::Draft(draft),
            held: Held::No,
        }
    };
    let unreadable = |text: &str| Listing {
        id: id(text),
        entry: Entry::Unreadable(UnreadableReason::Damaged),
        held: Held::No,
    };
    let mut listings = vec![
        unreadable("01M5CS4MJ0G7B0MPJD0KN0QMYB"),
        draft("01M5CS4MJ0G7B0MPJD0KN0QMY1", 10),
        unreadable("01M5CS4MJ0G7B0MPJD0KN0QMYA"),
        draft("01M5CS4MJ0G7B0MPJD0KN0QMY3", 20),
        draft("01M5CS4MJ0G7B0MPJD0KN0QMY2", 20),
    ];
    sort(&mut listings);
    let order: Vec<String> = listings.iter().map(|l| l.id.to_string()).collect();
    assert_eq!(
        order,
        [
            "01M5CS4MJ0G7B0MPJD0KN0QMY2",
            "01M5CS4MJ0G7B0MPJD0KN0QMY3",
            "01M5CS4MJ0G7B0MPJD0KN0QMY1",
            "01M5CS4MJ0G7B0MPJD0KN0QMYA",
            "01M5CS4MJ0G7B0MPJD0KN0QMYB",
        ]
    );
}
