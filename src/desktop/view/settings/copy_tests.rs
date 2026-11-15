//! Settings copy follows `06` (Wave 04 Cycle 04, design Part E; P18).
//!
//! These tests compare constants, as the design says, so they read the
//! words Settings draws and not how it draws them. They live under
//! `repositories` only because `repositories::manage` is private to it.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use super::{
    super::{hosts, keys, page_title, polling, remotes, repositories},
    manage,
};
use crate::desktop::{
    model::{
        polling::{attention as model_attention, status as model_status},
        stamp::SettingsPage,
    },
    services::KEY_STORE_UNAVAILABLE,
    view::{confirm::PINNED_KEY, settings},
};

/// The words `06` replaces: a Settings string uses none of them as a whole
/// word (inflections included).
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

/// The replaced terms `text` uses, whole words only.
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

/// Every Settings copy constant and every copy-producing function of the
/// Settings modules, as `(module, name, value)`, the functions called with
/// plain arguments. The test below fails when a module declares one this
/// table does not hold.
fn table() -> Vec<(&'static str, &'static str, String)> {
    macro_rules! c {
        ($module:ident, $name:ident) => {
            (
                stringify!($module),
                stringify!($name),
                $module::$name.to_owned(),
            )
        };
    }
    let mut rows = vec![
        // mod.rs (module `settings`)
        c!(settings, SETTINGS_HEADING),
        c!(settings, SETTINGS_NOTE),
        c!(settings, REPOSITORIES_PAGE),
        c!(settings, SHARING_AND_KEYS_PAGE),
        c!(settings, CHECKING_PAGE),
        // hosts.rs
        c!(hosts, PINS_HEADING),
        c!(hosts, CHECK_HEADING),
        c!(hosts, READING_PINS),
        c!(hosts, NO_PINS),
        c!(hosts, REAPPROVAL),
        c!(hosts, NO_REPOSITORY),
        c!(hosts, READING_REMOTES),
        c!(hosts, NO_SSH_PUBLICATION),
        c!(hosts, CHECK),
        c!(hosts, CHECK_NOTE),
        c!(hosts, CONNECTING),
        c!(hosts, CANCEL_UNAVAILABLE),
        c!(hosts, NOT_PINNED),
        c!(hosts, MATCHES),
        c!(hosts, DIFFERENT),
        c!(hosts, PINNED),
        c!(hosts, APPROVE),
        c!(hosts, REPLACE),
        // keys.rs
        c!(keys, KEYS_HEADING),
        c!(keys, KEYS_INTRO),
        c!(keys, GENERATE_HEADING),
        c!(keys, IMPORT_HEADING),
        c!(keys, READING_KEYS),
        c!(keys, NO_KEYS),
        c!(keys, KIND_GENERATED),
        c!(keys, KIND_IMPORTED),
        c!(keys, NO_FINGERPRINT),
        c!(keys, SOURCE_MISSING),
        c!(keys, SOURCE_UNAVAILABLE),
        c!(keys, SELECTED),
        c!(keys, SELECTION_NONE),
        c!(keys, COPY),
        c!(keys, COPIED),
        c!(keys, CLEAR),
        c!(keys, UNREGISTER),
        c!(keys, REMOVE_TOOLTIP),
        c!(keys, DELETE),
        c!(keys, MAKE_KEY),
        c!(keys, USE_EXISTING),
        c!(keys, RETRY_FIRST),
        c!(keys, KEYS_NOTE),
        c!(keys, LABEL),
        c!(keys, PASSPHRASE),
        c!(keys, CONFIRMATION),
        c!(keys, GENERATE),
        c!(keys, GENERATE_NOTE),
        c!(keys, NEED_LABEL),
        c!(keys, MISMATCH),
        c!(keys, REFUSED),
        c!(keys, NEED_PASSPHRASE),
        c!(keys, RETRY),
        c!(keys, RETRY_CANCEL),
        c!(keys, RETRY_OFFER),
        c!(keys, PRIVATE_PATH),
        c!(keys, PUBLIC_PATH),
        c!(keys, IMPORT),
        c!(keys, IMPORT_NOTE),
        c!(keys, NEED_PRIVATE),
        // remotes.rs
        c!(remotes, REPOSITORY_HEADING),
        c!(remotes, SHARING_HEADING),
        c!(remotes, ADD_HEADING),
        c!(remotes, NO_REPOSITORY),
        c!(remotes, READING_REMOTES),
        c!(remotes, NO_REMOTES),
        c!(remotes, FETCHES_FROM),
        c!(remotes, PUSHES_TO),
        c!(remotes, ELIGIBLE),
        c!(remotes, NOT_ELIGIBLE),
        c!(remotes, SELECTED),
        c!(remotes, SELECTION_NONE),
        c!(remotes, SELECT),
        c!(remotes, SELECT_NONE),
        c!(remotes, REMOVE),
        c!(remotes, NAME),
        c!(remotes, URL),
        c!(remotes, ADD),
        c!(remotes, ADD_NOTE),
        c!(remotes, NEED_NAME),
        c!(remotes, NEED_URL),
        // polling.rs
        c!(polling, POLLING_HEADING),
        c!(polling, INTRO),
        c!(polling, READING_REPOSITORIES),
        c!(polling, NO_REPOSITORIES),
        c!(polling, CHECK_NOW),
        c!(polling, RETRY_NOW),
        c!(polling, SET_UP_SHARING),
        c!(polling, SHOW_IN_ACTIVITY),
        c!(polling, AUTOMATIC),
        c!(polling, INTERVAL),
        // repositories.rs
        c!(repositories, ADD_HEADING),
        c!(repositories, CREATE_HEADING),
        c!(repositories, FOLDER),
        c!(repositories, CREATE_FOLDER),
        c!(repositories, BROWSE),
        c!(repositories, PRIMARY_BRANCH),
        c!(repositories, PREFIX),
        c!(repositories, NAME),
        c!(repositories, EMAIL),
        c!(repositories, CREATE_IDENTITY),
        c!(repositories, ADD),
        c!(repositories, CREATE),
        c!(repositories, CHECKING),
        c!(repositories, ALREADY_ADDED),
        c!(repositories, NOT_ADDED),
        c!(repositories, CHECKED_OUT),
        c!(repositories, IDENTITY_REQUIRED),
        c!(repositories, NEED_FOLDER),
        c!(repositories, NEED_BRANCH),
        c!(repositories, BROWSE_PROMPT),
        c!(repositories, BROWSE_UNAVAILABLE),
        c!(repositories, BROWSE_NOT_TEXT),
        // repositories/manage.rs
        c!(manage, MANAGED_HEADING),
        c!(manage, NO_REGISTRATIONS),
        c!(manage, READING_REGISTRATIONS),
        c!(manage, MANAGE),
        c!(manage, MANAGING),
        c!(manage, IDENTITY_HEADING),
        c!(manage, READING_IDENTITY),
        c!(manage, NO_IDENTITY),
        c!(manage, NO_INITIALS),
        c!(manage, INITIALS),
        c!(manage, INITIALS_FIELD),
        c!(manage, INITIALS_UNCHANGED),
        c!(manage, INITIALS_CLEAR),
        c!(manage, INITIALS_SET),
        c!(manage, CHOSEN),
        c!(manage, SET_IDENTITY),
        c!(manage, REMOVE),
        c!(manage, REMOVE_NOTE),
    ];
    // The functions, called with plain arguments.
    let plain = [
        ("hosts", "authority_line", hosts::authority_line("h", 22)),
        ("hosts", "key_line", hosts::key_line("alg", "fp")),
        ("hosts", "chosen_line", hosts::chosen_line("/r")),
        ("remotes", "location_line", remotes::location_line("n", "u")),
        ("remotes", "selection_line", remotes::selection_line("n")),
        ("remotes", "chosen_line", remotes::chosen_line("/r")),
        ("keys", "use_key", keys::use_key("k")),
        ("keys", "selection_line", keys::selection_line("k")),
        ("keys", "retrying_line", keys::retrying_line("k")),
        ("manage", "chosen", manage::chosen("c")),
    ];
    rows.extend(plain);
    for page in [
        SettingsPage::Repositories,
        SettingsPage::Remotes,
        SettingsPage::Keys,
        SettingsPage::Hosts,
        SettingsPage::Polling,
    ] {
        rows.push(("settings", "page_title", page_title(page).to_owned()));
    }
    // What the Settings pages draw that other modules define: the Checking
    // page's rows (status words, attention labels, intervals), the key store
    // message the Keys page shows, and the confirmations Settings opens.
    macro_rules! outside {
        ($module:ident, $($name:ident),+ $(,)?) => {
            $(rows.push((stringify!($module), stringify!($name), $module::$name.to_owned()));)+
        };
    }
    outside!(
        model_attention,
        UNLOCK,
        CHOOSE_A_KEY,
        APPROVE_HOST,
        OPEN_REMOTE_SETTINGS,
        RESUME_CHECKING
    );
    outside!(
        model_status,
        NO_SHARED_COPY,
        CHECKING_IS_OFF,
        PAUSED,
        HELD,
        CHECKING,
        CHECKING_SOON,
        WAITING_FOR_OTHER,
        WAITING_FOR_UNFINISHED,
        WAITING_FOR_CONFLICT,
        CANT_CHECK,
        NEEDS_UNLOCKING,
        CHOOSE_A_KEY,
        NEEDS_HOST_APPROVAL,
        NOT_CHECKED_YET,
        RETIRING,
        READING
    );
    for (_, label) in model_status::INTERVALS {
        rows.push(("model_status", "INTERVALS", label.to_owned()));
    }
    rows.push((
        "services",
        "KEY_STORE_UNAVAILABLE",
        KEY_STORE_UNAVAILABLE.to_owned(),
    ));
    rows.push(("confirm", "PINNED_KEY", PINNED_KEY.to_owned()));
    rows
}

