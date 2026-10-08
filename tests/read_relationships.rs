//! The ticket relationship queries against real repositories: readiness,
//! dependency trees, children, cycles, the plan, the critical path and the
//! short code lookup, across the primary context and item worktrees.

// A read error carries its whole scope by value, as the contract has it.
#![allow(clippy::result_large_err)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use manyhands::{
    repository::{
        AuthoringKind, AuthoringTarget, ClosureFilter, ContextIntent, ContextProvisionOutcome,
        CycleKind, DependencyDirection, DependencyState, DependencyTreeDto, DependencyTreeNodeDto,
        ExpectedPathObservation, IndexState, ItemContextKind, ItemDto, ItemListDto, PlanDto,
        ReadError, ReadinessFilter, ReadinessReasonCode, ReadinessState, RepositoryService,
        ResolvedRepository, SaveOutcome, SaveTicketRequest, TicketDraft, TicketFilter,
        UnplannableReasonCode,
    },
    results::{ProblemCode, ResultCode},
};
use serde_json::{Value, json};
use support::items::{
    CLOSURE, DOCUMENT_A, RELATED_A, RELATED_B, RELATED_C, RELATED_D, RELATED_E, RELATED_F,
    RELATED_G, SHARED_SLUG, TICKET_A, TICKET_ABSENT, TICKET_B, TICKET_C, commit, context_worktree,
    degraded_service, document_source, index, item_id, never_refreshed_repository,
    refresh_completely, relationship_repository, ticket_path, ticket_source, ticket_source_with,
    write,
};

mod support;

const TICKET_D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC3";
const TICKET_E: &str = "01ARZ3NDEKTSV4RRFFQ69G5FC4";

/// No read test may initialize the Git transport; every test ends with this.
fn assert_git_transport_uninitialized() {
    assert!(!manyhands::runtime::git_transport_initialized());
}

fn enabled() -> (support::TestRepository, support::EnabledRepository) {
    let fixture = support::born_repository();
    let enabled = support::enabled_repository(&fixture);
    (fixture, enabled)
}

/// A `deps` line naming these tickets.
fn deps(ids: &[&str]) -> String {
    format!("deps: [{}]\n", ids.join(", "))
}

/// Writes an open ticket in the working tree at `root`.
fn write_ticket(root: &Path, id: &str, extra: &str) -> PathBuf {
    write(root, &ticket_path(id), &ticket_source(id, id, extra))
}

fn filter(readiness: ReadinessFilter) -> TicketFilter {
    TicketFilter {
        readiness: Some(readiness),
        ..Default::default()
    }
}

/// The IDs of a list of tickets, in the list's order.
fn ids(list: &ItemListDto) -> Vec<&str> {
    list.items
        .iter()
        .map(|item| {
            item.id
                .as_deref()
                .expect("a relationship read lists tickets")
        })
        .collect()
}

/// The same, in ID order, where the test did not fix the change times the
/// list is ordered by.
fn sorted_ids(list: &ItemListDto) -> Vec<&str> {
    let mut ids = ids(list);
    ids.sort_unstable();
    ids
}

fn ready(service: &RepositoryService, repo: &ResolvedRepository) -> ItemListDto {
    service
        .ticket_readiness(repo, &filter(ReadinessFilter::Ready))
        .unwrap()
}

fn blocked(service: &RepositoryService, repo: &ResolvedRepository) -> ItemListDto {
    service
        .ticket_readiness(repo, &filter(ReadinessFilter::Blocked))
        .unwrap()
}

fn all_tickets(service: &RepositoryService, repo: &ResolvedRepository) -> ItemListDto {
    service
        .list_tickets(repo, &TicketFilter::default())
        .unwrap()
}

fn ticket<'a>(list: &'a ItemListDto, id: &str) -> &'a ItemDto {
    list.items
        .iter()
        .find(|item| item.id.as_deref() == Some(id))
        .unwrap_or_else(|| panic!("{id} is not listed"))
}

fn state(item: &ItemDto) -> ReadinessState {
    item.readiness.as_ref().unwrap().state
}

/// A ticket's reasons as `(code, ids, complete)`.
fn reasons(item: &ItemDto) -> Vec<(ReadinessReasonCode, Vec<&str>, bool)> {
    item.readiness
        .as_ref()
        .unwrap()
        .reasons
        .iter()
        .map(|reason| {
            (
                reason.code,
                reason.ids.iter().map(String::as_str).collect(),
                reason.complete,
            )
        })
        .collect()
}

fn codes(item: &ItemDto) -> Vec<(ProblemCode, Option<&str>)> {
    item.problems
        .iter()
        .map(|problem| (problem.code, problem.target_id.as_deref()))
        .collect()
}

/// A tree's lines as `id`, indented by depth, with `*` for a repeated
/// line, `+` for a truncated one and `?` for an unresolved dependency.
fn lines(nodes: &[DependencyTreeNodeDto]) -> Vec<String> {
    nodes
        .iter()
        .map(|node| {
            format!(
                "{}{}{}{}{}",
                " ".repeat(node.depth as usize - 1),
                node.id,
                if node.repeated { "*" } else { "" },
                if node.truncated { "+" } else { "" },
                if node.state == DependencyState::Unresolved {
                    "?"
                } else {
                    ""
                },
            )
        })
        .collect()
}

fn tree(
    service: &RepositoryService,
    repo: &ResolvedRepository,
    id: &str,
    direction: DependencyDirection,
    depth: Option<u32>,
) -> DependencyTreeDto {
    service
        .ticket_dependencies(repo, &item_id(id), direction, depth)
        .unwrap()
}

/// A plan's batches as `(number, ids)`.
fn batches(plan: &PlanDto) -> Vec<(u64, Vec<&str>)> {
    plan.batches
        .iter()
        .map(|batch| {
            (
                batch.batch,
                batch
                    .items
                    .iter()
                    .map(|item| item.id.as_deref().unwrap())
                    .collect(),
            )
        })
        .collect()
}

/// Why a ticket cannot be planned, as `(code, ids)`.
type Unplannable<'a> = (UnplannableReasonCode, Vec<&'a str>);

/// A plan's unplannable tickets, each as its ID and its reasons.
fn unplannable(plan: &PlanDto) -> Vec<(&str, Vec<Unplannable<'_>>)> {
    plan.unplannable
        .iter()
        .map(|entry| {
            (
                entry.ticket.id.as_deref().unwrap(),
                entry
                    .reasons
                    .iter()
                    .map(|reason| (reason.code, reason.ids.iter().map(String::as_str).collect()))
                    .collect(),
            )
        })
        .collect()
}

fn plan(service: &RepositoryService, repo: &ResolvedRepository) -> PlanDto {
    service.ticket_plan(repo, &TicketFilter::default()).unwrap()
}

fn recovery(error: &ReadError) -> Value {
    serde_json::to_value(&error.to_envelope::<Value>("ticket deps").recovery).unwrap()
}

/// Makes the configuration look unchanged to Git, which enabling leaves
/// looking changed and which a context is not prepared over.
fn clean_configuration_index(root: &Path) {
    let repository = git2::Repository::open(root).unwrap();
    let mut git_index = repository.index().unwrap();
    git_index.read(true).unwrap();
    git_index
        .add_path(Path::new(".manyhands/config.toml"))
        .unwrap();
    git_index.write().unwrap();
}

/// Creates the item worktree of a ticket that is committed on the primary
/// branch, and returns the ticket's file there.
fn edit_context(service: &RepositoryService, root: &Path, id: &str) -> PathBuf {
    clean_configuration_index(root);
    let outcome = service
        .prepare_context(AuthoringTarget {
            root: root.to_owned(),
            kind: AuthoringKind::Ticket,
            item_id: item_id(id),
            intent: ContextIntent::Edit,
            operation_id: support::new_operation_id(),
        })
        .unwrap();
    assert!(matches!(outcome, ContextProvisionOutcome::Created(_)));
    let file = context_worktree(root, id).join(ticket_path(id));
    assert!(file.is_file());
    file
}

