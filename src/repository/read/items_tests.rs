use super::*;

fn digest(input: &[u8]) -> String {
    format!("v1:{}", blake3::hash(input).to_hex())
}

fn length(bytes: &[u8]) -> [u8; 8] {
    (bytes.len() as u64).to_le_bytes()
}

#[test]
fn observation_token_is_the_digest_of_the_documented_bytes() {
    let (branch, path, source) = ("main", "docs/a.md", b"---\nid: x\n---\nbody\n".as_slice());
    let mut with_branch = vec![1];
    with_branch.extend(length(branch.as_bytes()));
    with_branch.extend(branch.as_bytes());
    let mut rest = Vec::new();
    rest.extend(length(path.as_bytes()));
    rest.extend(path.as_bytes());
    rest.extend(length(source));
    rest.extend(source);
    let mut without_branch = vec![0];
    with_branch.extend(&rest);
    without_branch.extend(&rest);

    let token = observation_token(Some(branch), path, source);

    assert_eq!(token, digest(&with_branch));
    assert_eq!(
        observation_token(None, path, source),
        digest(&without_branch)
    );
    assert_eq!(token.len(), 3 + 64);
    assert_eq!(token, token.to_lowercase());
    // Pinned: this value is part of what a caller stores and sends back.
    assert_eq!(
        observation_token(Some("main"), "docs/a.md", b"source"),
        "v1:8d41b412a774defa1e4bfb8c7a4512f2a1d907c0303c238e2eb81856ce10f50c"
    );
}

#[test]
fn observation_token_changes_with_the_branch_the_path_and_the_source() {
    let token = observation_token(Some("main"), "docs/a.md", b"source");

    assert_eq!(
        token,
        observation_token(Some("main"), "docs/a.md", b"source")
    );
    for other in [
        observation_token(Some("other"), "docs/a.md", b"source"),
        observation_token(None, "docs/a.md", b"source"),
        observation_token(Some(""), "docs/a.md", b"source"),
        observation_token(Some("main"), "docs/b.md", b"source"),
        observation_token(Some("main"), "docs/a.md", b"source "),
        observation_token(Some("main"), "docs/a.md", b""),
        // The same bytes, divided differently between the parts.
        observation_token(Some("maind"), "ocs/a.md", b"source"),
        observation_token(Some("main"), "docs/a.mds", b"ource"),
        observation_token(Some("main"), "docs/a.mdsource", b""),
    ] {
        assert_ne!(other, token);
    }
}

#[test]
fn only_document_and_ticket_paths_name_an_item_file() {
    let id = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    let kind = canonical_item_kind;

    assert_eq!(kind("docs/a.md"), Some(ItemDtoKind::Document));
    assert_eq!(kind("docs/a/b/c.md"), Some(ItemDtoKind::Document));
    assert_eq!(
        kind(&format!(".manyhands/tickets/{id}/ticket.md")),
        Some(ItemDtoKind::Ticket)
    );
    for path in [
        "",
        "/docs/a.md",
        "../docs/a.md",
        "docs/../a.md",
        "docs/./a.md",
        "docs//a.md",
        "docs/a.md/",
        "docs",
        "docs/a.txt",
        "a.md",
        ".manyhands/config.toml",
        ".manyhands/tickets/ticket.md",
        ".manyhands/tickets/x/ticket.md",
        &format!(".manyhands/tickets/{id}/other.md"),
        &format!(".manyhands/tickets/{id}/ticket.md/x"),
        &format!(".manyhands/comments/{id}/{id}.md"),
        &format!(".manyhands/worktrees/{id}/docs/a.md"),
        "docs\\a.md",
    ] {
        assert_eq!(kind(path), None, "{path:?}");
    }
}

fn context(worktree: &str) -> StoredContext {
    StoredContext {
        kind: ItemContextKind::Primary,
        branch: Some("main".to_owned()),
        worktree: worktree.to_owned(),
        head_oid: None,
    }
}

fn index() -> IndexStateDto {
    index_state(false, Some(0), true)
}

