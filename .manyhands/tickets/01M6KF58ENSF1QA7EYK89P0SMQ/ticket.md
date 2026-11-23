---
slug: vh-k12ka
id: 01M6KF58ENSF1QA7EYK89P0SMQ
title: Expose save, commit, push and merge as separate verbs
type: exploration
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Expose the intermediate steps of the item workflow as their own verbs, in the CLI and the desktop, so that an agent or a person can do one step at a time: save, commit, push (publish) and merge are separate things a caller may want.

## Today

- **`ticket save` / `document save`:** write the file **and** commit a checkpoint in one step; there is no write without a commit, and no separate commit of already-written edits.
- **`item sync`:** fetches, merges the remote context **and** primary into the item's context, **and** publishes, in one step; there is no push-only or merge-only form, except `item republish`, which is push-only for a branch the remote deleted.
- **`context sync-local`:** merges local primary into the context, with no network.
- **`repo sync`:** synchronizes and publishes primary.
- **`document promote` / `ticket close`:** context synchronization, closure checkpoint, merge into primary, publication **and** cleanup in one confirmed operation (Wave 03 C5). There is no merge-into-primary without the lifecycle, and no publish-without-cleanup step.

## Also noted (owner, 2026-11-04)

The first real `ticket close` (`01M6KD8E25BWXQKF4REYM1DZMC`) was slow and gave little progress: human mode prints few stages while it works. Granular verbs would also let a caller see and retry each step. Speed itself is `01M6K04KGRHJC870JQK9XEKTXG`.

## Explore

- Which steps to expose: write-only save, commit, publish (push the item's context), merge of the context into primary, cleanup, each with its own request, consent where needed, and recovery.
- How they compose with the existing one-step verbs, which stay; and whether the lifecycle verbs become compositions of the steps.
- Consent and closure invariants: closure's checkpoint and `closed_by` rules (C5 decision 6), immutable closure fields, the consent digest (Wave 02 Cycle 09 D6), and the remote reservation and leases.
- Progress reporting for long operations, in human mode and the desktop.
- The desktop's equivalents, in its worker and lanes.

Outcome: an RFC proposal for the owner (CLI contract and desktop RFC), then Cycle planning.

Raised by the owner on 2026-11-04 after Wave 03 C5 (`01M4CC0VMXTNXNH78C11DYXS8H`).

## Missing step: record the closure

The steps above do not include recording the closure: writing `closed_at` and `closed_by` on the item's branch without merging. In a repository whose primary changes only through pull requests, it is the one step a ticket still needs. `item sync` already publishes the branch, the forge does the merge, and `poll once` and `context retire` do the cleanup. `document promote` writes no completion marker, so a document needs no equivalent.

Precedent: C4 and C5 were closed by a canonical file edit before merge, because `ticket close` "cannot precede a reviewed merge of its own branch" (C5 design, "Dogfooding (R-12, decision 9)").

### What already supports it

Read in source at `66a115c`; not run.

- **Close accepts a recorded closure.** The closure observation plans `Existing` when the context already holds both fields, and `pre_integration` then writes nothing.
- **The checkpoint is already a separate step,** but it runs inside the integration lease (`closure/checkpoint.rs`).
- **Poll sees the merge.** `completed_deleted_context` answers `closed` once local primary holds the closure and the remote branch is gone.
- **Retirement accepts a squash.** `preservation` returns `Ancestor` for a merge commit and `Folded` when primary holds the branch's exact changes.
- **Sync is not guarded.** No closed-ticket guard was found in `remote/sync.rs`, so a branch with a recorded closure should still publish.

### Problems if it is exposed as it stands

1. **Review stops.** `require_ticket_open` refuses when either the primary or the context tip holds a closure, so saves and comments answer `ticket_closed` while the pull request is open.
2. **A rejected pull request is stuck.** There is no reopen (`vh-86fqm`), and nothing withdraws a closure that was never integrated.
3. **Dependents become ready too early.** Readiness reads the effective copy, which is the item's own worktree while it exists (`effective_rows`).

### Proposal: provisional until primary holds it

- A closure only on the item's own branch is provisional. It is derived by comparing the context with primary, so the canonical schema does not change.
- Reads report it as its own state (for example `closing`), and readiness treats it as open.
- The guard refuses on primary's copy only. Comments stay allowed while the closure is provisional.
- A withdraw step removes a provisional closure. It is not a reopen, because nothing was integrated, so it need not wait for `vh-86fqm`.
- Recording stays a confirmed step whose preview names only its own effects: the closure fields, the commit and, if asked, publishing the branch.

### To decide

- Whether `ticket save` is allowed while provisional, or requires a withdraw first.
- What `closed_at` means: when it was recorded, or when it reached primary.
- Whether `closed_by` may differ from the person who merges the pull request.
- Recording moves the branch head. If approvals bind to a commit, record first, or let an approval survive a commit that changes only the closure fields.
- Who deletes the remote branch: the forge at merge, or Manyhands. `vh-ew8pt` covers contained branches; a squashed branch is not contained.
- Whether this is a verb, a mode of `ticket close`, or a repository setting. A setting would also cover `document promote`.

### Cases to pin

- Record, publish, merge by merge commit; then poll answers `closed` and `context retire` succeeds.
- The same with a squash merge. `ticket close` answers `ticket_closed` there (the closure is on primary and the context is not contained), so `context retire` is the only cleanup.
- The same in the clone that recorded the closure, when it has pushed the branch but never fetched it: push-only evidence does not establish deletion.
- A pull request declined, the closure withdrawn, then recorded again.
- A comment added while provisional.
- Two clones record the same closure.
- A dependent stays blocked until primary holds the closure.

### Related

`vh-86fqm` (reopen), `vh-0ry0a` (agent identities), `vh-c97se` (setup on a branch for a pull request), `vh-ew8pt` (stale remote branch).

Left out on purpose
- Approval records, meaning what an approval binds to and whether close checks for one. They deserve their own ticket, and I found no open one.
- A PR mode for close and promote. vh-c97se calls it "a separate decision", and I found no ticket for that either. The last "To decide" item touches on it.
