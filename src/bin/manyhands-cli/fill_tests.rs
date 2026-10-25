use super::*;
use crate::input::Tristate;
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

fn ticket_fields() -> TicketFields {
    TicketFields {
        title: Some("Given title".into()),
        ticket_type: Some("task".into()),
        status: Some("open".into()),
        body: Some("Given body".into()),
        project: Tristate::Value("Given project".into()),
        team: Tristate::Null,
    }
}

fn ticket_item() -> ItemDto {
    ItemDto {
        kind: ItemDtoKind::Ticket,
        ticket_type: Some("cycle".into()),
        status: Some("planned".into()),
        project: Some("Current project".into()),
        team: Some("Current team".into()),
        ..item()
    }
}

#[test]
fn ticket_all_fields_given_need_no_read() {
    let fields = ticket_fields();
    assert!(!needs_read_ticket(&fields));
    let draft = ticket_draft(fields, None).ok().unwrap();
    assert_eq!(draft.title, "Given title");
    assert_eq!(draft.ticket_type, "task");
    assert_eq!(draft.status, "open");
    assert_eq!(draft.body, "Given body");
    assert_eq!(draft.project.as_deref(), Some("Given project"));
    assert!(draft.team.is_none());
}

#[test]
fn ticket_missing_nullable_fields_keep_current_including_absence_and_null_clears() {
    for present in [true, false] {
        let mut current = ticket_item();
        if !present {
            current.project = None;
            current.team = None;
        }
        let mut fields = ticket_fields();
        fields.project = Tristate::Missing;
        fields.team = Tristate::Missing;
        assert!(needs_read_ticket(&fields));
        let draft = ticket_draft(fields, Some(&current)).ok().unwrap();
        assert_eq!(draft.project, current.project);
        assert_eq!(draft.team, current.team);
        let mut fields = ticket_fields();
        fields.project = Tristate::Null;
        assert!(!needs_read_ticket(&fields));
        let draft = ticket_draft(fields, Some(&current)).ok().unwrap();
        assert!(draft.project.is_none());
        assert!(draft.team.is_none());
    }
}

#[test]
fn each_omitted_ticket_string_comes_from_read_and_missing_required_values_are_unfillable() {
    for omitted in 0..4 {
        let mut fields = ticket_fields();
        match omitted {
            0 => fields.title = None,
            1 => fields.ticket_type = None,
            2 => fields.status = None,
            _ => fields.body = None,
        }
        assert!(needs_read_ticket(&fields));
        let current = ticket_item();
        let draft = ticket_draft(fields, Some(&current)).ok().unwrap();
        assert_eq!(
            draft.title,
            if omitted == 0 {
                "Current title"
            } else {
                "Given title"
            }
        );
        assert_eq!(
            draft.ticket_type,
            if omitted == 1 { "cycle" } else { "task" }
        );
        assert_eq!(draft.status, if omitted == 2 { "planned" } else { "open" });
        assert_eq!(
            draft.body.as_bytes(),
            if omitted == 3 {
                current.body.as_ref().unwrap().as_bytes()
            } else {
                b"Given body"
            }
        );
        let mut fields = ticket_fields();
        let mut current = ticket_item();
        match omitted {
            0 => {
                fields.title = None;
                current.title = None;
            }
            1 => {
                fields.ticket_type = None;
                current.ticket_type = None;
            }
            2 => {
                fields.status = None;
                current.status = None;
            }
            _ => {
                fields.body = None;
                current.body = None;
            }
        }
        assert!(matches!(
            ticket_draft(fields, Some(&current)),
            Err(FillError::Unfillable)
        ));
    }
}

#[test]
fn ticket_draft_refuses_a_document_even_with_all_fields_given() {
    assert!(matches!(
        ticket_draft(ticket_fields(), Some(&item())),
        Err(FillError::Unfillable)
    ));
}
