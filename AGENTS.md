# Manyhands

## Structure

- This is one root Cargo package, not a workspace. It declares two binaries:
  `manyhands` is the desktop entry point in `src/main.rs` and requires the
  `desktop` feature; `manyhands-cli` is the headless entry point in
  `src/bin/manyhands-cli.rs`.
- Put domain logic shared by both front ends in `src/lib.rs`. It must not
  depend on GPUI or GPUI Kit; those dependencies belong only to the desktop
  binary and desktop-only modules.
- Desktop code depends directly on `gpui-kit` only. It supplies the compatible GPUI API through `gpui_kit::*`; a separate direct `gpui` dependency is forbidden.
- Initialize GPUI Kit with `gpui_kit::init(cx)` inside `app.run` before using components, and make `Root` the first view created for each window.

## Development

- Run local and agent Rust commands through Devenv: `devenv shell -- cargo <command>`.
  Existing GitHub Actions native runners may run Cargo directly for builds and
  tests (user-approved exception, 2026-10-05).
- Before submitting Rust changes, run the static gates across the whole tree:
  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```
- Then run the local test gate (owner decision, 2026-11-02). The full suite
  takes over two hours, and about 1 h 36 min of that is the SSH-harness
  targets, so a change runs the fast tier always and the SSH tier only where
  it touches that code. The full `cargo test` suite and native CI run after
  merge. This is a stopgap until a coverage-selected subset (70-80% of the
  full suite's coverage in 20 minutes or less) replaces it.
  1. **Fast tier, every change.** Every target except the SSH-harness ones,
     with the slow `repository::local_sync` unit tests skipped:
     ```sh
     devenv shell -- cargo test --all-features --locked --lib --bins \
       --test canonical_foundation --test cli --test discovery_rebuild \
       --test drafts --test journal_rejection --test key_material \
       --test key_storage --test local_authoring --test mutation_bridges \
       --test mutation_confirmation --test mutation_contract \
       --test mutation_relationships --test mutation_replay \
       --test read_boundary --test read_characterization --test read_comments \
       --test read_contract --test read_credentials --test read_items \
       --test read_relationships --test read_repository --test read_status \
       --test recovery_foundation_gate --test remote_reservation \
       --test repository_enablement --test session_credentials \
       --test shared_key_registry -- --skip repository::local_sync::
     ```
     Add a new integration test target to this list, or to the SSH tier
     below if it uses `tests/support/ssh_harness.rs`.
  2. **SSH tier, by changed path.** Run each target whose path the change
     touches, through Devenv as `cargo test --all-features --locked --test
     <target>`:

     | Changed path | Also run |
     | --- | --- |
     | `src/repository/transport/`, `src/repository/keys/`, `tests/support/ssh_*` | every SSH-harness target |
     | `src/repository/remote/` | `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote` |
     | `src/repository/local_sync/` | `--lib repository::local_sync::`, `remote_synchronization`, `remote_merge_recovery` |
     | `src/repository/mutation/` | `mutation_remote` |
     | `src/repository/promotion/` | `document_promotion` |
     | `src/repository/closure/` | `ticket_closure` |
     | `src/repository/comment_publication*` | `comment_publication` |
     | CLI remote commands in `src/bin/manyhands-cli/` | `cli_remote` |
     | a target's own files under `tests/` | that target |

     When unsure whether a change reaches an SSH target, run it.
  3. **Desktop-only and drafts changes** (`src/desktop/`, `src/drafts/`,
     `src/main.rs`) need only the fast tier.

  A ticket worked with the `working-on-a-ticket` skill may narrow the fast
  tier further to the tests relevant to its change (owner decision,
  2026-10-24); the static gates still run across the whole tree.
- Smoke-test the CLI skeleton with `devenv shell -- cargo run --locked --bin manyhands-cli`; it opens no window.
- Smoke-test the desktop app with `devenv shell -- cargo run --locked --features desktop --bin manyhands`; it requires an active desktop display.

## Native CI

- The Build workflow (`.github/workflows/build.yml`) runs only when dispatched
  by hand. Ask the product owner before dispatching it; approval for one run
  does not carry over to the next piece of work.
- Native CI runs **after merge, on main**, not as a gate before merge (owner
  decision, 2026-10-17: most native failures were harness-related, and the
  wait of two hours or more blocked the next Cycle). A Cycle is review-ready
  and mergeable on its local Devenv gates. A native failure found after merge
  is fixed as follow-on work in a separate session, not by holding the Cycle
  that was merged.
- Dispatch it with `gh workflow run build.yml --ref <branch>`. A full run
  covers four targets (Linux x86_64 and aarch64, Windows x86_64, macOS
  aarch64) and can take more than two hours; Windows is the slowest.
- When a platform fails:
  1. Narrow the workflow matrix to the failing platform only, in a commit of
     its own.
  2. Fix the failure.
  3. Run the affected tests locally through Devenv, only those, not the whole
     suite.
  4. Push and dispatch the workflow again.
  5. When the narrowed run is green, restore the matrix to all four targets in
     a commit of its own and run the full set once more, so that a fix for one
     platform does not break another.
- A new integration test target is not run on the native targets until it is
  added to the test list in the workflow.

## Dogfooding Workflow

- For every Cycle in a Wave, create one Manyhands ticket before creating its
  implementation plan. The ticket is the record for the complete Cycle
  lifecycle: planning, implementation, verification, review, and closure.
- After C3 is merged, create and update tickets with `manyhands-cli ticket create`
  and `ticket save` against the registered main checkout
  `/home/vhodges/work/src/manyhands`. Run from an up-to-date main with
  `devenv shell -- cargo run --locked --bin manyhands-cli -- …`; this ticket
  branch's implementation does not make C3 available on main before merge.
- Before `ticket save` of your own Cycle ticket, commit your owned edits to its
  `ticket.md`: the CLI save checkpoints into that ticket's active worktree.
- Comments remain canonical `.manyhands/comments/<item-id>/<comment-id>.md`
  edits until C4's `comment add`. Close a ticket with `manyhands-cli ticket close
  --repo /home/vhodges/work/src/manyhands --id <ID>` from the registered main
  checkout, after review approval: it records `closed_at`/`closed_by`, merges the
  ticket's branch into main, publishes main, and removes the branch and worktree,
  so it replaces the separate merge and cleanup. With a passphrase-protected key it
  runs at the owner's terminal, since a non-interactive run answers
  `unlock_required` (switched 2026-11-04 after the first real close,
  `01M6KD8E25BWXQKF4REYM1DZMC`; C5 decision 9). Saving status text `closed` is not
  lifecycle closure.
- Exception: `working-on-a-ticket` status edits and closure keep that skill's
  own procedure until it is revised. Its loop runs in sandbox clones without
  the main checkout's registration; C3 does not change that skill.
- Perform all implementation work for a Cycle in the ticket's worktree and on
  its branch. Exception (owner decision, 2026-11-02, D4 only): D4's Lanes A,
  B and C may run in local lane worktrees branched from the ticket branch and
  merged back into it, never rebased, before review-ready.
- Add Manyhands comments to the ticket at implementation-plan checkpoints to
  record progress, decisions, blockers, verification results, and review-ready
  status.
- Close the ticket only after the associated change receives code-review or PR
  approval, normally in the final push before merge.

## Working on a ticket

A ticket's branch and worktree already exist, created from some earlier point
on main, and other work may have landed since. Before starting any planning or
implementation, fetch the latest main and **merge** it into the ticket branch
from inside the ticket worktree (`git merge origin/main`). Ticket branches are
pushed, so never rebase them: a rebase rewrites published history. Never rebase
main onto the ticket branch either. Repeat the fetch and merge whenever main
moves during the Cycle, and check `git merge-tree --write-tree HEAD
origin/main` before calling the work review-ready.

## Linux Toolchain

- Keep `devenv.nix`'s GPUI runtime library list and `env.LD_LIBRARY_PATH` in sync. GPUI dynamically loads Wayland; removing the loader path causes `NoWaylandLib` at startup even when the Nix packages are installed.
- Commit `Cargo.lock` after Cargo dependency changes and `devenv.lock` after Devenv input changes.
