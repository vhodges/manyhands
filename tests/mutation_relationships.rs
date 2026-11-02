//! `save_ticket_with`: how a save writes `deps`, `parent` and `slug`; and
//! `check_ticket_relationships`: what a proposed `deps` and `parent` would
//! be rejected for, before anything is written.

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::Repository;
use manyhands::{
    canonical::{CanonicalItem, Ticket, parse_item},
    repository::{
        AuthoringKind, AuthoringTarget, ContextIntent, ExpectedPathObservation, FailurePoint,
        IndexState, LocalCheckpoint, OperationId, ProposedRelationships, RelationshipCheckDto,
        RelationshipWrite, RepositoryError, RepositoryErrorKind, RepositoryService, SaveOutcome,
        SaveTicketRequest, TicketDraft, TicketWriteOptions,
    },
    results::ResultCode,
};
use serde_yaml::Value;

mod support;

use support::{
    EnabledRepository, TestRepository,
    items::{
        self, CLOSURE, COMMENT_A, DOCUMENT_A, RELATED_A, RELATED_B, RELATED_C, RELATED_D, TICKET_A,
        TICKET_ABSENT, TICKET_B, TICKET_C,
    },
};

const SLUG: &str = "mh-vh-k9x2b";
const OTHER_SLUG: &str = "mh-ab-12345";

/// The fields every ticket has, as the serializer lays them out.
fn required(id: &str, title: &str) -> String {
    format!(
        "id: {id}\ntitle: {title}\ntype: task\nstatus: open\n\
         manyhands_managed: true\nmanyhands_kind: ticket\n"
    )
}

/// A ticket file as the serializer lays one out: `extra` front matter
/// lines, each ending in a newline, then the fields every ticket has.
fn source(id: &str, title: &str, extra: &str) -> String {
    format!("---\n{extra}{}---\nBody.\n", required(id, title))
}

/// An enabled repository whose primary branch holds a ticket titled
/// `Title` for each ID, with that front matter.
fn repository(tickets: &[(&str, &str)]) -> (TestRepository, EnabledRepository) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    let paths: Vec<String> = tickets
        .iter()
        .map(|(id, extra)| {
            let path = items::ticket_path(id);
            items::write(&fixture.root, &path, &source(id, "Title", extra));
            path
        })
        .collect();
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    // This also commits the configuration enabling wrote, which a save
    // requires to be clean.
    items::commit(&fixture, &paths, 0);
    (fixture, enabled)
}

/// Where the ticket is: in its editing context once there is one, and on
/// the primary branch before that.
fn ticket_file(root: &Path, id: &str) -> PathBuf {
    let context = root.join(".manyhands/worktrees").join(id);
    let base = if context.exists() {
        context
    } else {
        root.to_owned()
    };
    base.join(items::ticket_path(id))
}

fn request_as(
    root: &Path,
    id: &str,
    intent: ContextIntent,
    title: &str,
    operation_id: OperationId,
) -> SaveTicketRequest {
    SaveTicketRequest {
        target: AuthoringTarget {
            root: root.to_owned(),
            kind: AuthoringKind::Ticket,
            item_id: items::item_id(id),
            intent,
            operation_id,
        },
        draft: TicketDraft {
            title: title.to_owned(),
            ticket_type: "task".to_owned(),
            status: "open".to_owned(),
            project: None,
            team: None,
            body: "Body.\n".to_owned(),
        },
        expected_path: match fs::read(ticket_file(root, id)) {
            Ok(bytes) => ExpectedPathObservation::from_bytes(&bytes),
            Err(_) => ExpectedPathObservation::Missing,
        },
    }
}

fn create(root: &Path, id: &str) -> SaveTicketRequest {
    request_as(
        root,
        id,
        ContextIntent::Create,
        "Title",
        support::new_operation_id(),
    )
}

/// An edit that leaves the ticket's own fields as `repository` wrote them
/// when `title` is `Title`.
fn edit(root: &Path, id: &str, title: &str) -> SaveTicketRequest {
    request_as(
        root,
        id,
        ContextIntent::Edit,
        title,
        support::new_operation_id(),
    )
}

fn with_slug(slug: &str) -> TicketWriteOptions {
    TicketWriteOptions {
        slug: Some(slug.to_owned()),
        ..Default::default()
    }
}

fn ids(ids: &[&str]) -> RelationshipWrite<Vec<manyhands::canonical::ItemId>> {
    RelationshipWrite::Set(ids.iter().map(|id| items::item_id(id)).collect())
}

/// The commit a save that wrote the file made.
fn checkpointed(result: Result<SaveOutcome, RepositoryError>) -> git2::Oid {
    match result {
        Ok(SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::Checkpointed { commit_oid },
            ..
        }) => commit_oid,
        Ok(_) => panic!("the save was expected to make a checkpoint"),
        Err(error) => panic!("the save was rejected: {:?}", error.kind),
    }
}

