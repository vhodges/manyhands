//! Link parsing and nearest name (design, Links; decision 10), as pure
//! functions. `LinkTarget` holds body text and derives no `Debug`, so it
//! is compared with `assert!`.

use super::{CandidatePath, LinkTarget, link_path, nearest, parse_link, parse_wiki_link};

fn path(path: &str, from_root: bool) -> LinkTarget {
    LinkTarget::Path {
        path: path.to_owned(),
        from_root,
    }
}

fn external(url: &str) -> LinkTarget {
    LinkTarget::External(url.to_owned())
}

/// Where `raw`, clicked in an item in `from_dir`, is looked up: `None`
/// when it is refused. Panics when `raw` is no path link.
fn looked_up(raw: &str, from_dir: &str) -> Option<String> {
    match parse_link(raw) {
        LinkTarget::Path { path, from_root } => link_path(&path, from_root, from_dir),
        _ => panic!("{raw} is a path link"),
    }
}

/// Step 1 of Parse: the angle brackets and a trailing title go, then the
/// fragment, and only then is the path percent-decoded.
#[test]
fn titles_brackets_fragments_and_percent_escapes_are_stripped() {
    let cases = [
        (
            "<docs/My%20Notes.md#Intro> \"The title\"",
            path("docs/My Notes.md", false),
        ),
        ("guides/a%20b.md 'A title'", path("guides/a b.md", false)),
        ("  /docs/x.md (A title)  ", path("docs/x.md", true)),
        ("other.md#a-heading", path("other.md", false)),
        ("<../up one.md>", path("../up one.md", false)),
        ("caf%C3%A9.md", path("café.md", false)),
        // A broken escape stays as written.
        ("100%.md", path("100%.md", false)),
        ("a%zzb.md", path("a%zzb.md", false)),
        ("a%+1b.md", path("a%+1b.md", false)),
        ("a%2", path("a%2", false)),
    ];
    for (raw, expected) in cases {
        assert!(parse_link(raw) == expected, "{raw}");
    }
}

/// Step 2: the fragment is split and the scheme found on the raw text, so
/// an escaped `#` or `:` is part of the path.
#[test]
fn the_fragment_and_scheme_are_split_before_decoding() {
    assert!(parse_link("a%23b.md") == path("a#b.md", false));
    assert!(parse_link("a%3Ab.md") == path("a:b.md", false));
    assert!(parse_link("a%3Ab.md#c%23d") == path("a:b.md", false));
    assert!(parse_link("notes%3Ajavascript.md") == path("notes:javascript.md", false));
    // An escaped scheme is no scheme.
    assert!(parse_link("javascript%3Aalert(1)") == path("javascript:alert(1)", false));
    // A browser link keeps its fragment.
    assert!(parse_link("https://example.com/a#part") == external("https://example.com/a#part"));
}

#[test]
fn schemes_other_than_http_https_mailto_are_unsafe() {
    for raw in [
        "javascript:alert(1)",
        "JavaScript:alert(1)",
        "file:///etc/passwd",
        "data:text/html,<b>x</b>",
        "vbscript:msgbox",
        "ftp://example.com/a",
        "zed://open",
        "obsidian://open?vault=a",
        "x-y.z+w:anything",
        "<file:///etc/passwd> \"Title\"",
    ] {
        assert!(parse_link(raw) == LinkTarget::Unsafe, "{raw}");
    }
    for (raw, url) in [
        ("http://example.com/a", "http://example.com/a"),
        ("https://example.com/a?b=c", "https://example.com/a?b=c"),
        ("HTTPS://EXAMPLE.COM/A", "HTTPS://EXAMPLE.COM/A"),
        (
            "<https://example.com/a b> \"Title\"",
            "https://example.com/a b",
        ),
    ] {
        assert!(parse_link(raw) == external(url), "{raw}");
    }
}

#[test]
fn mailto_keeps_only_subject_and_body() {
    let cases = [
        (
            "mailto:ada@example.invalid?cc=bo@example.invalid&subject=Hi%20there&bcc=x@example.invalid&body=Some%20text&attach=/etc/passwd",
            "mailto:ada@example.invalid?subject=Hi%20there&body=Some%20text",
        ),
        ("mailto:ada@example.invalid", "mailto:ada@example.invalid"),
        (
            "mailto:ada@example.invalid?cc=bo@example.invalid",
            "mailto:ada@example.invalid",
        ),
        (
            "MAILTO:ada@example.invalid?Body=b&SUBJECT=s",
            "MAILTO:ada@example.invalid?Body=b&SUBJECT=s",
        ),
        (
            "mailto:ada@example.invalid?subject=s#fragment",
            "mailto:ada@example.invalid?subject=s",
        ),
    ];
    for (raw, url) in cases {
        assert!(parse_link(raw) == external(url), "{raw}");
    }
}

