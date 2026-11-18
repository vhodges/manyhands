//! Link targets (design, Links; decision 10): what a link clicked in a body
//! names, parsed from the text the editor gives, where a path link is
//! looked up, and the nearest of several candidates.
//!
//! **Parse.** The text is trimmed, `<…>` and a trailing title go, the
//! `#fragment` is split off and the scheme found on the raw text, and only
//! then is the path percent-decoded, so an escaped `#` or `:` is part of
//! the path. `http` and `https` open in the browser as written, `mailto`
//! with its query reduced to `subject` and `body`; any other scheme is not
//! opened. A single letter before `:` is a drive letter, not a scheme.
//!
//! **Refused as not found** (`link_path`): a backslash, a drive letter, a
//! leading `//` and a `..` that leaves the repository root. None of them is
//! ever looked up, and no nearest name stands in for them.
//!
//! A target is body text, so nothing here derives `Debug`.

/// What a link names.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum LinkTarget {
    /// `http`, `https` or `mailto`, as it opens in the browser.
    External(String),
    /// Any other scheme: "This kind of link is not opened".
    Unsafe,
    /// A path, percent-decoded: from the repository root when `from_root`,
    /// with the root's `/` removed, and otherwise from the linking item's
    /// folder.
    Path { path: String, from_root: bool },
    /// A `[[Title]]`'s title, its alias and heading removed.
    Title(String),
    /// Nothing to open: a place in the same text, an empty link, or a
    /// block reference.
    Ignored,
}

/// The text the editor gives for a clicked `[text](…)` link, parsed: the
/// raw text up to the first `)`, title and angle brackets included
/// (fact 35).
pub(crate) fn parse_link(raw: &str) -> LinkTarget {
    let destination = destination(raw);
    if let Some(scheme) = scheme(destination) {
        return match scheme.to_ascii_lowercase().as_str() {
            "http" | "https" => LinkTarget::External(destination.to_owned()),
            "mailto" => LinkTarget::External(mailto(destination)),
            _ => LinkTarget::Unsafe,
        };
    }
    let (path, _fragment) = split_fragment(destination);
    let path = percent_decode(path);
    match path.strip_prefix('/') {
        Some(rest) => LinkTarget::Path {
            path: rest.to_owned(),
            from_root: true,
        },
        None if path.is_empty() => LinkTarget::Ignored,
        None => LinkTarget::Path {
            path,
            from_root: false,
        },
    }
}

/// The target the editor gives for a clicked `[[wiki link]]`, parsed. A
/// block reference (`#^id`, `^id`) and its referencers (`refs:`) are
/// ignored; otherwise the alias after `|` and the heading after `#` go.
pub(crate) fn parse_wiki_link(raw: &str) -> LinkTarget {
    let target = raw.trim();
    let target = target
        .strip_prefix("[[")
        .and_then(|inner| inner.strip_suffix("]]"))
        .unwrap_or(target);
    if ["#^", "^", "refs:"]
        .iter()
        .any(|prefix| target.starts_with(prefix))
    {
        return LinkTarget::Ignored;
    }
    let title = target.split('|').next().unwrap_or_default();
    let title = title.split('#').next().unwrap_or_default().trim();
    if title.is_empty() {
        LinkTarget::Ignored
    } else {
        LinkTarget::Title(title.to_owned())
    }
}

/// The repository path a path link names, clicked in an item whose folder
/// is `from_dir`, with `.` and `..` resolved and empty steps dropped.
/// `None` when it is refused as not found: a backslash, a NUL, a drive
/// letter, a leading `//`, a `..` past the root, or nothing left. A refused
/// path is never looked up, and has no nearest-name fallback.
pub(crate) fn link_path(path: &str, from_root: bool, from_dir: &str) -> Option<String> {
    if path.contains(['\\', '\0']) || (from_root && path.starts_with('/')) {
        return None;
    }
    let mut steps: Vec<&str> = if from_root {
        Vec::new()
    } else {
        segments(from_dir).collect()
    };
    for step in path.split('/') {
        match step {
            "" | "." => {}
            ".." => {
                steps.pop()?;
            }
            step if is_drive(step) => return None,
            step => steps.push(step),
        }
    }
    (!steps.is_empty()).then(|| steps.join("/"))
}

/// The folder `path`, an item's path relative to its context, is in: ""
/// at the root.
pub(crate) fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(folder, _)| folder)
}

/// The last step of `path`.
pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}

/// A place a link could open: a repository path, and whether it is in the
/// linking item's own work context.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CandidatePath {
    pub path: String,
    pub in_context: bool,
}

/// The candidate nearest `from_dir`: one in the item's own context first,
/// then the fewest folder steps, up from `from_dir` to the folder both
/// share and down to the candidate's, then path order.
pub(crate) fn nearest<'a>(
    from_dir: &str,
    candidates: &'a [CandidatePath],
) -> Option<&'a CandidatePath> {
    candidates.iter().min_by_key(|candidate| {
        (
            !candidate.in_context,
            folder_steps(from_dir, folder_of(&candidate.path)),
            candidate.path.as_str(),
        )
    })
}

/// The folder steps from `from` to `to`.
fn folder_steps(from: &str, to: &str) -> usize {
    let from: Vec<&str> = segments(from).collect();
    let to: Vec<&str> = segments(to).collect();
    let shared = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    (from.len() - shared) + (to.len() - shared)
}

fn segments(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|step| !step.is_empty())
}

/// A step that starts with a drive letter, `X:`.
fn is_drive(step: &str) -> bool {
    let bytes = step.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// Step 1 of Parse: the destination, trimmed, without `<…>` or a trailing
/// title.
fn destination(raw: &str) -> &str {
    let raw = raw.trim();
    if let Some(inner) = raw.strip_prefix('<') {
        return inner.split_once('>').map_or(inner, |(inside, _)| inside);
    }
    raw.split(char::is_whitespace).next().unwrap_or_default()
}

/// The scheme `text` starts with: `[A-Za-z][A-Za-z0-9+.-]*:`, of two or
/// more characters, since one letter is a drive.
fn scheme(text: &str) -> Option<&str> {
    let (scheme, _) = text.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    let valid = first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'));
    (valid && scheme.len() > 1).then_some(scheme)
}

fn split_fragment(text: &str) -> (&str, Option<&str>) {
    match text.split_once('#') {
        Some((before, fragment)) => (before, Some(fragment)),
        None => (text, None),
    }
}

/// A `mailto` link with no fragment and only its `subject` and `body`
/// fields, in their order.
fn mailto(link: &str) -> String {
    let (link, _fragment) = split_fragment(link);
    let Some((address, query)) = link.split_once('?') else {
        return link.to_owned();
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|field| {
            let name = field.split_once('=').map_or(*field, |(name, _)| name);
            name.eq_ignore_ascii_case("subject") || name.eq_ignore_ascii_case("body")
        })
        .collect();
    if kept.is_empty() {
        address.to_owned()
    } else {
        format!("{address}?{}", kept.join("&"))
    }
}

/// Each `%` and two hex digits becomes its byte; any other `%` stays.
/// Bytes that are not UTF-8 become U+FFFD, which matches no item path.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let escaped = (bytes[at] == b'%')
            .then(|| bytes.get(at + 1..at + 3))
            .flatten()
            .filter(|hex| hex.iter().all(u8::is_ascii_hexdigit))
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                decoded.push(byte);
                at += 3;
            }
            None => {
                decoded.push(bytes[at]);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
