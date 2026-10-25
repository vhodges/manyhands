use super::*;
use manyhands::{
    repository::{
        IndexState, IndexStateDto, ItemContextDto, ItemContextKind, RelationshipRejectionDto,
    },
    results::ResultCode,
};

#[test]
fn path_fields_escape_controls_without_escaping_layout() {
    let output = path_observation(&PathObservationDto {
        path: "docs/a\u{1b}[2J\nb.md".into(),
        exists: false,
        observation: "token\nvalue".into(),
        context: ItemContextDto {
            kind: ItemContextKind::Primary,
            branch: Some("branch\nforged".into()),
            worktree: "root\tpath".into(),
            head_oid: None,
        },
    });
    assert_eq!(
        output.stdout,
        "path: docs/a\\u{1b}[2J\\nb.md\nexists: no\nobservation: token\\nvalue\ncontext: root\\tpath (branch\\nforged)\n"
    );
    assert!(output.notes.is_empty());
}

fn proposal() -> RelationshipCheckDto {
    RelationshipCheckDto {
        deps: vec![],
        parent: None,
        unresolved: vec![],
        rejection: None,
        complete: true,
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

#[test]
fn rejection_is_a_final_escaped_line() {
    let mut data = proposal();
    data.deps = vec!["dep\nvalue".into()];
    data.parent = Some("parent\tvalue".into());
    data.unresolved = vec!["unknown\rvalue".into()];
    data.rejection = Some(RelationshipRejectionDto {
        code: ResultCode::RelationshipCycle,
        ids: vec!["id\nvalue".into(), "second".into()],
    });
    let output = relationship_check(&data);
    assert_eq!(
        output.stdout,
        "deps: dep\\nvalue\nparent: parent\\tvalue\nunresolved: unknown\\rvalue\nrejected: relationship_cycle: id\\nvalue second\n"
    );
    assert!(output.notes.is_empty());
}

#[test]
fn empty_unresolved_is_explicit_and_has_no_rejection_line() {
    let output = relationship_check(&proposal());
    assert_eq!(output.stdout, "deps: -\nparent: -\nunresolved: -\n");
    assert!(output.notes.is_empty());
}

#[test]
fn incomplete_and_noncurrent_checks_use_c1_notes() {
    for (state, note) in [
        (IndexState::Stale, "Index is stale."),
        (IndexState::NeverRefreshed, "Index is never_refreshed."),
    ] {
        let mut data = proposal();
        data.complete = false;
        data.index.state = state;
        assert_eq!(
            relationship_check(&data).notes,
            ["The list is incomplete.", note]
        );
    }
}
