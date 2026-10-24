//! A session's flush schedule and recovery state, under an injected clock:
//! one edit, a burst, continuous typing, coalescing during a write, a
//! failed write and a rebase.
//!
//! Sessions hold draft text and derive no `Debug`, so their writes are
//! read through `match`.

use std::time::{Duration, Instant};

use manyhands::drafts::{
    BaseContext, ContextKind, DraftBase, DraftError, DraftErrorKind, DraftFields, DraftStore,
    DraftTarget, ItemKind, Retired, Written,
};
use tempfile::TempDir;
use time::OffsetDateTime;

use super::{DraftSession, IDLE, MAX_UNFLUSHED, PendingWrite, RecoveryState};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FG0";

fn fields(body: &str) -> DraftFields {
    DraftFields::Document {
        title: "Draft title".to_owned(),
        body: body.to_owned(),
    }
}

fn base(title: &str) -> DraftBase {
    DraftBase {
        observation: Some("observation".to_owned()),
        path: "docs/a.md".to_owned(),
        context: BaseContext {
            kind: ContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: "/a/repository".to_owned(),
            head_oid: None,
        },
        source: None,
        fields: DraftFields::Document {
            title: title.to_owned(),
            body: "Item body".to_owned(),
        },
    }
}

/// A session over a real claim in its own store, begun at `now`. The
/// folder lives as long as the returned guard.
fn session(base: Option<DraftBase>, now: Instant) -> (TempDir, DraftSession) {
    let data = tempfile::tempdir().unwrap();
    let store = DraftStore::open_in(data.path()).unwrap();
    let target = DraftTarget {
        repository: "/a/repository".to_owned(),
        item_kind: ItemKind::Document,
        item_id: TICKET.parse().unwrap(),
        comment: None,
    };
    let claim = match store.create(target, base, fields("first")) {
        Ok(claim) => claim,
        Err(error) => panic!("the draft must be created: {error}"),
    };
    (data, DraftSession::new(claim, now))
}

fn landed(write: &PendingWrite) -> Result<Written, DraftError> {
    Ok(Written {
        revision: write.revision,
        flushed_at: OffsetDateTime::UNIX_EPOCH,
    })
}

fn failed(kind: DraftErrorKind) -> Result<Written, DraftError> {
    Err(DraftError { kind })
}

fn body(write: &PendingWrite) -> &str {
    match &write.current {
        DraftFields::Document { body, .. } => body,
        _ => panic!("a document's write"),
    }
}

#[test]
fn a_new_session_is_saved_and_nothing_is_due() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    assert_eq!(session.revision(), 1);
    assert_eq!(session.flushed(), 1);
    assert!(matches!(session.recovery_state(), RecoveryState::Saved(_)));
    assert_eq!(session.flush_due(), None);
    assert!(session.next_write().is_none());
}

#[test]
fn one_edit_is_due_a_second_later() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    assert_eq!(session.edit(fields("a"), t0), 2);
    assert_eq!(session.flush_due(), Some(t0 + IDLE));
    assert_eq!(IDLE, Duration::from_secs(1));
}

#[test]
fn a_burst_inside_one_second_is_due_a_second_after_its_last_edit() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    for ms in [0, 200, 400] {
        session.edit(fields("a"), t0 + Duration::from_millis(ms));
    }
    assert_eq!(session.revision(), 4);
    assert_eq!(
        session.flush_due(),
        Some(t0 + Duration::from_millis(400) + IDLE)
    );
}

/// Typing every 300 ms for twelve seconds never goes idle: a write is due
/// five seconds after the first change it misses, at 5 s and then 10.1 s
/// (five after the edit at 5.1 s, the first after the write at 5 s). Each
/// write is taken when it is due, before the edit of that step.
#[test]
fn continuous_edits_for_twelve_seconds_are_written_every_five_seconds() {
    assert_eq!(MAX_UNFLUSHED, Duration::from_secs(5));
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    let mut writes = Vec::new();
    for step in 0..=40u64 {
        let now = t0 + Duration::from_millis(300 * step);
        while let Some(due) = session.flush_due().filter(|due| *due <= now) {
            writes.push(due - t0);
            let write = session.next_write().unwrap();
            session.write_landed(landed(&write));
        }
        session.edit(fields("typing"), now);
    }
    assert_eq!(
        writes,
        [Duration::from_secs(5), Duration::from_millis(10_100)]
    );
    // The last edit, at 12 s, is due once typing stops.
    assert_eq!(session.flush_due(), Some(t0 + Duration::from_secs(13)));
}

#[test]
fn after_a_write_lands_nothing_is_due_until_the_next_edit() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    session.edit(fields("a"), t0);
    let write = session.next_write().unwrap();
    session.write_landed(landed(&write));
    assert_eq!(session.flushed(), 2);
    assert_eq!(session.flush_due(), None);
    assert!(session.next_write().is_none());

    let later = t0 + Duration::from_secs(3);
    session.edit(fields("b"), later);
    assert_eq!(session.flush_due(), Some(later + IDLE));
}

