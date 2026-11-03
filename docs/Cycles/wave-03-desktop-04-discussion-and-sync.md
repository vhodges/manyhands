---
title: "Wave 03 D4: Desktop Discussion And Deliberate Synchronization"
date: 2026-11-02
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M6E695X6AJ8P9QP2ES0SEM38"
---

# Wave 03 D4: Desktop Discussion And Deliberate Synchronization

Ticket `01M4CC0VN1KTQFSVR99378TDHV`. Design:
[2026-11-02-wave-03-desktop-04-discussion-and-sync-design.md](../plans/2026-11-02-wave-03-desktop-04-discussion-and-sync-design.md).
Plan:
[2026-11-02-wave-03-desktop-04-discussion-and-sync-implementation.md](../plans/2026-11-02-wave-03-desktop-04-discussion-and-sync-implementation.md).

Status: **approved 2026-11-02** (revision 5), planned at `00bef3da`
(origin/main `15a23125` merged into the ticket branch). The owner approved
the Cycle, design and plan, confirmed decisions 5–15, and authorized
implementation by subagent-driven development on 2026-11-02. Pushing
beyond the ticket branch, merging and closing the ticket still need their
own go-ahead.

## Purpose And Authority

Make discussion and explicit publication understandable and usable in the
desktop: write comments and replies, post them, share an item or a
repository deliberately, see what happened, stop at a safe point, and
recover from a locked key, an unknown host or a failed publication without
losing or duplicating work.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 03](../Waves/wave-03-dogfooding.md): section D4 (lines 939–970),
   track rules (326–371), entry gate (113–215).
3. RFCs: [desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md),
   [canonical content and comment schema](../RFC/canonical-content-and-comment-schema.md),
   [MVP RFC](../RFC/mvp-rfc.md),
   [Git workflow and conflict recovery](../RFC/git-workflow-and-conflict-recovery.md),
   [authentication and credential handling](../RFC/authentication-and-credential-handling.md),
   [application runtime and polling](../RFC/application-runtime-and-polling.md),
   [test and compatibility strategy](../RFC/test-and-compatibility-strategy.md),
   and the [CLI contract](../RFC/cli-contract.md) for the bindings' meaning.
4. PRD [`docs/PRD/mvp.md`](../PRD/mvp.md): `MH-COMMENT-001/002`,
   `MH-COLLAB-003/004`, `MH-UX-001`, `MH-NFR-002/005`, and the journey
   "Durable Threaded Discussion".
5. The design handoff: [`docs/design/item_view/`](../design/item_view/README.md),
   especially [`04-comments.md`](../design/item_view/04-comments.md), and the
   workspace handoff [`docs/design/`](../design/README.md).
6. Predecessors: [D3](wave-03-desktop-03-rich-text-and-source-authoring.md)
   (decision 12), [D2](wave-03-desktop-02-recoverable-drafts.md),
   [D1](wave-03-desktop-01-shell-and-onboarding.md), and
   [C4](wave-03-cli-04-discussion-and-sync.md).

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`00bef3da`) |
| --- | --- |
| Wave 02 Cycles 06 and 07 on main | Closed and merged before the Wave 03 replan baseline. |
| C4 merged (track rule 2) | Merged at `89c8383b`; ticket `01M4CC0VMWEQQ9G4W0DAG7ADPY` closed 2026-10-31. |
| D1, D2, D3 merged (track rule 1) | All closed and merged. |
| API audit refreshed (entry-gate item 3) | Done at planning: design, API Audit Refresh. |
| Required checks at baseline (entry-gate item 7) | Not run during planning. Task 0 runs the full set, as D3's did. |
| Tester assignment (entry-gate item 6) | Linux (Wayland): the product owner (decision 6). Windows and macOS: G1. |

**Dependency boundary.** D4 changes no file in `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs` or `src/lib.rs` (track rule 5). A gap becomes its own
library ticket (`file-ticket`).

**In flight.** C5 (`01M4CC0VMXTNXNH78C11DYXS8H`) edits the sync, resume,
progress and DTO modules D4 consumes, and `src/desktop/view/confirm.rs`.
D4 merges main whenever it moves (design, Lane mechanics).

## Scope

