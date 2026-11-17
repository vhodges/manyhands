---
id: 01M7NFQ1F0SWTATKE8FY6Z6X49
item_id: 01M7MYETX7H5YYAYJJBP85FVYP
created_at: 2026-11-17T19:28:35.818895881Z
created_by: Claude Sonnet 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---
## Review-ready: distributable user skills

Eleven Agent Skills for using Manyhands are on this branch under `skills/`: overview, getting-started, keys-and-hosts, repositories, finding-work, documents, tickets, comments, sharing-and-sync, polling-and-index and recovery. `skills/README.md` is the catalog and install guide, the main README has a short pointer, release archives now carry `skills/` (and the macOS app's Resources), and the Windows installer treats `skills` as a managed root. No product code changed: nothing under `src/`, `Cargo.toml`, `Cargo.lock` or `schemas/`. The Rust change is test-only (`tests/cli/skills.rs`, a `mod` line, and two helpers made `pub(crate)` in `tests/cli/inventory.rs`).

Design: `docs/plans/2026-11-17-user-skills-design.md`. Plan: `docs/plans/2026-11-17-user-skills-implementation.md`. `origin/main` was merged in (last at 39371676, which brought in the ignored-files cleanup change), and the skills were reconciled with it.

## Tests run

- At b653eb8e: `cargo check --all-features --locked`, `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings` exit 0; the fast tier from AGENTS.md (with `--skip repository::local_sync::`) exit 0 (library 1574 passed; the `cli` target includes all the `skills::` tests).
- At 037e76a8 (after the final fix wave): check, fmt and clippy exit 0; the whole `cli` target exit 0 (485 passed); the release Python suite `python3 -B -m unittest discover -s .` in `scripts/release/tests` exit 0 (272 tests OK, 12 skipped for the Windows and Inno cases).
- SSH tier: none required (nothing under `src/repository/` or `tests/support/ssh_*` changed).
- Behavior evidence: each skill was tested with fresh agents on scratch repos (with a no-skill baseline for the tricky ones), and every skill claim about the CLI was re-run by a reviewer. A final whole-branch review (no Critical findings) and one fix wave with one scoped re-review followed.

## Windows verification route (please read)

The Pascal and Inno edits (`packaging/windows/installer-managed-policy.pas`, `installer-managed.iss`, and their test cases and lifecycle fixture) were checked by reading only: there is no `fpc`, `iscc` or `wine` here. They first run in the Release workflow's Windows job (`test_windows_installer.ps1`, `installer_policy.iss`, and the compile of `manyhands.iss`). The Build workflow runs only cargo, so it does not exercise them, and the Inno lifecycle fixtures (`test_installer_engine.py`) run in no CI job unless an Inno compiler is configured. I recommend running the Release workflow's Windows job, or the engine fixtures on a Windows machine, before relying on this installer change. I did not dispatch any workflow.

## Before `ticket close`

On a build that includes the ignored-files change, `ticket close` deletes the ticket worktree's ignored files with it, without listing them. This worktree holds ignored `.superpowers/` (the execution ledger and reports), `target/` and `.devenv/`. Copy out anything you want to keep first.