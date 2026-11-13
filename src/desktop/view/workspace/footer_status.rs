//! What the footer's status items read of the repositories (Part G): Update
//! check's text from the schedule and the worker, To share's total and the
//! repository it chooses next, and where focus goes when one of them hides.
//!
//! Both items read every listed repository, whatever its row shows (R-24);
//! the kept row has no schedule row of its own here and is never chosen
//! (R-39).

use std::path::Path;

use gpui_kit::{App, Context, Window, component::WindowExt};
use time::OffsetDateTime;

use super::Workspace;
use crate::desktop::{
    model::stamp::RepoKey,
    view::{
        footer::{
            Control, FooterStatus, Plan, focus_after_hiding, next_to_share, to_share_name,
            update_text,
        },
        repositories::checking_now,
    },
};

/// One listed repository, as the footer names it.
struct Listed {
    key: RepoKey,
    name: String,
    /// The folder it sits in: told apart from another of the same name.
    parent: Option<String>,
    /// Its unshared list's items (R-24).
    items: usize,
}

/// The listed repositories as the footer reads them, at one moment.
pub(super) struct ShareFacts {
    listed: Vec<Listed>,
    /// The chosen repository: listed, or the kept row.
    chosen: Option<RepoKey>,
    checking: bool,
    latest: Option<OffsetDateTime>,
    now: OffsetDateTime,
}

impl ShareFacts {
    fn order(&self) -> Vec<(RepoKey, usize)> {
        self.listed
            .iter()
            .map(|each| (each.key.clone(), each.items))
            .collect()
    }

    fn total(&self) -> usize {
        self.listed.iter().map(|each| each.items).sum()
    }

    /// The repository To share chooses now (R-39).
    pub(super) fn next(&self) -> Option<RepoKey> {
        next_to_share(&self.order(), self.chosen.as_ref())
    }

    /// `key`'s name, with its parent folder when another listed repository
    /// has the same name.
    fn display(&self, key: &RepoKey) -> String {
        let Some(this) = self.listed.iter().find(|each| &each.key == key) else {
            return key.clone();
        };
        let shared = self
            .listed
            .iter()
            .filter(|each| each.name == this.name)
            .count();
        match (&this.parent, shared > 1) {
            (Some(parent), true) => format!("{} in {parent}", this.name),
            _ => this.name.clone(),
        }
    }

    pub(super) fn status(&self, attention: usize) -> FooterStatus {
        FooterStatus {
            update: update_text(self.checking, self.latest, self.now),
            to_share: self.total(),
            attention,
        }
    }

    /// To share's name: what a press chooses, or the only repository with
    /// work when it is the chosen one.
    pub(super) fn to_share_name(&self) -> String {
        let total = self.total();
        match (self.next(), &self.chosen) {
            (Some(next), _) => to_share_name(total, &self.display(&next), false),
            (None, Some(chosen)) => to_share_name(total, &self.display(chosen), true),
            (None, None) => to_share_name(total, "", true),
        }
    }
}

impl Workspace {
    /// What the status items read, now: checking is this process's own Check
    /// now or a scheduled check in flight (R-26); the latest check is the
    /// latest completed one of any listed repository; the total is the sum
    /// of their unshared lists' items (R-24).
    pub(super) fn share_facts(&self, cx: &App) -> ShareFacts {
        let services = self.services();
        let scheduler = services.scheduler().read(cx);
        let mut checking = false;
        let mut latest: Option<OffsetDateTime> = None;
        let listed = self
            .navigation()
            .repositories()
            .rows()
            .into_iter()
            .flatten()
            .map(|entry| {
                let key = entry.key().clone();
                let schedule = scheduler.schedule().repo(&key);
                checking |= checking_now(services, Path::new(&key))
                    || schedule.is_some_and(|repo| repo.in_flight());
                latest = latest.max(schedule.and_then(|repo| repo.last_completed));
                Listed {
                    name: entry.name(),
                    parent: Path::new(&key)
                        .parent()
                        .map(|parent| match parent.file_name() {
                            Some(name) => name.to_string_lossy().into_owned(),
                            None => parent.display().to_string(),
                        }),
                    items: schedule.map_or(0, |repo| repo.unshared.items.len()),
                    key,
                }
            })
            .collect();
        ShareFacts {
            listed,
            chosen: self.navigation().selected().cloned(),
            checking,
            latest,
            now: scheduler.utc(),
        }
    }

    /// To share pressed: it chooses the next repository with work, asked
    /// again now so that a check landing since the name was drawn cannot make
    /// it choose another than it names. It reads nothing and changes nothing
    /// when there is none (decision 23), and never moves focus or the layout.
    pub(super) fn choose_next_to_share(&mut self, cx: &mut Context<Self>) {
        if let Some(next) = self.share_facts(cx).next() {
            self.select_repository(next, cx);
        }
    }

    /// A status item that had focus and is not in `plan` hands focus to the
    /// next shown control to its right, before the frame loses it to a
    /// region (R-38). A modal keeps its own trap.
    pub(super) fn keep_focus_in_footer(
        &self,
        plan: &Plan,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if window.has_active_dialog(cx) || window.has_active_sheet(cx) {
            return;
        }
        for hidden in [Control::UpdateCheck, Control::ToShare] {
            let shown = plan
                .right
                .iter()
                .flatten()
                .any(|control| *control == hidden);
            if shown || !self.footer.handle(hidden).is_focused(window) {
                continue;
            }
            if let Some(next) = focus_after_hiding(hidden, plan) {
                window.focus(self.footer.handle(next), cx);
            }
            return;
        }
    }
}
