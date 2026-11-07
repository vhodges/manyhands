---
title: "Missing Repository Registration Removal Execution"
date: 2026-10-19
status: complete
author: "OpenCode"
manyhands_managed: true
manyhands_kind: document
id: "01M5ANNZ72JQXN18155HJ43CJW"
---

# Missing Repository Registration Removal Execution

Plan: [approved implementation plan](2026-10-19-missing-repository-registration-removal-implementation.md).
Ticket: `01M4R19P943QZKHHKRXV2D2TP2`.

Owner approved the plan on 2026-10-19, including the recovery exclusions and
documented existing registration-stamp limitation. Execute inline and sequentially,
with a review checkpoint after Tasks 0–2. No commit/push/PR/merge authorization
has been given.

## Task ledger

| Task | State | Evidence |
| --- | --- | --- |
| 0 Baseline | Complete | Fresh fetch/merge and focused baseline below |
| 1 Domain removal | Complete | Missing-root/.git regression tests went red then green; 8 focused enablement tests pass |
| 2 Coordination/recovery | Complete | 16 domain unit tests, 4 recovery integration tests, 3 cascade tests pass |
| 3 Boundary scope/confirmation | Complete | Four new regressions went red then green; 106 mutation unit tests pass |
| 4 Replay | Complete | 308 replay tests pass; disappearance, re-entry, consent and mismatch cases covered |
| 5 Contracts/docs | Complete | 44 mutation contract, 91 read contract/boundary, 23 removal-boundary and 20 domain tests pass; RFC/audit updates recorded |
| 6 Final gates | Complete | All four required Devenv gates passed; independent review finding fixed and re-reviewed; CLI/desktop startup smokes completed |

## Task 0 — 2026-10-19T18:09:48Z

Existing ticket worktree and branch reused. Fetched `origin/main` at
`021f68425db6d4be38bcfbee0ee151829e06df41`; merge was already up to date.
Before/after HEAD: `cdd797cba21878eecb83beed6023c118be198256`. Ancestry
verified. Existing planning files and ticket comments preserved.

Focused baseline, all through Devenv:

- `cargo test --locked --test repository_enablement remove_registration`: 2 passed.
- `cargo test --locked --test mutation_confirmation repo_remove`: 8 passed.
- `cargo test --locked --test recovery_foundation_gate registration_removal`: 3 passed.

At this base the CLI catalog still reserves `repo remove` for C2 and
`comment add` for C4. Its supported `id new --json` generated this ledger ID.
Ticket updates use canonical filesystem paths.

The full Rust gates and native CI have not run for this implementation.

## Tasks 1–2 — First batch checkpoint, 2026-10-19T18:27:52Z

Changes are uncommitted on the original ticket branch. Task base remains
`cdd797cba21878eecb83beed6023c118be198256`; review the full working-tree diff
and new files, not `HEAD~1`.

### Implemented

- Moved domain removal to `src/repository/registration_removal.rs`. Exact
  stored-root selection precedes Git; live aliases still canonicalize; unknown
  missing paths, linked worktrees, damaged Git indirection and regular files
  are rejected. The public request structure is unchanged.
- Added a private `RemovalTarget` and checked deletion method, with expected
  registration comparison before operation creation and again in the deletion
  transaction. `None` means expected absence, never unchecked deletion.
- Added bootstrap → optional Git → exclusive cache locking and guarded reprobe.
  A repository that reappears returns busy. A same-path replacement of `.git`
  must also return busy: `RepositoryLease::still_names` compares lock-file
  identity using Unix device/inode or Windows volume/128-bit file ID. No new
  dependency or timeout was introduced.
- The registry-only route preserves pending local work, unknown local evidence,
  all unfinished remote/parent lifecycles, and unrelated accepted requests.
  It recognizes the removal's own request and operation, preserves operation
  mismatch answers, and retains the existing automatic-poll yield path.
- Deletion remains transactional and root-scoped. Tests cover rollback after
  local journal deletion but before repository deletion, service-reopen retry,
  both roots' local/remote cascades, and preservation of receipts, confirmation,
  selected key metadata/material, host pins and a local draft sentinel.

