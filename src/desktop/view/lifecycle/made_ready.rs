//! Made ready (D5 design, A3, decision 6; ruling R-20): the tickets a
//! closure made ready, drawn on any ended closure that reached main, Done or
//! Failed.
//!
//! - **`current`.** "Now ready:", then each ticket by its short code, or its
//!   short ID with none (R-19), and its title. The library's report holds no
//!   title, so the dialog reads each one (`show_item`) on its own
//!   generations; a ticket that cannot be read, deleted meanwhile, shows
//!   without one (K-8).
//! - **`index_pending`.** The list, and "The list may be incomplete until
//!   indexing finishes.": the library sends it with a Failed
//!   `discovery_pending` or `push_rejected` answer.
//! - **`index_unavailable_final`.** "Could not work out which tickets are
//!   now ready.", with Open ready tickets, which selects the repository and
//!   lists its open, ready tickets.
//! - **`not_computed`, `not_applicable`**, and any promotion: nothing.
//!
//! Titles are shown in the dialog only; the operation area's entry names
//! none. The titles held derive no `Debug`.

use std::{collections::HashMap, path::Path};

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Styled, Task,
    component::{ActiveTheme, v_flex},
};
use manyhands::{
    canonical::ItemId,
    repository::{ItemDtoKind, LifecycleMadeReadyDto, LifecycleMadeReadyState, MutationDataDto},
    results::Envelope,
};

use super::{LifecycleDialog, Phase, button, outcome::lifecycle_data, short_id, text};
use crate::desktop::{
    model::stamp::ViewScope,
    services::{ReadKind, ReadResult, requests::DesktopRequestId},
};

pub(crate) const MADE_READY_LEAD: &str = "Now ready:";
pub(crate) const MAY_BE_INCOMPLETE: &str = "The list may be incomplete until indexing finishes.";
pub(crate) const COULD_NOT_WORK_OUT: &str = "Could not work out which tickets are now ready.";
pub(crate) const OPEN_READY_TICKETS: &str = "Open ready tickets";

/// What an ended answer's made-ready part shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum MadeReady<'a> {
    Nothing,
    /// The tickets made ready; `incomplete` while the index is pending.
    Listed {
        tickets: &'a [LifecycleMadeReadyDto],
        incomplete: bool,
    },
    /// None could be worked out, and none will be.
    Unknown,
}

impl<'a> MadeReady<'a> {
    /// By the answer's `made_ready_state` (R-20): only a closure's, and only
    /// one that reached main, says anything.
    pub(crate) fn of(envelope: &'a Envelope<MutationDataDto>) -> Self {
        let Some(data) = lifecycle_data(envelope) else {
            return Self::Nothing;
        };
        if data.kind != ItemDtoKind::Ticket {
            return Self::Nothing;
        }
        let tickets = data.made_ready.as_deref().unwrap_or_default();
        match data.made_ready_state {
            Some(LifecycleMadeReadyState::Current) => Self::Listed {
                tickets,
                incomplete: false,
            },
            Some(LifecycleMadeReadyState::IndexPending) => Self::Listed {
                tickets,
                incomplete: true,
            },
            Some(LifecycleMadeReadyState::IndexUnavailableFinal) => Self::Unknown,
            Some(LifecycleMadeReadyState::NotComputed | LifecycleMadeReadyState::NotApplicable)
            | None => Self::Nothing,
        }
    }
}

/// A made-ready ticket's row: its short code, or short ID, and its title
/// when one was read.
pub(crate) fn row(ticket: &LifecycleMadeReadyDto, title: Option<&str>) -> String {
    let short = ticket.slug.clone().unwrap_or_else(|| short_id(&ticket.id));
    match title {
        Some(title) => format!("{short} \u{b7} {title}"),
        None => short,
    }
}

/// The titles of the tickets the shown run made ready, read for the
/// dialog. Derives no `Debug`: it holds titles.
#[derive(Default)]
pub(super) struct Titles {
    /// The run and the tickets whose titles were asked for.
    asked: Option<(DesktopRequestId, Vec<String>)>,
    /// Each ticket's title by ID, once read; `None` for one not read.
    read: HashMap<String, Option<String>>,
    task: Option<Task<()>>,
}

