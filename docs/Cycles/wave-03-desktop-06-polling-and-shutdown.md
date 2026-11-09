---
title: "Wave 03 D6: Desktop Background Polling And Shutdown"
date: 2026-11-07
status: "approved"
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

Status: **approved, 2026-11-09.** The owner approved the documents at
`04e1623f` (revision 6). The owner wrote: "The docs are approved. Named test edits approved (if during execution it's discovered that more tests would need to be edited, that's approved as well). Subagent driven development. Implementation is approved as well. File the two tickets."

- The approval covers decisions 3–15 and 28 at their written proposals, the
  rulings R-1 to R-38, and named test edits E-1 to E-6. Any further edit to an
  existing test that execution finds necessary is pre-approved; each is
  recorded in the ledger as E-7 onward, with its reason.
- Implementation is authorized, subagent-driven and serial. Pushes beyond the
  ticket branch, a PR, the merge, closure, native CI and cleanup stay
  separately gated.
- The approval commit applied the design's Document Amendments 1–5 and 7–10.
  Amendment 6 waits for D6's merge.
- The follow-up library tickets are filed: `01M70CF14M3YC6SR8T9JJ6XQBG`
  (`vh-ype2f`, a per-key passphrase cache) and `01M70CF1VBHJAX6MK39FY5W2RY`
  (`vh-xmhsj`, ending a dead poll's row whatever the eligibility).

Planned at `3364370c` (origin/main `1e5616af` merged into the ticket branch).

- 2026-11-08: origin/main `8da9afd4` was merged as `1869cf14`, and
  coordination with Wave 04 Cycle 01 was added.
- 2026-11-09: ticket `01M6KHVD6D0DB8KYT6VGQP10Y0` landed (main `f4442618`,
  merged as `9512080d`). The documents were re-grounded against it (design
  Re-grounding). They took on the explicit Retire action it assigns to D6
  (Scope 12, design Part E), and they now follow Wave 04 Cycle 01's approved
  decisions 3 and 7. The owner then decided that polls retire completed
  contexts, with the outcome of the CLI's `poll once --retire-completed`,
  always on (decision 16), and that every operation is per item, a
  repository operation being a loop over its items (decision 17).
  Retirement is therefore one request per reported item.
- 2026-11-09: review round 1 (two reviewers at `d48bc4aa`) found 8 Blocking,
  33 Important and 32 Minor issues across both lenses. Revision 4 applies
  them; six owner questions became decisions 18–23.
- 2026-11-09: review round 2 (a scoped re-review and an adversarial review at
  `4d22c393`) found 5 Blocking, 18 Important and 30 Minor issues. Revision 5
  applies them; four owner questions became decisions 24–27.
- 2026-11-09: review round 3 (a scoped re-review and an adversarial review at
  `c35719c2`) found 3 Blocking, 15 Important and 24 Minor issues. Three
  mechanisms drew blockers for the third time: preemption, stale rows and the
  retirement loop's rules. As the planning procedure asks, revision 6 states
  each as required outcomes (design P-1 to P-7, L-1 to L-6, RT-1 to RT-11),
  with a test-first task and a design checkpoint, and no fourth round runs.
  **Revision 6's edits had no independent review.**

Approval of these documents does not authorize implementation, which needs
the owner's separate authorization.

## Purpose And Authority

Keep discovery current inside the desktop process. After D6, while Manyhands
is open it checks each repository's shared copy on the repository's policy,
brings in other people's changes when that is safe, retires its own copies
of work finished elsewhere, never touches anything still being edited, never
shares anything, and stops cleanly when the person quits.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 03](../Waves/wave-03-dogfooding.md): section D6 (line 1111), entry
   gate items 3, 5, 6 and 7 (from line 113), track rules (335–415), and C5's
   amendment to D6.
