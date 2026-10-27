//! The recorded load normalization (decision 1) over the design's golden
//! fixtures in `tests/fixtures/editor/`, and `normalized` (R-11, R-72).
//!
//! `ItemDto` derives `Debug`, which would print a body, so items are
//! compared with `assert!`.

use std::{fs, path::PathBuf};

use manyhands::repository::{
    DependencyDto, DependencyState, IndexState, IndexStateDto, ItemContextDto, ItemContextKind,
    ItemDto, ItemDtoKind,
};

use super::{crlf_to_lf, load, normalized};

/// One fixture per rewrite, each with an `.expected` file.
const REWRITES: [&str; 9] = [
    "n1_crlf",
    "n1_lone_cr",
    "n2_mixed_pair",
    "n2b_padded_pair",
    "n3_attached_fences",
    "n4_inline_code",
    "n4_tilde_fence",
    "n4_indented",
    "n4_html",
];

/// The rewrites of N-1, the host's own part.
const LINE_ENDINGS: [&str; 2] = ["n1_crlf", "n1_lone_cr"];

/// What loads unchanged, each with an `.expected` file of its own bytes.
const EXEMPTIONS: [&str; 5] = [
    "x_backtick_fence",
    "x_table_row",
    "x_prices",
    "x_unclosed_opener",
    "x_lone_pair",
];

/// Bodies the editor round-trips, with no `.expected` file.
const ROUND_TRIPS: [&str; 6] = [
    "unicode",
    "tables",
    "html",
    "extensions",
    "mixed",
    "unknown_yaml_ticket",
];

const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
const DEP_A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB0";
const DEP_B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB1";
const DEP_C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FB2";

/// A body with both a line-ending and a math-fence rewrite.
const RAW_BODY: &str = "First\r\nWhat if words $$E=mc^2$$ more\r\n";

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/editor")
}

fn read(file: &str) -> String {
    fs::read_to_string(directory().join(file)).unwrap_or_else(|_| panic!("{file} is readable"))
}

/// Every fixture's name.
pub(crate) fn every_fixture() -> impl Iterator<Item = &'static str> {
    REWRITES.into_iter().chain(EXEMPTIONS).chain(ROUND_TRIPS)
}

/// The fixture `name`, as stored.
pub(crate) fn fixture(name: &str) -> String {
    read(&format!("{name}.md"))
}

fn expected(name: &str) -> String {
    read(&format!("{name}.expected"))
}

/// The directory holds exactly the listed fixtures, so none goes
/// untested; each rewrite really rewrites, and each loads to its
/// `.expected` file. The host's own part is N-1 alone.
#[test]
fn each_fixture_loads_to_its_expected_file() {
    let mut found: Vec<String> = fs::read_dir(directory())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|file| !file.starts_with('.'))
        .collect();
    found.sort();
    let mut listed: Vec<String> = every_fixture()
        .map(|name| format!("{name}.md"))
        .chain(
            REWRITES
                .into_iter()
                .chain(EXEMPTIONS)
                .map(|name| format!("{name}.expected")),
        )
        .collect();
    listed.sort();
    assert_eq!(found, listed);

    for name in REWRITES {
        assert!(fixture(name) != expected(name), "{name} rewrites nothing");
    }
    for name in REWRITES.into_iter().chain(EXEMPTIONS) {
        assert!(load(&fixture(name)) == expected(name), "{name}");
    }
    for name in every_fixture() {
        let host = if LINE_ENDINGS.contains(&name) {
            expected(name)
        } else {
            fixture(name)
        };
        assert!(crlf_to_lf(&fixture(name)) == host, "{name}: crlf_to_lf");
    }
}

#[test]
fn each_exemption_loads_unchanged() {
    for name in EXEMPTIONS {
        let body = fixture(name);
        assert!(load(&body) == body, "{name}");
        assert!(expected(name) == body, "{name}.expected");
    }
}

#[test]
fn load_is_idempotent() {
    for name in every_fixture() {
        let once = load(&fixture(name));
        assert!(load(&once) == once, "{name}");
    }
}

fn dependency(id: &str, state: DependencyState) -> DependencyDto {
    DependencyDto {
        id: id.to_owned(),
        state,
    }
}

/// An item as `show_item` gives it, with `body`, and `deps` for a ticket.
fn item(kind: ItemDtoKind, body: &str, deps: Vec<DependencyDto>) -> ItemDto {
    let ticket = kind == ItemDtoKind::Ticket;
    ItemDto {
        id: Some(ITEM.to_owned()),
        kind,
        path: "docs/a.md".to_owned(),
        title: Some("A title".to_owned()),
        ticket_type: ticket.then(|| "task".to_owned()),
        status: ticket.then(|| "open".to_owned()),
        project: None,
        team: None,
        closure: None,
        slug: None,
        parent: None,
        deps,
        readiness: None,
        unknown_metadata: serde_json::Map::new(),
        body: Some(body.to_owned()),
        source: Some(format!("---\nid: {ITEM}\n---\n{body}")),
        observation: Some("v1:observed".to_owned()),
        context: ItemContextDto {
            kind: ItemContextKind::Primary,
            branch: Some("main".to_owned()),
            worktree: "/repo".to_owned(),
            head_oid: None,
        },
        changed_at: None,
        change_source: None,
        problems: Vec::new(),
        index: IndexStateDto {
            state: IndexState::Current,
            refreshed_at: None,
        },
    }
}

/// The body goes through `load`; the source, the observation and every
/// other field, a ticket's already sorted `deps` included, stay as read.
#[test]
fn normalized_changes_only_the_body() {
    let sorted = vec![
        dependency(DEP_A, DependencyState::Open),
        dependency(DEP_B, DependencyState::Closed),
    ];
    for read in [
        item(ItemDtoKind::Document, RAW_BODY, Vec::new()),
        item(ItemDtoKind::Ticket, RAW_BODY, sorted),
    ] {
        let mut want = read.clone();
        want.body = Some(load(RAW_BODY));
        assert!(want.body.as_deref() != Some(RAW_BODY));
        assert!(normalized(&read) == want, "{:?}", read.kind);
    }

    let mut unreadable = item(ItemDtoKind::Document, RAW_BODY, Vec::new());
    unreadable.body = None;
    assert!(normalized(&unreadable) == unreadable);
}

/// R-72: a ticket's `deps` in ID order, as the library writes them, each
/// keeping its state.
#[test]
fn normalized_sorts_a_tickets_deps_by_id() {
    let read = item(
        ItemDtoKind::Ticket,
        "Body\n",
        vec![
            dependency(DEP_C, DependencyState::Unresolved),
            dependency(DEP_A, DependencyState::Open),
            dependency(DEP_B, DependencyState::Closed),
        ],
    );
    let mut want = read.clone();
    want.deps = vec![
        dependency(DEP_A, DependencyState::Open),
        dependency(DEP_B, DependencyState::Closed),
        dependency(DEP_C, DependencyState::Unresolved),
    ];
    assert!(normalized(&read) == want);
}
