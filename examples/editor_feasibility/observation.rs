//! Borrowed change gate for host observations, not native event/performance proof.
use super::session::{Draft, SessionError};

#[derive(Clone, Copy)]
pub enum Signal {
    Notification,
    Changed,
    NonEdit,
    EditHook,
    Capture,
}

/// Opt-in source-level operation counts, not allocations, frames or timings.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub notifications: u64,
    pub changed_events: u64,
    pub non_edit_hints: u64,
    pub edit_hooks: u64,
    pub capture_samples: u64,
    pub borrowed_checks: u64,
    pub owned_readbacks: u64,
    pub draft_changes: u64,
    pub host_notify_requests: u64,
}

impl Metrics {
    pub fn loaded() -> Self {
        Self {
            owned_readbacks: 1,
            draft_changes: 1,
            ..Self::default()
        }
    }

    pub fn owned_readback(&mut self) {
        self.owned_readbacks = self.owned_readbacks.saturating_add(1);
    }

    pub fn host_notify(&mut self) {
        self.host_notify_requests = self.host_notify_requests.saturating_add(1);
    }

    fn signal(&mut self, signal: Signal) {
        let count = match signal {
            Signal::Notification => &mut self.notifications,
            Signal::Changed => &mut self.changed_events,
            Signal::NonEdit => &mut self.non_edit_hints,
            Signal::EditHook => &mut self.edit_hooks,
            Signal::Capture => &mut self.capture_samples,
        };
        *count = count.saturating_add(1);
    }
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Decision {
    pub changed: bool,
    pub notify_host: bool,
}

/// The getter is lazy: non-edit hints and stale generations never read text.
/// Generic notify remains a borrowed full-byte comparison because stock native
/// undo/redo have no Changed event. Only unequal bytes are owned/observed.
pub fn synchronize<'a>(
    draft: &mut Draft,
    generation: u64,
    signal: Signal,
    visible: bool,
    metrics: &mut Option<Metrics>,
    text: impl FnOnce() -> &'a str,
) -> Result<Decision, SessionError> {
    draft.check_generation(generation)?;
    if let Some(metrics) = metrics {
        metrics.signal(signal);
    }
    if matches!(signal, Signal::NonEdit) {
        return Ok(Decision::default());
    }
    let text = text();
    if let Some(metrics) = metrics {
        metrics.borrowed_checks = metrics.borrowed_checks.saturating_add(1);
    }
    if draft.current() == Some(text) {
        return Ok(Decision::default());
    }
    draft.observe(generation, text.to_owned())?;
    if let Some(metrics) = metrics {
        metrics.owned_readback();
        metrics.draft_changes = metrics.draft_changes.saturating_add(1);
    }
    Ok(Decision {
        changed: true,
        notify_host: visible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Original;

    fn loaded(body: &str) -> Draft {
        let mut draft = Draft::new(Original::new(body.into()).unwrap());
        draft.observe(0, body.into()).unwrap();
        draft
    }

    fn sample(
        draft: &mut Draft,
        signal: Signal,
        visible: bool,
        metrics: &mut Option<Metrics>,
        text: &str,
    ) -> Decision {
        let decision =
            synchronize(draft, draft.generation(), signal, visible, metrics, || text).unwrap();
        // Mirrors the host's actual notify branch, not a native notification.
        if decision.notify_host
            && let Some(metrics) = metrics
        {
            metrics.host_notify();
        }
        decision
    }

    #[test]
    fn unchanged_scroll_caret_focus_blink_notifications_do_not_copy_update_or_notify() {
        let mut draft = loaded("same body");
        let mut metrics = Some(Metrics::default());
        let current_ptr = draft.current().unwrap().as_ptr();
        for _ in 0..100 {
            assert_eq!(
                sample(
                    &mut draft,
                    Signal::Notification,
                    true,
                    &mut metrics,
                    "same body"
                ),
                Decision::default()
            );
        }
        let m = metrics.unwrap();
        assert_eq!(m.notifications, 100);
        assert_eq!(m.borrowed_checks, 100); // fallback work is not zero
        assert_eq!(
            (m.owned_readbacks, m.draft_changes, m.host_notify_requests),
            (0, 0, 0)
        );
        assert_eq!(draft.current().unwrap().as_ptr(), current_ptr);
    }

    #[test]
    fn non_edit_hints_never_even_read_text() {
        let mut draft = loaded("same");
        let mut metrics = Some(Metrics::default());
        for _ in 0..20 {
            assert_eq!(
                synchronize(
                    &mut draft,
                    0,
                    Signal::NonEdit,
                    true,
                    &mut metrics,
                    || panic!("selection hint must not read text")
                )
                .unwrap(),
                Decision::default()
            );
        }
        let m = metrics.unwrap();
        assert_eq!(m.non_edit_hints, 20);
        assert_eq!(
            (
                m.borrowed_checks,
                m.owned_readbacks,
                m.draft_changes,
                m.host_notify_requests
            ),
            (0, 0, 0, 0)
        );
    }

    #[test]
    fn same_length_edit_and_duplicate_changed_notify_hook_have_one_update() {
        let mut draft = loaded("abc");
        let mut metrics = Some(Metrics::default());
        assert!(sample(&mut draft, Signal::Changed, true, &mut metrics, "axc").changed);
        for signal in [Signal::Notification, Signal::Changed, Signal::EditHook] {
            assert_eq!(
                sample(&mut draft, signal, true, &mut metrics, "axc"),
                Decision::default()
            );
        }
        let m = metrics.unwrap();
        assert_eq!(
            (m.owned_readbacks, m.draft_changes, m.host_notify_requests),
            (1, 1, 1)
        );
        assert!(draft.status().unwrap().user_edits);
        // GPUI scheduling can deliver notify before Changed as well.
        let mut reverse = loaded("abc");
        let mut metrics = Some(Metrics::default());
        assert!(
            sample(
                &mut reverse,
                Signal::Notification,
                true,
                &mut metrics,
                "axc"
            )
            .changed
        );
        assert_eq!(
            sample(&mut reverse, Signal::Changed, true, &mut metrics, "axc"),
            Decision::default()
        );
        let m = metrics.unwrap();
        assert_eq!(
            (m.owned_readbacks, m.draft_changes, m.host_notify_requests),
            (1, 1, 1)
        );
    }

    #[test]
    fn notify_only_undo_redo_and_table_hook_preserve_current_status() {
        let mut draft = loaded("initial");
        let mut metrics = Some(Metrics::default());
        sample(
            &mut draft,
            Signal::EditHook,
            true,
            &mut metrics,
            "table edit",
        );
        sample(
            &mut draft,
            Signal::Notification,
            true,
            &mut metrics,
            "initial",
        );
        assert!(!draft.status().unwrap().dirty);
        sample(
            &mut draft,
            Signal::Notification,
            true,
            &mut metrics,
            "table edit",
        );
        assert!(draft.status().unwrap().dirty);
        assert_eq!(metrics.unwrap().draft_changes, 3);
    }

    #[test]
    fn normalization_stays_dirty_and_inactive_edit_is_retained_without_redraw() {
        let mut draft = loaded("What if words $$E=mc^2$$ more");
        let normalized = "What if words\n$$E=mc^2$$\nmore";
        // Simulated initial normalization, never ActualEditor provenance.
        draft = Draft::new(draft.original().clone());
        draft.observe(0, normalized.into()).unwrap();
        let mut metrics = Some(Metrics::default());
        assert_eq!(
            sample(
                &mut draft,
                Signal::Notification,
                false,
                &mut metrics,
                normalized
            ),
            Decision::default()
        );
        let decision = sample(
            &mut draft,
            Signal::Changed,
            false,
            &mut metrics,
            "edited inactive doc",
        );
        assert!(decision.changed);
        assert!(!decision.notify_host);
        sample(
            &mut draft,
            Signal::Notification,
            false,
            &mut metrics,
            normalized,
        );
        let status = draft.status().unwrap();
        assert!(status.changed_on_load && status.dirty && !status.user_edits);
        assert_eq!(metrics.unwrap().host_notify_requests, 0);
    }

    #[test]
    fn stale_generation_never_reads_or_counts_and_disabled_metrics_stay_absent() {
        let mut draft = loaded("a");
        let mut metrics = Some(Metrics::default());
        assert_eq!(
            synchronize(
                &mut draft,
                1,
                Signal::Notification,
                true,
                &mut metrics,
                || panic!("stale getter")
            ),
            Err(SessionError::StaleGeneration)
        );
        assert_eq!(metrics.unwrap(), Metrics::default());
        let mut disabled = None;
        sample(&mut draft, Signal::Changed, true, &mut disabled, "b");
        assert!(disabled.is_none());
        assert_eq!(draft.current(), Some("b"));
    }
}
