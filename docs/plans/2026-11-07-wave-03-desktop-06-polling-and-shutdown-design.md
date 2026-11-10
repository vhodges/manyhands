---
title: "Wave 03 D6 Desktop Background Polling And Shutdown Design"
date: 2026-11-07
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6V02CAXQ9CJNRZS0KC9JSW6"
---

# Wave 03 D6 Desktop Background Polling And Shutdown Design

## Intent And Approval Boundary

This design covers the desktop's polling scheduler, its integration with the
one mutation worker, Poll now and the policy controls, the needs-attention
states, preservation of unsaved work, shutdown, the deliberate Retire action,
and their evidence. Approval covers the mechanisms, rulings and task order.
It does not authorize implementation (which needs the owner's separate
authorization), pushing beyond the ticket branch, merging or closing.

Revision 6, **approved 2026-11-09** with the Cycle and the plan. Revision 6's
edits had no independent review (the planning procedure stops at round 3);
Tasks 4 and 11b prove its required-outcome tables test-first after a design
checkpoint.

- Revision 1 was drafted at `3364370c` on 2026-11-07.
- Revision 2 (2026-11-08) added the Wave 04 Cycle 01 coordination.
- Revision 3 (2026-11-09) re-grounds the library facts at `9512080d`, after
  ticket `01M6KHVD6D0DB8KYT6VGQP10Y0` landed. It adds Part E (retiring
  completed contexts), Facts 24 to 28 and U-9 to U-12, and follows Wave 04
  Cycle 01's approved decisions 3 and 7 (W-1, W-2, W-6, W-7). The same day
  the owner decided that polls retire, with the outcome of the CLI's
  `--retire-completed` (decision 16), and that every operation is per item,
  a repository operation being a loop over items (decision 17). So Part E
  retires one reported item at a time.

- Revision 4 (2026-11-09) applies review round 1 (two reviewers at
  `d48bc4aa`) and the owner's decisions 18 to 23. It corrects Facts 4, 6, 7,
  10, 13 to 17, 23, 25, 26 and 28, adds Facts 29 to 31, and settles most
  U-questions.
- Revision 5 (2026-11-09) applies review round 2 (a scoped re-review and an
  adversarial review at `4d22c393`) and the owner's decisions 24 to 27. It
  adds Facts 32 to 36, the in-flight record (R-31), B3's required outcomes
  L-1 to L-6, and R-32 to R-35.
- Revision 6 (2026-11-09) applies review round 3 (a scoped re-review and an
  adversarial review at `c35719c2`). Three mechanisms drew blockers for the
  third time, so each is now a table of required outcomes with a test-first
  task and a design checkpoint: preemption (P-1 to P-7), stale rows (L-1 to
  L-6, now also at runtime) and the retirement loop (RT-1 to RT-11). It adds
  R-36 to R-38 and proposed decision 28. Not independently reviewed.

Library facts are at `9512080d`, checked again by the round 1 source review.
Desktop facts are at `3364370c` unless marked. Unverified points are
U-questions for Task 0.

Conventions: "Fact n" is a Baseline Fact; `D/` is `src/desktop/`; `R/` is
`src/repository/`; "the handoff" is `docs/design/05-sheets-and-dialogs.md`;
"the RFC" is the application runtime and polling RFC.

## Baseline Facts (Library At `9512080d`, Desktop At `3364370c`)

### Library

1. **`poll once` is explicit.** `PollOnceBinding` builds
   `PollPublicationRemoteRequest{ invocation: Explicit, approval: None, restart }`
   (`R/mutation/bind/poll.rs` ~379); `restart` is false in `run` and true in
   `rerun`. It needs no confirmation, skips eligibility (pause and interval do
   not stop it) and refuses a missing publication remote
   (`PublicationRemoteRequired`) or an invalid configuration. Results:
   `Completed` with category `Completed` → `Ok` (or `DiscoveryPending` for
   `IndexPending`); any other category → that category's code; an interrupted
   completion → `RecoveryRequired`, standing `Owed`; `Busy` and `PollYielding`
   are offered as a retry of the same request (`outcome.rs` 593–598, 878–884);
   a transport error with a row stored is final and a new request polls again
   (`outcome.rs` 868–871). Since `9512080d` a separate binding,
   `PollOnceRetiring(PollOnceInput)` (`R/mutation/bind/poll_retirement.rs`),
   runs the same command with `retire_completed=true`, also `Explicit`
   (~342). `PollOnce`, its digest and the automatic path are unchanged.
2. **Policy verbs are single-transaction writes.** `PollConfigure{interval_seconds}`,
   `PollPause`, `PollResume` share `PollPolicyBinding` and answer
   `MutationDataDto::PollPolicy{enabled, paused, interval_seconds,
   backoff_seconds, recovery_suspended, next_eligible_at}`; `Ok` if changed,
   `AlreadyApplied` if not; an interval outside 60–3600 is `InvalidInput`. No
   verb sets `enabled`. `PollResume` clears `paused` and `next_eligible_at`
   only; it leaves the recovery suspension, `enabled` and the backoff value.
   A policy verb reports `writing` and never `accepted` (the record is inserted
   and finished with the write).
3. **No boundary path runs an Automatic poll.** Only `Explicit` is built under
   `R/mutation`. A scheduler calls the public
   `RepositoryService::poll_publication_remote(request, &mut SessionCredentials<P>)`
   (`R/remote/poll.rs:1493`) with `PollPublicationRemoteRequest{root,
   operation_id, invocation: Automatic, approval, restart}`. That entry passes
   no probe; `poll_publication_remote_observed` and `CancellationProbe` are
   `pub(in super::super)`. The public stop is
   `RepositoryService::cancel_remote_operation(root, operation_id)`
   (`R/remote/reservation.rs:1431`), which sets `cancel_requested` on the
   active row; the fetch callback and safe points read it, throttled by
   `POLL_PROGRESS_THROTTLE` (250 ms).
4. **Automatic differs in three ways** (`poll_probed`, `poll.rs:1535`+):
   reservation priority is `Poll`, not `Manual`; `automatic_poll_eligibility`
   runs first and returns `Complete(NotEligible{reason, next_eligible_at})`
   without contacting the remote; and authentication runs with
   `automatic=true`. **That does not stop a first prompt.** `automatic` fails a
   locked key only when `session.blocked_unlock` is set
   (`R/transport/operation.rs:118-125`). That slot is set only after the
   provider has answered `Cancelled` or `Unavailable` (`R/keys/session.rs:367-374`),
   and every unlock starts by calling `clear()` (`session.rs:350`), which
   resets it (`session.rs:285-288`).
   So the first automatic poll of a locked key, and any after a user-lane
   prompt, reaches the provider (`tests/remote_polling.rs:1963-1965`). The
   desktop provider must refuse for background jobs itself (B5).
5. **Dispositions** (`PollDisposition`, `poll.rs:26`): `Current`,
   `FastForwarded`, `LocalAhead`, `Diverged`, `Dirty`, `Conflicted`,
   `Inaccessible`, `Materialized`, `Nonconforming`, `BlockedByLocalState`,
   `MaterializationIncomplete`, `Malformed`, `RemotelyDeleted`, `Closed`,
   `Promoted`, `HistoryUnknown`, `Unpublished`, `NotReached`; each has
   `guidance()` text (`poll.rs:47-83`).
   - `Closed` (a ticket whose closure local primary holds) and `Promoted` (a
     document whose work primary preserves) came with
     `01M6KHVD6D0DB8KYT6VGQP10Y0`. Their guidance names the CLI's
     `context retire`. Neither is permission to remove anything (Fact 24).
   - `Unpublished` came with `01M66HFV44YXNQ7ANJVA16TQ1H`. Its guidance names
     the CLI's `item sync`.
   - A poll target (`PollTargetDto`) carries `kind` (ticket or document) and
     `item_id`, which is what a Retire needs (Fact 24).
6. **Results and errors.** `RemotePollResult::{Complete, IndexPending}` over
   `RemotePollOutcome::{NotEligible{reason, next_eligible_at}, Polled(report)}`;
   `RemotePollReport` exposes `operation_id`, `completion`
   (`Completed|Interrupted|Cancelled`), `completed_at`, `next_eligible_at`,
   `targets` (each `target`, `kind`, `item_id`, `disposition`, `before_oid`,
   `after_oid`) and `outcome` (`Option<RemoteOutcomeCategory>`);
   `local_changes()` is crate-private. `RemotePollError::{Repository,
   Transport, Busy, PollYielding, Interrupted, ConflictPending}`.
   `PollYielding` is what a Manual reservation finds when a Poll row is active
   (it sets `yield_requested`). `Busy` is another operation holding the
   reservation, **or** the fetch meeting a held repository lease; in the lease
   case the row stays `interrupted` with no category and no backoff
   (`poll.rs:1801-1806`). With an existing non-terminal row and `restart=false`,
   `poll_probed` returns `Err(Interrupted)`; a terminal row returns its stored
   result; `restart=true` resumes with the original target (C5 decision 24).
   **A live failure is an error, not a completion.** Without a remote or with
   an invalid configuration, the library records `ConfigurationRequired` and
   returns `Err(Transport(_))` (`poll.rs:1621-1660`). A fetch failure records
   its category, then returns `Err(Transport(error))` (`poll.rs:1809-1821`).
   `Completed` with a category comes back only when a terminal row is replayed
   under the same ID (`poll.rs:1595-1603`). `SshTransportError` exposes `kind`,
   `selected_key_id` and `authority` (`R/transport/error.rs:7-14`). Its
   classification into categories is the crate-private
   `observation::transport_category` (`observation.rs:464-490`).
7. **Policy and eligibility.** `RemotePollingConfiguration` has `enabled`,
   `paused`, `interval` (60–3600 s, default 300), `automatic_backoff`
   (60–900 s, default none) and `recovery_suspended` (`state.rs:1929`).
   `eligibility()` (`poll.rs:1059`) checks `Disabled`, `Paused`,
   `RecoverySuspended` in that order, then `NotDue` with
   `next_eligible_at = min(next, now + max(interval, 900 s))` (the clock-
   rollback clamp, written back by the public
   `automatic_poll_eligibility(root)`, `poll.rs:1149`). `next_eligible`
   (`poll.rs:1096`): `Cancelled` keeps the stored deadline; `Completed` adds
   the interval; an `Explicit` failure answers `None`, which keeps the stored
   deadline (`poll.rs:1093-1105`, `state.rs:3141-3143`); an `Automatic`
   `TransportUnavailable` or `ProtocolRejected` adds min(2 × previous backoff,
   900) or 60 s; key, unlock, host-approval, configuration and
   repository-unavailable categories add the interval. Backoff doubles only
   for a Poll-priority transport or protocol failure and clears on `Completed`.
8. **Recovery suspension.** Its only setter is still the registry default
   while the `remote-history-recovery-required` marker exists (`state.rs:54`,
   `recovery.rs:2344`). No poll outcome sets it. Since `9512080d`, an explicit
   history repair retires the marker (`R/remote/resolve.rs:206`,
   `recovery.rs:2389`), and the polling table is then rebuilt with clear
   defaults, so later registrations start unsuspended (`state.rs:228`). Rows
   already suspended stay suspended.
   `resume_remote_polling_after_recovery(root)` (`state.rs:7180`) clears one
   row's flag alone and still has no caller.
9. **The poll refreshes the index itself.** A completed fast-forward or
   materialization sets `refresh_required`, finishes the row and calls
   `refresh_repository`; a failed refresh answers `IndexPending` (`poll.rs`
   ~1862–1920, 2065). The caller does not refresh; it may retry with an
   `IndexRefresh`.
10. **Read side.** `RepositoryService::polling_status(&ResolvedRepository)`
    (`R/read/status.rs:661, 718`), after `resolve_repository`
    (`R/read/resolve.rs:228`), gives `enabled`, `paused`, `interval_seconds`, `backoff_seconds`,
    `recovery_suspended`, `latest_outcome`, `latest_observed_at` (the
    observation batch, not the attempt), `active_operation_id` (any action) and
    `next_eligible_at` (stored, unclamped). It has no attempt time and no
    dispositions; those are on `RepositoryService::remote_snapshot(root)`
    (`latest_attempt_at()`, `poll_dispositions()`, `state.rs:2526-2533`).
11. **Dirty is Git-only.** `Dirty` and `Conflicted` come from
    `sync::require_clean_target` (`sync.rs:587`): foreign Git state files,
    index conflicts, any status entry including untracked. `src/drafts/` is
    outside every repository and no code under `R/remote/` references it, so
    a clean worktree can be fast-forwarded while the desktop holds unsaved
    editor text.

### Desktop

12. **Startup.** `main.rs:6-15` installs the panic hook, calls
    `initialize_git_transport_before_threads`, then `desktop::run()`.
    `run()` (`D/mod.rs:47`) initializes GPUI Kit, opens the service,
    builds `LibraryBackend` and `Services::start` (`D/services/mod.rs:342`),
    which spawns the worker thread `manyhands-worker`. Exactly one main window
    opens; `Services` is `Clone` and dialogs clone it.
13. **The worker.** One thread, two lanes: user first, never preempting, the
    first due background job next. It creates the one `DesktopProvider` and
    the one `SessionCredentials`, local to `run()` (`worker.rs:853-856`).
    `quit()` closes admission, removes queued jobs (sending `Removed`), cancels
    the running job's token and answers `Idle` or `Finishing(id)`. `busy`
    retries use 100/200/400 ms with `std::time::Instant` directly; `Backend`
    has no time method. `WorkerHandle` holds no backend; it is moved into the
    thread (`worker.rs:332-349`). `spawn` drops the `JoinHandle`, and the
    thread ends only when the last handle drops (`worker.rs:313-321, 343-347`).
    Handles are cloned into `Services`, `DraftRegistry`, `Releases`,
    `Submissions` and dialogs (`mod.rs:342-356`).
