---
title: "Wave 03 D6: Desktop Background Polling And Shutdown"
date: 2026-11-07
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6V02C9TW4DZDGBW8Z9JH4VH"
---

# Wave 03 D6: Desktop Background Polling And Shutdown

Ticket `01M4CC0VN3VPVVQ3RNWJKXWEVX`. Design:
[2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md](../plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-design.md).
Plan:
[2026-11-07-wave-03-desktop-06-polling-and-shutdown-implementation.md](../plans/2026-11-07-wave-03-desktop-06-polling-and-shutdown-implementation.md).

Status: **draft, 2026-11-07.** Not reviewed and not approved. Planned at
`3364370c` (origin/main `1e5616af` merged into the ticket branch). Approval
of these documents does not authorize implementation: that waits for ticket
`01M6KHVD6D0DB8KYT6VGQP10Y0` to land and for the re-grounding below.

## Purpose And Authority

Keep discovery current inside the desktop process. After D6, while Manyhands
is open it checks each repository's shared copy on the repository's policy,
brings in other people's changes when that is safe, never touches anything
still being edited, never shares anything, and stops cleanly when the person
quits.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 03](../Waves/wave-03-dogfooding.md): section D6 (line 1100), entry
   gate items 3, 5, 6 and 7 (113–215), track rules (326–371), and C5's
   amendment to D6.
