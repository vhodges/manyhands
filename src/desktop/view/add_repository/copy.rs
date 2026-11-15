//! The Add repository sheet's words (Wave 04 Cycle 05a design Part F), and
//! those it keeps from the forms it replaced. Each is a constant, or a
//! function that fills in a name the check found.

/// The sheet's header (B5; R-23).
pub(crate) const ADD_TITLE: &str = "Add a repository";
pub(crate) const ADD_DESCRIPTION: &str = "Bring a folder you already work in, or start a new one. Manyhands keeps its tickets and documents in the repository.";
/// The mode control's segments (B5).
pub(crate) const MODE_EXISTING: &str = "Add an existing folder";
pub(crate) const MODE_NEW: &str = "Create a new repository";
/// The footer (B10).
pub(crate) const CANCEL: &str = "Cancel";
pub(crate) const ADD: &str = "Add repository";
pub(crate) const CREATE: &str = "Create repository";
/// Add repository and Create repository while an add of this open is
/// pending (B10; R-27).
pub(crate) const ADDING: &str = "Adding\u{2026}";

/// The folder, its Browse… and the note under it (B6 item 1; R-18).
pub(crate) const FOLDER: &str = "Folder";
pub(crate) const BROWSE: &str = "Browse\u{2026}";
pub(crate) const BROWSE_PROMPT: &str = "Choose";
pub(crate) const CHOOSING: &str = "Choosing a folder\u{2026}";
pub(crate) const BROWSE_UNAVAILABLE: &str =
    "The folder chooser could not be opened. Type the folder's path instead.";
pub(crate) const BROWSE_NOT_TEXT: &str =
    "The chosen folder's path cannot be used as text. Type the folder's path instead.";

/// The folder's checks (B6 item 2).
pub(crate) const CHECKING: &str = "Checking the folder\u{2026}";
pub(crate) const IS_REPOSITORY: &str = "This folder is a Git repository.";
pub(crate) const ALREADY_ADDED: &str = "Already added to Manyhands.";
pub(crate) const NOT_SET_UP: &str = "Manyhands isn't set up here yet. Adding it creates a small settings file and saves it as the first change.";
pub(crate) const ALREADY_SET_UP: &str = "Manyhands is already set up here.";
pub(crate) const WILL_BE_ADDED: &str = "It will be added to this computer.";
pub(crate) const NEEDS_FIXING: &str = "Manyhands' settings here need fixing.";
pub(crate) const NO_BRANCH: &str = "No branch is checked out. Check one out, then check again.";
pub(crate) const NOT_REPOSITORY: &str = "Not a Git repository.";
pub(crate) const BUSY: &str = "Manyhands is busy. Try the check again in a moment.";
pub(crate) const NOT_TEXT: &str = "This folder's name can't be used here. Choose another.";

/// B6's row for a folder set up with another main branch than the one
/// checked out.
pub(crate) fn other_main_branch(primary: &str) -> String {
    format!(
        "Manyhands here uses {primary} as its main branch. Check out {primary}, then check again."
    )
}

/// B6's row for a folder set up to share through a remote that is not an
/// SSH address.
pub(crate) fn not_ssh_remote(remote: &str) -> String {
    format!(
        "Manyhands here shares through {remote}, which isn't an SSH address. Fix it in the folder's Git settings, then check again."
    )
}

/// The checks' actions (B6).
pub(crate) const CREATE_HERE: &str = "Create one here";
pub(crate) const CHOOSE_ANOTHER: &str = "Choose another";
pub(crate) const RETRY: &str = "Retry";
pub(crate) const CHECK_AGAIN: &str = "Check again";
pub(crate) const OPEN_SETTINGS: &str = "Open its settings";

/// The main branch (B7; decision 10).
pub(crate) const MAIN_BRANCH: &str = "Main branch";
pub(crate) const MAIN_BRANCH_HELP: &str =
    "The version everyone shares. Finished tickets and approved documents are merged here.";
pub(crate) const MAIN_BRANCH_TIP: &str =
    "Git: the primary branch used for promotion and ticket closure";
pub(crate) const SUGGESTED: &str = "Suggested";
pub(crate) const CHECK_OUT_FIRST: &str =
    "Check it out first. Manyhands starts from the branch you have checked out.";

/// The rest of the existing-folder mode (B6 items 5 to 7).
pub(crate) const PREFIX: &str = "Short code prefix (optional)";
pub(crate) const IDENTITY_HEADING: &str = "Your name & email";
pub(crate) const IDENTITY_REQUIRED: &str =
    "Git has no name and email for this repository. Enter the ones its commits are to carry.";
pub(crate) const REMOVE_NOTE: &str =
    "Removing a repository from Manyhands later never deletes, moves or changes its files.";

/// Create a new repository (B8): its Name and Location, the line that says
/// where the folder will be made, and the identity note kept from today's
/// form.
pub(crate) const NAME_FIELD: &str = "Name";
pub(crate) const LOCATION: &str = "Location";
pub(crate) const CREATE_IDENTITY: &str =
    "The name and email its commits are to carry. Leave both empty to use the ones Git finds.";

/// B8's line under Location: where the empty folder will be made, `path`
/// being Location joined with Name as typed (R-13).
pub(crate) fn will_create(path: &str) -> String {
    format!("Empty folder will be created at {path}")
}
