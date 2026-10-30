use std::{
    fs,
    io::{self, Read},
};

use super::{InputError, InputObject, Tristate, parse, read, schema_name};

const ALLOWED: &[&str] = &["passphrase", "none"];
const SENTINEL: &str = "VALUE-SENTINEL-41";

// Neither successful input nor input errors are formatted by test assertions.
fn ok<T>(result: Result<T, InputError>) -> T {
    match result {
        Ok(value) => value,
        Err(_) => panic!("expected successful input check"),
    }
}

fn error<T>(result: Result<T, InputError>) -> InputError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected an input error"),
    }
}

fn object(text: &str) -> InputObject {
    ok(parse(text.as_bytes()))
}

#[test]
fn one_object_parses() {
    let input = object(
        r#" {"name":"ÆØ", "identity":{"email":"a@example.test"}, "other":[true,false,null,1,-2,1.25]} "#,
    );
    assert_eq!(ok(input.string("name")), "ÆØ");
    let identity = ok(input.optional_object("identity")).unwrap();
    assert_eq!(ok(identity.string("email")), "a@example.test");
    assert!(input.only(&["name", "identity", "other"]).is_ok());
}

#[test]
fn a_leading_byte_order_mark_is_skipped() {
    assert!(parse(b"\xef\xbb\xbf {}\n").is_ok());
    assert!(matches!(
        parse(b"\xef\xbb\xbf\xef\xbb\xbf{}"),
        Err(InputError::Malformed)
    ));
}

#[test]
fn empty_input_is_refused() {
    for bytes in [b"".as_slice(), b" \r\n\t", b"\xef\xbb\xbf \n"] {
        assert!(matches!(parse(bytes), Err(InputError::Empty)));
    }
}

#[test]
fn malformed_and_trailing_json_are_refused() {
    for bytes in [
        b"{".as_slice(),
        br#"{"name":}"#,
        br#"{"name":"VALUE-SENTINEL-41"} {}"#,
        b"{}true",
        b"{\"n\":1e999}",
        b"\x0b",
    ] {
        assert!(matches!(parse(bytes), Err(InputError::Malformed)));
    }
}

#[test]
fn non_objects_are_refused() {
    for text in [
        "null",
        "true",
        "false",
        "42",
        "-1",
        "1.5",
        "[]",
        r#""VALUE-SENTINEL-41""#,
    ] {
        assert!(matches!(parse(text.as_bytes()), Err(InputError::NotObject)));
    }
}

