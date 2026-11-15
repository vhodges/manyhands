//! The constants other views share from the repository's detail that
//! Cycle 03a Part F took out of the item area: the Repositories panel
//! (`view::repositories`) and its menu now say what it said.

use manyhands::repository::ConfigurationState;

pub(crate) const RETRY: &str = "Retry";
/// The chosen repository's row says so in text: its mark's label.
pub(crate) const SELECTED: &str = "Selected";
/// The repository menu's Share main: `repo sync`. The words live in the
/// model, where Activity's rows read them (R-41).
pub(crate) use crate::desktop::model::activity::copy::{SHARE_MAIN, SHARING};
/// Share main with no publication remote: it opens Settings › Remotes, as
/// the item header's Share… does.
pub(crate) const SHARE_MAIN_SET_UP: &str = "Share main…";

/// A configuration state in fixed text.
pub(crate) fn configuration_label(state: ConfigurationState) -> &'static str {
    match state {
        ConfigurationState::Valid => "Configuration valid",
        ConfigurationState::Invalid => "Configuration invalid",
        ConfigurationState::Missing => "Configuration missing",
    }
}