#[test]
fn the_index_state_is_never_refreshed_only_when_nothing_was_ever_stored() {
    for (refresh_required, refreshed_at, has_contexts, state) in [
        (true, None, false, IndexState::NeverRefreshed),
        (false, None, false, IndexState::NeverRefreshed),
        // Written before refresh times were recorded: rows, and no time.
        (true, None, true, IndexState::Stale),
        (false, None, true, IndexState::Stale),
        (true, Some(0), true, IndexState::Stale),
        (false, Some(0), true, IndexState::Current),
        (true, Some(0), false, IndexState::Stale),
        (false, Some(0), false, IndexState::Current),
    ] {
        let index = index_state(refresh_required, refreshed_at, has_contexts);
        assert_eq!(
            index.state, state,
            "{refresh_required} {refreshed_at:?} {has_contexts}"
        );
        assert_eq!(index.refreshed_at.is_some(), refreshed_at.is_some());
    }
    assert_eq!(
        index_state(false, Some(1_700_000_000), true)
            .refreshed_at
            .as_deref(),
        Some("2023-11-14T22:13:20Z")
    );
    // Only a current index becomes stale; what is worse stays what it is.
    assert_eq!(
        behind(&index_state(false, Some(0), true)).state,
        IndexState::Stale
    );
    assert_eq!(
        behind(&index_state(true, Some(0), true)).state,
        IndexState::Stale
    );
    assert_eq!(
        behind(&index_state(true, None, false)).state,
        IndexState::NeverRefreshed
    );
}

#[test]
fn a_plain_relative_path_has_only_normal_components_outside_the_worktrees() {
    for path in [
        "docs/a.md",
        "a",
        ".manyhands/tickets/not-an-id/ticket.md",
        ".manyhands/worktreesx/a",
        ".git/config",
    ] {
        assert!(is_plain_relative(path), "{path:?}");
    }
    for path in [
        "",
        "/",
        "/docs/a.md",
        "docs//a.md",
        "docs/a.md/",
        "./docs/a.md",
        "docs/./a.md",
        "..",
        "../a.md",
        "docs/../a.md",
        "docs/..",
        "docs\\a.md",
        "docs/a\0b.md",
        ".manyhands/worktrees",
        ".manyhands/worktrees/x/docs/a.md",
    ] {
        assert!(!is_plain_relative(path), "{path:?}");
    }
}

#[test]
fn a_file_that_is_not_text_or_not_an_item_keeps_only_its_problem_code() {
    let sentinel = "SENTINEL-2b6f";
    let parse = |bytes: &[u8]| {
        parse_file(
            bytes.to_vec(),
            "docs/a.md",
            &context("/r"),
            &index(),
            &Targets::of(&[], &[]),
        )
    };

    let ParsedFile::Nonconforming { code, source } = parse(&[0xff, 0xfe]) else {
        panic!("bytes that are not UTF-8 are not an item");
    };
    assert_eq!((code, source), (ProblemCode::SourceUnreadable, None));

    let text = format!("---\n{sentinel}: 1\n{sentinel}: 2\n---\n");
    let ParsedFile::Nonconforming { code, source } = parse(text.as_bytes()) else {
        panic!("a key given twice is not an item");
    };
    assert_eq!(code, ProblemCode::MalformedFrontMatter);
    assert_eq!(source.as_deref(), Some(text.as_str()));
    // The parser's message does repeat the key, which is why it is dropped.
    let message = canonical::parse_item(Path::new("docs/a.md"), &text)
        .unwrap_err()
        .message;
    assert!(message.contains(sentinel), "{message}");
}

#[test]
fn nonconforming_entries_need_a_conformity_code_at_an_item_path_with_no_item() {
    let item = |worktree: &str, path: &str| StoredItem {
        row_id: 0,
        context: context(worktree),
        id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned(),
        kind: ItemDtoKind::Document,
        path: path.to_owned(),
        title: "T".to_owned(),
        ticket_type: None,
        status: None,
        project: None,
        team: None,
        closed_at: None,
        closed_by: None,
        unknown: UnknownMetadata::default(),
        relationships: Relationships::default(),
        activity_at: 0,
        change_source: ChangeSource::GitCommit,
    };
    let stored_problem = |worktree: &str, path: &str, code| StoredProblem {
        context: context(worktree),
        path: path.to_owned(),
        code,
    };
    let items = [item("/r", "docs/listed.md")];
    let problems = [
        stored_problem("/r", "docs/a.md", ProblemCode::MissingField),
        stored_problem("/r", "docs/a.md", ProblemCode::InvalidField),
        // Stored twice, listed once.
        stored_problem("/r", "docs/a.md", ProblemCode::MissingField),
        stored_problem("/r", "docs/listed.md", ProblemCode::DuplicateId),
        stored_problem("/r", "docs/source.md", ProblemCode::SourceUnreadable),
        stored_problem("/r", "docs/unknown.md", ProblemCode::UnknownProblem),
        stored_problem("/r", "notes/a.md", ProblemCode::InvalidPath),
        stored_problem(
            "/r",
            ".manyhands/tickets/x/ticket.md",
            ProblemCode::InvalidPath,
        ),
        // The same path in another context is another file.
        stored_problem("/w", "docs/a.md", ProblemCode::MissingField),
        stored_problem("/w", "docs/listed.md", ProblemCode::MissingField),
    ];
    let entries = |kind| {
        nonconforming_entries(kind, &items, &problems, &index())
            .into_iter()
            .map(|entry| {
                (
                    entry.context.worktree.clone(),
                    entry.path.clone(),
                    entry.problems.iter().map(|problem| problem.code).collect(),
                )
            })
            .collect::<Vec<(String, String, Vec<ProblemCode>)>>()
    };
    let entry = |worktree: &str, path: &str, codes: &[ProblemCode]| {
        (worktree.to_owned(), path.to_owned(), codes.to_vec())
    };

    assert_eq!(
        entries(ItemDtoKind::Document),
        [
            entry(
                "/r",
                "docs/a.md",
                &[ProblemCode::MissingField, ProblemCode::InvalidField]
            ),
            entry("/w", "docs/a.md", &[ProblemCode::MissingField]),
            entry("/w", "docs/listed.md", &[ProblemCode::MissingField]),
        ]
    );
    assert_eq!(
        entries(ItemDtoKind::Ticket),
        [entry(
            "/r",
            ".manyhands/tickets/x/ticket.md",
            &[ProblemCode::InvalidPath]
        )]
    );
    for entry in nonconforming_entries(ItemDtoKind::Document, &items, &problems, &index()) {
        assert_eq!(entry.id, None);
        assert_eq!(entry.kind, ItemDtoKind::Document);
        assert!(
            entry
                .problems
                .iter()
                .all(|problem| problem.path.as_deref() == Some(&entry.path))
        );
    }
}

