//! P20, `frame_copy_uses_the_vocabulary` (Wave 04 Cycle 05a design Part F;
//! `06`'s writing rules; acceptance A10).
//!
//! Every string the Add repository sheet and the Unlock key dialog draw, as
//! constants and formatting functions, against the text Part F gives it
//! character for character: Part F's table; the lines B6, B10 and C2 give
//! verbatim that Part F leaves out (`ADDING`, `SHARE_NEEDS_IDENTITY`,
//! `other_main_branch`, `not_ssh_remote`); and what Part F keeps from today,
//! as `origin/main` has it. Then `06`'s rules over each: sentence case, no
//! exclamation mark, no emoji, none of the terms `06` replaces, and `…` on
//! an action that opens a chooser, a confirmation or a sheet, with four
//! named exceptions (`EXCEPTIONS`). The table is checked against the
//! sources, so a constant or function added to the sheet or the dialog and
//! left out of it fails, and so does copy written inline outside
//! `add_repository/copy.rs`.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[path = "vocabulary_table.rs"]
mod table;

use self::table::{EXCEPTIONS, Kind, Line, table};

/// The capitalised words a sentence may hold after its first word, and why.
const NAMES: [(&str, &str); 8] = [
    ("Manyhands", "the product"),
    ("Git", "a proper noun"),
    ("SSH", "an acronym"),
    ("NUL", "an acronym (REFUSED, kept from today)"),
    (
        "Keep",
        "the quoted item \"Keep on this computer\" (SHARE_HELP)",
    ),
    ("Not", "the button Not now (NOT_NOW_NOTE)"),
    ("Needs", "the place Needs attention (NOT_NOW_NOTE)"),
    ("Settings", "the sheet's own name (SHARE_NEEDS_IDENTITY)"),
];

/// The words `06` replaces, as P18 has them (Wave 04 Cycle 04 Part E): a
/// string uses none of them as a whole word.
const WORDS: [&str; 10] = [
    "publication",
    "publish",
    "publishes",
    "published",
    "publishing",
    "pin",
    "pinned",
    "pins",
    "pinning",
    "sidebar",
];

/// The phrases `06` replaces, as lower-case words.
const PHRASES: [&[&str]; 4] = [
    &["ssh", "key"],
    &["remotes", "page"],
    &["hosts", "page"],
    &["back", "to", "tabs"],
];

fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The terms `06` replaces that `text` uses, whole words only.
fn replaced(text: &str) -> Vec<String> {
    let words = words(text);
    let mut found: Vec<String> = words
        .iter()
        .filter(|word| WORDS.contains(&word.as_str()))
        .cloned()
        .collect();
    for phrase in PHRASES {
        if words.windows(phrase.len()).any(|window| window == phrase) {
            found.push(phrase.join(" "));
        }
    }
    found
}

/// The words of `text` that break sentence case: a sentence that does not
/// start with a capital, or a capitalised word inside one that is not one
/// of `NAMES`.
fn not_sentence_case(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut starts = true;
    for word in text.split_whitespace() {
        let bare = word.trim_matches(|c: char| !c.is_alphanumeric());
        if let Some(first) = bare.chars().next().filter(|c| c.is_alphabetic()) {
            if starts && !first.is_uppercase() {
                found.push(format!("{bare} (starts a sentence)"));
            }
            if !starts && first.is_uppercase() && !NAMES.iter().any(|(name, _)| *name == bare) {
                found.push(bare.to_owned());
            }
        }
        starts = word.ends_with(['.', '?']);
    }
    found
}

/// Whether `c` is an emoji or a pictograph.
fn emoji(c: char) -> bool {
    matches!(c as u32, 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0xFE0F | 0x1F000..=0x1FFFF)
}

/// What `row` breaks of `06`'s rules and of its text.
fn broken(row: &Line) -> Vec<String> {
    let mut found = Vec::new();
    if row.value != row.wanted {
        found.push(format!("is not the text it must be, {:?}", row.wanted));
    }
    if row.value.contains('!') {
        found.push("has an exclamation mark".to_owned());
    }
    if row.value.contains("...") {
        found.push("has three dots for an ellipsis".to_owned());
    }
    if row.value.chars().any(emoji) {
        found.push("has an emoji".to_owned());
    }
    let replaced = replaced(&row.value);
    if !replaced.is_empty() {
        found.push(format!("uses {replaced:?}, which 06 replaces"));
    }
    let capitals = not_sentence_case(&row.value);
    if !capitals.is_empty() {
        found.push(format!("is not in sentence case: {capitals:?}"));
    }
    let ellipsis = row.value.matches('\u{2026}').count();
    let ends = row.value.ends_with('\u{2026}');
    let named = EXCEPTIONS.iter().any(|(name, _)| *name == row.name);
    match row.kind {
        Kind::Opens if !ends || ellipsis != 1 => {
            found.push("opens a chooser, a confirmation or a sheet, without its …".to_owned())
        }
        Kind::Excepted if !named => found.push("is excepted from … but not named".to_owned()),
        Kind::Excepted if ellipsis > 0 => {
            found.push("is a named exception, yet has a …".to_owned())
        }
        Kind::Acts if ellipsis > 0 => found.push("opens nothing, yet has a …".to_owned()),
        Kind::Words if ellipsis > 1 || (ellipsis == 1 && !ends) => {
            found.push("has a … that does not end a working state".to_owned())
        }
        _ => {}
    }
    found
}

fn view_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/desktop/view")
}

