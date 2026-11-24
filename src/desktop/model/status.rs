//! The colour a status value is shown in, one mapping for the item header
//! and the ticket list (decision 20).

/// How a known status is tinted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Tint {
    Working,
    Done,
    Draft,
    Blocked,
}

/// The tint of a known status value, by its words. A status the design
/// does not name, "waiting" and "not started" among them, has none.
pub(crate) fn tint(status: &str) -> Option<Tint> {
    match status.trim().to_lowercase().as_str() {
        "open" | "todo" | "to do" | "in progress" | "in-progress" | "in_progress" | "active" => {
            Some(Tint::Working)
        }
        "done" | "closed" | "complete" | "completed" | "approved" | "ready" => Some(Tint::Done),
        "draft" | "review" | "in review" => Some(Tint::Draft),
        "blocked" => Some(Tint::Blocked),
        _ => None,
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
