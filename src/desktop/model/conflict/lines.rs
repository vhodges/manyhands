//! A side's lines against Before, for the sheet's columns (handoff 2g;
//! R-27). Lines are matched by their place, not aligned: a line inserted
//! early marks every later line changed. Each line is unchanged, changed
//! (`~`) or added (`+`); a side emptied where Before had lines reads
//! `(empty)`, removed (`-`). A difference only in the trailing newline or
//! in CRLF line endings adds one marked note line. No counts are given
//! (R-3).

/// How a side's line stands against Before's line at its place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mark {
    /// The same as Before's.
    Same,
    /// Before has a different line there.
    Changed,
    /// Before has no line there.
    Added,
    /// Before's lines are gone.
    Removed,
}

impl Mark {
    /// The gutter: blank, `~`, `+` or `-`.
    pub(crate) fn gutter(self) -> char {
        match self {
            Self::Same => ' ',
            Self::Changed => '~',
            Self::Added => '+',
            Self::Removed => '-',
        }
    }
}

/// A side's lines, each with its mark.
pub(crate) type Marked = Vec<(Mark, String)>;

/// `text`'s lines, each without its line ending.
fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

/// `side`'s lines, each marked against Before's line at its place, then
/// a note of a line-ending difference, if any.
pub(crate) fn marked(before: &str, side: &str) -> Marked {
    let was = lines(before);
    let is = lines(side);
    if is.is_empty() {
        return if was.is_empty() {
            Vec::new()
        } else {
            vec![(Mark::Removed, "(empty)".to_owned())]
        };
    }
    let mut marked: Marked = is
        .iter()
        .enumerate()
        .map(|(at, line)| {
            let mark = match was.get(at) {
                Some(old) if old == line => Mark::Same,
                Some(_) => Mark::Changed,
                None => Mark::Added,
            };
            (mark, (*line).to_owned())
        })
        .collect();
    match (before.contains("\r\n"), side.contains("\r\n")) {
        (false, true) => marked.push((Mark::Changed, "(CRLF line endings)".to_owned())),
        (true, false) => marked.push((Mark::Changed, "(LF line endings)".to_owned())),
        _ => {}
    }
    if !before.is_empty() {
        match (before.ends_with('\n'), side.ends_with('\n')) {
            (true, false) => marked.push((Mark::Changed, "(no newline at end)".to_owned())),
            (false, true) => marked.push((Mark::Added, "(newline at end)".to_owned())),
            _ => {}
        }
    }
    marked
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
