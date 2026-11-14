use super::*;
use crate::input::{InputError, parse};

const COMMENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM0";
const PARENT: &str = "01ARZ3NDEKTSV4RRFFQ69G5FM1";
const AUTHOR: &str = "Reviewer <reviewer@example.invalid>";
const SENTINEL: &str = "BODY-SENTINEL-83";

// An input error is never formatted by an assertion: its detail is checked
// through `detail`, which names a field and never a value.
fn read(text: &str) -> Result<Fields, InputError> {
    match parse(text.as_bytes()) {
        Ok(object) => fields(&object),
        Err(error) => Err(error),
    }
}

fn ok(text: &str) -> Fields {
    match read(text) {
        Ok(fields) => fields,
        Err(_) => panic!("expected the comment input to be accepted"),
    }
}

fn detail(text: &str) -> String {
    match read(text) {
        Err(error) => error.detail(),
        Ok(_) => panic!("expected the comment input to be refused"),
    }
}

#[test]
fn the_input_has_four_fields_and_two_are_required() {
    let all = ok(&format!(
        r#"{{"comment_id":"{COMMENT}","body":"A body.\n","parent_id":"{PARENT}","created_by":"{AUTHOR}"}}"#
    ));
    assert_eq!(all.comment_id.to_string(), COMMENT);
    assert_eq!(all.body, "A body.\n");
    assert_eq!(
        all.parent_id.map(|id| id.to_string()).as_deref(),
        Some(PARENT)
    );
    assert_eq!(all.created_by.as_deref(), Some(AUTHOR));

    let least = ok(&format!(r#"{{"comment_id":"{COMMENT}","body":""}}"#));
    assert_eq!(least.body, "", "a body may be empty");
    assert!(least.parent_id.is_none());
    assert!(least.created_by.is_none());
}

#[test]
fn a_null_parent_is_a_root_comment() {
    let root = ok(&format!(
        r#"{{"comment_id":"{COMMENT}","body":"b","parent_id":null}}"#
    ));
    assert!(root.parent_id.is_none());
}

#[test]
fn a_field_that_is_missing_has_the_wrong_type_or_is_unknown_is_refused() {
    for (text, expected) in [
        (
            r#"{"body":"b"}"#.to_owned(),
            "Required input field is missing: comment_id.",
        ),
        (
            format!(r#"{{"comment_id":"{COMMENT}"}}"#),
            "Required input field is missing: body.",
        ),
        (
            format!(r#"{{"comment_id":"{COMMENT}","body":null}}"#),
            "Input field body must be a string.",
        ),
        (
            format!(r#"{{"comment_id":"{COMMENT}","body":1}}"#),
            "Input field body must be a string.",
        ),
        (
            r#"{"comment_id":1,"body":"b"}"#.to_owned(),
            "Input field comment_id must be a string.",
        ),
        (
            format!(r#"{{"comment_id":"{COMMENT}","body":"b","parent_id":1}}"#),
            "Input field parent_id must be a string or null.",
        ),
        (
            // Absent means the library's Git identity; null is no author.
            format!(r#"{{"comment_id":"{COMMENT}","body":"b","created_by":null}}"#),
            "Input field created_by must be a string.",
        ),
        (
            format!(r#"{{"comment_id":"{COMMENT}","body":"b","extra":"x"}}"#),
            "Unknown input field: extra.",
        ),
    ] {
        assert_eq!(detail(&text), expected, "{text}");
    }
}

#[test]
fn a_malformed_id_in_the_input_is_invalid_input_and_never_echoed() {
    for (field, text) in [
        (
            "comment_id",
            format!(r#"{{"comment_id":"{SENTINEL}","body":"b"}}"#),
        ),
        (
            "comment_id",
            // Lower case is not canonical.
            format!(
                r#"{{"comment_id":"{}","body":"b"}}"#,
                COMMENT.to_lowercase()
            ),
        ),
        (
            "parent_id",
            format!(r#"{{"comment_id":"{COMMENT}","body":"b","parent_id":"{SENTINEL}"}}"#),
        ),
        (
            "parent_id",
            format!(r#"{{"comment_id":"{COMMENT}","body":"b","parent_id":""}}"#),
        ),
    ] {
        let message = detail(&text);
        assert_eq!(
            message,
            format!("Input field {field} must be one of: a canonical ULID."),
            "{text}"
        );
        assert!(!message.contains(SENTINEL));
    }
}
