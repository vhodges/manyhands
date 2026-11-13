//! Resume on a lifecycle row (D5 decision 10, Part C; R-22, K-20; A15,
//! P5, P22).
//!
//! L2 lists an unfinished promotion's or closure's parent with `resume`,
//! held by its request or not. Its row offers Resume, which sends `operation
//! resume` of it and opens the lifecycle dialog on that resume: when the
//! answer names the request that holds the lifecycle, the drain sends that
//! close or merge again (`Resends`), and the dialog shows it running. So an
//! item left "Closing…" or "Merging…" after a restart goes on from here.
//!
//! While the item's lifecycle is in flight in this process, its close or
//! merge or a resume of its operation, the row offers no Resume and says it
//! is still running (P22). The row is kept apart from `resume_offer`, which
//! offers no lifecycle row.

use std::path::PathBuf;

use gpui_kit::{
    AnyElement, ClickEvent, Context, FocusHandle, IntoElement, Styled, Window, component::WindowExt,
};
use manyhands::{
    canonical::ItemId,
    repository::{ItemDtoKind, OperationDto, OperationId},
};

// The row moved to the model, where Activity's S9 reads it.
pub(super) use crate::desktop::model::operations::LifecycleRow;

use super::{RESUME, button, scope_name, text};
use crate::desktop::{
    model::{lifecycle_lock::LockKey, stamp::RepoKey},
    services::{
        lifecycle_of,
        requests::{Intent, Lane},
    },
    view::{
        lifecycle::{Heading, STILL_RUNNING, follow_resume, short_id},
        workspace::Workspace,
    },
};

impl Workspace {
    /// The lifecycle row `operation` of `root`, drawn `index`th: Resume, or
    /// that its lifecycle is still running; `None` for any other row.
    pub(super) fn lifecycle_resume(
        &mut self,
        root: &RepoKey,
        operation: &OperationDto,
        index: usize,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let row = LifecycleRow::of(operation)?;
        let key = LockKey::new(root, &row.item);
        let mut held = self
            .services()
            .operations()
            .read(cx)
            .locks()
            .operations(&key);
        held.push(row.operation.clone());
        if self
            .services()
            .worker()
            .in_flight(|intent| lifecycle_of(intent, &key, &held))
        {
            return Some(
                text(("pending-still-running", index), STILL_RUNNING)
                    .text_xs()
                    .into_any_element(),
            );
        }
        let focus = self.control_focus_for(format!("resume:{root}:{}", row.operation), cx);
        let origin = focus.clone();
        let root = root.clone();
        Some(button(
            ("pending-resume", index),
            RESUME,
            Some(&focus),
            cx,
            cx.listener(move |this, _: &ClickEvent, window, cx| {
                this.resume_lifecycle(&root, &row, origin.clone(), window, cx)
            }),
        ))
    }

    /// Resume on the lifecycle row `row` of `root`: its operation resumed,
    /// and the lifecycle dialog on that resume.
    fn resume_lifecycle(
        &mut self,
        root: &RepoKey,
        row: &LifecycleRow,
        origin: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (Ok(operation_id), Ok(id)) = (
            OperationId::parse(&row.operation),
            row.item.parse::<ItemId>(),
        ) else {
            return;
        };
        let key = LockKey::new(root, &row.item);
        let mut held = self
            .services()
            .operations()
            .read(cx)
            .locks()
            .operations(&key);
        held.push(row.operation.clone());
        let services = self.services().clone();
        if services
            .worker()
            .in_flight(|intent| lifecycle_of(intent, &key, &held))
        {
            return;
        }
        let path = PathBuf::from(root);
        let resume = services.worker().submit(
            Intent::OperationResume {
                root: path.clone(),
                operation_id,
            },
            Lane::User,
            None,
            None,
        );
        let intent = match row.kind {
            ItemDtoKind::Ticket => Intent::TicketClose { root: path, id },
            ItemDtoKind::Document => Intent::DocumentPromote { root: path, id },
        };
        let (primary, remote) = self.known(root);
        let heading = Heading {
            kind: row.kind,
            short: short_id(&row.item),
            title: None,
            repository: scope_name(Some(std::path::Path::new(root))),
            branch: None,
            primary,
            remote,
        };
        if !window.has_active_dialog(cx) {
            follow_resume(intent, heading, resume, origin, &services, window, cx);
        }
        cx.notify();
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
#[path = "resume_tests.rs"]
pub(in crate::desktop::view::operations) mod tests;
