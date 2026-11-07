use super::*;

fn pending(editor: &mut LineEditor, byte: u8, expected: &[u8]) {
    match editor.feed(byte) {
        Step::Pending(bytes) => assert_eq!(bytes, expected),
        _ => panic!("expected continued input"),
    }
}

fn line(editor: LineEditor, expected: &str) {
    assert!(
        editor
            .finish()
            .is_some_and(|line| line.as_str() == expected)
    );
}

#[test]
fn enter_ends_a_line_with_cr_or_lf() {
    for ending in *b"\r\n" {
        let mut editor = LineEditor::new(false);
        pending(&mut editor, b'a', b"");
        assert!(matches!(editor.feed(ending), Step::Done));
        line(editor, "a");
    }
    let mut editor = LineEditor::new(false);
    for byte in b"\r\n" {
        assert!(matches!(editor.feed(*byte), Step::Done));
    }
    line(editor, "");
}

#[test]
fn ctrl_c_cancels_and_ctrl_d_cancels_only_an_empty_line() {
    for prefix in [b"".as_slice(), b"a"] {
        let mut editor = LineEditor::new(false);
        for byte in prefix {
            pending(&mut editor, *byte, b"");
        }
        assert!(matches!(editor.feed(0x03), Step::Cancel));
    }
    let mut editor = LineEditor::new(false);
    assert!(matches!(editor.feed(0x04), Step::Cancel));
    pending(&mut editor, b'a', b"");
    pending(&mut editor, 0x04, b"");
    line(editor, "a");
}

#[test]
fn backspace_removes_a_whole_multibyte_character() {
    for erase in [0x7f, 0x08] {
        for text in ["aé", "a€", "a🦀"] {
            let mut editor = LineEditor::new(false);
            for byte in text.bytes().chain([erase]) {
                pending(&mut editor, byte, b"");
            }
            line(editor, "a");
        }
        let mut editor = LineEditor::new(false);
        pending(&mut editor, erase, b"");
        line(editor, "");
    }
}

#[test]
fn ctrl_u_clears_the_line() {
    let mut editor = LineEditor::new(false);
    for byte in "aé🦀".bytes().chain([0x15, b'z']) {
        pending(&mut editor, byte, b"");
    }
    line(editor, "z");
}

#[test]
fn other_control_bytes_are_ignored() {
    let mut editor = LineEditor::new(true);
    for byte in 0..=0x1f {
        if ![0x03, 0x04, 0x08, 0x0a, 0x0d, 0x15].contains(&byte) {
            pending(&mut editor, byte, b"");
        }
    }
    line(editor, "");
}

#[test]
fn bytes_past_the_limit_are_ignored() {
    let mut editor = LineEditor::new(true);
    for _ in 0..MAX_LINE_BYTES {
        pending(&mut editor, b'a', b"a");
    }
    for _ in 0..10 {
        pending(&mut editor, b'b', b"");
    }
    pending(&mut editor, 0x7f, b"\x08 \x08");
    pending(&mut editor, b'z', b"z");
    line(editor, &format!("{}z", "a".repeat(MAX_LINE_BYTES - 1)));
}

#[test]
fn invalid_utf8_finishes_as_none() {
    for bytes in [vec![0xff], vec![0xc3], vec![0xc3, b'a'], vec![0x80]] {
        let mut editor = LineEditor::new(false);
        for byte in bytes {
            pending(&mut editor, byte, b"");
        }
        assert!(editor.finish().is_none());
    }
}

#[test]
fn masked_lines_echo_nothing() {
    let mut editor = LineEditor::new(false);
    for byte in "masked é🦀"
        .bytes()
        .chain([0x7f, 0x08, 0x15, b'x', 0x04, 0])
    {
        pending(&mut editor, byte, b"");
    }
    assert!(matches!(editor.feed(b'\n'), Step::Done));
    line(editor, "x");
}

#[test]
fn echoed_lines_draw_and_erase() {
    let mut editor = LineEditor::new(true);
    pending(&mut editor, 0x08, b"");
    for byte in "aé🦀".bytes() {
        pending(&mut editor, byte, &[byte]);
    }
    pending(&mut editor, 0x7f, b"\x08 \x08");
    pending(&mut editor, 0x15, b"\x08 \x08\x08 \x08");
    pending(&mut editor, 0x15, b"");
    pending(&mut editor, b'z', b"z");
    line(editor, "z");
}