/// Saves the ticket through the ordinary save, creating its item worktree
/// when `intent` is to create it, and returns the ticket's file there.
fn save_ticket(
    service: &RepositoryService,
    root: &Path,
    id: &str,
    intent: ContextIntent,
) -> PathBuf {
    clean_configuration_index(root);
    let file = root
        .join(".manyhands/worktrees")
        .join(id)
        .join(ticket_path(id));
    let outcome = service
        .save_ticket(SaveTicketRequest {
            target: AuthoringTarget {
                root: root.to_owned(),
                kind: AuthoringKind::Ticket,
                item_id: item_id(id),
                intent,
                operation_id: support::new_operation_id(),
            },
            draft: TicketDraft {
                title: id.to_owned(),
                ticket_type: "task".to_owned(),
                status: "open".to_owned(),
                project: None,
                team: None,
                body: format!("Body of {id}.\n"),
            },
            expected_path: match fs::read(&file) {
                Ok(bytes) => ExpectedPathObservation::from_bytes(&bytes),
                Err(_) => ExpectedPathObservation::Missing,
            },
        })
        .unwrap();
    assert!(matches!(outcome, SaveOutcome::Saved { .. }));
    file
}

#[test]
fn ready_and_blocked_are_decided_across_primary_and_two_item_worktrees() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    // On the primary branch every ticket is open, and A and B depend on
    // nothing.
    write_ticket(root, TICKET_A, "");
    write_ticket(root, TICKET_B, "");
    write_ticket(root, TICKET_C, &deps(&[TICKET_A]));
    write_ticket(root, TICKET_D, &deps(&[TICKET_B]));
    let paths = [TICKET_A, TICKET_B, TICKET_C, TICKET_D].map(ticket_path);
    commit(
        &fixture,
        &paths.iter().map(String::as_str).collect::<Vec<_>>(),
        1_000,
    );
    // A is closed in its own worktree and nowhere else; B, in its own,
    // comes to depend on A.
    let a = edit_context(&enabled.service, root, TICKET_A);
    let b = edit_context(&enabled.service, root, TICKET_B);
    fs::write(&a, ticket_source(TICKET_A, TICKET_A, CLOSURE)).unwrap();
    fs::write(&b, ticket_source(TICKET_B, TICKET_B, &deps(&[TICKET_A]))).unwrap();
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let service = &enabled.service;

    let ready = ready(service, &repo);
    let blocked = blocked(service, &repo);

    // C waits for A, which is closed where it counts: in A's own context.
    // The copy of A on the primary branch, still open, decides nothing.
    assert!(
        fs::read_to_string(root.join(ticket_path(TICKET_A)))
            .unwrap()
            .contains("status: open\n---")
    );
    assert_eq!(sorted_ids(&ready), [TICKET_B, TICKET_C]);
    assert_eq!(sorted_ids(&blocked), [TICKET_D]);
    assert_eq!(ready.index.state, IndexState::Current);
    assert!(ready.complete && blocked.complete);
    for item in &ready.items {
        assert_eq!(state(item), ReadinessState::Ready);
        assert!(reasons(item).is_empty());
    }
    assert_eq!(
        ticket(&ready, TICKET_B).context.kind,
        ItemContextKind::Active
    );
    assert_eq!(
        ticket(&ready, TICKET_C).context.kind,
        ItemContextKind::Primary
    );
    let d = ticket(&blocked, TICKET_D);
    assert_eq!(state(d), ReadinessState::Blocked);
    assert_eq!(
        reasons(d),
        [(ReadinessReasonCode::OpenDependency, vec![TICKET_B], true)]
    );

    // With no readiness chosen, every open ticket; a closed one never.
    let open = service
        .ticket_readiness(&repo, &TicketFilter::default())
        .unwrap();
    assert_eq!(sorted_ids(&open), [TICKET_B, TICKET_C, TICKET_D]);
    let closed_only = service
        .ticket_readiness(
            &repo,
            &TicketFilter {
                closure: ClosureFilter::Closed,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(closed_only.items.is_empty());

    // The ticket list says the same of each ticket, and filters the same.
    let listed = all_tickets(service, &repo);
    assert_eq!(listed.items.len(), 4);
    let a = ticket(&listed, TICKET_A);
    assert_eq!(a.context.kind, ItemContextKind::Active);
    assert_eq!(state(a), ReadinessState::Closed);
    assert!(reasons(a).is_empty());
    for item in ready.items.iter().chain(&blocked.items) {
        assert_eq!(ticket(&listed, item.id.as_deref().unwrap()), item);
    }
    for (readiness, expected) in [
        (ReadinessFilter::Ready, &ready),
        (ReadinessFilter::Blocked, &blocked),
    ] {
        let filtered = service.list_tickets(&repo, &filter(readiness)).unwrap();
        assert_eq!(sorted_ids(&filtered), sorted_ids(expected));
    }
    // A complete read decides the same from the file.
    for (id, expected) in [
        (TICKET_A, ReadinessState::Closed),
        (TICKET_B, ReadinessState::Ready),
        (TICKET_C, ReadinessState::Ready),
        (TICKET_D, ReadinessState::Blocked),
    ] {
        let shown = service.show_item(&repo, &item_id(id)).unwrap();
        assert_eq!(state(&shown), expected, "{id}");
        assert_eq!(shown.readiness, ticket(&listed, id).readiness);
        assert_eq!(shown.index.state, IndexState::Current);
    }
    // The trees, the plan and the critical path follow the same copies.
    let down = tree(service, &repo, TICKET_D, DependencyDirection::Down, None);
    assert_eq!(
        lines(&down.dependencies),
        [TICKET_B.to_owned(), format!(" {TICKET_A}")]
    );
    assert_eq!(
        down.dependencies
            .iter()
            .map(|node| node.state)
            .collect::<Vec<_>>(),
        [DependencyState::Open, DependencyState::Closed]
    );
    let up = tree(service, &repo, TICKET_A, DependencyDirection::Up, None);
    assert_eq!(
        lines(&up.dependents),
        [
            TICKET_B.to_owned(),
            format!(" {TICKET_D}"),
            TICKET_C.to_owned(),
        ]
    );
    assert_eq!(up.ticket.state, DependencyState::Closed);
    let both = tree(service, &repo, TICKET_B, DependencyDirection::Both, Some(1));
    assert_eq!(lines(&both.dependencies), [TICKET_A.to_owned()]);
    assert_eq!(lines(&both.dependents), [TICKET_D.to_owned()]);
    let plan = plan(service, &repo);
    assert_eq!(
        batches(&plan),
        [(1, vec![TICKET_B, TICKET_C]), (2, vec![TICKET_D])]
    );
    assert!(plan.unplannable.is_empty());
    assert_eq!(batches(&plan)[0].1, sorted_ids(&ready));
    for (batch, kind) in [(0, ItemContextKind::Active), (1, ItemContextKind::Primary)] {
        assert_eq!(plan.batches[batch].items[0].context.kind, kind);
    }
    let path = service.ticket_critical_path(&repo).unwrap();
    assert_eq!(ids(&path), [TICKET_B, TICKET_D]);
    assert!(service.ticket_cycles(&repo).unwrap().items.is_empty());
    // Read by path, the primary copy of A is the open ticket its file is.
    let copy = service
        .show_path(&repo, None, Path::new(&ticket_path(TICKET_A)))
        .unwrap();
    assert_eq!(state(&copy), ReadinessState::Ready);
    assert_git_transport_uninitialized();
}

#[test]
fn an_unresolved_dependency_blocks_and_a_document_named_as_one_is_ignored_and_reported() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    write(root, "docs/a.md", &document_source(DOCUMENT_A, "D", ""));
    // Depends on an ID no context holds, and names a document twice over.
    write_ticket(
        root,
        TICKET_A,
        &format!(
            "parent: {DOCUMENT_A}\n{}",
            deps(&[DOCUMENT_A, TICKET_ABSENT])
        ),
    );
    // Depends on the document alone, which is no dependency at all.
    write_ticket(root, TICKET_B, &deps(&[DOCUMENT_A]));
    // Depends on a ticket whose file is not a ticket: no context holds a
    // ticket with that ID either.
    write(
        root,
        &ticket_path(TICKET_C),
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\n---\n",
    );
    write_ticket(root, TICKET_D, &deps(&[TICKET_C]));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let service = &enabled.service;

    let blocked = blocked(service, &repo);

    assert_eq!(sorted_ids(&blocked), [TICKET_A, TICKET_D]);
    let a = ticket(&blocked, TICKET_A);
    assert_eq!(
        reasons(a),
        [(
            ReadinessReasonCode::UnresolvedDependency,
            vec![TICKET_ABSENT],
            true
        )]
    );
    assert_eq!(a.parent, None);
    assert_eq!(a.deps.len(), 1);
    assert_eq!(
        codes(a),
        [(ProblemCode::RelationshipNotATicket, Some(DOCUMENT_A))]
    );
    assert_eq!(
        reasons(ticket(&blocked, TICKET_D)),
        [(
            ReadinessReasonCode::UnresolvedDependency,
            vec![TICKET_C],
            true
        )]
    );
    let ready_list = ready(service, &repo);
    assert_eq!(ids(&ready_list), [TICKET_B]);
    assert_eq!(
        codes(&ready_list.items[0]),
        [(ProblemCode::RelationshipNotATicket, Some(DOCUMENT_A))]
    );

    // The file that is not a ticket is an entry of the ticket list, under
    // every filter, and of no relationship read.
    let listed = service
        .list_tickets(&repo, &filter(ReadinessFilter::Ready))
        .unwrap();
    assert_eq!(
        listed
            .items
            .iter()
            .map(|item| item.id.as_deref())
            .collect::<Vec<_>>(),
        [Some(TICKET_B), None]
    );
    assert_eq!(listed.items[1].readiness, None);
    let plan = plan(service, &repo);
    assert_eq!(batches(&plan), [(1, vec![TICKET_B])]);
    assert_eq!(
        unplannable(&plan),
        [
            (
                TICKET_A,
                vec![(
                    UnplannableReasonCode::UnresolvedDependency,
                    vec![TICKET_ABSENT]
                )]
            ),
            (
                TICKET_D,
                vec![(UnplannableReasonCode::UnresolvedDependency, vec![TICKET_C])]
            ),
        ]
    );
    // The tree shows the unresolved dependency and not the document.
    let down = tree(service, &repo, TICKET_A, DependencyDirection::Down, None);
    assert_eq!(lines(&down.dependencies), [format!("{TICKET_ABSENT}?")]);
    assert_eq!(
        (&down.dependencies[0].slug, &down.dependencies[0].title),
        (&None, &None)
    );
    // A document has no readiness, and is no ticket to any of these reads.
    assert_eq!(
        service.list_documents(&repo).unwrap().items[0].readiness,
        None
    );
    for error in [
        service
            .ticket_dependencies(&repo, &item_id(DOCUMENT_A), DependencyDirection::Both, None)
            .unwrap_err(),
        service
            .ticket_children(&repo, &item_id(DOCUMENT_A))
            .unwrap_err(),
        service
            .ticket_children(&repo, &item_id(TICKET_ABSENT))
            .unwrap_err(),
        // The ID of the file that is not a ticket.
        service
            .ticket_dependencies(&repo, &item_id(TICKET_C), DependencyDirection::Up, None)
            .unwrap_err(),
    ] {
        assert_eq!(error.code(), ResultCode::ItemNotFound);
        assert!(error.scope.item_id.is_some() && error.scope.repository.is_some());
        // The index is current: a refresh would find nothing more.
        assert_eq!(recovery(&error), json!([]));
    }

    // When the ticket it waited for arrives, closed, the dependent is
    // ready, and its own file was never touched.
    write(
        root,
        &ticket_path(TICKET_ABSENT),
        &ticket_source(TICKET_ABSENT, "Arrived", CLOSURE),
    );
    refresh_completely(service, root);
    assert_eq!(sorted_ids(&ready(service, &repo)), [TICKET_A, TICKET_B]);
    assert_git_transport_uninitialized();
}