fn assert_no_change(result: Result<SaveOutcome, RepositoryError>) {
    match result {
        Ok(SaveOutcome::Saved {
            checkpoint: LocalCheckpoint::NoChange,
            ..
        }) => {}
        Ok(_) => panic!("the save was expected to change nothing"),
        Err(error) => panic!("the save was rejected: {:?}", error.kind),
    }
}

fn rejection(result: Result<SaveOutcome, RepositoryError>) -> RepositoryErrorKind {
    match result {
        Ok(_) => panic!("the save was expected to be rejected"),
        Err(error) => error.kind,
    }
}

fn text(root: &Path, id: &str) -> String {
    fs::read_to_string(ticket_file(root, id)).unwrap()
}

fn ticket(root: &Path, id: &str) -> Ticket {
    match parse_item(Path::new(&items::ticket_path(id)), &text(root, id)).unwrap() {
        CanonicalItem::Ticket(ticket) => ticket,
        _ => panic!("not a ticket"),
    }
}

/// The front matter keys the ticket does not define, in file order.
fn keys(ticket: &Ticket) -> Vec<&str> {
    ticket
        .unknown
        .keys()
        .map(|key| key.as_str().unwrap())
        .collect()
}

fn context_head(root: &Path, id: &str) -> git2::Oid {
    let repository = Repository::open(items::context_worktree(root, id)).unwrap();
    repository.head().unwrap().target().unwrap()
}

/// The ticket file in the commit `oid` of its editing context.
fn committed(root: &Path, id: &str, oid: git2::Oid) -> String {
    let repository = Repository::open(items::context_worktree(root, id)).unwrap();
    let bytes = support::commit_tree_path(&repository, oid, items::ticket_path(id)).unwrap();
    String::from_utf8(bytes).unwrap()
}

#[test]
fn a_create_writes_the_set_relationships_in_the_canonical_form() {
    let (fixture, enabled) = repository(&[]);

    let oid = checkpointed(enabled.service.save_ticket_with(
        create(&fixture.root, TICKET_A),
        TicketWriteOptions {
            deps: ids(&[RELATED_C, RELATED_A, RELATED_C, RELATED_B, RELATED_A]),
            parent: RelationshipWrite::Set(items::item_id(RELATED_D)),
            slug: Some(SLUG.to_owned()),
        },
    ));

    // `deps` is a block sequence, sorted, each ID once; `parent` is one
    // string; the short code is the one passed.
    let expected = source(
        TICKET_A,
        "Title",
        &format!(
            "slug: {SLUG}\nparent: {RELATED_D}\ndeps:\n- {RELATED_A}\n- {RELATED_B}\n- {RELATED_C}\n"
        ),
    );
    assert_eq!(text(&fixture.root, TICKET_A), expected);
    assert_eq!(committed(&fixture.root, TICKET_A, oid), expected);
}

#[test]
fn a_create_without_options_writes_no_relationship_and_no_short_code() {
    let (fixture, enabled) = repository(&[]);

    checkpointed(enabled.service.save_ticket(create(&fixture.root, TICKET_A)));
    checkpointed(enabled.service.save_ticket_with(
        create(&fixture.root, TICKET_B),
        TicketWriteOptions::default(),
    ));
    // A clear, and an empty list, of what is not there write nothing either.
    checkpointed(enabled.service.save_ticket_with(
        create(&fixture.root, TICKET_C),
        TicketWriteOptions {
            deps: ids(&[]),
            parent: RelationshipWrite::Clear,
            slug: None,
        },
    ));

    for id in [TICKET_A, TICKET_B, TICKET_C] {
        assert_eq!(text(&fixture.root, id), source(id, "Title", ""), "{id}");
    }
}

#[test]
fn an_empty_list_and_a_clear_both_remove_the_key() {
    let related = format!("parent: {RELATED_D}\ndeps:\n- {RELATED_A}\n- {RELATED_B}\n");
    let (fixture, enabled) = repository(&[(TICKET_A, &related), (TICKET_B, &related)]);

    checkpointed(enabled.service.save_ticket_with(
        edit(&fixture.root, TICKET_A, "Title"),
        TicketWriteOptions {
            deps: ids(&[]),
            ..Default::default()
        },
    ));
    checkpointed(enabled.service.save_ticket_with(
        edit(&fixture.root, TICKET_B, "Title"),
        TicketWriteOptions {
            deps: RelationshipWrite::Clear,
            parent: RelationshipWrite::Clear,
            slug: None,
        },
    ));

    assert_eq!(
        text(&fixture.root, TICKET_A),
        source(TICKET_A, "Title", &format!("parent: {RELATED_D}\n"))
    );
    assert_eq!(text(&fixture.root, TICKET_B), source(TICKET_B, "Title", ""));
}