14. **Intents.** The `Intent` enum has no poll variants. Adding one needs an
    arm in `confirmed`, `label`, `root`, `changes_items`, `stops_at_safe_points`
    and `job_kind` (`D/services/requests.rs`), the `JobKind` enum
    (`D/services/mod.rs:124`), `Intent::build`
    (`D/services/requests/build.rs`) and the operations model
    (`Progressing::of`, `stage_text`, `D/model/operations.rs`). `Forward::event`
    marks `accepted` on any progress stage (`worker.rs:1190`). The library's
    command table marks `PollOnce` and `ContextRetire` `stops_at_safe_points:
    true` (`R/mutation/command.rs:365-369`). A desktop arm that said `false`
    would answer Cancel with `TooLate` after acceptance and never offer Give up
    (`worker.rs:655-661`, `requests.rs:590-592`).
15. **Prompts.** `DesktopProvider::request_passphrase` runs on the worker,
    consults `DeclinedKeys`, otherwise sends `WorkerEvent::Prompt` and blocks on
    the reply (`D/services/prompts.rs:175`). `Operations` holds one prompt;
    the workspace opens a modal. Lane is never consulted, so a background job
    that needs a passphrase would open the same modal. The worker already
    sets per-job state the provider reads (`DeclinedKeys`, `worker.rs:866-867`,
    `prompts.rs:111-118`), which can carry the lane. Replies: `Supplied`,
    `Cancelled` (records the key declined for the session), `Unavailable`
    (quit). Declined state clears through `clear_declined_for`, which needs the
    request whose attempt asked (`prompts.rs:128-139`); the only per-key clear
    is `#[cfg(test)]` (`worker.rs:438-441`). The provider has only
    `request_passphrase` (`session.rs:200-202`): host approval never reaches
    it.
16. **Quit.** `Workspace::request_quit` (`D/view/workspace/lifecycle.rs:26`):
    unkept-draft asks, `save_drafts` (flush), then `worker().quit()`; `Idle`
    quits, `Finishing(id)` shows "Finishing {label}" with a **Quit now** button
    (`D/view/quit.rs:70-127`); completion is detected on each operations
    notify (`quit_if_finished`, `lifecycle.rs:276-281`;
    `workspace/requests.rs:123-128`), with no timer. A job with no request in
    `Operations` shows a bare "Finishing" (`quit.rs:69-80`). Quit now calls `cx.quit()`
    without joining the worker thread; credentials are dropped only when the
    thread exits. No "still stopping" text exists in `src/`. Timer idiom:
    `cx.background_executor().timer(..)` (`D/view/editor/toolbar.rs:195`,
    `D/services/drafts/flush.rs:183`, `D/services/reads.rs:78`).
17. **Bring-in reaches the screen already.** A `Finished` event with
    `changes_items` marks the repository changed: `Changed{roots, items:
    BTreeSet<RepoKey>}` is per repository, and its private `insert` is called
    only from the `Finished` arm (`D/model/operations.rs:153-171, 296-322`).
    `operations_changed` reloads the lists and every tab of the repository
    (`reload_tabs(&items)`, `D/view/workspace/requests.rs:123`). A poll's
    answer needs a new `Operations` entry point to do the same (Task 5).
    A clean edit session takes a newer read (`late_read_landed`,
    `D/view/item/edit/late.rs:40`); a session with a kept draft is classified
    (`registry.classify`, `TargetState::Changed`) and shows the changed banner;
    a save against a moved file answers `ExternalChange`.
18. **No poll UI.** Settings has four pages (Repositories, Remotes, Keys,
    Hosts; `D/view/settings/mod.rs:50`, `D/model/stamp.rs` `SettingsPage`). No
    "Poll now", "Check now", footer or Activity exists. `LATER_NOTE` names
    neither polling nor D6. `D/view/confirm.rs:732` carries the comment "No
    desktop path asks the boundary for this preview yet (D5, D6)". Since
    `9512080d` the generic operation-abandon confirmation shows a
    retirement's details (`detail_fields`, `confirm.rs` ~671–688): the
    request retained, the phase, worktree and branch removal, what is
    retained, and whether discovery is pending. That is the library ticket's
    D13. No Retire action exists.
19. **Background submissions today** are all `IndexRefresh`: the launch refresh
    of listed repositories (`D/view/workspace/repositories.rs:66`, gated by
    `launch_refresh`), after a post that left discovery pending
    (`D/view/item/posting.rs:125`) and after `RepoCreate`/`RepoEnable`
    (`worker.rs:1050`).
20. **Test seams.** `Backend` has hooks `read_gate`, `before_job`, `after_job`,
    `on_progress`, canned-answer hooks; `DelayingBackend`
    (`D/test_support/delaying.rs`) holds jobs at stages over a real service.
    Worker tests run on real time with a 10 s wait. UI tests use
    `#[gpui_kit::test]` and `D/view/harness.rs` (`keys`, `click`, `label`).

### Transport and evidence

21. **Initialization.** `initialize_git_transport_before_threads`
    (`src/runtime.rs:25`, once) sets a 10 s connect and 30 s server timeout;
    DNS and total transfer are unbounded. The transport fails closed with
    `RuntimeUninitialized` if it did not run. D6 keeps this order.
22. **Stalls are characterized without cancellation.** Wave 02 Cycle 03:
    handshake, authentication, advertisement and transfer stalls return at
    about 30 s; exec acknowledgement plus cleanup about 45 s
    (`tests/ssh_fixture.rs:660-733`, raw `git2`). Cancellation during DNS,
    connect or handshake is **not** measured; there is no DNS or blackhole
    test. Mid-transfer stop is measured (`poll_progress_stop`, under 2 s).
23. **Harness.** `Stall(FixtureBoundary::Handshake, d)` accepts TCP and sends
    nothing for `d`; `stall_at`, `hold_advertisement()` (blocks until release)
    and `accepted_sockets()` are in `tests/support/ssh_remote.rs:110-124, 212,
    549, 628` and `ssh_server.rs:99-127`. Each case runs in its own child
    process, killed after 100 s (`tests/support/ssh_harness.rs:237-249`).
    Existing poll cases cover yield, cancel at each phase, public cancel of a
    held advertisement, locked-key no-prompt, backoff cap and materialization
    once (`tests/remote_polling.rs`, `tests/ssh_transport/poll.rs`,
    `tests/cli_remote/polling.rs`).

### Retirement (library ticket `01M6KHVD6D0DB8KYT6VGQP10Y0`, at `9512080d`)

Sources: `R/mutation/bind/retire.rs`, `R/mutation/dto.rs`,
`R/context_retirement/`, the retirement Cycle and the API audit's
"Completed-context retirement delivery".

24. **`ContextRetire` is an ordinary user mutation.**
    `Mutation::ContextRetire(ContextRetireInput{root, kind: AuthoringKind, id:
    ItemId})`, command `context retire`.
    - It is unconfirmed (no prepare or confirm), noncheckpoint, stoppable and
      local, with a journal row (`command.rs:103`).
    - Each run takes a fresh read-only Fetch observation with the
      selected-key session, so it can need an unlock or a host approval.
    - Before removing anything, it proves the closure from primary's
      committed lifecycle fields, and that the item is clean.
    - `closed` and `promoted` are offers, not permission.
    - A Final answer replays stably for the same request; a new request runs
      anew.
25. **Answers.** `MutationDataDto::ContextRetirement(ContextRetirementMutationDto{kind,
    item_id, operation_id, state, code, worktree_removal, branch_removal,
    materialization_settled})` (`dto.rs:926`; mapping in `bind/retire.rs`
    ~110–175).
    - Final states: `retired`, `already_absent`, `refused`, `preserved`.
    - `pending` is Owed, with code `discovery_pending` for an index still to
      settle, or `recovery_required` or `cancelled` for cleanup still
      pending, or the code of a failure met after a snapshot was stored
      (`bind/retire.rs:163-177`).
    - **A failure before any effect often arrives as `refused`.** With no
      claim yet, any error becomes `refused`, Final (Owed only for `busy` and
      `poll_yielding`; `bind/retire.rs:178-189`). After the claim, an
      observation failure stops through `refuse_execution`, also Final
      `refused` (`run.rs:440-443, 740-750`). Its code comes from
      `transport_code`: `unlock_required` for an unavailable provider or a
      cancelled unlock, `host_approval_required` for a host (`run.rs:1048-1069`).
      So the desktop reads the worker's record state, then the answer's
      state, then its code (E3).
    - Removals are `none`, `possible` or `observed`; a removal other than
      `none` marks a local change.
    - The result registries in `src/results.rs` are unchanged.
26. **Recovery is the ordinary kind, but Resume needs the original.** The
    pending row lists an Owed retirement as action `retire_context`
    (`read/status.rs:82`), family Local, with Resume and Abandon
    (`model/operations.rs:1060-1087`). Its label is the library's action
    string (`view/operations.rs:767`). Abandon is the confirmed
    `OperationAbandon`, whose preview carries `details.retirement` (Fact 18).
    Resume of a row still held by an accepted request answers
    `original_request_required` (`bind/resume.rs:327-339, 919-927`), unless all
    it owes is the index hand-off (`resume.rs:1022`). The desktop resends a
    named request only for lifecycles (`D/services/resends.rs:1-13, 199`).
27. **Read precedence is unchanged.** Until a context is retired, reads
    prefer the item's own worktree copy, so a ticket closed on primary can
    still read as open. The latest-copy ticket `01M6K04YQM2JF6SBGPB1ZT6R8T` is
    separate.
28. **Unkept text and drafts on the desktop** (at `9512080d`).
    - `TabEditing.unkept` is `unkept_edit`: the edit session and repair text
      only, not the comment composer (`D/view/item/edit/leave.rs:121-147`).
    - Quit asks through `Tabs::unkept` (`D/view/tabs.rs:400-410`) and
      `EditsNotKept` (`D/view/workspace/lifecycle.rs` ~60–95: Cancel, the
      default, shows the first such tab). A repository removal counts through
      `EditTabs::unkept(root)` (`D/services/edit_tabs.rs:57`, needs `&mut
      self`; `confirm.rs:794-800`, `removing_closes` at `confirm.rs:112`).
      Filtering to one item works through `TabKey::Item{repo, kind, id}`
      (`model/tabs.rs:18-23`).
    - The quit asks' `open_asking` and `AskIds` are private and hard-code
      Cancel and Quit anyway (`D/view/quit.rs:213-294`).
    - There is no per-item flush; `flush_now(DraftId)` flushes one session
      (`D/services/drafts/flush.rs:35-55`).
    - Drafts live in `src/drafts/`, outside every repository. The Drafts
      page offers Restore, Open and Export… (`D/view/drafts.rs`).
    - `DraftRegistry::stored_for` answers `None` while the store is
      unavailable or not yet listed, and the listing is the last one taken
      (`D/services/drafts.rs:143, 326-340`). A second instance claims a draft
      at its first edit (`drafts.rs:360`).
    - Restore policy: a draft of a closed ticket classifies `TicketClosed`,
      for the item and for its comments, and Restore is refused; every
      comment draft is refused Restore whatever its state; a promoted
      document's draft is `Changed`, restorable flagged
      (`src/drafts/compare.rs:258-260, 304, 368-382`).

### Stale rows, cancellation and side effects (round 1, at `d48bc4aa`)

29. **A killed process leaves its row active.** No liveness check reclaims a
    stale owner. A configuration fence does: any configuration generation
    bump, and a key-selection change for every repository, moves each active
    row to `interrupted` (`state.rs:2925, 3003-3025`). Otherwise
    `phase='interrupted'` is written only by live code
    (`reservation.rs:625, 2091, 2232, 2356`; `poll.rs:2359`). Any active row
    makes a reservation under a fresh ID answer `Busy`, or `PollYielding` for
    a Manual request meeting a Poll row (`reservation.rs:1296-1313`). Only a
    same-ID restart takes the row over (an epoch bump,
    `reservation.rs:1577-1647`), and it must match the stored target,
    priority and kind (`:1260-1265`). `RemoteOperationInspection` exposes no
    action or priority (`reservation.rs:79-90`);
    `polling_status.active_operation_id` gives the ID only
    (`read/status.rs:736-747`).
30. **`cancel_remote_operation` is a write.** It takes a shared cache lock (up
    to 250 ms, then `RepositoryBusy`; `coordination.rs:14, 216-258`) and an
    IMMEDIATE transaction (5 s busy timeout; `repository.rs:147`,
    `state.rs:7292-7306`). It answers `recovery_required` when the operation
    has no row yet (`reservation.rs:1436-1438`), and `Ok(())` **without
    effect** when the row exists but is not active (`active()`,
    `reservation.rs:269-283, 1439-1443`), such as an `interrupted` row
    between a same-ID restart's attempts. A poll reserves its row only
    after eligibility, plan and configuration (`poll.rs:1609-1690`), so an
    early cancel is lost unless sent again.
31. **A poll without a remote writes state.** It records
    `ConfigurationRequired`, fences the configuration and stores an attempt
    with a deadline (`poll.rs:1625-1660`). About 19 test sites start
    `Services`, whose launch refresh is on by default
    (`D/services/mod.rs:570, 581-589`).

### Round 2 (at `4d22c393`)

32. **A pending local operation stops polls and retirements.** The poll path
    calls `require_no_pending_local` before it reserves
    (`R/remote/poll.rs:1674`; `src/repository/recovery.rs:729-740`) and
    returns `Err(Repository(recovery_required))` while any local operation of
    the repository is unfinished. No attempt is recorded, so the stored
    deadline stays in the past. Check now meets the same refusal. A
    retirement refuses before its claim, as `refused` with
    `recovery_required`, when a local operation is pending or a remote row is
    active, index-pending or needs reconciliation
    (`context_retirement/authority.rs:300-310`).
33. **The read side cannot rebuild a retirement.** Its journal row is
    inserted with no `item_id` (`context_retirement/authority.rs:325, 402`).
    `context_retirement_records` stores kind and item ID
    (`authority.rs:327, 404`), but no public read exposes them. `OperationDto` has no
    target and no kind, and its `item_id` is `None` for it
    (`read/dto.rs:833-860`; `read/status.rs:139-193`). `RequestDto` carries no
    input (`R/mutation/dto.rs:48-60`).
