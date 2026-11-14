//! The Needs attention popover's window tests (Wave 04 Cycle 03b, Part E2):
//! the footer item opens and closes it (`the_popover_opens_lists_and_closes`,
//! P22), each button it can carry runs through `choose_activity`
//! (`popover_actions_by_keyboard`, P23), it works with Activity hidden
//! (`the_popover_works_with_activity_hidden`, P24), each way it closes and
//! where focus goes (`the_popover_closes`, P25), and nothing acts behind a
//! dialog or sheet (`nothing_acts_behind_a_dialog_or_sheet`, P27). The
//! popover is a filtered view of the log's rows, so these tests read the
//! log through the Activity seam and the popover through `support`.

use gpui_kit::TestAppContext;

use crate::desktop::view::{
    activity::tests::list_support::{rows, settle},
    footer::Control,
};
use support::*;

#[path = "needs_attention_tests/actions.rs"]
mod actions;
#[path = "needs_attention_tests/closes.rs"]
mod closes;
#[path = "needs_attention_tests/hidden.rs"]
mod hidden;
#[path = "needs_attention_tests/open.rs"]
mod open;
#[path = "needs_attention_tests/requests.rs"]
mod requests;
#[path = "needs_attention_tests/stale.rs"]
mod stale;
#[path = "needs_attention_tests/states.rs"]
mod states;
#[path = "needs_attention_tests/support.rs"]
pub(in crate::desktop::view) mod support;