#[test]
fn an_edit_sets_and_replaces_relationships_where_the_keys_are() {
    let (fixture, enabled) = repository(&[(
        TICKET_A,
        &format!("zeta: 1\ndeps: [{RELATED_A}]\nparent: {RELATED_A}\nalpha: kept\n"),
    )]);

    checkpointed(enabled.service.save_ticket_with(
        edit(&fixture.root, TICKET_A, "Title"),
        TicketWriteOptions {
            deps: ids(&[RELATED_C, RELATED_B]),
            parent: RelationshipWrite::Set(items::item_id(RELATED_D)),
            slug: None,
        },
    ));

    assert_eq!(
        text(&fixture.root, TICKET_A),
        source(
            TICKET_A,
            "Title",
            &format!(
                "zeta: 1\ndeps:\n- {RELATED_B}\n- {RELATED_C}\nparent: {RELATED_D}\nalpha: kept\n"
            )
        )
    );
}

#[test]
fn a_hand_written_dependency_list_is_rewritten_only_by_a_save_that_writes() {
    let hand_written = format!("deps: [{RELATED_C}, {RELATED_A}, {RELATED_B}]\n");
    let (fixture, enabled) = repository(&[(TICKET_A, &hand_written)]);
    let primary = fixture.repository.head().unwrap().target().unwrap();
    let before = source(TICKET_A, "Title", &hand_written);

    // A save that changes nothing: no write, no commit.
    assert_no_change(
        enabled
            .service
            .save_ticket_with(edit(&fixture.root, TICKET_A, "Title"), Default::default()),
    );
    assert_eq!(text(&fixture.root, TICKET_A), before);
    assert_eq!(context_head(&fixture.root, TICKET_A), primary);

    // A save that changes the title rewrites the list it did not touch.
    let oid = checkpointed(
        enabled
            .service
            .save_ticket_with(edit(&fixture.root, TICKET_A, "Renamed"), Default::default()),
    );
    let after = source(
        TICKET_A,
        "Renamed",
        &format!("deps:\n- {RELATED_A}\n- {RELATED_B}\n- {RELATED_C}\n"),
    );
    assert_eq!(text(&fixture.root, TICKET_A), after);
    assert_eq!(committed(&fixture.root, TICKET_A, oid), after);
}

#[test]
fn values_with_a_problem_are_preserved_by_a_save_that_does_not_set_them() {
    let (fixture, enabled) = repository(&[(
        TICKET_A,
        &format!("slug: Not-A-Slug\nparent: nope\ndeps: [{RELATED_B}, {RELATED_A}, {RELATED_B}]\n"),
    )]);
    let before = ticket(&fixture.root, TICKET_A).unknown;

    checkpointed(
        enabled
            .service
            .save_ticket_with(edit(&fixture.root, TICKET_A, "Renamed"), Default::default()),
    );

    let after = ticket(&fixture.root, TICKET_A);
    assert_eq!(after.title, "Renamed");
    assert_eq!(after.unknown, before);
    assert_eq!(after.unknown.get("slug"), Some(&"Not-A-Slug".into()));
    assert_eq!(after.unknown.get("parent"), Some(&"nope".into()));
    assert_eq!(
        after.unknown.get("deps"),
        Some(&Value::Sequence(vec![
            RELATED_B.into(),
            RELATED_A.into(),
            RELATED_B.into()
        ]))
    );
}

#[test]
fn other_unknown_keys_keep_their_values_and_order_and_new_keys_follow_them() {
    let others = "zeta: 1\nalpha:\n- x\n- y\nmiddle:\n  nested: true\n";
    let (fixture, enabled) = repository(&[(TICKET_A, others), (TICKET_B, others)]);
    let before = ticket(&fixture.root, TICKET_A).unknown;
    assert_eq!(
        keys(&ticket(&fixture.root, TICKET_A)),
        ["zeta", "alpha", "middle"]
    );

    // A save that sets nothing.
    checkpointed(
        enabled
            .service
            .save_ticket_with(edit(&fixture.root, TICKET_A, "Renamed"), Default::default()),
    );
    assert_eq!(
        text(&fixture.root, TICKET_A),
        source(TICKET_A, "Renamed", others)
    );
    assert_eq!(ticket(&fixture.root, TICKET_A).unknown, before);

    // A save that adds all three keys.
    checkpointed(enabled.service.save_ticket_with(
        edit(&fixture.root, TICKET_B, "Renamed"),
        TicketWriteOptions {
            deps: ids(&[RELATED_A]),
            parent: RelationshipWrite::Set(items::item_id(RELATED_B)),
            slug: Some(SLUG.to_owned()),
        },
    ));
    let after = ticket(&fixture.root, TICKET_B);
    assert_eq!(
        keys(&after),
        ["zeta", "alpha", "middle", "slug", "parent", "deps"]
    );
    for (key, value) in &before {
        assert_eq!(after.unknown.get(key), Some(value), "{key:?}");
    }
}

