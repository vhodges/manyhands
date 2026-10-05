# Required worktree and rebase preflight

Read this before new planning or implementation work. The direction is
**ticket branch onto current main**, not main onto the ticket branch.

## Inspect before mutation

1. Locate the repository root, requested ticket, registered worktree, and branch.
   Inspect `git worktree list --porcelain`, status, and the checked-out branch.
   Confirm the worktree really belongs to the requested ticket.
2. Inspect status in both the main checkout and ticket worktree. Unrelated main
   edits do not prevent using a clean ticket worktree. Leave them untouched.
3. If the ticket worktree is dirty, identify the changes before rebasing. Preserve
   user edits; never use automatic stash, reset, clean, or checkout to erase them.
   Agent-owned work may be checkpointed coherently. Ask only when safe preservation
   or the intended base cannot be determined from existing authorization.
4. If a rebase/merge is already in progress, inspect and finish or safely unwind
   that operation before starting another. Do not discard unresolved conflicts.

## Fetch, rebase, verify

Use the configured authoritative main remote (normally `origin`). Refresh it
even when local main or cached `origin/main` appears recent. Do not update or
switch the user's main checkout merely to refresh this base. If local main has
unpublished/divergent commits, inspect them and resolve the intended base before
silently including or discarding them.

After confirming a clean ticket worktree, run the equivalent of the following
with inspected paths and refs, checking each result before continuing:

```sh
git fetch origin main
cycle_base=$(git rev-parse origin/main)
cycle_before=$(git -C "$cycle_worktree" rev-parse HEAD)
git -C "$cycle_worktree" rebase "$cycle_base"
git -C "$cycle_worktree" merge-base --is-ancestor "$cycle_base" HEAD
git -C "$cycle_worktree" status --short
git -C "$cycle_worktree" rev-parse HEAD
```

`cycle_worktree` is the verified existing ticket worktree, not the main checkout.
This sequence never rebases main. A no-op rebase is acceptable; the fresh fetch
and successful ancestry check establish that the gate was met.

Resolve routine conflicts using the approved intent. If the conflict changes a
product decision or leaves ownership of changes unclear, preserve state and ask
the focused question. Do not start new Cycle edits while the rebase is incomplete.
If fetching fails, diagnose/retry with the required environment access; do not
quietly substitute a stale ref. An explicitly authorized offline base must be
recorded as such.

Record base, old/new ticket HEAD, conflict decisions, and verification in the
ticket checkpoint and execution ledger. A rebase changes commit IDs: map existing
task checkpoints to the rebased commits instead of treating completed tasks as
missing or replaying implementation.

## When to repeat

Repeat before implementation when it is a later phase/session, and on resuming
after main may have moved. Rebase once at that work boundary, not before each
subagent or checkpoint commit. Reassess approved plans and affected tests when
upstream changes alter their assumptions; do not request approval again for an
unchanged plan. Record existing failures separately from new regressions.

## Publication consequence

Rebasing a published ticket branch can make a later push non-fast-forward.
Fetch the ticket ref and compare both histories before deciding how to publish.
Push/PR authorization does **not** authorize a force push, even with a lease.

Preserve remote work through a reviewed merge when appropriate. If the remote
contains only an equivalent original checkpoint, a history-only reconciliation
may preserve it; verify the resulting file tree equals the reviewed tree.
Otherwise incorporate the actual remote changes and verify the affected work.
Use force-with-lease only with explicit authorization, bound to the observed
remote SHA. Never overwrite an unexpectedly moved remote ref.
