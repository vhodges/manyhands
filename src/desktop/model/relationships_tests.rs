//! A ticket's relationships as rows, over real library reads: each named
//! only from the library's rows, an unresolved target with no way to open
//! it, the relationship problems, and reads from before a reload dropped.

use std::path::{Path, PathBuf};

use manyhands::{
    repository::{DependencyDirection, DependencyTreeDto, ItemDto, ItemDtoKind, ItemListDto},
    results::{ProblemCode, ResultCode},
};

use super::{
    RELATIONSHIP_PROBLEMS, RESOLVED, RelatedRow, Relationships, UNRESOLVED, resolution_label,
};
use crate::desktop::{
    model::{
        item::{ItemReads, ItemView},
        list_state::ListState,
        navigation::{NotReloaded, OpenTarget},
        stamp::{TabId, ViewScope},
        tickets::Lifecycle,
    },
    services::{ReadFailure, ReadResult},
    test_support::fixtures::{Fixture, item_id},
};

const MAIN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const DEP_OPEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD1";
const DEP_CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD2";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD3";
const DEPENDENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD4";
const CHILD_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD5";
const CHILD_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD6";
const MISSING: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD9";

struct Reads {
    _fixture: Fixture,
    root: PathBuf,
    main: ItemDto,
    tree: DependencyTreeDto,
    children: ItemListDto,
    parent: Option<ItemDto>,
}

/// The four reads the panel makes of `MAIN`, after `build` and a refresh.
fn reads(build: impl FnOnce(&Fixture, &Path)) -> Reads {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    build(&fixture, &root);
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    let id = item_id(MAIN);
    let main = service.show_item(&repo, &id).unwrap();
    let tree = service
        .ticket_dependencies(&repo, &id, DependencyDirection::Both, Some(1))
        .unwrap();
    let children = service.ticket_children(&repo, &id).unwrap();
    let parent = main
        .parent
        .as_ref()
        .and_then(|parent| service.show_item(&repo, &item_id(&parent.id)).ok());
    Reads {
        _fixture: fixture,
        root,
        main,
        tree,
        children,
        parent,
    }
}

/// Two dependencies, one open and one closed, a parent, one dependent and
/// two children of `MAIN`, each made through the library but the closed
/// one, which no local operation makes.
fn graph(fixture: &Fixture, root: &Path) {
    fixture.create_ticket(root, DEP_OPEN, "Open dependency", "open");
    fixture.commit_closed_ticket(root, DEP_CLOSED, "Closed dependency");
    fixture.create_ticket(root, PARENT, "The parent", "open");
    fixture.create_related_ticket(
        root,
        MAIN,
        "Main",
        "open",
        &[DEP_OPEN, DEP_CLOSED],
        Some(PARENT),
    );
    fixture.create_ticket_with_deps(root, DEPENDENT, "The dependent", "open", &[MAIN]);
    fixture.create_related_ticket(root, CHILD_A, "Child A", "open", &[], Some(MAIN));
    fixture.create_related_ticket(root, CHILD_B, "Child B", "open", &[], Some(MAIN));
}

fn loaded(reads: &Reads) -> Relationships {
    let mut relationships = Relationships::default();
    relationships.set_item(&reads.main);
    relationships.apply_dependents(ReadResult::Ok(reads.tree.clone()));
    relationships.apply_children(ReadResult::Ok(reads.children.clone()));
    if let Some(parent) = reads.parent.clone() {
        relationships.apply_parent(ReadResult::Ok(parent));
    }
    relationships
}

fn repo(reads: &Reads) -> String {
    reads.root.to_str().unwrap().to_owned()
}

fn ticket(reads: &Reads, id: &str) -> OpenTarget {
    OpenTarget::Item {
        repo: repo(reads),
        kind: ItemDtoKind::Ticket,
        id: item_id(id),
    }
}

/// The short code the library gave `id`.
fn slug(reads: &Reads, id: &str) -> Option<String> {
    let service = reads._fixture.service();
    let repo = service.resolve_repository(&reads.root).unwrap();
    service.show_item(&repo, &item_id(id)).unwrap().slug
}

fn ids(rows: &[RelatedRow]) -> Vec<&str> {
    rows.iter().map(|row| row.id.as_str()).collect()
}

