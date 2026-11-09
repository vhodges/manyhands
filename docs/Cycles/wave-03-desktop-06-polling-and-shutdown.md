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

Status: **draft, 2026-11-07; amended 2026-11-08 and 2026-11-09.** Not
reviewed and not approved. Planned at `3364370c` (origin/main `1e5616af`
merged into the ticket branch).

- 2026-11-08: origin/main `8da9afd4` was merged as `1869cf14`, and
  coordination with Wave 04 Cycle 01 was added.
- 2026-11-09: ticket `01M6KHVD6D0DB8KYT6VGQP10Y0` landed (main `f4442618`,
  merged as `9512080d`). The documents were re-grounded against it (design
  Re-grounding). They took on the explicit Retire action it assigns to D6
  (Scope 12, design Part E), and they now follow Wave 04 Cycle 01's approved
  decisions 3 and 7.

Approval of these documents does not authorize implementation, which needs
the owner's separate authorization.

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
| API audit refreshed (entry-gate item 3) | Library rows re-grounded at `9512080d` in the design's Baseline Facts and API Audit Refresh; Task 0 re-checks what moves after that (owner-accepted departure, as C4, C5 and D5 recorded theirs, proposed in decision 9). |
| Runtime readiness (entry-gate item 5) | Partly present: the transport initializes before threads (`src/runtime.rs:25`, `src/main.rs:11`) and raw stalls are characterized (Wave 02 Cycle 03). Cancellation during a stall is **not** characterized. Task 1 closes that. |
| Required checks at baseline (entry-gate item 7) | Not run during planning. Task 0 runs them. |
| Tester assignment (entry-gate item 6) | Linux (Wayland): the product owner. Windows and macOS: G1. |
| Wave 04 Cycle 01 (`01M6V8RK8B0SBCH0YZXFVSR9E3`) | **Approved 2026-11-08 and in implementation** (Tasks 1 to 4b done by 2026-11-09). Not a dependency; expected to merge first. Coordination below. |

**Dependency boundary.** D6 changes **no** file in `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs` or `src/lib.rs` (decision 1: no library part, no track-rule-5
exception). Its one change under `tests/` is a single case in
`tests/remote_polling.rs` (Task 1). Any other library gap becomes its own
ticket.

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

1. **Attention in the count (decision 11).** D6's five attention states (key
   locked, host not approved, key missing, configuration required, recovery
   hold) count toward Needs attention. Paused, Checking is off, Can't reach,
   Checking soon, Waiting for a conflict and a Retire offer do not. A refused
   or pending retirement is a finished request or a pending row, which Cycle
   01 already counts. The polling model exposes its attention through one
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
   default window. The three existing rows K-19 names stay with Wave 04
   Cycle 03 (design W-6).
4. **A small footprint in shared files.** The scheduler is held by
   `Services`, not by `Workspace`. D6's only edit to the repository list is one
   call beside the launch refresh in `repositories_loaded`, wherever Cycle 01
   has moved it. Expected textual conflicts: `D/view/workspace/repositories.rs`;
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
12. **Deliberate Retire (Wave D6 section, amended 2026-11-06).** A poll
    result's `closed` ticket or `promoted` document is listed with
    **Retire…**, which submits `Mutation::ContextRetire` for that one item on
    the user lane. First, text in a tab of that item that no draft keeps
    stops it, and the item's drafts are flushed and kept. Then the library's
    exact refusal is shown, or its pending cleanup with the ordinary Resume
    and Abandon (decisions 14 and 15; design Part E).

## Explicit Exclusions And Downstream Obligations

- A CLI daemon, process discovery or heartbeat tables, a shared service,
  coordination of several resident pollers, and a total transport deadline.
  External forced termination is tested as interruption, not rollback.
- Any background push, checkpoint, merge, cleanup, retirement or
  publication.
- Automatic or scheduled retirement. No automatic poll retires or builds
  `PollOnceRetiring` (Wave D6 section). The desktop offers no batch "check
  and retire" (decision 14).
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
8. **Honest shutdown.** "Still stopping" at ten seconds in the quit's entry,
   in Activity (decision 12); the UI stays responsive; nothing is called
   rolled back that was not.
9. **Keyboard.** Every new control is reachable and operable by keyboard
   with visible focus.
10. **Privacy.** No item title, body, comment or key material enters an
    operation entry, status line, log or error message.