impl LifecycleDialog {
    /// Reads the titles of the tickets the shown run made ready, once its
    /// answer lists any not asked for yet.
    #[allow(clippy::result_large_err)]
    pub(super) fn read_made_ready(&mut self, cx: &mut Context<Self>) {
        let Phase::Submitted(shown) = self.phase else {
            return;
        };
        let ids: Vec<String> = {
            let operations = self.services.operations().read(cx);
            match operations.lifecycle_answer(shown).map(MadeReady::of) {
                Some(MadeReady::Listed { tickets, .. }) => {
                    tickets.iter().map(|ticket| ticket.id.clone()).collect()
                }
                _ => return,
            }
        };
        let asking = (shown, ids);
        if asking.1.is_empty() || self.titles.asked.as_ref() == Some(&asking) {
            return;
        }
        let Some(root) = self.root().map(Path::to_path_buf) else {
            return;
        };
        let ids = asking.1.clone();
        self.titles.asked = Some(asking);
        self.titles.read.clear();
        let stamp = self.generations.next(ViewScope::MadeReady);
        self.titles.task = Some(self.services.read_job(
            stamp,
            move |reader| async move {
                let repo = match reader
                    .call(ReadKind::MadeReady, |service| {
                        service.resolve_repository(&root)
                    })
                    .await
                {
                    ReadResult::Ok(repo) => repo,
                    _ => return Vec::new(),
                };
                let mut titles = Vec::with_capacity(ids.len());
                for id in ids {
                    let title = match id.parse::<ItemId>() {
                        Ok(item) => match reader
                            .call(ReadKind::MadeReady, |service| {
                                service.show_item(&repo, &item)
                            })
                            .await
                        {
                            ReadResult::Ok(item) => item.title,
                            // Deleted, or unreadable: shown without (K-8).
                            _ => None,
                        },
                        Err(_) => None,
                    };
                    titles.push((id, title));
                }
                titles
            },
            cx,
            |dialog: &mut Self, stamp, titles, cx| {
                if dialog.generations.is_current(&stamp) {
                    dialog.titles.read = titles.into_iter().collect();
                    cx.notify();
                }
            },
        ));
    }

    /// The made-ready part of the ended answer `envelope`, and Open ready
    /// tickets where it is offered.
    pub(super) fn made_ready_part(
        &self,
        envelope: &Envelope<MutationDataDto>,
        cx: &Context<Self>,
    ) -> (Option<AnyElement>, Option<AnyElement>) {
        let muted = cx.theme().muted_foreground;
        match MadeReady::of(envelope) {
            MadeReady::Nothing => (None, None),
            MadeReady::Listed {
                tickets,
                incomplete,
            } => {
                if tickets.is_empty() && !incomplete {
                    return (None, None);
                }
                let mut part = v_flex().gap_1();
                if !tickets.is_empty() {
                    part = part
                        .child(text("lifecycle-made-ready-lead", MADE_READY_LEAD).text_sm())
                        .children(tickets.iter().enumerate().map(|(ix, ticket)| {
                            let title = self.titles.read.get(&ticket.id).cloned().flatten();
                            text(("lifecycle-made-ready", ix), row(ticket, title.as_deref()))
                                .text_sm()
                        }));
                }
                if incomplete {
                    part = part.child(
                        text("lifecycle-made-ready-incomplete", MAY_BE_INCOMPLETE)
                            .text_sm()
                            .text_color(muted),
                    );
                }
                (Some(part.into_any_element()), None)
            }
            MadeReady::Unknown => {
                let part = text("lifecycle-made-ready-unknown", COULD_NOT_WORK_OUT)
                    .text_sm()
                    .into_any_element();
                let go = self.on(cx, |dialog, window, cx| {
                    if let Some(root) = dialog.root().map(Path::to_path_buf) {
                        dialog.in_workspace(window, cx, move |workspace, window, cx| {
                            workspace.open_ready_tickets(root, window, cx)
                        });
                    }
                });
                (
                    Some(part),
                    Some(button(
                        "lifecycle-open-ready",
                        OPEN_READY_TICKETS,
                        false,
                        go,
                    )),
                )
            }
        }
    }
}

#[cfg(all(test, feature = "desktop-test-support"))]
impl LifecycleDialog {
    /// The made-ready titles read so far, by ticket ID, in ID order: `None`
    /// for one read without a title.
    pub(crate) fn made_ready_titles(&self) -> Vec<(String, Option<String>)> {
        let mut titles: Vec<_> = self
            .titles
            .read
            .iter()
            .map(|(id, title)| (id.clone(), title.clone()))
            .collect();
        titles.sort();
        titles
    }
}