1. **Composers.** A new-comment composer after the threads and one reply
   composer at a time inside a thread: small `zorite-editor` instances with
   the Rich text / Markdown switch (decision 3), a mini toolbar, Ctrl Enter
   to post, Esc to cancel with "Discard this reply?" (or "Discard this
   comment?") when text is present.
2. **Comment drafts.** Each composer's text is a D2 draft with a comment
   target, flushed within 1 s, restored after a crash, per item and
   thread, separate from the body draft. The Drafts page opens them.
3. **Posting.** Post and sync with a publication remote; Save comment
   locally without one. One comment ID per comment, kept for its life; one
   request ID per submission, reused by every retry the library offers.
   A second submit while posting is refused.
4. **Posting outcomes.** Posting, posted and shared, saved here (no remote),
   saved but not shared (shown once, with its next action), stopped,
   checkpoint failed.
5. **Deliberate synchronization.** Share in the item header (`item sync`)
   and Share main in the repository detail (`repo sync`). Outcomes:
   shared, already current, stopped, needs attention.
6. **Progress and safe-point stop** for shares, posts and resumes.
7. **Credential recovery.** Unlock… after Not now; Open Sharing & keys for
   a missing key; Check host for an unknown or changed host; nothing
   prompts again until the user acts.
8. **Conflicts as preserved states.** A share or post that stops on a
   conflict shows the item banner and a needs-attention entry, with no
   resolution (decision 13).
9. **Publication after restart.** A pending comment publication or remote
   synchronization shows Resume in the operation area.

## Explicit Exclusions And Downstream Obligations

- Conflict editing and the Resolve sheet; Close ticket… and Approve and
  merge…; `item republish`: D5.
- Scheduled polling, launch polls, background prompts, quit-while-stopping
  behaviour: D6.
- Activity panel, Needs attention popover, footer counts (decision 2).
- "N to share" counts and a persistent "not shared yet" marker: need a
  library read (decision 14).
- Abandoning a pending remote conflict (`01M4H33R34Z7C7EEKTY1ZCT950`).
- The PRD journey's worktree-switch step: G1 (Wave line 72).
- The dockable Comments panel and the design's "Later" items; scrolling to
  a comment from Activity (`04-comments.md` Navigation needs Activity).
- Known library limits K-1 to K-9 (decision 4, design Known Limits).

## Required Contract

1. **No implicit publication.** Only Post/Reply and sync, Share, Share
   main, Retry sharing, Unlock…, Resume and Give up start or end a
   synchronization. Navigation, Save, Done, closing a tab and quitting
   never start one. Quitting stops a running one at a safe point
   (decision 15).
2. **Unsaved body text is never posted or shared.**
3. **One comment, once.** No path creates a second comment for one
   composer text, and no retry writes a comment twice.
4. **Saved is not shared.** Posting, shared, saved here, saved but not
   shared, stopped and not saved are told apart; a comment shows once.
5. **Truthful progress.** The latest stage is shown; Stop is offered only
   while it can work; the outcome says what completed.
6. **Credentials.** One prompt at a time, only for a request the user
   started. After Not now, nothing prompts for that key until Unlock….
   Hosts are approved only through the Hosts page's confirmed flow.
7. **Conflicts are preserved and visible**, never retried automatically.
8. **Stale results lose** to newer composer text, listings and views.
9. **Keyboard.** Every new control is reachable and operable by keyboard
   with visible focus, in the design's tab order.
10. **Privacy.** Comment text never enters an operation entry, quit
    dialog, log, error or dialog message.

## Acceptance And Exit Evidence

| # | Wave exit evidence / contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | Keyboard discussion journey | Desktop keyboard test (no remote); native §2 against SSH | Planned |
| A2 | Keyboard explicit-sync journeys against real SSH | Native §3 | Planned |
| A3 | Offline / local pending | Desktop tests (no remote; `publish_over_ssh`); native §4 | Planned |
| A4 | Failed publication; saved once; retry without resubmission | Desktop tests: one file and one checkpoint after two Retry sharing; native §4 | Planned |
| A5 | Repeated-submit guard | `Submissions` and composer tests; Share guard test | Planned |
| A6 | Rebuild / thread ordering | Desktop test: out-of-order posts, Rebuild index, reopen; native §2 | Planned |
| A7 | Unlock cancellation | Provider and dialog tests; constructed `unlock_required` routing; native §5 (prompt once) | Planned |
| A8 | Continued UI responsiveness | Desktop tests typing while a post is held; native §6 | Planned |
| A9 | Safe-point stop | Worker test (token cancelled after acceptance); stop control tests; native §6 | Planned |
| A10 | Conflict as preserved state | Constructed-envelope tests for banner and entry; native §7 | Planned |
| A11 | Comment drafts recover | Draft and adoption tests; native §2 | Planned |
| A12 | Post never includes unsaved body edits | Desktop test | Planned |
| A13 | Key and host recovery | Constructed-envelope routing tests; native §5 | Planned |
| A14 | Publication resume after restart | Resume-offer tests; native §4 | Planned |
| A15 | Privacy | Sentinel tests | Planned |
| A16 | No implicit publication (contract 1) | Desktop test over navigation, Save, Done, close, quit | Planned |
| A17 | Stale results lose (contract 8) | Desktop test: outcome after new typing or a tab switch | Planned |

Native evidence: Linux (Wayland), release build, run by the product owner
from the checklist written in Task 15. The agent records only what the
owner reports.

## Product-Owner Decisions (2026-11-02)

Answered by the owner during planning:

1. **Parallel lanes on sub-branches.** After Part 0, Lanes A, B and C run
   in parallel, each in a short-lived worktree branched from the ticket
   branch and merged back, never rebased. A recorded exception to
   `AGENTS.md`'s "all implementation work … in the ticket's worktree and on
   its branch": lane branches are local and everything lands on the ticket
   branch before review-ready.
2. **D1's operation area carries progress and outcomes.** No Activity
   panel, Needs attention popover or footer counts in D4.
3. **Composers are small `zorite-editor` instances** with the Rich text /
   Markdown switch.
4. **Library defects are Known Limits** (`01M669FJR27FRHCNPWAMD6VH4V`,
   `01M66HFV44YXNQ7ANJVA16TQ1H`, `01M67BN4C1SY2HQBGNE9NBYMW8`).

Proposed by the planner and confirmed by the owner with the documents'
approval on 2026-11-02:

5. **Replies answer the root** (`04-comments.md`); deeper replies on disk
   keep D3's single indent (R-53).
6. **Linux-only native evidence** for D4, with the owner as tester.
7. **The design's words** in the UI: Share, Share main, Retry sharing,
   Reply and sync, Save reply locally; the desktop RFC is amended to match.
8. **Give up** ("Stop trying to share") is offered on any share or post
   of this session whose library record is still accepted (including after
   Not now). It cancels that request and sends it once more, which
   normally ends it `cancelled` and releases the repository's
   reservation; when a push's outcome is unknown it may first ask the
   remote, and its answer is shown as it comes. The comment stays saved
   and Share sends it later. After a deleted shared branch, no Share is
   offered (republishing is D5's).
9. **No Share dialog.** Scope is stated in the tooltip and the operation
   entry; API audit row D25 is amended.
10. **Share is enabled whenever a publication remote exists**; the library
    answers for an item with no editing context.
11. **Resume** is offered for comment publications and remote item/repo
    synchronizations, not for local-context synchronizations or
    resolutions (D5).
12. **Saves can queue behind a share** waiting on the network or a prompt
    (one worker); accepted for D4, revisited in D6.
13. **The conflict banner has no action** until D5.
14. **The "not shared yet" chip is session-only.** After a restart the
    pending publication shows as an operation row with Resume; a library
    read for a persistent marker becomes its own ticket.
15. **Quitting while sending stops at a safe point.** D1's quit cancels
    the running request; a stopped publication is final, so after relaunch
    nothing marks the comment and Share publishes it (K-10). D6 owns
    shutdown.

16. **Execution is subagent-driven** (2026-11-02, with the authorization
    to implement): the orchestrator dispatches one implementer per task,
    with an independent review after each, following the lane plan.

17. **Share item and Share main have their own job kinds** (2026-11-02,
    Task 0): the existing "kind of its own" test assertion is kept; its
    lists grow as named edit E-12.
18. **Share on a never-edited item** says "Nothing to share yet: this
    item has no saved changes." instead of a generic failure (2026-11-02,
    U-1).

19. **Tab indents in a comment composer** (2026-11-02, Task 4 review),
    as in the body editor; F6 and Shift-F6 leave the composer. This
    replaces the planner's "Tab leaves the composer" (design, Slots and
    keys), which the owner had not chosen.

