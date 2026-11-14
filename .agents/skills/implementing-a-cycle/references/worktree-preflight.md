# Worktree and merge preflight

Read this before new planning or implementation work in a ticket worktree.
The direction is **current main merged into the ticket branch**. Ticket
branches are pushed, so never rebase one: a rebase rewrites published history.
Never rebase main onto a ticket branch either.

## Inspect before changing anything

1. Locate the ticket's registered worktree and branch with
   `git worktree list --porcelain`. Confirm that the worktree belongs to the
   requested ticket.
2. Check the status of both the main checkout and the ticket worktree. Leave
   unrelated edits in the main checkout alone; they don't stop you using a
   clean ticket worktree.
3. If the ticket worktree is dirty, find out what the changes are before you
   merge.
   - Preserve user edits. Never use stash, reset, clean or checkout to erase
     them.
   - You may checkpoint agent-owned work in its own commit first.
4. If a merge is already in progress, finish it or abort it safely before
   starting another. Do not discard unresolved conflicts.
5. An old worktree carries an old `AGENTS.md`. After fetching, read the
   current rules with `git show origin/main:AGENTS.md`.

## Fetch, merge, verify

Merge from `origin/main`, not local main. If local main has unpushed commits,
say so rather than folding them in silently. Run the equivalent of the
following, with inspected paths, and check each result before continuing:

```sh
git -C "$ticket_worktree" fetch origin
base=$(git -C "$ticket_worktree" rev-parse origin/main)
before=$(git -C "$ticket_worktree" rev-parse HEAD)
git -C "$ticket_worktree" merge origin/main
git -C "$ticket_worktree" merge-base --is-ancestor "$base" HEAD
git -C "$ticket_worktree" status --short
git -C "$ticket_worktree" rev-parse HEAD
```

`ticket_worktree` is the verified ticket worktree, not the main checkout. If
the branch is already up to date and nothing merges, that is fine: the fetch
and the ancestry check show the gate was met.

- **Conflicts:** resolve routine conflicts by following the approved intent.
  If a conflict changes a product decision, or it is unclear who owns the
  conflicting changes, keep the state and ask the focused question.
- **Fetch failures:** diagnose and retry. Do not silently substitute a stale
  ref.

Record the base SHA, the HEAD before and after the merge, and any conflict
resolutions, in the ticket checkpoint and the execution ledger. A merge keeps
existing commit IDs, so task checkpoints recorded earlier stay valid.

## When to repeat

Repeat at the start of each phase or session, and whenever main moves during
the Cycle. Merge once at that point, not before each subagent or checkpoint
commit.

If upstream changes alter the plan's assumptions, reassess the plan and the
affected tests. An unchanged plan does not need approval again. Record
existing failures separately from new regressions.

Before calling the work review-ready, run
`git merge-tree --write-tree HEAD origin/main`. If it reports conflicts, merge
main again.

## Publishing

A ticket branch only moves forward, so pushing it is normally a fast-forward.
If a push is refused because the remote ticket branch moved:

1. Fetch the remote ticket branch.
2. Inspect what it holds.
3. Merge it.

Push/PR authorization does **not** authorize a force push, even with a lease.
A force push needs explicit authorization, bound to the observed remote SHA.