3. RFCs: [application runtime and polling](../RFC/application-runtime-and-polling.md)
   (the whole document governs D6),
   [desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md),
   [authentication and credential handling](../RFC/authentication-and-credential-handling.md),
   [Git workflow and conflict recovery](../RFC/git-workflow-and-conflict-recovery.md)
   (safe points, around 414; polling and cleanup, 293–295),
   [test and compatibility strategy](../RFC/test-and-compatibility-strategy.md)
   (224–229), [CLI contract](../RFC/cli-contract.md) (completed-context
   retirement).
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-COLLAB-003/004/006`,
   `MH-INDEX-001`–`003`, `MH-CRED-001`, `MH-NFR-001/002/003/004/005/006/007/008`.
5. The design handoff [`docs/design/05-sheets-and-dialogs.md`](../design/05-sheets-and-dialogs.md)
   ("Checking for updates", 61–68; "Unlock key", 121–131; "Needs attention"),
   `06-language-and-states.md` (lines 15–26), `03-panels.md` (28).
6. Predecessors: [C5](wave-03-cli-05-completion-and-polling.md) (decisions 1,
   5, 11, 15, 24, 28), [D5](wave-03-desktop-05-recovery-and-completion.md),
   [D4](wave-03-desktop-04-discussion-and-sync.md) (decisions 12, 15),
   [D2](wave-03-desktop-02-recoverable-drafts.md), D1, and Wave 02 Cycle 08.
7. The library follow-up
   [completed-context retirement](wave-03-completed-context-retirement.md)
   (ticket `01M6KHVD6D0DB8KYT6VGQP10Y0`; owner decisions D4, D6, D7, D13 and
   D15), which assigns D6 the desktop Retire action.

## Entry Evidence And Dependency Boundary

| Gate | State (planned at `3364370c`, re-grounded at `9512080d`) |
| --- | --- |
| Wave 02 Cycle 08 on main | Closed and merged before the Wave 03 replan baseline. |
| D1–D5 merged (track rule 1) | All merged; D5 closed at `1bce92d9`. |
| C5 merged (track rule 2, C5 decision 1) | Merged as `2d912d10` (2026-11-04). The four polling bindings are on main. |
| Library ticket `01M6KHVD6D0DB8KYT6VGQP10Y0` | **Merged 2026-11-09** (`f4442618`, brought in by `9512080d`). Adds the `closed` and `promoted` dispositions, `context retire` (`Mutation::ContextRetire`) and the deliberate `poll once --retire-completed` (`Mutation::PollOnceRetiring`). Assigns the explicit desktop Retire action to D6 (Wave D6 section, amended 2026-11-06). Re-grounded (Re-grounding below). |
| API audit refreshed (entry-gate item 3) | Library rows re-grounded at `9512080d` and checked again by review round 1, in the design's Baseline Facts and API Audit Refresh; Task 0 re-checks what moves after that. The timing departs from item 3 as C4, C5 and D5 did (proposed decision 9; Document Amendment 9). |
| Runtime readiness (entry-gate item 5) | Partly present: the transport initializes before threads (`src/runtime.rs:25`, `src/main.rs:11`) and raw stalls are characterized (Wave 02 Cycle 03). Cancellation during a stall is **not** characterized. Task 1 measures it during a handshake stall and a connect stall; DNS and teardown stay unmeasured (decisions 22 and 24, K-20). |
| Required checks at baseline (entry-gate item 7) | Not run during planning. Task 0 runs the static gates only; the fast tier runs once, at review-ready, under AGENTS.md's Testing Strategy (2026-11-06; Document Amendment 9). |
| Tester assignment (entry-gate item 6) | Linux (Wayland): the product owner. Windows and macOS: G1. |
| Wave 04 Cycle 01 (`01M6V8RK8B0SBCH0YZXFVSR9E3`) | **Approved 2026-11-08 and in implementation** (Tasks 1 to 4b done by 2026-11-09). Not a dependency; expected to merge first. Coordination below. |

**Dependency boundary.** D6 changes **no** file in `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs` or `src/lib.rs` (decision 1: no library part, no track-rule-5
exception). Its one change under `tests/` is two cases in
`tests/remote_polling.rs` (Task 1, decision 22). Any other library gap
becomes its own ticket.

## Wave 04 Cycle 01 Coordination

[Wave 04](../Waves/wave-04-workspace-design.md) Cycle 01 (workspace shell,
ticket `01M6V8RK8B0SBCH0YZXFVSR9E3`) moves the desktop into docked panels.
It was approved on 2026-11-08 (revision 5) and is in implementation. What it
does that meets D6, from its approved Cycle document and its note on this
ticket (comment `01M6Z8A8AYDGTK4BWKZR3931ST`):

- It wraps today's operation area, as it is, in an **Activity panel** in the
  bottom dock, and does not edit `D/view/operations.rs`, `D/view/operations/`
  or `D/model/operations.rs`.
- It adds a **footer** whose **Needs attention** item shows
  `needs_attention(&Operations, &DraftRegistry) -> usize`, a new read-only
  function in `D/model/attention.rs` (its decisions 3 and 14). It counts
  requests that finished partial, blocked or with an error and are not
  dismissed; draft flush failures; drafts unavailable; pending rows; and
  failed pending reads. D6's attention states join it when the second of the
  two Cycles merges.
- **A quit that has to wait changes no layout** (its decision 7, revised by
  the owner after its review round 3). "Finishing …" and Quit now stay in
  Activity and are visible when Activity is. With Activity hidden, the quit
  shows nothing until Activity is opened (its K-18). Quit now and Retry take
  focus only when Activity is displayed (`wait_to_quit`,
  `not_saved_chosen`).
- **Settings stays a page in the item area**, opened by `secondary-,`, the
  sidebar entry and the new footer button, until Wave 04 Cycle 04 moves it
  into a sheet.
- Its proposed **K-19**: at the default 1280 px window, Activity is about
  550 px wide. Three unwrapped operation-area rows (the draft-flush Retry,
  the pending repository Retry, and pending controls such as Abandon) can sit
  outside the visible part. It names Cycle 03's Activity rebuild or D6.
- The owner's **later work** (Wave 04 Deferred Work): one blocking quit
  dialog that replaces Quit now and Quit anyway, a watchdog for pending
  operations, and a footer status line.

Cycle 01 is in implementation while D6 is unapproved, so D6 plans to merge
second and to do the adapting. If D6 reached main first, Cycle 01 stops at
that merge for a design checkpoint with the owner (its note). If Cycle 01 has
not merged when D6 reaches a step below, D6 records that and leaves the step
to Cycle 01. The obligations:

1. **Attention in the count (decision 11).** D6's attention counts toward
   Needs attention once per locked or missing key, per unapproved host, and
   per misconfigured or held repository. Paused, Checking is off, Can't reach,
   Checking soon, Waiting for a conflict and a Retire offer do not. A refused,
   preserved or pending retirement is a finished request or a pending row,
   which Cycle 01 already counts. A request whose answer became attention is
   dismissed, a request and its pending row count once, and a running
   operation is never listed as pending, so nothing counts twice (design
   R-23). The polling model exposes its attention through one
   read-only query, which the second Cycle to merge passes to
   `needs_attention` as a third input; it also extends that function's test
   (design W-1; Task 8).
2. **Still stopping lives in Activity (decision 12).** "Still stopping" is a
   change to the existing `finishing_text` only, so it is in the quit's entry
   in Activity. With Activity hidden it shows nothing until Activity is
   opened, as for Cycle 01's own feedback. D6 shows no panel and adds no
   second place. With Cycle 01 on main, D6's still-stopping test also runs
   with Activity hidden (design W-2; Task 10).
3. **D6's rows fit Activity (decision 13).** Every row D6 adds to the
   operation area wraps so that its actions stay inside Activity at the
   default window; a test clicks each at its position. The three existing
   rows Cycle 01's proposed K-19 names stay with Wave 04 Cycle 03 (design
   W-6, Document Amendment 8).
4. **A small footprint in shared files.** The scheduler is held by
   `Services`, not by `Workspace`. D6's only edit to the repository list is one
   call beside the launch refresh in `repositories_loaded`, wherever Cycle 01
   has moved it, plus one window-activation subscription in
   `open_main_window`. Expected textual conflicts: `D/view/workspace/repositories.rs`;
   `D/view/workspace/lifecycle.rs` (`wait_to_quit`); the `Workspace` struct and
   constructor; `D/view/harness.rs` (window size, `Shell::use_list`); and the
   keyboard tests (`Region::Navigation` becomes `Region::Tickets` or
   `Region::Documents`, and the F6 order changes) (design W-3).
5. **The Settings page moves later.** "Checking for updates" is a fifth page
   of today's Settings, in a view module of its own, so Wave 04 Cycle 04 can
   fold it into the sheet. D6's keyboard walk opens it with `secondary-,`,
   which every layout keeps (design W-4; Task 13).
6. **Status surfaces for Wave 04 Cycle 03.** The footer's "Checked for
   updates …", the Repositories panel's "Checking…" and "Paused", Check all
   for updates now and the Needs attention popover are Cycle 03's. D6 exposes
   each repository's status text and last attempt from its model for them,
   and adds nothing to the sidebar or the footer (design W-5, K-10).
7. **Quit now stays until the later quit dialog.** D6 keeps Quit now
   (decision 7). The owner's blocking quit dialog replaces it later and takes
   over the still-stopping report (design W-7).

The follow-up tickets `01M6RAS63EWVHWWVBSZ836RXG2` and
`01M6H7VXJJHRYHTW9S8TJ88921` plan to split `D/model/operations.rs` and
`D/view/operations.rs`, which D6 edits (Tasks 6 and 8) and Cycle 01 does not.
Whichever of those and D6 merges second adapts, per `AGENTS.md`.

## Scope

1. **One scheduler for the process (decision 2).** An application-owned
   scheduler, held by `Services`, runs every automatic poll on the existing
   mutation worker's background lane. One worker and one credential session
   serve every window.
2. **Launch poll.** Each enabled, unpaused repository with an SSH
   publication remote is polled after launch when its stored deadline is due,
   whether that deadline came from the interval or a failure backoff
   (decision 23). A poll row this app left active is taken over first.
3. **Existing policy.** Five-minute default, 60 s–3600 s interval, persisted
   pause, automatic failure backoff (60 s doubling to 900 s). D6 reads and
   follows it; the library owns the arithmetic.
4. **Manual priority.** A user job submitted while a poll runs asks the poll
   to stop at its next safe point and runs next (decision 3). A background
   retirement is not preempted; a user job waits for the one that is running
   (decision 20). A poll that meets a held lease or another operation's row
   is restarted on a bounded schedule, never in a loop (C5 decision 24).
5. **Poll now and policy controls.** A "Checking for updates" page in
   Settings: per repository, status line, interval, Check now (or Retry now)
   and an on/off switch. Check now is C5's `poll once`; the interval and the
   switch are `poll configure`, `poll pause` and `poll resume` (decision 5).
6. **Suspend and clock recovery.** Monotonic waits inside the process,
   persisted UTC deadlines across restarts; one poll per repository after a
   suspend or a clock jump, no catch-up loop; a future deadline is clamped by
   the library.
7. **Worker and UI credential handoff.** No background job prompts, a poll
   or a retirement, even in a fresh session (the desktop provider refuses on
   the background lane). A
   locked key or an unapproved host is recorded as a needs-attention state
   for that key or host (distinct from the person's pause), shown with
   Unlock… or Approve…, and polling for the affected repositories resumes
   when the person acts (decision 6).
8. **Truthful status.** Checking, Checked N ago, Can't reach, Paused, Needs
   unlocking, Choose a key, Needs host approval, Waiting for a conflict,
   Waiting for an unfinished operation, Waiting for another operation to
   finish, An interrupted check is holding this repository, Stopping. Never a
   state the process cannot know.
9. **Brought in changes.** A fast-forward or materialization refreshes the
   lists and any open tab. An item with unsaved edits or a kept draft keeps
   them (the existing changed-item path, proven by D6 tests).
10. **Shutdown.** Quit stops admitting work from the moment it is asked,
    cancels the running poll at its next safe point, lets a running
    retirement finish (proposed decision 28), flushes drafts (D2), shows
    "still stopping" at ten seconds, and waits. It does not clear credentials in use or reclaim a
    live operation because ten seconds passed (decision 7).
11. **Runtime readiness evidence (Task 1).** Two SSH-harness cases measure a
    cancel during a stalled handshake and during a stalled connect, and a
    research note records the stall and teardown behaviour that the stopping
    status is built on. DNS and teardown stay unmeasured (decisions 22 and
    24).
12. **Retiring completed contexts (Wave D6 section, amended 2026-11-06;
    decisions 16 and 17).**
    - After every completed poll, automatic or Check now, each reported
      `closed` ticket or `promoted` document is checked on its own, one at a
      time, in one loop per repository. Unless it is skipped, its tabs turn
      read-only and it is retired by its own `Mutation::ContextRetire`,
      under a new request ID, on the background lane, with no prompt.
    - An item being edited, or with any draft of it or its comments, is
      skipped; this holds back only that item (decisions 15 and 18).
    - A pending answer, or an unlock or host answer, stops that repository's
      loop until the next attempt (decision 21).
    - **Retire…** (the same mutation, a new request, one item, user lane)
      shows for every reported item the loop did not retire.
    - Refusals are shown with the library's code, and pending cleanup with
      the ordinary Resume, which resends the original request, and Abandon
      (decisions 14 to 21; design Part E).

## Explicit Exclusions And Downstream Obligations

- A CLI daemon, process discovery or heartbeat tables, a shared service,
  coordination of several resident pollers, and a total transport deadline.
  External forced termination is tested as interruption, not rollback.
- Any background push, checkpoint, merge or publication, and any background
  cleanup other than decision 16's per-item retirement.
- A setting to turn retiring off (decision 16), and the `PollOnceRetiring`
  pass, which cannot leave out one item (decision 14).
- Retiring a context that is only `remotely_deleted`, deleting a remote
  branch, and changing read precedence. Those are the remote-deletion gap
  `01M6K06B7H8V7347SRM3XGV7CB` and the latest-copy ticket
  `01M6K04YQM2JF6SBGPB1ZT6R8T`. Until a context is retired, its own worktree
  copy may still read as open (K-12).
- The Activity panel, the footer, the Needs attention popover and the
  Repositories panel's polling states the handoff draws: Wave 04 Cycles 01
  and 03 (Wave 04 Cycle 01 Coordination). D6's entry points stay in the
  operation area (D4 decision 2), which Cycle 01 shows as the Activity panel,
  and on the Settings page. Moving that page into the Settings sheet: Wave 04
  Cycle 04. A global "pause all" switch (decision 8).
- Rewrapping the three existing operation-area rows of Cycle 01's proposed
  K-19: Wave 04 Cycle 03 (decision 13). The blocking quit dialog, the
  pending-operation watchdog and the footer status line: Wave 04 later work.
- Clearing the recovery suspension automatically. D6 offers a deliberate
  "Resume checking" only (decision 10).
- CLI-against-desktop scenarios: G1 (track rule 4). D6 proves a concurrent
  explicit operation with a second library actor in the same process, a
  recorded departure from track rule 4's separate process (decision 26, A4).
  Windows and macOS native evidence: G1.
- Cancellation during a DNS or teardown stall (decisions 22 and 24, K-20).
- A per-key passphrase cache and a poll takeover that does not need
  eligibility: follow-up library tickets (decisions 25 and 27; design
  Document Amendment 10).
- Known limits K-1 onward (design Known Limits).

## Required Contract

1. **Read-only toward the shared copy.** An automatic poll fetches,
   fast-forwards clean refs and materializes new contexts, and nothing else.
   The only work that may follow it is the index refresh it asks for
   (design R-10), D2's draft flush, and decision 16's per-item retirement,
   which removes local copies the library proves completed. No push,
   checkpoint, merge or publication starts from either (RFC, Required
   evidence, as amended by decision 16).
2. **Manual priority.** A user job is never kept waiting on a poll longer
   than the poll's next safe point, which for a blocked network call is the
   transport's own timeout. It waits for at most one running background
   retirement, never a whole loop (decision 20).
3. **No background prompt.** No background job, poll or retirement, ever
   opens a passphrase or host dialog. Needs-attention states carry an action
   the person chooses.
4. **Pause is not an unlock block.** Unlocking never clears the person's
   pause, and resuming never clears an unlock block. Each is shown.
5. **Unsaved work is never overwritten.** A bring-in with an open unsaved
   edit or a kept draft for that item leaves the text and shows the
   existing changed-item notice. An item being retired takes no edits
   (decision 19).
6. **Truthful status.** Status text comes from what the library stored or
   the worker did; it never claims a worker the process lacks (a CLI poll
   by another process is not shown as the desktop's).
7. **One worker, no leftovers.** One scheduler per process whatever the
   number of windows; after every graceful exit the worker thread is joined,
   and no child process or reservation remains that the exit did not
   release.
8. **Honest shutdown.** "Still stopping" at ten seconds in the quit's entry,
   in Activity (decision 12); the UI stays responsive; nothing is called
   rolled back that was not.
9. **Keyboard.** Every new control is reachable and operable by keyboard
   with visible focus.
10. **Privacy.** No item title, body, comment or key material enters an
    operation entry, status line, log or error message.
11. **Retirement loses nothing.** An item is retired only by its own
    request: after a poll that reported it as `closed` or `promoted`, or by a
    person's Retire…. The loop never retires an item that is being edited or
    has any draft, of it or its comments (decisions 15 and 18); a Retire…
    never retires one with unkept text. An item's tabs take no edits while it is
    retired. Retirement never deletes a draft: the item's drafts and its
    comments' drafts stay listed and exportable on the Drafts page, and
    restorable where D2's restore policy allows (a closed ticket's drafts and
    comment drafts are export-only). Refusals and pending cleanup are shown
    exactly as the library answers, with the ordinary recovery.

## Acceptance And Exit Evidence

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Real SSH background update and materialization journey | Native §2 against a real SSH remote (owner); library SSH tests `poll_materialize_*`; desktop tests over canned attempts (decision 9) | Pending |
| A2 | Exceptional-context preservation (dirty, diverged, conflicted, deleted, blocked targets) | Desktop tests per disposition: state kept, marked, no mutation call (`Closed` and `Promoted` go to A19 instead); library tests exist | Pending |
| A3 | Pause versus unlock state | Desktop tests: both shown; unlock keeps pause; resume keeps the block | Pending |
| A4 | Concurrent explicit operation from a second process calling the library | Desktop test over a real service: a second library actor over the same registry, in the same process (decision 26), holds the remote reservation; the automatic poll answers `busy` and restarts on schedule. Library SSH yield tests. The CLI-against-desktop run is G1's (track rule 4) | Pending |
| A5 | Stalled transport with responsive stopping status | Task 1's two SSH cases (cancel during a stalled handshake and a stalled connect; DNS and teardown depart, decisions 22 and 24); desktop test with a held poll and an injected clock shows "still stopping" at 10 s and stays interactive; native §3 | Pending |
| A6 | Configuration changes | Desktop tests: interval, pause, resume, remote, key and host-trust change, registration add and removal, re-added remote not resuming a pause, queued retirements dropped | Pending |
| A7 | Shutdown and restart | Desktop tests: nothing new starts from the entry to `request_quit`, through the unkept ask and the flush; quit mid-poll ends when the poll answers; quit mid-backoff; a graceful quit lets a running retirement finish (decision 28), and Quit now leaves it pending; relaunch keeps the stored deadline (decision 23); native §6 | Pending |
| A8 | No background push, checkpoint, merge or publication; background cleanup only as decision 16's per-item retirement | A recording double lists B1's five calls for an automatic poll and nothing else. A `ContextRetire` is submitted only for an item the attempt reported as `closed` or `promoted`, outside the refused set, with no unkept tab and no kept draft; `PollOnceRetiring` never | Pending |
| A9 | No remaining worker after graceful exit | Test joins the worker thread on both graceful paths (`Idle`, and a `Finishing` job that ends); native §6 checks the process list after quit | Pending |
| A10 | Manual priority | Design P-1 to P-7 as one table (Task 4, test-first after a design checkpoint): a user job from any entry point cancels a running poll, even before its row exists and between a same-ID restart's attempts, and runs first; the poll is rescheduled with no failure backoff; a background retirement is not cancelled (decision 20) | Pending |
| A11 | Launch poll follows the stored deadline (decision 23) | Model test with an injected clock: an interval deadline and a backoff deadline both hold at launch | Pending |
| A12 | Suspend and clock jump: one poll, no catch-up | Model test | Pending |
| A13 | No background prompt | Desktop tests through the real `DesktopProvider`, with a fresh session and a locked key: no `Prompt` event, and attention, for an automatic poll and for a background retirement; Unlock… after a cancelled first prompt prompts again; two protected keys evict each other as K-23 says (decision 25) | Pending |
| A14 | Unsaved edits survive a bring-in | Desktop test over a real repository: the file changes on disk under an open dirty edit session, and under a kept draft | Pending |
| A15 | Keyboard and focus (contract 9) | Keyboard tests; native §7 | Pending |
| A16 | Privacy (contract 10) | Sentinel tests | Pending |
| A17 | One scheduler whatever the windows | Test: a poll submitted through a `Services` clone is seen by the original's scheduler | Pending |
| A18 | Wave 04 Cycle 01 coordination: D6's attention in the Needs attention count; "still stopping" in the quit's Activity entry with no layout change; D6's rows fit Activity | With Cycle 01 on main: `needs_attention`'s test extended with each kind of attention entry and each state that does not count (E-4); D6's still-stopping test run with Activity hidden (E-5). In any order: each D6 action clicked at its position at the default window. The three R-23 dedupe rules are in E-4. Otherwise recorded as Cycle 01's to wire | Pending |
| A19 | Retirement journey: after a poll, automatic or Check now, each reported `closed` ticket or `promoted` document is retired by its own request, one at a time; lists and an open tab then read primary; Retire… retires a left item by keyboard | Desktop tests over canned `context_retirement` answers (decision 9), and over a real local repository for the end state; library SSH case `retirement_cli_authenticated_live_pass`; native §5 on two clones | Pending |
| A20 | Exact refusals and pending cleanup | Desktop table test, record state then state then code: before-effect codes retried next attempt (unlock, key, host and configuration codes as attention); `refused` and `preserved` shown with the library's code and added to the refused set; `pending` stops the loop and leaves a pending row whose Resume resends the original request and whose Abandon shows the retirement details (library ticket D13), by keyboard | Pending |
| A21 | Open editors and drafts preserved | Desktop tests: of two reported items, the one with an unkept tab, and one with a kept draft, are skipped and the other retired; the item's tabs are read-only while it is retired; Retire… stops at an unkept tab and warns about kept drafts; over a real local repository, a kept item draft and a comment draft outlive retirement, listed and exportable | Pending |
| A22 | Replay and interruption (track rule 7) | Desktop tests: a Final retirement answer replayed under the same request removes nothing twice and adds no second entry; a quit during a held retirement leaves a pending row that Resume finishes after relaunch, from the in-flight record; a resent policy verb follows C5 decision 15 | Pending |
| A23 | Stale rows and unfinished operations | Design L-1 to L-6 as one table (Task 4, test-first, after a design checkpoint): a recorded poll is taken over at launch, or when its repository is next due; a not-eligible one says why and what finishes it, and is taken over once eligible (decision 27); a fenced or mismatched row is dropped; a dead Check now is resumed from its pending row; nothing is taken over on `Busy`; a pending local operation makes polls wait on the bounded schedule, never in a loop; native §6 | Pending |

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

Proposed by the planner; approved with the documents on 2026-11-09:

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
11. **What D6 adds to Needs attention** (2026-11-08, for Wave 04 Cycle 01).
    Each attention entry counts once, since each waits for the person to act:
    a locked key, a missing key, an unapproved host, a misconfigured
    repository, a repository held for recovery. A key locked for three
    repositories is one entry.
    Paused and Checking is off are the person's own choice; Can't reach and
    Checking soon retry by themselves; Waiting for a conflict points at a
    conflict that is already shown where it is resolved; a Retire… offer is
    an option, not a failure. None of those count. A retirement that
    finished refused or preserved with a refusal code, or a pending one, is
    counted by Cycle 01's own rules, once per operation; one answered with an
    attention or retry code is dismissed, since attention or the next attempt
    covers it (design R-23).
12. **Still stopping lives in Activity** (2026-11-09, after Wave 04 Cycle
    01's revised decision 7). With Activity hidden, a quit still waiting at
    ten seconds shows nothing until Activity is opened, as Cycle 01's own
    quit feedback does (its K-18). D6 shows no panel and adds no second
    place: the owner removed showing Activity from Cycle 01 after its review
    round 3. This narrows the Wave's "shutdown reports ongoing work at ten
    seconds" to "reports it in Activity" (K-13). The owner's later blocking
    quit dialog is where a report with Activity hidden belongs.
13. **D6's rows fit Activity.** Each row D6 adds to the operation area wraps
    so that its actions are inside Activity at the default window, and a
    test clicks each at its position. The three existing rows Cycle 01's
    proposed K-19 names stay with Wave 04 Cycle 03, which Wave 04 records
    (K-16; design Document Amendment 8).
14. **How decisions 16 to 21 are carried out.**
    - Every poll stays a recognizing poll. An automatic poll keeps its
      eligibility, backoff, priority and no prompt, and Check now stays
      `PollOnce`.
    - After the attempt, the scheduler loops over the reported `closed` and
      `promoted` items in the order reported. It checks each alone and
      submits one `ContextRetire` per item on the background lane, the next
      only when the previous has answered. Each is a new request.
    - The answer is read by its record state, then its state, then its
      code. A `pending` or `Accepted` answer leaves the pending row. A
      `refused` answer with an unlock, key, host or configuration code gives
      attention; with a transport, remote, index, external-change,
      cancelled or pre-claim `recovery_required` code it means nothing was
      removed, and the next attempt tries again. Attention, pending, transport
      and remote-unavailable answers stop the repository's loop.
    - An item refused or preserved for another reason is not tried again
      automatically this session (design R-21).
    - Retire… (the same mutation, a new request, user lane) shows for every
      reported item the loop did not retire.
    - The desktop does not use `PollOnceRetiring`, which handles every
      candidate as one pass and cannot leave out one item. The cost is one
      observation per retired item (K-15).
15. **An edit holds back only its own item; there is no "Retire anyway".**
    The loop skips an item that is being edited: an open edit session, dirty
    or clean, a begin in flight, comment composer text, or repair text. The
    item gets Retire…. Retire… on an item whose tab holds text no draft keeps
    opens a dialog that gives the count, with no title or text, and offers
    Show tab (the default) and Cancel. Otherwise its drafts are flushed
    before its request. Retirement never removes a draft.

Answered by the owner on 2026-11-09:

16. **Polls retire completed contexts, always on.** Polls retire closed
    tickets and promoted documents that still have a local context (a
    branch, with or without its worktree, as the library retires), with the
    outcome of the CLI's `poll once --retire-completed`. There is no setting.
    Retire… shows only for what a poll did not retire.
    - This lifts the "no background cleanup" rule for retirement only. It
      amends the PRD (a new 0.8 entry: the release scope, `MH-COLLAB-004`,
      `MH-COLLAB-006`, `MH-INDEX-002`, `MH-NFR-007`), the runtime, Git
      workflow and CLI RFCs, the Wave's track rule 5 amendment, capability
      rows, Integration Gate bullet, D6 amendment and exit evidence, the
      retirement Cycle's decisions 4, 7 and 8 and its contract 5, and the
      ticket's Exit Evidence (design Document Amendment 7).
    - Background push, checkpoint, merge and publication stay
      prohibited.
17. **Operations are per item.** The app shows a union view of a
    repository, but a repository is a collection of separate branches and
    worktrees, one per item. Operations are done per item, and a repository
    operation is a loop over its items, both local ones and those a fetch
    will materialize. The CLI's retirement converged only once it was
    relaxed to per item. Decision 15 is narrowed to the reported items: an
    unsaved tab holds back only its own item. The poll itself stays one
    fetch per repository with per-target dispositions; there is no per-item
    poll.

Answered by the owner after review round 1 (2026-11-09):

18. **Kept drafts hold an item back.** The loop also skips an item with a
    kept draft, of the item or of a comment on it, and offers Retire….
    Retiring a closed ticket makes its drafts export-only (D2's restore
    policy refuses a closed ticket), so the person decides. Retire… on such
    an item says so and asks.
19. **An item's tabs are read-only while it is retired.** From the loop's
    check, or a Retire…, until the answer, the item's tab and comment
    composer take no edits and show "Retiring…", as D5 decision 12 did for
    close and promote.
20. **A background retirement is not preempted.** A Save or Share submitted
    meanwhile waits for the one retirement that is running, up to one
    observation and its removal (K-17), the same limit D4 decision 12
    accepted behind a share. Only an automatic poll is cancelled by a user
    job.
21. **A pending item stops its repository's loop.** As the CLI's
    `--retire-completed` does: refusals go on, real cleanup or index debt
    stops later items until the next attempt. An unlock or host answer stops
    it too, since the next item would meet the same answer.
22. **Entry gate 5: a connect stall too; DNS departs.** Task 1 measures a
    cancel during a stalled connect as well as a stalled handshake. Cancel
    during a DNS stall stays unmeasured, recorded as a departure and K-20.
23. **The launch poll follows the stored deadline.** At launch a repository
    is polled when it is due, whether the deadline came from the interval
    or a failure backoff, so restarting the app adds no polls. The runtime
    RFC's launch wording is amended (design Document Amendment 2).

Answered by the owner after review round 2 (2026-11-09):

24. **Teardown departs too.** Task 1 keeps two cases. Cancel during a
    teardown or exec-acknowledgement stall joins DNS as a recorded departure
    from entry gate 5 (K-20).
25. **The single passphrase cache is a known limit.** With two protected
    keys, unlocking one can be undone by a job using the other (K-23). A
    two-key test pins today's behaviour, and a follow-up library ticket adds
    a per-key cache.
26. **The second actor runs in the same process.** A4's concurrent explicit
    operation is a second library actor in the same test process, a recorded
    departure from track rule 4's separate process. The cross-process run is
    G1's CLI-against-desktop scenario.
27. **A stale poll row in a not-eligible repository is a known limit.** A
    poll killed by Quit now or a crash, whose repository is paused,
    disabled, held or not due at relaunch, keeps its row until the
    repository is eligible again; then the desktop takes it over (design
    L-2). The desktop says why and what finishes it (K-22). A follow-up
    library ticket can add a takeover that does not need eligibility.

Proposed after review round 3; approved with the documents on 2026-11-09:

28. **A graceful quit lets a running retirement finish.** Quit does not
    cancel a running background `ContextRetire`. Cancelling it part-way
    would leave the item pending, and the pending local operation would hold
    the repository's polls at the next launch. The quit waits for it, as for
    any finishing job, with "still stopping" at ten seconds and Quit now
    available. Cost: a quit can wait up to one observation and its removal
    (design R-38).

## Open Questions

None for the owner: decisions 3–15 and 28 were approved on 2026-11-09, and
16 to 27 are the owner's. U-2, U-9 (rest), U-11 (rest), U-15 and U-16 (design
Source Questions) are settled in Task 0 by reading the source, and U-14 by an
experiment in Task 1; an answer that differs stops for the owner.

## Re-grounding

**Done 2026-11-09 at `9512080d`** for `01M6KHVD6D0DB8KYT6VGQP10Y0` and for
`Unpublished` (ticket `01M66HFV44YXNQ7ANJVA16TQ1H`). The design's
Re-grounding records each check. In short:

- The library ticket brought `Closed` and `Promoted`, and
  `01M66HFV44YXNQ7ANJVA16TQ1H` brought `Unpublished`. All three are in Fact 5
  and in A4's lists.
- The library ticket assigns D6 the desktop Retire action, and the owner
  added that polls retire, item by item (decisions 16 and 17). That is
  Scope 12, contract 11, A19–A21, decisions 14 to 17 and design Part E.
- An explicit history repair now retires the recovery marker (Fact 8).
  Decision 10 still applies to repositories already suspended.
- The poll entry points, errors, eligibility and cancel are unchanged; only
  their lines moved (Facts 3, 4, 7, 10).

**Still owed:** when Wave 04 Cycle 01 merges, check obligations 1–7 above
against its code: `needs_attention`'s inputs, where the launch refresh and
`finishing` now live, and the panel and region names the tests use. Task 0
does this if Cycle 01 is on main by then, otherwise the merge that brings it
in.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | No boundary path runs an Automatic poll; the probe is crate-private | Owner decision | Decision 1. | `bind/poll.rs` builds `Explicit`; `poll.rs:1243`, `lifecycle.rs:82`. |
| Planning research | One worker thread cannot start a user job while a poll runs | Owner decision | Decision 2 with proposal 3. | `worker.rs` `take`; `poll.rs:1269`. |
| Planning research | Recovery suspension has one setter, the history-lost marker, not unlock | Settled | Needs-attention is desktop-held; decision 10 covers suspension. | `state.rs:39-55`, `6441`. |
| Planning research | The read boundary has no `latest_attempt_at` or dispositions | Settled | Status uses `remote_snapshot` through the backend (design U-3). | `read/status.rs:717`; `state.rs:2125`. |
| Planning research | Handoff draws a global pause and an on/off switch; the library has per-repository pause and no verb for `enabled` | Owner decision | Decision 5 (switch is pause) and 8. | `bind/poll.rs`; handoff 66. |
| Planning research | Raw stalls are characterized; cancellation during a stall is not | Settled | Task 1. | `tests/ssh_fixture.rs:660-733`; no cancel-under-stall test. |
| Owner, 2026-11-08 | Wave 04 Cycle 01 starts soon and reworks the operation area, the quit feedback's placement and Settings' entry points | Settled | Wave 04 Cycle 01 Coordination; decision 11 proposed; A18. | Wave 04 design 229–246, 512–514; Cycle 01 draft decisions 3 and 7; its ticket's "D6 Coordination". |
| Owner, 2026-11-09 | `01M6KHVD6D0DB8KYT6VGQP10Y0` landed and assigns D6 the explicit desktop Retire action | Settled | Re-grounding; Scope 12; contract 11; A19–A21; decisions 14 and 15 proposed; design Part E. | Wave D6 section (amended 2026-11-06); retirement Cycle D4, D6, D7, D13 and D15; API audit "Completed-context retirement delivery". |
| Owner, 2026-11-09 | Cycle 01 is approved and in flight; its quit no longer changes the layout, and it proposes K-19 | Settled | Coordination rewritten; decisions 12 and 13 proposed; A18 revised; K-13 and K-16. | Cycle 01 Cycle (revision 5) decisions 3, 7 and 14, K-18; its note on this ticket (`01M6Z8A8AYDGTK4BWKZR3931ST`); its checkpoint comment `01M6ZXNJFRHZP9381NQQE3FH0F`. |
| Owner, 2026-11-09 | Retire… should show only for what a poll did not retire; polls retire merged items, matching the CLI's `--retire-completed`, with no setting | Owner decision | Decision 16; decision 14 rewritten; decision 15 revised; Scope 12, contract 1 and 11, A8, A13, A19–A21; design Part E and Document Amendment 7. | Owner, 2026-11-09; `src/bin/manyhands-cli/polling.rs` (`retire-completed`); `R/mutation/bind/poll_retirement.rs`. |
| Owner, 2026-11-09 | Operations are per item; the CLI's retirement converged only per item. Narrow decision 15 to the reported items | Owner decision | Decision 17; decisions 14 and 15 rewritten (a per-item `ContextRetire` loop, no `PollOnceRetiring`); A8, A19–A21; design Part E, K-15, K-17 to K-19. | Owner, 2026-11-09. |
| Review round 1, source and feasibility (`d48bc4aa`) | 4 Blocking, 13 Important, 13 Minor | — | Revision 4. Blocking: a first automatic prompt (design B5, R-26); live failures are `Err(Transport)` (A4 rekeyed, R-29); a closed ticket's drafts are export-only (decision 18, contract 11); a killed process's row blocks every new poll (R-24). U-1, U-3–U-8, U-10, U-12 settled. | Design Review Record; Facts 4, 6, 25, 28–31. |
| Review round 1, contract, consistency and completeness (`d48bc4aa`) | 4 Blocking, 20 Important, 19 Minor; 12 owner questions | — | Revision 4. Blocking: PRD and other amendments listed in full (design Document Amendment 7); second-process evidence by a second library actor (A4); entry gate 5 (decision 22); replay and interruption cases (A22). Questions 1, 3, 4, 6 and 11 became decisions 20, 21, 19, 22 and 23; the others are proposals confirmed at approval. | Design Review Record. |
| Owner, after round 1 (2026-11-09) | Kept drafts, the edit race, preemption, a pending item, entry gate 5, launch timing | Owner decision | Decisions 18–23. | Owner, 2026-11-09. |
| Review round 2, scoped re-review (`4d22c393`) | 1 Blocking, 8 Important, 20 Minor | — | Revision 5. Blocking: R-24 could steal a live row (now launch-only from an in-flight record, L-1 to L-6). Retirement codes corrected; attention keying for a retirement (B5); a dead Check now's row (L-4); `Ok(Polled{Interrupted})`; who runs the cancel (B2); teardown (decision 24); same-process actor (decision 26). | Design Review Record. |
| Review round 2, adversarial (`4d22c393`) | 4 Blocking, 10 Important, 10 Minor | — | Revision 5. Blocking: a polling hot loop on `recovery_required` (R-35); stale rows (L-1 to L-6, decision 27); Resume after a relaunch (in-flight record, R-31); "kept draft" defined and edits skipped (R-18). Lane of the resend, passphrase cache (decision 25), quit freeze (R-33), one loop (R-32), transient refusals, dedupe (R-23), entry points (R-3), read-only clearing, R-11 timing. | Design Review Record. |
| Owner, after round 2 (2026-11-09) | Teardown, the passphrase cache, the second process, a stale row in a not-eligible repository | Owner decision | Decisions 24–27. | Owner, 2026-11-09. |
| Review round 3, scoped re-review (`c35719c2`) | 0 Blocking, 6 Important, 14 Minor | — | Revision 6: the lost cancel during a restart (P-1, P-2); the quit freeze from `request_quit` (R-33); decision 27 carried through at runtime (L-1, L-2); fences and `OperationMismatch` (L-3, U-13); L-4 given a task; the answer's reading order made consistent. | Design Review Record. |
| Owner, 2026-11-09 | Approval | Owner decision | Documents approved at `04e1623f`; decisions 3–15 and 28; named edits E-1 to E-6, and further test edits pre-approved; implementation authorized, subagent-driven; the two follow-up tickets filed. | Owner, 2026-11-09. |
| Review round 3, adversarial (`c35719c2`) | 3 Blocking, 9 Important, 10 Minor | — | Revision 6. Blocking: the lost cancel (P-1, P-2); runtime takeover (L-1); an unknown draft listing (RT-5, RT-6). Important: in-flight entry lifecycle (R-31); `Accepted` answers (E3); Resume clearing declined keys; Unlock… target; loop ends (RT-8 to RT-10); read-only per request (E1); transport codes stop the loop (R-37); a running retirement on quit (proposed decision 28); the lock retried on activation. Preemption, stale rows and the loop's rules became required outcomes; no round 4. | Design Review Record. |
| Drafting | Not yet reviewed | — | Review rounds follow. | — |
