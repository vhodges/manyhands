---
title: "Wave 03 D4: Desktop Discussion And Deliberate Synchronization"
date: 2026-11-02
status: "draft"
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

Status: **draft**, planned at `00bef3da` (origin/main `15a23125` merged
into the ticket branch). Nothing has been built or run.

## Purpose And Authority

Make discussion and explicit publication understandable and usable in the
desktop: write comments and replies, post them, share an item or a
repository deliberately, see what happened, stop at a safe point, and
recover from a locked key, an unknown host or a failed publication without
losing or duplicating work.

Authorities, in order:

1. `AGENTS.md` on origin/main.
2. [Wave 03](../Waves/wave-03-dogfooding.md), section D4 (lines 939–970),
   its track rules (326–371) and entry gate (113–182).
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
   workspace handoff [`docs/design/`](../design/README.md)
   (`04-item-area.md`, `05-sheets-and-dialogs.md`, `06-language-and-states.md`).
6. The predecessor Cycles: [D3](wave-03-desktop-03-rich-text-and-source-authoring.md)
   (decision 12), [D2](wave-03-desktop-02-recoverable-drafts.md),
   [D1](wave-03-desktop-01-shell-and-onboarding.md), and
   [C4](wave-03-cli-04-discussion-and-sync.md) for the library bindings.

## Entry Evidence And Dependency Boundary

| Gate | State at planning (`00bef3da`) |
| --- | --- |
| Wave 02 Cycles 06 and 07 on main | Closed and merged before the Wave 03 replan baseline. |
| C4 merged (track rule 2, C4 decision 5) | Merged to main at `89c8383b`; ticket `01M4CC0VMWEQQ9G4W0DAG7ADPY` closed 2026-10-31. |
| D1, D2, D3 merged (track rule 1) | All closed and merged. |
| API audit refreshed for the consumed rows (entry-gate item 3) | The audit's "C4 delivery reconciliation (2026-10-30)" covers `comment add`, `item sync`, `repo sync`, `operation resume` and the conflict data. Rows D23 and D24 name D4. Task 0 records the refresh against the merged code. |
| Required checks at baseline (entry-gate item 7) | Not run during planning. Task 0 runs them. |
| Tester assignments (entry-gate item 6) | Unrecorded. Decision 6: D4's native evidence is Linux only; Windows and macOS journeys stay with G1. |

**Dependency boundary.** D4 changes no file in `src/repository.rs`,
`src/repository/`, `src/results.rs`, `src/canonical.rs`, `src/drafts/`,
`src/runtime.rs` or `src/lib.rs` (track rule 5). It consumes the F2
boundary (`RepositoryService::execute`, `cancel_request`, `show_request`)
and the F1 reads only. A gap found during implementation stops the task and
becomes its own library ticket (`file-ticket`).

**In flight at planning.** C5 (`01M4CC0VMXTNXNH78C11DYXS8H`) edits the
sync, resume, progress and DTO modules D4 consumes, and
`src/desktop/view/confirm.rs`. It may merge to main during D4. D4 merges
main at each lane boundary (design, Execution Lanes) and re-checks the
facts the merge touched.

## Scope

1. **Composers.** A new-comment composer at the end of the discussion and
   one reply composer at a time inside a thread, each a small
   `zorite-editor` instance with the Rich text / Markdown switch (decision
   3), a mini toolbar, Ctrl Enter to post and Esc to cancel, with "Discard
   this reply?" when text is present.
2. **Comment drafts.** Each composer's text is a D2 draft with a comment
   target, flushed within 1 s, restored after a crash, scoped per item and
   thread, separate from the item's body draft. The Drafts page lists and
   restores them.
3. **Posting.** Post and sync with a publication remote; Save comment
   locally without one. One comment ID per submission, kept through every
   retry, and one request ID per submission, kept through every retry in the
   session. A repeated submit while one is in flight is refused.
4. **Posting outcomes.** In progress, posted and shared, saved but not
   shared (shown once, with Retry sharing), saved with no remote, and
   checkpoint failed, as `04-comments.md` tables them.
5. **Deliberate synchronization.** The item header's Share runs `item sync`
   on the item's context. The selected repository's detail offers Share for
   `repo sync` on the primary branch. Both report published, already
   current, pending, or needs attention.
