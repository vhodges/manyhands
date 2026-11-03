//! Claims: one writer per draft across processes and within one, and what
//! retirement and discard remove.

use crate::support::*;
use manyhands::drafts::{DraftErrorKind, DraftId, Entry, Held, Retired};
use std::io::{BufRead, BufReader, Read, Write};

const DATA: &str = "MANYHANDS_DRAFTS_CLAIM_DATA";
const ID: &str = "MANYHANDS_DRAFTS_CLAIM_ID";
const TEST: &str = "claims::a_claim_in_another_process_makes_the_draft_unavailable_here";
const CLAIMED: &str = "manyhands-drafts-child-claimed";

#[test]
fn a_claim_in_another_process_makes_the_draft_unavailable_here() {
    if let Some(data) = std::env::var_os(DATA) {
        // The child: claim, say so, and hold the claim until stdin closes.
        let id: DraftId = std::env::var(ID).unwrap().parse().unwrap();
        let store = store(std::path::Path::new(&data));
        let _claim = claim(&store, id);
        println!("{CLAIMED}");
        std::io::stdout().flush().unwrap();
        let mut rest = String::new();
        std::io::stdin().read_to_string(&mut rest).unwrap();
        return;
    }
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let id = create(&store).draft().id;
    let mut child = child(
        TEST,
        &[(DATA, data.path().to_str().unwrap()), (ID, &id.to_string())],
    )
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut seen = String::new();
    loop {
        let mut line = String::new();
        if stdout.read_line(&mut line).unwrap() == 0 {
            panic!("the child ended before claiming:\n{seen}");
        }
        seen.push_str(&line);
        // libtest names the test on the same line, before the marker.
        if line.trim_end().ends_with(CLAIMED) {
            break;
        }
    }

    assert_eq!(claim_error(&store, id), DraftErrorKind::OpenElsewhere);
    assert!(matches!(held(&store, id), Held::Elsewhere));
    assert_eq!(
        store.discard_unreadable(id).map_err(|e| e.kind),
        Err(DraftErrorKind::OpenElsewhere)
    );
    assert!(drafts(data.path()).join(format!("{id}.json")).exists());

    drop(child.stdin.take());
    stdout.read_to_string(&mut seen).unwrap();
    let status = child.wait().unwrap();
    ran_one_test(&seen);
    assert!(status.success(), "the child failed:\n{seen}");
    assert!(seen.contains("1 passed"), "{seen}");

    assert!(matches!(held(&store, id), Held::No));
    let claim = claim(&store, id);
    assert_eq!(claim.draft().revision, 1);
}

#[test]
fn a_claim_held_here_is_listed_as_here_and_cannot_be_taken_twice() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let clone = store.clone();
    let claim = create(&store);
    let id = claim.draft().id;
    assert!(matches!(held(&store, id), Held::Here));
    assert!(matches!(held(&clone, id), Held::Here));
    assert_eq!(claim_error(&store, id), DraftErrorKind::HeldHere);
    assert_eq!(claim_error(&clone, id), DraftErrorKind::HeldHere);
    assert_eq!(
        store.discard_unreadable(id).map_err(|e| e.kind),
        Err(DraftErrorKind::HeldHere)
    );
    drop(claim);
    assert!(matches!(held(&clone, id), Held::No));
    drop(self::claim(&clone, id));
}

#[test]
fn retirement_deletes_only_the_saved_revision() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let mut claim = create(&store);
    let id = claim.draft().id;
    assert_eq!(write(&mut claim, 3), Ok(3));
    let claim = match claim.retire(2) {
        Ok(Retired::Kept(claim)) => claim,
        Ok(Retired::Deleted) => panic!("a newer revision must be kept"),
        Err(error) => panic!("retire failed: {error:?}"),
    };
    assert!(matches!(held(&store, id), Held::Here), "the claim is kept");
    assert_eq!(listed_revision(&store, id), 3);
    assert!(matches!(claim.retire(3), Ok(Retired::Deleted)));
    assert!(list(&store).is_empty());
    assert!(names(data.path()).is_empty(), "{:?}", names(data.path()));
    assert_eq!(claim_error(&store, id), DraftErrorKind::NotFound);
}