#[test]
fn an_interrupted_create_keeps_the_short_code_it_first_wrote() {
    let (fixture, enabled) = repository(&[]);
    let request = create(&fixture.root, TICKET_A);
    let failing = support::FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        rejection(failing.save_ticket_with(request.clone(), with_slug(SLUG))),
        RepositoryErrorKind::Sqlite
    );
    let written = source(TICKET_A, "Title", &format!("slug: {SLUG}\n"));
    assert_eq!(text(&fixture.root, TICKET_A), written);

    // The retry has the same operation ID and another short code: the
    // initials or the prefix changed between the attempts.
    let service = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let oid = checkpointed(service.save_ticket_with(request, with_slug(OTHER_SLUG)));

    assert_eq!(text(&fixture.root, TICKET_A), written);
    assert_eq!(committed(&fixture.root, TICKET_A, oid), written);
    assert!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty()
    );
}

/// How many commits the ticket's editing context has that `base` does not.
fn commits_since(root: &Path, id: &str, base: git2::Oid) -> usize {
    let repository = Repository::open(items::context_worktree(root, id)).unwrap();
    let mut walk = repository.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.hide(base).unwrap();
    walk.count()
}

#[test]
fn an_interrupted_create_with_relationships_completes_once_as_first_written() {
    let (fixture, enabled) = repository(&[]);
    let base = Repository::open(&fixture.root)
        .unwrap()
        .head()
        .unwrap()
        .target()
        .unwrap();
    let request = create(&fixture.root, TICKET_A);
    // The dependencies are given out of order, one of them twice.
    let options = TicketWriteOptions {
        deps: ids(&[RELATED_C, RELATED_A, RELATED_C, RELATED_B]),
        parent: RelationshipWrite::Set(items::item_id(RELATED_D)),
        slug: Some(SLUG.to_owned()),
    };
    let failing = support::FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        rejection(failing.save_ticket_with(request.clone(), options.clone())),
        RepositoryErrorKind::Sqlite
    );
    let written = source(
        TICKET_A,
        "Title",
        &format!(
            "slug: {SLUG}\nparent: {RELATED_D}\ndeps:\n- {RELATED_A}\n- {RELATED_B}\n- {RELATED_C}\n"
        ),
    );
    assert_eq!(text(&fixture.root, TICKET_A), written);
    assert_eq!(commits_since(&fixture.root, TICKET_A, base), 0);

    // The retry has the same operation ID and the same options.
    let service = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let oid = checkpointed(service.save_ticket_with(request, options));

    assert_eq!(text(&fixture.root, TICKET_A), written);
    assert_eq!(committed(&fixture.root, TICKET_A, oid), written);
    assert_eq!(context_head(&fixture.root, TICKET_A), oid);
    assert_eq!(commits_since(&fixture.root, TICKET_A, base), 1);
    let saved = ticket(&fixture.root, TICKET_A);
    assert_eq!(
        saved.unknown.get("deps"),
        Some(&Value::Sequence(vec![
            RELATED_A.into(),
            RELATED_B.into(),
            RELATED_C.into()
        ]))
    );
    assert_eq!(saved.unknown.get("parent"), Some(&RELATED_D.into()));
    assert_eq!(saved.unknown.get("slug"), Some(&SLUG.into()));
    assert!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn an_edit_adds_a_short_code_only_to_a_ticket_without_one() {
    let (fixture, enabled) = repository(&[
        (TICKET_A, "other: kept\n"),
        (TICKET_B, "slug: null\nother: kept\n"),
        (TICKET_C, "slug: MH-ZZ-99999\nother: kept\n"),
        (RELATED_A, "slug: Not A Slug\nother: kept\n"),
    ]);
    let assign = |id| {
        enabled
            .service
            .save_ticket_with(edit(&fixture.root, id, "Title"), with_slug(SLUG))
    };

    // No key: the short code is a new key, after the others.
    checkpointed(assign(TICKET_A));
    assert_eq!(
        text(&fixture.root, TICKET_A),
        source(TICKET_A, "Title", &format!("other: kept\nslug: {SLUG}\n"))
    );
    // A null value: the short code takes its place.
    checkpointed(assign(TICKET_B));
    assert_eq!(
        text(&fixture.root, TICKET_B),
        source(TICKET_B, "Title", &format!("slug: {SLUG}\nother: kept\n"))
    );
    // A valid value and an invalid one: the option is ignored without a
    // rejection, and with nothing else to write the file is left alone.
    for (id, kept) in [(TICKET_C, "MH-ZZ-99999"), (RELATED_A, "Not A Slug")] {
        assert_no_change(assign(id));
        assert_eq!(
            text(&fixture.root, id),
            source(id, "Title", &format!("slug: {kept}\nother: kept\n")),
            "{id}"
        );
    }
}