6. **Progress and safe-point stop.** A running share or post shows its
   stage. "Stop at a safe point" asks the running request to stop; the
   result says whether it stopped before a durable step or completed it.
7. **Credential recovery.** A locked key leads to Unlock, which retries the
   same request and prompts once. Cancelling the prompt leaves local work
   usable and asks again only when the user chooses Unlock. A missing key
   leads to the Keys page. An unknown or changed host leads to the Hosts
   page check, then Approve or Replace, then the same request again.
8. **Conflicts as preserved states.** A share or post that stops on a merge
   conflict shows a banner and an operation entry that say so and offer no
   resolution (D5 owns resolution). Nothing is discarded.
9. **Publication retry across restart.** A comment whose publication is
   still owed after a restart shows as a pending operation with Resume,
   which publishes it without writing it again (`operation resume`).

## Explicit Exclusions And Downstream Obligations

- **Conflict result editing** and the Resolve sheet: D5.
- **Close ticket… and Approve and merge…** stay disabled: D5.
- **Scheduled polling**, launch polls and background prompts: D6.
- **Activity panel, Needs attention popover and footer counts** (decision
  2): workspace design Cycles. D4 uses D1's operation area, item banners and
  comment chips.
- **"N to share" counts.** No library read reports unpublished work, and
  track rule 5 forbids computing it in the front end. A library ticket is
  needed before any front end can show it.
- **Abandoning a pending remote conflict** (`01M4H33R34Z7C7EEKTY1ZCT950`).
- **The dockable Comments panel, a sort toggle and a comment's
  scroll-to-highlight from Activity** (`04-comments.md`, "Later").
- **Republishing a deleted item branch** (`item republish`): D4 shows
  `remote_branch_deleted` as needs attention, with C4's documented way out
  (Cancel, then the same request). A Republish control needs the
  confirmation dialog work that D5 brings.
- **Known library limits** (decision 4) are recorded in the design's Known
  Limits; D4 does not work around them in the front end.

## Required Contract

1. **No implicit publication.** Only Post and sync, Reply and sync, Share,
   Retry sharing, Resume and Unlock (a retry) start a synchronization.
   Navigation, Save, Done, closing a tab and quitting never do.
2. **Unsaved body text is never posted or shared.** A comment's draft and
   the item's body draft are separate drafts. Posting or sharing while the
   body is dirty publishes only checkpointed work, and the composer and the
   Share tooltip say so.
3. **One comment, once.** A submission allocates one comment ID when its
   draft begins and one request ID when it is first posted. Every retry in
   the session reuses both and the same body and author. No path creates a
   second comment for one submission.
4. **Saved is not shared.** The UI distinguishes posting, posted and
   shared, saved here and not shared yet, saved here with no remote, and
   checkpoint failed. A comment is shown once whatever the outcome.
5. **Progress is truthful.** Each running request shows the library's
   latest stage. Stop at a safe point is offered only while the request can
   still stop, and the outcome reports what completed.
6. **Credentials.** At most one passphrase prompt at a time, only for a
   request the user started. Cancelling it leaves the request finished
   with `unlock_required` and offers Unlock; nothing prompts again until the
   user acts. A host is approved only through the Hosts page's existing
   confirmed flow.
7. **Conflicts are preserved and visible.** A conflict stop is never shown
   as success, never retried automatically and never discarded.
8. **Stale results lose to newer state.** A late result never replaces a
   newer composer text, thread listing or item view.
9. **Keyboard.** Every new control is reachable and operable by keyboard,
   with visible focus, in the design's tab order.
10. **Privacy.** Comment text never enters an operation entry, a log, an
    error or a dialog message.

## Acceptance And Exit Evidence

