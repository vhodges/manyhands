---
slug: vh-gvzsr
id: 01M7MKQFCHPEXQ58N77D63S417
title: Ticket close and its cleanup should not block on ignored files
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
`ticket close`, and the cleanup it runs after the merge, stop on files that Git ignores in the ticket's worktree. Ignored files are build output and editor caches, not the user's work, so they should not block. Untracked and modified files must keep blocking, as they do now.

Found 2026-11-09, closing `01M70PYS3B3YWEFTY0QBJGCF6N` (`target/` and `.devenv/`, created by running Cargo inside the ticket worktree), and seen again 2026-11-15, closing `01M7D5Z7FZTGTC1HE25KQ1MYQY`. Each time the close merged and published main, then answered `partial` / `recovery_required` with `cleanup_pending: ignored_files`. The owner had to remove `target/` or `.devenv/` by hand and resend the same request. An editor with the worktree open (rust-analyzer's `target/flycheck0`) recreated `target/` within minutes, so the removal had to be in the same command as the retry.

## Where it happens

- `require_cleanup_clean` in `src/repository/cleanup.rs` asks libgit2 for statuses with `include_ignored(true)` and `recurse_ignored_dirs(true)`. If any entry is `IGNORED` it returns `Precheck::IgnoredFiles`; any other entry returns `Precheck::Changed`.
- `Precheck::IgnoredFiles` becomes `Clean::IgnoredFiles` in `src/repository/promotion/run.rs` (lines near 1263, 1343, 1436) and `PromotionCleanupPending::IgnoredFiles`, and the closure side records the same category. `ignored_files` is also a value of the `cleanup_category` CHECK constraint in `closure/record.rs` and `promotion/record.rs`, and is mapped in `mutation/dto.rs`.
- `ReleaseCheck::NotClean` and `context_release.rs` (`not_clean`: "holds a change, an untracked file or an ignored file") use the same precheck, as does `context retire`.
- The removal itself is `worktree.prune` with `working_tree(true)` in `remove_worktree`. Whether libgit2's prune removes ignored files without trouble is not verified: confirm it before relying on it.

## Expected

- A worktree whose only extras are ignored files does not block the cleanup. The ignored files are deleted with the worktree.
- An untracked file (not ignored), a modified or staged tracked file, a hidden index bit (assume-unchanged, skip-worktree), a submodule change, a lock and a foreign worktree registration still block, with today's answers.
- The decision is made from Git's own ignore rules (`.gitignore`, `.git/info/exclude`, global excludes) as the worktree sees them. A file that is both tracked and matches an ignore pattern is tracked, and its changes still block.

## To settle in planning

- **Scope.** The precheck is shared by ticket closure, document promotion, context release and context retirement. Closure is what the owner asked for. Decide whether the others follow (the least surprise is that they all do, since one precheck serves them) and say so in the design.
- **Submodules.** `require_cleanup_clean` recurses into submodules. Decide whether ignored files inside a submodule are also disposable. The submodule's Git metadata must still survive the prune, as today.
- **The `ignored_files` category.** Nothing should produce it any more. Keep the value readable for rows already stored, and keep the CHECK constraints valid, so no registry migration is needed (registry migrations are one-way, so avoid one for this). Remove the desktop and CLI copy that tells the user to remove ignored files.
- **Race with an editor.** An editor can recreate ignored files between the check and the prune. With ignored files no longer blocking this goes away, but a file that is untracked and not ignored appearing in that window must still stop the removal.
- **Data loss.** Ignored files can hold things a user wants (a local `.env`). The owner has decided build and tool output should not block. Say in the plan how the user is told which ignored paths were deleted, if at all.

## Acceptance

- A closed ticket whose worktree holds `target/` and `.devenv/` (ignored) closes, merges, publishes and removes its worktree and branch in one run, with no `cleanup_pending`.
- Rows that must still block: an untracked non-ignored file, an unstaged and a staged change, a skip-worktree entry, and a tracked file matching an ignore pattern with a change. Each answers as it does today.
- A close that was left `cleanup_pending: ignored_files` by an older build retries to completion.
- Library unit tests on a local repository with a `.gitignore`, not SSH-harness cases (AGENTS.md Testing Strategy). At most one SSH case, an end-to-end check of the close.