/// Every group, each row with the short code and title the library gave
/// it, its lifecycle and its resolution, and each opening its ticket by
/// ID in the same repository.
#[test]
fn each_group_names_its_tickets_from_the_librarys_rows() {
    let reads = reads(graph);
    let relationships = loaded(&reads);

    let dependencies = relationships.dependency_rows();
    // In the ticket's own order.
    assert_eq!(ids(&dependencies), [DEP_OPEN, DEP_CLOSED]);
    assert_eq!(dependencies[0].title.as_deref(), Some("Open dependency"));
    assert_eq!(dependencies[0].slug, slug(&reads, DEP_OPEN));
    assert!(
        dependencies[0].slug.is_some(),
        "create derives a short code"
    );
    assert_eq!(dependencies[0].lifecycle, Some(Lifecycle::Open));
    assert_eq!(dependencies[1].title.as_deref(), Some("Closed dependency"));
    assert_eq!(dependencies[1].lifecycle, Some(Lifecycle::Closed));
    assert!(dependencies.iter().all(|row| row.resolved));

    let dependents = relationships.dependent_rows();
    assert_eq!(ids(&dependents), [DEPENDENT]);
    assert_eq!(dependents[0].title.as_deref(), Some("The dependent"));
    assert_eq!(dependents[0].slug, slug(&reads, DEPENDENT));
    assert_eq!(dependents[0].lifecycle, Some(Lifecycle::Open));

    let parent = relationships.parent_row().unwrap();
    assert_eq!(parent.id, PARENT);
    assert_eq!(parent.title.as_deref(), Some("The parent"));
    assert_eq!(parent.slug, slug(&reads, PARENT));
    assert_eq!(parent.lifecycle, Some(Lifecycle::Open));
    assert!(parent.resolved);

    let children = relationships.child_rows();
    let mut child_ids = ids(&children);
    child_ids.sort();
    assert_eq!(child_ids, [CHILD_A, CHILD_B]);
    assert!(children.iter().all(|row| row.title.is_some()));

    for row in dependencies
        .iter()
        .chain(&dependents)
        .chain(Some(&parent))
        .chain(&children)
    {
        assert_eq!(row.target(&repo(&reads)), Some(ticket(&reads, &row.id)));
        assert_eq!(resolution_label(row.resolved), RESOLVED);
    }
    assert!(relationships.problems().is_empty());
}

/// Until the tree lands, a dependency is shown by its ID, its lifecycle
/// and its resolution, and no title is made up; the parent likewise until
/// its own read lands.
#[test]
fn before_the_reads_land_a_related_ticket_is_shown_by_its_id_only() {
    let reads = reads(graph);
    let mut relationships = Relationships::default();
    relationships.set_item(&reads.main);
    assert!(matches!(relationships.dependents(), ListState::Loading));
    assert!(matches!(relationships.children(), ListState::Loading));
    let dependencies = relationships.dependency_rows();
    assert_eq!(ids(&dependencies), [DEP_OPEN, DEP_CLOSED]);
    assert!(
        dependencies
            .iter()
            .all(|row| row.title.is_none() && row.slug.is_none())
    );
    assert_eq!(dependencies[1].lifecycle, Some(Lifecycle::Closed));
    assert!(relationships.dependent_rows().is_empty());
    assert!(relationships.child_rows().is_empty());
    let parent = relationships.parent_row().unwrap();
    assert_eq!(parent.id, PARENT);
    assert_eq!(parent.title, None);
    assert_eq!(relationships.parent_to_read(), Some(item_id(PARENT)));

    // A parent read that gave another ticket names nothing.
    relationships.apply_parent(ReadResult::Ok(reads.main.clone()));
    assert_eq!(relationships.parent_row().unwrap().title, None);
}

/// An unresolved dependency or parent shows the ID it names, has no
/// lifecycle and nothing to open, and its parent is not read.
#[test]
fn an_unresolved_target_shows_its_id_and_cannot_be_opened() {
    let reads = reads(|fixture, root| {
        fixture.commit_ticket(
            root,
            MAIN,
            "Main",
            &format!("deps:\n  - {MISSING}\nparent: {MISSING}\n"),
        );
    });
    let relationships = loaded(&reads);
    let rows = relationships.dependency_rows();
    assert_eq!(ids(&rows), [MISSING]);
    let row = &rows[0];
    assert!(!row.resolved);
    assert_eq!((row.slug.as_deref(), row.title.as_deref()), (None, None));
    assert_eq!(row.lifecycle, None);
    assert_eq!(row.target(&repo(&reads)), None);
    assert_eq!(resolution_label(row.resolved), UNRESOLVED);

    let parent = relationships.parent_row().unwrap();
    assert_eq!(parent.id, MISSING);
    assert!(!parent.resolved);
    assert_eq!(parent.target(&repo(&reads)), None);
    assert_eq!(relationships.parent_to_read(), None);
}

