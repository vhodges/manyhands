use super::*;
use manyhands::repository::{IndexState, IndexStateDto, ItemContextDto, ItemContextKind};

fn item() -> ItemDto {
    ItemDto {
        id: Some("01ARZ3NDEKTSV4RRFFQ69G5FDN".into()),
        kind: ItemDtoKind::Document,
        path: "docs/test.md".into(),
        title: Some("Current title".into()),
        ticket_type: None,
        status: None,
        project: None,
        team: None,
        closure: None,
        slug: None,
        parent: None,
        deps: vec![],
        readiness: None,
        unknown_metadata: Default::default(),
        body: Some("line one\r\nline two\rthree 🦀\nno final newline".into()),
        source: None,
        observation: Some("opaque-token".into()),
        context: ItemContextDto {
            kind: ItemContextKind::Primary,
            branch: Some("main".into()),
            worktree: "root".into(),
            head_oid: None,
        },
        changed_at: None,
        change_source: None,
        problems: vec![],
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

#[test]
fn all_fields_given_need_no_read() {
    let fields = DocumentFields {
        title: Some("Given title".into()),
        body: Some("Given body".into()),
    };
    assert!(!needs_read_document(&fields));
    let draft = document_draft(fields, None).ok().unwrap();
    assert_eq!(draft.title, "Given title");
    assert_eq!(draft.body, "Given body");
}

#[test]
fn omitted_title_comes_from_the_read() {
    let current = item();
    let fields = DocumentFields {
        title: None,
        body: Some("Given body".into()),
    };
    assert!(needs_read_document(&fields));
    let draft = document_draft(fields, Some(&current)).ok().unwrap();
    assert_eq!(draft.title, current.title.unwrap());
    assert_eq!(draft.body, "Given body");
}

#[test]
fn omitted_body_is_preserved_byte_for_byte_including_empty() {
    for body in [item().body.unwrap(), String::new()] {
        let mut current = item();
        current.body = Some(body.clone());
        let fields = DocumentFields {
            title: Some("Given title".into()),
            body: None,
        };
        assert!(needs_read_document(&fields));
        let draft = document_draft(fields, Some(&current)).ok().unwrap();
        assert_eq!(draft.title, "Given title");
        assert_eq!(draft.body.as_bytes(), body.as_bytes());
    }
}

#[test]
fn wrong_kind_and_missing_required_read_fields_are_unfillable() {
    let mut ticket = item();
    ticket.kind = ItemDtoKind::Ticket;
    let mut no_title = item();
    no_title.title = None;
    let mut no_body = item();
    no_body.body = None;
    for current in [Some(&ticket), Some(&no_title), Some(&no_body), None] {
        let fields = DocumentFields {
            title: None,
            body: None,
        };
        assert!(needs_read_document(&fields));
        assert!(matches!(
            document_draft(fields, current),
            Err(FillError::Unfillable)
        ));
    }
    // A supplied value must not turn a read of the other kind into a document.
    assert!(matches!(
        document_draft(
            DocumentFields {
                title: Some("title".into()),
                body: Some("body".into())
            },
            Some(&ticket)
        ),
        Err(FillError::Unfillable)
    ));
}
