//! P20's table (`vocabulary`): every string the Add repository sheet and
//! the Unlock key dialog draw, as constants and formatting functions, with
//! the text each must be, character for character, and how `06`'s ellipsis
//! rule reads it. Part F's rows; the lines B6, B10 and C2 give verbatim that
//! Part F's table leaves out; and what Part F keeps from today, as
//! `origin/main` has it.

use crate::desktop::view::{add_repository, passphrase, repositories, settings};

/// How `06`'s ellipsis rule reads a string (Part F: "`…` on an action that
/// opens a chooser, confirmation or sheet").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    /// An action that opens a chooser, a confirmation or a sheet: it ends
    /// with `…`.
    Opens,
    /// An action that opens one, drawn without `…`: one of `EXCEPTIONS`.
    Excepted,
    /// An action that opens nothing: no `…`.
    Acts,
    /// Not an action (a title, a label, a line, a note, a state): a `…` only
    /// at its end, as a working state ends.
    Words,
}

/// The actions that open a chooser, a confirmation or a sheet and are drawn
/// without `…`, and why.
///
/// `ADD` and `CREATE` are the design's own exception (P20): `05` draws "Add
/// repository" and "Create repository" without it. `CHOOSE_ANOTHER` and
/// `OPEN_SETTINGS` are Ruling 21's (orchestrator): Part F, the approved copy
/// contract, gives them verbatim without it, so they are flagged here for
/// the owner, not changed. `06` itself asks for `…` only on an action that
/// opens a confirmation or a sheet: Open its settings opens the Settings
/// sheet, so it is a Part F against `06` inconsistency; Choose another opens
/// the folder chooser, which only Part F's restatement of the rule (and
/// R-18, for Browse…) names.
pub(super) const EXCEPTIONS: [(&str, &str); 4] = [
    ("ADD", "05 draws Add repository without it (design P20)"),
    (
        "CREATE",
        "05 draws Create repository without it (design P20)",
    ),
    (
        "CHOOSE_ANOTHER",
        "Part F verbatim (Ruling 21); opens the chooser, which 06 does not name",
    ),
    (
        "OPEN_SETTINGS",
        "Part F verbatim (Ruling 21); opens the Settings sheet, against 06",
    ),
];

/// One row: where the string is declared, its name, its value, the text it
/// must be, and how the ellipsis rule reads it.
pub(super) struct Line {
    pub(super) module: &'static str,
    pub(super) name: &'static str,
    pub(super) value: String,
    pub(super) wanted: &'static str,
    pub(super) kind: Kind,
}

