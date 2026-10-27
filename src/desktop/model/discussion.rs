//! The discussion's threads and the words of its comment rows (design,
//! Discussion; `item_view/04-comments.md`).
//!
//! - **Threads.** One per root comment, in the library's order (fact 59),
//!   each with every reply beneath it. A reply deeper than one level names
//!   the author of the comment it answers (R-53). A file that is no
//!   comment, which the library lists after the threads, is a thread of
//!   its own.
//! - **Names.** The `created_by` value without its `<email>`, or D1's
//!   "unknown author".
//! - **Avatars.** Initials, on a colour derived from the name (R-56): a
//!   hue from a stable hash, dark enough that white keeps 4.5:1.
//! - **Times.** "Oct 22, 10:14", or "Today, 15:36" on the day of `now`,
//!   both at the offset the caller passes (fact 80).
//!
//! The avatar's `Hsla` is the one GPUI type here. Nothing here holds a
//! comment's body.

use gpui_kit::{Hsla, hsla};
use manyhands::repository::CommentListDto;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use super::item::UNKNOWN_AUTHOR;

/// A time on the day of `now`, before its clock.
pub(crate) const TODAY: &str = "Today";

/// The months as a comment's time names them.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The avatar's saturation, and the lightness it starts from before it is
/// darkened for contrast.
const AVATAR_SATURATION: f32 = 0.5;
const AVATAR_LIGHTNESS: f32 = 0.42;
/// How much each step darkens it.
const DARKEN: f32 = 0.02;
/// White text's least contrast on an avatar (WCAG AA, R-56).
const MIN_CONTRAST: f32 = 4.5;

/// A root comment and the replies beneath it, as indexes into
/// `CommentListDto.items`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Thread {
    pub root: usize,
    pub replies: Vec<Reply>,
}

/// A reply, and, when it is deeper than one level, the name of the author
/// it answers (R-53).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reply {
    pub ix: usize,
    pub replying_to: Option<String>,
}

/// The threads of `list`, in its order. The library puts each reply right
/// after the comment it answers and that comment's earlier replies, with
/// its depth (fact 59), so the comment a reply answers is the nearest
/// earlier one a level up.
pub(crate) fn threads(list: &CommentListDto) -> Vec<Thread> {
    let mut threads: Vec<Thread> = Vec::new();
    // The names from the root down to the last comment, one per depth.
    let mut names: Vec<String> = Vec::new();
    for (ix, comment) in list.items.iter().enumerate() {
        let depth = usize::try_from(comment.depth).unwrap_or(usize::MAX);
        match threads.last_mut() {
            Some(thread) if depth > 0 => {
                names.truncate(depth);
                let replying_to = if depth > 1 {
                    names.last().cloned()
                } else {
                    None
                };
                thread.replies.push(Reply { ix, replying_to });
            }
            _ => {
                threads.push(Thread {
                    root: ix,
                    replies: Vec::new(),
                });
                names.clear();
            }
        }
        names.push(display_name(comment.author.as_deref()));
    }
    threads
}

/// The name a comment row shows for `author`, its `created_by` value: the
/// value without a trailing `<email>`, the address alone when nothing
/// else is there, and "unknown author" for none.
pub(crate) fn display_name(author: Option<&str>) -> String {
    let Some(author) = author.map(str::trim).filter(|author| !author.is_empty()) else {
        return UNKNOWN_AUTHOR.to_owned();
    };
    if let Some(rest) = author.strip_suffix('>')
        && let Some((name, address)) = rest.rsplit_once('<')
    {
        let (name, address) = (name.trim(), address.trim());
        if !name.is_empty() {
            return name.to_owned();
        }
        if !address.is_empty() {
            return address.to_owned();
        }
    }
    author.to_owned()
}

/// An avatar's initials: the first letters of `name`'s first and last
/// words, in capitals.
pub(crate) fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let last = (words.len() > 1).then(|| words.last()).flatten();
    words
        .first()
        .into_iter()
        .chain(last)
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

/// The avatar's colour for `name` (R-56): the same for the same name, a
/// hue from its FNV-1a hash, darkened until white keeps 4.5:1 on it.
pub(crate) fn avatar_color(name: &str) -> Hsla {
    let hash = name.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    let hue = (hash % 360) as f32 / 360.;
    let mut lightness = AVATAR_LIGHTNESS;
    let mut color = hsla(hue, AVATAR_SATURATION, lightness, 1.);
    while contrast_with_white(color) < MIN_CONTRAST && lightness > DARKEN {
        lightness -= DARKEN;
        color = hsla(hue, AVATAR_SATURATION, lightness, 1.);
    }
    color
}

/// WCAG's contrast ratio of white against `color`.
fn contrast_with_white(color: Hsla) -> f32 {
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

/// A comment's time `at`, an RFC 3339 time, at `offset`: "Today, 15:36"
/// when it falls on the day `now` falls on there, else "Oct 22, 10:14". A
/// time that cannot be read is shown as written.
pub(crate) fn comment_time(at: &str, now: OffsetDateTime, offset: UtcOffset) -> String {
    let Ok(at_time) = OffsetDateTime::parse(at, &Rfc3339) else {
        return at.to_owned();
    };
    let local = at_time.to_offset(offset);
    let clock = format!("{:02}:{:02}", local.hour(), local.minute());
    if local.date() == now.to_offset(offset).date() {
        format!("{TODAY}, {clock}")
    } else {
        let month = MONTHS[usize::from(u8::from(local.month())) - 1];
        format!("{month} {}, {clock}", local.day())
    }
}

#[cfg(test)]
#[path = "discussion_tests.rs"]
mod tests;