/// The sources of the sheet and the dialog, as `(module, file name, text)`:
/// `add_repository.rs` and its directory are `add_repository`, whose copy
/// `add_repository/copy.rs` holds; `passphrase.rs` and its directory are
/// `passphrase`.
fn sources() -> Vec<(&'static str, String, String)> {
    let mut found = Vec::new();
    for module in ["add_repository", "passphrase"] {
        let file = view_dir().join(format!("{module}.rs"));
        found.push((
            module,
            format!("{module}.rs"),
            fs::read_to_string(&file).unwrap(),
        ));
        for entry in fs::read_dir(view_dir().join(module)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                found.push((module, name, fs::read_to_string(&path).unwrap()));
            }
        }
    }
    found
}

/// The text of `source`'s declaration `at`, up to its body or its end.
fn signature(source: &str, at: usize) -> &str {
    let rest = &source[at..];
    &rest[..rest.find(['{', ';']).unwrap_or(rest.len())]
}

/// Whether `text` is an element ID, an icon's name or a file's path, not
/// copy.
fn id_like(text: &str) -> bool {
    text.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "-_./".contains(c))
}

/// The string literals of `source`'s code lines, comments left out.
fn literals(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
    {
        let mut rest = line;
        while let Some(open) = rest.find('"') {
            let body = &rest[open + 1..];
            let (mut text, mut escaped, mut end) = (String::new(), false, None);
            for (ix, c) in body.char_indices() {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    end = Some(ix);
                    break;
                }
                text.push(c);
            }
            let Some(end) = end else { break };
            found.push(text);
            rest = &body[end + 1..];
        }
    }
    found
}

/// What the sources declare that the table does not hold, and copy written
/// inline outside `add_repository/copy.rs`.
fn missing(table: &[Line]) -> Vec<String> {
    let known: BTreeSet<(&str, &str)> = table.iter().map(|row| (row.module, row.name)).collect();
    let mut missing = Vec::new();
    for (module, file, source) in sources() {
        for line in source.lines().map(str::trim_start) {
            for declared in ["pub(crate) const ", "pub(super) const ", "const "] {
                let Some(rest) = line.strip_prefix(declared) else {
                    continue;
                };
                let Some((name, tail)) = rest.split_once(':') else {
                    continue;
                };
                if !tail.trim_start().starts_with("&str") {
                    continue;
                }
                // A constant whose value is an element ID is not copy.
                let value = source[source.find(line).unwrap()..]
                    .split('"')
                    .nth(1)
                    .unwrap_or("");
                if id_like(value) {
                    break;
                }
                if declared == "const " {
                    missing.push(format!("{module}::{name} is private but not an ID"));
                } else if !known.contains(&(module, name)) {
                    missing.push(format!("{module}::{name}"));
                }
                break;
            }
        }
        for marker in ["pub(crate) fn ", "pub(super) fn "] {
            for (at, _) in source.match_indices(marker) {
                let signature = signature(&source, at);
                if !["-> String", "-> &'static str", "-> &str"]
                    .iter()
                    .any(|returns| signature.contains(returns))
                {
                    continue;
                }
                let name = signature[marker.len()..].split('(').next().unwrap();
                if !known.contains(&(module, name)) {
                    missing.push(format!("{module}::{name}()"));
                }
            }
        }
        if module == "add_repository" && file != "copy.rs" {
            for text in literals(&source).into_iter().filter(|text| !id_like(text)) {
                missing.push(format!("{file}: the literal {text:?} is not in copy.rs"));
            }
        }
    }
    missing
}

/// P20 (Part F; `06`): every string of the Add sheet and the Unlock dialog
/// is the text Part F (or B6, B10, C2, or today) gives it, in `06`'s
/// vocabulary: sentence case, no `!`, no emoji, no term `06` replaces, and
/// `…` on an action that opens a chooser, a confirmation or a sheet, but
/// for the four named exceptions. The table is checked against the
/// sources.
#[test]
fn frame_copy_uses_the_vocabulary() {
    // The rules' own matchers.
    assert_eq!(not_sentence_case("Add a repository"), Vec::<String>::new());
    assert_eq!(not_sentence_case("Add Repository"), ["Repository"]);
    assert_eq!(
        not_sentence_case("Done. then"),
        ["then (starts a sentence)"]
    );
    assert_eq!(
        not_sentence_case("Fix it in Git, then SSH."),
        Vec::<String>::new()
    );
    assert_eq!(replaced("Select for publication"), ["publication"]);
    assert!(id_like("add-check") && id_like("add_repository_tests.rs"));
    assert!(!id_like("Add repository"));

    let table = table();
    let mut failed: Vec<String> = table
        .iter()
        .flat_map(|row| {
            broken(row)
                .into_iter()
                .map(move |why| format!("{}::{} = {:?} {why}", row.module, row.name, row.value))
        })
        .collect();

    // The exceptions named are exactly the rows excepted, and every action
    // that opens something is one or the other.
    let excepted: BTreeSet<&str> = table
        .iter()
        .filter(|row| row.kind == Kind::Excepted)
        .map(|row| row.name)
        .collect();
    let named: BTreeSet<&str> = EXCEPTIONS.iter().map(|(name, _)| *name).collect();
    if excepted != named {
        failed.push(format!(
            "the rows excepted {excepted:?} are not the named {named:?}"
        ));
    }
    let opens: BTreeSet<&str> = table
        .iter()
        .filter(|row| row.kind == Kind::Opens)
        .map(|row| row.name)
        .collect();
    if opens != BTreeSet::from(["BROWSE", "ADD_OR_CREATE"]) {
        failed.push(format!("the actions with … are {opens:?}"));
    }

    failed.extend(missing(&table));
    eprintln!(
        "P20: {} strings checked; the … exceptions: {EXCEPTIONS:?}",
        table.len()
    );
    assert!(
        failed.is_empty(),
        "P20: {} strings break Part F or 06:\n  {}",
        failed.len(),
        failed.join("\n  ")
    );
}