/// Three edits while a write is in flight produce one more write, of the
/// newest revision; the revisions between are never written.
#[test]
fn edits_during_a_write_coalesce_into_one_write_of_the_newest_revision() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    session.edit(fields("a"), t0);
    let first = session.next_write().unwrap();
    assert_eq!(first.revision, 2);
    assert!(session.next_write().is_none());
    assert_eq!(session.flush_due(), None);

    for (n, text) in ["b", "c", "d"].into_iter().enumerate() {
        session.edit(
            fields(text),
            t0 + Duration::from_millis(100 * (n as u64 + 1)),
        );
    }
    // Still in flight: nothing more is due or taken.
    assert_eq!(session.flush_due(), None);
    assert!(session.next_write().is_none());
    assert_eq!(session.recovery_state(), RecoveryState::Pending);

    session.write_landed(landed(&first));
    assert_eq!(session.flushed(), 2);
    assert_eq!(
        session.flush_due(),
        Some(t0 + Duration::from_millis(300) + IDLE)
    );
    let second = session.next_write().unwrap();
    assert_eq!(second.revision, 5);
    assert_eq!(body(&second), "d");
    session.write_landed(landed(&second));
    assert!(session.next_write().is_none());
    assert!(matches!(session.recovery_state(), RecoveryState::Saved(_)));
}

/// A failed write keeps what was flushed and is not retried on a timer;
/// the next edit schedules it again, and a write that lands clears it.
#[test]
fn a_failed_write_keeps_flushed_and_the_next_edit_schedules_again() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    session.edit(fields("a"), t0);
    let write = session.next_write().unwrap();
    session.write_landed(failed(DraftErrorKind::Access));
    drop(write);
    assert_eq!(
        session.recovery_state(),
        RecoveryState::Failed(DraftErrorKind::Access)
    );
    assert_eq!(session.flushed(), 1);
    assert_eq!(session.revision(), 2);
    assert_eq!(session.flush_due(), None);

    let later = t0 + Duration::from_secs(2);
    session.edit(fields("b"), later);
    assert_eq!(session.flush_due(), Some(later + IDLE));
    // Failed until a write lands.
    assert_eq!(
        session.recovery_state(),
        RecoveryState::Failed(DraftErrorKind::Access)
    );
    let retry = session.next_write().unwrap();
    assert_eq!(retry.revision, 3);
    session.write_landed(landed(&retry));
    assert!(matches!(session.recovery_state(), RecoveryState::Saved(_)));
}

/// A failed session with no new edit can still be written now: Retry.
#[test]
fn a_failed_write_can_be_taken_again_without_an_edit() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    session.edit(fields("a"), t0);
    let _write = session.next_write().unwrap();
    // As a folder sync that failed after the rename answers: the store
    // may hold revision 2, but the session keeps what was proven.
    session.write_landed(failed(DraftErrorKind::NoSpace));
    assert_eq!(session.flushed(), 1);
    let retry = session.next_write().unwrap();
    assert_eq!(retry.revision, 2);
    session.write_landed(landed(&retry));
    assert_eq!(session.flushed(), 2);
    assert!(matches!(session.recovery_state(), RecoveryState::Saved(_)));
}

/// `Saved` only when the newest revision is written and nothing failed;
/// `Pending` while a change waits or is being written; `Failed` while a
/// failure stands.
#[test]
fn recovery_state_is_saved_only_when_flushed_equals_revision_without_failure() {
    let t0 = Instant::now();
    let (_data, mut session) = session(None, t0);
    let created = match session.recovery_state() {
        RecoveryState::Saved(at) => at,
        _ => panic!("a new session is saved"),
    };

    session.edit(fields("a"), t0);
    assert_eq!(session.recovery_state(), RecoveryState::Pending);
    let write = session.next_write().unwrap();
    assert_eq!(session.recovery_state(), RecoveryState::Pending);
    session.write_landed(failed(DraftErrorKind::Io));
    assert_eq!(
        session.recovery_state(),
        RecoveryState::Failed(DraftErrorKind::Io)
    );
    let write_again = session.next_write().unwrap();
    assert_eq!(write_again.revision, write.revision);
    let at = OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(60);
    session.write_landed(Ok(Written {
        revision: write_again.revision,
        flushed_at: at,
    }));
    assert_eq!(session.recovery_state(), RecoveryState::Saved(at));
    assert_ne!(at, created);
}