#[test]
fn an_interrupted_edit_keeps_the_short_code_it_first_wrote() {
    let (fixture, enabled) = repository(&[(TICKET_A, "other: kept\n")]);
    let request = edit(&fixture.root, TICKET_A, "Title");
    let failing = support::FailOnce::at(FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)
        .open_service(enabled.data_directory.path());

    assert_eq!(
        rejection(failing.save_ticket_with(request.clone(), with_slug(SLUG))),
        RepositoryErrorKind::Sqlite
    );
    let written = source(TICKET_A, "Title", &format!("other: kept\nslug: {SLUG}\n"));
    assert_eq!(text(&fixture.root, TICKET_A), written);

    let service = RepositoryService::open_at(enabled.data_directory.path()).unwrap();
    let oid = checkpointed(service.save_ticket_with(request, with_slug(OTHER_SLUG)));

    assert_eq!(text(&fixture.root, TICKET_A), written);
    assert_eq!(committed(&fixture.root, TICKET_A, oid), written);
    assert!(
        service
            .recovery_inspection(&fixture.root)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_short_code_is_unchanged_by_a_later_save_that_changes_the_title() {
    let (fixture, enabled) = repository(&[(TICKET_A, ""), (TICKET_B, "slug: MH-VH-K9X2B\n")]);
    checkpointed(
        enabled
            .service
            .save_ticket_with(edit(&fixture.root, TICKET_A, "Title"), with_slug(SLUG)),
    );

    // With no option, and with one: neither rewrites the short code. One
    // written by hand in upper case stays in upper case.
    checkpointed(
        enabled
            .service
            .save_ticket(edit(&fixture.root, TICKET_A, "Renamed")),
    );
    checkpointed(enabled.service.save_ticket_with(
        edit(&fixture.root, TICKET_B, "Renamed"),
        with_slug(OTHER_SLUG),
    ));

    assert_eq!(
        text(&fixture.root, TICKET_A),
        source(TICKET_A, "Renamed", &format!("slug: {SLUG}\n"))
    );
    assert_eq!(
        text(&fixture.root, TICKET_B),
        source(TICKET_B, "Renamed", "slug: MH-VH-K9X2B\n")
    );
}

/// `repository`, with the index refreshed over what it committed.
fn indexed(tickets: &[(&str, &str)]) -> (TestRepository, EnabledRepository) {
    let (fixture, enabled) = repository(tickets);
    items::refresh_completely(&enabled.service, &fixture.root);
    (fixture, enabled)
}

fn check(
    fixture: &TestRepository,
    enabled: &EnabledRepository,
    id: &str,
    deps: &[&str],
    parent: Option<&str>,
) -> RelationshipCheckDto {
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    enabled
        .service
        .check_ticket_relationships(
            &repo,
            &items::item_id(id),
            &ProposedRelationships {
                deps: deps.iter().map(|id| items::item_id(id)).collect(),
                parent: parent.map(items::item_id),
            },
        )
        .unwrap()
}

/// The code a proposal would be rejected with and the IDs it names, or
/// `None` for one that is accepted.
fn verdict(check: &RelationshipCheckDto) -> Option<(ResultCode, Vec<&str>)> {
    check.rejection.as_ref().map(|rejection| {
        (
            rejection.code,
            rejection.ids.iter().map(String::as_str).collect(),
        )
    })
}

fn cycle(ids: &[&'static str]) -> Option<(ResultCode, Vec<&'static str>)> {
    Some((ResultCode::RelationshipCycle, ids.to_vec()))
}

fn invalid(ids: &[&'static str]) -> Option<(ResultCode, Vec<&'static str>)> {
    Some((ResultCode::InvalidRelationship, ids.to_vec()))
}

fn dep(id: &str) -> String {
    format!("deps:\n- {id}\n")
}

fn parent(id: &str) -> String {
    format!("parent: {id}\n")
}

#[test]
fn a_dependency_that_closes_a_cycle_is_a_relationship_cycle_naming_its_members() {
    // B depends on A, and C on B.
    let (fixture, enabled) = indexed(&[
        (TICKET_A, ""),
        (TICKET_B, &dep(TICKET_A)),
        (TICKET_C, &dep(TICKET_B)),
    ]);

    let two = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);
    let three = check(&fixture, &enabled, TICKET_A, &[TICKET_C], None);

    assert_eq!(verdict(&two), cycle(&[TICKET_A, TICKET_B]));
    assert_eq!(verdict(&three), cycle(&[TICKET_A, TICKET_B, TICKET_C]));
    // The answer still says what the proposal comes to.
    assert_eq!(three.deps, [TICKET_C]);
    assert_eq!(three.parent, None);
    assert!(three.unresolved.is_empty());
}

#[test]
fn a_parent_that_closes_a_cycle_is_a_relationship_cycle_naming_its_members() {
    let (fixture, enabled) = indexed(&[
        (TICKET_A, ""),
        (TICKET_B, &parent(TICKET_A)),
        (TICKET_C, &parent(TICKET_B)),
    ]);

    let ring = check(&fixture, &enabled, TICKET_A, &[], Some(TICKET_C));

    assert_eq!(verdict(&ring), cycle(&[TICKET_A, TICKET_B, TICKET_C]));
    assert_eq!(ring.parent.as_deref(), Some(TICKET_C));
}

#[test]
fn a_ticket_naming_itself_in_either_field_is_a_cycle_of_one() {
    let (fixture, enabled) = indexed(&[(TICKET_A, ""), (TICKET_B, "")]);

    let as_dependency = check(&fixture, &enabled, TICKET_A, &[TICKET_B, TICKET_A], None);
    let as_parent = check(&fixture, &enabled, TICKET_A, &[TICKET_B], Some(TICKET_A));

    assert_eq!(verdict(&as_dependency), cycle(&[TICKET_A]));
    assert_eq!(verdict(&as_parent), cycle(&[TICKET_A]));
}

#[test]
fn a_cycle_through_a_closed_ticket_is_rejected() {
    let closed = format!("{}{CLOSURE}", dep(TICKET_A));
    let (fixture, enabled) = indexed(&[(TICKET_A, ""), (TICKET_B, &closed)]);

    let through_closed = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);

    assert_eq!(verdict(&through_closed), cycle(&[TICKET_A, TICKET_B]));
}

#[test]
fn the_proposed_edges_replace_the_stored_ones() {
    // A and B already name each other in both fields, as a merge can leave
    // them, and C depends on nothing.
    let both = |id: &str| format!("{}{}", parent(id), dep(id));
    let (fixture, enabled) = indexed(&[
        (TICKET_A, &both(TICKET_B)),
        (TICKET_B, &both(TICKET_A)),
        (TICKET_C, ""),
    ]);

    // Keeping what A has is still the cycle it is on.
    let kept = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);
    let kept_parent = check(&fixture, &enabled, TICKET_A, &[], Some(TICKET_B));
    // Removing the edge that closes it while adding another is accepted.
    let replaced = check(&fixture, &enabled, TICKET_A, &[TICKET_C], Some(TICKET_C));

    assert_eq!(verdict(&kept), cycle(&[TICKET_A, TICKET_B]));
    assert_eq!(verdict(&kept_parent), cycle(&[TICKET_A, TICKET_B]));
    assert_eq!(verdict(&replaced), None);
    assert_eq!(replaced.deps, [TICKET_C]);
    assert_eq!(replaced.parent.as_deref(), Some(TICKET_C));
    assert!(replaced.unresolved.is_empty());
}

/// Two tickets, a document and a comment on it that no ticket names, all
/// indexed.
fn repository_with_a_document_and_a_comment() -> (TestRepository, EnabledRepository) {
    let (fixture, enabled) = repository(&[(TICKET_A, ""), (TICKET_B, "")]);
    items::write(
        &fixture.root,
        "docs/guide.md",
        &items::document_source(DOCUMENT_A, "Guide", ""),
    );
    items::write_comment(
        &fixture.root,
        DOCUMENT_A,
        COMMENT_A,
        None,
        "2026-10-01T00:00:00Z",
        "",
    );
    items::commit(
        &fixture,
        &["docs/guide.md", &items::comment_path(DOCUMENT_A, COMMENT_A)],
        10,
    );
    items::refresh_completely(&enabled.service, &fixture.root);
    (fixture, enabled)
}

#[test]
fn a_target_the_index_holds_as_a_document_or_a_comment_is_an_invalid_relationship() {
    let (fixture, enabled) = repository_with_a_document_and_a_comment();

    let document = check(&fixture, &enabled, TICKET_A, &[TICKET_B, DOCUMENT_A], None);
    let document_parent = check(&fixture, &enabled, TICKET_A, &[], Some(DOCUMENT_A));
    let comment = check(&fixture, &enabled, TICKET_A, &[COMMENT_A], None);
    let comment_parent = check(&fixture, &enabled, TICKET_A, &[], Some(COMMENT_A));
    let several = check(
        &fixture,
        &enabled,
        TICKET_A,
        &[COMMENT_A, TICKET_ABSENT],
        Some(DOCUMENT_A),
    );

    assert_eq!(verdict(&document), invalid(&[DOCUMENT_A]));
    assert_eq!(verdict(&document_parent), invalid(&[DOCUMENT_A]));
    assert_eq!(verdict(&comment), invalid(&[COMMENT_A]));
    assert_eq!(verdict(&comment_parent), invalid(&[COMMENT_A]));
    // Every offending ID, sorted, each once. What is not a ticket is not
    // unresolved either.
    assert_eq!(verdict(&several), invalid(&[DOCUMENT_A, COMMENT_A]));
    assert_eq!(several.deps, [TICKET_ABSENT, COMMENT_A]);
    assert_eq!(several.unresolved, [TICKET_ABSENT]);
}

#[test]
fn an_unknown_target_is_accepted_and_listed_as_unresolved() {
    let (fixture, enabled) = indexed(&[(TICKET_A, ""), (TICKET_B, "")]);

    let unknown = check(
        &fixture,
        &enabled,
        TICKET_A,
        &[TICKET_ABSENT, TICKET_B],
        Some(RELATED_A),
    );
    let both_fields = check(
        &fixture,
        &enabled,
        TICKET_A,
        &[TICKET_ABSENT],
        Some(TICKET_ABSENT),
    );

    assert_eq!(verdict(&unknown), None);
    assert_eq!(unknown.deps, [TICKET_B, TICKET_ABSENT]);
    assert_eq!(unknown.parent.as_deref(), Some(RELATED_A));
    assert_eq!(unknown.unresolved, [TICKET_ABSENT, RELATED_A]);
    assert_eq!(verdict(&both_fields), None);
    assert_eq!(both_fields.unresolved, [TICKET_ABSENT]);
}

#[test]
fn repeated_entries_are_removed_and_the_rest_sorted() {
    let (fixture, enabled) = indexed(&[(TICKET_A, ""), (TICKET_B, ""), (TICKET_C, "")]);

    let repeated = check(
        &fixture,
        &enabled,
        TICKET_A,
        &[TICKET_C, TICKET_B, TICKET_C, TICKET_ABSENT, TICKET_ABSENT],
        None,
    );
    let nothing = check(&fixture, &enabled, TICKET_A, &[], None);

    assert_eq!(verdict(&repeated), None);
    assert_eq!(repeated.deps, [TICKET_B, TICKET_C, TICKET_ABSENT]);
    assert_eq!(repeated.unresolved, [TICKET_ABSENT]);
    assert_eq!(verdict(&nothing), None);
    assert!(nothing.deps.is_empty());
    assert_eq!(nothing.parent, None);
    assert!(nothing.unresolved.is_empty());
}

#[test]
fn a_ticket_the_index_does_not_hold_yet_can_be_checked() {
    // B already waits for the ticket that is about to be created.
    let (fixture, enabled) = indexed(&[(TICKET_A, ""), (TICKET_B, &dep(TICKET_C))]);

    let accepted = check(&fixture, &enabled, TICKET_C, &[TICKET_A], Some(TICKET_B));
    let rejected = check(&fixture, &enabled, TICKET_C, &[TICKET_B], Some(TICKET_A));

    assert_eq!(verdict(&accepted), None);
    assert_eq!(accepted.deps, [TICKET_A]);
    assert_eq!(accepted.parent.as_deref(), Some(TICKET_B));
    assert!(accepted.unresolved.is_empty());
    assert_eq!(verdict(&rejected), cycle(&[TICKET_B, TICKET_C]));
}

#[test]
fn one_rejection_is_reported_invalid_then_self_reference_then_dependency_then_parent_cycle() {
    // B depends on A, and A is the parent of C.
    let (fixture, enabled) = repository(&[
        (TICKET_A, ""),
        (TICKET_B, &dep(TICKET_A)),
        (TICKET_C, &parent(TICKET_A)),
    ]);
    items::write(
        &fixture.root,
        "docs/guide.md",
        &items::document_source(DOCUMENT_A, "Guide", ""),
    );
    items::commit(&fixture, &["docs/guide.md"], 10);
    items::refresh_completely(&enabled.service, &fixture.root);

    let all_three = check(
        &fixture,
        &enabled,
        TICKET_A,
        &[TICKET_B, DOCUMENT_A],
        Some(TICKET_C),
    );
    let invalid_and_self = check(&fixture, &enabled, TICKET_A, &[TICKET_A], Some(DOCUMENT_A));
    let self_and_cycles = check(
        &fixture,
        &enabled,
        TICKET_A,
        &[TICKET_A, TICKET_B],
        Some(TICKET_C),
    );
    let both_cycles = check(&fixture, &enabled, TICKET_A, &[TICKET_B], Some(TICKET_C));
    let parent_only = check(&fixture, &enabled, TICKET_A, &[], Some(TICKET_C));

    assert_eq!(verdict(&all_three), invalid(&[DOCUMENT_A]));
    assert_eq!(verdict(&invalid_and_self), invalid(&[DOCUMENT_A]));
    assert_eq!(verdict(&self_and_cycles), cycle(&[TICKET_A]));
    assert_eq!(verdict(&both_cycles), cycle(&[TICKET_A, TICKET_B]));
    assert_eq!(verdict(&parent_only), cycle(&[TICKET_A, TICKET_C]));
}

#[test]
fn the_check_is_index_unavailable_when_the_index_cannot_be_read() {
    let (fixture, enabled) = indexed(&[(TICKET_A, ""), (TICKET_B, "")]);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let (_data, service) = items::degraded_service(enabled);

    let error = service
        .check_ticket_relationships(
            &repo,
            &items::item_id(TICKET_A),
            &ProposedRelationships {
                deps: vec![items::item_id(TICKET_B)],
                parent: None,
            },
        )
        .unwrap_err();

    assert_eq!(error.code(), ResultCode::IndexUnavailable);
    assert_eq!(error.scope.item_id.as_deref(), Some(TICKET_A));
    assert_eq!(error.recovery.len(), 1);
    assert_eq!(error.recovery[0].action.as_str(), "index.rebuild");
}

#[test]
fn a_stale_index_passes_a_cycle_that_the_reads_report_after_a_refresh() {
    let (fixture, enabled) = indexed(&[(TICKET_A, "")]);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    // B arrives depending on A, and the index is not refreshed: it does
    // not hold B.
    let path = items::ticket_path(TICKET_B);
    items::write(
        &fixture.root,
        &path,
        &source(TICKET_B, "Title", &dep(TICKET_A)),
    );
    items::commit(&fixture, &[&path], 10);

    // The check does not scan, so a commit nothing has told the index of
    // is not seen: it reports the index as the relationship reads do.
    let unnoticed = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);
    assert_eq!(
        unnoticed.index,
        enabled.service.ticket_cycles(&repo).unwrap().index
    );
    // The index is marked for a refresh, as it is once a change is noticed.
    items::index(enabled.data_directory.path())
        .execute("UPDATE repositories SET refresh_required = 1", [])
        .unwrap();

    // The check answers from what the index has.
    let stale = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);
    assert_eq!(verdict(&stale), None);
    assert_eq!(stale.unresolved, [TICKET_B]);
    // And says that what it answered from is behind. The refresh that was
    // completed saw every ticket there was then.
    assert_eq!(stale.index.state, IndexState::Stale);
    assert!(stale.complete);

    // So the cycle is written, and reported when it is read.
    let path = items::ticket_path(TICKET_A);
    items::write(
        &fixture.root,
        &path,
        &source(TICKET_A, "Title", &dep(TICKET_B)),
    );
    items::commit(&fixture, &[&path], 20);
    items::refresh_completely(&enabled.service, &fixture.root);

    let cycles = enabled.service.ticket_cycles(&repo).unwrap();
    assert_eq!(cycles.items.len(), 1);
    assert_eq!(cycles.items[0].kind.as_str(), "deps");
    assert_eq!(cycles.items[0].ids, [TICKET_A, TICKET_B]);
    let again = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);
    assert_eq!(verdict(&again), cycle(&[TICKET_A, TICKET_B]));
    assert_eq!(again.index.state, IndexState::Current);
    assert!(again.complete);
}

