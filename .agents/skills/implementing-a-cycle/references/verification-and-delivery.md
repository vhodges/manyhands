# Verification and delivery

## Local evidence

Read current main's `AGENTS.md`. Run local/agent Rust commands through Devenv. Before
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

## Review and push

Obtain independent whole-branch review against the approved requirements.
Resolve findings and verify fixes; retain the evidence and any stated limits.

A Cycle is review-ready and mergeable on its local Devenv gates once
`git merge-tree --write-tree HEAD origin/main` shows that it merges cleanly.
Native CI is not a gate before merge.

When push/PR is authorized:

- Inspect remote changes, then publish the reviewed branch.
- If the push is not a fast-forward, follow the publishing section of
  [the preflight](worktree-preflight.md). Do not infer force-push
  authorization.
- Write PR prose to a body file or pass it as a structured argument.
- Describe the problem, the resulting behavior, the scope and the
  verification, and say that native CI runs after merge.

## Native CI, after merge

Native CI runs after merge, on main (owner decision, 2026-10-17). The Build
workflow runs only when dispatched by hand. Each dispatch needs the owner's
approval; approval for one run does not carry over to the next. Dispatch it
with `gh workflow run build.yml --ref <branch>`. A full run covers four
targets and can take more than two hours.

The workflow's native runners may run Cargo directly; local commands stay in
Devenv. A new integration test target runs on the native targets only once it
is in the workflow's test list. Inspect the actual build and test results on
the intended commit. Record exact commit and run links. Never report pending
jobs as passed.

A native failure found after merge is follow-on work in a separate session.
Do not hold or reopen the merged Cycle for it. In that session:

- Obtain the job logs. Tell compilation, library-test, integration-test and
  infrastructure failures apart. An early failure can hide later failing
  targets.
- Follow AGENTS.md's platform-failure steps:
  1. Narrow the matrix to the failing platform, in a commit of its own.
  2. Fix the demonstrated cause, and add or keep a covering regression test.
  3. Run only the affected tests locally, through Devenv.
  4. Push and dispatch again.
  5. Once that run is green, restore the full matrix in a commit of its own
     and run it once more.
- Preserve existing platform and artifact coverage. Do not disable failing
  tests to make CI green.

## Approved merge and closure

Code review alone does not authorize merging. Once merge is approved, add the
requested approval/verification comment and close the ticket in the final
pre-merge update, following current `AGENTS.md`. Refresh stale test counts.
Push that update and merge the exact intended PR head using
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
