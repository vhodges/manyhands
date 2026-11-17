//! P8's model half: the table of Part C, and a key with no name.

use super::field_name;

#[test]
fn field_name_gives_the_words_of_the_table() {
    for (key, name) in [
        ("id", "ID"),
        ("title", "Title"),
        ("type", "Type"),
        ("status", "Status"),
        ("project", "Project"),
        ("team", "Team"),
        ("closed_at", "Closed at"),
        ("closed_by", "Closed by"),
        ("item_id", "Item"),
        ("parent_id", "Reply to"),
        ("created_at", "Created at"),
        ("created_by", "Created by"),
        // Any other key is the key as written, whatever its case or shape.
        ("priority", "priority"),
        ("Title", "Title"),
        ("estimate_days", "estimate_days"),
        ("", ""),
    ] {
        assert_eq!(field_name(key), name, "{key:?}");
    }
}
