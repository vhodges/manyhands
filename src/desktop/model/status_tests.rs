use super::{Tint, tint};

/// One mapping for the list and the item header (decision 20): the
/// header's words, with "ready" done and "waiting" and "not started"
/// untinted.
#[test]
fn statuses_tint_by_their_words() {
    for (status, tint_of) in [
        ("open", Some(Tint::Working)),
        ("In Progress", Some(Tint::Working)),
        ("done", Some(Tint::Done)),
        (" Closed ", Some(Tint::Done)),
        ("ready", Some(Tint::Done)),
        ("Ready", Some(Tint::Done)),
        ("draft", Some(Tint::Draft)),
        ("in review", Some(Tint::Draft)),
        ("blocked", Some(Tint::Blocked)),
        ("waiting", None),
        ("not started", None),
        ("", None),
        ("whatever", None),
    ] {
        assert_eq!(tint(status), tint_of, "{status:?}");
    }
}