34. **A poll's stale row has no desktop recovery path today**, apart from a
    configuration fence (Fact 29). Eligibility is
    checked before a restart is attempted (`poll.rs:1608-1617`), so a paused,
    disabled, held or not-due repository returns `NotEligible` without
    touching the row. A takeover checks target, priority and kind, not
    liveness (`reservation.rs:1255-1273, 1577-1647`). An automatic row is
    `NotResumable` (`bind/resume.rs:439-447`). A row that holds its
    reservation is listed with no next action (`read/status.rs:325-332`), and
    the desktop offers neither Resume nor Abandon for a remote row
    (`model/operations.rs:1021-1027, 1060-1087`). A Manual row meets a Manual
    reservation as `Busy`, and a Poll row makes a Manual one set
    `yield_requested` and answer `PollYielding` (`reservation.rs:1296-1313`).
35. **The session caches one passphrase.** Every authenticated operation
    calls `reconcile_source`, which clears the cache when the cached key
    differs (`keys/session.rs:304-310`; `transport/operation.rs:108`), and
    every unlock clears it before asking (`session.rs:350`).
36. **Worker entry points and quit plumbing.** User-lane jobs enter through
    `submit`, `submit_with_request_id`, `requeue` (which keeps the original
    lane) and `observe_host` (`worker.rs:357-370, 466-546, 511-515, 667-678`),
    all through `admit()`. The in-flight twin check does not cover
    `ContextRetire` (`worker.rs:771`). `quit()` cancels the running job's
    token (`worker.rs:688-705`). The unkept ask opens while `QuitPhase` is
    still `Running` (`lifecycle.rs:82-97, 113-115`).
    `Finished` is sent after `running = None` (`worker.rs:1032-1046`).
    `quit_if_finished` calls `worker().quit()` on every `Operations` notify
    (`lifecycle.rs:276-281`), and the graceful exits reach `cx.quit()`
    through `quit_now` (`lifecycle.rs:152-159, 268-281`). The worker's `Owner`
    must not park the GPUI test scheduler (`worker.rs:316-319`).

## API Audit Refresh (Entry-Gate Item 3)

| Operation D6 needs | Entry point | Status |
| --- | --- | --- |
| Automatic poll | `poll_publication_remote`, `Automatic` | Public; no probe (Fact 3) |
| Stop a poll | `cancel_remote_operation(root, op)` | Public (Fact 3); errors before the row exists (Fact 30) |
| Eligibility | `automatic_poll_eligibility(root)` | Public (Fact 7) |
| Check now | `Mutation::PollOnce` | C5 binding (Fact 1) |
| Interval, pause, resume | `Mutation::PollConfigure/PollPause/PollResume` | C5 bindings (Fact 2) |
| Clear suspension | `resume_remote_polling_after_recovery` | Public, no caller (Fact 8) |
| Policy read | `RepositoryService::polling_status(&ResolvedRepository)` after `resolve_repository` | Public (Fact 10) |
| Attempt time, dispositions, active operation | `remote_snapshot(root)`; `polling_status.active_operation_id` | Public; `latest_attempt_at` is stored (U-3) |
| Index retry | `Mutation::IndexRefresh` | Existing |
| Retire one item | `Mutation::ContextRetire` | Library ticket binding (Fact 24) |
| Retirement recovery | `Mutation::OperationResume` / `OperationAbandon`, and a resend of the original request | Existing; Resume of an accepted retirement needs the original request (Fact 26) |
| Poll and retire in one pass | `Mutation::PollOnceRetiring` | Exists (Fact 1); D6 does not use it: one pass cannot leave out one item (decisions 14 and 17) |
| New operation ID | `OperationId::new()` | Public (`src/repository.rs:309-312`; U-1) |
| "Has a publication remote" | a remote read | U-2 |

## Source Questions

Round 1's source review settled most of them (2026-11-09, at `d48bc4aa`).
Task 0 re-checks each answer at its baseline.

| U | Answer | Evidence |
| --- | --- | --- |
| U-1 | `OperationId::new()` is public; desktop tests already use it. No library gap. | `src/repository.rs:309-312`; `requests_tests.rs:106` |
| U-3 | `remote_snapshot` is a public registry read; `latest_attempt_at` is a stored column and survives a restart. | `state.rs:7193`; `repository.rs:1747-1777` |
| U-4 | A killed process leaves its row active, not interrupted. A fresh ID then answers `Busy` (Automatic) or `PollYielding` (Manual); only a same-ID `restart` with the same target, priority and kind takes the row over. | Fact 29 |
| U-5 | `RepositoryService` is `Send + Sync` and no in-process lock spans the fetch, but the cancel itself takes a shared cache lock and an IMMEDIATE transaction, so it runs off the UI thread (R-25). | Fact 30 |
| U-6 | The cancel sets `cancel_requested` on an active row only; the poll sees it at the next safe point or within the 250 ms transfer throttle and returns `Ok(Polled{completion: Cancelled})`, keeping the deadline and backoff. | `reservation.rs:631-683, 700-735, 1431-1444` |
| U-7 | Resume holds: the flag is a stored column, and nothing re-derives it on read. | `state.rs:39-54, 234-236, 7180-7190` |
| U-8 | The transport error carries `selected_key_id` and `authority`; a passphrase is cached only after success, so the proof is the user-lane `PollOnce`'s own answer. | `transport/error.rs:7-14`; `session.rs:352-363` |
| U-10 | `Outcome::of_mutation`: `retired` → Success; `already_absent` → Noop; `refused` with a blocked-class code → Blocked, with a transport or busy code → Error; `preserved` → Partial with a removal, else Blocked; `pending` → Partial. `changes_items` is per intent: true for `ContextRetire`. | `results.rs:713-735`; `bind/retire.rs:229-230`; `requests.rs:484` |
| U-12 | The provider does not know the lane, but can learn it from per-job state the worker already sets. `ContextRetire` authenticates with `automatic=false`. An `Unavailable` answer that does not decline the key gives `unlock_required`, state `refused`, outcome Blocked. | `worker.rs:866-867`; `prompts.rs:111-118`; `run.rs:783-793, 1061` |

Still open for Task 0:

- **U-2.** Which read tells the desktop that a repository has an SSH
  publication remote, and what a non-SSH remote answers. Without a remote, or
  with an invalid configuration, a poll answers `Err(Transport)` and writes
  state (Fact 31), so the scheduler polls only repositories with an SSH
  publication remote (R-27).
- **U-9 (settled in round 2, mostly).** The pending row lists an Owed
  retirement as action `retire_context`, with Resume and Abandon, but carries
  neither kind nor id (Fact 33), so E4 takes them from the in-flight record
  (R-31). Open: whether the retirement's remote reservation row is listed
  too.
- **U-11 (rest).** What an open tab of the item reads after its worktree is
  removed, and whether Export works for a closed ticket's item and comment
  drafts. If Export fails, stop for the owner.
- **U-13 (settled in round 3).** A target, priority or kind mismatch answers
  `OperationMismatch` (`reservation.rs:207-214, 1259-1263, 1599-1604`). A
  configuration fence answers `recovery_required` on a row it already moved
  to `interrupted` (`reservation.rs:1612-1618`; `state.rs:2925`), and so
  does a pending local operation (Fact 32). L-3 therefore judges "not ours"
  by `active_operation_id`, never by the code.
- **U-14 (an experiment, not a reading).** How Task 1 makes a TCP connect that never completes, on Linux and
  in CI, without root. A non-routable address can fail at once instead. If
  nothing is reliable, stop for the owner: decision 22 then falls back to a
  departure.
- **U-15.** Whether a Check now killed by Quit now or a crash leaves a
  pending row the desktop can find (its request ID), and whether
  `OperationResume` then resending `PollOnce{root}` under that ID restarts
  its Manual row (L-4). If not, L-4 becomes a known limit with the CLI's
  recovery, and stops for the owner.
- **U-16.** Where the in-flight record and the instance lock live in the app
  data directory, and that an advisory lock works on Linux, macOS and Windows
  for R-31.

## Execution Order

Serial, one task at a time: no lane branches, no `AGENTS.md` exception, as
D5. Task 1 (readiness) precedes the worker work so the stopping status rests
on measurements.

## Part A: The Schedule Model (`D/model/polling/`)

Pure, synchronous and clock-injected, so the policy is table-tested without a
worker or a thread.

### A1: Clock seam (R-5)

```rust
pub(crate) trait Clock { fn utc(&self) -> OffsetDateTime; fn mono(&self) -> Instant; }
```

The production clock reads `OffsetDateTime::now_utc()` and `Instant::now()`;
tests advance both by hand. The model never calls either itself.

### A2: State

Per repository (`RepoSchedule`), in memory:

- `policy`: the last `polling_status` read (`enabled`, `paused`,
  `interval_seconds`, `backoff_seconds`, `recovery_suspended`,
  `next_eligible_at`, `active_operation_id`).
- `phase`: `Idle`, `Queued`, `Running{op}`, `Waiting{until: Instant, why, op}`
  (`Busy`, `Interrupted`, `Unfinished`, `NotDue`), `Preempted` (until the user
  lane empties, no time), `Attention`, `Retiring`, `Stopped`.
- `last`: the last `AttemptSummary` (time, outcome, counts of brought-in items,
  problem dispositions), restored at launch from `remote_snapshot`.
- `preempted`: how many times in a row the repository's poll was preempted
  (R-34).

Attention is held per key or host, not per repository (R-8): `Reason` is
`KeyLocked(KeyId)`, `KeyProblem(Option<KeyId>, Root)` (a missing, unreadable
or rejected key, or none selected, which has no key ID),
`HostUnapproved(Authority)`, and per repository `ConfigurationRequired` and
`RecoveryHeld`. A key or host reason names every repository it holds back.
Attention never changes `policy`.

### A3: Decisions the model makes

- `due(now) -> Vec<Root>`: repositories whose `phase` is `Idle` or whose wait
  elapsed, whose policy is enabled, unpaused, unsuspended and past
  `next_eligible_at`, with no attention and no retirement loop running
  (R-32); ordered by earliest `last.completed` first (R-7, fairness). At
  launch the stored deadline governs, whether it came from the interval or a
  failure backoff (decision 23).
- `next_wake() -> Option<Instant>`: the earliest of every wait and every
  `next_eligible_at` converted to the monotonic clock at arming time.
- `after(attempt)`: moves the phase from the attempt (table below).
- `clock_event(utc_delta, mono_delta)`: if the two differ by more than 60 s
  (a suspend or a jump), every `Waiting{NotDue}` becomes `Idle`, a
  `Waiting{Busy | Interrupted | Unfinished}` keeps its operation and its
  place in R-4's schedule, policies and remotes are marked stale for
  re-reading, and at most one poll per repository is due, staggered one at a
  time (R-6). No catch-up loop.
- `attention() -> impl Iterator<Item = (&Reason, &[Root])>`: one entry per
  key, host or repository, and nothing else (decision 11, W-1).

### A4: After an attempt

A live failure does not come back as `Completed` with a category. The
library records the category, then returns `Err(Transport(e))` (Fact 6). The
desktop classifies `e.kind` the way the crate-private
`observation::transport_category` does (R-29). A `Completed` answer with a
category comes back only when a terminal row is replayed under the same ID,
and it is classified by that category the same way. Every poll after a
terminal answer uses a **new** operation ID (`OperationId::new()`); only the
R-4 schedule after `Busy` or an interruption, and R-24's launch takeover,
reuse one.

| Attempt answer | Next phase | Status text |
| --- | --- | --- |
| `Ok(Polled{completion: Completed})` | `Idle`, due at the stored `next_eligible_at`; clears any attention for this repository | Checked just now · up to date / Brought in N changes |
| `Err(Transport(e))`, transport unavailable, remote unavailable, protocol failure or push rejected | `Idle` at the stored backoff deadline | Can't reach {repo}. Your work is safe. |
| `Err(Transport(e))`, unlock cancelled, provider unavailable or unlock failed | Attention `KeyLocked(id)` for `e.selected_key_id`; `NoKeySelected(root)` if it is `None` (owner ruling 2026-11-09: with no key named there is nothing to unlock) | Needs unlocking; Choose a key when no key is named |
| `Err(Transport(e))`, no selected key, or a key missing, unreadable, invalid or rejected | Attention `KeyProblem(e.selected_key_id, root)` | Choose a key |
| `Err(Transport(e))`, host approval or replacement required, or host verification unavailable | Attention `HostUnapproved(e.authority)`, or per repository if `None` | Needs host approval |
| `Err(Transport(e))`, configuration invalid, username required or publication remote missing | Attention `ConfigurationRequired` | Can't check |
| `Err(Transport(e))`, key source, selection, endpoint or host trust changed | `Idle` after re-reading policy and remotes (R-11) | Checking soon |
| `Err(Transport(e))`, registry unavailable or runtime uninitialized | `Idle` at the interval | Can't check |
| `Ok(Polled{completion: Cancelled})` by preemption (R-3) | `Waiting{Preempted}` until the user lane is empty, then `Idle`; the library keeps the deadline | Checking soon |
| `Ok(Polled{completion: Cancelled})` by quit, reconfiguration or a launch takeover's stored cancel | `Stopped` / `Idle` after re-reading | — |
| `Ok(Polled{completion: Interrupted})` (it yielded to a Manual operation) | `Waiting{Interrupted}` on R-4's schedule, same operation, `restart` | Checking soon |
| `Err(Interrupted)` | `Waiting{Interrupted}` on R-4's schedule, same operation, `restart` | Checking soon |
| `Err(Busy)` | `Waiting{Busy}` on R-4's schedule, same operation, then the interval with a new one | Checking soon; after R-4's steps, Waiting for another operation to finish |
| `Err(Repository(recovery_required))` | `Waiting{Unfinished}` on R-4's schedule, then the interval; never at the stored deadline (R-35) | Waiting for an unfinished operation (with a link to its pending row) |
| `Err(Repository)`, any other code | `Waiting` on R-4's schedule, then the interval (R-35) | Can't check |
| `Err(ConflictPending)` | `Idle` at the interval | Waiting for the conflict to be resolved |
| `Complete(NotEligible{Paused \| Disabled, next})` | `Idle` at `next` | Paused / Checking is off |
| `Complete(NotEligible{RecoverySuspended, _})` | Attention `RecoveryHeld` | Checking is held until recovery |
| `Complete(NotEligible{NotDue, next})` | `Waiting{NotDue}` at `next` | Checked N ago |