#[test]
fn duplicate_keys_are_refused_at_any_depth() {
    for text in [
        r#"{"name":null,"name":"VALUE-SENTINEL-41"}"#,
        r#"{"identity":{"name":"VALUE-SENTINEL-41","name":null}}"#,
        r#"{"unknown":[[{"name":false,"name":true}]]}"#,
        r#"{"name":1,"na\u006de":2}"#,
        r#"[{"name":0,"name":1}]"#,
    ] {
        assert!(
            matches!(parse(text.as_bytes()), Err(InputError::DuplicateKey(key)) if key == "name")
        );
    }
    assert!(parse(br#"{"left":{"name":1},"right":{"name":2}}"#).is_ok());
}

#[test]
fn duplicate_capture_preserves_escaped_and_error_like_key_names() {
    for key in ["k\x1b[2J", "line\nquote\"\\ at line 1 column 2", "", "雪"] {
        let encoded = serde_json::to_string(key).unwrap();
        let text = format!("{{{encoded}:null,{encoded}:false}}");
        assert!(
            matches!(parse(text.as_bytes()), Err(InputError::DuplicateKey(found)) if found == key)
        );
    }
    // An early abort must not contaminate a later parse or reinterpret backend text.
    assert!(matches!(parse(b"{"), Err(InputError::Malformed)));
}

#[test]
fn excessive_nesting_is_refused_without_unbounded_recursion() {
    let text = format!("{{\"nested\":{}0{}}}", "[".repeat(256), "]".repeat(256));
    assert!(matches!(parse(text.as_bytes()), Err(InputError::Malformed)));
}

#[test]
fn unknown_fields_are_refused() {
    assert!(
        matches!(object(r#"{"passphrase":"VALUE-SENTINEL-41"}"#).only(&["name"]), Err(InputError::UnknownField(key)) if key == "passphrase")
    );
    assert!(object("{}").only(&[]).is_ok());
}

#[test]
fn required_strings_distinguish_missing_wrong_type_and_values() {
    assert!(matches!(
        object("{}").string("name"),
        Err(InputError::Missing("name"))
    ));
    for value in ["null", "false", "3", "[]", "{}"] {
        assert!(matches!(
            object(&format!("{{\"name\":{value}}}")).string("name"),
            Err(InputError::WrongType {
                field: "name",
                expected: "a string"
            })
        ));
    }
    for value in ["", "  ", "VALUE-SENTINEL-41"] {
        let text = format!("{{\"name\":{}}}", serde_json::to_string(value).unwrap());
        assert_eq!(ok(object(&text).string("name")), value);
    }
}

#[test]
fn optional_strings_allow_missing_but_refuse_null() {
    assert!(ok(object("{}").optional_string("name")).is_none());
    assert_eq!(
        ok(object(r#"{"name":"VALUE-SENTINEL-41"}"#).optional_string("name")).as_deref(),
        Some(SENTINEL)
    );
    for value in ["null", "true", "1", "[]", "{}"] {
        assert!(matches!(
            object(&format!("{{\"name\":{value}}}")).optional_string("name"),
            Err(InputError::WrongType {
                field: "name",
                expected: "a string"
            })
        ));
    }
}

#[test]
fn nullable_strings_are_required_but_accept_null() {
    assert!(matches!(
        object("{}").nullable_string("name"),
        Err(InputError::Missing("name"))
    ));
    assert!(ok(object(r#"{"name":null}"#).nullable_string("name")).is_none());
    assert_eq!(
        ok(object(r#"{"name":"VALUE-SENTINEL-41"}"#).nullable_string("name")).as_deref(),
        Some(SENTINEL)
    );
    for value in ["true", "1", "[]", "{}"] {
        assert!(matches!(
            object(&format!("{{\"name\":{value}}}")).nullable_string("name"),
            Err(InputError::WrongType {
                field: "name",
                expected: "a string or null"
            })
        ));
    }
}

#[test]
fn tristate_preserves_missing_null_and_string() {
    assert!(matches!(
        ok(object("{}").tristate("initials")),
        Tristate::Missing
    ));
    assert!(matches!(
        ok(object(r#"{"initials":null}"#).tristate("initials")),
        Tristate::Null
    ));
    assert!(
        matches!(ok(object(r#"{"initials":"VALUE-SENTINEL-41"}"#).tristate("initials")), Tristate::Value(value) if value == SENTINEL)
    );
    for value in ["true", "1", "[]", "{}"] {
        assert!(matches!(
            object(&format!("{{\"initials\":{value}}}")).tristate("initials"),
            Err(InputError::WrongType {
                field: "initials",
                expected: "a string or null"
            })
        ));
    }
}

#[test]
fn optional_objects_allow_missing_but_refuse_null_and_check_nested_fields() {
    assert!(ok(object("{}").optional_object("identity")).is_none());
    for value in ["null", "true", "1", "[]", r#""VALUE-SENTINEL-41""#] {
        assert!(matches!(
            object(&format!("{{\"identity\":{value}}}")).optional_object("identity"),
            Err(InputError::WrongType {
                field: "identity",
                expected: "an object"
            })
        ));
    }
    let input = object(r#"{"identity":{"name":"VALUE-SENTINEL-41","extra":null}}"#);
    let nested = ok(input.optional_object("identity")).unwrap();
    assert_eq!(ok(nested.string("name")), SENTINEL);
    assert!(
        matches!(nested.only(&["name", "email"]), Err(InputError::UnknownField(key)) if key == "extra")
    );
    assert!(matches!(
        nested.string("email"),
        Err(InputError::Missing("email"))
    ));
}

#[test]
fn enums_require_an_exact_allowed_string() {
    for value in ALLOWED {
        let input = object(&format!(r#"{{"protection":"{value}"}}"#));
        let chosen: &'static str = ok(input.one_of("protection", ALLOWED));
        assert_eq!(chosen, *value);
    }
    assert!(matches!(
        object("{}").one_of("protection", ALLOWED),
        Err(InputError::Missing("protection"))
    ));
    assert!(matches!(
        object(r#"{"protection":null}"#).one_of("protection", ALLOWED),
        Err(InputError::WrongType {
            field: "protection",
            expected: "a string"
        })
    ));
    assert!(
        matches!(object(r#"{"protection":"VALUE-SENTINEL-41"}"#).one_of("protection", ALLOWED), Err(InputError::InvalidValue { field: "protection", allowed }) if allowed == ALLOWED)
    );
}

#[test]
fn no_detail_echoes_a_value() {
    let mut invalid_utf8 = br#"{"name":"VALUE-SENTINEL-41"}"#.to_vec();
    invalid_utf8.push(0xff);
    let errors = [
        error(parse(&invalid_utf8)),
        error(parse(b"")),
        error(parse(br#"{"name":"VALUE-SENTINEL-41",}"#)),
        error(parse(br#""VALUE-SENTINEL-41""#)),
        error(parse(
            br#"{"name":"VALUE-SENTINEL-41","name":"VALUE-SENTINEL-41"}"#,
        )),
        error(object(r#"{"unknown":"VALUE-SENTINEL-41"}"#).only(&[])),
        error(object(r#"{"other":"VALUE-SENTINEL-41"}"#).string("name")),
        error(object(r#"{"identity":"VALUE-SENTINEL-41"}"#).optional_object("identity")),
        error(object(r#"{"name":["VALUE-SENTINEL-41"]}"#).string("name")),
        error(object(r#"{"name":["VALUE-SENTINEL-41"]}"#).optional_string("name")),
        error(object(r#"{"name":["VALUE-SENTINEL-41"]}"#).nullable_string("name")),
        error(object(r#"{"initials":["VALUE-SENTINEL-41"]}"#).tristate("initials")),
        error(object(r#"{"protection":"VALUE-SENTINEL-41"}"#).one_of("protection", ALLOWED)),
        error(read("-", std::path::Path::new("."), &mut BrokenReader)),
    ];
    for error in errors {
        let detail = error.detail();
        assert!(!detail.is_empty());
        assert!(!detail.contains(SENTINEL));
        assert!(!detail.contains('\n'));
    }
}

#[test]
fn a_key_name_is_escaped_in_its_detail() {
    let unknown = error(object(r#"{"k\u001b[2J":"VALUE-SENTINEL-41"}"#).only(&[]));
    let duplicate = error(parse(
        br#"{"k\u001b[2J":null,"k\u001b[2J":"VALUE-SENTINEL-41"}"#,
    ));
    for error in [unknown, duplicate] {
        let detail = error.detail();
        assert!(detail.contains(&crate::render::field("k\x1b[2J")));
        assert!(!detail.contains('\x1b'));
        assert!(!detail.contains(SENTINEL));
    }
}

#[test]
fn utf16_input_is_not_utf8() {
    assert!(matches!(
        parse(&[0xff, 0xfe, 0x7b, 0, 0x7d, 0]),
        Err(InputError::NotUtf8)
    ));
}

#[test]
fn read_refuses_a_directory_and_a_missing_file() {
    let temp = tempfile::tempdir().unwrap();
    for source in [".", "missing.json"] {
        assert!(matches!(
            read(source, temp.path(), &mut BrokenReader),
            Err(InputError::Unreadable)
        ));
    }
}

#[test]
fn read_refuses_empty_stdin() {
    let temp = tempfile::tempdir().unwrap();
    assert!(matches!(
        read("-", temp.path(), &mut io::empty()),
        Err(InputError::Empty)
    ));
}

#[test]
fn read_refuses_utf16_file_with_encoding_detail() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("input.json"),
        [0xff, 0xfe, 0x7b, 0, 0x7d, 0],
    )
    .unwrap();
    let error = error(read("input.json", temp.path(), &mut BrokenReader));
    assert!(matches!(error, InputError::NotUtf8));
    assert!(error.detail().contains("not UTF-8"));
}

struct BrokenReader;
impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other(SENTINEL))
    }
}

#[test]
fn read_maps_stdin_io_errors_without_backend_text() {
    let temp = tempfile::tempdir().unwrap();
    let error = error(read("-", temp.path(), &mut BrokenReader));
    assert!(matches!(error, InputError::Unreadable));
    assert!(!error.detail().contains(SENTINEL));
}

#[test]
fn read_resolves_relative_and_absolute_files_without_reading_stdin() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("input.json");
    fs::write(&path, br#"{"name":"VALUE-SENTINEL-41"}"#).unwrap();
    for source in ["input.json", path.to_str().unwrap()] {
        assert_eq!(
            ok(ok(read(source, temp.path(), &mut BrokenReader)).string("name")),
            SENTINEL
        );
    }
}

#[test]
fn stdin_is_consumed_to_eof_once_including_trailing_input() {
    struct Once {
        bytes: &'static [u8],
        ended: bool,
    }
    impl Read for Once {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            assert!(!self.ended, "read after EOF");
            if self.bytes.is_empty() {
                self.ended = true;
                return Ok(0);
            }
            buffer[0] = self.bytes[0];
            self.bytes = &self.bytes[1..];
            Ok(1)
        }
    }
    let temp = tempfile::tempdir().unwrap();
    for (bytes, valid) in [(b"{} \n".as_slice(), true), (b"{} {}".as_slice(), false)] {
        let mut stdin = Once {
            bytes,
            ended: false,
        };
        let result = read("-", temp.path(), &mut stdin);
        assert!(stdin.ended);
        if valid {
            assert!(result.is_ok());
        } else {
            assert!(matches!(result, Err(InputError::Malformed)));
        }
    }
}

#[test]
fn schema_names_replace_verb_hyphens() {
    assert_eq!(schema_name("remote", "add"), "remote_add.schema.json");
    assert_eq!(
        schema_name("repo", "identity-set"),
        "repo_identity_set.schema.json"
    );
}

#[test]
fn list_readers_preserve_order_repeats_empty_missing_and_null() {
    use super::ListChange;
    for values in [vec![], vec!["z", "a", "z", ""]] {
        let input = object(&serde_json::json!({"deps":values}).to_string());
        assert_eq!(ok(input.optional_string_list("deps")).unwrap(), values);
        assert!(matches!(ok(input.tristate_list("deps")), ListChange::Value(v) if v == values));
    }
    assert!(ok(object("{}").optional_string_list("deps")).is_none());
    assert!(matches!(
        ok(object("{}").tristate_list("deps")),
        ListChange::Missing
    ));
    assert!(matches!(
        ok(object(r#"{"deps":null}"#).tristate_list("deps")),
        ListChange::Null
    ));
    assert!(matches!(
        object(r#"{"deps":null}"#).optional_string_list("deps"),
        Err(InputError::WrongType { field: "deps", .. })
    ));
}

#[test]
fn list_readers_refuse_wrong_shapes_without_disclosing_values() {
    for value in [
        "true",
        "1234567",
        r#""VALUE-SENTINEL-41""#,
        r#"{"VALUE-SENTINEL-41":1}"#,
        r#"["VALUE-SENTINEL-41",1234567]"#,
        r#"["VALUE-SENTINEL-41",null]"#,
        r#"["VALUE-SENTINEL-41",{"secret":"VALUE-SENTINEL-41"}]"#,
    ] {
        let input = object(&format!("{{\"deps\":{value}}}"));
        for error in [
            error(input.optional_string_list("deps")),
            error(input.tristate_list("deps")),
        ] {
            assert!(matches!(error, InputError::WrongType { field: "deps", .. }));
            let detail = error.detail();
            assert!(detail.contains("deps"));
            assert!(!detail.contains(SENTINEL));
            assert!(!detail.contains("1234567"));
        }
    }
}

#[test]
fn object_lists_keep_their_order_and_the_objects_are_readable() {
    let input = object(r#"{"items":[{"name":"b"},{"name":"a"},{"name":"b"}]}"#);
    let names: Vec<_> = ok(input.object_list("items"))
        .iter()
        .map(|item| ok(item.string("name")))
        .collect();
    assert_eq!(names, ["b", "a", "b"]);
    // An object of the list is closed like any other.
    let items = ok(input.object_list("items"));
    assert!(items[0].only(&["name"]).is_ok());
    assert!(matches!(
        items[0].only(&["other"]),
        Err(InputError::UnknownField(field)) if field == "name"
    ));
}

#[test]
fn an_object_list_that_is_missing_empty_or_not_a_list_of_objects_is_refused() {
    assert!(matches!(
        error(object("{}").object_list("items")),
        InputError::Missing("items")
    ));
    for value in [
        "[]",
        "null",
        "true",
        "1234567",
        r#""VALUE-SENTINEL-41""#,
        r#"{"VALUE-SENTINEL-41":{}}"#,
        r#"["VALUE-SENTINEL-41"]"#,
        "[1234567]",
        "[null]",
        "[[]]",
        r#"[{"a":"b"},"VALUE-SENTINEL-41"]"#,
        r#"[{"a":"b"},{"a":"b"},[{"a":"b"}]]"#,
    ] {
        let error = error(object(&format!("{{\"items\":{value}}}")).object_list("items"));
        assert!(
            matches!(error, InputError::WrongType { field: "items", .. }),
            "{value}"
        );
        // The detail names the field and the shape, never a value.
        let detail = error.detail();
        assert_eq!(
            detail,
            "Input field items must be a non-empty array of objects."
        );
        assert!(!detail.contains(SENTINEL));
    }
}