20. **Share learns the remote from navigation** (2026-11-02, Lane B Task 8):
    the item tab takes its repository's publication remote from the
    workspace's repository summary, so opening a tab adds no remotes read
    (D3's fact 41 unchanged); a seam on the ticket branch passes it in.
    Named edit E-13 adds Share to the frame tab-order test.

21. **The new-comment composer's toolbar is always shown** (2026-11-02,
    Lane C Task 11a review), not revealed on focus as
    `04-comments.md:43` draws it: with Tab indenting (decision 19) a
    revealed-on-focus toolbar could not be reached by keyboard. Ctrl K
    opens Link in the composer.

22. **The new-comment composer starts collapsed** (2026-11-02, Task 12)
    as one "Write a comment…" tab stop; Enter, Space or a click opens it
    and focuses its editor, and it stays open while it holds text or a
    draft. An empty composer at the foot of every item had trapped a
    keyboard walk, since Tab indents inside it (decision 19). Once open,
    Tab indents and F6 leaves, like the body editor. An open composer
    with no text and no draft collapses again when focus leaves it; an
    empty Cancel, or focus leaving a composer emptied of a draft it began
    this session and never submitted, discards that draft and collapses it
    (orchestrator, Task 12 review, within this decision).

23. **Post as a new comment after a provably empty attempt** (2026-11-03,
    Task 13 design checkpoint): when Check again finds a finished request
    whose stored result shows nothing written and nothing committed, and
    the comment is not in a complete, current listing, Post as a new
    comment is offered (extends P16).