### Red/green and investigation evidence

1. Three new domain tests failed on the old implementation: missing root,
   missing `.git`, and missing `.git` below an ancestor repository. They now pass.
2. A same-path Git replacement test failed against the initial path-only
   reprobe (`Removed` instead of busy). Comparing the held lease's filesystem
   identity fixed it. This realizes the plan's “matching Git lease” requirement.
3. Two focused tests found outcome/validation gaps: reuse of an operation ID
   returned recovery-required rather than operation-mismatch, and an unknown
   completed local step was discarded. Both now pass with explicit validation.
4. One parent-lifecycle test initially violated the closure fixture's
   `closure_kind` constraint. The fixture was corrected to a valid `record`
   closure; no production behavior was changed for that fixture failure.

### Verified commands

Every Rust command below ran through `devenv shell --`:

| Command | Result |
| --- | --- |
| `cargo test --locked --test repository_enablement remove_registration` | 8 passed |
| `cargo test --locked --lib repository::registration_removal` | 16 passed |
| `cargo test --locked --test recovery_foundation_gate registration_removal` | 4 passed |
| `cargo test --locked --test discovery_rebuild remove_registration` | 3 passed |
| `cargo test --locked --test mutation_confirmation repo_remove` | 8 existing live-boundary tests passed |
| `cargo fmt --check` | Passed |

`git diff --check` passed. `git merge-tree --write-tree HEAD origin/main`
reported no conflict against the fetched base. These are focused first-batch
results, not the Task 6 full gates or native-platform evidence.

### Review rulings and remaining work

- Ruling: use `Option<RegistrationStamp>` for the two expected states rather
  than a separate two-variant enum; the checked method always compares it.
- Ruling: keep the deterministic cache-boundary hook in the new module's
  `cfg(test)` test module. No new public hook/service field is needed.
- Ruling: reuse the existing two-root cascade helper via a path-based helper,
  retaining all old assertions and adding remote rows and survivor identity.
- Task 2's detailed recovery matrix and rollback tests live primarily in the
  new domain unit-test module, with restart and two-root proofs in the planned
  integration targets. No new native test target is required.
- Tasks 3–4 still need to wire stable scope, consent-bound target observations
  and checked deletion through the mutation boundary. The original missing-root
  preview/replay failures are not yet fixed end-to-end.
- Task 5 contracts/docs and Task 6 all-feature gates/final review remain pending.
  Windows file-identity code follows the API already used by key storage but
  has not been run natively. No CI was dispatched.

Paused at the agreed first-batch review checkpoint.

## Tasks 3–5 — Second batch checkpoint, 2026-10-19T19:20:21Z

Owner reviewed the first batch and instructed continuation. Fresh fetch/merge
at the start and end of this batch found no main movement; HEAD/base remain
the SHAs recorded above. Work remains uncommitted. Task 6 is next.

### Implemented

- Added the defaulted `Binding::resolve_scope` hook. Ordinary bindings keep
  live scope resolution. Removal freezes an exact registered root or a live
  canonical alias; an exact matching persisted scope can recover identity
  after disappearance. It does not guess a missing alias's target.
- `prepare` uses that hook; removal execution looks up request/confirmation
  scope evidence before resolving its identity. The hint grants no consent:
  existing command/target/digest, ownership and expiry checks still run. The
  new `RequestIdentity::with_scope` avoids a second filesystem resolution.
  Other commands retain their original execution ordering.
- Separated fresh selection from consent/request-bound registration observation.
  Bound observation works without disk state. The binding carries the observed
  registration stamp into checked deletion rather than rereading a new
  expectation at the domain-call boundary. A guarded `ExternalChange` leaves
  no new blocking request or journal when nothing ran.
- Covered finished-envelope replay after disappearance, lost settlement both
  before/after root loss, own pending removal after restart, another actor's
  removal, distinct re-registration, confirmation expiry/reuse/wrong-root,
  changed command/root, relative/dot/symlink aliases and retargeted aliases.
  An unrelated pending operation remains intact and the rejected removal's own
  request/confirmation reservation is released.
