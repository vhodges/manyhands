use super::*;
use manyhands::results::{RecoveryAction, RecoveryActionKind, Scope};
use serde_json::{Value, json};

#[test]
fn every_exit_class_has_its_contract_status() {
    for (class, expected) in [
        (None, 0),
        (Some(FailureClass::Input), 2),
        (Some(FailureClass::Blocked), 3),
        (Some(FailureClass::Incomplete), 4),
        (Some(FailureClass::Transient), 5),
        (Some(FailureClass::Internal), 1),
        (Some(FailureClass::Cancelled), 130),
    ] {
        assert_eq!(exit_status(class), expected);
    }
}

fn failure() -> Envelope<Value> {
    Envelope::failure(
        "repo inspect",
        Scope::default(),
        ResultCode::NotRepositoryRoot,
        vec![
            RecoveryAction::new(RecoveryActionKind::RepoInspect, [("root", json!("/a b"))]),
            RecoveryAction::new(RecoveryActionKind::IndexRebuild, []),
        ],
    )
}

#[test]
fn json_is_compact_envelope_plus_newline_without_human_rendering() {
    for envelope in [
        Envelope::read_success("id new", Scope::default(), json!({"id": "value"})),
        failure(),
    ] {
        let output = envelope_output(true, &envelope, |_| panic!("human renderer called"));
        assert_eq!(
            output.stdout,
            format!("{}\n", serde_json::to_string(&envelope).unwrap()).as_bytes()
        );
        assert!(output.stderr.is_empty());
        assert_eq!(output.status, exit_status(envelope.failure_class()));
    }
}

#[test]
fn human_success_separates_text_and_notes() {
    let envelope = Envelope::read_success("test read", Scope::default(), "body");
    let output = envelope_output(false, &envelope, |data| Human {
        stdout: format!("{data}\n"),
        notes: vec!["stale".into(), "incomplete".into()],
    });
    assert_eq!(output.stdout, b"body\n");
    assert_eq!(output.stderr, b"note: stale\nnote: incomplete\n");
    assert_eq!(output.status, 0);
}

#[test]
fn human_failure_names_code_and_each_recovery_without_rendering_data() {
    let envelope = failure();
    let output = envelope_output(false, &envelope, |_| panic!("failure rendered as data"));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, format!("error: {}\ncode: not_repository_root\nnext: repo.inspect root=/a b\nnext: index.rebuild\n", envelope.message).as_bytes());
    assert_eq!(output.status, 2);
}

#[test]
fn extra_is_one_line_after_code_before_recovery_and_ignored_in_json() {
    let envelope = failure();
    let output = failure_output(false, &envelope, Some("detail\n".into()));
    assert_eq!(output.stderr, format!("error: {}\ncode: not_repository_root\ndetail\nnext: repo.inspect root=/a b\nnext: index.rebuild\n", envelope.message).as_bytes());
    assert!(output.stdout.is_empty());
    let json = failure_output(true, &envelope, Some("SECRET extra".into()));
    assert_eq!(
        json.stdout,
        format!("{}\n", serde_json::to_string(&envelope).unwrap()).as_bytes()
    );
    assert!(json.stderr.is_empty());
}

#[test]
fn recovery_fields_escape_controls_preserving_unicode_and_action_lines() {
    let root = "/é 中文\nnext: forged\r\u{1b}[31m\t\u{85}";
    let escaped = "/é 中文\\nnext: forged\\r\\u{1b}[31m\\t\\u{85}";
    let envelope = Envelope::<Value>::failure(
        "remote list",
        Scope::default(),
        ResultCode::NotRepositoryRoot,
        vec![
            RecoveryAction::new(RecoveryActionKind::RepoInspect, [("root", json!(root))]),
            RecoveryAction::new(RecoveryActionKind::IndexRebuild, []),
        ],
    );
    for output in [
        envelope_output(false, &envelope, |_| panic!("failure rendered as data")),
        failure_output(false, &envelope, None),
    ] {
        assert_eq!(output.status, 2);
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(
            stderr,
            format!(
                "error: {}\ncode: not_repository_root\nnext: repo.inspect root={escaped}\nnext: index.rebuild\n",
                envelope.message
            )
        );
        assert_eq!(stderr.lines().count(), 4);
        assert!(stderr.chars().all(|c| c == '\n' || !c.is_control()));
    }
    // Human escaping must not modify the envelope or its JSON field values.
    for output in [
        envelope_output(true, &envelope, |_| panic!("JSON called human renderer")),
        failure_output(true, &envelope, None),
    ] {
        assert_eq!(output.status, 2);
        assert!(output.stderr.is_empty());
        assert_eq!(
            output.stdout,
            format!("{}\n", serde_json::to_string(&envelope).unwrap()).as_bytes()
        );
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["recovery"][0]["arguments"]["root"], root);
    }
}

struct CannotSerialize;

impl Serialize for CannotSerialize {
    fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "SECRET backend serialization failure",
        ))
    }
}

#[test]
fn serialization_failure_is_internal_without_backend_text_or_panic() {
    let envelope = Envelope::read_success("test read", Scope::default(), CannotSerialize);
    let output = envelope_output(true, &envelope, |_| panic!("human renderer called"));
    assert_eq!(output.status, 1);
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["code"], "internal_error");
    assert_eq!(value["command"], "test read");
    assert!(!String::from_utf8(output.stdout).unwrap().contains("SECRET"));
}

#[derive(Default)]
struct Writer {
    bytes: Vec<u8>,
    calls: Vec<&'static str>,
    fail_write: bool,
    fail_flush: bool,
}

impl Write for Writer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.calls.push("write");
        if self.fail_write {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        // Short writes prove the caller uses write_all, not a single write.
        let count = bytes.len().min(2);
        self.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.calls.push("flush");
        if self.fail_flush {
            Err(io::ErrorKind::BrokenPipe.into())
        } else {
            Ok(())
        }
    }
}

fn buffers() -> Output {
    Output {
        stdout: b"output".to_vec(),
        stderr: b"errors".to_vec(),
        status: 0,
    }
}

#[test]
fn writes_all_bytes_and_flushes_both_streams() {
    let (mut stdout, mut stderr) = (Writer::default(), Writer::default());
    write_output(&buffers(), &mut stdout, &mut stderr).unwrap();
    assert_eq!(stdout.bytes, b"output");
    assert_eq!(stderr.bytes, b"errors");
    assert_eq!(stdout.calls, ["write", "write", "write", "flush"]);
    assert_eq!(stderr.calls, stdout.calls);
}

#[test]
fn any_write_or_flush_error_stops_further_output() {
    for position in 0..4 {
        let mut stdout = Writer {
            fail_write: position == 0,
            fail_flush: position == 1,
            ..Writer::default()
        };
        let mut stderr = Writer {
            fail_write: position == 2,
            fail_flush: position == 3,
            ..Writer::default()
        };
        assert!(write_output(&buffers(), &mut stdout, &mut stderr).is_err());
        if position < 2 {
            assert!(stderr.calls.is_empty());
        }
        let failed = if position < 2 { &stdout } else { &stderr };
        if position % 2 == 0 {
            assert_eq!(failed.calls, ["write"]);
        } else {
            assert_eq!(failed.calls.last(), Some(&"flush"));
        }
    }
}
