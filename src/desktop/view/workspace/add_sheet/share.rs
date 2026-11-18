//! The share an add sends once it has added the repository (Wave 04 Cycle
//! 05a design C2; decisions 3, 12 and 17; R-29; K-10, K-14): today's
//! remote selection, the request Settings › Repositories › Sharing makes,
//! submitted for the root the library registered.
//!
//! The pending add lives in the workspace, so its share is sent also when
//! the sheet closed while the add ran, and for an earlier open of the
//! sheet (R-26). Nothing is sent once the app is finishing: a share not yet
//! sent then is lost, as is one waiting for its add when the app quits
//! (K-10). A selection that fails is answered by its own Activity row, and
//! the repository stays added (K-14).

use std::path::Path;

use crate::desktop::{
    services::requests::{Intent, Lane},
    view::workspace::Workspace,
};

impl Workspace {
    /// `share`, the choice an add of `root` was confirmed with, sent as a
    /// user job: a remote's name, or `None` to keep changes on this
    /// computer. Nothing for none, or once quitting has begun.
    pub(super) fn send_share(&self, share: Option<Option<String>>, root: &Path) {
        let Some(name) = share else {
            return;
        };
        let worker = self.services.worker();
        if worker.quitting() {
            return;
        }
        let intent = Intent::RemoteSelect {
            root: root.to_owned(),
            name,
        };
        debug_assert!(!intent.confirmed());
        worker.submit(intent, Lane::User, None, None);
    }
}
