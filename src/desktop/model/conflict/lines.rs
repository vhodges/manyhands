//! A side's lines against Before, for the sheet's columns (handoff 2g):
//! each unchanged, changed (`~`) or added (`+`). Lines are compared by
//! their place; no counts are given (R-3).

/// How a side's line stands against Before's line at its place.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Mark {
    /// The same as Before's.
    Same,
    /// Before has a different line there.
    Changed,
    /// Before has no line there.
    Added,
}

impl Mark {
    /// The gutter: blank, `~` or `+`.
    pub(crate) fn gutter(self) -> char {
        match self {
            Self::Same => ' ',
            Self::Changed => '~',
            Self::Added => '+',
        }
    }
}

/// `side`'s lines, each marked against Before's line at its place.
pub(crate) fn marked<'a>(before: &str, side: &'a str) -> Vec<(Mark, &'a str)> {
    let before: Vec<&str> = before.lines().collect();
    side.lines()
        .enumerate()
        .map(|(at, line)| {
            let mark = match before.get(at) {
                Some(was) if *was == line => Mark::Same,
                Some(_) => Mark::Changed,
                None => Mark::Added,
            };
            (mark, line)
        })
        .collect()
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