#[test]
fn an_accepted_check_on_a_current_index_says_it_is_current_and_complete() {
    let (fixture, enabled) = indexed(&[(TICKET_A, ""), (TICKET_B, "")]);
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();

    let accepted = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);

    assert_eq!(verdict(&accepted), None);
    assert_eq!(accepted.index.state, IndexState::Current);
    assert!(accepted.index.refreshed_at.is_some());
    assert!(accepted.complete);
    // As the sibling ticket reads report it, from the same index.
    let cycles = enabled.service.ticket_cycles(&repo).unwrap();
    assert_eq!(accepted.index, cycles.index);
    assert_eq!(accepted.complete, cycles.complete);
}

#[test]
fn a_ticket_naming_itself_is_a_cycle_of_one_whatever_other_cycle_it_closes() {
    // B depends on A, and A is the parent of C.
    let (fixture, enabled) = indexed(&[
        (TICKET_A, ""),
        (TICKET_B, &dep(TICKET_A)),
        (TICKET_C, &parent(TICKET_A)),
    ]);

    // Itself beside a dependency that closes a larger cycle.
    let in_deps = check(&fixture, &enabled, TICKET_A, &[TICKET_A, TICKET_B], None);
    // Itself as the parent, while its dependencies close a cycle.
    let as_parent = check(&fixture, &enabled, TICKET_A, &[TICKET_B], Some(TICKET_A));
    // Itself as a dependency, while its parent closes a cycle.
    let over_parent_cycle = check(&fixture, &enabled, TICKET_A, &[TICKET_A], Some(TICKET_C));

    assert_eq!(verdict(&in_deps), cycle(&[TICKET_A]));
    assert_eq!(verdict(&as_parent), cycle(&[TICKET_A]));
    assert_eq!(verdict(&over_parent_cycle), cycle(&[TICKET_A]));
    // Without itself, the larger cycle is what is named.
    let larger = check(&fixture, &enabled, TICKET_A, &[TICKET_B], None);
    assert_eq!(verdict(&larger), cycle(&[TICKET_A, TICKET_B]));
}

#[test]
fn an_id_the_index_holds_as_a_document_or_a_comment_naming_itself_is_a_cycle_of_one() {
    let (fixture, enabled) = repository_with_a_document_and_a_comment();

    let document = check(&fixture, &enabled, DOCUMENT_A, &[DOCUMENT_A], None);
    let document_parent = check(&fixture, &enabled, DOCUMENT_A, &[], Some(DOCUMENT_A));
    let comment = check(&fixture, &enabled, COMMENT_A, &[COMMENT_A], None);
    // An invalid target still comes first.
    let with_invalid = check(
        &fixture,
        &enabled,
        DOCUMENT_A,
        &[DOCUMENT_A, COMMENT_A],
        None,
    );

    assert_eq!(verdict(&document), cycle(&[DOCUMENT_A]));
    assert_eq!(verdict(&document_parent), cycle(&[DOCUMENT_A]));
    assert_eq!(verdict(&comment), cycle(&[COMMENT_A]));
    assert_eq!(verdict(&with_invalid), invalid(&[COMMENT_A]));
}