#[test]
fn dependency_trees_run_down_up_and_both_ways_to_a_depth() {
    let (fixture, enabled) = relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let service = &enabled.service;
    let tree = |id, direction, depth| tree(service, &repo, id, direction, depth);

    // Down from D: an ID no context holds, and C, which needs B, which
    // needs A.
    let down = tree(RELATED_D, DependencyDirection::Down, None);
    assert_eq!(
        lines(&down.dependencies),
        [
            format!("{TICKET_ABSENT}?"),
            RELATED_C.to_owned(),
            format!(" {RELATED_B}"),
            format!("  {RELATED_A}"),
        ]
    );
    assert!(down.dependents.is_empty());
    assert_eq!(down.direction, DependencyDirection::Down);
    assert_eq!(down.depth, None);
    assert!(down.complete);
    assert_eq!(down.index.state, IndexState::Current);
    // Each line carries closure state, and what to show beside the ID.
    assert_eq!(
        down.dependencies
            .iter()
            .map(|node| node.state)
            .collect::<Vec<_>>(),
        [
            DependencyState::Unresolved,
            DependencyState::Open,
            DependencyState::Open,
            DependencyState::Closed,
        ]
    );
    assert_eq!(down.dependencies[1].slug.as_deref(), Some(SHARED_SLUG));
    assert_eq!(down.dependencies[1].title.as_deref(), Some("C"));
    assert_eq!(down.dependencies[3].slug, None);
    let root = &down.ticket;
    assert_eq!(
        (root.id.as_str(), root.state, root.depth),
        (RELATED_D, DependencyState::Open, 0)
    );
    assert_eq!(root.title.as_deref(), Some("D"));
    assert!(!root.repeated && !root.truncated);

    // Up from A, which is closed: everything that waits for it.
    let up = tree(RELATED_A, DependencyDirection::Up, None);
    assert_eq!(
        lines(&up.dependents),
        [
            RELATED_B.to_owned(),
            format!(" {RELATED_C}"),
            format!("  {RELATED_D}"),
        ]
    );
    assert!(up.dependencies.is_empty());
    assert_eq!(up.ticket.state, DependencyState::Closed);

    // Both ways from C, to a depth of one.
    let both = tree(RELATED_C, DependencyDirection::Both, Some(1));
    assert_eq!(lines(&both.dependencies), [format!("{RELATED_B}+")]);
    assert_eq!(lines(&both.dependents), [RELATED_D.to_owned()]);
    assert_eq!(both.depth, Some(1));
    assert!(!both.ticket.truncated);
    // To a depth of nothing, the ticket itself says what was left out.
    let none = tree(RELATED_C, DependencyDirection::Both, Some(0));
    assert!(none.dependencies.is_empty() && none.dependents.is_empty());
    assert!(none.ticket.truncated);
    assert!(
        !tree(RELATED_A, DependencyDirection::Down, Some(0))
            .ticket
            .truncated
    );

    // Through the cycle of E and F the tree ends, in both directions.
    let cycle = tree(RELATED_E, DependencyDirection::Both, None);
    assert_eq!(
        lines(&cycle.dependencies),
        [RELATED_F.to_owned(), format!(" {RELATED_E}*")]
    );
    assert_eq!(
        lines(&cycle.dependents),
        [
            RELATED_F.to_owned(),
            format!(" {RELATED_E}*"),
            RELATED_G.to_owned(),
        ]
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_parent_groups_tickets_and_never_blocks_them() {
    let (fixture, enabled) = relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let service = &enabled.service;
    let children = |id: &str| service.ticket_children(&repo, &item_id(id)).unwrap();

    // B's direct children, latest change first.
    let of_b = children(RELATED_B);
    assert_eq!(ids(&of_b), [RELATED_G, RELATED_C]);
    assert!(of_b.complete);
    // Each child carries its parent on its own DTO.
    for child in &of_b.items {
        let parent = child.parent.as_ref().unwrap();
        assert_eq!(
            (parent.id.as_str(), parent.state),
            (RELATED_B, DependencyState::Open)
        );
    }
    // A child's children are not its parent's: nothing is rolled up.
    assert!(children(RELATED_C).items.is_empty());
    assert!(children(RELATED_A).items.is_empty());

    // E and F are each the other's parent. Each is a root: it is no
    // ticket's child, still names its parent, and says what is wrong.
    assert!(children(RELATED_E).items.is_empty());
    assert!(children(RELATED_F).items.is_empty());
    let listed = all_tickets(service, &repo);
    for (id, parent) in [(RELATED_E, RELATED_F), (RELATED_F, RELATED_E)] {
        let item = ticket(&listed, id);
        assert_eq!(item.parent.as_ref().unwrap().id, parent);
        // They wait for each other as well, which is a problem of its own,
        // named by the lowest ID of that cycle.
        assert_eq!(
            codes(item),
            [
                (ProblemCode::DependencyCycle, Some(RELATED_E)),
                (ProblemCode::ParentCycle, Some(parent)),
            ]
        );
        let shown = service.show_item(&repo, &item_id(id)).unwrap();
        assert_eq!(shown.problems, item.problems);
        assert_eq!(shown.index.state, IndexState::Current);
    }
    // No other ticket has a problem, and none is nonconforming.
    for id in [RELATED_A, RELATED_B, RELATED_C, RELATED_D, RELATED_G] {
        assert_eq!(codes(ticket(&listed, id)), []);
    }
    assert_eq!(listed.items.len(), 7);

    // C is ready or not by its dependency alone. Its parent B is open, and
    // closing B is what makes C ready, as closing any dependency would;
    // G, another child of B, stays blocked by the dependency it has.
    assert_eq!(state(ticket(&listed, RELATED_C)), ReadinessState::Blocked);
    write(
        &fixture.root,
        &ticket_path(RELATED_B),
        &ticket_source(RELATED_B, "B", CLOSURE),
    );
    refresh_completely(service, &fixture.root);
    let listed = all_tickets(service, &repo);
    assert_eq!(state(ticket(&listed, RELATED_C)), ReadinessState::Ready);
    assert_eq!(state(ticket(&listed, RELATED_G)), ReadinessState::Blocked);
    // A closed parent still has its open children.
    assert_eq!(ids(&children(RELATED_B)), [RELATED_G, RELATED_C]);
    assert_eq!(
        ticket(&listed, RELATED_C).parent.as_ref().unwrap().state,
        DependencyState::Closed
    );
    assert_git_transport_uninitialized();
}

#[test]
fn filters_choose_which_tickets_are_returned_and_never_which_are_ready() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let project = |project: &str, extra: &str| format!("project: {project}\n{extra}");
    // alpha waits for beta, which waits for nothing.
    write_ticket(root, TICKET_A, &project("beta", ""));
    write_ticket(root, TICKET_B, &project("alpha", &deps(&[TICKET_A])));
    write_ticket(root, TICKET_C, &project("alpha", ""));
    write(
        root,
        &ticket_path(TICKET_D),
        &ticket_source_with(
            TICKET_D,
            "D",
            "bug",
            "triage",
            &project("alpha", &deps(&[TICKET_B])),
        ),
    );
    // A file under the tickets that is not a ticket.
    write(
        root,
        &ticket_path(TICKET_E),
        "---\nmanyhands_managed: true\n---\n",
    );
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let service = &enabled.service;
    let of = |project: &str, readiness| TicketFilter {
        project: Some(project.to_owned()),
        readiness,
        ..Default::default()
    };
    let readiness = |filter: &TicketFilter| service.ticket_readiness(&repo, filter).unwrap();

    // Leaving beta out does not make the ticket that waits for it ready.
    assert_eq!(
        sorted_ids(&readiness(&of("alpha", Some(ReadinessFilter::Ready)))),
        [TICKET_C]
    );
    assert_eq!(
        sorted_ids(&readiness(&of("alpha", Some(ReadinessFilter::Blocked)))),
        [TICKET_B, TICKET_D]
    );
    assert_eq!(
        sorted_ids(&readiness(&of("beta", Some(ReadinessFilter::Ready)))),
        [TICKET_A]
    );
    assert!(
        readiness(&of("beta", Some(ReadinessFilter::Blocked)))
            .items
            .is_empty()
    );
    assert!(readiness(&of("gamma", None)).items.is_empty());
    let blocked_alpha = readiness(&of("alpha", Some(ReadinessFilter::Blocked)));
    assert_eq!(
        reasons(ticket(&blocked_alpha, TICKET_B)),
        [(ReadinessReasonCode::OpenDependency, vec![TICKET_A], true)]
    );
    // The other list filters apply as they do to the ticket list.
    let triage = readiness(&TicketFilter {
        status: Some("triage".to_owned()),
        ticket_type: Some("bug".to_owned()),
        ..Default::default()
    });
    assert_eq!(ids(&triage), [TICKET_D]);

    // The plan of every ticket: A and C, then B, then D.
    let whole = plan(service, &repo);
    assert_eq!(
        batches(&whole),
        [
            (1, vec![TICKET_A, TICKET_C]),
            (2, vec![TICKET_B]),
            (3, vec![TICKET_D]),
        ]
    );
    assert!(whole.unplannable.is_empty() && whole.complete);
    // A project's part of it keeps each ticket in its own batch.
    let alpha = service.ticket_plan(&repo, &of("alpha", None)).unwrap();
    assert_eq!(
        batches(&alpha),
        [
            (1, vec![TICKET_C]),
            (2, vec![TICKET_B]),
            (3, vec![TICKET_D]),
        ]
    );
    let beta = service.ticket_plan(&repo, &of("beta", None)).unwrap();
    assert_eq!(batches(&beta), [(1, vec![TICKET_A])]);
    // A batch the filter empties is not listed, and the next keeps its
    // number.
    let bugs = service
        .ticket_plan(
            &repo,
            &TicketFilter {
                ticket_type: Some("bug".to_owned()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(batches(&bugs), [(3, vec![TICKET_D])]);
    let blocked_only = service
        .ticket_plan(&repo, &filter(ReadinessFilter::Blocked))
        .unwrap();
    assert_eq!(
        batches(&blocked_only),
        [(2, vec![TICKET_B]), (3, vec![TICKET_D])]
    );
    // No relationship read lists the file that is not a ticket; the
    // ticket list does, under any filter.
    let listed = service
        .list_tickets(&repo, &of("gamma", Some(ReadinessFilter::Blocked)))
        .unwrap();
    assert_eq!(listed.items.len(), 1);
    assert_eq!(listed.items[0].id, None);
    assert_eq!(
        whole
            .batches
            .iter()
            .flat_map(|batch| &batch.items)
            .filter(|item| item.id.is_none())
            .count(),
        0
    );
    assert_eq!(
        ids(&service.ticket_critical_path(&repo).unwrap()),
        [TICKET_A, TICKET_B, TICKET_D]
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_status_that_says_closed_closes_nothing() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    // A says it is closed and carries no closure metadata.
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source_with(TICKET_A, "A", "task", "closed", ""),
    );
    write_ticket(root, TICKET_B, &deps(&[TICKET_A]));
    refresh_completely(&enabled.service, root);
    let repo = enabled.service.resolve_repository(root).unwrap();
    let service = &enabled.service;

    assert_eq!(ids(&ready(service, &repo)), [TICKET_A]);
    let blocked_list = blocked(service, &repo);
    assert_eq!(ids(&blocked_list), [TICKET_B]);
    assert_eq!(
        reasons(&blocked_list.items[0]),
        [(ReadinessReasonCode::OpenDependency, vec![TICKET_A], true)]
    );
    assert_eq!(blocked_list.items[0].deps[0].state, DependencyState::Open);
    assert_eq!(
        batches(&plan(service, &repo)),
        [(1, vec![TICKET_A]), (2, vec![TICKET_B])]
    );

    // Closure metadata closes it, whatever the status says.
    write(
        root,
        &ticket_path(TICKET_A),
        &ticket_source_with(TICKET_A, "A", "task", "in progress", CLOSURE),
    );
    refresh_completely(service, root);
    assert_eq!(ids(&ready(service, &repo)), [TICKET_B]);
    assert!(blocked(service, &repo).items.is_empty());
    assert_eq!(batches(&plan(service, &repo)), [(1, vec![TICKET_B])]);
    assert_git_transport_uninitialized();
}

#[test]
fn closing_a_dependency_makes_its_dependent_ready_without_the_dependent_being_observed_again() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let data = enabled.data_directory.path();
    let service = &enabled.service;
    // B, on the primary branch, depends on A, which has its own worktree.
    write_ticket(root, TICKET_B, &deps(&[TICKET_A]));
    commit(&fixture, &[&ticket_path(TICKET_B)], 1_000);
    let a = save_ticket(service, root, TICKET_A, ContextIntent::Create);
    refresh_completely(service, root);
    let repo = service.resolve_repository(root).unwrap();
    assert_eq!(ids(&blocked(service, &repo)), [TICKET_B]);
    let edges = || -> Vec<(String, String)> {
        index(data)
            .prepare(
                "SELECT items.item_id, edges.target_id FROM item_edges AS edges
                   JOIN discovered_items AS items ON items.id = edges.item_id
                  ORDER BY edges.id",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    let edges_before = edges();
    assert_eq!(edges_before.len(), 1);
    let dependent = fs::read(root.join(ticket_path(TICKET_B))).unwrap();

    // The index is told that A is closed, and nothing else: what it holds
    // once A's context has been observed again and B's has not. No public
    // call observes one context alone, so A's row is changed directly.
    let closed = index(data)
        .execute(
            "UPDATE discovered_items SET closed_at = 1, closed_by = 'Ada' WHERE item_id = ?1",
            [TICKET_A],
        )
        .unwrap();
    assert_eq!(closed, 1);

    // B is ready: nothing about it was stored that could go stale.
    assert_eq!(edges(), edges_before);
    let ready_list = ready(service, &repo);
    assert_eq!(ids(&ready_list), [TICKET_B]);
    assert_eq!(ready_list.items[0].deps[0].state, DependencyState::Closed);
    assert!(blocked(service, &repo).items.is_empty());
    assert_eq!(batches(&plan(service, &repo)), [(1, vec![TICKET_B])]);

    // The index follows the files: A's still says it is open.
    refresh_completely(service, root);
    assert_eq!(ids(&blocked(service, &repo)), [TICKET_B]);
    // Closed in its own worktree and refreshed, it unblocks B, whose file
    // nobody wrote to.
    let open = fs::read_to_string(&a).unwrap();
    fs::write(&a, open.replacen("---\n", &format!("---\n{CLOSURE}"), 1)).unwrap();
    refresh_completely(service, root);
    assert_eq!(ids(&ready(service, &repo)), [TICKET_B]);
    assert_eq!(edges(), edges_before);
    assert_eq!(
        fs::read(root.join(ticket_path(TICKET_B))).unwrap(),
        dependent
    );
    assert_git_transport_uninitialized();
}

#[test]
fn two_branches_each_valid_alone_form_a_cycle_and_share_a_short_code_and_neither_is_repaired() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let service = &enabled.service;
    write_ticket(root, TICKET_A, "");
    write_ticket(root, TICKET_B, "");
    write_ticket(root, TICKET_C, &deps(&[TICKET_A]));
    let paths = [TICKET_A, TICKET_B, TICKET_C].map(ticket_path);
    commit(
        &fixture,
        &paths.iter().map(String::as_str).collect::<Vec<_>>(),
        1_000,
    );
    let a = edit_context(service, root, TICKET_A);
    let b = edit_context(service, root, TICKET_B);
    // On A's branch A comes to depend on B, which depends on nothing
    // there; on B's branch it is the other way round. Each gives its
    // ticket the same short code, which the other branch does not have.
    let slug = format!("slug: {SHARED_SLUG}\n");
    fs::write(
        &a,
        ticket_source(TICKET_A, TICKET_A, &format!("{slug}{}", deps(&[TICKET_B]))),
    )
    .unwrap();
    fs::write(
        &b,
        ticket_source(TICKET_B, TICKET_B, &format!("{slug}{}", deps(&[TICKET_A]))),
    )
    .unwrap();
    for (branch, other) in [(TICKET_A, TICKET_B), (TICKET_B, TICKET_A)] {
        let copy = fs::read_to_string(context_worktree(root, branch).join(ticket_path(other)));
        assert!(!copy.unwrap().contains("deps"));
    }
    refresh_completely(service, root);
    let repo = service.resolve_repository(root).unwrap();
    let before = support::repository_and_worktree_snapshot(&fixture);

    let blocked = blocked(service, &repo);
    let cycles = service.ticket_cycles(&repo).unwrap();
    let found = service.find_tickets_by_slug(&repo, SHARED_SLUG).unwrap();
    let plan = plan(service, &repo);
    let path = service.ticket_critical_path(&repo).unwrap();

    // Both tickets, and the one that waits for them, are blocked; the two
    // on the cycle name it.
    assert_eq!(sorted_ids(&blocked), [TICKET_A, TICKET_B, TICKET_C]);
    for (id, other) in [(TICKET_A, TICKET_B), (TICKET_B, TICKET_A)] {
        assert_eq!(
            reasons(ticket(&blocked, id)),
            [
                (ReadinessReasonCode::OpenDependency, vec![other], true),
                (
                    ReadinessReasonCode::DependencyCycle,
                    vec![TICKET_A, TICKET_B],
                    true
                ),
            ]
        );
    }
    assert_eq!(
        reasons(ticket(&blocked, TICKET_C)),
        [(ReadinessReasonCode::OpenDependency, vec![TICKET_A], true)]
    );
    assert!(ready(service, &repo).items.is_empty());
    // The cycle is reported once.
    assert_eq!(cycles.items.len(), 1);
    assert_eq!(cycles.items[0].kind, CycleKind::Deps);
    assert_eq!(cycles.items[0].ids, [TICKET_A, TICKET_B]);
    assert!(cycles.complete);
    assert_eq!(cycles.index.state, IndexState::Current);
    // Nothing can be planned, and each ticket says why.
    assert!(plan.batches.is_empty());
    assert_eq!(
        unplannable(&plan),
        [
            (
                TICKET_A,
                vec![(
                    UnplannableReasonCode::DependencyCycle,
                    vec![TICKET_A, TICKET_B]
                )]
            ),
            (
                TICKET_B,
                vec![(
                    UnplannableReasonCode::DependencyCycle,
                    vec![TICKET_A, TICKET_B]
                )]
            ),
            (
                TICKET_C,
                vec![(UnplannableReasonCode::UnplannableDependency, vec![TICKET_A])]
            ),
        ]
    );
    assert!(path.items.is_empty());

    // Both tickets carry the short code. Both are found, each with its ID
    // and the context it is in, and neither has a problem for it.
    assert_eq!(sorted_ids(&found), [TICKET_A, TICKET_B]);
    for item in &found.items {
        let id = item.id.as_deref().unwrap();
        assert_eq!(item.slug.as_deref(), Some(SHARED_SLUG));
        assert_eq!(item.context.kind, ItemContextKind::Active);
        assert_eq!(
            Path::new(&item.context.worktree),
            context_worktree(root, id)
        );
        // The short code is no problem. The cycle is, on both of them.
        assert_eq!(
            codes(item),
            [(ProblemCode::DependencyCycle, Some(TICKET_A))]
        );
        assert!(item.title.is_some() && item.closure.is_some());
    }
    let listed = all_tickets(service, &repo);
    assert_eq!(listed.items.len(), 3);
    assert!(listed.items.iter().all(|item| item.id.is_some()));

    // Nothing was repaired: every file, ref and worktree is as it was.
    assert!(before == support::repository_and_worktree_snapshot(&fixture));
    for file in [&a, &b] {
        assert!(fs::read_to_string(file).unwrap().contains("deps: ["));
    }
    assert_git_transport_uninitialized();
}

#[test]
fn a_short_code_finds_every_ticket_that_carries_it_and_identifies_none() {
    let (fixture, enabled) = relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let service = &enabled.service;
    write_ticket(&fixture.root, TICKET_A, "slug: zz-ab-00001\n");
    // Not a short code: kept in the file, reported, and not searchable.
    write_ticket(&fixture.root, TICKET_B, "slug: Not-A-Slug\n");
    refresh_completely(service, &fixture.root);
    let find = |slug: &str| service.find_tickets_by_slug(&repo, slug).unwrap();

    // None, one and two matches; none is success with an empty list.
    let none = find("mh-vh-00000");
    assert!(none.items.is_empty() && none.complete);
    assert_eq!(none.index.state, IndexState::Current);
    assert_eq!(ids(&find("zz-ab-00001")), [TICKET_A]);
    // Two, latest change first, each with what a person needs to choose.
    let two = find(SHARED_SLUG);
    assert_eq!(ids(&two), [RELATED_C, RELATED_B]);
    for (item, title) in two.items.iter().zip(["C", "B"]) {
        assert_eq!(item.title.as_deref(), Some(title));
        assert_eq!(item.ticket_type.as_deref(), Some("task"));
        assert_eq!(item.status.as_deref(), Some("open"));
        assert!(item.closure.is_some() && item.readiness.is_some());
        assert_eq!(item.context.kind, ItemContextKind::Primary);
        assert_eq!(codes(item), []);
    }
    // Without regard to case, and only as a whole code.
    assert_eq!(ids(&find("MH-VH-K9X2B")), [RELATED_C, RELATED_B]);
    assert_eq!(ids(&find("Mh-Vh-k9X2b")), [RELATED_C, RELATED_B]);
    for partial in [
        "mh-vh-k9x2",
        "vh-k9x2b",
        "k9x2b",
        "mh-vh-k9x2b ",
        " mh-vh-k9x2b",
        "mh-vh-k9x2b\n",
        "",
        "%",
        "mh-vh-k9x2_",
        // A ticket's ID is not its short code.
        RELATED_B,
        // The value that is not a short code finds nothing.
        "Not-A-Slug",
        "not-a-slug",
    ] {
        assert!(find(partial).items.is_empty(), "{partial:?}");
    }
    // The ticket list's own filter finds the same two.
    let listed = service
        .list_tickets(
            &repo,
            &TicketFilter {
                slug: Some(SHARED_SLUG.to_uppercase()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(ids(&listed), [RELATED_C, RELATED_B]);
    assert_git_transport_uninitialized();
}

/// The Cycles of Wave 03 as tickets: each depends on what the Wave
/// document's track rules say it depends on.
const WAVE: [(&str, &[&str]); 14] = [
    ("F1", &[]),
    ("F2", &["F1"]),
    ("C1", &["F2"]),
    ("C2", &["C1"]),
    ("C3", &["C2"]),
    ("C4", &["C3"]),
    ("C5", &["C4"]),
    ("D1", &["F2"]),
    ("D2", &["D1"]),
    ("D3", &["D2"]),
    ("D4", &["D3"]),
    ("D5", &["D4"]),
    ("D6", &["D5"]),
    ("G1", &["C5", "D6"]),
];

/// The ticket ID of a Wave 03 Cycle, in the order the Cycles are listed.
fn cycle_id(name: &str) -> String {
    let position = WAVE.iter().position(|(cycle, _)| *cycle == name).unwrap();
    format!("01ARZ3NDEKTSV4RRFFQ69G5W{position:02}")
}

#[test]
fn the_projects_own_wave_gives_one_ready_cycle_and_a_plan_that_follows_its_tracks() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let service = &enabled.service;
    let foundation = cycle_id("F1");
    for (name, needs) in WAVE {
        let needs: Vec<String> = needs.iter().map(|name| cycle_id(name)).collect();
        // Every Cycle of the Wave belongs to the Wave's own ticket.
        let parent = if name == "F1" {
            String::new()
        } else {
            format!("parent: {foundation}\n")
        };
        let id = cycle_id(name);
        write(
            root,
            &ticket_path(&id),
            &ticket_source(
                &id,
                name,
                &format!(
                    "{parent}{}",
                    deps(&needs.iter().map(String::as_str).collect::<Vec<_>>())
                ),
            ),
        );
    }
    refresh_completely(service, root);
    let repo = service.resolve_repository(root).unwrap();
    let named = |items: &[ItemDto]| {
        items
            .iter()
            .map(|item| item.title.clone().unwrap())
            .collect::<Vec<_>>()
    };

    // F1 alone is ready, and thirteen wait.
    assert_eq!(named(&ready(service, &repo).items), ["F1"]);
    assert_eq!(blocked(service, &repo).items.len(), 13);
    // The foundation serially, the two tracks side by side, then the
    // convergence Cycle once the longer track is done.
    let plan = plan(service, &repo);
    assert_eq!(
        plan.batches
            .iter()
            .map(|batch| (batch.batch, named(&batch.items)))
            .collect::<Vec<_>>(),
        [
            (1, vec!["F1".to_owned()]),
            (2, vec!["F2".to_owned()]),
            (3, vec!["C1".to_owned(), "D1".to_owned()]),
            (4, vec!["C2".to_owned(), "D2".to_owned()]),
            (5, vec!["C3".to_owned(), "D3".to_owned()]),
            (6, vec!["C4".to_owned(), "D4".to_owned()]),
            (7, vec!["C5".to_owned(), "D5".to_owned()]),
            (8, vec!["D6".to_owned()]),
            (9, vec!["G1".to_owned()]),
        ]
    );
    assert!(plan.unplannable.is_empty());
    // The Wave document's longest serial path, nine Cycles.
    assert_eq!(
        named(&service.ticket_critical_path(&repo).unwrap().items),
        ["F1", "F2", "D1", "D2", "D3", "D4", "D5", "D6", "G1"]
    );
    assert!(service.ticket_cycles(&repo).unwrap().items.is_empty());
    assert_eq!(
        service
            .ticket_children(&repo, &item_id(&foundation))
            .unwrap()
            .items
            .len(),
        13
    );
    // Everything waits for F1, directly or not.
    let up = tree(service, &repo, &foundation, DependencyDirection::Up, None);
    assert_eq!(up.dependents.len(), 14);
    assert_eq!(up.dependents.iter().filter(|node| node.repeated).count(), 1);
    let down = tree(
        service,
        &repo,
        &cycle_id("G1"),
        DependencyDirection::Down,
        None,
    );
    assert_eq!(down.dependencies.len(), 14);
    assert_eq!(
        down.dependencies.iter().map(|node| node.depth).max(),
        Some(7)
    );
    assert_git_transport_uninitialized();
}

#[test]
fn a_relationship_read_says_how_far_behind_the_index_is_and_fails_only_when_it_is_unavailable() {
    // Never refreshed: the index holds nothing, and says so.
    let never = never_refreshed_repository();
    let repo = never
        .service
        .resolve_repository(&never.fixture.root)
        .unwrap();
    let service = &never.service;
    let default = TicketFilter::default();
    let states = |service: &RepositoryService, repo: &ResolvedRepository| {
        let readiness = service.ticket_readiness(repo, &default).unwrap();
        let cycles = service.ticket_cycles(repo).unwrap();
        let plan = service.ticket_plan(repo, &default).unwrap();
        let path = service.ticket_critical_path(repo).unwrap();
        let found = service.find_tickets_by_slug(repo, SHARED_SLUG).unwrap();
        (
            [
                readiness.items.len(),
                cycles.items.len(),
                plan.batches.len() + plan.unplannable.len(),
                path.items.len(),
                found.items.len(),
            ],
            [
                readiness.index.state,
                cycles.index.state,
                plan.index.state,
                path.index.state,
                found.index.state,
            ],
        )
    };
    assert_eq!(
        states(service, &repo),
        ([0; 5], [IndexState::NeverRefreshed; 5])
    );
    // A ticket the index may not hold yet: a refresh is the recovery.
    let root = fs::canonicalize(&never.fixture.root).unwrap();
    for error in [
        service
            .ticket_dependencies(&repo, &item_id(TICKET_A), DependencyDirection::Down, None)
            .unwrap_err(),
        service
            .ticket_children(&repo, &item_id(TICKET_A))
            .unwrap_err(),
    ] {
        assert_eq!(error.code(), ResultCode::ItemNotFound);
        assert_eq!(error.scope.item_id.as_deref(), Some(TICKET_A));
        assert_eq!(
            recovery(&error),
            json!([{
                "action": "index.refresh",
                "operation_id": null,
                "arguments": {"root": root.to_str().unwrap()},
            }])
        );
    }

    // Current, then marked for a refresh: the same rows, and stale.
    let (fixture, enabled) = relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let current = states(&enabled.service, &repo);
    assert_eq!(current, ([6, 2, 6, 2, 2], [IndexState::Current; 5]));
    index(enabled.data_directory.path())
        .execute("UPDATE repositories SET refresh_required = 1", [])
        .unwrap();
    assert_eq!(
        states(&enabled.service, &repo),
        (current.0, [IndexState::Stale; 5])
    );
    let stale = tree(
        &enabled.service,
        &repo,
        RELATED_C,
        DependencyDirection::Both,
        None,
    );
    assert_eq!(stale.index.state, IndexState::Stale);
    let children = enabled
        .service
        .ticket_children(&repo, &item_id(RELATED_B))
        .unwrap();
    assert_eq!(children.index.state, IndexState::Stale);
    assert_eq!(children.items[0].index.state, IndexState::Stale);
    // A file changed since the refresh is not looked at: no relationship
    // read opens a ticket's file.
    fs::remove_dir_all(fixture.root.join(".manyhands/tickets")).unwrap();
    assert_eq!(
        states(&enabled.service, &repo),
        (current.0, [IndexState::Stale; 5])
    );

    // Unavailable: the one failure, with a rebuild as the recovery.
    let (_data, service) = degraded_service(enabled);
    let failures = [
        service.ticket_readiness(&repo, &default).map(drop),
        service
            .ticket_dependencies(&repo, &item_id(RELATED_C), DependencyDirection::Down, None)
            .map(drop),
        service
            .ticket_children(&repo, &item_id(RELATED_B))
            .map(drop),
        service.ticket_cycles(&repo).map(drop),
        service.ticket_plan(&repo, &default).map(drop),
        service.ticket_critical_path(&repo).map(drop),
        service.find_tickets_by_slug(&repo, SHARED_SLUG).map(drop),
    ];
    for failure in failures {
        let error = failure.unwrap_err();
        assert_eq!(error.code(), ResultCode::IndexUnavailable);
        assert_eq!(error.recovery.len(), 1);
        assert_eq!(error.recovery[0].action.as_str(), "index.rebuild");
    }
    assert_git_transport_uninitialized();
}

#[test]
fn relationship_reads_are_in_the_ticket_list_ordering_unless_they_define_their_own() {
    let (fixture, enabled) = relationship_repository();
    let repo = enabled.service.resolve_repository(&fixture.root).unwrap();
    let service = &enabled.service;

    // Each ticket was committed ten seconds after the one before, so the
    // list ordering, latest change first, is the reverse of ID order.
    assert_eq!(
        ids(&service
            .ticket_readiness(&repo, &TicketFilter::default())
            .unwrap()),
        [
            RELATED_G, RELATED_F, RELATED_E, RELATED_D, RELATED_C, RELATED_B
        ]
    );
    assert_eq!(
        ids(&blocked(service, &repo)),
        [RELATED_G, RELATED_F, RELATED_E, RELATED_D, RELATED_C]
    );
    assert_eq!(
        ids(&service.ticket_children(&repo, &item_id(RELATED_B)).unwrap()),
        [RELATED_G, RELATED_C]
    );
    // The plan is in batch order and, within a batch and among the
    // unplannable, in ID order.
    let plan = plan(service, &repo);
    assert_eq!(batches(&plan), [(1, vec![RELATED_B]), (2, vec![RELATED_C])]);
    assert_eq!(
        unplannable(&plan),
        [
            (
                RELATED_D,
                vec![(
                    UnplannableReasonCode::UnresolvedDependency,
                    vec![TICKET_ABSENT]
                )]
            ),
            (
                RELATED_E,
                vec![(
                    UnplannableReasonCode::DependencyCycle,
                    vec![RELATED_E, RELATED_F]
                )]
            ),
            (
                RELATED_F,
                vec![(
                    UnplannableReasonCode::DependencyCycle,
                    vec![RELATED_E, RELATED_F]
                )]
            ),
            (
                RELATED_G,
                vec![(
                    UnplannableReasonCode::UnplannableDependency,
                    vec![RELATED_E]
                )]
            ),
        ]
    );
    assert!(plan.unplannable.iter().all(|entry| {
        entry.reasons.iter().all(|reason| reason.complete)
            && state(&entry.ticket) == ReadinessState::Blocked
    }));
    // The critical path is in the order the tickets are worked.
    assert_eq!(
        ids(&service.ticket_critical_path(&repo).unwrap()),
        [RELATED_B, RELATED_C]
    );
    // Dependency cycles, then parent cycles.
    let cycles = service.ticket_cycles(&repo).unwrap();
    assert_eq!(
        cycles
            .items
            .iter()
            .map(|cycle| (cycle.kind, cycle.ids.iter().map(String::as_str).collect()))
            .collect::<Vec<(CycleKind, Vec<&str>)>>(),
        [
            (CycleKind::Deps, vec![RELATED_E, RELATED_F]),
            (CycleKind::Parent, vec![RELATED_E, RELATED_F]),
        ]
    );
    // Every read gives the same answer every time.
    assert!(plan == self::plan(service, &repo));
    assert!(cycles == service.ticket_cycles(&repo).unwrap());
    assert_git_transport_uninitialized();
}

#[test]
fn a_file_changed_since_the_refresh_decides_its_own_readiness_when_read_whole() {
    let (fixture, enabled) = relationship_repository();
    let root = &fixture.root;
    let repo = enabled.service.resolve_repository(root).unwrap();
    let service = &enabled.service;

    // B, which was ready, comes to depend on D, which waits for C, which
    // waits for B: a cycle the index has not been told of.
    write(
        root,
        &ticket_path(RELATED_B),
        &ticket_source(RELATED_B, "B", &deps(&[RELATED_A, RELATED_D])),
    );

    let shown = service.show_item(&repo, &item_id(RELATED_B)).unwrap();
    assert_eq!(shown.index.state, IndexState::Stale);
    assert_eq!(
        reasons(&shown),
        [
            (ReadinessReasonCode::OpenDependency, vec![RELATED_D], true),
            (
                ReadinessReasonCode::DependencyCycle,
                vec![RELATED_B, RELATED_C, RELATED_D],
                true
            ),
        ]
    );
    // The lists give what the index holds until it is refreshed.
    assert_eq!(ids(&ready(service, &repo)), [RELATED_B]);
    assert_eq!(service.ticket_cycles(&repo).unwrap().items.len(), 2);
    refresh_completely(service, root);
    assert!(ready(service, &repo).items.is_empty());
    assert_eq!(service.ticket_cycles(&repo).unwrap().items.len(), 3);
    assert_git_transport_uninitialized();
}

#[test]
fn a_cycle_through_a_closed_ticket_is_reported_and_blocks_nothing() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let service = &enabled.service;
    // A waits for B, B for C, and C, which is closed, for A.
    write_ticket(root, TICKET_A, &deps(&[TICKET_B]));
    write_ticket(root, TICKET_B, &deps(&[TICKET_C]));
    write(
        root,
        &ticket_path(TICKET_C),
        &ticket_source(TICKET_C, "C", &format!("{CLOSURE}{}", deps(&[TICKET_A]))),
    );
    refresh_completely(service, root);
    let repo = service.resolve_repository(root).unwrap();

    // B's only dependency is closed, so B is ready, and A waits for B and
    // for nothing else.
    assert_eq!(ids(&ready(service, &repo)), [TICKET_B]);
    let blocked_list = blocked(service, &repo);
    assert_eq!(ids(&blocked_list), [TICKET_A]);
    assert_eq!(
        reasons(&blocked_list.items[0]),
        [(ReadinessReasonCode::OpenDependency, vec![TICKET_B], true)]
    );
    let plan = plan(service, &repo);
    assert_eq!(batches(&plan), [(1, vec![TICKET_B]), (2, vec![TICKET_A])]);
    assert!(plan.unplannable.is_empty());
    assert_eq!(
        ids(&service.ticket_critical_path(&repo).unwrap()),
        [TICKET_B, TICKET_A]
    );
    // The cycle is there all the same, and each of the three says so.
    let cycles = service.ticket_cycles(&repo).unwrap();
    assert_eq!(cycles.items.len(), 1);
    assert_eq!(cycles.items[0].ids, [TICKET_A, TICKET_B, TICKET_C]);
    let listed = all_tickets(service, &repo);
    for id in [TICKET_A, TICKET_B, TICKET_C] {
        let item = ticket(&listed, id);
        assert_eq!(
            codes(item),
            [(ProblemCode::DependencyCycle, Some(TICKET_A))],
            "{id}"
        );
        let shown = service.show_item(&repo, &item_id(id)).unwrap();
        assert_eq!(shown.problems, item.problems);
        assert_eq!(shown.readiness, item.readiness);
        assert_eq!(shown.index.state, IndexState::Current);
    }
    assert_eq!(state(ticket(&listed, TICKET_C)), ReadinessState::Closed);
    assert_git_transport_uninitialized();
}

/// More dependencies than a read could afford to compare each with each.
const MANY_DEPENDENCIES: usize = 20_000;

#[test]
fn a_ticket_with_very_many_dependencies_is_read_from_the_index_with_all_of_them() {
    let (fixture, enabled) = enabled();
    let root = &fixture.root;
    let service = &enabled.service;
    let targets: Vec<String> = (0..MANY_DEPENDENCIES)
        .map(|number| format!("01ARZ3NDEKTSV4RRFFQ6{number:06X}"))
        .collect();
    write_ticket(
        root,
        TICKET_A,
        &deps(&targets.iter().map(String::as_str).collect::<Vec<_>>()),
    );
    refresh_completely(service, root);
    let repo = service.resolve_repository(root).unwrap();

    let listed = all_tickets(service, &repo);

    let a = ticket(&listed, TICKET_A);
    assert_eq!(a.deps.len(), MANY_DEPENDENCIES);
    assert_eq!(
        a.deps.iter().map(|dep| dep.id.as_str()).collect::<Vec<_>>(),
        targets
    );
    assert_eq!(reasons(a).len(), MANY_DEPENDENCIES);
    assert_eq!(codes(a), []);
    let down = tree(service, &repo, TICKET_A, DependencyDirection::Down, None);
    assert_eq!(down.dependencies.len(), MANY_DEPENDENCIES);
    assert_eq!(
        plan(service, &repo).unplannable[0].reasons.len(),
        MANY_DEPENDENCIES
    );
    assert_git_transport_uninitialized();
}

// A refresh that could not read all of the tickets' directory may have
// missed tickets, and then every answer made from the tickets may lack them.
#[test]
fn a_relationship_read_is_not_complete_when_the_refresh_could_not_read_every_ticket() {
    let (fixture, enabled) = relationship_repository();
    let service = &enabled.service;
    let repo = service.resolve_repository(&fixture.root).unwrap();
    let all = TicketFilter::default();
    let complete = || {
        [
            service.ticket_readiness(&repo, &all).unwrap().complete,
            service
                .ticket_dependencies(&repo, &item_id(RELATED_C), DependencyDirection::Both, None)
                .unwrap()
                .complete,
            service
                .ticket_children(&repo, &item_id(RELATED_B))
                .unwrap()
                .complete,
            service.ticket_cycles(&repo).unwrap().complete,
            service.ticket_plan(&repo, &all).unwrap().complete,
            service.ticket_critical_path(&repo).unwrap().complete,
            service
                .find_tickets_by_slug(&repo, SHARED_SLUG)
                .unwrap()
                .complete,
        ]
    };
    let insert = |path: &str, code: &str| {
        index(enabled.data_directory.path())
            .execute(
                "INSERT INTO problems
                     (repository_id, context_id, path, code, guidance, observed_at)
                 SELECT repository_id, id, ?1, ?2, 'unused', 0
                   FROM contexts WHERE kind = 'primary'",
                [path, code],
            )
            .unwrap();
    };
    assert_eq!(complete(), [true; 7]);

    // The documents' directory, one ticket's file, and another problem at
    // the tickets' directory say nothing about which tickets were seen.
    insert("docs", "source");
    insert(&ticket_path(TICKET_ABSENT), "source");
    insert(".manyhands/tickets", "context");
    assert_eq!(complete(), [true; 7]);

    for path in [".manyhands/tickets", ".manyhands"] {
        index(enabled.data_directory.path())
            .execute("DELETE FROM problems WHERE code = 'source'", [])
            .unwrap();
        insert(path, "source");
        assert_eq!(complete(), [false; 7], "{path}");
    }
    // The answers themselves are made from what the index holds.
    assert_eq!(
        sorted_ids(&service.find_tickets_by_slug(&repo, SHARED_SLUG).unwrap()),
        [RELATED_B, RELATED_C]
    );
    assert_eq!(
        service.ticket_cycles(&repo).unwrap().index.state,
        IndexState::Current
    );
    assert_git_transport_uninitialized();
}