/// Only the problems with a ticket's `deps` and `parent` are listed under
/// the groups, in the item's order.
#[test]
fn the_relationship_problems_are_listed_and_no_other() {
    assert_eq!(
        RELATIONSHIP_PROBLEMS,
        [
            ProblemCode::RelationshipWrongType,
            ProblemCode::RelationshipInvalidId,
            ProblemCode::RelationshipSelfReference,
            ProblemCode::DuplicateDependency,
            ProblemCode::RelationshipNotATicket,
            ProblemCode::DependencyCycle,
            ProblemCode::ParentCycle,
        ]
    );
    let reads = reads(|fixture, root| {
        fixture.commit_ticket(
            root,
            MAIN,
            "Main",
            &format!("slug: Not-A-Slug\ndeps:\n  - {MAIN}\n"),
        );
    });
    let relationships = loaded(&reads);
    let codes: Vec<ProblemCode> = relationships
        .problems()
        .iter()
        .map(|problem| problem.code)
        .collect();
    assert_eq!(codes, [ProblemCode::RelationshipSelfReference]);
    assert!(
        reads
            .main
            .problems
            .iter()
            .any(|problem| problem.code == ProblemCode::InvalidSlug)
    );
    // The ignored entry is no dependency.
    assert!(relationships.dependency_rows().is_empty());
}

fn target(kind: ItemDtoKind) -> OpenTarget {
    OpenTarget::Item {
        repo: "/work/a".to_owned(),
        kind,
        id: item_id(MAIN),
    }
}

/// A ticket's view has relationships; a document's and a nonconforming
/// entry's have none.
#[test]
fn only_a_ticket_has_relationships() {
    let mut view = ItemView::new(target(ItemDtoKind::Ticket), ViewScope::Tab(TabId(0)));
    view.load();
    assert!(view.relationships().is_some());
    let mut document = ItemView::new(target(ItemDtoKind::Document), ViewScope::Tab(TabId(1)));
    document.load();
    assert!(document.relationships().is_none());
    let inspect = ItemView::new(
        OpenTarget::Inspect {
            repo: "/work/a".to_owned(),
            worktree: "/work/a".to_owned(),
            path: ".manyhands/tickets/x/ticket.md".to_owned(),
        },
        ViewScope::Tab(TabId(2)),
    );
    assert!(inspect.relationships().is_none());
}

/// Review Focus 1: the relationship reads are stamped to the tab. A load
/// from before a reload is dropped; a reload keeps what is shown until its
/// load lands, and one whose relationship reads give nothing keeps their
/// rows and the parent's name, and says why. A first load starts again
/// from loading.
#[test]
fn a_relationship_read_from_before_a_reload_is_dropped() {
    let reads = reads(graph);
    let mut view = ItemView::new(ticket(&reads, MAIN), ViewScope::Tab(TabId(4)));
    let first = view.load();
    assert!(view.apply_reads(
        first.clone(),
        ItemReads {
            item: ReadResult::Ok(reads.main.clone()),
            comments: None,
            dependents: Some(ReadResult::Ok(reads.tree.clone())),
            children: Some(ReadResult::Ok(reads.children.clone())),
            parent: Some(ReadResult::Ok(reads.parent.clone().unwrap())),
        },
    ));
    assert_eq!(view.parent_to_read(), Some(item_id(PARENT)));

    let second = view.reload();
    assert_eq!(second.scope, ViewScope::Tab(TabId(4)));
    let relationships = view.relationships().unwrap();
    assert_eq!(ids(&relationships.dependent_rows()), [DEPENDENT]);
    assert_eq!(relationships.child_rows().len(), 2);
    assert_eq!(
        relationships.parent_row().unwrap().title.as_deref(),
        Some("The parent")
    );

    let busy = || ItemReads {
        item: ReadResult::Busy,
        comments: None,
        dependents: Some(ReadResult::Busy),
        children: Some(ReadResult::Busy),
        parent: None,
    };
    assert!(!view.apply_reads(first, busy()));
    assert!(view.not_refreshed().is_none());

    let failure = ReadFailure {
        code: ResultCode::ItemNotFound,
        recovery: Vec::new(),
    };
    assert!(view.apply_reads(
        second,
        ItemReads {
            item: ReadResult::Ok(reads.main.clone()),
            comments: None,
            dependents: Some(ReadResult::Failed(failure)),
            children: Some(ReadResult::Busy),
            parent: Some(ReadResult::Busy),
        },
    ));
    let relationships = view.relationships().unwrap();
    assert_eq!(ids(&relationships.dependent_rows()), [DEPENDENT]);
    assert_eq!(relationships.child_rows().len(), 2);
    assert_eq!(
        relationships.parent_row().unwrap().title.as_deref(),
        Some("The parent"),
        "a busy parent read keeps the name while the ID is the same"
    );
    assert!(matches!(
        view.not_refreshed(),
        Some(NotReloaded::Failed(why)) if why.code == ResultCode::ItemNotFound
    ));

    view.load();
    let relationships = view.relationships().unwrap();
    assert!(matches!(relationships.dependents(), ListState::Loading));
    assert!(matches!(relationships.children(), ListState::Loading));
    assert!(relationships.parent_row().is_none());
    assert!(view.not_refreshed().is_none());
}