fn settings_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/desktop/view/settings")
}

/// The Settings modules' non-test sources, as `(module, text)`: `mod.rs`
/// is `settings`, any other file is its stem.
fn sources(dir: &Path, found: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        if name.ends_with("_tests") || name.ends_with("_support") {
            continue;
        }
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let module = if name == "mod" {
                "settings".to_owned()
            } else {
                name
            };
            found.push((module, fs::read_to_string(&path).unwrap()));
        }
    }
}

/// The text of `source`'s declaration `at`, up to its body or its end.
fn signature(source: &str, at: usize) -> &str {
    let rest = &source[at..];
    &rest[..rest.find(['{', ';']).unwrap_or(rest.len())]
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

/// P18 (Part E; fact 42): no Settings copy constant uses, as a whole word,
/// a term `06` replaces. The table is checked against the sources, so a
/// constant or function added to a Settings module and left out of it
/// fails, and so does a copy string written inline.
#[test]
fn settings_copy_uses_the_vocabulary() {
    // The matcher takes whole words only.
    assert_eq!(replaced("Pinned key"), ["pinned"]);
    assert_eq!(replaced("Select for publication"), ["publication"]);
    assert_eq!(replaced("Shared SSH keys"), Vec::<String>::new());
    assert_eq!(
        replaced("Choose one on the Remotes page."),
        ["remotes page"]
    );
    assert!(replaced("Spinning, mapping and upping").is_empty());

    let table = table();
    let mut stale: Vec<String> = table
        .iter()
        .filter(|(_, _, value)| !replaced(value).is_empty())
        .map(|(module, name, value)| {
            format!("{module}::{name} = {value:?} uses {:?}", replaced(value))
        })
        .collect();

    // Every constant and function the Settings sources declare is in it.
    let mut found = Vec::new();
    sources(&settings_dir(), &mut found);
    let known: BTreeSet<(String, String)> = table
        .iter()
        .map(|(module, name, _)| ((*module).to_owned(), (*name).to_owned()))
        .collect();
    let mut missing = Vec::new();
    for (module, source) in &found {
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
                let id = value
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if declared == "const " {
                    if !id {
                        missing.push(format!(
                            "{module}::{name} is private but not an ID: {value:?}"
                        ));
                    }
                } else if !id && !known.contains(&(module.clone(), name.to_owned())) {
                    missing.push(format!("{module}::{name}"));
                }
                break;
            }
        }
        for marker in ["pub(crate) fn ", "pub(super) fn "] {
            for (at, _) in source.match_indices(marker) {
                let signature = signature(source, at);
                if !["-> String", "-> &'static str", "-> &str"]
                    .iter()
                    .any(|returns| signature.contains(returns))
                {
                    continue;
                }
                let name = signature[marker.len()..].split('(').next().unwrap();
                if !known.contains(&(module.clone(), name.to_owned())) {
                    missing.push(format!("{module}::{name}()"));
                }
            }
        }
        for text in literals(source) {
            let id = text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !id && !replaced(&text).is_empty() {
                stale.push(format!(
                    "{module}: the literal {text:?} uses {:?}",
                    replaced(&text)
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "Settings copy missing from the P18 table: {missing:#?}"
    );
    assert!(
        stale.is_empty(),
        "Settings copy still uses a term 06 replaces: {stale:#?}"
    );
}