#[test]
fn ticket_filters_compare_whole_values_and_closure_reads_only_closed_at() {
    let ticket = |status: &str, closed_at| StoredItem {
        row_id: 0,
        context: context("/r"),
        id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_owned(),
        kind: ItemDtoKind::Ticket,
        path: "p".to_owned(),
        title: "T".to_owned(),
        ticket_type: Some("task".to_owned()),
        status: Some(status.to_owned()),
        project: None,
        team: None,
        closed_at,
        closed_by: None,
        unknown: UnknownMetadata::default(),
        relationships: Relationships::default(),
        activity_at: 0,
        change_source: ChangeSource::GitCommit,
    };
    let text = |value: &str| Some(value.to_owned());
    let closure = |closure| TicketFilter {
        closure,
        ..Default::default()
    };
    let says_closed = ticket("closed", None);
    let is_closed = ticket("open", Some(1));

    assert!(closure(ClosureFilter::Open).matches(&says_closed));
    assert!(!closure(ClosureFilter::Closed).matches(&says_closed));
    assert!(closure(ClosureFilter::Closed).matches(&is_closed));
    assert!(!closure(ClosureFilter::Open).matches(&is_closed));
    assert!(closure(ClosureFilter::All).matches(&says_closed));
    assert!(closure(ClosureFilter::All).matches(&is_closed));
    // A filter on a field the ticket does not have matches nothing.
    let by_project = TicketFilter {
        project: text("alpha"),
        ..Default::default()
    };
    assert!(!by_project.matches(&is_closed));
    for (status, matches) in [("open", true), ("Open", false), ("ope", false), ("", false)] {
        let filter = TicketFilter {
            status: text(status),
            ..Default::default()
        };
        assert_eq!(filter.matches(&is_closed), matches, "{status:?}");
    }
}

const SELF: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA0";
const OPEN: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA1";
const CLOSED: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA2";
const DOCUMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA3";
const ABSENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA4";

fn stored(id: &str, kind: ItemDtoKind, closed_at: Option<i64>) -> StoredItem {
    StoredItem {
        row_id: 0,
        context: context("/r"),
        id: id.to_owned(),
        kind,
        path: "p".to_owned(),
        title: "T".to_owned(),
        ticket_type: None,
        status: None,
        project: None,
        team: None,
        closed_at,
        closed_by: None,
        unknown: UnknownMetadata::default(),
        relationships: Relationships::default(),
        activity_at: 0,
        change_source: ChangeSource::GitCommit,
    }
}

const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FA5";

fn about(code: ProblemCode, target_id: Option<&str>) -> ProblemDto {
    ProblemDto {
        target_id: target_id.map(str::to_owned),
        ..problem(code, "p")
    }
}