/// Every string of the sheet and the dialog, with the text each must be.
pub(super) fn table() -> Vec<Line> {
    macro_rules! c {
        ($module:ident :: $name:ident, $kind:ident, $wanted:expr) => {
            Line {
                module: stringify!($module),
                name: stringify!($name),
                value: $module::$name.to_owned(),
                wanted: $wanted,
                kind: Kind::$kind,
            }
        };
    }
    macro_rules! f {
        ($module:ident :: $name:ident ($arg:expr), $wanted:expr) => {
            Line {
                module: stringify!($module),
                name: stringify!($name),
                value: $module::$name($arg),
                wanted: $wanted,
                kind: Kind::Words,
            }
        };
    }
    vec![
        // Part F, the Add sheet.
        c!(add_repository::ADD_TITLE, Words, "Add a repository"),
        c!(
            add_repository::ADD_DESCRIPTION,
            Words,
            "Bring a folder you already work in, or start a new one. Manyhands keeps its tickets and documents in the repository."
        ),
        c!(
            add_repository::MODE_EXISTING,
            Acts,
            "Add an existing folder"
        ),
        c!(add_repository::MODE_NEW, Acts, "Create a new repository"),
        c!(add_repository::FOLDER, Words, "Folder"),
        c!(add_repository::BROWSE, Opens, "Browse\u{2026}"),
        c!(add_repository::CHOOSING, Words, "Choosing a folder\u{2026}"),
        c!(
            add_repository::CHECKING,
            Words,
            "Checking the folder\u{2026}"
        ),
        c!(
            add_repository::IS_REPOSITORY,
            Words,
            "This folder is a Git repository."
        ),
        c!(
            add_repository::ALREADY_ADDED,
            Words,
            "Already added to Manyhands."
        ),
        c!(
            add_repository::NOT_SET_UP,
            Words,
            "Manyhands isn't set up here yet. Adding it creates a small settings file and saves it as the first change."
        ),
        c!(
            add_repository::ALREADY_SET_UP,
            Words,
            "Manyhands is already set up here."
        ),
        c!(
            add_repository::WILL_BE_ADDED,
            Words,
            "It will be added to this computer."
        ),
        c!(
            add_repository::NEEDS_FIXING,
            Words,
            "Manyhands' settings here need fixing."
        ),
        c!(
            add_repository::NO_BRANCH,
            Words,
            "No branch is checked out. Check one out, then check again."
        ),
        c!(
            add_repository::NOT_REPOSITORY,
            Words,
            "Not a Git repository."
        ),
        c!(
            add_repository::BUSY,
            Words,
            "Manyhands is busy. Try the check again in a moment."
        ),
        c!(
            add_repository::NOT_TEXT,
            Words,
            "This folder's name can't be used here. Choose another."
        ),
        c!(add_repository::CREATE_HERE, Acts, "Create one here"),
        c!(add_repository::CHOOSE_ANOTHER, Excepted, "Choose another"),
        c!(add_repository::RETRY, Acts, "Retry"),
        c!(add_repository::CHECK_AGAIN, Acts, "Check again"),
        c!(add_repository::OPEN_SETTINGS, Excepted, "Open its settings"),
        c!(add_repository::MAIN_BRANCH, Words, "Main branch"),
        c!(
            add_repository::MAIN_BRANCH_HELP,
            Words,
            "The version everyone shares. Finished tickets and approved documents are merged here."
        ),
        c!(
            add_repository::MAIN_BRANCH_TIP,
            Words,
            "Git: the primary branch used for promotion and ticket closure"
        ),
        c!(add_repository::SUGGESTED, Words, "Suggested"),
        c!(
            add_repository::CHECK_OUT_FIRST,
            Words,
            "Check it out first. Manyhands starts from the branch you have checked out."
        ),
        c!(
            add_repository::SHARE_THROUGH,
            Words,
            "Share changes through"
        ),
        c!(
            add_repository::SHARE_HELP,
            Words,
            "You can also choose \"Keep on this computer\" and set this up later. Only SSH addresses can be used for sharing."
        ),
        c!(add_repository::KEEP_LOCAL, Acts, "Keep on this computer"),
        c!(
            add_repository::SSH_ONLY,
            Words,
            "Only SSH addresses can be used for sharing."
        ),
        c!(add_repository::IDENTITY_HEADING, Words, "Your name & email"),
        c!(
            add_repository::REMOVE_NOTE,
            Words,
            "Removing a repository from Manyhands later never deletes, moves or changes its files."
        ),
        c!(add_repository::NAME_FIELD, Words, "Name"),
        c!(add_repository::LOCATION, Words, "Location"),
        f!(
            add_repository::will_create("/home/me/new"),
            "Empty folder will be created at /home/me/new"
        ),
        c!(add_repository::CANCEL, Acts, "Cancel"),
        c!(add_repository::ADD, Excepted, "Add repository"),
        c!(add_repository::CREATE, Excepted, "Create repository"),
        // Verbatim from B10 and R-27, B6 and C2 (R-30), which Part F's table
        // leaves out.
        c!(add_repository::ADDING, Words, "Adding\u{2026}"),
        f!(
            add_repository::other_main_branch("trunk"),
            "Manyhands here uses trunk as its main branch. Check out trunk, then check again."
        ),
        f!(
            add_repository::not_ssh_remote("origin"),
            "Manyhands here shares through origin, which isn't an SSH address. Fix it in the folder's Git settings, then check again."
        ),
        c!(
            add_repository::SHARE_NEEDS_IDENTITY,
            Words,
            "Set your name and email in Settings first, then choose how it shares."
        ),
        // Kept from today, as `origin/main` has them.
        c!(
            add_repository::IDENTITY_REQUIRED,
            Words,
            "Git has no name and email for this repository. Enter the ones its commits are to carry."
        ),
        c!(
            add_repository::CREATE_IDENTITY,
            Words,
            "The name and email its commits are to carry. Leave both empty to use the ones Git finds."
        ),
        c!(
            add_repository::PREFIX,
            Words,
            "Short code prefix (optional)"
        ),
        c!(settings::NAME, Words, "Name"),
        c!(settings::EMAIL, Words, "Email"),
        c!(
            add_repository::BROWSE_UNAVAILABLE,
            Words,
            "The folder chooser could not be opened. Type the folder's path instead."
        ),
        c!(
            add_repository::BROWSE_NOT_TEXT,
            Words,
            "The chosen folder's path cannot be used as text. Type the folder's path instead."
        ),
        // The platform chooser's own button: it takes the choice.
        c!(add_repository::BROWSE_PROMPT, Acts, "Choose"),
        // Part F, Settings.
        c!(
            repositories::ADD_OR_CREATE,
            Opens,
            "Add or create a repository\u{2026}"
        ),
        // Part F, the Unlock key dialog, and what it keeps from today.
        c!(passphrase::TITLE, Words, "Unlock your key"),
        c!(passphrase::PASSPHRASE, Words, "Passphrase"),
        // The field's accessible name, which names the key (owner Ruling 26).
        f!(
            passphrase::passphrase_for("primary"),
            "Passphrase for primary"
        ),
        c!(
            passphrase::NOT_NOW_NOTE,
            Words,
            "Kept in memory only, never written to disk. If you choose Not now, you can keep working and unlock later from Needs attention."
        ),
        c!(
            passphrase::REFUSED,
            Words,
            "Enter the passphrase. It cannot be empty or contain a NUL character."
        ),
        c!(passphrase::UNLOCK, Acts, "Unlock"),
        c!(passphrase::CANCEL, Acts, "Not now"),
        f!(
            passphrase::waiting_subtitle("a"),
            "Sharing a is waiting for your passphrase."
        ),
    ]
}
