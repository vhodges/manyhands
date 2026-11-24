//! A part's name in words (D5b design, Part C): the front matter key of a
//! field the sheet's card names, as the details sheet and the header's
//! fields name it.

use std::borrow::Cow;

/// The name the card of the field `key` shows. A key with no name here is
/// shown as written.
pub(crate) fn field_name(key: &str) -> Cow<'static, str> {
    Cow::Borrowed(match key {
        "id" => "ID",
        "title" => "Title",
        "type" => "Type",
        "status" => "Status",
        "project" => "Project",
        "team" => "Team",
        "closed_at" => "Closed at",
        "closed_by" => "Closed by",
        "item_id" => "Item",
        "parent_id" => "Reply to",
        "created_at" => "Created at",
        "created_by" => "Created by",
        other => return Cow::Owned(other.to_owned()),
    })
}

#[cfg(test)]
#[path = "names_tests.rs"]
mod tests;