11. **Retire is deliberate and loses nothing.** Retire runs only from a
    person's action on one `closed` or `promoted` item, never from a schedule
    or a poll. It does not run while a tab of that item holds text no draft
    keeps. It never deletes a draft: the item's drafts and its comments'
    drafts stay listed, restorable and exportable on the Drafts page.
    Refusals and pending cleanup are shown exactly as the library answers,
    with the ordinary recovery.

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
| A8 | No background push, checkpoint, merge, cleanup or retirement | Test double records every service call an automatic poll makes: only eligibility, poll and status reads, never `ContextRetire` or `PollOnceRetiring` | Pending |
| A9 | No remaining worker after graceful exit | Test joins the worker thread; native §6 checks the process list after quit | Pending |
| A10 | Manual priority | Desktop test: a user job submitted during a poll cancels it and runs first; the poll is rescheduled with no failure backoff | Pending |
| A11 | Launch poll respects backoff | Model test with an injected clock | Pending |
| A12 | Suspend and clock jump: one poll, no catch-up | Model test | Pending |
| A13 | No background prompt | Desktop test: a locked key yields needs-attention and no `Prompt` event | Pending |
| A14 | Unsaved edits survive a bring-in | Desktop test with an open dirty edit session and a kept draft | Pending |
| A15 | Keyboard and focus (contract 9) | Keyboard tests; native §7 | Pending |
| A16 | Privacy (contract 10) | Sentinel tests | Pending |
| A17 | One scheduler whatever the windows | Test: the scheduler is owned by `Services`; a second `Services` clone starts none | Pending |
| A18 | Wave 04 Cycle 01 coordination: D6's attention states in the Needs attention count; "still stopping" in the quit's Activity entry with no layout change; D6's rows fit Activity | With Cycle 01 on main: `needs_attention`'s test extended with each attention state and each state that does not count (E-4); D6's still-stopping test run with Activity hidden (E-5); D6's rows checked at the default window by the actions' keyboard reach. Otherwise recorded as Cycle 01's to wire | Pending |
| A19 | Retire journey: a `closed` ticket and a `promoted` document are retired by keyboard, and lists and an open tab then read primary | Desktop tests over canned `context_retirement` answers (decision 9); library SSH case `retirement_cli_authenticated_live_pass`; native retire section on two clones | Pending |
| A20 | Exact refusals and pending cleanup | Desktop table test: each `refused` or `preserved` code shown as the library gives it, and nothing else submitted; a `pending` answer leaves a pending row whose Resume and Abandon (with the retirement details of the library ticket's D13) work by keyboard | Pending |
| A21 | Open editors and drafts preserved | Desktop tests: unkept tab text stops Retire and shows the tab; a kept item draft and a comment draft outlive retirement, listed, restorable and exportable on the Drafts page | Pending |

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
11. **What D6 adds to Needs attention** (2026-11-08, for Wave 04 Cycle 01).
    The five attention states count, since each waits for the person to act.
    Paused and Checking is off are the person's own choice; Can't reach and
    Checking soon retry by themselves; Waiting for a conflict points at a
    conflict that is already shown where it is resolved; a Retire offer is
    an option, not a failure. None of those count.
12. **Still stopping lives in Activity** (2026-11-09, after Wave 04 Cycle
    01's revised decision 7). With Activity hidden, a quit still waiting at
    ten seconds shows nothing until Activity is opened, as Cycle 01's own
    quit feedback does (its K-18). D6 shows no panel and adds no second
    place: the owner removed showing Activity from Cycle 01 after its review
    round 3. This narrows the Wave's "shutdown reports ongoing work at ten
    seconds" to "reports it in Activity" (K-13). The owner's later blocking
    quit dialog is where a report with Activity hidden belongs.
13. **D6's rows fit Activity.** Each row D6 adds to the operation area wraps
    so that its actions are inside Activity at the default window. The
    three existing rows Cycle 01's proposed K-19 names stay with Wave 04
    Cycle 03 (K-16).
14. **Retire one item at a time, from a poll result.** Retire… appears on
    each `closed` or `promoted` target of a repository's last poll,
    automatic or Check now, and submits `ContextRetire` for that item. The
    desktop never builds `PollOnceRetiring`: no "Check now and retire" and
    no batch (K-15). The library proves closure and cleanliness again on
    every run, so an offer from an older poll is safe (K-14).
15. **Unkept text stops Retire; there is no "Retire anyway".** When a tab of
    the item holds text that no draft keeps, Retire opens a dialog that
    gives the count, with no title or text, and offers Show tab (the
    default) and Cancel. Otherwise the item's draft sessions are flushed
    before submission. Retirement never removes a draft.

## Open Questions

None for the owner beyond confirming decisions 3–15 at approval. U-1 to U-11
(design) are settled in Task 0 by reading the source; an answer that
differs stops for the owner.

## Re-grounding

**Done 2026-11-09 at `9512080d`** for `01M6KHVD6D0DB8KYT6VGQP10Y0` and for
`Unpublished` (ticket `01M66HFV44YXNQ7ANJVA16TQ1H`). The design's
Re-grounding records each check. In short:

- The library ticket brought `Closed` and `Promoted`, and
  `01M66HFV44YXNQ7ANJVA16TQ1H` brought `Unpublished`. All three are in Fact 5
  and in A4's lists.
- The library ticket assigns D6 the desktop Retire action. That is Scope 12,
  contract 11, A19–A21, decisions 14 and 15 and design Part E.
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
| Drafting | Not yet reviewed | — | Review rounds follow. | — |
