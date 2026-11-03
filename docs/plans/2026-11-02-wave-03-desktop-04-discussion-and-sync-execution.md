---
title: "Wave 03 D4 Desktop Discussion And Deliberate Synchronization Execution Ledger"
date: 2026-11-02
status: "in progress"
author: "Claude"
---

# D4 Execution Ledger

Ticket `01M4CC0VN1KTQFSVR99378TDHV`. Plan:
[implementation](2026-11-02-wave-03-desktop-04-discussion-and-sync-implementation.md).
Approved 2026-11-02 (`5c26a5cd`); execution is subagent-driven
(Cycle decision 16). The orchestrator alone edits this file.

## Ticket branch (Part 0, Lane D, Part E)

| Task | State | Base | Commits | Tests | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | done | `605ff289` | `dba94593`, `38b3c18f` (main), `2394b88e` | Full suite at `605ff289`: 32 result lines, 4901 passed, 0 failed, 1 ignored | Orchestrator spot-check of U-items | Decisions 17, 18; main's tiered gate adopted. |
| 1 | done | `45028fc9` | `71f8eefe` | `requests`: 11 passed; fmt, check, both clippy lines clean | APPROVE, 0 Blocking, 0 Important, 2 Minor (optional, not applied) | E-12 applied; `DelayingBackend` needed no change. |
| 2 | done | `31ce9f15` | `0a54d50a`, fixes `7af85640` | `worker` 37, `prompts` 8, `submissions` 7 (each twice); `services::` 153; desktop bin 1107 at `0a54d50a`; four lines clean | Round 1 CHANGES REQUIRED (1 Important: a dropped re-send forgot a written post; 5 Minor); all fixed; re-review APPROVE | Record state kept on the handle (not on `Finished`); `request_not_found` counts as Finished (P16 calls `show_request` itself); test-only constructed answers in `DelayingBackend`; Give up with a failing `cancel_request` on an accepted record is untested (no tool). Carry: `model/operations.rs` marks acceptance only on `Accepted` (R-14) — Lane B Task 9/10a. `worker.rs` ~1100 lines. |
| 3 | done | `8e4aca09` | `2f1b1925`, fixes `5bd74b38`, `79d1b84a` | `publication` 40 (37 in the module); four lines clean | Round 1 APPROVE with 2 Important (record decoupled from offers; P14 unmodelled) and a design gap; fixed; re-review APPROVE; row 14 extension checked by the orchestrator | Ruling R-17 (no same-request resend on a Finished record; row 14 Finished: Discard, Copy text, Check again). Renamed tests: `request_mismatch_is_mismatched`, `internal_error_is_maybe_saved_only_on_a_resend`. `sync_outcome` takes `ShareAttempt { target, record, after_give_up }`. Carry to Lane B: Share offered after a finished NeedsUnlock must clear the declined key first (R-3, R-13). |
| 4 | done | `acfc7290` | `c1e00eab`, fixes `0aa3387e` (docs `1643a500`) | `item` 380, `keyboard` 41; desktop bin 1146 at `0aa3387e`; four lines clean | Round 1 CHANGES REQUIRED (Blocking: composer Tab binding shadowed by zorite; Important: Share stub had no window/cx); owner decision 19 (Tab indents, F6 leaves); re-review APPROVE | E-1–E-7 and E-6 first half applied. Escape also bound in `CommentComposer > Editor`. `SHARE` and the `ItemPane` slot fields carry plain `allow(dead_code)` (no test reads them); Lane B removes `SHARE`'s, Lane D the fields'. Lane C must not wrap the composer in `EditHost`. |
| 5a | done | `92296eca` | `8cdf23ee` | `submissions` 11, `worker` 37, `prompts` 9, `item` 380, `tabs` 58, `publication` 40; four lines clean | Part 0 whole review CHANGES REQUIRED (6 Important seams, 4 Minor); fixed; re-review APPROVE | Seams: `Attempt` stored per submission (resend, after_give_up); `Posted { draft, saved }` for the drain; operation-area retries go through `Submissions`; declined keys recorded per request with `clear_declined_for`; `ItemRoute` from the item tab to `Workspace::item_route` (`view/tabs.rs`, `[_; 8]`); `share_opened`/`share_progressed` hooks. Accepted risks: record state and `after_give_up` are read when the drain runs (safe: Operations and Submissions update in one drain step, nothing awaited between); `clear_declined_for` covers a request's last attempt (safe: a declined key ends with `unlock_required`, not `busy`). Ownership handovers in the design (`92296eca`). |
| 5 | done | `1ee62aa2` | — | Desktop bin 1151 passed at `8cdf23ee`; four lines clean; merge-tree with `origin/main` `0dfd5a68` clean | Fresh whole-Part-0 review (→ Task 5a), re-review APPROVE | Ticket branch pushed; lanes created from the checkpoint HEAD. |
| 8a seam | done | `f04b8d16` | `7a1c2466`, `9dd49e38` | `tabs` 60, `item` 380, `keys` 29; fact-41 tests pass unchanged; four lines clean | APPROVE (2 Minor, not applied) | Decision 20: `PublicationRemote` from navigation, pushed on every workspace notify (no loop; cheap). Decision 21: Ctrl K → `InsertLink` in `CommentComposer > Editor`. |
| O-1 seam | done | `c6f00ce7` | `bf2b8338` | `settings` 102; four lines clean | Orchestrator review (23 lines) | `SettingsHost::open_remotes_on(root, window, cx)` and `RemotesPage::open_on`; Lane B Task 10a switches Share…/Share main… to it (its marker says Task 9) and adds a two-repository test. |