- Added two normalized wire fixtures for missing-root preview/removal, using
  the unchanged published schemas. The same cases cover missing `.git`.
- Added platform-oriented coverage for stored canonical spelling (including
  Windows verbatim prefixes), spaces/non-ASCII names, dangling Git symlinks,
  real Unix permission denial and fixed/redacted error messages. Permission
  errors now map to inaccessible state rather than a generic I/O result;
  deterministic error-classification coverage remains available on all targets.
- Updated CLI/persistence RFCs and the API audit. The historical F2 design
  links the follow-on fix. CLI C2 still owns executable exposure. All affected
  test targets already appear in `build.yml`; no CI matrix/dispatch change.

### Red/green and verification notes

- The four Task 3 regressions initially failed: missing-root preview,
  disappearance before execution, expiry while the root is absent, and a
  registration replacement between boundary validation and the domain call.
  They passed after scope/binding wiring.
- The actual Unix permission-denial test initially received `Io` rather than
  `InaccessibleRepository`. A removal-specific filesystem error mapper fixed
  the projection while keeping all non-absence failures blocked. The real
  permission case ran on this Linux user (no elevated-privilege skip).
- The first full `mutation_replay` run was terminated by the tool's 120-second
  limit, without a reported failing test. Rerunning the same command with a
  600-second allowance completed: 308 passed in 221.86 seconds.

Every Rust command below ran through `devenv shell --`:

| Command | Result |
| --- | --- |
| `cargo test --locked --lib repository::mutation::` | 106 passed |
| `cargo test --locked --test mutation_confirmation repo_remove` | 23 passed on the final batch tree |
| `cargo test --locked --test mutation_confirmation registration` | 10 passed |
| `cargo test --locked --test mutation_replay not_enabled` | 10 passed |
| `cargo test --locked --test mutation_replay` | 308 passed |
| `cargo test --locked --lib repository::registration_removal` | 20 passed |
| `cargo test --locked --test mutation_contract` | 44 passed |
| `cargo test --locked --test read_contract --test read_boundary` | 71 + 20 passed |
| `cargo test --locked --test repository_enablement remove_registration` | 8 passed |
| `cargo fmt --check` | Passed |

The filtered registration/not-enabled runs overlap broader suites; do not sum
them as independent test counts. `git diff --check` and merge-tree checks passed.
No schema, Cargo dependency or native workflow change was required.

### Remaining checkpoint

The public mutation path now exercises the missing-root fix end to end in
focused tests. Task 6 still requires all-feature check/format/Clippy/test gates,
complete-change review and final evidence. These have not been claimed complete.
Windows/macOS runtime behavior remains native evidence to obtain after an
authorized merge/dispatch. The approved same-second stamp limitation remains.

Paused at the second-batch review checkpoint.

## Task 6 — Final verification, 2026-10-19T22:27:35Z

The owner instructed continuation after the second batch. The complete change
is locally verified and review-ready. Changes remain uncommitted on the ticket
branch; no push, PR, merge, ticket closure or worktree cleanup was performed.

### Independent review and correction

Independent review covered the full working-tree diff against
`cdd797cba21878eecb83beed6023c118be198256`, including all new/untracked files,
the approved contract and the known same-second stamp limitation. Reviewer
session: `ses_eaa596262ffeVG5FxXZSLFs2JQ`.

One important finding was reproduced and fixed: a concurrent retry could
observe the registration, wait while an older attempt removed it, then receive
`ExternalChange` with an absent journal. Treating that as `NotRun` could delete
the accepted request's durable receipt even though removal succeeded.

- Added `repo_remove_concurrent_retry_keeps_the_receipt_when_the_older_attempt_removes_first`
  using two actual attempts and bounded channel/request-hook synchronization.
  It failed before the fix (`ExternalChange` instead of `AlreadyApplied`).
- Accepted re-entry (`attempt > 1`) with a checked-registration mismatch and
  absent journal now finishes with a durable `AlreadyApplied` result. Fresh
  execution still refuses stale consent. Pending journals/read failures remain
  protected, and a replacement registration is not deleted.
