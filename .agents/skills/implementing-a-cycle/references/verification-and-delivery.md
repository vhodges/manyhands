# Verification and delivery

## Local evidence

Read the rebased `AGENTS.md`. Run local/agent Rust commands through Devenv. Before
submitting Rust changes, require success from:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Use focused checks during each task. Run the full gates on the final amended
Rust tree; do not repeat them for unchanged code or ticket-only bookkeeping.
Commit lockfiles when their dependencies change. Preserve the library/desktop
boundary and GPUI toolchain rules in `AGENTS.md`.

Smoke-test the CLI with
`devenv shell -- cargo run --locked --bin manyhands-cli`.
When an active display is available, run
`devenv shell -- cargo run --locked --features desktop --bin manyhands` and verify
actual launch. Inspect display availability rather than assuming a headless host.
Stop only processes started for the smoke test. Record unavailable checks as
not run, with the reason.

For platform-sensitive changes, cover native path representation, permissions,
file-handle semantics, and platform-dependent C ABI types as relevant. Compare
paths using their intended representation; avoid mixed-separator fixture strings.
Credential tests must use isolated homes and fixed-message assertions that never
print secret buffers. Preserve production checks when fixing test assumptions.

## Review, push, and native CI

Obtain independent whole-branch review against the approved requirements.
Resolve findings and verify fixes; retain the evidence and any stated limits.

When push/PR is authorized, inspect remote changes and publish the reviewed
branch. Follow the starting skill's non-fast-forward procedure; do not infer
force-push authorization. Use a body file or structured argument for PR prose.
Describe the problem, resulting behavior, scope, verification, and remaining gates.

Run the relevant native CI matrix and inspect actual build and test results on
the intended commit. Use the native-Cargo exception only where repository
instructions authorize it; local commands stay in Devenv. Include the affected
library and integration tests, preserve existing platform/artifact coverage, and
do not disable failing tests to make CI green.

On CI failure, obtain the job logs and distinguish compilation, library tests,
integration tests, and infrastructure. An early failure can hide later failing
targets. Reproduce locally where meaningful, add or retain the covering regression,
fix the demonstrated cause, review and push, then inspect the new native run.
Continue until the authorized change passes or an actual blocker needs input.
Record exact commit/run links; never report pending jobs as passed.

## Approved merge and closure

Code review or green CI alone does not authorize merging. Once merge is approved,
add the requested approval/verification comment and close the ticket in the final
pre-merge update, following current `AGENTS.md`. Refresh stale test counts and
platform evidence. Push that update and merge the exact intended PR head using
normal repository rules; do not bypass required checks or use admin overrides.
Verify the remote PR is merged and record its merge commit. Preserve unrelated
edits in the main checkout; a remote merge does not require resetting local main.

## Separately authorized local cleanup

Merge approval alone does not request worktree deletion. When cleanup is requested,
inspect tracked, untracked, and relevant ignored files in the exact ticket worktree.
Preserve user data; remove disposable agent/build artifacts only within that scope.
Fetch main and verify the ticket tip is its ancestor. From outside the worktree,
remove it with `git worktree remove`, then use `git branch -d` for the local branch.
If removal refuses because of unique work, inspect and resolve preservation before
retrying; do not default to `--force`. Verify both removals. Leave other cycle
worktrees, the remote branch, and unrelated main-checkout edits alone unless
separately authorized.
