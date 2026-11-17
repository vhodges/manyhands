---
id: 01M7MZR2WSK673PWXSK9AQF2W2
item_id: 01M7MKQFCHPEXQ58N77D63S417
created_at: 2026-11-17T14:49:31.808322226Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Implementation complete, 2026-11-17. The shared precheck `require_cleanup_clean` no longer reports ignored files, so ticket closure, document promotion, `context release` and `context retire` all clean up a worktree whose only extras are ignored files. Design and plan: `docs/plans/2026-11-17-ignored-files-cleanup-design.md` and `-implementation.md`.

**Behavior.** Untracked (not ignored), modified, staged, skip-worktree, assume-unchanged, submodule changes, locks and foreign registrations still block. A tracked file that matches an ignore pattern is reported modified by Git and still blocks. An ignored directory that holds a `.git` entry (a nested repository) or cannot be read also still blocks (owner decision). The stored `ignored_files` category stays readable; nothing produces it. A close an older build left at `ignored_files` finishes on resend (pinned by a test). `worktree.prune(working_tree)` deleting ignored files was checked with a probe on git2 0.20.4.

**Tests run, on the final tree (head c4bc2281).**
- Static gates across the whole tree: `cargo check --all-features --locked`, `cargo fmt --check`, `cargo clippy --all-targets --all-features --locked -- -D warnings`: clean.
- Fast tier: `--lib` 1,571 passed and 1 failed (a context_release test that pinned the old behavior, fixed); integration and bin targets with `--no-fail-fast`: 4,499 passed and 1 failed (a mutation_bridges test that pinned the old behavior, fixed). Both fixed tests rerun by name and pass.
- SSH tier by changed path: document_promotion and ticket_closure (exit 0), remote_polling 86 cases, mutation_remote 808 cases, `--lib repository::context_retirement::` 142 passed. All passed.
- One SSH case per verb for the end-to-end close and promote with an ignored file: `c5_lifecycle_cleanup_ignored_files_close` and `_promote`. Everything else is a library test in `cleanup_tests.rs`, `run_tests.rs` and `context_release_tests.rs`.

**Next.** A fresh whole-branch review, then merge with origin/main (no commits had landed at the last fetch; `git merge-tree` clean). Not closed: `ticket close` waits for review approval.