### Task 0 record

- **Step 1.** `origin/main` `15a23125` already merged; the merge is a
  no-op. Gates (check all-features, fmt, clippy all-targets, check and
  clippy desktop, `check-cli-no-gpui.sh`) passed. Full suite
  (`cargo test --all-features --locked`): 4901 passed, 0 failed, 1 ignored,
  exit 0.
- **Step 2.** `git diff --stat 00bef3da HEAD` touches only the approved
  RFC and Wave amendments; no source fact moved. Amendments 1–6 applied
  (`5c26a5cd`, `3eb38cdb`).
- **Step 3 (read-only subagent; orchestrator spot-checked).**
  - U-1 MATCH: `item sync` with no editing context answers
    `item_not_found`, finished, no reservation (`remote/sync.rs:540-552`,
    `mutation/outcome.rs:203`); `publication_remote_required` with no
    remote; `busy` beside a pending merge (`bind/sync.rs:1108-1119`). Owner
    chose specific copy: decision 18, `NothingToShare`.
  - U-2 MATCH by reading: `internal_error`, written, committed,
    publication pending, `pending_reason: synchronization`, standing Owed
    (`bind/comment.rs:620-633, 678-705`; transport uninitialized in the
    bin, `transport/operation.rs:83-85`). Task 13 confirms by test.
  - U-3 MATCH: a finished record offers `request.retry` only for `busy`,
    `poll_yielding`, `remote_unavailable`, `transport_unavailable`,
    `registration_pending`, plus an unsettled record (`outcome.rs:646-653,
    843-850`).
  - U-5 MATCH with conditions: resume takes up an unfenced remote sync row
    and its token is honoured at safe points, freeing the reservation
    (`bind/resume.rs:313-327`, `bind/sync.rs:486-500`). Conditions for
    Task 10b: fenced, resolution-only and `item republish` rows answer
    `original_request_required` / `external_change`; a cancelled resume
    over an unsettled push answers `recovery_required`, not `cancelled`
    (`bind/sync.rs:927-949`), which `sync_outcome` must render; whether a
    safe point precedes the unlock prompt is unverified, so Task 10b adds a
    test that holds a resume, cancels it, fails the transport and checks
    the reservation.
- **Step 4.** Sweep hits match E-1–E-10, except `services/requests_tests.rs`
  (exhaustive intent lists and a "kind of its own" `JobKind` assertion).
  Owner: separate `JobKind::SynchronizeItem`/`SynchronizeRepo`, named edit
  E-12 (decision 17). E-5's wording corrected: the test asserts later rows
  `[SHARE, CLOSE_TICKET]`.

## Lane A (`d4/lane-a`)

Worktree `<main>/.worktrees/d4-lane-a`, from `a1faca4a`.

| Task | State | Base | Commits | Tests | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 6 | done | `a1faca4a` | `e9528d4a`, fixes `fd0368c8` | `drafts` 179 (twice); `drafts::comments` 13; four lines clean | Round 1 CHANGES REQUIRED (3 Important: retirement lost pending text, returned nothing, trusted a stale listing); fixed; re-review APPROVE | Ruling R-18. `ItemKey` defined in `services/drafts/comments.rs`. For Task 13's checkpoint: never adopt a draft whose submission is posting, and retry `retire_comment` on `HeldHere` once an adoption lands; R-18's open items. |
| 7 | done | `fd0368c8` | `3f10c120`, test fix `77ac73f2` | `drafts` 180; desktop bin 1165; four lines clean | APPROVE (1 Minor applied) | E-8, E-9 (two struct literals gained `open: false`, accepted as part of E-9). Open carries only the item; Lane D finds the draft through `comment_drafts`, including when the tab is already open. Known limit: a comment draft whose item is gone offers Open; the tab shows its error (Discard and Export still work). Library doc `src/drafts/compare.rs:342` is stale (follow-up ticket). |
| checkpoint | merged | `77ac73f2` | lane merge of the ticket branch `2d9bcbfe`; `--no-ff` into the ticket branch `c6f00ce7` | Lane: `drafts` 180, desktop bin 1167. Ticket branch after merge: desktop bin 1167, clippy clean | — | Lane worktree and branch removed. |