#[test]
fn a_target_says_what_it_is_and_a_document_or_comment_is_not_a_ticket() {
    let rows = [
        // A status that says closed closes nothing.
        StoredItem {
            status: Some("closed".to_owned()),
            ..stored(OPEN, ItemDtoKind::Ticket, None)
        },
        stored(CLOSED, ItemDtoKind::Ticket, Some(1)),
        stored(DOCUMENT, ItemDtoKind::Document, None),
    ];
    // An ID that is both an item's and a comment's is the item's.
    let comments = [COMMENT.to_owned(), CLOSED.to_owned()];
    let targets = Targets::of(&rows.iter().collect::<Vec<_>>(), &comments);
    let dependency = |id: &str, state| DependencyDto {
        id: id.to_owned(),
        state,
    };
    let ticket = StoredItem {
        relationships: Relationships {
            slug: Some("mh-vh-k9x2b".to_owned()),
            parent: Some(ABSENT.to_owned()),
            deps: [ABSENT, DOCUMENT, CLOSED, COMMENT, OPEN]
                .map(str::to_owned)
                .to_vec(),
            problems: vec![
                (ProblemCode::RelationshipWrongType, None),
                (ProblemCode::InvalidSlug, None),
                (ProblemCode::RelationshipWrongType, None),
                (ProblemCode::DuplicateDependency, Some(OPEN.to_owned())),
                (ProblemCode::DuplicateDependency, Some(CLOSED.to_owned())),
                (ProblemCode::DuplicateDependency, Some(OPEN.to_owned())),
            ],
        },
        unknown: UnknownMetadata {
            not_representable: true,
            ..Default::default()
        },
        ..stored(SELF, ItemDtoKind::Ticket, None)
    };

    let dto = stored_item_dto(&ticket, &targets, &index());

    assert_eq!(dto.slug.as_deref(), Some("mh-vh-k9x2b"));
    // A parent no context holds is still the parent, and says so.
    assert_eq!(
        dto.parent,
        Some(dependency(ABSENT, DependencyState::Unresolved))
    );
    // In the file's order, without the document and the comment.
    assert_eq!(
        dto.deps,
        [
            dependency(ABSENT, DependencyState::Unresolved),
            dependency(CLOSED, DependencyState::Closed),
            dependency(OPEN, DependencyState::Open),
        ]
    );
    assert_eq!(dto.readiness, None);
    // The item's own problems first. Problems alike in every way are one;
    // problems about different IDs are not.
    assert_eq!(
        dto.problems,
        [
            about(ProblemCode::MetadataNotRepresentable, None),
            about(ProblemCode::RelationshipWrongType, None),
            about(ProblemCode::InvalidSlug, None),
            about(ProblemCode::DuplicateDependency, Some(OPEN)),
            about(ProblemCode::DuplicateDependency, Some(CLOSED)),
            about(ProblemCode::RelationshipNotATicket, Some(DOCUMENT)),
            about(ProblemCode::RelationshipNotATicket, Some(COMMENT)),
        ]
    );

    for (parent, state) in [
        (OPEN, Some(DependencyState::Open)),
        (CLOSED, Some(DependencyState::Closed)),
        (DOCUMENT, None),
        (COMMENT, None),
    ] {
        let child = StoredItem {
            relationships: Relationships {
                parent: Some(parent.to_owned()),
                ..Default::default()
            },
            ..stored(SELF, ItemDtoKind::Ticket, None)
        };
        let dto = stored_item_dto(&child, &targets, &index());
        assert_eq!(dto.parent, state.map(|state| dependency(parent, state)));
        let expected = match state {
            Some(_) => Vec::new(),
            None => vec![about(ProblemCode::RelationshipNotATicket, Some(parent))],
        };
        assert_eq!(dto.problems, expected, "{parent}");
    }
}

#[test]
fn the_slug_filter_matches_the_whole_short_code_without_regard_to_case() {
    let with_slug = StoredItem {
        relationships: Relationships {
            slug: Some("mh-vh-k9x2b".to_owned()),
            ..Default::default()
        },
        ..stored(SELF, ItemDtoKind::Ticket, None)
    };
    let without = stored(OPEN, ItemDtoKind::Ticket, None);
    let by_slug = |slug: &str| TicketFilter {
        slug: Some(slug.to_owned()),
        ..Default::default()
    };

    for (slug, matches) in [
        ("mh-vh-k9x2b", true),
        ("MH-VH-K9X2B", true),
        ("Mh-vH-k9X2b", true),
        ("vh-k9x2b", false),
        ("mh-vh-k9x2", false),
        ("k9x2b", false),
        ("", false),
    ] {
        assert_eq!(by_slug(slug).matches(&with_slug), matches, "{slug:?}");
        assert!(!by_slug(slug).matches(&without), "{slug:?}");
    }
    assert!(TicketFilter::default().matches(&without));
}

#[test]
fn each_relationship_problem_is_stored_under_its_registry_string() {
    let codes = canonical::RelationshipProblemCode::ALL.map(|code| {
        let registered = ProblemCode::from(code);
        assert_eq!(
            registered.stored(),
            Some(crate::repository::relationship_problem_code_name(code)),
            "{code:?}"
        );
        registered
    });
    assert_eq!(codes, RELATIONSHIP_PROBLEMS);
}
