//! The discussion's threads and its comment rows' words: threads in the
//! library's order, deeper replies named by whom they answer (R-53), the
//! author without an email, avatar colours (R-56), and the time at a given
//! offset (fact 80).

use std::path::Path;

use manyhands::repository::CommentListDto;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use super::{
    Reply, Thread, avatar_color, avatar_initials, comment_time, display_name, initials, threads,
};
use crate::desktop::{
    model::item::UNKNOWN_AUTHOR,
    test_support::fixtures::{Fixture, item_id},
};

const TICKET: &str = "01ARZ3NDEKTSV4RRFFQ69G5FD0";
const A: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE0";
const B: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE1";
const C: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE2";
const D: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE3";
const E: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE4";
const F: &str = "01ARZ3NDEKTSV4RRFFQ69G5FE5";
const ADA: &str = "Ada Lovelace <ada@example.invalid>";
const BO: &str = "Bo Carter <bo@example.invalid>";
const CY: &str = "Cy";

/// The comments of a ticket that `build` writes, as the library lists them.
fn listed(build: impl FnOnce(&Fixture, &Path)) -> CommentListDto {
    let fixture = Fixture::new();
    let root = fixture.create_repository("a");
    fixture.commit_ticket(&root, TICKET, "Discussed", "");
    build(&fixture, &root);
    fixture.refresh(&root);
    let service = fixture.service();
    let repo = service.resolve_repository(&root).unwrap();
    service.list_comments(&repo, &item_id(TICKET)).unwrap()
}

fn by(author: &str) -> String {
    format!("created_by: {author}\n")
}

fn at(second: u32) -> String {
    format!("2026-09-30T12:00:{second:02}Z")
}

fn reply(ix: usize, replying_to: Option<&str>) -> Reply {
    Reply {
        ix,
        replying_to: replying_to.map(str::to_owned),
    }
}

/// One thread per root, in the library's order, each reply beneath its
/// root; a file that is no comment ends the list as a thread of its own.
#[test]
fn threads_keep_the_librarys_order() {
    let list = listed(|fixture, root| {
        fixture.commit_comment(root, TICKET, A, None, &at(1), &by(ADA));
        fixture.commit_comment(root, TICKET, B, Some(A), &at(2), &by(BO));
        fixture.commit_comment(root, TICKET, C, None, &at(3), "");
        fixture.commit_comment(root, TICKET, D, Some(C), &at(4), "");
        fixture.commit_comment(root, TICKET, E, Some(A), &at(5), "");
        fixture.commit_file(
            root,
            &format!(".manyhands/comments/{TICKET}/notes.md"),
            b"---\nmanyhands_managed: true\n---\n",
        );
    });
    let ids: Vec<Option<&str>> = list.items.iter().map(|c| c.id.as_deref()).collect();
    assert_eq!(ids, [Some(A), Some(B), Some(E), Some(C), Some(D), None]);

    assert_eq!(
        threads(&list),
        [
            Thread {
                root: 0,
                replies: vec![reply(1, None), reply(2, None)],
            },
            Thread {
                root: 3,
                replies: vec![reply(4, None)],
            },
            Thread {
                root: 5,
                replies: Vec::new(),
            },
        ]
    );
}

/// A reply deeper than one level names the author of the comment it
/// answers, without the email; a reply to the root names no one.
#[test]
fn a_deeper_reply_names_who_it_replies_to() {
    let list = listed(|fixture, root| {
        fixture.commit_comment(root, TICKET, A, None, &at(1), &by(ADA));
        fixture.commit_comment(root, TICKET, B, Some(A), &at(2), &by(BO));
        fixture.commit_comment(root, TICKET, C, Some(B), &at(3), &by(CY));
        fixture.commit_comment(root, TICKET, D, Some(C), &at(4), "");
        fixture.commit_comment(root, TICKET, E, Some(D), &at(5), "");
        fixture.commit_comment(root, TICKET, F, Some(A), &at(6), "");
    });
    let depths: Vec<u32> = list.items.iter().map(|c| c.depth).collect();
    assert_eq!(depths, [0, 1, 2, 3, 4, 1]);

    assert_eq!(
        threads(&list),
        [Thread {
            root: 0,
            replies: vec![
                reply(1, None),
                reply(2, Some("Bo Carter")),
                reply(3, Some("Cy")),
                reply(4, Some(UNKNOWN_AUTHOR)),
                reply(5, None),
            ],
        }]
    );
}