/// Refused as not found: none of these is ever looked up. A single letter
/// before `:` is a drive letter, not a scheme.
#[test]
fn backslashes_drive_letters_and_double_slashes_are_refused() {
    for raw in [
        "a\\b.md",
        "docs\\a.md",
        "a%5Cb.md",
        "C:/Windows/x.md",
        "c:x.md",
        "C%3A/x.md",
        "sub/D:/x.md",
        "//host/share/x.md",
        "/%2Fhost/x.md",
        "a%00b.md",
    ] {
        let target = parse_link(raw);
        assert!(
            matches!(target, LinkTarget::Path { .. }),
            "{raw} is neither a scheme nor ignored"
        );
        assert!(looked_up(raw, "docs/guides").is_none(), "{raw} is refused");
    }
    // A `/` in the middle is only a separator.
    assert_eq!(
        looked_up("sub//x.md", "docs").as_deref(),
        Some("docs/sub/x.md")
    );
}

/// A `..` that leaves the repository root is refused as not found: it is
/// never looked up, and no nearest name stands in for it. One that stays
/// inside is followed.
#[test]
fn dot_dot_past_the_root_is_not_found_with_no_fallback() {
    for raw in [
        "../../../x.md",
        "/../x.md",
        "../../../docs/x.md",
        "a/../../../../x.md",
    ] {
        assert!(looked_up(raw, "docs/guides").is_none(), "{raw}");
    }
    for (raw, at) in [
        ("../../x.md", "x.md"),
        ("../x.md", "docs/x.md"),
        ("./a/../b.md", "docs/guides/b.md"),
        ("/docs/./x.md", "docs/x.md"),
        ("b.md", "docs/guides/b.md"),
    ] {
        assert_eq!(looked_up(raw, "docs/guides").as_deref(), Some(at), "{raw}");
    }
    // From an item at the root, one step up already leaves it.
    assert!(looked_up("../x.md", "").is_none());
    // A link to the root itself names nothing.
    assert!(looked_up("..", "docs").is_none());
}

fn candidate(path: &str, in_context: bool) -> CandidatePath {
    CandidatePath {
        path: path.to_owned(),
        in_context,
    }
}

fn nearest_path(from_dir: &str, candidates: &[CandidatePath]) -> Option<String> {
    nearest(from_dir, candidates).map(|found| found.path.clone())
}

/// Steps go up from the linking folder to the common folder, then down;
/// equal steps fall to path order. The item's own context's copies come
/// before every other.
#[test]
fn nearest_counts_folder_steps_and_breaks_ties_by_path() {
    let here = candidate("docs/a/b/x.md", false);
    let down = candidate("docs/a/b/c/x.md", false);
    let up = candidate("docs/a/x.md", false);
    let across = candidate("docs/z/x.md", false);
    assert_eq!(
        nearest_path(
            "docs/a/b",
            &[across.clone(), up.clone(), down.clone(), here.clone()]
        )
        .as_deref(),
        Some("docs/a/b/x.md")
    );
    // One step down and one step up tie: path order decides.
    assert_eq!(
        nearest_path("docs/a/b", &[up.clone(), across.clone(), down.clone()]).as_deref(),
        Some("docs/a/b/c/x.md")
    );
    // Up two, then down one, is three steps: more than one.
    assert_eq!(
        nearest_path("docs/a/b", &[across.clone(), up.clone()]).as_deref(),
        Some("docs/a/x.md")
    );
    // From the root, the shallower copy is nearer.
    assert_eq!(
        nearest_path("", &[down.clone(), up.clone()]).as_deref(),
        Some("docs/a/x.md")
    );
    // The item's own context's copy wins, however far.
    assert_eq!(
        nearest_path(
            "docs/a/b",
            &[here.clone(), candidate("docs/z/x.md", true), up.clone()]
        )
        .as_deref(),
        Some("docs/z/x.md")
    );
    assert!(nearest("docs", &[]).is_none());
}

#[test]
fn caret_and_refs_wiki_targets_are_ignored() {
    for raw in [
        "#^abc123",
        "^abc123",
        "refs:^abc123",
        "",
        "  ",
        "|alias",
        "#heading",
    ] {
        assert!(parse_wiki_link(raw) == LinkTarget::Ignored, "{raw:?}");
    }
    for (raw, title) in [
        ("A Title", "A Title"),
        ("A Title|the alias", "A Title"),
        ("A Title#A heading", "A Title"),
        ("A Title#A heading|alias", "A Title"),
        ("[[A Title]]", "A Title"),
        ("  A Title  ", "A Title"),
    ] {
        assert!(
            parse_wiki_link(raw) == LinkTarget::Title(title.to_owned()),
            "{raw}"
        );
    }
}

/// A link to a place in the same text, or to nothing, opens nothing.
#[test]
fn a_fragment_alone_or_an_empty_link_is_ignored() {
    for raw in ["#a-heading", "", "   ", "<>"] {
        assert!(parse_link(raw) == LinkTarget::Ignored, "{raw:?}");
    }
}
