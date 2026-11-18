---
id: 01M7N0BQH1Z8GJBBY6NYK6
item_id: 01M7MKQFCHPEXQ58N77D63S417
created_at: 2026-11-17T15:13:16.448143197Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready, 2026-11-17, head 47d2dc96. A fresh whole-branch review found two Important issues, both fixed with a test that failed first.

**Fixed.** (1) A Git repository in a directory that is not ignored but has nothing Git reports (a fresh `git init notes`, or a clone with no checked-out files) was not seen by the nested-repository check and was pruned with the worktree. The check now also asks for untracked directories without recursing, so these come back as ignored and the walk finds the `.git`. Pinned by the row `untracked_empty_repository` in `cleanup_ignored_directories_holding_a_repository_or_unreadable_block`. (2) A directory that vanished during the walk (an editor or Cargo renaming build output) answered `context_changed`, bringing back the spurious `cleanup_pending`. A path that is gone is no longer a block; other read errors still are. Pinned by `a_directory_that_vanishes_during_the_walk_is_not_a_block`.

**Tests run after the fixes.** `cargo fmt --check` and clippy with `-D warnings`: clean. Library tests for `repository::cleanup::`, `promotion::`, `closure::`, `context_release` and `context_retirement::`: 573 passed. SSH: only the two end-to-end cases `c5_lifecycle_cleanup_ignored_files_close` and `_promote` (the change since the full SSH run is a status-option change and a walk helper in the shared precheck, so whole targets were not rerun). origin/main had not moved at the last fetch.

**Deferred minors.** The unreadable-directory test fails when run as root. The `.git` name is compared case-sensitively (macOS and Windows treat `.GIT` the same). The design doc overstates that a file appearing between the check and the prune still stops the removal (pre-existing; nothing sits between the refusal check and the prune). The resend of an older `ignored_files` row is pinned for promotion, not closure. A nested bare repository (no `.git` entry) in an ignored directory is pruned.

Not closed: `ticket close` waits for the owner's approval.
