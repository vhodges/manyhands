# Ignored files must not block a context's cleanup

Ticket `01M7MKQFCHPEXQ58N77D63S417` (vh-gvzsr). Status: approved by the owner in conversation, 2026-11-17.

## Intent

`ticket close` merges and publishes main, then removes the ticket's worktree and branch. When the worktree holds files that Git ignores (`target/`, `.devenv/`, editor caches), the removal stops with `cleanup_pending: ignored_files` after the merge has already happened, and the owner has to delete the files by hand and resend the request. Ignored files are build output, not work. They should not block. Untracked files that are not ignored, and changes to tracked files, keep blocking: those may be the user's work.

## Decisions (owner)

- The change is made once, in the shared precheck, so ticket closure, document promotion, `context release` and `context retire` all follow.
- Untracked (not ignored), modified, staged, skip-worktree and assume-unchanged entries, submodule changes, locks and foreign worktree registrations still block, with today's answers.

## Goals

- A worktree whose only extras are ignored files is cleaned up in the same run as the close, with no `cleanup_pending`.
- A close that an older build left at `cleanup_pending: ignored_files` finishes when the same request is retried.

## Non-goals and anti-goals

- No new listing of the deleted ignored paths, and no prompt before deleting them.
- No registry migration. Registry migrations are one-way.
- Nothing that lets an untracked, non-ignored file, or a Git repository nested in an ignored directory, be deleted. If such a file appears between the check and the prune, the removal still stops.

## Facts checked

- `require_cleanup_clean` (`src/repository/cleanup.rs`) is the only place that asks libgit2 for ignored entries (`include_ignored(true)`, `recurse_ignored_dirs(true)`) and turns one into `Precheck::IgnoredFiles`. All of closure, promotion, release and retirement reach it through `precheck_context` or `precheck_release`.
- `worktree.prune` with `valid(true).working_tree(true).locked(false)`, the options `remove_worktree` uses, removes ignored files with the worktree (checked 2026-11-17 with `git2` 0.20.4 on a worktree holding a nested `target/`, `.devenv/`, a top-level ignored file, a dangling symlink and a read-only file). So the removal needs nothing done first.
- `ignored_files` is a value of the `cleanup_category` CHECK constraints in `closure/record.rs` and `promotion/record.rs`, is mapped in `mutation/dto.rs`, and is shown by the desktop (`lifecycle_lines.rs`, `lifecycle_lock.rs`).
- A tracked file that matches an ignore pattern is reported by Git as modified, not ignored, so its changes keep blocking with no extra code.

## Approaches

1. **Drop the ignored check in the shared precheck (chosen).** One function changes. Ignored entries no longer appear in the status the precheck inspects, and the dead `IgnoredFiles` branches go. Trade-off: every user of the precheck changes together, which is what the owner asked for.
2. **A mode on the precheck, so only closure skips ignored files.** Rejected by the owner: promotion, release and retirement should behave the same, and a mode adds a parameter for no reader.
3. **Delete ignored files before the precheck.** Rejected: it adds a second deletion step before the guard, and it fails the owner's rule that the guard decides what is safe to delete.

## Design

- `require_cleanup_clean`: ask for untracked entries only. Any entry in the result is `Precheck::Changed`. This applies to the submodule recursion too, so ignored files inside a submodule are also disposable, and the submodule's Git metadata must still survive the prune as today.
- Remove `Precheck::IgnoredFiles` and the code that returns or maps it: `Clean::IgnoredFiles` and its three uses in `promotion/run.rs`, `PromotionCleanupPending::IgnoredFiles`, and the matching desktop and DTO copy.
- Keep the stored string `ignored_files` readable so existing rows and the CHECK constraints stay valid, but nothing produces it. A row already at `ignored_files` is treated like any other retryable cleanup category, so resending the request finishes it.
- Reword `context_release.rs`'s `not_clean` message: it no longer mentions ignored files.

- An ignored directory that holds a `.git` entry (a nested repository, which may hold unpushed commits), or that cannot be read, still blocks as `Precheck::Changed` (owner decision, 2026-11-17). The check walks ignored directories without following symlinks.

## Tests

- A table-driven library test on a local repository with a `.gitignore`, in `src/repository/cleanup_tests.rs`: ignored-only extras pass; each of an untracked file, an unstaged change, a staged change, a skip-worktree entry, an assume-unchanged entry, and a modified tracked file that matches an ignore pattern is refused as `Changed`; an ignored file inside a clean submodule passes.
- A test that a closure row stored as `ignored_files` resumes to completion.
- Update the existing tests that expect `IgnoredFiles` (`cleanup_tests.rs` and the lifecycle tests that name it).
- At most one SSH-harness case, an end-to-end `ticket close` of a worktree holding `target/`; the transport is not what changed, so everything else is a unit test (AGENTS.md Testing Strategy).

## Left to the builder

- The exact retry handling of a stored `ignored_files` row, once the code that produced it is found.
- Which existing tests name `IgnoredFiles` and so change.