3. RFCs: [application runtime and polling](../RFC/application-runtime-and-polling.md)
   (the whole document governs D6),
   [desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md),
   [authentication and credential handling](../RFC/authentication-and-credential-handling.md),
   [Git workflow and conflict recovery](../RFC/git-workflow-and-conflict-recovery.md)
   (334–340, safe points), [test and compatibility strategy](../RFC/test-and-compatibility-strategy.md)
   (224–229).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-COLLAB-003/004`,
   `MH-INDEX-001`–`003`, `MH-CRED-001`, `MH-NFR-001/002/003/004/005/006/007/008`.
5. The design handoff [`docs/design/05-sheets-and-dialogs.md`](../design/05-sheets-and-dialogs.md)
   ("Checking for updates", 61–68; "Unlock key", 121–131; "Needs attention"),
   `06-language-and-states.md` (lines 15–26), `03-panels.md` (28).
6. Predecessors: [C5](wave-03-cli-05-completion-and-polling.md) (decisions 1,
   5, 11, 15, 24, 28), [D5](wave-03-desktop-05-recovery-and-completion.md),
   [D4](wave-03-desktop-04-discussion-and-sync.md) (decisions 12, 15),
   [D2](wave-03-desktop-02-recoverable-drafts.md), D1, and Wave 02 Cycle 08.

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`3364370c`) |
| --- | --- |
| Wave 02 Cycle 08 on main | Closed and merged before the Wave 03 replan baseline. |
| D1–D5 merged (track rule 1) | All merged; D5 closed at `1bce92d9`. |
| C5 merged (track rule 2, C5 decision 1) | Merged as `2d912d10` (2026-11-04). The four polling bindings are on main. |
| Library ticket `01M6KHVD6D0DB8KYT6VGQP10Y0` | **Open, in flight.** Adds a `closed` poll disposition and a deliberate retire verb. D6 is drafted against it and re-grounded when it lands (Re-grounding below). |
| API audit refreshed (entry-gate item 3) | Drafted from current source in the design's Baseline Facts; Task 0 re-checks the rows after re-grounding (owner-accepted departure, as C4, C5 and D5 recorded theirs, proposed in decision 9). |
| Runtime readiness (entry-gate item 5) | Partly present: the transport initializes before threads (`src/runtime.rs:25`, `src/main.rs:11`) and raw stalls are characterized (Wave 02 Cycle 03). Cancellation during a stall is **not** characterized. Task 1 closes that. |
| Required checks at baseline (entry-gate item 7) | Not run during planning. Task 0 runs them. |
| Tester assignment (entry-gate item 6) | Linux (Wayland): the product owner. Windows and macOS: G1. |

**Dependency boundary.** D6 changes **no** file in `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs` or `src/lib.rs` (decision 1: no library part, no track-rule-5
exception). Its one change under `tests/` is a single case in
`tests/remote_polling.rs` (Task 1). Any other library gap becomes its own
ticket.

## Scope

1. **One scheduler for the process (decision 2).** An application-owned
   scheduler, held by `Services`, runs every automatic poll on the existing
   mutation worker's background lane. One worker and one credential session
   serve every window.
2. **Launch poll.** Each enabled, unpaused repository with a publication
   remote is polled once after launch, unless its stored deadline (an
   outstanding failure backoff) says otherwise.
3. **Existing policy.** Five-minute default, 60 s–3600 s interval, persisted
   pause, automatic failure backoff (60 s doubling to 900 s). D6 reads and
   follows it; the library owns the arithmetic.
4. **Manual priority.** A user job submitted while a poll runs asks the poll
   to stop at its next safe point and runs next (decision 3). A poll that
   meets a held lease or yields is restarted on a bounded schedule, never in
   a loop (C5 decision 24).
5. **Poll now and policy controls.** A "Checking for updates" page in
   Settings: per repository, status line, interval, Check now (or Retry now)
   and an on/off switch. Check now is C5's `poll once`; the interval and the
   switch are `poll configure`, `poll pause` and `poll resume` (decision 5).
6. **Suspend and clock recovery.** Monotonic waits inside the process,
   persisted UTC deadlines across restarts; one poll per repository after a
   suspend or a clock jump, no catch-up loop; a future deadline is clamped by
   the library.
7. **Worker and UI credential handoff.** An automatic poll never prompts. A
   locked key or an unapproved host is recorded as a needs-attention state
   for that key or host (distinct from the person's pause), shown with
   Unlock… or Approve…, and polling for the affected repositories resumes
   when the person acts (decision 6).
8. **Truthful status.** Checking, Checked N ago, Can't reach, Paused, Needs
   unlocking, Needs host approval, Waiting for a conflict, Stopping. Never a
   state the process cannot know.
9. **Brought in changes.** A fast-forward or materialization refreshes the
   lists and any open tab. An item with unsaved edits or a kept draft keeps
   them (the existing changed-item path, proven by D6 tests).
10. **Shutdown.** Quit stops admitting work, cancels the running poll at its
    next safe point, flushes drafts (D2), shows "still stopping" at ten
    seconds, and waits. It does not clear credentials in use or reclaim a
    live operation because ten seconds passed (decision 7).
11. **Runtime readiness evidence (Task 1).** One SSH-harness case measures a
    cancel during a stalled handshake, and a research note records the
    stall and teardown behaviour that the stopping status is built on.

## Explicit Exclusions And Downstream Obligations

- A CLI daemon, process discovery or heartbeat tables, a shared service,
  coordination of several resident pollers, and a total transport deadline.
  External forced termination is tested as interruption, not rollback.
- Any background push, checkpoint, merge, cleanup or publication.
- Automatic retirement of a remotely deleted context (the library ticket's
  verb is explicit; D6 only shows what polls report).
- The Activity panel, the Needs attention popover and the footer the handoff
  draws: D1's operation area carries entry points (D4 decision 2). A global
  "pause all" switch (decision 8).
- Clearing the recovery suspension automatically. D6 offers a deliberate
  "Resume checking" only (decision 10).
- CLI-against-desktop scenarios: G1 (track rule 4). Windows and macOS native
  evidence: G1.
- Known limits K-1 onward (design Known Limits).

## Required Contract

1. **Read-only toward the shared copy.** An automatic poll fetches,
   fast-forwards clean refs and materializes new contexts, and nothing else.
   No push, checkpoint, merge, cleanup or publication starts from it
   (RFC, Required evidence).
2. **Manual priority.** A user job is never kept waiting on a poll longer
   than the poll's next safe point, which for a blocked network call is the
   transport's own timeout.
3. **No background prompt.** A poll never opens a passphrase or host dialog.
   Needs-attention states carry an action the person chooses.
4. **Pause is not an unlock block.** Unlocking never clears the person's
   pause, and resuming never clears an unlock block. Each is shown.
5. **Unsaved work is never overwritten.** A bring-in with an open unsaved
   edit or a kept draft for that item leaves the text and shows the
   existing changed-item notice.
6. **Truthful status.** Status text comes from what the library stored or
   the worker did; it never claims a worker the process lacks (a CLI poll
   by another process is not shown as the desktop's).
7. **One worker, no leftovers.** One scheduler per process whatever the
   number of windows; after a graceful exit no worker thread, child process
   or reservation remains that the exit did not release.
8. **Honest shutdown.** "Still stopping" at ten seconds; the UI stays
   responsive; nothing is called rolled back that was not.
9. **Keyboard.** Every new control is reachable and operable by keyboard
   with visible focus.
10. **Privacy.** No item title, body, comment or key material enters an
    operation entry, status line, log or error message.

## Acceptance And Exit Evidence

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Real SSH background update and materialization journey | Native §2 against a real SSH remote (owner); library SSH tests `poll_materialize_*`; desktop tests over canned attempts (decision 9) | Pending |
| A2 | Exceptional-context preservation (dirty, diverged, conflicted, deleted, blocked targets) | Desktop tests per disposition: state kept, marked, no mutation call; library tests exist | Pending |
| A3 | Pause versus unlock state | Desktop tests: both shown; unlock keeps pause; resume keeps the block | Pending |
| A4 | Concurrent explicit operation from a second process | Desktop test: `busy`/`interrupted` answers restart on schedule; library SSH yield tests; native §3 runs `manyhands-cli poll once` against the open app | Pending |
| A5 | Stalled transport with responsive stopping status | Task 1 SSH case (cancel during a stalled handshake); desktop test with a held job and an injected clock shows "still stopping" at 10 s and stays interactive | Pending |
| A6 | Configuration changes | Desktop tests: interval, pause, resume, remote and key change, registration removal, re-added remote not resuming a pause | Pending |
| A7 | Shutdown and restart | Desktop tests: quit mid-poll, quit mid-backoff, relaunch resumes the stored deadline; worker thread joined | Pending |
| A8 | No background push, checkpoint, merge or cleanup | Test double records every service call an automatic poll makes: only eligibility, poll and status reads | Pending |
| A9 | No remaining worker after graceful exit | Test joins the worker thread; native §6 checks the process list after quit | Pending |
| A10 | Manual priority | Desktop test: a user job submitted during a poll cancels it and runs first; the poll is rescheduled with no failure backoff | Pending |
| A11 | Launch poll respects backoff | Model test with an injected clock | Pending |
| A12 | Suspend and clock jump: one poll, no catch-up | Model test | Pending |
| A13 | No background prompt | Desktop test: a locked key yields needs-attention and no `Prompt` event | Pending |
| A14 | Unsaved edits survive a bring-in | Desktop test with an open dirty edit session and a kept draft | Pending |
| A15 | Keyboard and focus (contract 9) | Keyboard tests; native §7 | Pending |
| A16 | Privacy (contract 10) | Sentinel tests | Pending |
| A17 | One scheduler whatever the windows | Test: the scheduler is owned by `Services`; a second `Services` clone starts none | Pending |

Native evidence: Linux (Wayland), release build, against a real SSH remote,
run by the product owner from the checklist written in the final task. The
agent records only what the owner reports.

## Product-Owner Decisions (2026-11-07)

Answered by the owner during planning:

1. **No library part.** The scheduler calls the public
   `RepositoryService::poll_publication_remote` with `Automatic` and stops
   it with the public `cancel_remote_operation`. The mutation boundary has
   no automatic binding and D6 adds none. No track-rule-5 exception is
   requested.
2. **Same worker, background lane.** Automatic polls are background jobs on
   the one mutation worker, which owns the process's only credential session.

Proposed by the planner, confirmed with the documents' approval:

3. **A user job cancels a running poll.** The worker is one thread, so a
   user job queued behind a poll cannot start the library's own yield; the
   scheduler cancels the poll's operation instead, and reschedules it with no
   failure backoff (design R-3).
4. **Restart a `busy` or interrupted automatic poll on a short bounded
   schedule** (5 s, 15 s, 60 s, then the normal interval), resending the
   same operation with `restart` (C5 decision 24's D6 note).
5. **The page uses C5's bindings.** Check now is `poll once`; the interval
   is `poll configure`; the switch is `poll pause`/`poll resume`.
6. **A needs-attention state per key and host.** Set by an automatic poll's
   unlock or host-approval answer; cleared by a supplied passphrase, an
   approved host or a key or remote change; never by pause or resume.
7. **Quit keeps D1's Quit now.** "Still stopping" appears at ten seconds
   beside the existing Quit now button, which the person chooses and which is
   interruption, not rollback.
8. **No global "pause all" switch.** The library's policy is per repository
   and the desktop adds no policy of its own. The handoff's global switch is
   recorded as a Known Limit.
9. **Remote evidence: canned attempts, library tests and native** (D5
   decision 9). Desktop tests drive the scheduler from the desktop's own
   attempt type; the library's SSH targets prove remote behaviour; the
   native checklist runs a real SSH journey. Entry-gate item 3 departs from
   its timing as C4, C5 and D5 did.
10. **Resume checking is deliberate.** A recovery-suspended repository shows
    "Checking is held until recovery" and a Resume checking action that calls
    `resume_remote_polling_after_recovery`. Nothing clears it automatically.

## Open Questions

None for the owner beyond confirming decisions 3–10 at approval. U-1 to U-8
(design) are settled in Task 0 by reading the source; an answer that
differs stops for the owner.

## Re-grounding Obligation

`01M6KHVD6D0DB8KYT6VGQP10Y0` adds a poll disposition for a remotely deleted
context whose item is closed on primary, and a retire verb. When it merges,
before approval: merge main, compare the design's Baseline Fact 5,
add the new disposition to the desktop's mapping and to the tests that
enumerate dispositions, and record the check in the design's Re-grounding.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | No boundary path runs an Automatic poll; the probe is crate-private | Owner decision | Decision 1. | `bind/poll.rs` builds `Explicit`; `poll.rs:1243`, `lifecycle.rs:82`. |
| Planning research | One worker thread cannot start a user job while a poll runs | Owner decision | Decision 2 with proposal 3. | `worker.rs` `take`; `poll.rs:1269`. |
| Planning research | Recovery suspension has one setter, the history-lost marker, not unlock | Settled | Needs-attention is desktop-held; decision 10 covers suspension. | `state.rs:39-55`, `6441`. |
| Planning research | The read boundary has no `latest_attempt_at` or dispositions | Settled | Status uses `remote_snapshot` through the backend (design U-3). | `read/status.rs:717`; `state.rs:2125`. |
| Planning research | Handoff draws a global pause and an on/off switch; the library has per-repository pause and no verb for `enabled` | Owner decision | Decision 5 (switch is pause) and 8. | `bind/poll.rs`; handoff 66. |
| Planning research | Raw stalls are characterized; cancellation during a stall is not | Settled | Task 1. | `tests/ssh_fixture.rs:660-733`; no cancel-under-stall test. |
| Drafting | Not yet reviewed | — | Review rounds follow. | — |