- The test covers live roots, missing roots and distinct re-registration,
  including attempt-2 settlement, service restart, result replay and refusal
  to reuse the confirmation. All 24 focused removal-boundary tests passed.
- The independent reviewer re-read the correction and regression test and
  reported the finding resolved, with no outstanding critical, important or
  minor findings. This was static review; execution evidence is below.

Clippy also required the permission fixture's zero mode to be written `0o000`.
That spelling-only correction and the replay-race fix preceded the final gates.

### Required gates on the final Rust tree

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed |
| `devenv shell -- cargo fmt --check` | Passed |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed |
| `devenv shell -- cargo test --all-features --locked` | Passed to completion, including doc tests |

The successful complete test invocation reported **2,892 Rust-harness tests
(including doc tests) plus 621 SSH-harness cases: 3,513 passed, zero failed**.
One pre-existing opt-in characterization test was ignored:
`tests/read_characterization.rs::reads_of_a_thousand_item_repository_are_characterized`.
No test was disabled for this fix.

The first full invocation exceeded the tool's one-hour limit without reporting
a failing test and was recorded as incomplete. The second invocation used a
larger allowance and completed. Only its summaries contribute to the counts
above. Full successful output is retained in the local tool log:

```text
/home/vhodges/.local/share/opencode/tool-output/tool_155ec517a0011zRUAW513RuuDd
```

Notable complete-suite counts: library 1,216; CLI unit tests 79; removal-containing
confirmation target 170; mutation replay 308; mutation contract 44; repository
enablement 84; recovery foundation 52; discovery/rebuild 79. These are subsets
of the aggregate above, not additional tests.

### Startup smokes and platform evidence

- `devenv shell -- cargo run --locked --bin manyhands-cli`: passed, displaying
  root help/catalog and opening no window. The catalog still marks `repo remove`
  as arriving with C2 and `comment add` with C4.
- Inspected display environment: `DISPLAY=:1`, `WAYLAND_DISPLAY=wayland-1`,
  `XDG_RUNTIME_DIR=/run/user/1000`.
- Built the desktop executable through Devenv, then launched it with
  `timeout --signal=TERM --kill-after=5s 15s devenv shell -- cargo run --locked --features desktop --bin manyhands`.
  It reached `Running target/debug/manyhands` without startup errors. The
  bounded smoke stopped only the process group it launched. This establishes
  startup, not interactive editor behavior.
- Windows/macOS runtime evidence remains the separately authorized post-merge
  native workflow. No workflow was dispatched and no matrix was narrowed.

### Final handoff

Fresh fetch and merge after the full run remained up to date:
`origin/main = 021f68425db6d4be38bcfbee0ee151829e06df41`, ticket
`HEAD = cdd797cba21878eecb83beed6023c118be198256`. The merge-tree check was
conflict-free and `git diff --check` passed. No Cargo dependency, lockfile,
published schema or native workflow changed. Subsequent edits are bookkeeping
of this evidence, not Rust changes requiring repeated gates.

The approved existing `(id, enabled_at)` same-second ABA limitation remains.
The ticket stays open for the owner's integration decision; close it only in
the authorized reviewed lifecycle checkpoint before merge.

## Delivery authorization and closure — 2026-10-19T22:33:49Z

The owner approved closing the ticket and committing the complete reviewed
change, merging into local main and pushing main to origin, removing the local
and remote ticket branches and the ticket worktree, then dispatching one Build
workflow on main. The ticket is closed in this final pre-merge checkpoint.

Fresh fetch confirmed unchanged main at
`021f68425db6d4be38bcfbee0ee151829e06df41`. The remote ticket tip
`bc673d95bc8df7a60f731a7e8dd4de34f01269c3` is already an ancestor of this branch.
Only closure/approval bookkeeping changed after the final Rust gates. Local
main has an unrelated untracked `.superpowers/` directory, which is preserved.

The worktree's ignored paths are Devenv state and Cargo/test build artifacts;
cleanup is limited to this ticket's worktree. Native CI results remain the
post-merge follow-up and are not inferred from local verification.