| # | Wave exit evidence / contract | Proof |
| --- | --- | --- |
| A1 | Keyboard discussion journey | Desktop keyboard tests (no remote, real repository): compose, reply twice to one root, post, reopen; native checklist §2 against a real SSH repository. |
| A2 | Keyboard explicit-sync journeys against real SSH repositories | Native checklist §3 (item Share, repository Share, already current, incoming change merged). |
| A3 | Offline / local pending | Desktop tests: Save comment locally with no remote; native §4 with the remote unreachable. |
| A4 | Failed publication, saved once, retry without resubmission | Desktop tests with an unreachable SSH remote (the transport is uninitialized in tests, so publication fails after the checkpoint): one comment file, one thread row, Retry sharing reuses the request; native §4 succeeds on retry. |
| A5 | Repeated-submit guard | Desktop tests: second Ctrl Enter / click while posting queues nothing. |
| A6 | Rebuild / thread ordering | Desktop tests: two roots and three replies posted out of order, then index rebuild and reopen; order and parent links hold (PRD journey). |
| A7 | Unlock cancellation | Desktop tests with a protected key fixture: Cancel at the prompt leaves `unlock_required`, Unlock prompts once; native §5. |
| A8 | Continued UI responsiveness | Desktop tests: typing in the body and composer while a post is held by `DelayingBackend`; native §6 with a stalled remote and Stop at a safe point. |
| A9 | Safe-point stop | Worker tests: cancelling an accepted sync signals its token; desktop tests: the stop control and outcome text; native §6. |
| A10 | Conflict as preserved state | Native §7 (second actor via `manyhands-cli` changes the same ticket); model tests map the conflict data to the banner. |
| A11 | Comment drafts recover | Desktop tests: crash simulated by dropping the registry; composer restores per thread; Drafts page restores. |
| A12 | Post never includes unsaved body edits | Desktop tests: dirty body, post comment, body file unchanged in the context. |
| A13 | Host / key recovery | Desktop tests: `no_selected_key` → Keys page; host codes → Hosts page check; native §5. |
| A14 | Publication resume after restart | Desktop tests: pending publication row offers Resume; native §4. |
| A15 | Privacy | Privacy tests: sentinel comment text absent from operation entries and messages. |

Native evidence runs on Linux (Wayland), on a release build, by the
product owner, from the design's checklist. The agent never records a
result it did not see.

## Product-Owner Decisions (2026-11-02)

1. **Parallel lanes on sub-branches.** After a serial Part 0, the work
   runs in lanes, each in its own short-lived worktree branched from the
   ticket branch and merged back after review, never rebased. This is a
   recorded exception to `AGENTS.md`'s "all implementation work in the
   ticket's worktree and branch": lane branches are local, live only during
   the Cycle, and everything lands on the ticket branch before review-ready.
2. **D1's operation area carries progress and outcomes.** Activity, the
   Needs attention popover and footer counts are not built in D4.
3. **Composers are small `zorite-editor` instances** with the Rich text /
   Markdown switch, reusing D3's editor host and its rulings.
4. **Library defects are Known Limits.** `01M669FJR27FRHCNPWAMD6VH4V`,
   `01M66HFV44YXNQ7ANJVA16TQ1H` and `01M67BN4C1SY2HQBGNE9NBYMW8` are not
   fixed in D4. Fixtures and the checklist avoid them; the host stop uses
   the existing check, approve and retry path.

Recorded with the owner's answers, proposed by the planner and not yet
confirmed:

5. **Replies answer the root**, as `04-comments.md` specifies, and deeper
   replies already on disk keep D3's single indent (R-53).
6. **Native evidence is Linux only** for D4; Windows and macOS journeys
   are G1's.
7. **The design's words win in the UI**: "Share", "Retry sharing",
   "Reply and sync", "Save reply locally". The desktop RFC gets an
   amendment mapping its action names to them.

## Open Questions

None beyond decisions 5–7, which await confirmation.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning research | D4 ticket `deps` omits C4 | Settled | The ticket's deps gain `01M4CC0VMWEQQ9G4W0DAG7ADPY` with the planning checkpoint. | Ticket lines 4–5; Wave line 336. |
| Planning research | The desktop worker cannot stop an accepted request | Ruling R-1 | Part 0 lets the worker signal an accepted sync or comment publication; local commands stay unstoppable. | `services/worker.rs:437-454`. |
| Planning research | No read reports unpublished work | Decision 2 | "N to share" excluded; chips come from the mutation result. | Design fact 14. |
| Planning research | No Share dialog is designed | Ruling R-6 | Share runs without a dialog; progress in the pill and operation area. | `05-sheets-and-dialogs.md` has Close and Approve dialogs only. |
| Planning research | Design vocabulary differs from the RFC | Decision 7 | Pending owner confirmation. | RFC desktop IA 257–262; `06-language-and-states.md:12-14`. |
| Planning research | Design says "in this ticket"; documents have comments too | Ruling R-9 | "in this ticket" / "in this document" by kind. | `04-comments.md:39`; RFC desktop IA 249. |
