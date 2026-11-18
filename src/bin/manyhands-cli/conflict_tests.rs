use super::*;
use crate::input::{InputError, parse};

const OBSERVATION: &str =
    "conflict-v1:0000000000000000000000000000000000000000000000000000000000000000";
const TOKEN_A: &str =
    "conflict-path-v1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TOKEN_B: &str =
    "conflict-path-v1:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SENTINEL: &str = "SOURCE-SENTINEL-61";

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
        Err(_) => panic!("expected the conflict input to be accepted"),
    }
}

fn detail(text: &str) -> String {
    match read(text) {
        Err(error) => error.detail(),
        Ok(_) => panic!("expected the conflict input to be refused"),
    }
}

#[test]
fn the_input_is_an_observation_and_its_resolutions_in_the_order_given() {
    let input = ok(&format!(
        r#"{{"observation":"{OBSERVATION}","resolutions":[
            {{"token":"{TOKEN_B}","source":"second\r\n"}},
            {{"token":"{TOKEN_A}","source":""}}
        ]}}"#
    ));
    assert_eq!(input.observation, OBSERVATION);
    let pairs: Vec<_> = input
        .resolutions
        .iter()
        .map(|resolution| (resolution.token.as_str(), resolution.source.as_str()))
        .collect();
    // The order is the caller's, and the library orders them (the digest
    // takes token order). A source is passed on byte for byte, and may be
    // empty.
    assert_eq!(pairs, [(TOKEN_B, "second\r\n"), (TOKEN_A, "")]);
}

#[test]
fn a_token_is_whatever_string_is_given_and_the_library_judges_it() {
    let input = ok(
        r#"{"observation":"x","resolutions":[{"token":"","source":"s"},{"token":"x","source":"s"}]}"#,
    );
    assert_eq!(input.observation, "x");
    assert_eq!(input.resolutions.len(), 2);
}

#[test]
fn input_that_is_not_that_shape_is_refused_by_field_and_never_by_value() {
    for (text, expected) in [
        (
            format!(r#"{{"resolutions":[{{"token":"{TOKEN_A}","source":"s"}}]}}"#),
            "Required input field is missing: observation.",
        ),
        (
            format!(r#"{{"observation":"{OBSERVATION}"}}"#),
            "Required input field is missing: resolutions.",
        ),
        (
            format!(r#"{{"observation":1,"resolutions":[{{"token":"{TOKEN_A}","source":"s"}}]}}"#),
            "Input field observation must be a string.",
        ),
        (
            format!(
                r#"{{"observation":null,"resolutions":[{{"token":"{TOKEN_A}","source":"s"}}]}}"#
            ),
            "Input field observation must be a string.",
        ),
        (
            format!(r#"{{"observation":"{OBSERVATION}","resolutions":[]}}"#),
            "Input field resolutions must be a non-empty array of objects.",
        ),
        (
            format!(r#"{{"observation":"{OBSERVATION}","resolutions":null}}"#),
            "Input field resolutions must be a non-empty array of objects.",
        ),
        (
            format!(r#"{{"observation":"{OBSERVATION}","resolutions":"{SENTINEL}"}}"#),
            "Input field resolutions must be a non-empty array of objects.",
        ),
        (
            format!(r#"{{"observation":"{OBSERVATION}","resolutions":["{SENTINEL}"]}}"#),
            "Input field resolutions must be a non-empty array of objects.",
        ),
        (
            format!(
                r#"{{"observation":"{OBSERVATION}","resolutions":[{{"token":"{TOKEN_A}","source":"{SENTINEL}"}},7]}}"#
            ),
            "Input field resolutions must be a non-empty array of objects.",
        ),
        (
            format!(
                r#"{{"observation":"{OBSERVATION}","resolutions":[{{"source":"{SENTINEL}"}}]}}"#
            ),
            "Required input field is missing: token.",
        ),
        (
            format!(r#"{{"observation":"{OBSERVATION}","resolutions":[{{"token":"{TOKEN_A}"}}]}}"#),
            "Required input field is missing: source.",
        ),
        (
            format!(
                r#"{{"observation":"{OBSERVATION}","resolutions":[{{"token":7,"source":"{SENTINEL}"}}]}}"#
            ),
            "Input field token must be a string.",
        ),
        (
            format!(
                r#"{{"observation":"{OBSERVATION}","resolutions":[{{"token":"{TOKEN_A}","source":null}}]}}"#
            ),
            "Input field source must be a string.",
        ),
        (
            format!(
                r#"{{"observation":"{OBSERVATION}","resolutions":[{{"token":"{TOKEN_A}","source":"{SENTINEL}","path":"a"}}]}}"#
            ),
            "Unknown input field: path.",
        ),
        (
            format!(
                r#"{{"observation":"{OBSERVATION}","resolutions":[{{"token":"{TOKEN_A}","source":"s"}}],"request_id":"r"}}"#
            ),
            "Unknown input field: request_id.",
        ),
    ] {
        let message = detail(&text);
        assert_eq!(message, expected, "{text}");
        assert!(!message.contains(SENTINEL), "{message}");
    }
}
