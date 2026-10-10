//! `save_ticket_with`: how a save writes `deps`, `parent` and `slug`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use git2::Repository;
use manyhands::{
    canonical::{CanonicalItem, Ticket, parse_item},
    repository::{
        AuthoringKind, AuthoringTarget, ContextIntent, ExpectedPathObservation, FailurePoint,
        LocalCheckpoint, OperationId, RelationshipWrite, RepositoryError, RepositoryErrorKind,
        RepositoryService, SaveOutcome, SaveTicketRequest, TicketDraft, TicketWriteOptions,
    },
};
use serde_yaml::Value;

mod support;

use support::{
    EnabledRepository, TestRepository,
    items::{self, RELATED_A, RELATED_B, RELATED_C, RELATED_D, TICKET_A, TICKET_B, TICKET_C},
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