## Lane B (`d4/lane-b`)

Worktree `<main>/.worktrees/d4-lane-b`, from `a1faca4a`.

| Task | State | Base | Commits | Tests | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 8 | done | `a1faca4a` | WIP `0c966cbd`, merge `e69b3535` (seam `7a1c2466`, `9dd49e38`), `153ab866` | `share` 17, `header` 21, `keyboard` 41; desktop bin 1158; four lines clean | APPROVE for Lane B's scope (1 Important deferred to O-1, 3 Minor) | First stop: a remotes read on open broke fact 41's tests → decision 20 and E-13 (applied in both walks of the frame test). Share main with no remote reads "Share main…" and opens Remotes. O-1: the Remotes page keeps its previous repository (`settings/remotes.rs:283`); orchestrator seam, then Task 9 switches the call site and adds a two-repository test. Stale markers in `view/item.rs` (`ItemRoute`, `share` field) for Lane D. |
| 8 fix | done | `153ab866` | `1a20329d` | `share` 19, `sidebar` 2 | Orchestrator check | Tests for Share main… and the RegionEnd path. |
| 9 | done | `1a20329d` | `4680e638` | `share` 23, `operations` 33, `passphrase` 24; desktop bin 1166; four lines clean | APPROVE; 2 Important carried | R-14 in `model/operations.rs`. Carried to 10b: Stop is offered for a local-operation resume, which ignores the token; restrict Stop for `OperationResume` to remote sync and comment publication rows, and test a local resume reads Finishing. Ruling: `view/operations.rs` and `model/operations.rs` may split into child modules in Lane B's row. Stale `// Called from Task 9.` markers in `model/publication.rs` and Task 8 markers in `services/requests.rs`: orchestrator cleanup at Task 16. |
| 10a | in progress | `4680e638` | — | — | — | Takes the ticket branch (Lanes A, C, O-1); carries O-1/I-1, R-17's declined-key clear, decision 18. |

## Lane C (`d4/lane-c`)

Worktree `<main>/.worktrees/d4-lane-c`, from `a1faca4a`.

| Task | State | Base | Commits | Tests | Review | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 11a | done | `a1faca4a` | `9f014c56` | `composer` 7 (four runs), `editor` 90 (twice); four lines clean | APPROVE; 1 Important (math click lands before the block) carried into 11b; 4 Minor | Decision 21 (toolbar always shown; Ctrl K for Link, bound on the ticket branch). Hint copy "Ctrl Enter to post · Esc to cancel" per design. No `CaretInView` yet (Task 12). Task 12 removes the composer's dead-code markers. |
| 11b | done | `9f014c56` | `d0608ce4`, fixes `a0d03350`, `afa4ac3d` | `composer` 16 (twice), `editor` 90; desktop bin 1167; four lines clean | Round 1 CHANGES REQUIRED (Post as a new comment missing); round 2 CHANGES REQUIRED (discard prompt focus order); APPROVE after `afa4ac3d` (orchestrator check) | Rulings: discard prompt "Discard · Keep editing" with Keep editing focused on open; a Failed reply offering only Edit text closes via Edit text then Cancel, or F6; Cancel/Escape while Posting do nothing. Test-first order not evidenced for 11b. Helpers in `composer/guard.rs` mirror `view/editor/read_only.rs` (share after D4). Lane D sets Posting on Submit. |
| checkpoint | merged | `afa4ac3d` | lane merge `8c425e83`; `--no-ff` into the ticket branch `e66dda58` | Lane: `composer` 16, desktop bin 1184. Ticket branch after merge: desktop bin 1184, clippy clean | — | Lane worktree and branch removed. |

## Decisions during execution

- Lane task ticket comments are batched into each lane's checkpoint
  comment rather than one per task; the ledger carries each task's record.

- 2026-11-02, Task 0: decisions 17 (separate sync job kinds, E-12) and 18
  ("Nothing to share yet" copy).
- 2026-11-02, Task 3 review: orchestrator ruling R-17 (design), derived
  from R-12 and round 4's no-loop rule where the design was silent.
- 2026-11-02, Task 0: main moved to `0dfd5a68` (`AGENTS.md` tiered test
  gate); merged as `38b3c18f`, no conflict, no code change. The running
  Task 0 full suite is allowed to finish (owner). From then on the full
  suite is replaced by `AGENTS.md`'s fast tier; D4 touches only
  `src/desktop/`, so no SSH-tier target applies. Plan Task 16 updated.