An automatic poll never answers `PollYielding`: only a Manual reservation
meets a Poll row (Fact 29). Check now can.

A `recovery_required` refusal is what a poll answers while any local
operation of the repository is pending, including a retirement left pending
(Fact 32). No attempt is recorded, so the stored deadline stays in the past;
R-35's bounded schedule is what keeps the scheduler from polling in a loop.

The summary's counts come from the attempt's targets:

- `FastForwarded` and `Materialized` count as brought in, also in a cancelled
  or interrupted attempt that reports them, and mark the repository changed.
- `Current` and `LocalAhead` are counted as up to date; `LocalAhead` adds
  "N not shared yet".
- `Dirty`, `Diverged`, `Conflicted`, `Inaccessible`, `BlockedByLocalState`,
  `MaterializationIncomplete`, `Nonconforming`, `Malformed`,
  `RemotelyDeleted`, `HistoryUnknown`, `Unpublished` and `NotReached` are
  listed with their text under the repository's attempt entry.
- `Closed` and `Promoted` go to Part E's loop, but only from a `Completed`
  attempt; a cancelled or interrupted attempt's are listed and wait for the
  next one (E1).

None of them is an error. Where `guidance()` names a CLI command, the
desktop shows its own words naming the desktop action instead (R-20):
Retire… for `Closed` and `Promoted`, Share… for `Unpublished`.

**The attempt entry** (R-23). Each repository has at most one attempt entry
in the operation area: the latest attempt, replaced by the next. It is shown
only when it has something to say: changes brought in, a disposition listed
above, a retirement summary, a failure or attention. A quiet "up to date"
attempt shows no entry and updates the Settings row only.

## Part B: The Worker (`D/services/`)

### B1: An automatic poll is a `Work` variant (R-1, R-2)

`Work::Poll{ root, operation, restart }` on the background lane. It is not an
`Intent`, because it does not go through the mutation boundary (decision 1).
The worker, holding `SessionCredentials`, calls thin `Backend` methods, one
per library call, so that a recording double can list them (R-30):

```rust
fn resolve(&self, root: &Path) -> Result<ResolvedRepository, ReadError>;
fn polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError>;
fn poll_eligibility(&self, root: &Path) -> Result<AutomaticPollEligibility, RepositoryError>;
fn poll_automatic(&self, request: PollPublicationRemoteRequest,
                  session: &mut SessionCredentials<DesktopProvider>)
                  -> Result<RemotePollResult, RemotePollError>;
fn remote_snapshot(&self, root: &Path) -> Result<RemoteSnapshot, RepositoryError>;
fn cancel_poll(&self, root: &Path, op: OperationId) -> Result<(), RepositoryError>;
fn resume_after_recovery(&self, root: &Path) -> Result<(), RepositoryError>;
```

The automatic path calls the first five. `cancel_poll` is called only by
B2's cancel task and by quit, and `resume_after_recovery`
(`resume_remote_polling_after_recovery`) only by Resume checking (C2).

A desktop function, tested on its own, converts the answers into the
desktop's `PollAttempt` (completion, A4's classification, `completed_at`,
`next_eligible_at`, targets as `(item_id, kind, disposition)`,
`index_pending`). Tests fabricate library answers where they can and
`PollAttempt`s where they cannot (decision 9). The worker emits
`WorkerEvent::Polled(root, PollAttempt)` after it clears its running job, as
it does for `Finished` (Fact 36). The drain applies it to the scheduler and
notifies `Operations`, so that a waiting quit sees the poll end (Part D).

An automatic poll sends no progress events. Its only UI states are Running and
its answer.

### B2: Preemption (R-3, R-25, R-34)

Three review rounds found blockers in this mechanism, so, as the planning
procedure asks, its contract is stated as required outcomes. Task 4 is
test-first, after a design checkpoint.

