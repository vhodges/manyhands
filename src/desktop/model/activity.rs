//! Activity's rows (Wave 04 Cycle 03b, Part A): the types of one store of
//! rows, the request rows (S1), the state rows (S2-S14) and the words every
//! row says (Part B). The
//! model has no GPUI dependency and reads no view module.

use time::{OffsetDateTime, UtcOffset};

pub(crate) mod copy;
mod intent_words;
pub(crate) mod lifecycle_lines;
pub(crate) mod requests;
pub(crate) mod rows;
pub(crate) mod states;

/// A row's time and its tooltip's time line (B6): "HH:MM", and
/// "YYYY-MM-DD HH:MM" in the same zone. With no offset both are UTC and the
/// tooltip says so.
#[allow(dead_code)] // Used from Task 5
pub(crate) fn activity_time(at: OffsetDateTime, offset: Option<UtcOffset>) -> (String, String) {
    let local = at.to_offset(offset.unwrap_or(UtcOffset::UTC));
    let clock = format!("{:02}:{:02}", local.hour(), local.minute());
    let date = format!(
        "{:04}-{:02}-{:02}",
        local.year(),
        u8::from(local.month()),
        local.day()
    );
    let zone = if offset.is_none() { " UTC" } else { "" };
    (clock.clone(), format!("{date} {clock}{zone}"))
}

#[cfg(test)]
#[path = "activity_tests.rs"]
mod tests;
