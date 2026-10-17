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
- Before submitting Rust changes, run:
  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  ```
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
- Until the CLI supports the required operation, create and update tickets and
  comments directly in the canonical `.manyhands/` filesystem paths. Switch to
  the Manyhands CLI as soon as it supports the operation.
- Perform all implementation work for a Cycle in the ticket's worktree and on
  its branch.
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