Every way a user-lane job enters the worker (`submit`,
`submit_with_request_id`, `requeue` and `observe_host`, Fact 36) goes through
the shared admission step. There it sets a worker-side "preempt asked" flag
for the running poll's operation and sends `WorkerEvent::PreemptPoll{root,
op}`. The `Services` drain, which holds the backend and an executor, spawns
the cancel task on a background executor, never on the UI thread.

| # | Situation | Required outcome |
| --- | --- | --- |
| P-1 | A user job is admitted while a `Work::Poll` runs | The task calls `cancel_poll(root, op)` and sends it again on a short injected schedule **until the poll answers**. An `Ok` does not end it: `cancel_remote_operation` answers `Ok` without effect on a row that is not active (Fact 30). A `RepositoryBusy` or a missing-row error is sent again. |
| P-2 | The poll is between attempts of a same-ID restart (after `Busy` or an interruption) when the flag is set | The worker checks the flag before every `poll_automatic`. With the flag set it does not restart; the poll is rescheduled as preempted. A cancel can never be lost in the restart's `interrupted` window. |
| P-3 | The poll answers `Cancelled` | The user job runs. The poll is rescheduled with no failure backoff, under a new operation ID. |
| P-4 | A poll was preempted three times in a row | It waits for the interval instead of restarting when the user lane empties (R-34). |
| P-5 | A background job is submitted while a poll runs | Nothing is cancelled. |
| P-6 | A background `ContextRetire` runs when a user job arrives | Nothing is cancelled (decision 20). |
| P-7 | Quit, or a reconfiguration (R-11), while a poll runs | The same task and flag, so P-1 and P-2 hold. |

The cost is the safe-point latency: milliseconds in a transfer (250 ms
throttle), up to the transport timeout in a blocked call (K-1).

### B3: Restart, interruption and stale rows (R-4, R-24, R-31)

`Busy` and interruption answers resend the **same** operation ID with
`restart=true`, after 5 s, 15 s and 60 s, then fall back to the normal
interval with a new ID and no failure backoff (the library stores none for
these, Fact 6). The delays are model values under the injected clock, not
sleeps. The scheduler never takes over another operation's row: a `Busy`
answer for an operation the in-flight record does not hold only waits.

**The in-flight record** (R-31). The desktop keeps, in its own data
directory and never in a repository, a small versioned record of its
unfinished remote work, written atomically (to a temporary file, then
renamed). An unreadable record is set aside, replaced, and reported once in
Activity.

- **Poll entries: at most one per repository**, root and operation ID. It is
  written before a poll is submitted, replaced when a new ID is used, and
  removed after any answer once `polling_status.active_operation_id` is no
  longer that ID, or on a terminal answer.
- **Retirement entries**: request ID, root, kind, item ID, and the operation
  ID from the request's first progress event. Written before the request is
  submitted. Removed only when the worker's record for the request is
  `Finished` (Fact 36), or on `Removed` or `NotSent`; an `Accepted` (Owed)
  request keeps its entry.
- Only the holder of the data directory's instance lock writes it. A second
  instance keeps its in-flight work in memory, takes nothing over and runs
  no retirement loop (K-25). The lock is tried at launch and again on window
  activation; once acquired, L-1 runs.

**Stale rows.** Three review rounds found blockers in this mechanism, so it
is stated as required outcomes, and Task 4 is test-first with a design
checkpoint.

| # | Situation | Required outcome |
| --- | --- | --- |
| L-1 | The lock holder's record has a poll entry for a repository | While the entry exists, its operation ID is that repository's next poll, sent with `restart=true` whenever the repository is due, at launch or later in the session, until a terminal answer or until `active_operation_id` is no longer that ID. A takeover finishes the poll. A replayed terminal answer whose `completed_at` is older than this launch counts as no attempt: no status line, no loop. |
| L-2 | As L-1, but the repository is not eligible | No takeover is possible yet (Fact 34). The row says why, per cause: paused, "An interrupted check is holding this repository. Turn checking on to finish it."; held, "… Resume checking to finish it."; not due, "… It finishes at the next check."; off, "… It finishes once checking is on." The entry stays, and L-1 applies as soon as the repository is eligible (decision 27, K-22). |
| L-3 | A restart answers `OperationMismatch`, or `recovery_required` while `active_operation_id` is not the entry's ID | The row is not this app's any more: a configuration fence interrupted it (Fact 29), or it was replaced. The entry is removed, and the next poll uses a new ID. "Not ours" is judged by `active_operation_id`, never by the code alone. |
| L-4 | At launch, a Check now's row is left active (its request was recorded by the boundary) | Its pending row offers Resume, through a change to `resume_offer` for that row (Fact 34). Resume sends `OperationResume`, and on `original_request_required` resends `PollOnce{root}` under the named request ID, on the user lane; its `rerun` restarts the row (U-15). Until then the status links to that pending row. |
| L-5 | Any time, `Busy` from an operation the record does not hold | Wait on R-4's schedule, then the interval. Never a takeover. |
| L-6 | A second instance (no lock) | It never takes a row over, writes no record and runs no retirement loop. |

### B4: What a poll may call

The automatic path calls exactly B1's first five methods (A8). B2's task and
quit may call `cancel_poll`; L-1's takeover is a `poll_automatic` like any
other. Nothing else, so nothing can push, checkpoint, merge, clean up or
retire from a poll. The automatic poll itself never calls `ContextRetire` or
`PollOnceRetiring`. Each retirement is a separate per-item request that the
scheduler submits afterwards (E1, R-19).

### B5: No background job prompts (R-26, decision 6)

`automatic=true` does not stop a first prompt. The session blocks a key only
after the provider has answered once, and every unlock starts by clearing
that block (`session.rs:350`; Fact 4). So the desktop provider itself
refuses: the worker tells it the running job's lane (Fact 15).

- For a background job, a passphrase request is answered `Unavailable` at
  once, with no `Prompt` event and without marking the key declined. This
  covers `Work::Poll` and every background `Intent`, including a retirement
  (E2).
- The provider records the key it refused against the running request, as
  `DeclinedKeys::record` does for a declined key (`prompts.rs:143-148`).
  That gives a retirement's `unlock_required` answer, which carries no key, a
  key to attach attention to.
- The library then answers `unlock_required` (or `UnlockCancelled` /
  `ProviderUnavailable` for a poll), which becomes the key's attention (C3).
- Host approval never reaches the provider (Fact 15). A poll's host answer
  carries `e.authority` (A4). A retirement's `host_approval_required` is keyed
  by the repository's remote authority from its remote read (U-8), or by the
  repository when that read has none.

The library's session caches one passphrase at a time and evicts it for
another key (Fact 35). With two protected keys, an unlocked key can need
unlocking again after a job using the other. That is K-23 (decision 25),
pinned by a two-key test.

### B6: When the scheduler polls (R-27)

`Services` gains a polling switch that is off unless switched on. Production
`run()` switches it on; the test constructors do not, so no existing test
starts a poll (Fact 31). D6's own tests switch it on. `launch_refresh` works
the other way round (on by default, `without_launch_refresh` opts out), so
the two switches are separate.

The scheduler polls only repositories whose remote read shows an SSH
publication remote (U-2). Others show "No shared copy, nothing to check" and
are never polled, since a poll without a remote writes state (Fact 31).
Remotes are read again on window activation and on a clock event, so a
remote added through the CLI is seen without a relaunch.

## Part C: Controls And Status

### C1: Intents (decision 5)

`Intent::PollOnce`, `PollConfigure{interval_seconds}`, `PollPause`,
`PollResume` go through the boundary on the user lane, with every exhaustive
match of Fact 14.

- Check now submits `PollOnce`; its retirements are Part E's loop.
- `PollOnce` is `stops_at_safe_points: true` in the library's command table
  (Fact 14), and so is the desktop arm, so Cancel and Give up work.
- The policy verbs are `writing`-only commands: the "not yet accepted"
  branch handles a command that is never accepted.
- `PollOnce` may prompt: it is the person's action.
- After `PollOnce`, the scheduler re-reads `polling_status` (`PollOnce`
  answers a `PollMutationDto`, Fact 1); a policy verb's `PollPolicy` answer
  carries the policy itself.
- A policy verb sent again follows C5 decision 15 (last commit wins).

**Check now and a running poll.** Check now for a repository whose automatic
poll is running is a user job, so B2 cancels that poll and Check now runs.
Check now is disabled only while this process's own Check now for that
repository is queued or running. While a retirement of the repository is
pending, Check now answers `recovery_required` like a poll (Fact 32), and the
row shows the pending row's link instead.

### C2: Settings page "Checking for updates"

A fifth `SettingsPage`. The page's intro says what checking does: "Manyhands
checks your shared copies for other people's changes and brings them in when
it's safe. It never shares anything. It also removes this computer's copies
of tickets closed and documents promoted elsewhere, unless you have unsaved
or kept work in them." One row per repository (handoff 61–68):

- Name; status line from the model (A4); interval select
  (Every 1, 5, 10, 15, 30, 60 minutes, all inside 60–3600 s); **Check now**
  (**Retry now** after a failure); a switch (on = unpaused).
- A repository with no SSH publication remote: "No shared copy, nothing to
  check" and Set up sharing… (opens D1's remote setup).
- A held repository: "Checking is held until recovery" and Resume checking
  (decision 10). Resume checking calls the public
  `resume_remote_polling_after_recovery` through the backend, on a
  background-executor task, never the UI thread (R-13).
- A needs-attention row carries Unlock…, Choose a key…, Approve host… or Open
  remote settings… (C3).
- A repository held by an interrupted check (L-2) says so and offers turning
  checking on.

No global switch (decision 8, K-4). The switch is Pause/Resume because no verb
sets `enabled` (Fact 2); a repository whose `enabled` is off shows "Checking is
off" with no control (K-5).

D5's lifecycle way-out `PollOnce` ("Check origin for others' changes, then
prepare again", `D/view/lifecycle/prepare.rs:100`) gains a Check now button
for the item's repository (Document Amendment 3).

### C3: Needs attention (R-8, decision 6)

The operation area (D1) gains one entry per attention reason: per key, per
host, or per repository. The entry names the repositories it holds back. Its
action is the person's:

- **Unlock…** first clears the key's declined mark, through a new production
  per-key clear: today the only one is test-only, and `clear_declined_for`
  needs a request (Fact 15). It then picks the first of the key's
  repositories whose next step reaches authentication:
  - one with a pending retirement in the in-flight record: Unlock… resends
    that retirement on the user lane (E4), since a poll there would answer
    `recovery_required` before authenticating (Fact 32);
  - otherwise one with no pending local operation and no hold: Unlock…
    submits a user-lane Check now there, which prompts through the existing
    modal;
  - if there is none, Unlock… is disabled, and the entry points at the
    pending rows that hold the key's repositories.
- An accepted unlock clears the key's attention for every repository using
  it. The proof is the poll's own success (U-8), or a user-lane retirement
  that got past its observation (answered `retired`, `already_absent`, or a
  refusal code that is not an unlock, key or host code). Those repositories poll at their
  stored deadlines; an unlock failure had moved them by the interval
  (Fact 7). A failed unlock answers `UnlockRequired` again and the attention
  returns. If the chosen repository cannot be reached, the passphrase is not
  cached and the attention stays (K-26).
- **Choose a key…** opens the repository's key setting; **Approve host…** uses
  D1's host approval; **Open remote settings…** opens D1's remote setup for
  `ConfigurationRequired`. Each then submits Check now.
- A completed poll of a repository clears any attention for that repository,
  however the cause was fixed. Since `due()` skips a repository with
  attention (A3), a fix made through the CLI is seen at the person's next
  Check now.
- A key or remote change clears the state (R-11).

No modal ever opens from a background job (B5, contract 3).

**Prompts are never doubled.** The one worker runs one job at a time, so at
most one prompt is open. Background jobs never prompt (B5). Cancelling Check
now's prompt marks the key declined for the session (Fact 15), and the poll's
answer becomes attention. Later automatic polls and retirements do not
prompt, and Unlock… clears the mark and prompts again.

These reasons are the ones Wave 04's footer counts (decision 11, W-1),
through `attention()` (A3).

### C4: Status truthfulness (R-9)

Text comes from the model's `last` and `phase`. At launch `last` is restored
from the library (U-3). The page says "Checking…" only while this process's
worker runs a poll; it does not infer that another process is polling. A poll
by `manyhands-cli` appears only through the stored latest attempt.

## Part D: Shutdown

`Workspace::request_quit` keeps its order: unkept-draft asks, draft flush,
`worker().quit()`. D6 changes six things:

1. **Nothing new starts once quitting is asked** (R-33). A "quit asked" flag
   is set on entry to `request_quit`, before the unkept ask, which opens
   while `QuitPhase` is still `Running` (Fact 16). While the flag is set the
   scheduler arms no timer and submits no poll and no loop item. Cancel on
   either ask clears it and resumes the scheduler.
2. **Cancel the poll.** `quit()` also cancels a running `Work::Poll` through
   B2's cancel task and flag (P-7), since an automatic poll has no token
   (Fact 3), and queued retirements are dropped (K-18). `quit_if_finished`
   calls `quit()` on every `Operations` notify (Fact 36), so the cancel is
   started once per operation, not once per call.
3. **A running retirement finishes** (proposed decision 28, R-38). `quit()`
   does not cancel a running background `ContextRetire`: stopping it
   part-way would leave the item pending, and the pending local operation
   would then hold the repository's polls at the next launch (Fact 32). The
   quit waits for it, as for any `Finishing` job, with "still stopping" at
   ten seconds and Quit now available. Quit now leaves it pending (K-18).
4. **The poll's end is seen.** `Polled` is sent after the worker clears its
   running job and notifies `Operations` (B1), so `quit_if_finished` runs
   when a poll ends (Fact 16). While a poll is the job being finished,
   `finishing_text` says "Finishing a check for updates".
5. **Still stopping.** On entering `Finishing`, a ten-second timer (injected
   duration; `cx.background_executor().timer`, Fact 16) changes
   `finishing_text` to "Still stopping — waiting for Manyhands to reach a safe
   point. Your work is saved." The UI stays interactive; the Quit now button
   stays (decision 7). Nothing is cleared or reclaimed at ten seconds. The
   text changes in `finishing_text` only, the quit feedback's one element
   (`D/view/quit.rs`, drawn by `D/view/workspace/render.rs`), so it goes
   where Wave 04 Cycle 01 puts that feedback: the quit's entry in Activity
   (W-2, decision 12).
6. **Join on every graceful exit.** The worker keeps its `JoinHandle` in
   `Shared` (today `spawn` drops it, Fact 13) and gains an explicit "shut down
   and join": it sets `shut_down`, notifies, and joins. A graceful exit calls
   it before `cx.quit()`: `finish_job` when `quit()` answers `Idle`, and
   `quit_if_finished` when a `Finishing` job ends (A9). Both reach `cx.quit()`
   through `quit_now`'s code today, so the join goes in the graceful callers,
   not in `quit_now`. **Quit now** itself is unchanged: it exits at once, and
   the library's journals recover the interruption. A test joins through a
   seam that does not park the GPUI test executor (Fact 36).

On relaunch the stored deadline governs (decision 23), and the in-flight
record's stale rows are handled by B3's L-1 to L-4.

## Part E: Retiring Completed Contexts

The library ticket assigns the explicit desktop Retire action to D6 (Wave D6
section, amended 2026-11-06). The owner then decided, on 2026-11-09:

- Polls retire, with the outcome of the CLI's `poll once --retire-completed`,
  always on and with no setting (decision 16).
- Every operation is per item, and a repository operation is a loop over its
  items (decision 17).
- An item with kept drafts is held back (decision 18).
- An item's tabs are read-only while it is retired (decision 19).
- A background retirement is not preempted (decision 20).
- A pending item stops its repository's loop (decision 21).

The library decides everything about safety. The desktop decides only which
items to submit, what to keep first, and how to show the answer. The poll
itself stays the library's one fetch per repository with per-target
dispositions; decision 17 adds no per-item poll.

### E1: After a completed poll, one loop over the reported items (decisions 14, 16–21; R-17, R-32, R-36, R-37)

Every completed attempt reports per-target dispositions: an automatic poll
(A4), or Check now's `PollOnce` (`PollMutationDto.targets`). If the attempt
answered `IndexPending`, R-10's `IndexRefresh` runs first. A cancelled or
interrupted attempt starts no loop, and nor does a second instance (L-6).
The loop takes the `Closed` and `Promoted` targets in the order the attempt
reported them, one at a time, and submits the next only when the previous
has answered.

Three review rounds found blockers in the loop's rules, so they are stated
as required outcomes, and Task 11b is test-first with a design checkpoint.
"Skipped" means the item gets Retire… (E4) and nothing is submitted for it.

| # | For the next reported item | Required outcome |
| --- | --- | --- |
| RT-1 | A retirement of the item is queued, running or pending (in the worker or the in-flight record) | Nothing new |
| RT-2 | The item is in the session's refused set (R-21) | Skipped |
| RT-3 | The item is being edited: any open edit session, dirty or clean, a begin in flight, comment composer text, or repair text in its Inspect tab (Fact 28) | Skipped |
| RT-4 | D2 stores a draft of the item or of a comment on it, a live session's included | Skipped (decision 18) |
| RT-5 | The draft store is unavailable, or its listing has not landed or failed (Fact 28) | Skipped: an unknown listing counts as a draft (R-36) |
| RT-6 | None of the above | The item is marked "Retiring…" (decision 19) in the same UI update. The draft store is listed again off the UI thread, and RT-3 to RT-5 are checked again on the fresh listing. If they still pass, one `Intent::ContextRetire{root, kind, id}` (Fact 24) goes on the background lane under a **new** request ID (R-28), recorded in the in-flight record first (R-31). Otherwise the mark is cleared and the item is skipped |
| RT-7 | The previous answer was `retired`, `already_absent`, `refused` or `preserved` with a refusal code, or a retry code other than a transport one (E3) | Go on to the next item |
| RT-8 | The previous answer was `pending`, `Accepted` (Owed), an attention code, or a transport or remote-unavailable code (E3) | Stop the loop for that repository until the next completed attempt (decision 21, R-37). The next item would meet the same answer, as the CLI's single pass would |
| RT-9 | The previous request ended `Removed` or `NotSent` (quit, R-11, Cancel) | Stop the loop until the next completed attempt |
| RT-10 | A reconfiguration of the repository is submitted (R-11) | Drop the whole loop at once |
| RT-11 | A Check now of the repository completes during the loop | Its attempt replaces the loop's remaining items |

An item that passes RT-3 to RT-5 has no edit session and no draft, so the
loop flushes nothing. An item is retired only after its own check, and only
if this attempt reported it, so an edit or a draft holds back nothing but its
own item.

**One loop per repository** (R-32). While a repository's loop runs, the
scheduler starts no automatic poll of it, and the loop always ends by RT-8 to
RT-10 or by running out of items, so the repository is polled again.

A pending retirement makes the repository's polls answer `recovery_required`
(Fact 32), so A4 shows "Waiting for an unfinished operation" until the
pending row is settled. When it is settled, the repository is due again at
once.

Check now itself stays `PollOnce` on the user lane, and its retirements are
the same background loop. The desktop does not use `PollOnceRetiring`: one
pass covers every candidate it finds and cannot leave out an item (decision
14). The price is one observation per retired item instead of one per pass
(K-15).

**Read-only while retiring** (decision 19). The "Retiring…" state is keyed by
(root, kind, id) and counted per request, so one request ending does not
clear another's. It applies to every `ContextRetire` of the item: a loop
item, a Retire…, a Resume resend or an Unlock… resend. It also holds while
the item's retirement is pending, including after a relaunch, from the
in-flight record. It covers every tab of the item, one opened during the
retirement included, and Restore from the Drafts page. While it is set, the
item takes no new edit session, comment or restore. A request's share of it
clears on that request's `Finished` (unless the answer is `pending` or
`Accepted`), `Removed` or `NotSent`, and when the loop drops it.

### E2: A background retirement neither prompts nor yields (decision 20)

`ContextRetire` takes a fresh authenticated observation (Fact 24). On the
background lane it never prompts (B5). An attention answer becomes that
key's, host's or repository's attention (C3), and the repository's loop waits
for the next completed attempt. Check now's own poll is on the user lane and
may prompt first, so its loop usually finds the key unlocked, unless another
key's job evicted it meanwhile (K-23).

A background retirement is not preempted by a user job (R-3 cancels only an
automatic poll), because stopping it part-way leaves the item pending. A
user job waits for the one retirement that is running, up to one observation
and its local removal (K-17). The rest of the loop is queued behind the user
job.

### E3: The answer (U-10)

`MutationDataDto::ContextRetirement(ContextRetirementMutationDto)`, one per
item (Fact 25). The desktop reads the worker's **record state** first
(`Finished` or `Accepted`), then the answer's **state**, then its **code**
(retirement's codes come from `transport_code`, `run.rs:1049-1069`, and the
refusals before its claim, Fact 32):

| Answer | What the person sees | Then |
| --- | --- | --- |
| `retired`, `already_absent` | "Retired the copy of {item} on this computer." in the repository's attempt entry | The repository's lists and tabs reload (`Changed` is per repository, Fact 17); the item's tab reads primary's copy (U-11) |
| `pending`, or the request is left `Accepted` (Owed), any code | "Retiring {item} didn't finish." For `discovery_pending`, also "The lists are still catching up." With an unlock, key or host code, that attention is raised too | D5's pending row offers Resume (E4) and Abandon (confirmed; the preview shows the retirement's details, Fact 18). The entry stays. The loop stops (RT-8) |
| `refused`, code `unlock_required` or `unlock_failed` | Attention `KeyLocked` for the key B5 recorded, or for the repository's selected key when none was recorded (a user-lane request) | Not in the refused set; the loop stops; the next completed attempt tries again |
| `refused`, code `no_selected_key`, `key_unavailable` or `key_rejected` | Attention `KeyProblem` for the repository | as above |
| `refused`, code `host_approval_required` or `host_replacement_required` | Attention `HostUnapproved` (B5) | as above |
| `refused`, code `invalid_remote` or `publication_remote_required` | Attention `ConfigurationRequired` | as above |
| `refused`, code `transport_unavailable` or `remote_unavailable` | Nothing per item | Not in the refused set; the loop stops (RT-8); the next completed attempt tries again |
| `refused`, code `index_unavailable`, `external_change`, `internal_error` or `cancelled`, or `recovery_required` with no removal | Nothing per item | Not in the refused set; the loop goes on; the next completed attempt tries again. An item refused `recovery_required` three times in one session joins the refused set |
| `refused` or `preserved`, any other code | The item row, the desktop's existing text for that code, and Retire… (E4) | Added to the session's refused set (R-21) |
| No `ContextRetirement` data, with the request `Finished` | The code, as for any failed request | Not in the refused set; the next completed attempt tries again |

`{item}` is the short code the lists already read for that id, or the id. It
is never a title. Rows wrap inside Activity (W-6). A request that is left
`Accepted` whatever its answer (`busy` or `poll_yielding` after a claim, or
no data) is handled as `pending`, never as a retry.

**Rows and the count** (R-23, W-1). Every background `ContextRetire` is a
request in `Operations`, which Cycle 01's `needs_attention` counts when it
finishes `Blocked`, `Partial` or `Error`.

- A request answered in an attention or retry row, or retired, is dismissed
  automatically, user-lane requests included. Attention or the next attempt
  covers it, so it is neither shown twice nor counted twice.
- `refused` and `preserved` ones with a refusal code stay, and they count.
- Requests and pending rows are grouped by operation ID, so a pending
  retirement counts once, however many requests (a Resume resend among them)
  share its operation.
- The worker's running operation is filtered out of the pending lists, so a
  retirement or poll in progress is never shown or counted as pending.
- Two `ContextRetire` requests under one request ID are one: the worker's
  in-flight twin check covers `ContextRetire` too (Fact 36).

### E4: Retire… for what is left (decision 15, R-18)

Retire… shows on every reported `Closed` or `Promoted` item that the loop did
not retire. That covers items skipped (being edited, kept drafts, an unknown
listing), refused, preserved, failed before any effect, or not reached
because the loop stopped. It does not show while the item's retirement is
queued, running or pending (RT-1); Resume and Abandon cover a pending one.
It submits the same `Intent::ContextRetire` for that one item on the user
lane, under a new request ID recorded in the in-flight record (R-28, R-31),
so it may prompt. This is the explicit desktop action the Wave requires.

1. **It checks RT-1 again** when it submits, and marks the item "Retiring…"
   before anything else.
2. **Unkept text stops it.** If a tab of that item holds text that no draft
   keeps, nothing is submitted. A dialog gives the count, with no title or
   text, and offers **Show tab** (the default) and **Cancel**. There is no
   "Retire anyway". It is a new ask variant, since the quit asks hard-code
   Cancel and Quit anyway (Fact 28).
3. **Kept drafts are the person's choice** (decision 18). If the item has a
   kept draft, a dialog says: "This ticket has N kept drafts. After it is
   retired you can still export them, but not restore them into it." It
   offers **Retire** and **Cancel** (the default). For a promoted document:
   "You can still restore or export the document's drafts. Drafts of
   comments can only be exported."
4. **Drafts are flushed** for the item (`flush_now` over its sessions,
   Fact 28), while it is read-only.
5. **The answer** follows E3's rows. A Retire… removes the item from the
   refused set.

**Resume of a pending retirement** (Fact 26). Resume first clears the
declined keys of the request's last attempt (`clear_declined_for`, Fact 15),
so its resend can prompt. Then, if Resume answers `original_request_required`,
the desktop resends the original: it rebuilds `Intent::ContextRetire{root,
kind, id}` under the named request ID and sends it as a **new user-lane**
desktop request, as `D/services/resends.rs` does for lifecycles. Kind and id
come from the in-flight record (R-31), matched to the pending row by the
operation ID the entry stores; the read side has neither (Fact 33). Without
an entry, Resume is still offered for a retirement that owes only its index
hand-off, which needs no original request (Fact 26); otherwise only Abandon
is (K-27).

### E5: Drafts survive (contract 11, decision 18)

Retirement cannot touch `src/drafts/`, which lies outside every repository.
After retirement, the item's kept drafts and its comments' drafts stay on
the Drafts page with Open and Export…. Restore follows D2's restore policy:

- A closed ticket's drafts (`TicketClosed`) and every comment draft cannot
  be restored.
- A promoted document's draft is restorable, flagged as changed.

D6 adds no draft handling of its own. The automatic loop never retires an
item with kept drafts, or one whose listing is unknown (RT-4, RT-5), so this
happens only after the person chose Retire….

### E6: Where retirement comes from (R-19)

Retirement starts only from E1's loop, a Retire…, a Resume resend or an
Unlock… resend (C3, E4). The automatic poll's own call list stays Part B4's.
The loop submits a retirement only for an item a completed attempt reported
as `Closed` or `Promoted`. There is no setting to turn it off (decision 16).

### E7: Reads before retirement (K-12)

Until a context is retired, the item's own worktree copy is what reads see
(Fact 27). So a ticket closed elsewhere can still show as open until its
retirement after the poll that found it. D6 does not change read precedence.

## Wave 04 Coordination

Wave 04 Cycle 01 (`01M6V8RK8B0SBCH0YZXFVSR9E3`) was approved on 2026-11-08
(revision 5) and is in implementation, so D6 is designed to merge second and
adapt. Each item says what D6 does in either order. Sources: Cycle 01's
approved Cycle document (scope 4, 7 and 12; decisions 3, 7 and 14; K-18), its
note on this ticket (comment `01M6Z8A8AYDGTK4BWKZR3931ST`) and its checkpoint
comment `01M6ZXNJFRHZP9381NQQE3FH0F` (proposed K-19). Cycle 01's branch also
carries the matching Wave 04 amendments, so D6 edits Wave 04 only at its own
approval (Document Amendment 8). At `e24d8ce9` Cycle 01's branch had not yet
changed `lifecycle.rs`; W-2's focus rule and its A7 test exist in its plan
only, so Task 0 re-checks them.

- **W-1. Needs attention count.** Cycle 01's footer shows
  `needs_attention(&Operations, &DraftRegistry) -> usize`
  (`D/model/attention.rs`). It counts requests that finished `Partial`,
  `Blocked` or `Error` and are not dismissed, draft flush failures, one for
  drafts unavailable, pending rows, and one per failed pending read. D6's
  model adds `attention()` (A3): one entry per locked or missing key, per
  unapproved host, and per held or misconfigured repository, and nothing else
  (decision 11). A key locked for three repositories counts once.
  `needs_attention` does not deduplicate today, so D6 adds three rules, the
  only change it makes to how that function counts: requests and pending
  rows are grouped by operation ID and count once; a request whose answer
  became attention or a retry is dismissed; the worker's running operation is
  not listed as pending (E3, R-23). With
  Cycle 01 on main, Task 8 makes the polling model a third input of
  `needs_attention`, adds its count, and extends that function's test
  (E-4). Otherwise Cycle 01 adds the term when it merges. Retire needs no
  term: its refusals and pending rows are counted already, and its
  before-effect answers are dismissed so they are not counted twice (E3,
  R-23).
- **W-2. Quit feedback.** A quit that has to wait changes no layout (Cycle
  01 decision 7). Its entries are in Activity, visible when Activity is, and
  with Activity hidden nothing shows until it is opened (Cycle 01 K-18). D6
  keeps "still stopping" inside `finishing_text` (Part D) and adds no other
  place and no panel change (decision 12, K-13). With Cycle 01 on main, Task
  10's still-stopping test hides Activity first. After the injected ten
  seconds it asserts that the layout is unchanged and that `quit-finishing`
  carries the still-stopping text (E-5).
- **W-3. Footprint in shared files.** The scheduler entity is held by
  `Services` (R-16), so `Workspace` gains no field for it. The launch poll is
  one call to the scheduler beside the launch refresh in
  `repositories_loaded` (`D/view/workspace/repositories.rs`), or wherever
  Cycle 01 moved that refresh. Window activation, which re-reads policies
  (Task 5), is one subscription made in `open_main_window`, because
  `Services::start` has no window. Expected textual conflicts:
  - `repositories.rs`;
  - `D/view/workspace/lifecycle.rs` (`wait_to_quit`, `not_saved_chosen`);
  - the `Workspace` struct and constructor;
  - `D/view/harness.rs` (window size, `SHELL_CHROME_HEIGHT`,
    `Shell::use_list`);
  - the keyboard tests (`Region::Navigation` becomes `Region::Tickets` or
    `Region::Documents`, and the F6 order changes).
- **W-4. Settings page.** "Checking for updates" is a fifth `SettingsPage`
  with its rows in `D/view/settings/polling.rs`, using no route but
  `SettingsPage`, so Wave 04 Cycle 04 can move it into the sheet. Cycle 01
  keeps Settings as a page in the item area and adds a footer button for it.
  `secondary-,` reaches it in both layouts, and Task 13 uses it.
- **W-5. Surfaces for Wave 04 Cycle 03.** The footer's "Checked for updates
  …", the Repositories panel's "Checking…" and "Paused", Check all for updates
  now and the popover are Cycle 03's. D6's model answers, per repository, the
  status text of A4 and the last attempt (`AttemptSummary`), so Cycle 03 reads
  them instead of deriving its own. Check all can submit Check now
  (`PollOnce`) per repository; D6 adds no separate action for it.
- **W-6. Rows that fit Activity** (decision 13). At the default 1280 px
  window, Activity is about 550 px wide (Cycle 01's proposed K-19). Each row
  D6 adds to the operation area wraps so its actions stay inside it: the
  attention entries, the attempt entries with their dispositions, the Retire
  offers, and Resume checking. The three existing unwrapped rows of K-19 stay
  with Wave 04 Cycle 03 (K-16). A behaviour test clicks each D6 action at
  its position at the default window size: a hit test, which fails if the
  action is clipped. The actions are Unlock…, Choose a key…, Approve host…,
  Open remote settings…, Share…, Resume checking, Retire… and the pending
  row's Resume and Abandon. No test measures appearance. Wave 04's Cycle 03 section
  records that it owns the three existing rows (Document Amendment 8).
- **W-7. Later quit work.** The owner's blocking quit dialog (Wave 04
  later work) replaces Quit now and Quit anyway. D6 keeps Quit now (decision
  7), and its still-stopping text is the wording that dialog inherits.

## Required Outcomes

| # | Outcome |
| --- | --- |
| O-1 | With the app open and a due repository, one automatic poll runs on the background lane and updates lists and tabs. |
| O-2 | A user job submitted during a poll runs after the poll's next safe point, and the poll returns later without failure backoff. |
| O-3 | A locked key or unapproved host, met by a live poll that answers `Err(Transport)`, produces a needs-attention state and no modal, even in a fresh session; pause stays as it was. |
| O-4 | A bring-in over an item with unsaved edits or a kept draft leaves the text and shows the changed notice, proved over a real repository whose file changes on disk. |
| O-5 | `busy` and `interrupted` answers restart the same operation on the bounded schedule; a row this app recorded and left active is taken over, at launch or when its repository is next due, and no other row ever is (L-1 to L-6); an unfinished local operation never makes the scheduler poll in a loop (R-35). |
| O-6 | A suspend or clock jump produces at most one poll per repository. |
| O-7 | Quit mid-poll cancels, flushes drafts, shows "still stopping" in the quit's Activity entry at ten seconds and exits when the poll returns; the worker thread is joined on every graceful exit. |
| O-8 | An automatic poll calls no service function beyond Part B4's. |
| O-9 | Configuration changes (interval, pause, resume, remote, key, removal) take effect without a stale poll using the old configuration. |
| O-10 | After a poll, automatic or Check now, each reported `closed` or `promoted` item is retired by its own request, one at a time, and the lists and an open tab then read primary. |
| O-11 | Unkept text or a kept draft holds back that item only; its tabs are read-only while it is retired; kept item and comment drafts outlive retirement, listed and exportable, and restorable where D2 allows. |
| O-12 | A refused or preserved item offers Retire… and is not tried again automatically this session; a pending one stops its repository's loop and leaves a pending row whose Resume resends the original request. |
| O-13 | No background job opens a prompt, a poll or a retirement; a locked key or unapproved host becomes an attention state. |
| O-14 | A Final answer replayed under the same request removes nothing a second time and adds no second entry; a quit during a retirement leaves a pending row that Resume finishes after relaunch. |

## Rulings

Each ruling's cost if wrong is rework inside D6, unless it says otherwise.

- **R-1.** An automatic poll is a desktop `Work` variant, not an `Intent`
  (decision 1).
- **R-2.** The desktop has its own `PollAttempt`; library answers are
  converted by a desktop function tested on its own (B1).
- **R-3.** A user job, entering by any of the worker's four user-lane entry
  points, cancels a running automatic poll (decision 3); a background job
  does not, and nothing cancels a background retirement (decision 20).
- **R-4.** The restart schedule 5 s, 15 s, 60 s, then the interval
  (decision 4).
- **R-5.** The model is clock-injected; no test sleeps (AGENTS.md).
- **R-6.** After a suspend or jump, one poll per repository, staggered; the
  library's clamp bounds a future deadline; a bounded restart keeps its
  operation.
- **R-7.** Fair order: earliest last completion first; one repository at a time.
- **R-8.** Attention is keyed by key, host or repository, held in memory,
  never written to policy (decision 6, A2).
- **R-9.** Status never claims more than this process knows.
- **R-10.** Items fast-forwarded or materialized in an attempt mark the
  repository changed through a new `Operations` entry point (Fact 17); an
  `IndexPending` poll answer queues one background `IndexRefresh`, which runs
  before that attempt's retirement loop. A retirement's pending index is not
  this case (E3).
- **R-11.** A key, host-trust, remote or registration change (`RemoteAdd`,
  `RemoteRemove`, `RemoteSelect`, `KeySelect`, `KeyClear`, `KeyRemove`,
  `KeyDelete`, `HostApprove`, `HostReplace`, `RepoCreate`, `RepoEnable`,
  `RepoRemove`) affects every repository using that key, host or remote. When
  the change is **submitted**, those repositories' queued retirements are
  dropped. When it finishes, their polls are cancelled, their attention is
  cleared, their policies are re-read and they re-arm. A policy intent only
  re-reads the policy. Removal drops the schedule; a re-added remote leaves a
  stored pause alone.
- **R-12.** Quit follows Part D (decision 7).
- **R-13.** The page uses only C5's bindings and public reads, plus the
  public `resume_remote_polling_after_recovery` for Resume checking, called
  off the UI thread (decision 5, C2).
- **R-14.** Needs attention counts exactly `attention()`'s entries from D6
  (decision 11, W-1).
- **R-15.** "Still stopping" lives only in `finishing_text`, in the quit's
  Activity entry; D6 changes no layout for it (decision 12, W-2).
- **R-16.** The scheduler is held by `Services`; D6 adds no `Workspace` field,
  one call in `repositories_loaded`, and one subscription in
  `open_main_window` (W-3).
- **R-17.** After every completed attempt, each reported `Closed` or
  `Promoted` item is checked alone and, unless E1 skips it, retired by its
  own background `ContextRetire`, one at a time (decisions 14, 16 and 17).
- **R-18.** An item being edited, or with a kept draft (any stored draft,
  a live session's included), is skipped by the loop. An item's tabs are
  read-only while it is retired. Drafts are never removed (decisions 15, 18
  and 19; E1, E4, E5).
- **R-19.** Retirement starts only from E1's loop, a Retire… or a Resume
  resend (E4). The automatic poll's own call list stays B4's, and the desktop
  never builds `PollOnceRetiring` (E6).
- **R-20.** Where a disposition's `guidance()` names a CLI command, the
  desktop names its own action instead (A4).
- **R-21.** An item whose retirement was refused or preserved with a code
  outside E3's attention and retry rows joins the session's refused set, and
  the loop skips it. Retire… removes the item from the set (E3, E4). Cost if
  wrong: a refusal that clears by itself waits for a relaunch or a Retire….
- **R-22.** A pending retirement, or an attention answer, stops its
  repository's loop until the next completed attempt (decision 21, E1).
- **R-23.** One attempt entry per repository, replaced each time and shown
  only when it has something to say. A request whose answer became attention
  or a retry, or a retirement that retired, is dismissed automatically.
  Requests and pending rows are grouped by operation ID and count once. The
  worker's running operation is never listed as pending (A4, E3, W-1). Cost if wrong: Activity shows more or
  fewer rows than the person expects; no state is lost.
- **R-24.** A poll row is taken over only by the instance lock's holder, and
  only under an operation ID its in-flight record holds, at launch or
  whenever that repository is next due. Never for an operation the record
  does not hold (B3, L-1 to L-6).
- **R-25.** The poll cancel runs on a background-executor task spawned by the
  `Services` drain, never on the UI thread. It is sent again until the poll
  answers, and the worker checks the preempt flag before every
  `poll_automatic` (B2, P-1, P-2, Fact 30).
- **R-26.** The desktop provider refuses every background job's passphrase
  request at once, without marking the key declined, and records the key it
  refused against the running request (B5, Fact 4).
- **R-27.** A polling switch in `Services`, off unless production `run()`
  switches it on, and polls only of repositories with an SSH publication
  remote, re-read on window activation (B6, Fact 31).
- **R-28.** Every loop item and every Retire… is a new request ID. Only Resume
  names the original request, and its resend is a new user-lane desktop
  request under that ID (E1, E4, Fact 24).
- **R-29.** A poll's `Err(Transport(e))` is classified by `e.kind`, as the
  library's `transport_category` does, and attention is keyed by
  `e.selected_key_id` and `e.authority` (A4, Fact 6).
- **R-30.** The poll path is five thin `Backend` methods plus `cancel_poll`,
  so a recording double lists them (B1, B4).
- **R-31.** The desktop keeps an in-flight record of its automatic polls (one
  per repository) and its retirement requests in its own data directory,
  written atomically, only by the holder of an instance lock that is retried
  on window activation. B3 says when each entry is written, replaced and
  removed. Cost if wrong: an orphan entry costs one extra poll or one
  misleading "interrupted check" line (K-25, K-27).
- **R-32.** One retirement loop per repository; no automatic poll of a
  repository while its loop runs; a Check now's attempt replaces the loop's
  remaining items (E1).
- **R-33.** From the entry to `request_quit`, the scheduler starts nothing
  new; Cancel on either ask resumes it (Part D).
- **R-34.** A poll preempted three times in a row waits for the interval
  instead of restarting when the user lane empties (B2).
- **R-35.** An `Err(Repository)` answer, `recovery_required` included, waits
  on R-4's schedule and then the interval, never at the stored deadline (A4,
  Fact 32).
- **R-36.** An unknown draft listing counts as a draft; the store is listed
  again after an item is marked read-only and before its request; a second
  instance runs no loop (RT-5, RT-6, L-6). Cost if wrong: an item stays
  unretired until the next attempt or a Retire….
- **R-37.** The loop stops on a pending or `Accepted` answer, an attention
  code, a transport or remote-unavailable code, `Removed` or `NotSent`, and
  is dropped whole by a reconfiguration (RT-8 to RT-10).
- **R-38.** A graceful quit does not cancel a running retirement, whatever its
  lane (Retire…, Resume and Unlock… resends included; owner, 2026-11-10,
  Amendment 11); it waits for it like any finishing job (proposed decision 28,
  Part D).
  Cost if wrong: a quit can wait up to one observation and its removal.

## Testing (AGENTS.md Testing Strategy; Decision 9)

Default layer is a unit test. The cost of each layer is the point. Runs
follow AGENTS.md's Testing Strategy (2026-11-06), which overrides any broader
run: between steps, `cargo check` or `clippy` and the tests just written or
changed, by exact name. The static gates, the fast tier and the SSH tier for
`remote_polling` run once, at review-ready. Task 0 runs only the static gates
(entry-gate item 7 departure, Document Amendment 9).

1. **Model unit tests (most of the evidence).** One table-driven test per
   decision: A4's table (including each `Err(Transport)` kind,
   `Ok(Polled{Interrupted})`, `Err(Repository)` and each `NotEligible`
   reason), `due` ordering and the launch deadline, `clock_event` keeping a
   bounded restart, the restart schedule, R-34's preemption bound,
   reconfiguration, pause versus attention, `attention()`'s keys, E1's
   per-item decision and R-32's one loop.
2. **Worker tests** over a `Backend` double that records B1's calls and holds
   a poll on a gate that `cancel_poll` releases (the existing hold hooks
   cannot hold a poll, which emits no progress):
   - B2's P-1 to P-7 as one table, test-first after a design checkpoint
     (Task 4): each of the four user-lane entry points; a cancel sent before
     the row exists; an `Ok` that changed nothing because the row was
     between a same-ID restart's attempts; the flag checked before a
     restart;
   - quit mid-poll, with the worker still running at ten seconds instead of a
     "no credential drop" seam; nothing new started from the entry to
     `request_quit`, while the unkept ask is open or drafts are flushing
     (R-33); a running background retirement finishing before the quit ends
     (R-38);
   - the B4 call list, with no `ContextRetire` or `PollOnceRetiring` inside
     the automatic poll;
   - the join on both graceful paths, through a seam that does not park the
     GPUI test executor;
   - "one scheduler": a poll submitted through a clone is seen by the
     original's model;
   - **no prompt**, for `Work::Poll` and for a background `ContextRetire`,
     through the real `DesktopProvider` and a fresh `SessionCredentials`
     with a locked key. Nothing reaches the provider without SSH, so the
     double calls `session.with_passphrase` with a public `UnlockRequest`, as
     the library's transport would. The test fails if the provider is not
     lane-aware (B5);
   - two protected keys: a background job using key B evicts key A's cached
     passphrase, and A's attention returns. This pins today's behaviour
     (decision 25, K-23).
3. **Desktop tests over a real service** where no network is needed
   (decision 9 keeps canned answers for the rest):
   - a paused repository answers `NotEligible` and the row says Paused;
   - a repository without a publication remote is never polled (B6);
   - `ContextRetire` on an open ticket is refused, and nothing is removed;
   - a pending local operation makes the poll answer `recovery_required`,
     and the scheduler polls at most once per R-4 step under the injected
     clock (R-35);
   - a second library actor over the same registry, in the same process,
     holds the remote reservation, and the automatic poll answers `Busy` and
     restarts on schedule (A4; the separate process is a departure, decision
     26);
   - bring-in over a dirty session whose file changes on disk (A14);
   - retirement end states built for real in a local repository, with
     primary holding the closed ticket and the item's branch and worktree
     removed: the item's tab reads primary, and a kept item draft and a
     comment draft are listed and export (A21).
4. **Desktop behaviour tests** over canned answers:
   - the Settings rows' actions and enablement, status text for each state,
     Check now (including during a running poll and while a retirement is
     pending), the switch and the interval;
   - attention actions, Unlock… after a cancelled first prompt, and Unlock…
     resending a pending retirement;
   - still-stopping at the injected ten seconds with the UI interactive;
   - L-1 to L-6 as one table over canned library answers and a seeded
     in-flight record, after a design checkpoint (Task 4), including L-2
     followed by turning checking on, which takes the row over at runtime,
     and the in-flight record's write, replace and remove rules;
   - retirement: RT-1 to RT-11 as one table, test-first after a design
     checkpoint (Task 11b), including a launch poll that completes before
     the draft listing lands, which retires nothing; E3's rows; the refused
     set; the read-only state, its keying
     and every way it clears; the unkept and kept-draft asks; R-23's
     dismissals and dedupe; Resume's resend of the original request from the
     in-flight record; a replayed Final answer that removes nothing twice and
     adds no second entry; a quit during a retirement leaving a pending row
     (track rule 7);
   - each D6 action clicked at its position at the default window (W-6);
   - one keyboard walk.

   Behaviour only: no tests for fonts, icons, styles or that a view
   "renders".
5. **Two SSH-harness cases, by exception** (Task 1, decisions 22 and 24), in
   `tests/remote_polling.rs`: `poll_cancel_during_handshake_stall` and
   `poll_cancel_during_connect_stall`. They need SSH because the behaviour is
   transport-dependent: how long a cancel waits when a blocking call has not
   returned. Their edge cases (preemption, reschedule) are model tests. They
   run singly, by exact name. Retirement adds **no** SSH case: the library's
   `retirement_cli_authenticated_live_pass` covers `context retire` over
   SSH, and the desktop's part is decided above the transport.
6. **Native** (Linux, owner): the sections below.

Existing tests change as named edits E-1 to E-6. On 2026-11-09 the owner also
pre-approved any further edit to an existing test that execution finds
necessary: each is recorded in the ledger as E-7 onward, with its reason, and
reported at the next checkpoint. The polling switch (B6) keeps launch polls out
of existing tests.

- **E-1.** Exhaustive matches that now include poll variants and
  `ContextRetire` (Tasks 6 and 11a).
- **E-2.** The Settings page list (Task 7).
- **E-3.** `finishing_text`'s test, for the new wording (Task 10).
- **E-4.** With Wave 04 Cycle 01 on main: `needs_attention`'s test gains a
  row for each kind of attention entry, for each state that does not count,
  and for R-23's three dedupe rules, the only change to how it counts (W-1,
  Task 8).
- **E-5.** With Wave 04 Cycle 01 on main: D6's still-stopping test is the one
  extended to hide Activity first; Cycle 01's quit test is left alone (W-2,
  Task 10).
- **E-6.** D5's pending-list tests where the worker's running operation was
  listed as pending (R-23, Task 11a).

## Privacy

`PollAttempt`, `AttemptSummary`, the status model and the in-flight record
hold item ids, kinds, roots, operation and request IDs, dispositions, counts
and times. Retire offers and answers hold ids, short codes, kinds, states,
codes and removal flags. None holds a title, body, comment or key material.
No type that can hold a passphrase derives `Debug` or `Display`. A sentinel
test fails if a title or a passphrase appears in an operation entry, a Retire
offer, ask or answer, status text or the in-flight record.

## Native Checklist (Linux Wayland, Release Build, Product Owner)

Written in Task 14 from the D5 checklist's shape. Sections:

1. **Smoke.**
2. **A real SSH journey.** Another clone shares an update; the app brings it
   in within the interval; a new context materializes.
3. **Stall.** A server that accepts and never answers; the app stays
   responsive; quit shows "still stopping" in Activity at ten seconds.
4. **Lock and host.** A locked key gives attention and no prompt; Unlock…
   prompts; pause and unlock stay separate.
5. **Retire on two clones.**
   - The other clone closes a ticket and deletes its branch, and the app's
     next poll retires it with no prompt; the ticket then reads closed.
   - With an edit open on a second closed ticket, the poll leaves it and
     offers Retire…, which stops at the tab.
   - A third with a kept draft is left and offered Retire…, which warns that
     the drafts become export-only. After Retire, the draft is on the Drafts
     page and exports.
   - Check now retires another.
   - Quit while a retirement runs; at relaunch its pending row's Resume
     finishes it.
6. **Restart.** Relaunch keeps the stored deadline. Quit now during a Check
   now; at relaunch its pending row's Resume finishes it (L-4). After a
   graceful quit, no `manyhands-worker` thread or child process remains
   (check the process list).
7. **Keyboard only.**
8. **Results.**

A concurrent explicit operation from a second process is proved in the same
process by the real-service test (Testing 3; departure, decision 26). The
CLI-against-desktop run belongs to G1 (track rule 4).

## Document Amendments

Applied by the approval commit on 2026-11-09, after their line references were
checked again: 1 to 5 and 7 to 10. Amendment 6 waits for D6's merge.

1. Wave D6 section: note the decisions 1 and 2 (no library part; background
   lane) and that automatic polls need no boundary binding.
2. Application runtime and polling RFC:
   - record the restart schedule, and that a user job cancels a running poll
     because the worker is one thread;
   - the launch poll follows the stored deadline (decision 23), amending its
     "poll at launch unless backoff" wording;
   - still-stopping is reported in Activity (decision 12), in "Status and
     shutdown";
   - its Operation adapter clause ("MUST NOT turn an ordinary save, refresh,
     startup … into authorization to … clean up") gains decision 16's
     exception.
3. D5 K-12 ("`PollOnce` is guidance text until D6") is superseded by C2's
   Check now button on the way-out guidance.
4. D1/D2 quit documentation: D6 replaces the quit path's waiting text.
5. Test and compatibility strategy RFC, lines 226–229: "the UI reports still
   stopping" in Activity (decision 12). Wave D6 section and the Wave's exit
   criteria: the Retire obligation is planned as Part E (decisions 14 to
   21), and the ten-second shutdown report is in Activity.
6. API audit, "Completed-context retirement delivery": its line "D6 still
   owes explicit desktop Retire" is updated when D6 merges.
7. Decision 16 lifts the "no background cleanup" rule for per-item
   retirement only. Background push, checkpoint, merge and publication stay
   prohibited. It amends:
   - **PRD**, as a new 0.8 changelog entry rather than a rewrite of 0.7's
     line 27: the release scope (line 99, "deliberate, guarded local
     retirement"); `MH-COLLAB-004` (lines 408–410: "without removing a local
     worktree", manual precedence, and "never … deletes a branch, or deletes
     a worktree"); `MH-COLLAB-006` (bullets 1 to 5); `MH-INDEX-002` (505–508,
     "Only explicit `poll once --retire-completed` may…"); `MH-NFR-007`
     (635–637);
   - **Application runtime RFC**: the Operation adapter clause (Amendment 2);
   - **Git workflow RFC**, lines 293–295: "Default and automatic polling MUST
     NOT clean up state";
   - **CLI RFC**, lines 186, 198–200 and 512, and the `context retire` row
     ("Explicit guarded local retirement"): background polling does not
     inherit retirement consent, and execution is the deliberate consent;
   - **Retirement Cycle**: owner decisions 4 ("never background cleanup"),
     7 and 8 (consent by explicit invocation), Required Contract 5, its
     Explicit Exclusions, its last Scope bullet and A9;
   - **Wave 03**: track rule 5's 2026-11-06 amendment ("no background
     cleanup"); the capability rows "Explicit one-shot polling" and
     "Peer-completed context recognition and deliberate local retirement";
     the Integration Gate bullet on background work's permitted boundary;
     the D6 amendment ("must invoke … deliberately"; "never a schedule
     policy"; D6 still never chooses `PollOnceRetiring`) and its exit
     evidence ("No background push/checkpoint/merge/cleanup");
   - **The D6 ticket**'s Exit Evidence.
8. Wave 04: the Cycle 03 section records that it owns the three unwrapped
   operation-area rows of Cycle 01's proposed K-19 (decision 13), and that
   "Check all for updates now" submits Check now per repository (W-5).
   Applied after checking Cycle 01's own Wave 04 edits, to avoid a conflict.
9. Wave entry gate and track rules: D6's API audit timing departs from item
   3 as C4, C5 and D5 did (decision 9); item 5's DNS and teardown stalls stay
   unmeasured (decisions 22 and 24); item 7's baseline runs the static gates
   only, under AGENTS.md's Testing Strategy; track rule 4's separate process
   for the second actor departs to the same process, with the cross-process
   run in G1 (decision 26).
10. Follow-up library tickets, filed 2026-11-09: `01M70CF14M3YC6SR8T9JJ6XQBG`
    (`vh-ype2f`), a per-key passphrase cache (decision 25, K-23), and
    `01M70CF1VBHJAX6MK39FY5W2RY` (`vh-xmhsj`), ending a dead poll's row
    whatever the eligibility (decision 27, K-22).
11. Task 11b design checkpoint, owner rulings of 2026-11-10 (RT-1 to RT-11
    checked against the code at `44ac1bb1`):
    - **Decision 28 / R-38 covers any running `ContextRetire`**, not only the
      loop's (lane no longer matters). Supersedes "background" in R-38 and in
      Part D item 3.
    - **RT-5 and R-36: an unreadable draft entry is unknown.** It has no
      target and cannot be attributed, so while the listing holds one no item
      of any repository is auto-retired (the items get Retire…, E4).
    - **R-37: a finished request with no retirement data (`Verdict::Failed`)
      stops the loop** for that repository until the next completed attempt.
      `recovery_required` with no removal stays as E3 and RT-7 say (go on,
      counted toward the three-strikes refused set).
    - **A successful Check now is recorded as the repository's latest attempt**
      (its per-item targets converted in report order), and may start a loop
      from Idle, Preempted or Waiting, or replace a running one (RT-11).
    Also settled by the existing rows, without a ruling: a user Cancel of a
    running loop item stops the loop (RT-9); a quit that freezes a loop drops
    it; the loop ends at a reconfiguration's `Finished` as well as at
    `Submitted`, and does not start while one is queued or running (RT-10);
    duplicate targets in one report are checked once; E1's "due again at once"
    after a settled pending row follows R-35 (the normal schedule), which E1's
    sentence contradicts.

## Known Limits

- **K-1.** A Save or Share submitted during a poll waits for the poll's safe
  point, up to the transport timeout (about 30 s; about 45 s with an
  acknowledgement stall) when the call is blocked. Same-worker was chosen
  (decision 2); D4 decision 12 accepted the same wait behind a share.
- **K-2.** One stalled repository delays the polls of the others by its stall,
  never the UI.
- **K-3.** The desktop sees a poll by another process only through the stored
  latest attempt.
- **K-4.** No global "pause all" (decision 8).
- **K-5.** `enabled` has no verb; an off repository shows "Checking is off".
- **K-6.** Automatic polls leave no mutation-boundary request record and no
  progress events (decision 1); the library's stored attempt is the record.
- **K-7.** `IndexPending` cannot say whether the poll changed local state
  (`local_changes` is crate-private); the desktop queues one `IndexRefresh`.
- **K-8.** A retained conflict stranded by a raised generation (C5 limit)
  stays; the poll reports `ConflictPending`, shown as waiting.
- **K-9.** Quit now is an interruption, not a rollback; DNS and total transfer
  have no deadline (RFC).
- **K-10.** D6 builds no footer status, Repositories panel state or popover.
  Its status lives on the Settings page and in the operation area, which Wave
  04 Cycle 01 shows as the Activity panel; Cycle 03 adds the rest from D6's
  model (W-5).
- **K-11.** Windows and macOS stall and suspend behaviour is unmeasured (G1).
- **K-12.** Until a context is retired, its own worktree copy is read, so a
  ticket closed elsewhere can still show as open (Fact 27, E7).
- **K-13.** With Activity hidden, a quit still waiting at ten seconds shows
  nothing until Activity is opened (decision 12; Cycle 01 K-18).
- **K-14.** A Retire… offer comes from the last attempt and can be stale; the
  library proves everything again and refuses or answers `already_absent`
  (E4).
- **K-15.** Each retired item costs its own observation, where the CLI's
  `--retire-completed` pass shares one. Retirement is per item (decision
  17), and D6 adds no library part (decision 1).
- **K-16.** The three existing unwrapped operation-area rows of Cycle 01's
  proposed K-19 can still sit outside the visible Activity; D6's own rows
  wrap (decision 13).
- **K-17.** A Save or Share submitted while an item's background retirement
  runs waits for that one retirement, up to one observation and its local
  removal (decision 20, E2).
- **K-18.** Quit drops the queued retirements; the next launch's poll finds
  those items again. A graceful quit lets the running one finish (R-38).
  Quit now, or a crash, can leave it pending; its pending row offers Resume
  at the next launch, and the repository's polls wait until it is settled.
- **K-19.** An item being edited or with kept drafts is not retired until the
  person acts (decisions 15 and 18).
- **K-20.** Cancellation during a DNS or teardown stall is unmeasured
  (decisions 22 and 24); the stopping status says "waiting for Manyhands to
  reach a safe point", never a deadline.
- **K-21.** Another process's live operation keeps a repository waiting on
  R-4's schedule and then the interval, with "Waiting for another operation
  to finish".
- **K-22.** A poll killed by Quit now or a crash, in a repository that is not
  eligible, keeps its row until the repository is eligible again, or a key
  or remote change fences it (decision 27; L-2, L-3). The row says why and
  what finishes it. Until then other remote work on that repository answers
  busy or yielding. Follow-up ticket `01M70CF1VBHJAX6MK39FY5W2RY`
  (`vh-xmhsj`) can remove this.
- **K-23.** The library caches one passphrase. With two protected keys, a
  job using one key evicts the other's, whose attention returns at its next
  poll (decision 25, Fact 35). Follow-up ticket `01M70CF14M3YC6SR8T9JJ6XQBG`
  (`vh-ype2f`) can add a per-key cache.
- **K-24.** The second-process evidence runs in the same test process
  (decision 26); the cross-process run is G1's.
- **K-25.** A second instance of the app never takes over stale rows, writes
  no in-flight record and runs no retirement loop (L-6, R-36); it takes the
  lock, and those duties, once the first instance has exited and a window is
  activated.
- **K-26.** Unlock… through a repository that cannot be reached does not
  cache the passphrase, so the key's attention stays (C3).
- **K-27.** A pending retirement with no in-flight entry, such as one a
  second instance submitted, offers Abandon but not Resume (E4).

## Alternatives Rejected

- **A library automatic binding** (owner chose direct calls; decision 1).
- **A second poll thread sharing the session** (decision 2): single-owner
  credentials, a second prompt path.
- **Cancelling nothing and letting user jobs wait for the whole poll:** a
  poll of many contexts could hold a Save for minutes.
- **Prompting from the background:** forbidden by the RFC and by B5.
- **Persisting needs-attention:** an unlock is a process fact; it would be
  stale after a restart.
- **Taking a row over on any `Busy` answer, under `active_operation_id`:**
  the library checks no liveness, so it could steal a live row of a second
  instance or another process (round 2; R-24).
- **Parsing a retirement's journal `target` text for its kind and id:** the
  read side does not expose it (Fact 33); the in-flight record holds what the
  desktop itself submitted.
- **Retire anyway, closing tabs with unkept text:** it would lose text the
  contract keeps (decision 15).
- **A `PollOnceRetiring` pass, as the CLI's flag runs:** one observation for
  every candidate, but it cannot leave out one item, and it handles the
  repository as one unit (decisions 14 and 17).
- **Holding back a whole repository when any tab in it is unsaved:** the
  owner narrowed this to the reported items (decisions 15 and 17).
- **Preempting a background retirement:** it can leave the item pending
  (decision 20).
- **A per-item automatic retirement in the library** (sharing the poll's
  observation): a library part, which decision 1 excludes. It can be a
  follow-up ticket if K-15's cost matters.
- **A setting to turn retiring off:** the owner wants none (decision 16).
- **Showing Activity when a quit waits:** the owner removed it from Cycle 01
  after its review round 3 (decision 12).
- **Library tickets for the passphrase cache and stale rows before D6:** the
  owner chose known limits and follow-ups (decisions 25 and 27).

## Risks

- Cancel latency under a blocked connect makes K-1 visible to the person.
- U-15 may show a dead Check now's row cannot be resumed from the desktop, so
  L-4 becomes a known limit with the CLI's recovery.
- U-16 may find no portable advisory lock, so R-31's instance lock needs
  another form.
- U-14 may find no reliable connect stall in CI (decision 22 then falls back
  to a departure).
- Wave 04 Cycle 01 merging first moves the files D6 edits in the view
  (W-3); Task 0 re-grounds against it, and a rename there renames D6's calls,
  not its behaviour.

## Re-grounding

**2026-11-09, at `9512080d`** (origin/main `f4442618`, which integrates
`01M6KHVD6D0DB8KYT6VGQP10Y0`):

- **Dispositions.** `PollDisposition` has `Closed`, `Promoted` and
  `Unpublished` beside the earlier fifteen. Fact 5 and A4 list all three, and
  E1 offers Retire on the first two. No disposition was removed or renamed.
- **Poll entry points.** `poll_publication_remote`, `poll_probed`,
  `automatic_poll_eligibility`, `eligibility`, `next_eligible`,
  `cancel_remote_operation`, `remote_snapshot`, `polling_status` and
  `RemotePollError` are unchanged apart from their lines (Facts 3, 4, 7, 10,
  11). `PollOnce` still builds `Explicit`. `PollOnceRetiring` is new and is
  also `Explicit` (Fact 1).
- **Recovery suspension.** An explicit history repair now retires the marker,
  and the table defaults are rebuilt; already-suspended rows are unchanged,
  and `resume_remote_polling_after_recovery` still has no caller (Fact 8).
  Decision 10 stands.
- **Retirement.** `ContextRetire`, its DTO and states, and the ordinary
  recovery are Facts 24–26. The desktop's only change from the ticket is
  the abandon preview's retirement details (Fact 18).
- **Desktop.** The ticket also touched `D/view/harness.rs`,
  `D/test_support/`, `D/view/keyboard_tests/settings.rs` and two test files.
  None of these changes a Fact.
- **API Audit Refresh.** Three retirement rows were added. The poll rows
  stand.

**Still owed.** If Wave 04 Cycle 01 has merged by Task 0, check W-1 to W-7
against its code: `needs_attention`'s inputs, the launch refresh's new home,
where `finishing` is drawn, and the panel and region names in tests.
Otherwise check them at the merge that brings it in. Anything else that moves
on main after `9512080d` is checked at Task 0.

## Review Record

| Round | Reviewers, at | Findings | Resolution |
| --- | --- | --- | --- |
| 1 | Source and feasibility; contract, consistency and completeness (two, in parallel), at `d48bc4aa` | Source: 4 Blocking, 13 Important, 13 Minor. Contract: 4 Blocking, 20 Important, 19 Minor, 12 owner questions | Revision 4. Facts 4, 6, 7, 10, 13–17, 23, 25, 26, 28 corrected; Facts 29–31 added; A4 rekeyed on live answers; B1–B6, C1–C3, Part D and Part E rewritten; R-23 to R-30; testing per AGENTS.md; real-service tests; native sections numbered; amendments completed. Six questions became owner decisions 18–23. The rest are rulings or settled. |

| 2 | Scoped re-review; adversarial (two, in parallel), at `4d22c393` | Scoped: 1 Blocking, 8 Important, 20 Minor. Adversarial: 4 Blocking, 10 Important, 10 Minor | Revision 5. A polling hot loop on `recovery_required` (R-35); stale-row takeover rewritten as required outcomes L-1 to L-6 with an in-flight record and an instance lock (R-24, R-31); Resume after a relaunch from the in-flight record; "kept draft" defined and edits skipped (R-18); retirement codes corrected (E3); one loop per repository (R-32); nothing new while quitting (R-33); preemption at every user-lane entry point (R-3, R-34); dedupe in the count (R-23). Four questions became owner decisions 24–27. |

| 3 | Scoped re-review; adversarial (two, in parallel), at `c35719c2` | Scoped: 0 Blocking, 6 Important, 14 Minor. Adversarial: 3 Blocking, 9 Important, 10 Minor | Revision 6. Blocking: a cancel lost at a no-op `Ok` during a same-ID restart (P-1, P-2); a stale row that could only be taken over at launch (L-1, L-2 at runtime); an unknown draft listing read as "no draft" (RT-5, RT-6). Preemption, stale rows and the loop's rules each drew blockers for the third time, so each is now required outcomes with a test-first task and a design checkpoint. Also: the quit freeze from `request_quit` (R-33); `Accepted` answers and transport codes stop the loop (R-37); entry lifecycle (R-31); Resume clears declined keys; Unlock… picks a reachable repository; a running retirement finishes on a graceful quit (proposed decision 28). |

Revision 6's edits had no independent review: the planning procedure stops
rewriting a mechanism after a third round and moves its proof to a
test-first task with a design checkpoint.