#[test]
fn discard_removes_only_its_own_draft() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let first = create(&store);
    let second = create(&store);
    let (first_id, second_id) = (first.draft().id, second.draft().id);
    drop(second);
    assert!(first.discard().is_ok());
    let listed: Vec<DraftId> = list(&store).into_iter().map(|l| l.id).collect();
    assert_eq!(listed, vec![second_id]);
    assert_eq!(
        names(data.path()),
        vec![format!("{second_id}.json"), format!("{second_id}.lock")]
    );
    assert_eq!(claim_error(&store, first_id), DraftErrorKind::NotFound);
}

#[test]
fn a_lock_file_with_no_draft_survives_listing() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let stray = DraftId::generate();
    std::fs::write(drafts(data.path()).join(format!("{stray}.lock")), b"").unwrap();
    assert!(list(&store).is_empty());
    assert!(drafts(data.path()).join(format!("{stray}.lock")).exists());
}

#[test]
fn names_not_the_stores_are_ignored_and_left_alone() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let id = create(&store).draft().id;
    let lower = DraftId::generate().to_string().to_lowercase();
    let others = [
        "notes.txt".to_owned(),
        format!("{lower}.json"),
        format!("{id}.json.bak"),
        "short.json".to_owned(),
        ".hidden".to_owned(),
    ];
    for name in &others {
        std::fs::write(drafts(data.path()).join(name), b"not a draft").unwrap();
    }
    std::fs::create_dir(drafts(data.path()).join("folder")).unwrap();
    let listed: Vec<DraftId> = list(&store).into_iter().map(|l| l.id).collect();
    assert_eq!(listed, vec![id]);
    drop(claim(&store, id));
    drop(create(&store));
    for name in &others {
        assert_eq!(
            std::fs::read(drafts(data.path()).join(name)).unwrap(),
            b"not a draft"
        );
    }
    assert!(drafts(data.path()).join("folder").is_dir());
}

#[test]
fn listing_puts_the_latest_flush_first_and_unreadable_entries_last() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let mut older = create(&store);
    let newer = create(&store);
    // Rewriting the first draft makes it the latest.
    assert_eq!(write(&mut older, 2), Ok(2));
    let (older_id, newer_id) = (older.draft().id, newer.draft().id);
    let damaged = [DraftId::generate(), DraftId::generate()];
    for id in damaged {
        std::fs::write(drafts(data.path()).join(format!("{id}.json")), b"{").unwrap();
    }
    let mut unreadable = damaged.to_vec();
    unreadable.sort();
    let listed: Vec<(DraftId, bool)> = list(&store)
        .into_iter()
        .map(|l| (l.id, matches!(l.entry, Entry::Draft(_))))
        .collect();
    assert_eq!(
        listed,
        vec![
            (older_id, true),
            (newer_id, true),
            (unreadable[0], false),
            (unreadable[1], false),
        ]
    );
}

#[test]
fn a_store_and_its_claims_cross_threads() {
    fn send_sync<T: Send + Sync + Clone>() {}
    fn send<T: Send>() {}
    send_sync::<manyhands::drafts::DraftStore>();
    send::<manyhands::drafts::DraftClaim>();
}

/// A temporary file a dead writer left for `id`, holding text.
fn leave_temporary_file(data: &std::path::Path, id: DraftId) -> std::path::PathBuf {
    let path = drafts(data).join(format!(".{id}.0123456789abcdef.tmp"));
    std::fs::write(&path, b"{\"half\": \"a draft's text").unwrap();
    path
}

#[test]
fn discard_and_retirement_remove_the_drafts_temporary_files() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());

    let discarded = create(&store);
    let retired = create(&store);
    let unreadable = create(&store);
    let ids = [
        discarded.draft().id,
        retired.draft().id,
        unreadable.draft().id,
    ];
    drop(unreadable);
    for id in ids {
        leave_temporary_file(data.path(), id);
    }

    assert!(discarded.discard().is_ok());
    assert!(matches!(retired.retire(1), Ok(Retired::Deleted)));
    assert!(store.discard_unreadable(ids[2]).is_ok());

    assert!(list(&store).is_empty());
    assert!(names(data.path()).is_empty(), "{:?}", names(data.path()));
}

#[test]
fn deleting_one_draft_keeps_another_drafts_temporary_files() {
    let data = tempfile::tempdir().unwrap();
    let store = store(data.path());
    let first = create(&store);
    let second = create(&store);
    let kept = leave_temporary_file(data.path(), second.draft().id);
    assert!(first.discard().is_ok());
    assert!(kept.exists());
}
