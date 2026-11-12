//! What a repository's state says: its labels, its sentences and the
//! tooltip's lines, each a constant or a formatting function so tests
//! compare them (Cycle 03a, Part B). Sentence case; no text here names a
//! key or a passphrase.

use super::{RepoFacts, RepoState};
use crate::desktop::{
    model::polling::{AttemptTarget, status},
    services::scheduler::SharedCopy,
};

pub(crate) const CANT_OPEN: &str = "Can't open";
pub(crate) const NEEDS_FIXING: &str = "Needs fixing";
pub(crate) const LOCKED: &str = "Locked";
pub(crate) const NEEDS_ATTENTION: &str = "Needs attention";
/// D6's "Checking…", as Settings says it.
pub(crate) const CHECKING: &str = status::CHECKING;
pub(crate) const CANT_REACH: &str = "Can't reach";
/// D6's "Paused", as Settings says it.
pub(crate) const PAUSED: &str = status::PAUSED;
pub(crate) const UP_TO_DATE: &str = "Up to date";
pub(crate) const THIS_COMPUTER: &str = "This computer";

/// "{n} to share".
pub(crate) fn to_share(count: usize) -> String {
    format!("{count} to share")
}

/// The tail of every "can't open" sentence.
pub(crate) const UNCHANGED: &str = "Its files are unchanged. Retry, or remove it from Manyhands.";
pub(crate) const NEEDS_FIXING_SENTENCE: &str = "Its configuration needs fixing:";
/// D6's locked headline with a full stop (fact 54).
pub(crate) const LOCKED_SENTENCE: &str = "Checking is waiting for your key to be unlocked.";
pub(crate) const PAUSED_SENTENCE: &str = "Checking for updates is paused.";
pub(crate) const THIS_COMPUTER_SENTENCE: &str = "No shared copy, so changes stay on this computer.";
/// Where "Not shared yet:" begins.
pub(crate) const NOT_SHARED_YET: &str = "Not shared yet: ";
pub(crate) const SHARE_EACH: &str = "Share each from its tab or from Activity.";
pub(crate) const SHARE_MAIN_SHARES: &str = "Share main shares main.";
/// How main is named among the items not shared yet.
pub(crate) const MAIN_NAME: &str = "main";
/// How an item whose ID did not parse is named: it has no ID to say.
pub(crate) const UNNAMED_ITEM: &str = "an item";
/// How many items a sentence names before "and {n} more".
pub(crate) const NAMED_AT_MOST: usize = 5;

/// "{repo} is no longer registered with Manyhands. …"
pub(crate) fn kept_sentence(repo: &str) -> String {
    format!("{repo} is no longer registered with Manyhands. {UNCHANGED}")
}

/// "Manyhands was busy opening {repo}. …"
pub(crate) fn busy_sentence(repo: &str) -> String {
    format!("Manyhands was busy opening {repo}. Its files are unchanged. Retry.")
}

/// "Can't check {repo} for updates. Retry."
pub(crate) fn unreadable_sentence(repo: &str) -> String {
    format!("Can't check {repo} for updates. Retry.")
}

/// "{n} items need a look. Activity lists them."
pub(crate) fn problems_sentence(count: usize) -> String {
    match count {
        1 => "1 item needs a look. Activity lists them.".to_owned(),
        count => format!("{count} items need a look. Activity lists them."),
    }
}

/// "Checking {repo} for updates…"
pub(crate) fn checking_sentence(repo: &str) -> String {
    format!("Checking {repo} for updates\u{2026}")
}

/// "Checked {ago} · up to date".
pub(crate) fn up_to_date_sentence(ago: &str) -> String {
    format!("Checked {ago} \u{b7} up to date")
}

/// "{remote} isn't an SSH shared copy, so Manyhands can't check or share it."
pub(crate) fn not_ssh_sentence(remote: &str) -> String {
    format!("{remote} isn't an SSH shared copy, so Manyhands can't check or share it.")
}

/// " and {n} more", after the five items a sentence names.
pub(crate) fn and_more(count: usize) -> String {
    format!(" and {count} more")
}

/// The tooltip's second line.
pub(crate) fn folder_line(root: &str) -> String {
    format!("Folder: {root}")
}

/// The tooltip's third line.
pub(crate) fn main_branch_line(branch: &str) -> String {
    format!("Main branch: {branch}")
}

/// The tooltip's fourth line: the library's redacted push location.
pub(crate) fn shared_copy_line(copy: &SharedCopy) -> String {
    format!("Shared copy: {} \u{b7} {}", copy.name, copy.location)
}

/// How an item not shared yet is named: main as "main", an item by its
/// title when it has a kind and its title is known, else by its ID.
fn target_name(facts: &RepoFacts<'_>, target: &AttemptTarget) -> String {
    if target.main {
        return MAIN_NAME.to_owned();
    }
    let Some(id) = &target.item_id else {
        return UNNAMED_ITEM.to_owned();
    };
    match target.kind {
        Some(_) => (facts.titles)(id).unwrap_or_else(|| id.to_string()),
        None => id.to_string(),
    }
}

/// "Not shared yet: a, b and 2 more.", then how to share what is listed.
pub(crate) fn to_share_sentence(facts: &RepoFacts<'_>, items: &[AttemptTarget]) -> Vec<String> {
    let names: Vec<String> = items
        .iter()
        .take(NAMED_AT_MOST)
        .map(|target| target_name(facts, target))
        .collect();
    let mut first = format!("{NOT_SHARED_YET}{}", names.join(", "));
    if items.len() > NAMED_AT_MOST {
        first.push_str(&and_more(items.len() - NAMED_AT_MOST));
    }
    first.push('.');
    let mut sentence = vec![first];
    if items.iter().any(|target| !target.main) {
        sentence.push(SHARE_EACH.to_owned());
    }
    if items.iter().any(|target| target.main) {
        sentence.push(SHARE_MAIN_SHARES.to_owned());
    }
    sentence
}

/// The sentence as one line: the tooltip's first line and the menu's first
/// item. A part followed by another ends in a full stop unless it already
/// ends in terminal punctuation, since D6's statuses carry none.
pub(crate) fn sentence_line(state: &RepoState) -> String {
    let mut line = String::new();
    for (index, part) in state.sentence.iter().enumerate() {
        if index > 0 {
            line.push(' ');
        }
        line.push_str(part);
        let last = index + 1 == state.sentence.len();
        if !last && !part.ends_with(['.', '!', '?', ':', '\u{2026}']) {
            line.push('.');
        }
    }
    line
}
