//! What the editor makes of a body when it loads it: the design's recorded
//! load normalization (decision 1). The host turns every line ending into
//! `\n` (N-1); the editor's own load then rewrites `$$` math fences (N-2 to
//! N-4) through `zorite_markdown::syntax::normalize_math_fences`, the pinned
//! 0.9.0's. A body with no `$$` and no `\r` loads byte for byte.

use std::borrow::Cow;

use manyhands::repository::{ItemDto, ItemDtoKind};
use zorite_markdown::syntax::normalize_math_fences;

/// N-1: every `\r\n`, then every remaining `\r`, becomes `\n`. A carriage
/// return in the editor's buffer garbles its rendering (fact 34).
pub(crate) fn crlf_to_lf(body: &str) -> String {
    if !body.contains('\r') {
        return body.to_owned();
    }
    body.replace("\r\n", "\n").replace('\r', "\n")
}

/// The text an editor holds after loading `body`: N-1, then the editor's
/// own math-fence rewrite, which it applies only to a text holding `$$`
/// (`zorite-editor` `normalize_loaded`). Baselines and comparisons use it.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 14.
pub(crate) fn load(body: &str) -> String {
    let text = crlf_to_lf(body);
    if !text.contains("$$") {
        return text;
    }
    match normalize_math_fences(&text) {
        Cow::Owned(rewritten) => rewritten,
        Cow::Borrowed(_) => text,
    }
}

/// `item` as a session's base holds it (R-11): its body through `load`,
/// and a ticket's `deps` in ID order, as the library writes them (R-72).
/// Nothing else changes.
#[cfg_attr(not(test), allow(dead_code))] // Called from Task 14.
pub(crate) fn normalized(item: &ItemDto) -> ItemDto {
    let mut item = item.clone();
    item.body = item.body.as_deref().map(load);
    if item.kind == ItemDtoKind::Ticket {
        item.deps.sort_by(|a, b| a.id.cmp(&b.id));
    }
    item
}

#[cfg(test)]
#[path = "editor_text_tests.rs"]
pub(crate) mod tests;