24. **A listed comment with no request record** (records lost): Check
    again retires the draft and clears the composer, and the thread shows
    the comment, without claiming "Saved" or showing the not-shared chip.
25. **Reopening a tab after an unsaved post finished** re-adopts its draft
    into the composer when that slot is untouched and empty, with the
    failed state and its actions.

## Open Questions

None. U-1 to U-5 (design) are settled in Task 0 by reading the source.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | D4 ticket `deps` omits C4 | Settled | Document Amendment 6. | Ticket lines 4–5; Wave line 336. |
| Planning research | The desktop worker cannot stop an accepted request | Ruling R-1 | Part 0. | `services/worker.rs:437-454`. |
| Planning research | No read reports unpublished work | Decisions 2, 14 | Session-only chip. | Design fact 15. |
| Planning research | No Share dialog is designed | Decision 9 | Confirmed; API audit D25 amended. | `05-sheets-and-dialogs.md`. |
| Planning research | Design vocabulary differs from the RFC | Decision 7 | Confirmed; desktop RFC amended. | RFC desktop IA 257–262. |
| Review round 1, source and feasibility (`eec96a4c`) | 5 Blocking, 10 Important, 7 Minor | — | Revision 2. | Design Review Record. |
| Review round 1, contract and consistency (`eec96a4c`) | 3 Blocking, 18 Important, 1 Minor group | — | Revision 2; rulings that changed behaviour became decisions 8–14. | Design Review Record. |
| Review round 2, scoped with adversarial brief (`6ed12ae6`) | 2 Blocking, 7 Important, 9 Minor | — | Revision 3: required outcomes P1–P10; decision 8 widened; decision 15. | Design Review Record. |
| Review round 3, scoped with adversarial brief (`0fa796dd`) | 1 Blocking, 5 Important, 6 Minor | — | Revision 4: durable request ID (R-16), record state in outcomes. Not independently re-reviewed; Task 13's design checkpoint guards the mechanism. | Design Review Record. |
| Review round 4, narrow review of revision 4 (`2fdd1809`) | 2 Blocking, 6 Important, 3 Minor | — | Recorded as required outcomes P11–P18; not re-reviewed; Task 13's design checkpoint owns the mechanism. | Design Review Record. |
| Owner approval (2026-11-02) | Documents, decisions 5–15, E-1 to E-11, execution method | Owner decision | Approved at revision 5; implementation authorized, subagent-driven (decision 16). Document Amendments 1–6 applied in the approval commit. | Ticket approval comment. |