/// A base change raises the revision and is due at once, not on the
/// timer, even when it arrives during a write.
#[test]
fn a_rebase_raises_the_revision_and_is_due_at_once() {
    let t0 = Instant::now();
    let (_data, mut session) = session(Some(base("Item title")), t0);
    let now = t0 + Duration::from_secs(1);
    assert_eq!(session.rebase(base("Saved title"), now), 2);
    assert_eq!(session.flush_due(), Some(now));
    assert_eq!(session.recovery_state(), RecoveryState::Pending);
    let write = session.next_write().unwrap();
    assert!(matches!(
        &write.base,
        Some(DraftBase { fields: DraftFields::Document { title, .. }, .. }) if title == "Saved title"
    ));

    let during = now + Duration::from_millis(100);
    assert_eq!(session.rebase(base("Again"), during), 3);
    assert_eq!(session.flush_due(), None);
    session.write_landed(landed(&write));
    assert_eq!(session.flush_due(), Some(during));
}

/// The session names its item by the title read when the draft began,
/// never the draft's own title, and by its ID without a base.
#[test]
fn the_item_is_named_by_its_base_title_or_its_id() {
    let t0 = Instant::now();
    let (_data, mut titled) = session(Some(base("Item title")), t0);
    titled.edit(
        DraftFields::Document {
            title: "Draft's own title".to_owned(),
            body: "x".to_owned(),
        },
        t0,
    );
    titled.rebase(base("Later title"), t0);
    assert_eq!(titled.item_title(), "Item title");

    let (_data, untitled) = session(None, t0);
    assert_eq!(untitled.item_title(), TICKET);
}

/// While its revision retires, a session takes no write, and an edit made
/// meanwhile is kept in memory, never dropped.
#[test]
fn a_retiring_session_takes_no_write_and_keeps_its_edits() {
    let t0 = Instant::now();
    let (_data, mut session) = session(Some(base("Item title")), t0);
    let saved = session.revision();
    let slot = session.begin_retire();
    assert!(session.busy());
    session.edit(fields("typed while retiring"), t0);
    assert_eq!(session.flush_due(), None);
    assert!(session.next_write().is_none());
    assert!(matches!(
        session.current(),
        DraftFields::Document { body, .. } if body == "typed while retiring"
    ));
    // The retirement takes the claim; a newer revision in memory means
    // the session is to be stored again.
    let claim = slot.lock().unwrap().take().unwrap();
    assert!(matches!(claim.retire(saved), Ok(Retired::Deleted)));
    assert!(session.revision() > saved);
}

/// A session whose draft retired under an edit is stored again: it takes
/// the new draft's claim and ID, and the edit made after that store
/// began is written next, with the new base.
#[test]
fn a_session_stored_again_takes_the_new_claim_and_writes_what_came_after() {
    let t0 = Instant::now();
    let (data, mut session) = session(Some(base("Item title")), t0);
    let old = session.id();
    let slot = session.begin_retire();
    session.edit(fields("typed while retiring"), t0);
    drop(slot.lock().unwrap().take());

    let store = DraftStore::open_in(data.path()).unwrap();
    let from = session.revision();
    let claim = match store.create(
        session.target().clone(),
        Some(base("Saved title")),
        session.current().clone(),
    ) {
        Ok(claim) => claim,
        Err(error) => panic!("the draft must be created: {error}"),
    };
    let new = claim.draft().id;
    // Nothing came after the store began: nothing to write.
    let mut quiet = DraftSession::new(
        match store.create(session.target().clone(), None, fields("x")) {
            Ok(claim) => claim,
            Err(error) => panic!("{error}"),
        },
        t0,
    );
    let quiet_from = quiet.revision();
    let quiet_slot = quiet.begin_retire();
    let quiet_claim = quiet_slot.lock().unwrap().take().unwrap();
    quiet.recreated(quiet_claim, quiet_from, t0);
    assert!(!quiet.busy());
    assert!(quiet.next_write().is_none());

    session.edit(fields("typed while storing"), t0);
    session.recreated(claim, from, t0);
    assert_eq!(session.id(), new);
    assert_ne!(session.id(), old);
    assert!(!session.busy());
    assert_eq!(session.recovery_state(), RecoveryState::Pending);
    let write = session.next_write().unwrap();
    assert_eq!(body(&write), "typed while storing");
    assert!(matches!(
        &write.base,
        Some(DraftBase { fields: DraftFields::Document { title, .. }, .. }) if title == "Saved title"
    ));
    session.write_landed(write.write());
    assert!(matches!(session.recovery_state(), RecoveryState::Saved(_)));
}

/// A kept retirement hands the claim back: writes resume.
#[test]
fn a_kept_retirement_hands_the_claim_back() {
    let t0 = Instant::now();
    let (_data, mut session) = session(Some(base("Item title")), t0);
    let slot = session.begin_retire();
    let claim = slot.lock().unwrap().take().unwrap();
    session.edit(fields("newer"), t0);
    session.retire_kept(claim);
    assert!(!session.busy());
    let write = session.next_write().unwrap();
    session.write_landed(write.write());
    assert!(matches!(session.recovery_state(), RecoveryState::Saved(_)));
}
