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

- Run all Rust commands through Devenv: `devenv shell -- cargo <command>`.
- Before submitting Rust changes, run:
  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  ```
- Smoke-test the CLI skeleton with `devenv shell -- cargo run --locked --bin manyhands-cli`; it opens no window.
- Smoke-test the desktop app with `devenv shell -- cargo run --locked --features desktop --bin manyhands`; it requires an active desktop display.

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

Since each the branch and worktree will be pre-existing from some branch point in the last, before starting, rebase main onto the ticket branch before starting any work, planning or implementation.

## Linux Toolchain

- Keep `devenv.nix`'s GPUI runtime library list and `env.LD_LIBRARY_PATH` in sync. GPUI dynamically loads Wayland; removing the loader path causes `NoWaylandLib` at startup even when the Nix packages are installed.
- Commit `Cargo.lock` after Cargo dependency changes and `devenv.lock` after Devenv input changes.