/// The name is the `created_by` value without its `<email>`; with no
/// value it is D1's "unknown author".
#[test]
fn display_names_drop_the_email() {
    assert_eq!(display_name(Some(ADA)), "Ada Lovelace");
    assert_eq!(
        display_name(Some("  Ada Lovelace   <ada@x.invalid>  ")),
        "Ada Lovelace"
    );
    assert_eq!(display_name(Some("Cy")), "Cy");
    assert_eq!(display_name(None), UNKNOWN_AUTHOR);
    assert_eq!(display_name(Some("   ")), UNKNOWN_AUTHOR);
    // An address alone is all the name there is.
    assert_eq!(display_name(Some("<ada@x.invalid>")), "ada@x.invalid");
    // Only a trailing address is one.
    assert_eq!(display_name(Some("a <b> c")), "a <b> c");
}

/// The initials are the first letters of the name's first and last words.
#[test]
fn initials_take_the_first_and_last_words() {
    assert_eq!(initials("Ada Lovelace"), "AL");
    assert_eq!(initials("ada byron lovelace"), "AL");
    assert_eq!(initials("Cy"), "C");
    assert_eq!(initials("élodie roux"), "ÉR");
    assert_eq!(initials(""), "");
}

/// An avatar shows the name's initials; with no author, or a blank one,
/// whose name is "unknown author", it shows "?".
#[test]
fn a_blank_or_missing_author_has_no_initials() {
    assert_eq!(avatar_initials(Some(ADA)), "AL");
    assert_eq!(avatar_initials(Some("<ada@x.invalid>")), "A");
    assert_eq!(avatar_initials(None), "?");
    assert_eq!(avatar_initials(Some("")), "?");
    assert_eq!(avatar_initials(Some("   ")), "?");
}

/// WCAG's contrast of `color` against white.
fn contrast_with_white(color: gpui_kit::Hsla) -> f32 {
    let rgb = color.to_rgb();
    let linear = |channel: f32| {
        if channel <= 0.040_45 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b);
    1.05 / (luminance + 0.05)
}

/// A name gives the same colour every time, names spread over several
/// colours, and white initials keep 4.5:1 on every one (R-56).
#[test]
fn avatar_colours_are_stable_and_keep_contrast() {
    assert_eq!(avatar_color(ADA), avatar_color(ADA));
    assert_eq!(avatar_color("Bo Carter"), avatar_color("Bo Carter"));

    let names: Vec<String> = (0..400)
        .map(|n| format!("Person {n}"))
        .chain([ADA, BO, CY, UNKNOWN_AUTHOR, "", "Élodie"].map(str::to_owned))
        .collect();
    let mut hues: Vec<u32> = Vec::new();
    for name in &names {
        let color = avatar_color(name);
        assert_eq!(color.a, 1., "{name}: opaque");
        let contrast = contrast_with_white(color);
        assert!(contrast >= 4.5, "{name}: {contrast} against white");
        let hue = (color.h * 360.).round() as u32;
        if !hues.contains(&hue) {
            hues.push(hue);
        }
    }
    assert!(
        hues.len() > 100,
        "names spread over many hues: {}",
        hues.len()
    );
}

fn time(at: &str) -> OffsetDateTime {
    OffsetDateTime::parse(at, &Rfc3339).unwrap()
}

fn offset(hours: i8) -> UtcOffset {
    UtcOffset::from_hms(hours, 0, 0).unwrap()
}

/// A time on the local day of `now` reads "Today, 15:36"; any other reads
/// "Oct 22, 10:14". Both are at the offset given, never the machine's.
#[test]
fn times_read_today_or_the_date_at_the_given_offset() {
    let now = time("2026-10-22T20:00:00Z");
    assert_eq!(
        comment_time("2026-10-22T13:36:00Z", now, offset(2)),
        "Today, 15:36"
    );
    assert_eq!(
        comment_time("2026-10-20T10:14:00Z", now, UtcOffset::UTC),
        "Oct 20, 10:14"
    );
    // The offset moves the time across midnight, both ways.
    assert_eq!(
        comment_time("2026-10-21T23:30:00Z", now, offset(2)),
        "Today, 01:30"
    );
    assert_eq!(
        comment_time("2026-10-21T23:30:00Z", now, UtcOffset::UTC),
        "Oct 21, 23:30"
    );
    assert_eq!(
        comment_time("2026-10-22T08:14:00Z", now, offset(-10)),
        "Oct 21, 22:14"
    );
    // "Today" is the local day of `now` too.
    assert_eq!(
        comment_time(
            "2026-10-22T08:05:00Z",
            time("2026-10-23T01:00:00Z"),
            offset(-5)
        ),
        "Today, 03:05"
    );
    // A date at the start of the year, one-digit day.
    assert_eq!(
        comment_time("2026-01-02T09:05:00+01:00", now, offset(1)),
        "Jan 2, 09:05"
    );
    // A time that cannot be read is shown as written.
    assert_eq!(comment_time("not a time", now, offset(2)), "not a time");
}
