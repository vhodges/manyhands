---
title: "Wave 03 C5 CLI Completion And One-Shot Polling Execution Ledger"
date: 2026-10-31
status: "in-progress"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M69C6TB91HGD0AWHBKV5GBYM"
---

# Wave 03 C5 CLI Completion And One-Shot Polling Execution Ledger

The plan's record of execution. Task 0 created it, and every task commits it.
Facts, rulings (R-N), named edits (E-N) and decisions are the numbers of the
[Cycle](../Cycles/wave-03-cli-05-completion-and-polling.md) and the
[design](2026-10-31-wave-03-cli-05-completion-and-polling-design.md). The
[plan](2026-10-31-wave-03-cli-05-completion-and-polling-implementation.md) names
the tasks. Task 0 ran on 2026-10-31 UTC.

## Baseline

Task 0 ran in the ticket worktree
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMXTNXNH78C11DYXS8H`,
on `manyhands/ticket/01M4CC0VMXTNXNH78C11DYXS8H`. No subagent was dispatched by
the Task 0 implementer.

### The base (Step 1)

- **HEAD** at the start: `131bddf9c869a559e4bd891e85dc8520a966afe7`
  (`docs: record the owner's approval of the C5 plan`).
- **`origin/main`** was `a52273ffe9b2b8e82b8e2c6e2d004e3a973cb8f7` while the facts
  were re-checked, then **moved during Task 0** to
  `87d87dead12f4e94b226b5d0eba03e9c0110a44a` (Windows timestamp correction). It
  touches only `docs/plans`, `packaging/windows` and `scripts/release`, so no fact,
  pin or reproduction here is affected. It was merged after the first Task 0 commit,
  `febf2a47`, without conflicts and without a rebase; that commit's check of main was
  run in the same command as its commit and push, so the move was seen only afterwards.
- `git merge-tree --write-tree` against `a52273ff` gave tree
  `43ddf77e4ea21713b39d9fc4353e31755bc9b918`, and against `87d87dea` gave
  `4d95dafe35e224de9b6a742ad43096b894bb9910`: no conflict.
- `git diff 717b664d HEAD` over `src`, `tests`, `schemas`, `Cargo.toml`,
  `Cargo.lock`, `scripts`, `.github` and `devenv.nix` is empty at `131bddf9`. The five
  changed files are the Cycle, design, plan and two comments.
- **Commit identity.** The owner approved a per-command identity at planning's
  approval. Every commit is
  `git -c user.name="Vince Hodges" -c user.email=vince@imbas.ca commit ...`, and
  no Git configuration is written.

### Baseline checks

Copied from planning's run at `717b664d`, which is this code. Task 0 did not
repeat them. All exit 0.

| Command | Time |
| --- | --- |
| `cargo check --all-features --locked` | 47 s |
| `cargo fmt --check` | 10 s |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | 51 s |
| `cargo check --locked --features desktop` | 27 s |
| `cargo clippy --all-targets --locked --features desktop -- -D warnings` | 40 s |
| `cargo check --locked --bin manyhands-cli` | 12 s |
| `sh scripts/check-cli-no-gpui.sh` | ok |

## Confirmed facts (Step 2)

Every Baseline Fact 1–41 was re-read at `131bddf9`, by grep and by reading the
cited lines. **No fact changed in substance.** Fact 18's "confirms by test" ran:
`cargo test --all-features --locked --lib bypasses_policy` and `--lib
explicit_bypass` each passed 1 of 1 (of 1478 filtered).

Citation notes, none of which changes a fact or a task:

| Fact | The design cites | The source |
| --- | --- | --- |
| 12 | `closure/mod.rs:92` | `:92` is the `derive`; `pub enum PlannedClosure` is `:93` |
| 25 | `readiness_state` at `read/graph.rs:272` | `:272` is the doc comment; `pub(super) fn` is `:273` |
| 29 | `mutation/settle.rs:48` for `Stopped{Some(Absent)}` | `:48` is the `NotRun` arm's `Absent => Delete`; the `Stopped` arm's is `:64-70`, which the fact also cites |
| 30 | `run.rs:491-498`, `:977` | `:977` means `promotion/lifecycle_record.rs:977`, as the plan cites it; `run.rs:977` is an unrelated line |
| 8 | Cycle 09 item 10, quoted | At `docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md:364-371`, wrapped across lines |

## Test pins (Step 3)

Grep of `tests`, `src/bin`, `src/repository`, `src/results.rs`, `src/lib.rs`,
`src/desktop`, `schemas`, `scripts` and `.github` for `NotResumable`, `"C5"`,
`arrives with`, `Planned` and `command_unavailable`, with a further sweep for the
six verbs' names. `PlannedIntegration`, `PlannedClosure`, `PlannedCleanup`,
`PlannedPublication` and `PlannedResult` are domain types, not pins.

**Listed, as the design says.**

| Hit | Named edit |
| --- | --- |
| `src/bin/manyhands-cli/catalog_tests.rs:95` | E-1 |
| `tests/cli/help.rs:291-304`, `tests/support/cli.rs:321`, `tests/fixtures/cli_v1/unavailable_ticket_close.json` | E-2 |
| `src/bin/manyhands-cli/parse_tests.rs:216,228,232,235-238` | E-3 |
| `src/bin/manyhands-cli/startup_tests.rs:332,362,371,440-442` | E-4 |
| `tests/cli/boundaries.rs:358` | E-5 |
| `tests/cli/exits.rs:18,141-143` | E-6 |
| `tests/fixtures/cli_v1/help_root.json:1964-2004`, `help_ticket_close.json:9`, `help_resource_ticket.json:504` | E-7 |
| `tests/read_relationships.rs:1183,1190,1254` | Data |

`NotResumable` has no test hit: its seven hits are in `bind/resume.rs`, which
Task 3 changes. E-10's `tests/mutation_replay/resume.rs:1186` is found by test name.

**Unlisted hits: open, for a ruling.** The plan says an unlisted hit stops for the
owner, so Task 0 does not settle these. Each is read, not run, since the verbs
aren't bound.

| Hit | What it is | Reading of what happens under C5 |
| --- | --- | --- |
| `src/bin/manyhands-cli/dispatch_tests.rs:96,100` | `assert_ne!(code, "command_unavailable")`; `assert_eq!(count, 28)` | Filtered to `delivered_by() == "C1"` reads (`:58-61`). C5's reads are `"C5"`, so 28 holds, and no edit is needed |
| `src/bin/manyhands-cli/catalog_tests.rs:122` | Maps `Planned(cycle)` to its cycle for the comparison with `DESIGN` | Compiles and holds under R-8; no edit |
| `src/bin/manyhands-cli/catalog_tests.rs:156` | The `Planned` arm of a shape test | Compiles under R-8; unexercised over `COMMANDS` once no verb is planned; no edit |
| `src/bin/manyhands-cli/catalog_tests.rs:263` | Builds `Availability::Planned("C2")` for `delivered_by()` | Holds, and needs `Planned` under `cfg(test)` as R-8 allows; no edit |
| `tests/mutation_contract.rs:1737` | `CODES_NO_MUTATION_GOLDEN_HAS`: `CommandUnavailable`, "a CLI parse result" | Stays true (R-8); no edit |
| `src/bin/manyhands-cli/startup_tests.rs:171` (verb sweep) | `ticket close --id X --prepare` makes no startup call | Becomes a usage error for the missing `--repo`, as `item republish --id X --prepare` answers today (run: exit 2, `usage_error`); passes, for a different reason |
| `src/bin/manyhands-cli/startup_tests.rs:293-294` (verb sweep) | `ticket close --help` is a help target | Holds; no edit |
| `src/bin/manyhands-cli/parse_tests.rs:204,467` (verb sweep) | `--version` and the raw flag scans | Hold; no edit |
| `src/bin/manyhands-cli/parse_tests.rs:563` (verb sweep) | `non_utf8_values_are_usage_errors_including_planned_arguments` uses `ticket close --id` | Should hold through clap's value parsing, and its name goes stale |
| `tests/cli/help.rs:110,157-159` (verb sweep) | Help stability and the `help_ticket_close` golden | Hold; the golden is E-7 |

No hit is seen to need a semantic edit. The rulings wanted: whether these go on the
record as no-edit (the reading above), and whether `parse_tests.rs:563`'s name and
`startup_tests.rs:171`'s changed reason deserve a named edit E-16.

## Readings (Step 4)

- **A same-ID retry clears `cancel_requested`.** Yes, in the runner:
  `promotion/run.rs:489-498` calls `L::clear_cancellation` when `existing &&
  r.cancel_requested()`, which is `promotion/lifecycle_record.rs:966-980` (`UPDATE
  ... cancel_requested=0` at `:977`). `lifecycle_record.rs:345-354` does not clear
  it: that is `begin_record`'s collision check, and at `:353-355` its local-sync
  fence, `require_no_pending`, which runs for a new record and for an unfinished
  one. So a same-ID retry is also fenced by a pending local sync, and only a
  terminal record is exempt. The clear comes after refused, superseded and complete
  records have returned (`run.rs:482-490`).
- **`state::configure` leaves `paused` alone.** Yes
  (`remote/state.rs:2250-2290`). Its one policy `UPDATE`, `:2286`, writes
  `history_unknown`, `configuration_generation`, `remote_name`, `primary_branch`,
  `automatic_backoff_seconds` and `latest_outcome`. It leaves `enabled`, `paused`,
  `interval_seconds` and `next_eligible_at`.
- **A registered repository always has its polling row.** Yes: the trigger
  `remote_policy_on_registration` (`remote/state.rs:64-66`), a backfill at `:108`,
  and a missing row is `recovery_required` when the schema is checked (`:34-37`).
  Through the binary, `repo enable` gave `(paused 0, enabled 1)`, one row per
  repository.
- **`request_records` and `remote_polling_state` share the registry connection.**
  Yes: both open `open_registry(&registry_path)` in an `Immediate` transaction
  (`mutation/records.rs:802-808`, `remote/state.rs:6402-6414`), and the one file
  `manyhands.sqlite3` held both tables when opened read-only. **The guards differ in mode only, over the same lock file (`cache_lock_path`):**
  records take `cache_write_guard` over the data directory, and `with_transaction`
  takes `cache_read_guard` (the root is only named in its errors). Task 5a's boundary path takes the write
  guard, as the design says.
- **Recovery after a stop in `set_publication_remote_recorded`.** It has no separate
  module. Re-entry goes through `set_publication_remote` (`src/repository.rs:4406`)
  and `begin_lifecycle` back into the same function, and the
  `config.publication_remote == request.name` branch, `:4500-4541`, is the recovery:
  `is_pending` with `PublicationCommitted` hands off at `:4515-4539`. The step comes
  from `:4628`. `ensure_configuration_path_clean` (`:4468`, `:10856-10905`) lets a
  stale index through, since it accepts any first-parent configuration entry
  (`:10889-10896`). So **the named function holds both sites, and the scope doesn't
  widen**: the index update belongs after the commit (`:4593-4603`) and on that
  branch. `recovery.rs:1047,1105` and `repository.rs:3643-3644` only classify the
  step. See Risks for the stop's test seam.
- **A confirmation stores no `closed_by`.** Yes. `confirmation_records`
  (`mutation/records.rs:114-133`) holds `confirmation_ulid`, `scope_key`, `command`,
  `target`, `intent_digest`, `observation_digest`, `created_at`, `expires_at`,
  `accepted_by_request` and `accepted_at`, confirmed by `PRAGMA table_info` on a real
  registry. It holds a digest of the expectation, which hashes `closed_by`
  (`closure/mod.rs:56-57`). See Risks.
- **What creates a missing context.** `ticket save` and `document save` (RFC:506-507).
  Run through the binary, on a ticket on primary with its worktree and branch removed:
  after `index refresh` and `ticket show`, `ticket save` with only its `observation`
  exits 0 with `already_applied`, writes nothing, and the branch and worktree exist
  again. An `index refresh` is needed first: until then `ticket show` is
  `item_not_found`.
- **A `FailurePoint` with an `ExternalChange`-kind error after a checkpoint.** None.
  `check_failure` always builds `RepositoryErrorKind::InjectedFailure`
  (`src/repository.rs:3372-3387`), and `boundary` only calls it
  (`promotion/run.rs:192-200`). **So Task 2b's row becomes a unit test of the
  mapping**, as the plan says. `repository_errors_of_every_kind_stay_repository_errors`
  (`closure/run_tests.rs:676`) already pins the domain's half. The one site that reads
  that kind as consent is the pre-record preview, `promotion/run.rs:437-439`.

## Reproduction (Step 5)

All runs used a new `/tmp` directory, with its own `HOME`, `XDG_CONFIG_HOME`,
`GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM=1` and `MANYHANDS_DATA_DIR`, with
`SSH_AUTH_SOCK` unset, and `target/debug/manyhands-cli` at `131bddf9`. The remote was
`ssh://git@example.invalid/team/repo.git`. No real data, key or registration was used.

**`01M669FJR2` reproduces exactly as its ticket says.**

| Step | Exit | Code | `.manyhands/config.toml` in `git status` | HEAD blob, index blob |
| --- | --- | --- | --- | --- |
| `repo enable` (prepare, confirm) | 0 | `ok` | clean | `bc6e1cc`, `bc6e1cc` |
| `remote select` | 0 | `ok`, committed | `MM` | `e2a1caf`, `bc6e1cc` |
| `repo sync` | 3 | `worktree_not_clean`, every effect `not_requested`, no action | `MM` | unchanged |
| The same `remote select` request (replay) | 0 | `ok`, stored | `MM` | unchanged |
| A new `remote select` request | 0 | `already_applied` | `MM` | unchanged |
| `git reset -- .manyhands/config.toml`, control | | | clean | `e2a1caf`, `e2a1caf` |
| `repo sync` after that, no key selected | 3 | `no_selected_key` | clean | |

So nothing but the stale index blocks the synchronization. For Task 6, a repository
already in this state isn't healed by a replay (fact 16 makes no domain call) or by a
new no-op request as things stand. The ticket's retry and replay criterion needs the
`:4500` branch to update the index too.

**The later-publication order doesn't reach `01M66HFV44`.**

| Order | `history_unknown` after | `repo sync` after clearing the stale index |
| --- | --- | --- |
| `key generate`, `key select`, then `remote select` (the design's order) | 0, before and after the selection | `transport_unavailable`, exit 5: it went on to the unreachable host |
| Control: `remote select`, then `key select` | 1 | `transport_unavailable`, exit 5, since a primary sync with nothing to publish doesn't reach the ticket's new-context path |

The control's `history_unknown = 1` is the mark that ticket describes, and the
design's order doesn't set it. `key select` with no remote configured touches no row
(`invalidate_key_selection`, `remote/state.rs:2297`). No surprise.

## Audit refresh (Step 6)

`docs/research/wave-03-api-audit.md` gains "Wave 03 CLI Cycle C5 refresh
(2026-10-31)", with the bindings, code lists and counts over the C4 refresh's groups.
Inventory rows for the six verbs are replaced in place, and four `context *-local` rows
are added. The counts are upper bounds. `context observe-local`'s no-context answer is
left to Task 10a's test, which the design allows ("as the read reports"). The sentence
near `:190`, and the audit's other document amendments, stay with Task 14b.

## Risks

- **The stop between `remote select`'s commit and its index update has no test seam.**
  `FailurePoint` (`src/repository.rs:890`) has only
  `BeforePublicationConfigurationCommit` on this path, and `src/repository.rs`'s library
  scope is the named function and its recovery. A new variant or hook for Task 6's test
  would be outside that sentence. Task 6 reports this if it needs one, or uses an existing
  stop: `advance_lifecycle` failing, or a lease hook.
- **A confirmation's digests hash `closed_by`.** Task 7's sentinel scan can't see this,
  and the design permits it ("A confirmation stores a digest"). `closure/mod.rs:56-57`
  calls the input low-entropy, and that's why `Debug` hides it. Recorded, not changed.
- **`repo sync`'s `worktree_not_clean` offers no action** in the reproduction, as the
  ticket says. The fix removes the case, and no action is added.

## Rulings and blockers

**Settled (owner, 2026-10-31, decisions 16 and 17):** E-16 for `startup_tests.rs:171` and `parse_tests.rs:563`; the other unlisted hits are no-edit, and one that fails when its verb is bound stops for the owner. One `FailurePoint` variant is allowed in Task 6. Task 0 is complete.

**Decision 20 (owner, 2026-11-01):** Task 4's stop on `remote_context_changed` is a known limit. `c5_made_ready_remote_context_changed` stays as written, and no library change follows.

## Task log

### Task 0: Baseline, Re-Grounding, Audit Refresh And Ledger

Documents only: this ledger, the audit, and one ticket comment. No `src` or `tests`
edit. The temporary directories were removed afterwards.

### Task 2a: Lifecycle Binding: Accessor, Prepare And First Run

**Base.** `05b8d4f0` (the owner's rulings). `origin/main` had moved to `25a8174d` (v0.1.4 metadata), and was merged first as `961ae44a`, without conflicts.

**Tests first.** 32 `c5_lifecycle_` cases ran against a stub binding answering `internal_error`, and all 32 failed. The harness reports only a panic's line number. Against the stub, the prepare-ready, published and rejected rows and conformance failed by assertion. The rows on the local world failed earlier, by a fixture fault (no probes saved), which was fixed only once the real binding was in, so **those rows were not seen to fail by assertion against the stub**. Twelve of them did fail by assertion against the real binding on the way (an unsupported schema keyword, then the unknown-ID rule).

**Commands, all exit 0, on the final code unless noted.**

| Command | Result | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_lifecycle` | 32 cases | 211 s |
| `--test mutation_contract` | 44 | 11 s |
| `--test read_contract` | 72 (one failure before its registration, below) | 8 s |
| `--test mutation_replay` | 383 | 273 s |
| `--test mutation_confirmation` | 183 | 59 s |
| `--test mutation_bridges` | 166 | 61 s |
| `--lib mutation` | 126 | 34 s |
| `--bin manyhands-cli -- human_admin` | 12 | 5 s |
| `cargo fmt --check`; both clippy commands; `cargo check --locked --features desktop` | ok | 10–25 s |

Goldens were regenerated with `MANYHANDS_UPDATE_GOLDEN=1` and read.

**Decisions.**

- The first run's standing is implemented, since a first run needs one and `lifecycle_parent` has no other caller: no parent deletes the request; a consent refusal, superseded or complete parent finishes it; an unfinished parent leaves it accepted. Task 2b tests the table.
- `Binding::prepare` checks nothing, so a state blocked since its preview is `external_change` with no record (the review's R-22 below makes one exception). `Blocked` at run is reached only after acceptance, and a case stages it with the request hook.
- The domain reports an unknown ID as `ContextMissing`, not `NotA*`. The binding asks the index: `ContextMissing` with `show_item` answering `item_not_found` is `item_not_found`, and, after the review below, only when the index is not marked behind.
- A result's scope names primary as its branch and no worktree, since the context doesn't outlive a complete run.
- `ParentView` holds only what this task reads: step, refused, checkpoint, integration. Later tasks add fields with their callers, because clippy denies unread ones.
- A checkpoint counts as an effect only once the context's branch holds it, since `begin_closing` records it before the ref moves.
- Seven `PreviewEffect` values are added: `context_synchronized`, `closure_recorded`, `primary_integrated`, `primary_published`, `remote_branch_deleted`, `worktree_removed`, `branch_deleted`.

**Deviations from the task's file list, none changing behaviour.**

- Compile-required arms in `tests/mutation_remote/host.rs`, `tests/support/mutation.rs` (six) and `tests/support/repair.rs`, and the blocked arm in `tests/support/confirmation.rs`.
- `tests/read_contract.rs`: 15 enumeration rows, and `schema_enum` learns the path step `items`. The plan's Step 3 does not list this target, and `published_enumerations_list_exactly_the_registered_values` failed until they were added.
- No `outcome_tests.rs` edit (E-12) was needed: its `Command::ALL` loops hold for both commands.

**Left for later, each with a comment where it stands.**

- Task 2b: `row` and `observe_again` answer `recovery_required` with the record left accepted and no domain call, whatever the parent holds, so a resend does not yet continue. `recorded_effects` claims nothing for these commands.
- Task 2c: `cancel_request` sets a flag nothing reads yet for these commands, and `Interrupted` is always `recovery_required`.
- Task 3: resume. Task 4: made-ready. Task 6: the promotion identity row.
- Task 7: the human rendering of `LifecycleBlocked`, which is the code's message for now; no verb reaches it yet.

#### Task 2a review fixes (controller's rulings)

- **R-22.** A closure observed as exactly `[IdentityRequired]` at execute answers `identity_required` from `Binding::prepare`, with no record and no domain call, and its blockers in `data`. Other states blocked since their preview stay `external_change`. This amends the design's Identity row, and R-21's terminal row now covers only the race after this check. The Cycle's decisions are unchanged. Case: `c5_lifecycle_identity_removed_after_prepare_close`.
- **Stale index.** An ID is unknown only when `repository_snapshot(root).refresh_required` is false; otherwise the `recovery_required` answer naming `context_missing` stands. Cases: `prepare_stale_index` and, for the design's new known limit, `prepare_index_not_marked`, both verbs.
- **Seen first.** The identity row and both stale-index rows failed by assertion before the fix. The slug row and `local_only`'s Git assertions passed at once: they pin existing behaviour.
- **Minors.** `Binding::reenter` delegates to `confirm::reenter`. `Ready.context_child` is computed once in each `ready_of_*`, and the verb method and `tracking_context` are gone. `from_stored` has one `optional_known`. Two comments corrected. `lifecycle_world` saves a distinct title, which `prepared()` and `called()` scan for. `DATA_ROOTS` is in order.
- **Carried to Task 2b.** `parent_effects` under-reports past integration (publication and cleanup steps), and the `stopped()` rows.
- **The shortfall** recorded above needs no rerun against a stub (controller).
- **Verified after the fixes.** `c5_lifecycle` 38 cases (210 s), `mutation_contract` 44 (11 s), `read_contract` 72 (7 s), `mutation_confirmation` 183 (58 s), `--lib mutation` 126 (13 s), `fmt --check`, clippy (11 s). `src/desktop` was not touched, so the desktop commands were not rerun. No golden changed.

### Task 2b: Re-Entry, Standing And The Answers Table

**Base.** `4462d3c6`. `origin/main` was still `25a8174d`, so nothing was merged.

**Tests first.** 67 rows were added and run against Task 2a's binding before any library edit: of 105 cases, 55 failed and 50 passed. The harness reports a panic's line number.

- **Failed by assertion on the behaviour:** every re-entry row, since the placeholder answered `recovery_required`.
- **Failed for another reason, so not seen to fail on the behaviour:** `deletion_rejected` and `local_sync_pending` (a fixture fault, below), and `index_pending` (only its golden was missing: Task 2a's first run already answered it).
- **Passed against the old binding, pinning what exists:** those rows answered on a first run or before the boundary, such as `expired_confirmation`, `consent_refused_at_run`, the cache-loss rows and `comment_publication_pending`.

**Fixture faults found against the real binding, none a changed answer.**

- `deletion_rejected`: the fixture refuses only the receiver's current branch, so the row sets the bare `HEAD`, as the domain's journeys do.
- `superseded`: a closure's further commit is never merged, so the row uses an untracked file, then a poll, as the domain's journeys do. **A superseding prepare needs that poll**: a push does not advance tracking, and the preview says `primary_ahead_of_remote` until then.
- `local_sync_pending`: C4's helper leaves primary ahead of the remote, so the row uses a local world.
- `primary_advanced`: the merge count now walks primary's first-parent line, since a child's merge is on the side.

**Commands, all exit 0, on the final code.**

| Command | Result | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_lifecycle` | 105 cases | 949 s |
| `--test mutation_replay` | 383 | 279 s |
| `--test mutation_confirmation` | 183 | 55 s |
| `--test mutation_contract` | 44 | 8 s |
| `--lib mutation` | 135 | 1 s |
| `cargo fmt --check`; `cargo clippy --all-targets --all-features --locked -- -D warnings` | ok | 10 s |

No schema enum and no desktop file changed, so `--test read_contract` and the desktop commands were not run. Seven goldens are new and were read. No existing golden changed.

**Decisions, for review.**

- **A stop with no parent offers `request.prepare` for `busy`, `poll_yielding`, `remote_unavailable` and `transport_unavailable`.** The answers table's Next column says `request.retry`, but the design says that column is for `Owed`, and that a stop with no parent is followed by a new prepare "whatever its code". Other codes keep their own actions.
- **`discovery: pending` on a rejected push stays.** Decided by test: `push_rejected_continues` asserts that it equals the index's `refresh_required`, and the finished request reports `current`.
- **A superseded request's stored `request_mismatch` is `partial`, naming its integration,** with cleanup `pending` and no actions. Its integration stands, as the table's "4 after a replay's effects" allows. `Superseded` counts as past integration, since only a cleanup stop is ever superseded.
- **"Data naming the state" is what a fresh preview says.** A `RecoveryRequired` stop or a child's `recovery_required` with a standing parent reads the preview, and its blockers go in `data`, with the checkpoint when one is written. The refused checkpoint merge names `integration_conflict`. The human `next:` lines are Task 7's.
- **Host stops with a standing parent offer `request.retry`, `repo.inspect`,** for a child's stop and for `host_replacement_required` too, since lifecycle data names no authority.
- **A child's `remote_branch_deleted` offers `request.retry` alone,** as the table says. The reservation is the child's, so `request.cancel` would be dropped anyway.
- **A stop's `discovery` past integration reads the index,** not the step, so a lost result of a complete parent reports what is so.
- `ParentView` is generic over its blockers, holds the refusal, expectation, item ID and publication, and loses its derives. `cleanup` and `cancel_requested` wait for a caller.
- `PrimaryAdvanced`, a repository `ExternalChange` and the remaining categories: the first is a row, the second a unit test of the mapping, as Task 0 found, and every publication and deletion category has a unit row beside the transport, unlock, host and rejected rows.

**Risk, for review.** With `remote_branch_deleted` the same request can't get past the deleted branch, and the table offers it nothing else. The answer is not `partial`, so `should_retry` won't repeat it, but its way out is unproven. Task 12a or 13b's walk should reach it.

**Deviation from the task's file list.** Seven names in `tests/support/mutation_golden.rs`, as Global Constraints allow.

**Left.** Task 2c: cancellation, including `Interrupted` as `cancelled`. Task 3: resume. Task 4: made-ready. Task 6: identity rows. Task 7: the human lines.

#### Task 2b review fixes (controller's rulings)

The design's Rulings table holds R-23 to R-28, and its standing and answers sections are amended to match.

- **R-23, a stale final answer.** `stopped()` no longer stores a stop's code because the parent reads as ended afterwards. Refused or superseded answers from `ended_answer`, `Final`. Complete is `Owed`, offering `request.retry`, and the next call gets the stored outcome.
- **R-24, a child's `remote_branch_deleted` and `merge_required`.** They offer `request.retry`, then `repo.inspect`. **Task 2c owes the walk of the way out (cancel, republish, retry) to a finish, and if it cannot finish, that stops for the owner there.** The row now resends once, and the same request is refused the same way. `merge_required` is a unit row only, because `synchronization_error_code` says the bindings merge and none is answered with it.
- **R-25, a kill between the ref and the record.** `parent_effects` at `Integrating` asks whether primary holds `integration_oid`. Rows at `BeforeIntegrationRef` and `AfterIntegrationRef`, each counting one merge.
- **R-26, the checkpoint's holder.** It is the context's branch, or primary once the branch is gone, and no longer depends on `HEAD`.
- **R-27, finding 5: the race is real.** `coordination::repository_lease` fails with `RepositoryBusy`, the preflight's `?` keeps it a repository error, and `repository_error_code` makes it `busy`. Against the code as it stood, `busy_lease_on_reentry` answered `busy` and offered `request.prepare`, with the record deleted under the other call. A re-entered call with no parent and a code that passes by itself now leaves the record and offers `request.retry`. A first call still deletes its own.
- **R-28.** With no parent, `internal_error` offers nothing. `ended_answer` is the one test of an ended parent, and `ended` is gone: `Abandoned` with no refusal answers `external_change` and is not run.
- **Quality.** `outcome.rs` has one `prepare_again` and one list of codes that pass by themselves (`passes_by_itself!`), which `no_parent_recovery` uses. `blocked_data` and `retry_then_inspect` are extracted, `parent_effects` computes `checkpoint` once, and `row`'s comment is corrected. `child_conflict` asserts `conflict_pending`.

**Added rows.** `busy_lease_on_reentry`, `integrating_before_ref`, `integrating_after_ref`, the three `parent_unreadable_*`, `external_refusal_with_record_accepted`, `busy_with_a_standing_parent`, `busy_after_a_checkpoint` (exit 4's outcome), `poll_yielding_past_integration` (exit 4's outcome), `interrupted_by_a_changed_remote`, and `deletion_host` continuing after approval.

**Deviations.**

- **The concurrent-retry row is not end to end, against the ruling's wording.** No public seam gives a stop after another call completes the parent. The request hook runs before the domain, whose first read returns a stored `Complete`. Failure points take no hook. The observation hook, tried in a row, fires in the call's own refresh, after it has finished, and that call answered `ok`, so the row was removed. Inside a child, the other call would be refused by the reservation. R-23 is pinned by unit rows through a verb with a fixed parent, in `bind/lifecycle.rs`, for complete, superseded and refused parents. `complete_with_record_accepted` pins the resend's stored success. A real two-process row would need a new seam, which is outside this fix.
- **The superseded variant is a unit row** for the same reason, and because a superseded parent's own stops return as outcomes.
- **The refused merge's human `next:` lines move to Task 7**, from the design's answers row. This task gives its code, actions, data and golden.

**Seen first.** Against the code before the fixes, `busy_lease_on_reentry`, `integrating_after_ref` and `child_remote_branch_deleted` failed on the behaviour. Rows that passed at once pin existing behaviour: `integrating_before_ref`, `parent_unreadable_on_mismatch`, `external_refusal_with_record_accepted`, both `busy` rows, `poll_yielding_past_integration`, `interrupted_by_a_changed_remote`, `child_conflict` and `deletion_host`. The other two `parent_unreadable_*` rows failed then on a fixture fault (a table rename can't be undone past the schema check, so the registry's image is restored) and were not rerun against the old code. **The unit rows were written with the fix**, since the signature changed, so they were not seen to fail.

**Verified on the final code, all exit 0.** `c5_lifecycle` 126 cases (1037 s), `mutation_replay` 383 (277 s), `mutation_confirmation` 183, `mutation_contract` 44, `--lib mutation` 141, `cargo fmt --check`, clippy `-D warnings`. No golden changed.

### Task 2c: Cancellation Reach

**Base.** `ab6a9750`. `origin/main` had moved to `b411e208` (v0.1.5, Windows signing), and was merged first as `f3dc3f81`, without conflicts.

**Tests first.** 51 `c5_cancel_` cases ran against the unchanged library, and all 51 failed. 49 failed on the behaviour: 45 on the `cancelled` answer, 2 (`killed_after_acknowledging`) on the flag, and 2 (`walk_remote_branch_deleted`) at the end of the walk. **`complete_with_record_accepted` failed on a fixture fault** (it stopped at a point that finishes the record), so it wasn't seen to fail on the behaviour. It uses `BeforeRequestSettlement` now. The child-flag and walk rows were tightened to their final assertions after that first run, once the answers were read, and were not rerun against the old library.

**Commands, all exit 0, on the final code.**

| Command | Result | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_cancel_` | 51 cases | 831 s |
| `--test mutation_remote -- c5_lifecycle` | 126 cases | 1032 s |
| `--test document_promotion` | 74 | 750 s |
| `--test ticket_closure` | 73 | 669 s |
| `--lib remote` | 478 | 209 s |
| `--lib local_sync` | 164 | 291 s |
| `--lib integration` | 26 | 16 s |
| `--lib mutation` | 141 | 2 s |
| `cargo fmt --check`; `cargo clippy --all-targets --all-features --locked -- -D warnings` | ok | |

`src/repository/remote/` was not touched. No schema, golden or desktop file changed, so `read_contract` and the desktop commands weren't run.

**Superseded by decision 18, below.** What follows records what the walk found before it.

**R-24, the walk: a way out finishes, and it isn't the one the ruling named.** Cancel, republish, retry does not finish. Found by testing, for both verbs:

| Step | Answer |
| --- | --- |
| The request | `remote_branch_deleted`; `request.retry`, `repo.inspect`; the child holds the reservation |
| `request cancel` | `requested` |
| The same request | `cancelled`: the parent stops at its first safe point, before any child, so **the child still holds the reservation** |
| `item republish` | `busy`, `request.retry` |
| The same request, instead (the dead end, first exploration) | The row is retired and a new attempt answers `remote_branch_deleted` again, holding the reservation again |
| `operation resume` of the child, whose ID `repo inspect` names as `active_operation_id` | `cancelled`: the row's flag is acknowledged and the reservation is free |
| `item republish` | `ok` |
| The same request | `ok`, one merge |

The row is `c5_cancel_walk_remote_branch_deleted`. It reads the active operation through the library call, not through `repo inspect` itself.

**`request.cancel` is not offered: open, for a ruling.** It is the first step of what works, and the task allows offering it. But offering it needs `offering_cancel` in `outcome.rs` to see a parent's child as the request's own, and it changes `c5_lifecycle_child_remote_branch_deleted`'s pinned actions, in a file outside this task's list with no named edit. And the action alone doesn't describe the way out, since the `operation resume` step is what frees the reservation. So the answer still offers `request.retry`, `repo.inspect`.

**A pending child's flag.**

- A child stopped by a transport fault keeps its reservation in a nonterminal phase. `request cancel` sets its row's `cancel_requested` with nothing running, and the `cancelled` answer leaves it, since no child ran.
- **`restart` does not clear the flag.** It acknowledges it (`restart_synchronization_guarded`, `reservation.rs:2510`): the row ends `cancelled` with the flag still 1, and the next call runs a new attempt under a new ID. Pinned by `c5_cancel_child_flag_unowned`.
- A retained conflict's row has already ended `interrupted`, so `request cancel` sets no flag on it. The cancellation is answered once, and the next call answers `recovery_required` as before. Pinned by `c5_cancel_child_flag_retained_close`.

**Decisions, for review.**

- **The probe is data:** `CancellationProbe { token, request }`, on `LifecycleRequest`, reached through `promote_document_observed` and `close_ticket_observed`. `Session`'s two methods take the request's ID. Public signatures stay. A flag that can't be read is an error at a safe point, and a `Stop` in a child, not "no".
- **A refused deletion is `Pending{Cancelled}` when a cancellation was asked.** `request cancel` sets the parent's own reservation row, whose boundary refuses the deletion as a repository error, which answered `recovery_required`. `finish_steps` now reads `cancelled::<L>` on a deletion error, as `child` already does. This is the one runner behaviour beyond the probe, and only a run asked to stop reaches it.
- **`cancel_request` reports `acknowledged: false` for these commands while the record is accepted.** The acknowledgement is the `cancelled` answer, and the journal loop isn't run, since `request_cancellation` already sets the rows.
- **`Stop::Cancelled` is new:** `Interrupted` becomes it when the token, the request's flag or the parent's flag says so, so `interrupted_by_a_changed_remote` still answers `recovery_required`.
- `promote_with_io` is now `#[cfg(test)]`: production reaches the runner through `run_lifecycle`.
- **A new ID past integration has nothing to confirm.** Before integration it answers `busy`, offering `request.prepare`, with its record deleted. At `AfterPublication` its prepare is already refused, `recovery_required`, naming `primary_ahead_of_remote` and `remote_context_deleted` until a poll has run. Both pinned, and in both the same request then finishes.

**Risks, for review.**

- `remote_context_deleted` in that prepare: see decision 18's section, which explains it.
- A non-cancelled stop between a `request cancel` and its answer leaves the flag, so the next resend answers `cancelled`, as R-20 intends.

**Deviation.** `tests/mutation_remote/c5_cancel.rs` duplicates a few helpers, since `c5_lifecycle.rs`'s are private and outside this task's files.

**Left.** The ruling above. Task 5b: the probe on `remote/poll.rs`. Task 7: `stops_on_ctrl_c`, and the human lines.

#### Decision 18: the runner releases the child

The owner ruled on Task 2c's stop: "Runner releases the child (Recommended)".

**The route now, both verbs, `c5_cancel_walk_remote_branch_deleted`.** `remote_branch_deleted` offering `request.cancel`, `repo.inspect`; `request cancel`; the same request, `cancelled`, reservation free; `item republish`, `ok`; the same request, `ok`, one merge. No resume step.

**What is retired.** `release_child` runs at the two places a cancellation stops a parent before its child runs again: the first safe point at `confirmed` (context child), and `finish_steps`'s first check at `integrated` or `publication_pending` (primary child). It reuses `retire_child`, extracted from `child()`. It retires only an unowned, unfinished row with no resolution, no reconciliation owed, no recorded push, and a checkpoint of none, `fetch_prepared` or `fetch_observed`. That list is stricter than `child()`'s retry path, which also retires later rows and then has its new attempt find out what happened. No connection is made, and a failure leaves the row for the same request.

**What stays, pinned.**

- `c5_cancel_child_push_unknown_stays`: a push that may have landed. `cancelled`, the row untouched and still reserved. The same request finishes, with one merge.
- `c5_cancel_child_flag_retained_close`: a retained conflict, untouched. `recovery_required` afterwards.
- Later checkpoints (a local update prepared or made) stay by the same rule, with no row of their own.

**Also released:** the primary's child, `c5_cancel_publication_child_released`. The ruling says "its pending unowned child", and this is the same stop after integration. Integration stands, and one merge.

**A pending child's flag.** `restart` still doesn't clear it. In `c5_cancel_child_flag_unowned`, the retire ends the row `cancelled` with the flag still 1, with no connection, and the next call runs a new attempt under a new ID.

**`request.retry` is out.** A child's `remote_branch_deleted` offers `request.cancel`, `repo.inspect`, by `outcome.rs`'s rule that the same request is not offered for what it cannot do. The resend is refused the same way (`c5_lifecycle_child_remote_branch_deleted`, E-17). `cancelled` offers `request.retry`, which by then can succeed.

**`merge_required` (R-29).** It takes the same actions, through `never_gets_past`. **The walk does not prove it:** no binding is answered with it, so only a unit row pins it.

**`offering_cancel`.** It asks `bind::lifecycle::held_by_its_child`, which answers false for every command but these two, so no other command's list changes. `ParentView` gains `children`.

**`AfterPublication`'s `remote_context_deleted` is a false report, and it predates this work.** Read in the row, at that prepare: the branch stands at the remote, the context has no tracking ref, and primary's tracking ref is behind the remote's primary. `promotion/observe.rs:919-928` checks only when the tracking context is absent and the integration isn't already contained, and the base it plans from is the stale tracking primary, so the check is reached. `remote_context_history` then reads stored publication evidence as `Deleted`, and absent tracking is all it had to go on: **it means "published, and no tracking ref", not "deleted".** The 2b ledger's note applies: a push doesn't advance tracking. `closure/observe.rs:792-799` is the same check. The values were logged for the closure; the promotion row asserts the blockers and the branch. `primary_ahead_of_remote` beside it is true, and a prepare is refused either way until a poll.

**The row doesn't pin the value.** `new_id_after_integration` asserts the code, `primary_ahead_of_remote` among the blockers and that the branch stands. Pinning `remote_context_deleted` would freeze a wrong word. Correcting it is outside this task, and is for the owner to ticket if wanted.

**v0.1.6.** `git diff --stat b411e208 818ddf88` has 16 files: `.github/workflows/release.yml`, `Cargo.toml` and `Cargo.lock` (version), docs, packaging, `scripts/release`, and `tests/fixtures/cli_v1/version*`. **No file under `src/`.** The run below is on that merge.

**Seen first.** Against the runner before this, `walk`, `child_flag_unowned`, `publication_child_released` and `c5_lifecycle_child_remote_branch_deleted` failed by assertion, both verbs. `child_push_unknown_stays` and `new_id_after_integration` passed at once: they pin what must not change. The unit row was rewritten with the fix.

**Verified on the final code, all exit 0.** `c5_cancel_` 55 cases (889 s), `c5_lifecycle` 126 (1034 s), `document_promotion` 74 (749 s), `ticket_closure` 73 (668 s), `--lib mutation` 141 (34 s), `cargo fmt --check`, clippy `-D warnings` (28 s). `src/repository/remote/` was not touched, so the three lib filters were not rerun for this change.

#### Task 2c review fixes (controller's rulings, within decision 18)

**1. Atomic release.** `release_promotion_child` evaluates the guards and ends the row in one immediate transaction, with `acknowledge` inside it, so no flag is left if the ack would fail, and no owner passes a checkpoint between. `retire_child` is gone: `child()`'s retry is back as it was, and the release makes no `io` call. `release_child`'s comment says there's no liveness proof, and that a live owner is fenced. The design's Known Limits has the second live process.

**A row answered otherwise, and I fixed it, for review.** `request cancel` also sets the flag on a child's row (`lifecycle_record.rs:961`). A kept child held that flag, so the next restart ended the row and answered `recovery_required` once, then `ok`. On a real resolved child that made "stays" false. So the same transaction clears the flag on a kept child, since the cancellation has been answered. The rows' `assert_kept` pins phase unchanged and flag cleared. `released_resolution_publishes` then finishes on the next resend, and primary at the remote holds the resolution. This clearing is beyond the ruling's wording.

**2. The guards, each with its conjunct removed (`--test mutation_remote`, closure rows).**

| Removed | Row | With it removed |
| --- | --- | --- |
| `reconciliation_required` | `stays_reconciliation_required` | failed by assertion |
| `push_oid` | `stays_recorded_push` | failed by assertion |
| the checkpoint match | `stays_local_prepared` | failed by assertion |
| window intent and steps | `stays_applied_merge_at_fetch_observed` | failed by assertion |
| resolution alone | `stays_released_resolution` | **passed**: the resolution's integration steps also keep it |
| resolution and steps together | `stays_released_resolution` | failed by assertion |

So the resolution guard is second to the step guard on this state, and either alone keeps it. Not exercised by removal: `retains_promotion_child`, which `retained_conflict_is_kept` can't show since its row is already `interrupted`; the publication attempt; the generation.

These rows change a row by SQL, except `stays_past_its_fetch` and `released_resolution_publishes`, which are real. `stays_past_its_fetch` doesn't finish: a peer's commit changes what the preview consented to, so the next resend is `external_change`, as it is with no cancellation. Also added: `release_by_token`, and a promote variant of the retained conflict, renamed `retained_conflict_is_kept`.

**3. After a failed child.** `release_child` also runs at the context child's failure and in the primary's `Err` arm, when cancellation is asked.

- `token_during_failed_child` lands the token at the first advertisement after `fetch_observed`. No boundary reads a token, so the child ends `remote_branch_deleted`. With the context arm's release call removed it failed by assertion (`!reserved`), and it passes with it restored.
- **`request_during_failed_child` passed against the code before this fix.** A `request cancel` also flags the child's row, and I did not trace where the child acts on that, so this row may not reach the failed path. It pins the route, not the arm.
- The primary's `Err` arm has no row of its own.

**4. Deletion mapping: narrowed.** It can be told apart. Only `Repository(RecoveryRequired)`, which is what the boundary gives, maps to `Pending{Cancelled}`. `deletion_transport_keeps_its_answer` failed against the wider mapping and passes. `deletion_rejected_keeps_its_answer` passed against both, so it doesn't prove the narrowing: the rejection reaches a different path. **Limit:** the boundary also runs after the transport call (`sync.rs` `with_scoped_action_remote`), so a cancellation landing just then answers `cancelled` after a deletion went through. The next call's guarded delete reconciles it. I did not write a row for that.

**5.** E-17 names the unit row's rewrite.

**6. A killed process.** `answered_without_taking_the_flag` is the real window: the token answers each probe, so the only flag read is the taking, and its one-shot failure leaves the flag. It answers `cancelled`, then `cancelled` once more, then finishes. The old row is renamed `killed_before_the_domain_call`. `flag_unreadable_at_a_safe_point` answers `recovery_required`, with no merge, and the same request finishes. The observer's `Stop` can't be reached that way, since the failure point fires once and a safe point reads first, so it's the unit row `an_unreadable_flag_stops_a_child`, which hides the table.

**7. What the suspected cases showed.**

- **A retained child hitting `remote_branch_deleted`: doesn't arise on this route.** A retained conflict whose branch is then deleted answers `recovery_required`, `request.retry`, `repo.inspect`, with no reservation, so no cancellation is offered (`retained_child_remote_branch_deleted`). I checked that one route only.
- **`offering_cancel` with nothing held:** it drops `request.cancel`, leaving `repo.inspect`. The unit row `a_cancellation_is_not_offered_when_nothing_is_held` reaches it through a repository that isn't registered, so the read fails. The branch for a read of no reservation is the same code, by reading, and isn't run.
- **A clean `Applied` merge at `fetch_observed`.** `checkpoint_synchronization_merge_applied` moves the row to `local_fast_forwarded` in one transaction, after `integrate_divergence` has moved the branch, so this state exists only for a kill between the two. The merge is on the branch, so nothing is lost if retired, but it isn't proven to have nothing to find out, so the guard keeps it. The row reaches the state by putting the checkpoint back by SQL.
- `close_ticket_observed`'s path is tidied.

**Not run:** `--test remote_synchronization`. The reservation change is one added function, which only `release_child` calls.

**Verified on the final code, all exit 0.** `c5_cancel_` 86 cases (1382 s), `c5_lifecycle` 126 (1031 s), `document_promotion` 74 (743 s), `ticket_closure` 73 (665 s), `--lib mutation` 142 (7 s), `--lib remote` 478 (176 s), `--lib local_sync` 164 (293 s), `--lib integration` 26 (16 s), `cargo fmt --check`, clippy `-D warnings` (11 s). `an_unreadable_flag_stops_a_child`, which those filters don't select, passed on its own (1 test). No lib source changed after it, apart from the removal above, which was restored.

### Task 3: Resume Routes

**Base.** `3f6037c7`. `origin/main` had not moved, so nothing was merged.

**Tests first.** 31 `c5_resume_` cases and the E-10 row ran against the unchanged library. 29 cases and the E-10 row failed by assertion on the answer. `pending_child`, both verbs, passed: it pins what exists. `no_record_past_integration` and `complete_with_record_accepted` first failed on fixture faults (below). Once those were fixed they were run against the old library again, with its three files put back for the run, and failed by assertion on the result code. The harness reports only the line, so which code the old library gave wasn't read.

**Fixture faults, none a changed answer.**

- A domain stop past integration is an outcome with cleanup pending, not an error, so `begun_outside` doesn't assert an error.
- A call that dies at `BeforeRequestSettlement` names no operation, so that row reads the ID from the parent table.
- Past integration, one further accepted update is the context's deletion. The row asserts `pushes + 1`, and that primary doesn't move.
- **Not a fixture fault, and it should have stopped for a ruling:** `grants_no_new_consent` first expected `request.prepare`, the code answered an empty list, and I changed the expectation to match. Reverted by R-31, below.
- The E-10 row first expected outcome `error`. `original_request_required` is `blocked`, as this file's other pins have it.

**Commands, all exit 0, on the final library.**

| Command | Result | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_resume` | 31 cases | 202 s |
| `--test mutation_remote -- resume` | 71 cases | 657 s, detached |
| `--test mutation_replay` | 384 (383 and E-10) | 283 s |
| `--lib mutation` | 142 | 2 s |
| `cargo fmt --check`; `cargo clippy --all-targets --all-features --locked -- -D warnings` | ok | 10 s each |

The first `mutation_replay` run failed only on the E-10 row's expected outcome. It was rerun in full after that test-only fix. No schema, golden or desktop file changed. `src/repository/remote/` wasn't touched.

**Decisions, for review.**

- **The route reads the parent tables directly, inside `named`'s one read.** `lifecycle_parent` takes an immediate transaction, and `takes_up` runs on every envelope that offers a resume. A parent table that doesn't exist yet, or a repository that isn't registered, reads as no parent.
- **The order is the table's.** Complete, then refused or superseded, are decided before any holder is looked for. So a complete parent with an accepted record is `already_applied`, and that request still gets its stored outcome.
- **A continuation goes through the verb's own binding,** by `Confirmed::row` and `Binding::rerun`, so the expectation is the record's and a parent that ended since answers from what it holds. `bind/lifecycle.rs` needed no code. Only its header comment changed.
- **A stop leaves no record.** Each same-request action becomes `operation.resume` of the parent, and a stop with no actions gets that one, unless it offers nothing. A final answer is stored, and its replay is unchanged through the `lifecycle` tag.
- **The poll arm is implemented,** since E-10 can test it: the row seeds an accepted record holding the poll. A poll with no record stays `invalid_input`. Task 5b has no resume arm left to add.
- **A resume entered again that finds its parent complete answers `already_applied`,** as any other non-synchronization does, not the parent's stored outcome.
- **The position is primary's,** so a stored commit is checked on the branch that holds it.
- **Withdrawn (R-34, below):** this entry said `request cancel` of a resume that is continuing a parent stops it. That was false.

**Deviation from the task's file list.** One comment in `bind/lifecycle.rs`'s header, which still listed resume as Task 3's to do. It is within the library scope.

**The Ctrl-C row.** `ctrl_c` runs at the library, by a token cancelled in the request hook: `cancelled`, no record, `operation.resume`, then a finish with one merge. `stops_on_ctrl_c` for `operation resume` is Task 7's.

**Risks, for review.**

- **A resume's `discovery_pending` offers nothing.** The parent is complete by then, so `offering_resume` drops the action, and the next resume is `already_applied`. Pinned by `index_pending`. Nothing in the answer says how the hand-off gets done.
- **A child's `remote_branch_deleted` under a resume has no row.** By reading, `request.cancel` is dropped, because a resume's envelope names no operation, which would leave `repo.inspect` alone. Decision 18's release is reached through a request record's cancel.

**Left.** Task 4: made-ready in the resume's data, so the plan's "pending child … and its made-ready" row is only half done here. Task 7: `stops_on_ctrl_c`.

#### Task 3 review fixes (controller's rulings R-30 to R-36, owner's decision 19)

**R-30, a stop's actions.** `operation.resume`, then the verb's list through `outcome::offered`, less `request.retry` and `request.cancel`. Rows assert the whole list: `identity_stop_promote` and `child_identity_stop` (`operation.resume`, `repo.identity_set`); `cleanup_recovery` and `refused_checkpoint_merge_close` (`operation.resume`, `repo.inspect`).

**R-31, a consent refusal.** `Final`, `external_change`, `request.prepare`, data naming kind and item. **The replay's limit:** no actions are stored, so a replay of that request gives the same code and data and offers nothing. A later resume is `invalid_input`, with no actions.

**R-32.** `discovery_pending` under a resume offers `index.refresh` at the root.

**R-33.** `Named::Complete`. Entered again goes through `lifecycle()` and gets the stored outcome and report. A fresh resume stays `already_applied`. The row runs on a local world, which has no transport to reach, and it doesn't count connections.

**R-34.** `request cancel` of a resume answers `requested: false`, pinned while a parent is continued, from a second service in the request hook. **No probe read needed removing:** the probe and `interrupted_as_asked` read the flag of whichever request the call is under, and nothing was resume-specific. The doc is corrected.

**Decision 19.**

- **The ruling's wording would not have stopped the call, so the mechanism differs (R-36), for review.** `run.rs:512-518` clears the parent's flag on entry and continues, so `cancel_document_promotion` then one more call would not stop. The resume sets its own record's flag instead, which the probe reads. Answered `cancelled`, the binding takes it. No `promotion/` behaviour changed, and the runner is untouched, so `c5_cancel_` wasn't run. Only `TABLE` constants moved into the two `record.rs` files.
- **What it showed, both verbs.** `walk_remote_branch_deleted`: `remote_branch_deleted`, `operation.resume`, `repo.inspect`, no record, reservation free; `item republish` `ok`; resume `ok`, one merge. `kept_child_stays`: the same code and actions, reservation held.
- **The kept row marks the child by SQL during the first call,** from the credential prompt, since nothing can come between the two calls. So it is staged, as the earlier `stays_` rows are.
- **Limit: the answer does not tell released from kept.** The same `operation.resume` is a dead end for a kept child, and `repo inspect` shows the reservation.
- **The route has no action kind,** as ruled. Task 7 prints it, and Task 13b walks it.

**R-35, item 9.** Item 4 does not close it: it covers a complete parent, not a `busy` one. Against the old code, `concurrent_same_id` failed by assertion: the second call's `busy` deleted the record. Now a call entered again with a code that passes by itself leaves the record. A first call's `busy` still leaves none. Item 4 does help what follows: if the first call finishes after losing its attempt, the next resend gets the stored outcome. The row stages the first call by a death at `BeforeRequestDomainCall` and a held lease; two live processes aren't run.

**Rows answering otherwise, stopped on or settled from the design.**

- `identity_stop` on a closure past its checkpoint answered `ok`, not `identity_required`. **I changed no expectation to fit it without a source:** the design's Identity table says "After a closure checkpoint: `closed_by` comes from the commit: none needed". That row is now `identity_after_checkpoint_close`, pinning `ok`. The closure's identity stop is `child_identity_stop`, a context child that must merge. **For the controller to confirm.**

**Seen first.** 54 cases ran against Task 3's library: 17 failed by assertion and 37 passed. Failing: `grants_no_new_consent`, `entered_again`, `index_pending`, `identity_stop`, `cleanup_recovery`, `walk_remote_branch_deleted`, `kept_child_stays`, `concurrent_same_id`, `refused_checkpoint_merge`. `child_identity_stop`, written after that run, failed by assertion against that library too. `identity_stop_close`'s failure was on the code, for the reason above. **Passed at once, so pinning what exists:** `superseded_no_record`, `refused_no_record`, `before_integration_ref`, `after_integration_ref`, `ctrl_c_releases_child`, `request_cancel_asks_nothing`. `identity_after_checkpoint_close` was not run against the old library.

**Quality (item 8).** The schema's description names `operation resume`. The stale docs are corrected. `repository_id` and `resume_action` are the one helper and builder. `TABLE` is beside each record.

**Corrected from Task 3's report.** Its risks on a resume's `discovery_pending` and on a child's `remote_branch_deleted` are closed by R-32 and decision 19.

**Verified on the final code, all exit 0.** `c5_resume` 56 cases (438 s), `resume` filter 96 (885 s, detached), `mutation_replay` 384 (279 s), `mutation_contract` 44 (26 s), `read_contract` 72 (14 s), `--lib mutation` 142 (15 s), `cargo fmt --check`, clippy `-D warnings`. After those runs only comments changed (a ruling's number), and fmt and clippy were rerun. No golden changed.

### Task 4: Made-Ready Report

**Base.** `98a176a5`. `origin/main` had not moved, so nothing was merged.

**Failed and incomplete reads: the test hook.** `set_made_ready_hook_for_testing` (doc-hidden, in `bind/lifecycle.rs`) runs once, on the calling thread, immediately before the report's read. The failed read's row uses it to set a stored context's kind to a value no read can decode, so the read answers `invalid_stored_data`. The incomplete read's row uses it to store a `source` problem at `.manyhands/tickets`. I did not use the parent-table rename because the report's read doesn't touch that table, and a rename can't be undone without restoring the registry image, which would lose the settlement. The existing failure points and hooks all fire before the domain answers. A hook field on the service would need `src/repository.rs`, which is outside the library scope. (Corrected below: the stated reason for not using `cfg(test)` was wrong.) Cost: one thread-local take on the production read path.

**Rows that answered otherwise, for the controller to confirm. I did not stop on these.**

1. **A refused push.** The row first expected `current` with the set. It answered `index_pending`, because the domain returns `IndexPending` for a publication stop. No read is made. The table's own row says this, so I changed the row (`push_rejected_is_index_pending`). The same applies to a poll yield. The golden shows that a rejected deletion is `index_pending` too. So most partial runs report `index_pending`, and `current` on a partial run was seen only for a cleanup stop (a file of the owner's in the worktree). "A partial run reports it too" is narrower in practice than the design reads.
2. **A stop past integration.** I wrote two rows expecting a `Stop` past integration: a held lease, and a poll yield. The lease row did not answer `busy`; its actual code was not read. The yield answered `poll_yielding`, as an outcome with domain `IndexPending`. Both rows were removed. No world I tried reaches `stopped()` past integration.
3. **`promotion_stop_has_no_data`.** It first stopped at `AfterIntegrationRef`, which is an outcome with data, so it now stops at `BeforeIntegrationRef`. This was my fixture error.

**The design's gap, implemented and pinned, for a ruling.** A failed or incomplete read while the operation is unfinished. The table's row says `index_unavailable_final`, `Final`, and decision 13 speaks of a complete closure. Storing `Final` on an unfinished parent would contradict decision 8 and would end a request that still has work. So `Final` applies only when the answer's code is `ok`. Otherwise the report is `index_pending` and the request stays `Owed`, since its next call does read again. Pinned by `failed_read_on_partial_stays_owed`. The effects can then say discovery `current` beside `index_pending`.

**The closed ticket counts as closed in the read.** `TicketGraph::made_ready` treats T as closed whatever the index's copy says. This departs from plain `readiness_state`. The design says the context holds the closure, but the index's row for T was seen open (`closed_at` null) after a refused push. I made the change on a wrong first diagnosis of that row, whose real cause was item 1. It is kept because it is what the design's "behind changes nothing" requires, but only a unit row pins it; no end-to-end row needs it. If the index has no row for T, the read is incomplete.

**Other decisions.**

- A closure's stop with a parent now always carries data, which it didn't before. A promotion's doesn't.
- A stop with no parent and no blockers still has no data, so no `not_computed` there.
- A refusal's data says `not_computed` with `Final`, not the table's `Owed`: the answers table's standing for a refusal stands.
- `not_applicable` comes before `index_pending` in the check order, so a superseding run with domain `IndexPending` reports `not_applicable`.
- A resume's `index_pending` has nothing to follow it: the next resume is `already_applied`, so that report is never computed (R-32's limit).
- Promotion: both fields required and null in the schema, no read made, pinned.

**Seen first.** The first 24 rows ran against the DTO and schema alone: 23 failed and `promotion_reports_none` passed. Fifteen failed by assertion, one of them `promotion_stop_has_no_data` on its wrong fixture. Eight failed with no line, by a returned error and not by assertion: three local stop rows (`before_integration`, `checkpoint_only`, `resume_stop`), because a stop had no data yet, and five remote rows, because my fixture committed on the clone's primary. That fixture was fixed afterwards. The final rows then ran with the library forced to `not_computed`: 22 of 25 failed by assertion. `checkpoint_only` and both promotion rows passed, since they pin what is expected of that state. `made_ready`'s unit rows and `a_closures_stop_reports_from_its_parent` were written with the code and not seen failing.

**Not exercised.** `Close::already_contained`, the real read of the record, is run by no row. With the stop-path branch removed, `superseding_stop` still passes as far as its stop, which is an outcome. The unit row covers the branch through a stand-in verb. `stopped()`'s `Complete` arm is also unit only.

**Goldens.** 14 changed, additive only, each read. One new: `ticket_close_index_unavailable_final`.

**Verified on the final code, all exit 0.**

| Command | Result | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_made_ready` | 25 cases | 137 s |
| `--test mutation_remote -- c5_lifecycle` (with goldens updated) | 126 | 1058 s |
| `--test mutation_remote -- c5_resume` | 56 | 425 s |
| `--test mutation_remote -- c5_cancel_` | 86 | 1370 s |
| `--test mutation_replay` | 384 | 279 s |
| `--test mutation_contract` | 44 | 12 s |
| `--test read_contract` | 72 | 9 s |
| `--test read_relationships` | 17 | 5 s |
| `--lib read::graph` | 24 | 10 s |
| `--lib mutation` | 143 | 1 s |
| `cargo fmt --check`, `cargo check --all-features --locked`, clippy `-D warnings` | ok | 10, 12, 24 s |

`c5_lifecycle` ran with the update flag, so its goldens were written, not compared. `c5_cancel_` and `mutation_replay` are beyond the task's list, since a closure's stop data and the stored shape changed.

**File outside the task's list.** `src/repository/read/graph_tests.rs`, `tests/read_contract.rs`, `tests/support/mutation_golden.rs` and the fixtures, all within the scope and named edits. No `promotion/` or `closure/` file changed.

#### Task 4 review fixes (controller's rulings)

**Item 2 is open, before the controller. A row answered otherwise and I stopped on it.** `remote_context_changed`: stopped at `AfterPublication`, whose answer holds `current` with the set. Then a peer's push races the guarded delete, and the resend answers `recovery_required`, `cleanup_pending: remote_context_changed`, discovery `pending`, `made_ready: []`, `made_ready_state: index_pending`. So the state that a supersede follows holds no set, contrary to the design's Known Limits line ("the set stays in the old request's partial envelope"); only the earlier stop's answer held it. The row asserts the code and the cleanup reason and no made-ready state, and no library or design text was changed for it. Not established: an unstaged run. One try, racing before the first call, met the race at an earlier push (`transport_unavailable`, then `ok` with `current` on the resend), so it never reached this reason. A supersede after this reason was not run.

**Item 1, R-38.** The override is gone. `made_ready` uses `is_ready` and also returns whether T read closed. An outcome reports what is read. A stop with T open answers `index_pending`. `peer_reopens_the_ticket`: a peer pushes T without `closed_at`, `repo sync`, clear the file, resend. With the fix: `ok`, `current`, `[]`, and `ticket ready` does not list the dependent. With the override put back the same row failed by assertion: it stored `current` with the dependent. `repo sync` answers `already_applied` here, which is how a fast-forward with nothing to publish reads, and the row also checks that primary moved. The unit row `made_ready_never_lists_the_ticket_itself` covers finding 2. I withdraw the Task 4 entry that kept the override: it was made on a wrong diagnosis.

**Item 3.** `ParentView.integration_kind`, in `promotion/mod.rs` and `closure/mod.rs`. `already_contained`, its second read and its `unwrap_or(false)` are gone. `the_parent_view_says_how_the_operation_reached_primary` is a unit row in `closure/run_tests.rs`, on a merge and a context primary held. With the closure accessor hard-coded to `None` it failed. It tests the accessor, not the report: `AfterCompletion` and `PromotionAfterCompletionCommit` both answered `ok`, so no end-to-end world reaches `stopped()` past integration, and `stopped()` reading the field is covered through `Fixed` only. Those two rows were removed.

**Item 4.** `pending_child` asserts the child's answer has null data, and its comment says so. `poll_yielding_past_integration` asserts `index_pending` and `[]` for a closure and null for a promotion. `resume_index_pending` in `c5_made_ready` asserts the record gone, `index.refresh` offered, and the next resume `already_applied` with no data.

**Item 5.** `Tickets`' doc comment is back on it. `stop_past_integration`'s comment is corrected. `index_pending`'s text allows for a resume. **Corrected reason:** a library is never built with `cfg(test)` for any integration target, so a `cfg(test)` hook in it could not be set from `tests/`. That is why the thread-local departs from the `cfg(test)` precedent. `harness = false` had nothing to do with it. The hook stays.

**Item 6.** `MadeReadyFrom` replaces the three bools, `BeforeIntegration` included, so `self.data()` isn't built twice. New golden `ticket_close_made_ready`, with a non-empty set.

**The design** now has R-37 (an unfinished operation's failed read is `index_pending`, `Owed`; `index_unavailable_final` is a complete closure's only) and R-38, and says that most partial runs report `index_pending`.

**File beyond the plan's list, as ruled.** `promotion/mod.rs`, `closure/mod.rs`, `closure/run_tests.rs`, and the additive assertion in `tests/mutation_remote/c5_lifecycle.rs`.

**Verified on the final code, in compare mode with no `MANYHANDS_UPDATE_GOLDEN`, all exit 0.** `c5_made_ready` 27 cases (189 s), `c5_lifecycle` 126 (1032 s, detached), `c5_resume` 56 (423 s), `mutation_contract` 44 (12 s), `read_contract` 72 (7 s), `read_relationships` 17 (5 s), `--lib read::graph` 25 (10 s), `--lib mutation` 143 (2 s), `--lib the_parent_view` 1 (1 s), `cargo fmt --check`, clippy `-D warnings`. Not rerun: `c5_cancel_`, `mutation_replay`. After those runs only the order of two rows in the design's table changed.

### Task 5a: Policy Bindings

Base `9fba2036`; `origin/main` is contained, and `merge-tree` is clean.

**What landed.** `poll configure`, `poll pause` and `poll resume` with every predicate, `from_stored`, `recorded_effects` and `Replay` conformance rows. `update_remote_polling` reads the policy and writes only its own field in the caller's transaction. A binding offering `one_transaction` goes through `written_with_its_record` in `execute.rs` and `finish_in_one_transaction` in `records.rs`, which insert the record, write and finish it in one transaction under `cache_write_guard`. `stored_result` in `settle.rs` is now what both writers of a finished result use, and the "one writer" comment says so. `read/dto.rs:723` is corrected.

**Decisions the design left open, for review.**

- **Data.** The design names no data for these verbs. They report `PollPolicyMutationDto` (`poll_policy_mutation.schema.json`, new data root): the policy as the transaction left it, on success and on a no-op. It has no enum, so `read_contract` needed no row. It lets Task 8 name a suspension without a second read. A replay gives the policy as it was then.
- **Progress.** The path reports the `Writing` stage before the transaction and no `Accepted` event, since there is no accepted state to report.
- **Position.** Null, as for an index refresh: nothing is committed, and the design's primary-branch line can only apply to a command with a commit to check on replay.
- **Fault points.** Existing ones reused, no new variant: `BeforeRequestDomainCall` before the transaction and `BeforeRequestSettlement` inside it.
- **`maximum`.** The schema checker doesn't support the keyword, so the interval's schema has `minimum: 60` and states the upper bound in its description. The binding enforces it.
- **The rows' home.** The three conformance rows are in `tests/mutation_replay/remotes.rs`.

**Files beyond the plan's list, test support only.** `tests/support/mutation.rs` and `tests/support/repair.rs` gained one panic arm per exhaustive match on `MutationDataDto`; `tests/mutation_replay/remotes.rs` as above. E-9 needed the new names and one set (`unjournaled`) in `command.rs`, plus a new predicate test. E-12 needed no edit: `outcome_tests.rs` iterates `Command::ALL` and passes unchanged, so that listed file is untouched. No desktop or bin arm was needed, so the desktop gate commands weren't run separately; `--all-features` check and clippy passed.

**Seen failing.** Against a binding whose `prepare` answers `internal_error`, 22 of the first 25 rows failed by assertion, each at its own line. Three passed against it, which proves nothing of them: `unavailable_index`, `crash_before_the_transaction` and `stop_inside_the_transaction`. The last two are shown by the mutations below; `unavailable_index` was not mutated. Each mutation was run against the real code and reverted:

| Mutation | Rows that failed |
| --- | --- |
| Pause also writes `interval_seconds=300` | `stale_pause_keeps_the_newer_interval` |
| Resume also clears backoff and suspension | `resume_success` |
| Resume leaves the deadline | `resume_success` |
| A failed write commits what it wrote | `stop_inside_the_transaction`, `unregistered_after_the_check` |
| Fault points not honoured | `crash_before_the_transaction`, `stop_inside_the_transaction`, `never_committed_resend_is_a_new_write` |
| `set_remote_polling` stops writing the interval | `stale_whole_tuple_control`, the Review Focus 6 control |

Not mutated: `stale_resume_keeps_the_newer_interval`, `the_same_request_races`, and the remaining rows, which were only seen failing against the stub. `the_same_request_races` and `unregistered_after_the_check` were written after the stub run, and only the second has been seen to fail.

**Verified, compare mode, all exit 0.**

| Command | Ran | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_poll` | 27 | 17 s |
| `--test read_status` | 24 | 4 s |
| `--test read_contract` | 72 | 8 s |
| `--test mutation_contract` | 44 | 10 s |
| `--test mutation_replay` | 384 | 286 s |
| `--test mutation_confirmation` | 183 | 58 s |
| `--test mutation_bridges` | 166 | 60 s |
| `--lib mutation` | 144 | 1 s |
| `--lib remote` | 478 | 185 s |
| `--lib local_sync` | 164 | 302 s |
| `--lib integration` | 26 | 17 s |
| `cargo fmt --check`, `cargo check --all-features --locked`, clippy `-D warnings` | ok | 10, 10, 25 s |

`mutation_replay` ran before the last two `c5_poll` rows were added, which changed only `c5_poll.rs`, with no library change since.

**Limits.** A registration removed mid-call answers `repository_not_registered` from the transaction, with no recovery beyond what the read gives. `human` output for these verbs is Task 8, and `poll once` is Task 5b.

#### Task 5a review fixes (controller's list)

Tests and documents only; no library behaviour changed. Decisions 1-6 above are approved.

**Open, for a ruling: item 4's row answered otherwise, and I stopped on it.** With `remote_polling_state`'s row deleted by an acceptance hook, `poll pause` answers `internal_error`, not the `recovery_required` the review named. Established by running the row against each code: `internal_error` passes. `read_policy` does give `RepositoryErrorKind::RecoveryRequired`, but the error reaches the boundary through `From<RepositoryError> for ReadError`, whose read mapping (`read/mod.rs`) sends that kind to `InternalError`; `outcome::repository_error_code` would give `recovery_required`. Mapping it in the binding's `write` would be a behaviour change, so I made none. `policy_row_gone_inside_the_path` pins a failure, no data and `request_rows == 0`, and deliberately leaves the code unpinned until the ruling. `unregistered_after_the_check` is unaffected: that kind maps the same either way.

**Item 3, the mutation.** `write` moved before `insert_request` in `finish_in_one_transaction`, then reverted. `the_same_request_races` **catches it**, and so does the new `the_race_is_lost_to_another_verb`; the other 33 rows passed. This supersedes the note above that the first had never been seen to fail.

**The rest.** Eligibility rows `resume_makes_a_poll_due` and `resume_does_not_lift_the_suspension` answered as the review expected. `*_leaves_the_rest` stage every column and a current batch, and compare the whole `PollingStatusDto` and every other raw column of the row; only `active_operation_id` stays null, since it reads the remote journal. `unavailable_index` asserts the registry bytes and `index.rebuild`. `stop_inside_the_transaction` includes resume, and the never-committed row loops over both fault points. `request_show_and_cancel_of_a_policy_record` pins `finished`, `requested: false` and an unchanged record. The design's position line, `polling_status`'s description, the progress schema's clause, the comments and Task 0's "guards differ" entry (mode only, same lock file) are corrected. The new `*_leaves_the_rest` and eligibility rows passed the first time they ran, and no mutation was run against them, so they have not been seen to fail.

**Carried forward.**

- Task 5b: the row for a pause during a running poll (review item 12).
- D6: a policy verb sends no `Accepted` event, so `running.accepted` at `src/desktop/services/worker.rs:790` stays false for one. The desktop must not read that as "nothing was recorded".

**Verified, compare mode, all exit 0.** `--test mutation_remote -- c5_poll` 35 cases (18 s), `--test mutation_contract` 44 (10 s), `--test read_contract` 72 (8 s), `--lib mutation` 144 (8 s), `cargo fmt --check`, clippy `-D warnings` (10 s).

#### Task 5a item 4: the ruling, and a row still open

**The ruling.** `OneTransaction::write` in `bind/poll.rs` maps repository errors through `outcome::repository_error_code`, so a repository error on this path keeps its real code. Done.

**My earlier diagnosis was wrong, and this corrects it.** The section above says the missing row's `internal_error` comes from `read_policy` through the read mapping. `read_policy` is not reached. `write_records` runs `migrate_registry`, whose `state::migrate` refuses a registration with no policy row (`missing_policy`), and its `?` converts that as a read, before `write` is called. Shown by running the rows three ways:

| Build | `policy_row_gone_inside_the_path`, pinned to `recovery_required` | `policy_row_unreadable_inside_the_path` |
| --- | --- | --- |
| Without the mapping | fails | fails |
| With the mapping | **still fails**: `internal_error` | passes |
| With the mapping, `missing_policy` check removed (reverted) | passes | passes |

**Open, for a ruling.** The ruling can't move `policy_row_gone_inside_the_path`, so I stopped on it, and it pins a failure, null data and no record, with its code unpinned. The new `policy_row_unreadable_inside_the_path` stages a remote name no ref plan can be made from, which reaches `read_policy`, and pins `recovery_required`, no record and every column unchanged. It is the row that was seen to fail before the change. Pinning the first row needs one of: accept `internal_error` for it; or map the error in `write_records`, which every record function shares and so reaches beyond this path.

**Codes moved by the mapping.** On `update_remote_polling`'s two kinds: `RecoveryRequired` moves from `internal_error` to `recovery_required`. `RepositoryNotRegistered` stays, and `unregistered_after_the_check` passes. The mapping builds a plain `ReadError`, as the conversion's source and any recovery aren't read for this kind, and no other row moved.

**Verified, compare mode, all exit 0.** See the Task 5a commit's report: `--test mutation_remote -- c5_poll` 36 cases, `--lib mutation`, `cargo fmt --check`, clippy `-D warnings`.

### Task 5b: `poll once` Binding

Base `d6aee381`; `origin/main` is contained, and `merge-tree` is clean.

**Carried from Task 5a.** `policy_row_gone_inside_the_path` pins `internal_error`, by the controller's ruling, and its awaiting-a-ruling comment is gone.

**What landed.** `Command::PollOnce` with every predicate as the design's row, `Mutation::PollOnce`, `PollOnceBinding`, `Session::poll_once` on both impls, `PollMutationDto` with `poll_mutation.schema.json` (a new data root), `from_stored`, the `Remote` conformance row, and 14 goldens. `poll_publication_remote` keeps its signature and calls `poll_publication_remote_observed`, which threads an optional `CancellationProbe` to every safe point. The named rule is `final_transport_stop` in `outcome.rs`: `remote_unavailable` and `transport_unavailable` of `poll once` offer nothing unless the record was left accepted, and `replay` reaches it through the same function.

**The design checkpoint: every row answered as the frozen table says, and no mechanism changed.** Seven rows failed on the first real run. All seven were fixture faults, read and corrected without changing an expected code:

| Row | Fault |
| --- | --- |
| `interrupted`, `interrupted_after_a_target_moved`, `held_then_cancelled`, `cancelled_by_request_cancel_while_running` | The boundary hook stayed armed, so it fired on the later poll. It fires once now |
| `fenced_is_final` | A generation staged by SQL broke the next new poll. It now changes the push URL, which is a real fence. See the second decision below |
| `err_interrupted` | Changing the fetch URL under a held transport made a fetch fail, which is a transport stop. It now changes the push URL, which only the configuration check reads |
| `transport_with_no_row` | An `https` URL reserves before it fails, so it has a row and is `Final`. That is kept as `invalid_remote`, and the no-row case removes the remote in the request hook |

**Checks the brief asked for.**

- **`Final` over a pending-read `Failed` row settles.** `transport_stop_is_final` asserts the row `failed`, `JournalRow::in_flight()` true, the record finished, no reservation, a replay offering nothing with no transport, and a new request polling.
- **Report first, index pending second.** With that order swapped (M3), `interrupted_after_a_target_moved` fails.
- **A first call and a later one can differ, each with a golden.** `poll_once_key_rejected` holds `key_rejected` and null data. `poll_once_key_rejected_reentered` holds `no_selected_key` and a report.
- **A killed poll resent restarts.** `killed_resent_restarts` holds the same operation, a raised `owner_epoch` and one row. The stop is a failure point at `PollBeforeTarget` on a service then dropped, which leaves the row a kill would. A real kill is Task 13a's.
- **`operation resume` of a held poll.** Through a boundary-begun poll, it answers `original_request_required` naming the request, and that request finishes. A finally stopped poll answers `invalid_input`.
- **Pause during a running poll** (carried). The poll finishes `ok` with primary moved, `paused` stands, and the next automatic call is `NotEligible{Paused}`.

**Seen failing.** Against a binding whose `prepare` answered `internal_error`, all 42 rows then written failed, 42 of 42. The harness reports a line per case; the lines read were assertions, and not every one was read individually. `invalid_remote` was split off afterwards and was not run against the stub. The stub proves little beyond the assertions being reached, so these mutations were run against the real code, each reverted:

| Mutation | Filter | Failed |
| --- | --- | --- |
| M1 no probe at safe points | `cancelled` | `by_the_token`, `by_the_flag`, `at_the_last_safe_point`. `by_request_cancel_while_running` **passes**: `request cancel` also sets the row's flag, so it doesn't prove the probe |
| M2 no outcome rule | `stored_` | `transport_unavailable`, `protocol_rejected` |
| M3 index pending classified first | `interrupted` | `after_a_target_moved` |
| M4 the row settles a transport stop | `key_rejected` | `key_rejected`. `reentered` passes, as it should |
| M5 `rerun` without `restart` | `killed` | `killed_resent_restarts` |
| M6 `moved` always false | `interrupted_after` | that row |
| M7 no fenced answer | `fenced` | `fenced_is_final` |
| M8 no host data | `host` | `host_approval_required` |
| M9 a stored category read as `ok` | `stored_unlock` | that row |
| M10 no missing-remote pre-check | `no_remote` | `no_remote` |
| No `take_cancellation` | `cancelled_by_the_flag` | that row |

Filters ran only the named rows, so a mutation may fail others too.

**Decisions, for review.**

- **Host data on the first call.** The table's `host approve` needs an authority, so a first-call host stop carries `SynchronizationMutationDto`, as a sync's does, and `from_stored` tells the two shapes apart by `completion`. A replay or a re-entry offers no `host.approve`: no action is stored, and a category names no host.
- **A fenced row is beyond the table.** A held row whose configuration changed can never restart. `reenter` answers `fenced_answer`, `Final`, `external_change`, which is what `request cancel`'s `finish_fenced` says "its own next call" gives. A `failed` row is excepted, so its stored category stands. **The call that discovers the change first answers `recovery_required`, offering `request.retry`,** since `configure_poll_endpoints` raises the generation inside it; only the next call is final. Pinned as three steps. `request cancel` of a `failed` row left accepted with a changed generation still finishes it `external_change`, not its category. That is an edge, and I wrote no row for it.
- **`Err(Interrupted)` is exit 3, never 4.** It carries no targets. By reading, each path that returns it comes before any local move in that pass, or a safe point returned a report instead. Only `err_interrupted`'s path has a row.
- **`Err(Repository)` over a cancelled row keeps the error's code**, as the table reads, and finishes by the row. A sync says `cancelled` there. It needs a repository failure at that moment, and has no row.
- **Effects.** `discovery` is `current` on `ok`, `pending` whenever wrapped in `IndexPending`, and otherwise `not_requested`. No other effect is claimed, so `recorded_effects` is nothing for a mismatch against an unfinished poll, even one that moved a target.
- **No guidance in the data.** The table says "with guidance" and the DTO's listed fields hold none. `PollDisposition::guidance` is public, so Task 9 can print it.
- **Probe failures.** A flag that can't be read at a safe point is `Err(Repository)`, `recovery_required`, record kept (`flag_unreadable`). A cancellation that can't be recorded on the row is ignored until the next safe point. `poll_after_fetch` is now `#[cfg(test)]`, as `promote_with_io` is.
- **Progress.** A poll reports its `Connecting` event only: no stages at safe points, and no `Stopping`.

**Limits.**

- **The probe in the transfer callback has no row that proves it.** Removing it changes no answer, as the next safe point reads the same token; it only stops a transfer sooner. It also reads the registry from that callback, every 250 ms at most, which `poll_progress` opens its own connection to do.
- `remote_unavailable` has no golden. `poll_once_protocol_rejected` was written and removed, since it would have failed `CODES_NO_FIXTURE_CAN_PRODUCE`, whose reason still holds for a real fixture, and removing that is no named edit. The row asserts the code and no actions, on a staged category.
- The seven `stored_` rows stage their category by SQL over a real row. `key_rejected_reentered` is the one real path to a stored category's code. The first-call codes `transport_unavailable`, `key_rejected`, `unlock_required`, `host_approval_required`, `no_selected_key` and `invalid_remote` are real. First-call `remote_unavailable` has no row.
- A stale remote context of a closed ticket, and dirty, deleted and divergent contexts through the binary, are Task 9's. Only dirty primary and malformed are pinned here.

**Files beyond the plan's list, test support only.** `tests/read_contract.rs` (three enumeration rows), `tests/mutation_remote.rs` (the row and the array's length), `tests/support/mutation.rs` and `tests/support/repair.rs` (panic arms). `bind/mod.rs` and `bind/sync.rs` (`host_data` made `pub(super)`) are within the library scope and not named by the task. `bind/resume.rs` needed no change. No desktop or bin arm was needed, so the desktop commands weren't run.

**Named edits.** E-9 (`stopping`, and the hand-off set, whose test is renamed `only_a_folder_creation_and_a_poll_hand_off_to_any_refresh`), E-11, E-12 (`a_hand_off_no_operation_owes_is_offered_as_a_refresh`, which now expects a poll to take the refresh; `a_polls_final_transport_stop_offers_nothing` is added), E-13.

**Verified, compare mode, all exit 0.**

| Command | Ran | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_poll` | 79 | 303 s |
| `--test remote_polling` | 84 | 188 s |
| `--test mutation_remote -- resume` | 108 | 900 s |
| `--test mutation_replay` | 384 | 281 s |
| `--test mutation_contract` | 44 | 11 s |
| `--test read_contract` | 72 | 7 s |
| `--test mutation_confirmation` | 183 | 58 s |
| `--test mutation_bridges` | 166 | 57 s |
| `--lib mutation` | 146 | 7 s |
| `--lib remote` | 479 | 173 s |
| `--lib local_sync` | 164 | 287 s |
| `--lib integration` | 26 | 15 s |
| `cargo fmt --check`, `cargo check --all-features --locked`, clippy `-D warnings` | ok | 10, 3, 7 s |

**What ran after what.** The first pass of this set failed `--lib mutation` (one row, the E-12 edit above) and `--test mutation_contract` (two rows, the extra golden). After those test-only fixes, `--lib mutation`, `mutation_contract`, `read_contract`, `c5_poll_once_stored`, `c5_poll_once_conformance`, fmt and clippy were rerun. The `c5_poll` count is from before the fixes, which changed no library code and removed one golden assertion. The rest ran on the same library code.

#### Task 5b review fixes (controller's rulings)

Decisions (a), (b), (c), (e), (f) and (g) above stand. Decision (d), `Err(Repository)` over a cancelled row keeping the error as a stored answer, is overruled by item 2.

**A correction to the section above.** It says only `invalid_remote` missed the stub run. `err_interrupted`, `fenced_is_final` and `transport_with_no_row` were also rewritten after that run and not rerun against it.

**1. The transfer callback.** It no longer reads the registry. `CancellationProbe::token_asked` reads the token only, and `poll_progress` then reads the row's flag. `token_stops_a_transfer` uses `hold_transfer_after`: the token is cancelled during the transfer, and the tracking ref doesn't move. It passed before the fix too, since the old code also read the token. With the token check removed (mutated), it fails, so this limit from the section above is closed.

**2. `Err(Repository)` over a `Final` row is `Owed`.** The design's row is amended. `repository_error_over_a_cancelled_row` failed before the fix and passes after it: `internal_error`, then `cancelled`, then a replay. `repository_error_after_completion` changed for the same reason. I first expected `ok` on its re-read and it answered `discovery_pending`. No refresh ran, so that is the table's `IndexPending` row, and the expectation was mine, written this round. The one-shot failure is `PollAfterCompletion` at a safe point's capture. I found no seam inside `poll_stored_result` itself.

**3. The suspected race: no effect can duplicate, so takeover stays.**

- **By reading.** Every local effect in the target loop runs under `repository_lease`, taken before the `owned` check and held through `materialize_context_unlocked` or `poll_fast_forward_unlocked`, up to the row's write. The fetch also takes that lease to move tracking refs (`transport/remote.rs:639`). So B can raise the epoch and reset the phase while A is inside an effect, and B then can't fetch or reach a target until A is done. If B's frozen state predates A's effect, its per-target check interrupts it. If it follows A's effect, B reads `current` or `Present`.
- **By a row.** `resend_during_a_materialization`, staged in a hook at `PollAfterMaterializationIntent`, with B on its own thread. To reach that point, `check_failure` there became `poll_boundary`, which does the same check and then calls the hook. B raises the epoch and answers `internal_error`. A finishes the effect and answers `recovery_required`, having lost its row. Git holds one branch and one worktree, before and after a new request polls.
- **The fast-forward has no row of its own**, as no hook sits inside its lease. It rests on the same lease by reading.
- **A finding, and a new known limit in the design.** A poll's fetch that meets the lease held by anyone, whether a resend or a local write, reports `ProtocolFailure`. By the table that is `Err(Transport)` over a row, so the request answers `internal_error`, final, and a new request polls. That is the domain's mapping, unchanged. A's own answer then isn't what the request stores.

**4. `finish_fenced`.** It skips a poll's `failed` row, so `request cancel` is `requested` and the next call gives the stored category. `request_cancel_of_a_failed_fenced_row` failed before the fix and fails again with the skip mutated. Its generation is staged by SQL.

**5. Documents.** The design's `poll once` section records decision (a), a fenced held poll, the takeover and M6. Both schema descriptions name the new request. `PollTargetDisposition::guidance` gives the domain's words. The plan's Task 9 says to print guidance, to say "new request" on host and unlock stops, and to hint `operation list` then `operation resume` on `busy`.

**6. Stale text.** All named sites are done. Removing the `allow`s left `materialize_context` and `poll_fast_forward` used only by tests, so both are `#[cfg(test)]`, and clippy accepts the rest. `killed_then_the_remote_removed` pins the comment's correction: the plan failure fences a restart to `interrupted` with `configuration_required`, and the answer is `publication_remote_required`, `Final`, with a new request polling. It passed before any fix. **`bind/poll.rs:369` could not be matched exactly,** since the numbering didn't line up with the file. I took it to be `reenter`'s doc on a fenced row, and corrected that.

**7. Tests.**

- The `stored` rows assert empty next actions. First-call rows already did, or assert the whole list.
- `interrupted_then_one_worktree` passes: `Partial`, then a restart over one worktree. It passed before any fix.
- `busy_beside_a_lifecycle_child` passes, with the child's row unchanged. It passed before any fix.
- **S1 was wrong, by reading, and is fixed.** `moved` read dispositions and OIDs, so a worktree that appeared under `Refused(MaterializationIncomplete)` on a branch that already existed read as unmoved, where the domain's `changed` says otherwise. The report now carries the executor's own `local_changes`, excluded from equality, since a stored report has no executor and `remote_polling` compares a replay with the original. **No row reaches that refusal,** since no seam makes the validation fail after the worktree is added. `interrupted_after_a_target_moved` fails with the flag mutated false, which shows the new source is used, not that the refused path is right.
- **S2 is confirmed, and is a known limit in the design.** `fences_a_lifecycle_child`: with the push URL changed and a close's child holding the reservation, the poll answers `ok` and the child's row ends `interrupted`. It's the order in the domain, `configure_poll_endpoints` before the reservation. I did not trace what the close then answers.
- **M6** is left as it is, and recorded in the design.

**Verified on the final tree, compare mode, whole filters, all exit 0.**

| Command | Ran | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_poll` | 87 | 359 s |
| `--test remote_polling` | 84 | 187 s |
| `--test mutation_remote -- resume` | 108 | 894 s |
| `--test mutation_replay` | 384 | 281 s |
| `--test mutation_contract` | 44 | 12 s |
| `--test read_contract` | 72 | 13 s |
| `--test mutation_confirmation` | 183 | 56 s |
| `--test mutation_bridges` | 166 | 59 s |
| `--lib mutation` | 146 | 30 s |
| `--lib remote` | 479 | 176 s |
| `--lib local_sync` | 164 | 295 s |
| `--lib integration` | 26 | 16 s |
| `cargo fmt --check`, `cargo check --all-features --locked`, clippy `-D warnings` | ok | 11, 9, 9 s |

Only this ledger and a ticket comment changed after that run.

### Task 6: Identity And Remote Selection Fixes (Steps 1-5)

**Not complete. Three rulings are needed, listed under Blockers.** The code is committed in two commits, identity then index, so each ruling can be applied alone. No pre-existing test was edited. `--lib promotion` and `--test repository_enablement` fail until blockers 2 and 3 are ruled, so the Part A gate can't pass yet.

**Reproduction of `01M5MK57`, before the fix, measured by temporary probes.**

| Path | Identity | Answer before the fix | What it left |
| --- | --- | --- | --- |
| Synchronization | Bracketed, configured | `RecoveryRequired` | No commit |
| Synchronization | Trimmed, configured | **Success** | A merge signed `Example Person`, not the configured `Example Person,`, and pushed |
| Synchronization | Bracketed, confirmed | `RecoveryRequired` | **The identity written to local configuration**, confirmation `applied` |
| Promotion | Bracketed, configured | Repository unavailable | Record stuck at `context_synchronized` |
| Promotion | Trimmed, configured | **Success** | Merge signed with the altered name |
| Closure | Bracketed, configured | Preview `Ready`, then `reconfirmation_required` | Row `abandoned` |
| Closure | Trimmed, configured | Preview `Ready`, then **success** | Merge signed with the altered name |

**Does the ticket's reasoning hold?** Yes for the bracketed and confirmed cases. It says the commit fails, which is **incomplete**: a trimmed identity doesn't fail, it silently commits and publishes under a name other than the configured one, so that case is worse than the ticket describes. Its open question on durable effects before the refusal is answered by blocker 1.

**Reproduction of `01M669FJR2`.** It matches the ticket and Task 0. Eight new rows failed on the stale index before the fix.

**Seen failing before the fix** (rows as first written).

| Target | Failed | Passed before the fix |
| --- | --- | --- |
| `remote_synchronization -- c5_identity` | 4 of 5 | Plain control |
| `document_promotion -- c5_identity` | 4 of 5 | Plain control |
| `ticket_closure -- c5_identity` | 3 of 5 | Plain control, kept records |
| `mutation_remote -- c5_lifecycle_identity` | 11 of 20 | The rest, which pin behaviour that already held, including the R-21 race row, since a bracketed identity already ended `abandoned` |
| `repository_enablement`, new rows | 8 of 8 | None |

Two corrections to those counts. `identity_child_stop_continues` (2 of the 11) failed on an expectation of mine that was wrong, see decision (c), and not on the defect. `identity_prepare_missing_promote` also had a double removal in its fixture, since fixed; its assertion on the code comes first, and I didn't separately confirm where it stopped before the fix. `the_synchronization_after_a_selection_...` was rewritten after the fix, with a stale index as its control, which answers `worktree_not_clean` in the same row. The final form wasn't run against the unfixed code.

**The fix.** `select_committing_identity` applies `commits_unchanged` to both identities. Promotion's prepare check is in `bind/lifecycle.rs`, reached only for a ready preview planning a merge commit. The domain's check is `Lifecycle::require_identity`, a no-op default that only promotion overrides, called with no prior record once the preview is unchanged. `publish_configuration_index` runs after the commit and on the already-selected branch, and takes no lock when the entry is current. `FailurePoint::AfterPublicationConfigurationCommit` is the one new variant (decision 17).

**Decisions.**

- (a) **A failed index update is an error**, unlike enablement's `let _ =`. Ignoring it would report success with the ticket's stale index. The row is pending, and the same request heals it.
- (b) **Two commits, not the plan's one**, so a ruling can be applied to either alone.
- (c) **A context child's identity stop continues, and then ends `external_change`.** The design says the same request continues. It does: the stopped child row finishes. But the child merges a peer's commit the preview never showed. A control in the same world with no identity stop answers `external_change` too, for both verbs, so the stop isn't the cause. The row pins this. A merging child can't end `ok` in this world, so the design's row may want that sentence.
- (d) **A stale confirmation on a promotion with no prior record** is now refused with no record, where it used to begin a record. It follows from `require_identity` using the selector's whole answer.
- (e) `effective_identity`'s per-key cascade stays (R-6). **A follow-up ticket is needed and has not been created.**
- (f) **Decision 10 overrides `01M669FJR2`'s "merged on its own" bullet:** the fix ships inside C5. It is recorded here and on this ticket's comment. Neither that ticket nor `01M5MK57` has been edited: both live on their own branches, and R-21's note is also unrecorded there.

**Blockers.**

1. **Synchronization's refusal can't leave refs and journal unchanged.** The plan, R-21 and the ticket ask for "journal unchanged" and "no unfinished row" on synchronization. The selector runs only once a merge is known to be needed, which is after the fetch. Measured on the fixed code with a bracketed identity: the answer is `IdentityRequired`, `refs/remotes/origin/main` moved, a new operation row stands in `fetch_observed` with one prepared step, and the same ID without `restart` answers `recovery_required`. Before the fix, a missing identity left the same shape, and the ticket asks for "as when no identity is configured", so the two acceptance lines conflict. The committed rows assert what holds: configuration, local primary, pushes and confirmation rows unchanged, and they say in a comment what they leave out. The strict row is not committed. A check before the fetch would refuse fast-forward synchronizations that need no identity, so I didn't add one. Ruling needed: narrow the ticket for synchronization, as R-21 did for closure, or ask for a different mechanism.
2. **`tests/repository_enablement.rs:3816` pins the defect.** `assert_recovery...` asserts the index bytes are unchanged after a retry that commits the selection, which contradicts `01M669FJR2`. It fails `recovery_before_publication_configuration_commit_restores_config_and_live_index` and `enablement_rollback_and_recovery_with_symlink_parent_use_canonical_roots`. It isn't a named edit. Ruling needed: a new named edit, which would assert the configuration is current and the other entries unchanged.
3. **`promotion::run_tests::identity_and_mismatch_do_not_strand_the_item` pins the record R-3 removes.** With no identity, its first call is now refused with no record, so its wrong-digest call on the same ID no longer answers `operation_mismatch`. It isn't a named edit, and wasn't in Task 6's verify list: an extra `--lib promotion` run found it. Ruling needed: a new named edit.

**Risks.**

- **Conflict resolution isn't covered.** `resolve_synchronization` reads `repository.signature()` and doesn't use the selector, so by reading a trimmed identity is still altered there and a bracketed one still answers `RecoveryRequired`. Not run. It is outside the design's fix and the ticket's acceptance lines, though the ticket names that request as reachable.
- **`repo sync` in the library row** answers `internal_error` past the clean check, where the binary answered `no_selected_key` in Task 0. Not investigated. The row asserts only that it isn't refused as not clean and that publication was requested.
- **No golden** was added for a promotion's `identity_required`, since the golden support files are outside Task 6's Files.

**Verified.**

| Command | Result | Time |
| --- | --- | --- |
| `--test remote_synchronization -- identity` | 5 passed | 52 s |
| `--test document_promotion -- identity` | 5 passed | 27 s |
| `--test ticket_closure -- identity` | 6 passed | 30 s |
| `--test mutation_remote -- c5_lifecycle_identity` | 20 passed | 83 s |
| `--test repository_enablement` | **101 passed, 2 failed** (blocker 2) | 18 s |
| `--lib remote` | 479 passed | 200 s |
| `--lib local_sync` | 164 passed | 299 s |
| `--lib integration` | 26 passed | 15 s |
| `--lib mutation` | 146 passed | 2 s |
| `--lib promotion` (beyond the plan) | **204 passed, 1 failed** (blocker 3) | 136 s |
| `--lib closure` (beyond the plan) | 200 passed | 79 s |
| `--test document_promotion` (beyond the plan) | 79 passed | 794 s |
| `--test mutation_remote -- c5_lifecycle` | 145 passed | 1098 s |
| `--test mutation_replay -- remote` (beyond the plan) | 38 passed | under 300 s |
| `cargo fmt --check`, `cargo check --all-features --locked`, clippy `-D warnings` | ok | |

After that set, only a test-file lint fix and a comment changed. The three `c5_identity` filters, fmt and clippy were rerun on the final tree, and passed 5, 5 and 5. `--test ticket_closure` as a whole, `mutation_replay` as a whole and the Part A gate were not run.

#### Task 6: the owner's rulings (decisions 21-24)

The three blockers above are settled. Each is in the Cycle's decisions, with the design and the plan amended. No row answered otherwise on the final code, and nothing stopped this round.

- **Decision 21.** The rows stand, and their comment now states the rule. `c5_identity_same_id_recovers` is new: refused, the same ID without `restart` answers `recovery_required`, and with a usable identity its restart merges and completes.
- **Decision 22.** E-18 asserts the index matches the commit. E-19 takes a stop at `AfterConfirmed` before the identity goes, so its mismatch has a real record. The rest of that test is unchanged.
- **Decision 23.** I read `resolve_synchronization` and did not route it through the selector, since its confirmation is bound to its observation's digest, not the selector's. It applies `commits_unchanged` to both identities before any write. Anything unusable answers `IdentityRequired`, as a confirmation bound to another observation does there. **Correction to the risk above:** a bracketed configured identity already answered `IdentityRequired`, because `repository.signature()` fails and was read as none; that row and the control passed before the fix. The trimmed and confirmed rows failed before it.
- **Decision 24.** `fetch_poll_set_noting_lease` reports a held lease in a `Cell`, so no public error kind changed and no exhaustive match grew. The domain then interrupts its row and returns `RemotePollError::Busy`. **What the row says:** `busy`, outcome `error`, `request.retry`, record `accepted`, row `interrupted` with no stored outcome, so no failure replays and no backoff grows. The same request completes under the same operation, and a replay is identical. A cancel still comes first, at the safe point before it. `c5_poll_once_resend_during_a_materialization` changed with the ruling: B answers `busy`, and the same request then finishes where it used to answer `internal_error` for good. It is C5's own row. **Synchronization shares the defect by reading:** its fetch passes the same line, so a held lease answers `internal_error`. I left it alone, since its row's retry isn't what this ruling covers, and no row pins it.

**The `internal_error` risk: not a defect.** `tests/repository_enablement` never initializes the transport's runtime, unlike the binary and the SSH harness, so `transport/operation.rs:83-85` answers `RuntimeUninitialized`, which maps to `internal_error`. With the runtime initialized the same journey answers `no_selected_key`: `c5_lifecycle_remote_select_then_sync_finds_no_key`. The enablement row keeps its stale-index control and says why it can't name the final code.

**Also done.** The design's sentence on a child's identity stop ending `external_change`. A promotion's `identity_required` golden, `document_promote_identity_required`, added now rather than in Task 7, read before commit. The two-commit split is recorded in the plan. The design's "recorded on that ticket" now points at Task 15's closure, and those branches were not edited.

**Verified on the final tree, all exit 0.**

| Command | Ran | Time |
| --- | --- | --- |
| `--test remote_synchronization -- identity` | 6 | 69 s |
| `--test document_promotion` | 79 | 766 s |
| `--test ticket_closure` | 78 | 686 s |
| `--test mutation_remote -- c5_lifecycle_identity` | 20 | 83 s |
| `--test mutation_remote -- c5_poll` | 88 | 359 s |
| `--test remote_polling` | 84 | 184 s |
| `--test repository_enablement` | 103 | 17 s |
| `--test remote_merge_recovery` | 29 | 237 s |
| `--test mutation_remote -- conflict` | 41 | 399 s |
| `--test mutation_contract` | 44 | 11 s |
| `--lib promotion` | 205 | 130 s |
| `--lib closure` | 200 | 74 s |
| `--lib remote` | 479 | 171 s |
| `--lib local_sync` | 164 | 279 s |
| `--lib integration` | 26 | 16 s |
| `--lib mutation` | 146 | 1 s |
| `--test mutation_remote -- c5_lifecycle_remote_select` | 1 | |
| `cargo fmt --check`, `cargo check --all-features --locked`, clippy `-D warnings` | ok | |

`--test ssh_transport -- poll` passed 9 after the transport edit. `--test mutation_remote -- c5_lifecycle` as a whole was last run before this round's library changes, with 145 passing. The Part A gate is the controller's.

### Part A checkpoint

**Gate at `d2fe86e3`.** All gate commands exit 0: check, fmt, clippy, desktop check and clippy, CLI check, no-GPUI. `--lib` 1506 passed in 722 s; the `--bin`, cli, contract, replay, confirmation and bridges line in 469 s; read_status, read_relationships, repository_enablement in 26 s; mutation_remote, document_promotion, ticket_closure, remote_polling, remote_synchronization and cli_remote in 8691 s. merge-tree clean.

**Decision 25.** `parent_effects` reads the child's durable state, and `recorded_effects` reads it through that. Two named rows failed by assertion first: `c5_cancel_child_merged_then_refused_push` (at `outcome == partial`) and `c5_cancel_child_push_landed_then_cancelled` (at `publication == published`). Made-ready now reads the parent's step, since a context's merge reports `integration: complete`.

- **`cancelled` stays exit 130, not 4.** The decision says "it exits 4", and the second row it names is `cancelled`. RFC:982 gives a cancellation 130 "even after a durable effect", `failure_class` puts it first, and `Outcome::Cancelled` pins it past integration. The row reports the push and answers 130. **For the owner to confirm.**
- One golden moved, `ticket_close_checkpoint_merge_refused`, read in its diff: `publication: published`, `commit_oid` set, `discovery: pending`, from the context's push. The Cycle's other goldens did not.
- `unfinished_continues` said "nothing durable yet" of a stop after the context's push. It is C5's own row and the decision overrides it; it now asserts `partial`.

**Decision 26: the row passed on the unchanged code.** Names `Example Person,`, `<Example Person>` and ` Example Person` all answer `identity_required`, reason `identity_required`, with no owner, no durable effect, tip and repository image unchanged, and the same operation then merges under a usable identity. So nothing is altered: `validate_identity` already applies `commits_unchanged` (`src/repository.rs:10837`). **No library change was needed**, and the review's premise doesn't hold. The scope is widened as ruled, and `run.rs` is unchanged. `resolve.rs:346` calls the same `identity`, by reading; no row. The named filters ran as part of the whole targets below.

**Decision 27.** One verb, each noting why: `prepare_not_enabled`, `prepare_not_registered`, `busy_with_no_parent`, `busy_lease_on_reentry`, `busy_with_a_standing_parent`, the three `parent_unreadable_*`. `busy_with_no_parent` reaches each verb's own `begin_record`, whose `busy` for both is `c5_cancel_new_id_before_integration`. **Kept both:** `poll_yielding_past_integration`, which branches on `made_ready`.

**Timing.** `World::new` measured, twice alike: server start 25 ms, key encryption 222 ms, first clone 285 ms, service 37 ms, enable 71 ms, configuration commit and push 330 ms, second clone 284 ms, key registration 19 ms, privacy 71 ms: 1.34 s. `remote()` adds trust 290 ms, primary sync 2.85 s and save 115 ms. So `World::new` is not the cost: unlocking is, about 2.6 s a connection. A plain-key world (`World::plain_key`), for rows that don't test unlock or stop a child from inside its credentials: `c5_cancel_stage_after_publication` 31.4 s to 13.7 s for two rows, `c5_lifecycle_published` 30.8 s to 13.2 s, `c5_cancel_new_id_before` 31.4 s to 13.8 s. All C5 rows of `mutation_remote`, 406 at that run: 1899 s. Gate line 4: 8691 s to 7143 s. `--lib` 722 s to 799 s and the second line 469 s to 496 s are load: neither changed.

**Spec review's fixes, as ruled.**

- `once_fences_a_lifecycle_child` resends the close: `ok`, one merge, once the URL and the deleted branch are back. A branch left deleted answers its own stop, unrelated to the fence.
- Stuck cases pinned: `c5_cancel_fenced_released_resolution_close`, whose unfenced twin publishes, and `c5_cancel_fenced_past_publication_close`, by `run.rs:1188`.
- **The ruling's retained case answered otherwise, and needs confirming.** A poll doesn't fence a retained conflict: it checks `has_pending_conflict` before `configure_poll_endpoints` (`remote/poll.rs`), answers `recovery_required`, and the generation stays. `c5_cancel_poll_leaves_a_retained_conflict_close` pins that, and the design's limit says so instead of the ruling's wording. **Corrected by the Part A re-review, below:** other routes do raise the generation over a retained child.
- Supersede after cancel: `c5_cancel_supersede_after_a_cancelled_answer` and `_straight_after_the_cancel`, both as ruled, with the design's sentence.
- Known limits added: a live call losing its stop when the flag is cleared on a kept child; `cancelled` after a guarded deletion that went through, read and confirmed, `transport/remote.rs:240` answering `deleted` for a branch already gone.
- The primary `Err` arm's row (`promotion/run.rs:1139-1145`) is owed to Task 13a, whose step names it. Task 13b gains a released resolution with its branch deleted, which stops for the owner if no route frees it.
- **Follow-up candidates, left out of C5:** synchronization's lease as `internal_error` is a ticket candidate, not filed. Decision 24's `interrupted` row for automatic polls is a D6 note: nothing restarts one.
- Plan's scope gains `remote/reservation.rs` (decision 18) and `local_sync/run.rs` (decision 26); `transport/remote.rs` was there. Resume's row and the Cycle's status corrected.

**Quality review's fixes, no behaviour change.** All of items 1-4, 6 and 7 and the minors. Rule numbers are dropped from new `src` comments, except two wrapped ones this missed, "(decision 18)" at `promotion/run.rs:793-794` and "(decision 11)" at `remote/state.rs:6410`, removed in the re-review's corrections below; test comments keep theirs. `lifecycle_parent` is generic on `ParentRecord`, with a `written_checkpoint` method. Duplicate helpers matched but for one comment and `golden_at`, whose general form made-ready now calls with its schema. `poll once`'s test sleep names `POLL_PROGRESS_THROTTLE` in its comment, since the constant isn't reachable from a test target.

**Verified on the final code, compare mode, before merging main.** All exit 0.

| Command | Ran | Time |
| --- | --- | --- |
| check, fmt, clippy, desktop check, desktop clippy, CLI check, no-GPUI | ok | under 25 s each |
| `--lib` | 1507 | 799 s |
| `--bin`, cli, contract, replay, confirmation, bridges | 252, 350, 44, 384, 183, 166, with read_contract 72 | 496 s |
| read_status, read_relationships, repository_enablement | 17, 24, 103 | 27 s |
| mutation_remote, document_promotion, ticket_closure, remote_polling, remote_synchronization, cli_remote | 664, 79, 78, 84, 69, 66 | 7143 s |

**After that, `origin/main` moved** to `15a23125` (packaging, release scripts, `src/main.rs`, `src/startup_reporting.rs`), and was merged without conflict. Rerun after the merge: the seven static commands and `--test mutation_contract` (44). **The test lines were not rerun on the merge.**

## Task 7: CLI lifecycle verbs

Base `9832423f`. `document promote` and `ticket close` are available: `lifecycle.rs` through `mutate::drive`, inside the Ctrl-C window, with `human_lifecycle.rs` for text. Neither reads the item first, since the library answers an unknown ID and a wrong kind.

**Seen first.** Against the unchanged binary, 15 of the 16 rows of `tests/cli/lifecycle.rs` failed by assertion on `command_unavailable`, and `the_window_covers...` on `false != true`. The 16th, the resume row below, passed, since it pins existing code. With a stub renderer, 8 of the 11 `lifecycle_tests` rows failed by assertion. The other three pin C2's fallback, its passphrase hint and blockers named by the field list. The filter `--test cli -- lifecycle` also matches one row of `ticket_authoring`, hence 17 below.

**Corrected rows: three of my expectations, none a row of the design.**

- A never-enabled repository is `repository_not_enabled`, exit 2. I had written `repository_not_registered`, 3. `prepare_not_enabled` and `results.rs:550` pin the answer.
- `confirmation_mismatch` is exit 2, not 3 (`results.rs:588`).
- The made-ready fixture used `depends_on:`. The key is `deps:`.

**Deviations, for a ruling.**

- **`collaboration.rs` is beyond the plan's Files.** It gains one arm in `run`, so that an `operation resume` with lifecycle data renders through `human_lifecycle`. The task's scope names this rendering and no listed file can hold it: `render.rs` is mounted by `tests/cli_remote.rs` without `human_lifecycle`. Dropping the arm leaves the C4 lines, whose `remote_branch_deleted` note is wrong for a lifecycle.
- **E-16 in part, earlier than its task.** `startup_tests.rs:171` fails once `ticket close` has a handler, so it moved now, as E-4's rows did. `parse_tests.rs:563` still passes and is left.
- **The planned rows moved to `poll once`,** still planned, for E-3, E-4 and E-6. Task 9 moves them to the test-only spec. E-1 needed no edit: `DESIGN` already holds both under C5, and its context verbs are Task 10's.
- `human.rs` and `render.rs` needed no change. The preview's effects and slug and a prepare's blockers already render through `human_admin`, and rows pin them.

**Evidence that isn't through the binary.** A published `ok`, a child's deleted branch and its resume route need SSH, and `tests/cli_remote.rs` is not this task's. Their text is pinned literally in `lifecycle_tests.rs`, on built envelopes, and the JSON by the library's goldens. Tasks 12a and 13b owe the binary walk. No `cli_v1` golden is a published `ok`: `*_local_only` is this task's `ok` and publication-pending golden in one.

**The four context verbs arrive in Task 10.** The refused merge prints them now, and they answer `usage_error` until then. The text is pinned by `ticket_close_checkpoint_merge_refused_text` and a unit row, and Task 10's grammar answers to it: `<new-request-id>` and `<sync-operation-id>` are placeholders for the caller to fill.

**Findings.**

- **`should_retry` takes the refused merge.** It is `partial` with `request.retry` first, so a run makes four attempts and prints `progress: retrying` three times before the answer, about 0.7 s, and JSON mode prints nothing. The golden pins it. Any stop of that shape is the same, and a retry can't change this one's answer.
- **`collaboration.rs:259` degrades quietly.** An `operation resume` naming a parent a request holds answers `original_request_required`, exit 3, naming the request. The conflict read finds nothing under a parent's ID and adds nothing: no note and no failure. This passed on unchanged code.
- **The sentinel scan reads every table,** wider than the design's list. `closed_by` and the title are only in `discovered_items`, in both verbs. The preview's JSON holds `closed_by` for a closure and no title.
- **A blocked prepare naming `integration_conflict` has no repair lines.** It goes through `render::prepare_output`, as all prepares do. Only a run's answer prints them, which is the row the design names. The same goes for `context_missing`'s save hint.
- **`progress: connecting` prints in a local-only run.** That is the library's event, left alone.

**Verified, compare mode, no golden variable.** All exit 0.

| Command | Ran | Time |
| --- | --- | --- |
| `--bin manyhands-cli` | 263 | 2 s |
| `--test cli -- lifecycle` | 17 | 6 s |
| `--test cli -- help` | 9 | under 1 s |
| `--test cli -- boundaries` | 4 | 3 s |
| `--test cli -- exits` | 31 | 16 s |
| `--test cli`, whole | 366 | 79 s |
| fmt, clippy `-D warnings`, `cargo check --all-features`, CLI check, no-GPUI | ok | under 10 s each |

Goldens were written with `MANYHANDS_UPDATE_GOLDEN=1` and read. Help's four moved only for the two verbs. `unavailable_ticket_close.json` and its registration are gone (E-2). merge-tree against `origin/main` at `15a23125` is clean.

**Left.** PTY consent, Ctrl-C through the binary and lost stdout: Task 13a. Replay and same-request continuation over SSH: Task 12a. The recovery table's row for the refused merge: Task 14a.

**Rulings on Task 7's two points.** The coordinator approved both: the `collaboration.rs` arm, now in the plan's Files, and the early move of `startup_tests.rs:171`.

### Part A re-review's corrections

- **Generation raises over a retained child.** The design's limit is corrected, with the routes it names. `key clear` is added beside `key select`, since the row's fence goes through both and `keys/registry.rs:557` is the clear.
- **Two rows, and where they answered otherwise than the review's text.**
  - `c5_cancel_key_select_fences_a_retained_conflict_close`: the generation goes from 1 to 3, and the resend answers `recovery_required`, offering `request.retry`, `repo.inspect`, with its record accepted, on every send.
  - `c5_cancel_poll_plan_failure_fences_a_retained_conflict_close`: **`poll once` doesn't reach `remote/poll.rs:1293`.** Its binding answers `invalid_configuration`, and the generation stays. Called directly, `poll_publication_remote` errs and raises it from 1 to 2, and the resend is as above. So this route is the domain's, and automatic polls', and not the CLI's.
  - **A new ID is not `busy` in either row.** A new prepare answers `recovery_required`, blocked, with no confirmation, so the design doesn't say `busy`. A confirmation issued before the conflict, presented under a new ID, was not tried.
  - **These rows don't show what the fence costs.** An unfenced retained conflict answers the same (`c5_cancel_poll_leaves_a_retained_conflict_close`). The difference would be in `conflict resolve`, which neither row runs. Task 13b's stop-for-owner check now names both.
- **Decision 25's limits** are in the design: the current attempt's child only, and the over-claim after a kill between `push_verified` and its authority.
- **`REPORTS_MADE_READY`** also gates the slug in `preview`, and its comment now says so.
- **`World::plain_key`** no longer encrypts a key it won't write.
- **`Command` stays over the old `Verb`** in resume's `lifecycle`. Its `_ => InternalError` arm is unreachable, since `lifecycle_standing` names only the two commands, and its comment says so.

### Task 7 review's fixes (controller's rulings)

**Seen first.** Against stubs that kept the old renderer and driver: 7 unit rows and 3 binary rows failed by assertion. Passing at once: the mixed-blocker and other-blocked prepare rows, which pin that nothing is added, the control-character row, and the sentinel rows, whose positive control held. `a_missing_context_names_the_save_that_makes_one` was written with the fix, so it wasn't seen to fail. No row answered otherwise.

1. **The refused merge's route** prints only for a ticket with `recording: record` and `request.retry` or `operation.resume` offered. Under a resume it says to resume it again, and prints the resume's command. This supersedes Task 7's "any integration conflict".
2. **A blocked prepare offers its way out,** through a new `Driven::Prepared`, so `render.rs` is untouched. This supersedes Task 7's risk about prepares. **My reading of "where that route applies":** `integration_conflict` for either verb, since with no closure row `sync-local` isn't limited to tickets, and not beside a dirty or conflicted context, a pending conflict or a local operation. That is from reading `local_sync/record.rs`; Task 10b's walk covers only the closure's route.
3. **Honest wording:** "the local repair to try". Task 10b gains the walk and the exact strings, and drops "to try" if it holds.
4. **Sentinel scan:** positive controls for each expected sentinel, after prepare, after completion and after the `Refused` stop. At that stop the control is the title only, since no index copy of `closed_by` is promised before integration.
5. **Driver:** a lifecycle answer naming blockers isn't retried, which supersedes Task 7's finding. Other bounded repeats stay, and `Pending{Synchronization(Rejected)}` may push up to four times, as the design now says.
6. **Note:** placeholders, `--input -` and when show and resolve apply. A `next:` line stays a command that can be pasted, so "add --request-id unless you run it at a terminal" is in the note beside it, not on the resume line.
7. **Smaller items.** A control-character row for the root and slug. `human_collaboration::republish` is `pub(crate)` and reused. The closure in `lifecycle.rs` is named. A human no-op row, `document_promote_noop_text`. `Item id:` and `Slug:` stay apart, because the field list is sorted. The missing slug on a result's `Item id:` is in Known Limits for the owner. Task 14a's step now names `document promote`, and `help_document_promote` is added.

**Follow-up candidate:** `progress: connecting` in a local-only run comes from `command.rs:490`, whose `stage` gives `Connecting` for both verbs whatever the repository has.

**Part A's corrections, at `2a9bc6f3`, went beyond the brief in one place:** a second wrapped rule number, "(decision 11)" at `remote/state.rs:6410`, was also removed.

## Task 8: CLI policy verbs

Base `08ac45ca`; `origin/main` at `15a23125` is contained. `poll configure`, `poll pause` and `poll resume` are available through `polling.rs`, with `poll_configure.schema.json` and its example. Each answer renders by C2's lines, with one added note.

**Seen first.** Against the unchanged binary, all 18 rows of `tests/cli/polling.rs` and `input_schemas::poll_configure_schema_is_closed_and_its_example_is_accepted` failed by assertion, on `command_unavailable` or its exit, each read. No expected code was changed afterwards. Two faults of my own, neither in an expected code: a `request show` and a `poll status` under the held lease answered `busy`, so those reads moved after its release; and the text cases' registration missed by a bad edit. With the note's condition mutated false, `a_suspension_is_named_from_the_answer_s_own_data`, `resume_names_a_suspension_it_does_not_lift` and `a_suspension_is_named_on_a_noop_and_by_the_other_verbs` failed, and the mutation is reverted. `no_suspension_has_no_note` and `json_mode_holds_one_envelope_and_no_note` pin an absence, and weren't seen to fail.

**The renderer and progress.** `progress: writing` is the one stage, and no `accepted` appears. `HumanProgress` renders what the library sends, so no change was needed. A no-op prints it too, since the stage comes before the transaction. Goldens pin both.

**Decisions, for review.**

- **The suspension note prints for all three verbs, and on a no-op.** The design names `poll resume`. The data holds the fact on any policy answer, and the note is true on each; narrowing it would add a condition with no gain. It reads only the answer's data, and a unit row shows that on a built envelope.
- **Options carry `--prepare` and `--confirm`, as every mutation's do.** `parse::mode_rules` calls `get_flag("prepare")`, which panics for an option the spec doesn't declare, and `parse.rs` isn't in the Files. So `--prepare` answers `not_confirmable`, exit 2, as `item sync`'s does. The design's list for `poll once` reads `--repo`, `--request-id`, `--json`; Task 9 follows the same convention, and Task 14a's inventory should read it so.
- **The interval's upper bound stays with the library**, as Task 5a's schema note says. The schema has `minimum: 60` and states 3600 in its description. An input of 3601 passes the CLI's type check and gets `invalid_input` from the library, with no detail line, as the golden shows.
- **"Same-request continuation" and "lost stdout" for a one-transaction verb.** There is no accepted state. Continuation is shown two ways through the binary: a refused `invalid_input` under R1, then R1 corrected, is a new write; and a `busy` with the lease held by a second process, through all bounded retries, then the same request, which writes. Lost stdout is a closed pipe: the process exits non-zero, the write stands, and the same request replays the stored answer, which a direct column change after it shows was not rewritten.
- **A policy `busy` leaves no record.** `request show` answers `request_not_found` after the release. The binary's rows pin it.

**Files beyond the plan's list, each harmless.** `src/bin/manyhands-cli/input.rs` and `input_tests.rs`: `InputObject`'s fields are private and no reader took a number, so `whole_number` is added, with its row. `tests/cli/input_schemas.rs`: the schema's row, as its verify names that filter. `tests/support/cli.rs`: registration, which the Global Constraints name. `tests/fixtures/cli_v1/help_root*`: regenerate for the three verbs only, read in the diff.

**Planned rows.** No row names these three as planned: Task 7 already moved the planned rows to `poll once`, which still answers them, so the move here is the help goldens. `tests/cli/help.rs` and the four help goldens of the poll verbs are Task 9's.

**Verified, compare mode, no golden variable.** All exit 0.

| Command | Ran | Time |
| --- | --- | --- |
| `--bin manyhands-cli` | 275 | 2 s |
| `--test cli -- polling` | 18 | 3 s |
| `--test cli -- input_schemas` | 8 | 1 s |
| `--test cli`, whole | 390 | 91 s |
| `cargo fmt --check`, clippy `-D warnings` | ok | under 10 s each |

**Left.** `poll once` is Task 9, with `stops_on_ctrl_c`'s rows for all four verbs.

## Task 9: `poll once` verb

Base `1343462b`; `origin/main` at `15a23125` is contained. `poll once` is available: through `mutate::drive`, inside the Ctrl-C window (`stops_on_ctrl_c`), with C2's unlock prompt and bounded retry. Nothing in the catalog is planned now. `planned()` is `#[cfg(test)]`, `Availability::Planned` has `cfg_attr(not(test), allow(dead_code))`, and `catalog::PLANNED` is the test-only spec (R-8).

**Human output.** The report's fields, then a line per target with `PollTargetDisposition::guidance`: `target: <name> kind= item= disposition= before= after=`. Null fields print `none`, and every value goes through `render::field`. Final stops add a note that names a new request, for the code's correction. A `busy` adds `next:` lines for `operation list` and then `operation resume`, after C2's `request.retry`, with a note on what they answer.

**Seen first.** Against the planned verb, 13 of the 14 `c5_polling_` cases failed by assertion: the harness printed `json_once`'s code assertion (line 93) for 12, and `c5_polling_usage_failure`'s (line 700) for the other. The 14th, `c5_polling_a_second_poll_overlaps`, failed with no line, since its first process exited at once and the server's hold was never reached. So it failed, but not by an assertion. A stub of the renderer with no target lines, final notes or busy route, and C2's passphrase hint, failed 10 of the 14 `polling` unit rows. The four it passed pin the policy note or an absence: `a_suspension_is_named_from_the_answer_s_own_data`, `a_stop_the_same_request_continues_has_no_final_note`, `json_mode_holds_one_envelope_and_no_note` and `no_suspension_has_no_note`. With `PollOnce` out of `stops_on_ctrl_c`, `the_window_covers_exactly_the_stoppable_verbs` failed. Each mutation is reverted.

**No design row answered otherwise. Two of my own expectations did, and I corrected them, each on a reading.**

- `c5_polling_a_second_poll_overlaps` expected `poll_yielding` from a real second process while the first is inside its transfer. It answers `busy`, exit 5, with `request.retry` and its own operation ID, and the first process finishes `ok`. Decision 24 says "poll overlap answers `busy`", so my row was wrong. `poll_yielding` stays with the library's staged reservation (`c5_poll_once_poll_yielding`), and I did not establish whether a real overlap can reach it.
- `c5_polling_fast_forward_and_materialize_once` expected only `progress: connecting`. A poll has an accepted record, so `progress: accepted` comes first. Task 5b's "reports its `Connecting` event only" speaks of a poll's own stages. A golden pins both lines.

**The closed ticket's disposition, read before the row ran.** `materialized`, in the closing clone and in the other. The only closure check in materialization is `written_closure_unfinished` (`remote/poll.rs:325`), which a finished closure doesn't meet. So a stale peer's branch of a closed ticket gets a local branch and a worktree again, while primary keeps its closure and nothing else moves. **For the owner:** this is what the design said to pin, but a person may find it surprising, and it leaves a worktree that nothing retires.

**Decisions, for review.**

- **A rule for which stops say "new request".** `final_note` is an explicit code list, excluding any answer that offers `request.retry`, so an `Owed` stop doesn't say it. No note says "final" or that the same request only repeats, since a no-remote answer has no record, so its ID would run again. A replay of a stored stop prints the note too.
- **C2's passphrase hint is replaced for this verb.** "run the command again" would imply the same request under `--request-id`. The hint says a new request. Its row is a unit row: the PTY route with a wrong passphrase isn't run for `poll once`.
- **Options, as Task 8 records.** `--prepare` is `not_confirmable`, by `mode_rules`.
- **The no-remote stage.** `c5_polling_no_remote` removes the `publication_remote` line from the root's `.manyhands/config.toml` on disk, uncommitted, and shows `publication_remote_required`, exit 3, in both modes: no request record, no new connection and the same refs. It doesn't run `remote select`, so what that verb does with a null name is not shown here.
- **The lease row holds the lock in the test's own process,** and the CLI child is the second process, as the library's row does. `c5_polling_a_second_poll_overlaps` has two real CLI processes.
- **`poll_once_host_stop_text`'s port and fingerprint** are replaced in the golden, since the fixture makes them per run.

**A pin not in the named edits, in a file the brief names: for a ruling.** `tests/cli/help.rs` `root_help_and_bare_call_match_in_both_modes_without_data` asserted that help's `repository` values include null, which only a planned verb gives. With R-8 none does, so `.chain([None])` is removed. No code changed for it, and the schema still allows null, for a later planned verb.

**Named edits.** E-3, E-4, E-6 and E-16 are moved, including `parse_tests.rs:563`'s planned half and `startup_tests.rs:171`'s row; `exits.rs` loses the `unavailable` row, and `command_unavailable` joins `REASONS`. E-14: four `stops_on_ctrl_c` rows. E-7: help goldens, read.

**Files beyond the plan's list.**

- `src/bin/manyhands-cli/parse.rs`: `verb_command` is extracted from `command()`, with no change in behaviour, so a planned spec's arguments can be tested without a catalog entry. `parse()`'s `Availability::Planned => Action::Unavailable` arm therefore has no row: nothing in the catalog reaches it.
- `src/bin/manyhands-cli/human_lifecycle.rs`: `refusal_of` is `pub(crate)`, one word.
- `src/bin/manyhands-cli/dispatch.rs`: the arm.
- `tests/cli/help.rs`, `tests/cli/exits.rs`: as the brief names. `tests/support/cli.rs`: registration.

**Evidence that isn't through the binary.** The text of `cancelled`, a stored transport stop, a wrong passphrase and a stop with a report is on built envelopes. `transport_unavailable` and `remote_unavailable` over SSH aren't run through the binary. Replay and mismatch are pinned on a completed poll only. The matrix's Ctrl-C, `request cancel`, same-request continuation and lost stdout for `poll once` are Task 13a's, though `busy` then the same request is here.

**Transport.** `startup.rs` starts the Git transport for every `Run` of a verb past its mode rules, so the three policy verbs start it too, as C2's do. The design says transport starts only for `poll once`. It spawns no thread, and changing it is beyond these Files. **Risk to rule on.**

**Verified, compare mode, no golden variable.** All exit 0.

| Command | Ran | Time |
| --- | --- | --- |
| `--test cli_remote -- c5_polling` | 14 | 58 s |
| `--bin manyhands-cli` | 288 | 2 s |
| `--test cli` | 392 | 92 s |
| `--test cli_remote`, whole | 80 SSH cases and the target's other rows | 369 s |
| fmt, clippy `-D warnings`, CLI check, no-GPUI | ok | under 10 s each |

No library file changed, so the `remote`, `local_sync` and `integration` filters were not run. merge-tree isn't needed: `origin/main` is contained.

**Left.** Task 13a's Ctrl-C and lost stdout for `poll once`; Task 14a's inventory, which should read `--prepare` and `--confirm` as listed; `tests/cli/inventory.rs`.

## Task 10a: context reads and `context sync-local`

Base `0efc7dfb`; `origin/main` is contained. `context observe-local`, `context show-local` and `context sync-local` are available, the last inside the Ctrl-C window. `resolve-local` is 10b's: until then the conflict stop's `next:` line for it answers `usage_error`.

**Seen first.** `--test cli -- local_sync input_schemas` against no verb: 37 of 38 rows failed, by assertion on the code (`usage_error`), or for the help row on a missing golden. `a_verb_without_its_options_is_a_usage_error` passed: it pins what parsing gives, so it says nothing of these verbs. The remote-operation row first failed on a fixture fault of mine, a phase the table's `CHECK` refuses, and was corrected; it then failed by assertion before the verb's reading was enabled, in the control below.

**Rows the library's reads answer otherwise, and the binary answers as the table.** A control run, with the bin's two readings disabled, shows each:

| Row | Library's read | Binary |
| --- | --- | --- |
| `show-local`, unknown operation | `recovery_required` | `operation_not_found`, by `show_operation` first |
| `show-local`, remote or other operation | `recovery_required` | `invalid_input`, by `show_operation`'s action and family |
| `show-local --token`, tampered, foreign or stale | `external_change` | `invalid_input`, by the inspection's path list, before any side read |

No library file changed. A genuine `recovery_required` or `external_change` of a real local sync still passes through. This is my reading of "the table is the contract", not a ruling: the table's codes are met by the verb, and the library's own answer is unchanged.

**`observe-local` with no context (Task 0's open row).** `item_not_found`, exit 2, as the read reports. `show_item` finds the item first, so only this case prints `next: manyhands-cli ticket save ...` (or `document save`); an unknown ID prints nothing. No recovery kind names a save and none may be added, so JSON's `recovery` is empty and JSON alone can't tell the two apart.

**Stages that aren't the binary's.** `discovery_pending` and a closure past `Closing` (`AfterIntegrationRef`) are stopped in the test's own process, as C3's rows record of the same seam, because a CLI child has no failure point. So `discovery_pending`'s exit 4 is shown by the library's `FailureClass::Incomplete`, not read from a child, and the binary's same request then finishes with no second merge.

**Other decisions.**

- **Human notes.** `identity_required` says the same request follows, because the data has no `required_request_id` to offer it. The repair-outside note prints only for unrelated histories, a closure disagreement or an unsupported conflict, not a closure's or a lost authority's `external_repair`.
- **Input.** `{observation}`, closed. The schema has no `minLength` (the checked keywords refuse it), and an empty string is the library's `invalid_input`.
- **`--prepare`** is `not_confirmable`, by `mode_rules`, as the poll verbs.
- **`busy`** from a held `CacheWrite` carries no `request_id` or scope, and offers no retry. That is the library's answer before acceptance, and differs from `poll_once_busy`, whose lease is met later. The golden pins it as given.
- **Help goldens** live in `tests/cli/local_sync.rs`, so `tests/cli/help.rs` is untouched. `help_root` and `help_root_text` gain the three verbs and the `context` resource, and those diffs were read.

**Files beyond the plan's list.** `tests/support/cli.rs` (golden registration, as the Global Constraints say) and `tests/cli/input_schemas.rs` (the schema's own row, as Task 8 added). `tests/cli/help.rs` and library files are untouched. No existing test's meaning changed; E-1 and E-14 are the named rows.

**Risks.**

- The kind is read before the library looks up the request, so a replay after the item has gone from the index answers `item_not_found`, not the stored answer. Not staged.
- `human_lifecycle.rs`'s comment on `REPAIR` says these verbs answer `usage_error`, which is now true of `resolve-local` only. 10b changes that text with the walk.

**Verification.** A compare run after every golden diff was read: `--bin manyhands-cli` 293, `--test cli -- local_sync` 38 in 13 s, `--test cli -- input_schemas` 9, plus `help` 13, `boundaries` 4 and `exits` 31, since the catalog changed. `fmt --check`, whole-tree clippy with `-D warnings`, `cargo check --bin manyhands-cli` and the no-GPUI script pass. The checkpoint gate's other targets are not run here.

## Decision 28: the latest copy is authoritative

Base `5c318b51`; `origin/main` at `15a23125` is contained. The owner's "Go with A" (2026-11-02), now decision 28 in the Cycle.

**What landed.** In `materialize_context_unlocked` (`remote/poll.rs`), a fresh materialization (`MaterializationAction::Start`, no intent) of a branch whose tip primary already holds, as the local branch or its tracking ref, is refused as `PollDisposition::Current`. Nothing is written for it: no intent row, no branch, no worktree. The check follows the closure and conformance refusals, so a nonconforming tree still says so, and it comes before every effect. A started materialization goes on. A branch with a commit primary lacks, diverged or not, materializes as before.

**The disposition: `current`, reused.** Its guidance, "Local state is current.", is true: nothing that branch holds is missing here. A new disposition would change the `remote_poll_targets` CHECK constraint (`state.rs:320`), an audited column, which the Global Constraints forbid. So no schema, `read_contract` row or enum golden changed.

**Seen first.**
- `c5_poll_once_a_closed_ticket_s_older_branch_is_current` failed at its disposition assertion before the change.
- The control `c5_poll_once_a_branch_primary_lacks_still_materializes` passed before and after, as a control should.
- The CLI's closed-ticket case was rewritten after the change. Its old golden (`materialized`) failed against the new answer, and was regenerated and read: `current`, `after_oid` null.

**A row answered otherwise, and I resolved it without an edit.** With the check first placed before the conformance check, `--lib remote`'s `poll_materialization_nonconforming_tree_or_unrelated_operation_creates_nothing` answered `Refused(Current)` for a nonconforming tree at an older primary commit. Rather than edit a pre-existing test, I moved the check after the conformance refusal. The row then passes unchanged, and the new rows still hold.

**Rows.**
- `c5_poll_once_a_closed_ticket_s_older_branch_is_current` (library): a close whose remote deletion is refused leaves its pushed branch, which primary contains. A second clone, registered with the same service, polls: `current`, no branch, no worktree, primary keeps the closure, no push. A second poll is the same.
- `c5_poll_once_a_branch_primary_lacks_still_materializes`: the control.
- `c5_polling_remote_context_of_a_closed_ticket` (CLI): a dependent ticket is put on primary first. After the close and a stale branch at the pre-closure tip, both clones poll `current` with no branch or worktree. `ticket show` reads `closed`, and `ticket ready` lists the dependent and not the ticket. `ticket save` of the closed ticket answers `ticket_closed`, exit 3, writing nothing. Pushes are counted before and after, and are unchanged.

**S2: `item sync` of a closed ticket's copy, pinned in two forms.**
- `c5_polling_item_sync_of_a_closed_ticket_s_old_copy`: B materialized before A's close deleted the branch. `item sync` answers `remote_branch_deleted`, exit 3, publication pending, offering `request.cancel`. Nothing is pushed, and B's copy stays. `item republish` was not tried, and would push the old copy back.
- `c5_polling_item_sync_of_a_copy_the_remote_still_holds`: the re-materialized form. B holds the context, and A's close had its deletion refused, so the remote holds B's tip. `item sync` answers `ok`, with publication `published`. It does **not** publish the stale copy: it takes B's context up to primary, which holds the closure, and pushes that, once. The remote branch then holds the closure, and stays. B's own primary isn't moved.

**Documents.** Decision 28 is in the Cycle, quoting "Go with A". The design's `poll once` paragraph names it. Its Known Limit now says a clone that materialized such a branch before isn't repaired, with what `item sync` then does, and that a branch with a commit primary lacks isn't covered.

**Follow-up candidates, not to be opened without the owner's go-ahead.**
- **B:** reads take the latest copy by Git history across the copies of an item, primary and every context, so that a stale context can't be what a read shows.
- **C:** ticket reopening. It needs an RFC change, since the MVP RFC excludes reopening and closure immutability is enforced (`ticket save` of a closed ticket answers `ticket_closed`).

**Limits.**
- A context whose tip equals a primary commit and holds no commit of its own isn't materialized. A save always commits, so no Manyhands route publishes one, but a branch pushed by hand at a primary commit is now `current`.
- An unreadable ancestry reads as "not held", so the branch materializes as before.

**Verified, compare mode.** All exit 0 but one, and that one predates this change.

| Command | Ran | Time |
| --- | --- | --- |
| `--test mutation_remote -- c5_poll` | 90 | 232 s |
| `--test remote_polling` | 84 | 179 s |
| `--test read_contract` | 72 | 7 s |
| `--test mutation_contract` | 43 of 44 | 11 s |
| `--test cli_remote -- c5_polling` | 16 | 80 s |
| `--test cli` | 431 | 107 s |
| `--bin manyhands-cli` | 293 | 3 s |
| `--lib remote` | 479 | 172 s |
| `--lib local_sync` | 165 | 287 s |
| `--lib integration` | 26 | 16 s |
| fmt, clippy `-D warnings`, no-GPUI | ok | |

**`mutation_contract`'s failure predates this change.** `every_result_code_has_a_golden_or_a_reason_no_fixture_can_produce_it` says `command_unavailable` has neither a golden nor a reason. Its only golden, `unavailable_ticket_close.json`, went in Task 7 (E-2, `56477a7f`), and no verify list since has run this target. The review-fix commit adds the reason, as E-6 did for the exit table.

After the clippy fix, which changed one test line, `c5_poll_once_a_` was rerun.

## Tasks 8-9 review fixes (controller's rulings)

Base `744b7b5d`. No library file changed in this commit.

**Seen first.** Against the unchanged renderer, these failed by assertion:
- `a_final_stop_says_a_new_request_and_never_this_one`, with the four added codes;
- `the_unlock_and_host_stops_say_what_comes_first`, with "approved or replaced";
- `busy_hints_operation_list_and_then_operation_resume`, with the new words;
- `the_interval_s_bounds_hold_and_nothing_is_written_beyond_them`, on the range detail.

Some rows passed at once, as they pin what existed: the `--confirm` row (`not_confirmable`), the `600.0` row and the closed-pipe lost-stdout row. So did `a_recovery_with_a_way_on_has_no_final_note`, which pins an absence.

1. **`final_note`.** `HostReplacementRequired` shares the host note, now "once the host is approved or replaced". `recovery_required` (for `HostVerificationUnavailable`), `index_unavailable` and `internal_error` get the new-request note when Final. The envelope carries no standing, so Final is read as: no `request.retry` and no `repo.inspect` offered. A conflict or a pending local operation, which offers `repo.inspect`, gets no note, and a row pins both exclusions.
2. —
3. **`busy` note.** It adds "If it names only this request, wait and send this request again (add --request-id unless you run it at a terminal)."
4. Decision 28's commit.
5. **Lost stdout.** The pipe's read end is closed before spawn (`std::io::pipe`, then the child's `status()`), so no write to stdout can succeed.
6. **The overlap row.**
   - A mutation run, `RemotePollError::Busy => PollYielding` in `bind/poll.rs`, failed this row at its `busy` assertion, and also failed `c5_polling_a_second_process_holds_the_lease`. It is reverted.
   - The hold's 10 s wait is fixed in `ssh_remote.rs`, and the helper takes no other, so it wasn't raised.
   - A `Reaped` guard kills and reaps the first process if the case stops before collecting it.
7. **`whole_number`** is `number.as_u64().ok_or(..)`, and the CLI row has `600.0`.
8. **An out-of-range interval** in human mode adds "Input field interval_seconds must be from 60 to 3600." The CLI checks only the type, so the library's `invalid_input` for `poll configure` is the range. JSON is unchanged. Golden: `poll_configure_out_of_range_text`.
9. **`--prepare` and `--confirm` for Task 14a and the RFC.** All four poll verbs list both options, as every C2 mutation does, because `parse::mode_rules` reads `prepare`. The design's option list for `poll once` (`--repo`, `--request-id`, `--json`) is therefore not the catalog's. Task 14a's inventory and the RFC's grammar table should list them, with `--prepare` answering `not_confirmable` and `--confirm` the same, exit 2. Rows: `pause --prepare`, `configure --prepare`, `pause --confirm`, and `poll once --prepare` over SSH.
10. **The human replay after host approval** says a new request takes it, and prints no `host.approve`.
11. **Helpers.** `selected` and `plan()` serve all four verbs. `refusal_of` moved to `render.rs`. `nothing_but_a_fetch`'s doc names what it compares: the server's refs, the clone's branches and worktrees, and accepted pushes, not tracking refs.
12. —
13. **`repository_not_registered`** is covered for all three policy verbs by the library's `c5_poll_unregistered`, so no CLI row was added.

**Also fixed: a failure that predates this work.** `tests/mutation_contract.rs`'s `CODES_NO_FIXTURE_CAN_PRODUCE` gains `command_unavailable`, with R-8's reason. Its only golden left in Task 7 (E-2, `56477a7f`). This is a reason row, as E-6 is for the exit table, and not a named edit: **for a ruling.**

**Goldens regenerated and read:** `poll_once_busy_text` and `poll_once_host_stop_text`, for the new words only, and the new `poll_configure_out_of_range_text`.

**Verified, compare mode.** All exit 0.

| Command | Ran | Time |
| --- | --- | --- |
| `--bin manyhands-cli` | 294 | 4 s |
| `--test cli` | 431 | 105 s |
| `--test cli_remote -- c5_polling` | 16 | 80 s |
| `--test mutation_contract` | 44 | 9 s |
| fmt, clippy `-D warnings`, no-GPUI | ok | |

The library filters, `remote_polling`, `read_contract` and `c5_poll` ran on decision 28's commit, and no library file has changed since.

### Task 10a review's follow-ups (controller's rulings)

The verb-side answers for an unknown operation, another operation and a bad token are accepted.

- **S1. The save hint** prints only when the item's effective copy is primary's (`item.context.kind` is `primary`), as lifecycle keys its note on `ContextMissing`. It also stays off for a lifecycle-closed ticket, since `ticket save` refuses one (`bind/ticket.rs` `refuse_closed`); that is my addition, and the closed-ticket row asserts it. New row: a context whose item file was removed and committed gets no save hint. Its read also offers `index.refresh`, which is the library's.
- **C1.** The `conflict show` hint keys on `SynchronizeContext` or `SynchronizePrimary`, not the remote family. The remote row now seeds a synchronization: `insert_remote_synchronization` plus the `remote_integration_windows` row a synchronization is read with. Without that row `show_operation` answers `internal_error`. A poll's ID answers `invalid_input` with no hint.
- **C2. The token check** refuses with `invalid_input` only where `conflict::read` refuses too: the inspection lists paths and not this token, or the operation has finished and lists none. A listed token, readable or not, and any state that lists no paths while it goes on, go to the library. New rows: a binary file's listed token, `content_available: false`, answers `external_change`, as the library's read does in the test's process. A conflict seeded to `recovery_required` lists no paths, and the library's read gives the sides, and so does the binary. Control: with 10a's check both rows failed by assertion (`invalid_input`, against `external_change` and `ok`).
- **C4.** Added goldens and text runs for the three refusals with no owner (`context_sync_local_unrelated`, `_closures`, `_unsupported`), and `context_sync_local_discovery_pending` (the library's envelope). New rows: `sync-local` on an item with no context (`item_not_found`, nothing created, no request), and `{"observation":""}` (the library's `invalid_input`). The walk test's docstring is corrected. `nothing_here_contacts_a_remote_or_moves_primary` now also reads a conflict's sides with `--token`.
  - **The repair-outside note** now keys on the reason alone. The library offers `next_action: none` for unrelated histories and a closure disagreement, and the table's "External repair" holds for them too. This is human text only.
  - **`LocalSyncBeforeSideRead` is not armed.** A CLI child can't take a failure point: the binary opens its service with `open_at`, and no environment variable or flag sets one. Review Focus 5's evidence stays the refusals with no side read and the positive control.
- **C5.** `collaboration::authoring_kind` replaces the local copy. `human_admin::fields_without` is shared with `polling.rs`. The save line and the note's first sentence come from `human_lifecycle` (`save_line`, `NO_CONTEXT_NOTE`). `sides` is a plain renderer for the JSON-only path.
- **C6.** `5c318b51` also carried E-20, the transport sentence and the closed-ticket Known Limit. "No transport start" for 10a means no remote contact, shown by the listener test, and not that `startup.rs` skips the transport's start.
- **Design.** Known Limits gains the replay after the item leaves the index and the `busy` with no request ID. Named edits gain E-21.

## Task 10b: `context resolve-local`

Base `76c746fc`, the follow-ups' commit; `origin/main` is contained. `context resolve-local` is available, with `--input {observation, resolutions: [{token, source}]}` (closed schema, with an example), inside the Ctrl-C window.

**Seen first.** Against no verb: 8 resolve rows failed by assertion on `usage_error`. The help row and the abandon row failed on a missing golden; abandon is an existing verb, and its code assertion held before the golden. The schema's row failed on `usage_error` too. The walk failed at its fourth step, `resolve-local` (`unrecognized subcommand`, exit 2), after observe, sync and show had answered.

**The walk holds.** `the_refused_merges_route_finishes_the_close` runs the route through the binary:
- **The refusal.** A closure stops past its checkpoint (`AfterClosureCheckpoint`, in this process) and primary moves the ticket. The binary's `ticket close` resend refuses the checkpoint's merge (exit 4).
- **The four steps.** They are run from the four `next:` lines it prints, split as a shell would, with only `<new-request-id>` and `<sync-operation-id>` filled:
  - observe-local, exit 0;
  - sync-local, with the printed observation as `{observation}`: a conflict, exit 4, whose `Operation id:` fills the placeholder;
  - show-local, exit 0, giving the token and the resolution observation;
  - resolve-local, with the local side (the checkpoint, read by `--token --json`) as the source: `ok`.
- **The finish.** The same `ticket close` request then answers `ok`, `integration: merged`. Primary's ticket keeps the checkpoint's `closed_at` and `closed_by`.
- **"One merge", my reading.** Primary has two merge commits: the local resolution's checkpoint `[context, primary]` ("Resolve local context conflicts"), and the closure's one integration ("Integrate closed ticket ..."). The close itself merges once. The row asserts both subjects in that order. If "one merge" meant one merge commit on primary in all, this route can't give it, since the conflict's resolution is itself a merge.
- **The note.** `REPAIR` drops "to try", and its comment and the module comment now name the walk. That changes three text goldens (both prepare-conflict notes share `REPAIR`) and two `lifecycle_tests.rs` pins of the same string, as Step 2 says. The walk covers the closure's route only; the prepare route uses the same four verbs.

**Two rows answer otherwise, and wait for a ruling.** Each is an `#[ignore]` row that asserts the table and fails by assertion when run with `--ignored`. Neither is fitted, and no library file changed.

| Row | Table | Binary |
| --- | --- | --- |
| `resolve-local`, an unknown token (well formed, tampered) | `invalid_input`, 2 | `external_change`, 3 (`reason: foreign_change`), nothing written |
| `resolve-local`, a new request after a partial accepted resolution | `original_request_required`, 4 | `external_change`, 3, with the original observation or a fresh one from `show-local` |

In the partial case, `original_request_required` comes from `operation resume`. Its outcome is `blocked`, so it exits 3, and it offers `request.retry` naming the original request. `show-local` also names that request (`reason: original_resolution_required`, `required_request_id`). The original request with its sources then finishes, which `a_partial_accepted_resolution_is_finished_by_the_same_request_and_source` pins. The table has no row for the resume. A verb-side token check like `show-local`'s would meet the first row; I didn't add one, since the accepted ruling named `show-local` only.

**Other rows.**
- **Complete set:** `ok`, exit 0. The replay gives the stored answer and no write. Another source under the same request is `request_mismatch`.
- **Before any write:** an incomplete set, a duplicate token, an invalid source and an empty list are each `invalid_input`, exit 2, with `next: ... context show-local`, and the files and state are unchanged.
- **Stale:** a guide edited after the inspection is `external_change`, exit 3, with the show-local line.
- **CLI input failures:** `invalid_input`, with no request recorded.
- **`operation abandon`** of a pending conflict prepares, and its confirm is `recovery_required`. The conflict's files and state are kept: the gate's guarded abandonment.
- **Source sentinel:** absent from every table but `discovered_items` (the resolved title) and from every output, in the complete, partial and invalid rows. The resolved file holds it.

**Files beyond the plan's list.** `tests/support/cli.rs` (registration), `tests/cli/input_schemas.rs` (the schema's row), `src/bin/manyhands-cli/mutate.rs`, `collaboration_tests.rs`, `catalog_tests.rs` (`stops_on_ctrl_c`, E-14, E-1), `lifecycle_tests.rs` (the `REPAIR` pin), `tests/cli/lifecycle.rs` untouched.

**Verification.**

| Command | Count | Time |
| --- | --- | --- |
| `--bin manyhands-cli` | 295 | 2.6 s |
| `--test cli`, whole | 446 pass, 2 ignored | 104 s |
| `--test mutation_contract` | 44 | 9.5 s |
| `--test mutation_replay -- local_sync` | 75 | 124 s |
| `--test cli -- input_schemas` | 10 | 1 s |

`fmt --check`, whole-tree clippy with `-D warnings` and the no-GPUI script pass. `merge-tree` against `origin/main` is clean.

### Task 10b: the owner's rulings

1. **"One merge" means the close merges once.** The resolution's checkpoint merge is the repair. The walk's assertion stands: primary holds the resolution's merge and the closure's one integration. The plan's Task 10b wording now says so.
2. **An unknown token on `resolve-local`** is `invalid_input`, exit 2, before any write. A verb-side check matches `show-local`'s: it uses `refused_unread`, so it refuses only where `conflict::read` would. With no inspection, the library judges every token. A request the library already holds a record of is left to it, so a replay or mismatch isn't pre-empted; the complete-set row's replay failed without that and passes with it. The row is no longer ignored, and has a text golden.
3. **A new request after a partial accepted resolution** is the library's `external_change`, exit 3. That follows the gate (Local operation 5–6, owner decision 4): the original request is the one to resend. The design's row is corrected. Human output names the original request as `next: request.retry request_id=<original>` with a note. The ID comes from the data's `required_request_id`, or, after an `external_change` or `original_request_required` that names none, from the inspection's (`reason: original_resolution_required`). JSON keeps the library's envelope. The row covers the sent observation and a re-read one, then the pinned finish by the original request, with JSON and text goldens.

The resolve schema's example row expected `external_change`. Its made-up token now meets the verb's check, so the row expects `invalid_input`, and asserts the refusal is the check's (the `show-local` line) and not the input reading's.

No `#[ignore]` remains. Verification: `--test cli` whole 448 (112 s), `--bin manyhands-cli` 297 (2.4 s), `--test mutation_replay -- local_sync` 75 (124 s), `fmt --check` and whole-tree clippy with `-D warnings` pass.

## Task 11: real content

Step 1 only, run 2026-11-02 UTC. The binary is `target/debug/manyhands-cli`, built once at `bcdb609a`
(`cargo build --locked --bin manyhands-cli`, already current). No source, test or other document
changed. No subagent was dispatched.

### Isolation

- **Temp root.** `mktemp -d` under `/tmp`. Its `HOME`, `GIT_CONFIG_GLOBAL` (a new, empty file),
  `GIT_CONFIG_NOSYSTEM=1`, `MANYHANDS_DATA_DIR`, and `XDG_CONFIG_HOME`, `XDG_DATA_HOME` and
  `XDG_STATE_HOME` were all inside it. `SSH_AUTH_SOCK`, `SSH_AGENT_PID`, `GIT_SSH` and
  `GIT_SSH_COMMAND` were unset. The root was removed at the end, after the fixture `sshd` was stopped.
- **The clone.** `git clone --no-local` of the main checkout, then a read-only `git fetch` of that
  checkout's `refs/remotes/origin/main` (`15a23125`), so that the clone is current main and not the
  checkout's older local `main` (`a52273ff`). Its `origin` was removed at once. The main checkout, its
  registration, the real application data and the real `origin` were not written; afterwards the
  checkout's `main` is still `a52273ff`, and its status shows only the untracked worktrees directory it had before.
- **Identity.** Repository-local `user.name "Manyhands Dogfood"`,
  `user.email dogfood@example.invalid`. Nothing global.
- **The remote: real SSH, no real key.** A bare repository in the temp root, seeded with the clone's
  `main`, served by an unprivileged OpenSSH `sshd` on `127.0.0.1:22422`. It had a host key made for the run, an
  `AuthorizedKeysFile` in the temp root holding only the key `key generate` made (protection `none`),
  no password or agent, and a `ForceCommand` that runs `git-shell` under `env -i`. The clone's
  `origin` was `ssh://<user>@127.0.0.1:22422<temp>/remote.git`. The key and host were taken through
  the CLI's own verbs: `key generate`, `key public`, `key select`, `host observe`, `host approve`. A
  throwaway probe key checked the fixture once before the CLI key existed, and was then deleted.
  `sshd` logged 87 accepted logins. **So `poll once` and every publication ran networked.** Tasks 12a
  and 12b are still needed for the harness's refusal and fault rows.
- **A peer.** A plain-Git clone of the bare repository (by path), with its own local identity
  (`Dogfood Peer`), pushed the remote changes the poll and divergence cases need. It ran no Manyhands
  command.

### Real content

Current main holds 174 `docs/` Markdown files, and 38 of them have no front matter (`docs/design/`,
plus three plans). `document list` lists all 174, and gives those 38 `missing_front_matter` problems
with `id: null`. `index status` reports `problem_count` 40, and is `current` throughout. All 32
tickets on main are lifecycle-closed. No verb refused because of a nonconforming file, including a
`poll once` that fast-forwarded over a peer's edit to `docs/design/README.md`. No merge (promote,
close, both `sync-local` merges, the resolution) refused or reported a problem because of them. Their
bytes after each merge weren't compared.
C4's `comment add` failure is not repeated by any C5 verb.

### Setup (not C5 verbs)

| Step | Exit | Code |
| --- | --- | --- |
| `repo enable --prepare` with no `origin` (main's `config.toml` names `publication_remote = "origin"`) | 2 | `invalid_remote` |
| The same, after `git remote add origin ssh://…` | 0 | `ok` |
| `repo enable --confirm` | 0 | `ok` |
| `key generate` (`protection: none`) | 0 | `ok` |
| `key select --repo <root> --prepare` / `--confirm` | 2 | `usage_error`: `key select` takes no `--repo`, and its help lists none. Run from inside the clone instead |
| `key select --prepare`, from inside the clone | 2 | `not_confirmable` |
| `key select`, from inside the clone | 0 | `ok` |
| `host observe`, before the key was selected / after | 3 / 0 | `no_selected_key` / `ok` |
| `host approve --prepare` / `--confirm` | 0 / 0 | `ok` / `ok` |
| `repo sync`, to create `refs/remotes/origin/main` | 0 | `already_applied` |
| `document create` (one), `ticket create` (two: T, and D depending on T) | 0 | `ok`, local only |
| `ticket show` / `ticket save`, used to make local edits | 0 | `ok` |

### C5 verbs

Exit and code are what the binary printed: JSON `code`, or for human runs the `code:` line on stderr.
"Real" means an item from main's own content; "throwaway" means one made in the clone.

| Verb | Case | Exit | Code |
| --- | --- | --- | --- |
| `poll status` | fresh registration, JSON and human | 0 | `ok` |
| `document promote --prepare` | real document, before any fetch (no `origin/main` tracking ref) | 3 | `recovery_required`, blockers `context_missing`, `primary_missing` |
| `ticket close --prepare` | real closed ticket (C4's), before any fetch | 3 | `recovery_required`, blockers `context_missing`, `primary_missing` |
| `document promote --prepare` | real document, after `repo sync` | 3 | `recovery_required`, `[context_missing]`; human prints the `document save` line, then `repo.inspect` |
| `ticket close --prepare` | real closed ticket, after `repo sync` | 3 | `ticket_closed`, `[already_closed]` |
| `document promote --prepare` | a ticket's ID (wrong kind) | 2 | `item_not_found` |
| `ticket close --prepare` | a document's ID (wrong kind) | 2 | `item_not_found` |
| `context observe-local` | real document with no context | 2 | `item_not_found`, with the `document save` hint |
| `context observe-local` | real closed ticket with no context | 2 | `item_not_found`, no hint (closed) |
| `context observe-local` | unknown ID | 2 | `item_not_found` |
| `context show-local` | unknown operation ID | 2 | `operation_not_found` |
| `document promote --prepare` | throwaway, JSON and human | 0 | `ok` |
| `ticket close --prepare` | throwaway T, JSON and human | 0 | `ok`, `closure.recording: record`, `closed_by` the local identity, slug shown |
| `document promote --confirm` | throwaway | 0 | `ok`: merged (`d54e9d48`, two parents), published to the bare remote, cleanup `complete` |
| `document promote` | the same request again (replay), JSON and human | 0 | `ok`, JSON byte-identical |
| `document promote --prepare` | the promoted document | 3 | `recovery_required`, `[context_missing, primary_ahead_of_remote]` |
| `ticket close --prepare` | throwaway T, right after the promotion | 3 | `recovery_required`, `[primary_ahead_of_remote]` (see Blockers 1) |
| `poll configure` | `600` / the same request again / a new request, same value / the same request, `30` | 0 / 0 / 0 / 2 | `ok` / `ok` (replay) / `already_applied` / `request_mismatch` |
| `poll configure` | `30`, new request, human | 2 | `invalid_input`, "Input field interval_seconds must be from 60 to 3600." |
| `poll pause` | first / again | 0 / 0 | `ok` / `already_applied` |
| `poll pause --prepare` | | 2 | `not_confirmable` |
| `poll resume` | paused, human / not paused | 0 / 0 | `ok` / `already_applied` |
| `poll once` | **while paused**; a peer had pushed `main` and a ticket branch | 0 | `ok`: `main` `fast_forwarded`, the peer ticket `materialized` (one branch, one worktree), the two local unpublished tickets `history_unknown` |
| `poll once` | the same request again, JSON and human | 0 | `ok`, JSON byte-identical |
| `poll resume` | after that poll | 0 | `ok`, `next_eligible_at` cleared |
| `ticket close --prepare` | throwaway T, after the poll | 0 | `ok` |
| `ticket close --confirm` | throwaway T | 0 | `ok`: context synchronized with primary (a merge), checkpoint `b9d45a71`, integrated `b8bed005`, published, remote and local cleanup `complete`; `made_ready: [{D, md-sg0wv}]`, `made_ready_state: current` |
| `ticket close` | the same request again, JSON and human | 0 | `ok`, JSON byte-identical; human prints `made ready: <id> md-sg0wv` |
| `ticket close --prepare` | the closed ticket T, no fetch since its push | 3 | `recovery_required`, `[context_missing, primary_ahead_of_remote]`, not `ticket_closed` (see Blockers 1) |
| `ticket ready` | after the close | 0 | `ok`; D is listed, matching the report |
| `context observe-local` | the materialized peer ticket, behind primary | 0 | `ok` |
| `context sync-local` | that ticket, clean merge, human | 0 | `ok`, merge `61eaf139` |
| `context sync-local` | the same request again | 0 | `ok` (replay) |
| `context sync-local` | the old observation, new request, JSON and human | 3 | `external_change`, `reason: foreign_change`; human adds the `observe-local` line |
| `context sync-local` | a fresh observation, now contained | 0 | `already_applied` |
| `context sync-local` | D: the same file added on both sides, then edited differently on each | 4 | `recovery_required`, `conflict_pending`, `unsupported_conflict`, `external_resolution_required`: the conflict is installed in the worktree (`AA`). This matches the `context_sync_local_external_only` golden |
| `context show-local` | that operation, JSON and human | 0 | `ok`, `can_start_resolution: false`, one token |
| `context show-local --token` | JSON / human | 0 / 2 | `ok` (`base: null`) / `usage_error`, pointing to `--json` |
| `context resolve-local` | that external-only conflict, with an empty source / with the local side | 3 / 3 | `external_change`, `reason: foreign_change`, nothing written (see Blockers 2) |
| `poll once` | with that local sync pending | 3 | `recovery_required` (the pinned domain answer) |
| `poll pause`, `poll resume` | with that local sync pending | 0 | `ok` (not fenced, as designed) |
| `operation abandon` | that local sync: `--prepare` / `--confirm` | 0 / 3 | `ok` / `recovery_required`, files kept (the guarded abandonment) |
| `context sync-local` | the original request again, after a Git merge commit in the worktree | 0 | `ok`: the external repair was adopted |
| `context sync-local` | ticket E: on primary from the peer, edited by `ticket save` locally (which made its context) and by the peer on primary | 4 | `recovery_required`, `canonical_conflict`, `eligible_canonical`; human prints the `show-local` and `resolve-local` lines |
| `context show-local` (+ `--token`) | that operation | 0 | `ok`, `can_start_resolution: true`, sides `base`, `local`, `incoming` |
| `context show-local --token` | the token with its last character changed | 2 | `invalid_input` |
| `context resolve-local` | `resolutions: []` | 2 | `invalid_input` |
| `context resolve-local` | a complete set, `--input -`, human | 0 | `ok`: `5f36cbd1`, parents context and primary, "Resolve local context conflicts" |
| `context resolve-local` | the same request again | 0 | `ok` (replay) |
| `context sync-local` | E, after the resolution | 0 | `already_applied` |
| `index status`, `document list` | at the end | 0 | `ok`; 175 documents, 38 with problems |

**Application data.** A scan of every table of `manyhands.sqlite3` found the resolution's source
sentinel and the local edit's text in no table, and in no output. The throwaway ticket's title and
`closed_by` are only in `discovered_items`, the index, as the Global Constraints allow. The local
identity's email is also in `local_context_sync_execution.commit_email` (three rows): that's the identity
the local sync commits with, not `closed_by`.

### Blockers (for the owner)

1. **After a published promotion or closure, the next lifecycle prepare is refused until a fetch,
   and a closed ticket isn't reported as closed.** Neither push advances `refs/remotes/origin/main`
   (it stayed at `15a23125` after the promotion's push to `d54e9d48`). So:
   - `ticket close --prepare` on another, ready ticket answered `recovery_required`,
     `[primary_ahead_of_remote]`, exit 3, offering only `repo.inspect`;
   - after the close, `ticket close --prepare` of the ticket it had just closed answered
     `recovery_required`, `[context_missing, primary_ahead_of_remote]`. The design's `[AlreadyClosed]: ticket_closed`
     row isn't reached, because the observation reads the closure from the stale tracking ref
     (`closure/observe.rs`, `base = tracking_primary`). Before any fetch, a real closed ticket
     answered `[context_missing, primary_missing]` in the same way.

   A `poll once` (or `repo sync`) clears it. The next prepare was `ok`. Nothing tells a CLI user that
   a poll is the way out: the only offered action is `repo.inspect`. The 2b and 2c entries above record that "a push
   does not advance tracking". The design's Known Limits do not, and this is the plain sequence of
   promote, then close. Not fixed here. **Closed by decision 29**, see "Decision 29: tracking after a
   push" below.
2. **`context resolve-local` on an external-only conflict answers `external_change`,
   `reason: foreign_change`**, exit 3, and nothing is written. Nothing foreign changed, and `show-local` had said
   `can_start_resolution: false`. The human answer's `next:` line is `context show-local`, which gives the same
   inspection again, so it leads back to the same place. The design's required-outcomes table has no row for this case.
3. **An add/add conflict is external-only.** Here a ticket's file had reached primary by another
   commit (the peer landed the same bytes), and both sides then edited one line. The merge base has no copy of the file, so
   the conflict is `unsupported_conflict`. That is the documented `external_only` answer, but in real use it is the
   ordinary case for a ticket that lands on primary twice. The way out was a merge commit made by hand in Git, then the same
   `sync-local` request, which adopted it (`ok`). Recorded for a ruling on whether it stays external.

### Surprises in human output (not blockers)

- **Misleading messages.**
  - `context observe-local` of an existing item with no context prints "error: No item has that
    ID." above its save hint.
  - A stale `sync-local` prints `Next action: external_repair` beside a `next:` line for `observe-local`.
  - Its `Operation id: none` follows a header line `operation: <ULID>`.
- **Lifecycle prepare preview.**
  - `changes` is flattened as `Changes change: added` and `Changes path: …`, one field per line, so
    several changes would print unpaired.
  - Fields are sorted, which puts `Item id:` far from `Slug:` (a known limit already).
  - For a context never pushed, it says "Will delete the editing branch on the remote" and "Will
    synchronize the editing branch with the remote", and shows `Cleanup remote branch: true`.
- **Blockers label.** A blocked prepare prints `Blockers:` once per blocker (`Blockers:
  context_missing`, then `Blockers: primary_missing`).
- **Placeholders.** `poll status` prints `-` for null fields, while the policy verbs print `none`.
- **Progress lines.**
  - `poll configure` that answers `already_applied` still prints `progress: writing`.
  - Local-sync results print every null field (`Eligibility: none`, `Reason: none`,
    `Required request id: none`, `Resolution request id: none`).
- **`history_unknown` for local tickets.** `poll once` reports the two local, never-published tickets
  as `history_unknown`, "Publication history is unknown; inspect remote state before proceeding". The
  remote was configured at enable and the key selected later, which is the order Task 0's control
  shows setting the `01M66HFV44` mark.
- **Closed status text.** A closed ticket's file keeps `status: open` beside `closed_at` and
  `closed_by`. That is by design (status text plays no part), but a reader won't expect it.
- **`effects.write`.** The policy verbs report `write: not_requested` while writing the policy row.
- **Slow on real content.** These are times from the unoptimized debug build. Each prepare took about
  5 s, even for `item_not_found`. `promote --confirm` took 72 s and `close --confirm` 76 s. `poll once` took
  11–18 s. `sync-local` took 14 s at first, then grew: 23 s, 34 s, 42 s (the resolution) and 41–47 s for
  an `already_applied` answer. The last clean merge took 85 s. All of it was CPU-bound in the CLI
  process (`user` time about equal to `real`). A release build wasn't measured.

### Not run, and why

- `operation resume` of a lifecycle, Ctrl-C, `request cancel`, refused pushes and deletions, host
  and unlock stops: Tasks 12a and 13a own them, and they need the harness's faults. The fixture here
  refuses nothing.
- `document promote` and `ticket close` with a dirty primary, a missing identity or a pending local
  sync weren't staged, since the plan's tables hold them.
- A `resolve-local` partial resolution was not staged: it needs a failure point that a CLI child can't take.

### Task 10b review and Task 11's findings (controller's rulings)

**Seen first.** Against HEAD's verb code (`c01db51c`): 7 of the new or changed rows failed by assertion. The `--prepare` row and the second `request_mismatch` row's code passed already. That second row's exit (4) was an expectation of mine; see item 2.

- **1. The token check** in `resolve-local` refuses only when the inspection lists paths and the token isn't among them (`unlisted`). A finished operation goes to the library (`terminal_unlinked`). Rows: a new request after `complete` answers `external_change`, state `complete`. After an artifact-free `abandoned` it answers `cancelled`, exit 130. That abandonment comes from a resolution stopped at `LocalSyncBeforeIndex`, then `operation abandon` through the binary, as the library's `artifact_free_abandon_settles_all_associated_requests_atomically` does. The library's own cancelled-resolution row is `cancelled_partial_resolution_remains_accepted_and_resendable` (`tests/mutation_replay/local_sync.rs:970`).
- **2. Recorded requests.** `REQUEST_3`, completed and resent with a tampered token, is `request_mismatch`, exit 2. The synchronization's own request (`REQUEST_2`) sent to `resolve-local` with a tampered token is `request_mismatch` with outcome `partial`, from that request's own effects, so it exits 4. The ruling named the code, and I took the exit as the library gives it.
- **3.** A JSON golden for the verb's unknown-token refusal (`context_resolve_local_unknown_token`). Known Limits gains: a new request's JSON after a partial resolution names no original request (the text does), and the verb's refusals carry no scope, data or `repo.inspect`.
- **4.** The two "this formatter" comment lines are back above `fields`. A `--prepare` row answers `not_confirmable`: both checks now run only when the request isn't a prepare.
- **6.** The example row's negatives are one exact comparison of stderr.
- **7. Raw byte scan.** `files_holding` reads every file of the data directory, WAL included, as raw bytes for the source sentinel. It runs in the complete-set, unknown-token, after-partial and external-only rows, with the table scan. The sentinel now sits in the submitted source's body only (`sentinel_ticket`), which no index table keeps.
- **8.** The integration-window seed is in `operations::insert_remote_synchronization`. Its callers' targets pass: `read_status` 20, `read_contract` 72, `read_boundary` 24, and `mutation_replay -- resume journal_lookup` 53.
- **9.** A request counts as unrecorded only when `request show` answers `request_not_found`.
- **External-only conflict (Task 11).** `resolve-local` refuses it up front, from the inspection (`eligibility: external_resolution_required`): `recovery_required`, exit 3, the repair-outside note, no `show-local` line, nothing written and no request. Design row and goldens added.
- **`observe-local` with no context (Task 11).** The code stays `item_not_found`. The human message is "This item has no editing context.", beside the save hint. JSON keeps the read's message.
- **Consistent lines (Task 11).** Local-sync results in human mode list the data's non-empty fields only, dropping nulls, `false`, `none` and the tag. This is `human_collaboration`'s compact pattern for this data, not C2's full field list. An answer whose data names no operation prints no `operation:` line. Where the verb prints a line to observe or show again, `Next action:` isn't printed, so no repair outside is claimed beside it. 14 text goldens changed, and each diff was read.
- **Decision 30** is in the Cycle and the design's Known Limits. **Follow-up candidate:** make a canonical add/add of a ticket file resolvable through `resolve-local`.

**Verification.** `--test cli` whole 453 (117 s), `--bin manyhands-cli` 297, `--test mutation_replay -- local_sync` 75 (121 s). `fmt --check`, whole-tree clippy with `-D warnings` and the no-GPUI script pass.

## Decision 29: tracking after a push

Task 11's Blocker 1. The owner chose "Update after a push", and after my stop "Advance existing refs only (Recommended)", with E-22. Decision 31's release measurement is recorded in the Cycle.

**The push paths.** Every push goes through `push_exact`, called only by `push_and_verify` and `publish_synchronization_attempt` in `remote/sync.rs`. Every verified outcome reaches `finish_synchronization_publication`:
- synchronization's primary and context pushes: `repo sync`, `item sync`, and a lifecycle's primary publication and context child (`synchronize_promotion_child_observed`);
- republish;
- comment publication, which calls `synchronize_remote`;
- the reconciliation of a `PushVerified` row, or of a verified publication attempt.

So one helper there covers all of them. It runs after the `AfterPushVerification` safe point, the configuration check and the owner boundary, and before the row is completed, under the repository's lease. It moves an existing tracking ref forward to the verified OID when Fetch and Push are one endpoint. It doesn't create a ref, move one backwards or beside, or fail the publication. Ambiguous, refused and fenced pushes never reach it. A stop after it leaves a verified row that comes back to it, and a `restart` re-fetches anyway. The guarded remote delete already removes a context's tracking ref in cleanup. Polls, fetches and the evidence tables are unchanged.

**The first stop.** My first version also created a missing tracking ref. A context's first publication then made one, and after a stop at `AfterRemoteDeletion` and a cache loss, no fetch prunes it. The next preview planned a remote branch that is gone, and closure answered `recovery_required`. Four existing cache-loss rows failed on it. The owner then ruled existing refs only, and all four pass unchanged.

**Rows, seen failing.** Eight `c5_lifecycle_tracking_` rows. With the helper disabled, four fail: promotion then a close prepare (`recovery_required` where it should be `ok`), a just-closed re-prepare (should be `ticket_closed`), `repo sync`, and `item sync` advancing a fetched ref. The rest pass either way, and they pin that tracking stays put:
- `item sync` with no tracking ref stays without one;
- a refused push;
- an ambiguous stop at `AfterPushReturn`;
- a stop at `AfterPushVerification` then `restart`. Its re-fetch moves tracking whatever this helper does, so it isn't evidence for the fix. A resend without `restart` answers `recovery_required` both before and after.

**E-22.** Four `scope_after` assertions in `remote_merge_recovery` (lines 1397, 1458, 1525 and 1574) now expect the pushed ref's tracking at the pushed commit. Primary's tracking at line 1575 is unpushed there, so it is unchanged. Their comments say so.

**Documents.** The design's section "Tracking After A Push (decision 29)" states the chosen rule, and E-22 is in its named edits. The plan's constraint now reads E-1 to E-22. Task 11's Blocker 1 above is marked closed. The design had no Known Limits entry for it.

**Verified on the final tree, all exit 0.**

| Command | Ran | Time |
| --- | --- | --- |
| `--lib remote` | 479 | 186 s |
| `--lib local_sync` | 165 | 298 s |
| `--lib integration` | 26 | 16 s |
| `--lib promotion` | 205 | 136 s |
| `--lib closure` | 200 | 78 s |
| `--test remote_synchronization` | 69 | 583 s |
| `--test comment_publication` | 41 | 196 s |
| `--test remote_merge_recovery` | 29 | 236 s |
| `--test mutation_remote -- c5_poll` | 90 | 238 s |
| `--test mutation_remote -- c5_lifecycle` | 146 | 539 s |
| `--test document_promotion` | 79 | 760 s |
| `--test ticket_closure` | 78 | 684 s |
| `--test cli_remote` | 82 | 389 s |
| `cargo fmt --check`, clippy `-D warnings` | ok | |

## Part B checkpoint

**The gate, at `2b98365d`.** `origin/main` is `15a23125`, unmoved; `git merge-tree --write-tree HEAD origin/main` is clean.

| Command | Result | Time |
| --- | --- | --- |
| `check`, `fmt --check`, `clippy -D warnings` (all features) | exit 0 | |
| desktop `check` and `clippy`, CLI `check`, `check-cli-no-gpui.sh` | exit 0 | |
| `--lib` | 1507 passed | 756 s |
| `--bin manyhands-cli`, `--test cli`, the contract line | exit 0 | 512 s |
| `read_status`, `read_relationships`, `repository_enablement`, `remote_merge_recovery`, `comment_publication` | exit 0 | 455 s |
| `--test mutation_remote` | exit 1: 2 SSH cases failed, `c5_cancel_new_id_after_integration_promote` and `_close` | 5763 s |

### Item 0: the gate failure (stopped for a ruling)

`new_id_after_integration` (`tests/mutation_remote/c5_cancel.rs:505-530`) pins
`LifecycleBlocker::PrimaryAheadOfRemote` as why a new ID's prepare is refused once the first request has
published and been cancelled at `AfterPublication`. Decision 29 now advances primary's tracking ref on the
verified push, so that blocker is gone. A temporary probe in the row (reverted, not committed) read, for
both verbs:

- the new ID's `prepare` answers **`ok`**, a preview: `integration: already_contained`,
  `publication: publish`, `cleanup.remote_branch: false`, `worktree: true`, `local_branch: true`
  (closure: `recording: existing`). The context branch still stands on the remote;
- confirming that preview under a second request answers `busy`, `error`, recovery `[request.prepare]`;
  the first request stays `accepted`, and the second has no record.

So nothing starts beside the unfinished parent (the confirm is refused, as in
`new_id_before_integration`), but the prepare is `Ready`, and its preview also says no remote deletion
while the remote branch stands. That is the brief's stop condition, so the expectation was not changed
and items 1 onwards were not begun.

**Other rows naming the blocker.** None asserts it after a push. Run at `2b98365d`, each passes:
`document_promotion -- blocker_primary_ahead` and `ticket_closure -- blocker_primary_ahead_of_remote`
(a local unpushed commit), and `ticket_closure -- true_race_distinct_identities_needs_external_recovery`
(B's primary diverged by a refused push).

### The rulings on item 0, and the round

The controller's rulings: owner decision 33, "Refuse, name the holder", and decision 32's "published" test
from stored push evidence, not the tracking ref. Decision 32 was then withdrawn ("Decision 32 withdrawn",
below).

**Decision 33.** `LifecycleBinding::preview` (the prepare) first reads the item's unfinished parent through a
new `ParentRecord::supersedable` and `promotion::unfinished_holder`, and `LifecycleVerb::holder`. A parent a
new request couldn't supersede refuses the prepare: `recovery_required`, exit 3, no data, one action:
`request.retry` for the request whose accepted record holds the operation (`records::operation_holder`), or
`operation.resume` with the operation. Supersedable cleanup stops (`context_changed`, `ignored_files`,
`remote_context_changed`) still prepare. Human output prints the verb with `--request-id <holder>`, or the
resume, and a note. Rows, both verbs; the two `new_id` rows and `prepare_names_the_resume` were seen
failing with the check removed:

- `c5_cancel_new_id_after_integration`: refused naming `FIRST`, the original finishes with one merge, then a
  new prepare isn't `ok`. A consequence of decision 29, then 33: its expectation was `PrimaryAheadOfRemote`.
- `c5_cancel_new_id_before_integration`: the same before integration. Its prepare answer changed (it was a
  preview, then `busy` at confirm), a consequence, recorded.
- `c5_cancel_supersede_straight_after_the_cancel` (unchanged) still prepares and supersedes.
  `c5_cancel_supersede_after_a_cancelled_answer`: its `cancelled` answer overwrites the supersedable reason,
  so the new ID's prepare is now held (it was a preview, then `busy`), a consequence, recorded.
- `c5_resume_prepare_names_the_resume`: a parent no record holds names `operation.resume` with its operation;
  the resume finishes it with one merge.
- `c5_lifecycle_busy_with_no_parent_close`, a consequence too: it took its confirmation after the parent
  stood. It now takes it before the first request runs, which keeps the row's `busy` at confirm, and asserts
  that a prepare made once the parent stands is held.
- CLI: `a_prepare_while_a_request_holds_the_item_names_the_request` (`tests/cli/lifecycle.rs`), JSON and text
  goldens `ticket_close_prepare_held`, `_text`.

All 419 other `c5_` cases of `mutation_remote` passed against decision 33's binary before the busy row's fix
(`c5_` 420 cases, run detached, 1 failed, the busy row, since passing).

**Decision 32: nothing of it was changed, and it is withdrawn (below).** The ruling's row "a never-pushed context's
preview has `remote_branch: false` ... and the run deletes nothing remote" answers otherwise:

- the run publishes a never-pushed context itself. Its context child (`runs_context_child`,
  `promotion/lifecycle.rs:242`, and closure's `needs_context_child`) runs unless the context is already
  contained with no tracking context, and pushes the context branch; cleanup then deletes it, guarded by that
  child's publication authority (`promotion/run.rs`, the remote delete). Item 0's probe saw it: the fixture's
  item is "saved and unpublished" (`lifecycle_world::remote`), and after `FIRST` stopped at
  `AfterPublication` the branch stood on the remote. So for that case the preview's `remote_branch: true` is
  what happens, and false would leave the published branch behind;
- the case item 0 found (contained, branch on the remote, no tracking ref) previews `false` because the runner
  plans no deletion there either (`remote_deletion_required`, `promotion/lifecycle.rs:187`,
  `closure/record.rs:902`), and has no guard OID for one (`remote_delete_guard` with no child and no tracking
  answers none). Previewing `true` from stored push evidence needs the runner to delete with that evidence as
  its guard: a change to the run, beyond "promotion/ and closure/ observe".

**Spec review fixes.**

1. Decision 29's guards, `c5_lifecycle_tracking_split_endpoints` (a second server as `pushurl`, tracking
   unchanged), `_moved_past_before_the_update` and `_moved_beside_before_the_update` (an observer moves
   tracking at `AfterPushVerification`, which is left), `_fenced_at_verification` (the observer takes the
   reservation's epoch: the push lands, tracking unchanged). Each fails with its guard removed: the endpoint
   check, the descendant check, and the move placed before the safe point.
2. Known Limits gains tracking that isn't advanced, with the follow-up candidate: advance from a verified
   fetch-side read. A blocked prepare whose blockers are only `primary_ahead_of_remote` or `primary_missing`,
   plus perhaps `context_missing`, prints `poll once` and "Then prepare again"
   (`c5_polling_a_missing_tracking_ref_names_the_poll`, which then polls and prepares `ok`; golden
   `ticket_close_prepare_primary_missing_text`; bin row `a_tracking_ref_behind_or_missing_names_the_poll`).
3. `sync-local`'s external-only stop with an operation says "After the repair, send this same request again
   (<id>)"; `resolve-local`'s external refusal names the inspection's `starting_request_id`. Goldens.
4. `c5_polling_promote_then_close_needs_no_poll`: `document create`, a save, promote `--confirm` published,
   `ticket close --prepare` ready and confirmed, and a re-prepare is `ticket_closed`, all through the binary.
5. The schema checker takes no `minItems` (`tests/support/schema.rs` refuses unknown keywords), so the
   description of `resolutions` says an empty list is `invalid_input`. The row is
   `an_incomplete_duplicate_unknown_or_invalid_set_is_invalid_input_before_any_write` (`tests/cli/local_sync.rs`),
   whose `empty` case answers `invalid_input` with nothing written.
6. `worktree_not_clean`, a malformed observation's `invalid_input`, and the refused no-owner external cases say
   to observe again and use a new request, in `OBSERVE_NOTE`'s words.
7. A preview prints `Changes: <change> <path>` per change; blockers print once, `Blockers: a, b`. Known Limits
   gains the `-` against `none` placeholders, `progress: writing` on `already_applied`, and
   `write: not_requested` on the policy verbs.
8. The repair note names `{"observation": "..."}` for `sync-local`, and the save note what `ticket save` and
   `document save` take.
9. The Cycle's decisions are in number order, with 32 (withdrawn) and 33 added.

**S2.** Neither comment publication (`synchronize_comment_receipt`) nor republish (`republish_reserved`) holds
the repository lease when it reaches the advance; read in the source, and recorded in the design.

**Quality review fixes, no behaviour change.**

1. `render::compact` for the four compact renderers; C4's inline refusal is `render::refusal_of` through it.
2. `mutate::finish` for the `drive` tails; `mutate::RETRYING` for `{identity_prompt: false, retry: true}`.
3. `way_on` and `resolve_way_on` return `WayOn { lines, replaces_next_action }`, and `local_output` takes that.
   The `original` branch prints no line to look again, so it keeps `Next action:` now: the one golden change,
   `context_resolve_local_after_partial_text` gains `Next action: external_repair`.
4. `human_local_sync.rs` holds the context verbs' human output (its tests are `human_local_sync_tests.rs`);
   `save_line` and `NO_CONTEXT_NOTE` are in `render.rs`.
5. The six policy JSON goldens repeating `mutation_v1` (`poll_pause`, `_noop`, `poll_resume`, `_noop`,
   `poll_configure`, `_noop`) and `poll_once_usage` are gone; text goldens and schema checks stay, and
   `poll_pause_usage` is the one usage golden.
6. `cli::stdout`, `cli::stderr`, `cli::run_text`, `cli::scrub_text` and `cli::text_golden_scrubbed`.
7. Permanent comments name tests and files.
8. The policy test is two (`a_request_refused_for_its_input_...`, `a_request_refused_busy_...`);
   `the_refused_merges_strings_work_exactly` is `the_refused_merges_first_three_commands_parse_and_reach_their_verbs`;
   the cleanup-only goldens are `*_already_contained`.
9-18. The module doc of `local_sync.rs`; `resolve_line`; notes named `*_NOTE`, `*_DETAIL`, `*_HINT`
   (`TOKEN_JSON_DETAIL`, `NEW_REQUEST_NOTE`, `REPAIR_NOTE`, module-level `REFUSED_NOTE`) and
   `render::note_line`; `data_fields` through `fields_without`, and `fields` `pub(crate)` beside it; the
   lifecycle verb passes its `ItemDtoKind` to `prepare`; `policy_toggle` and `MUTATION_NO_INPUT`;
   `state.rs`'s `Resume` doc rewrapped; decision 29's helper doc tightened and its single-use `let` gone;
   decision 28's comment trimmed; `LocalConflictResolutionInput` derives `Clone` only, and the verb clones it.
19. `PLANNED` has three readers, not one: `startup_tests.rs`, `parse_tests.rs` and `catalog_tests.rs` (E-16
   moved the first two to it).

**Verified on the final tree (`1d35433a`), all exit 0.**

| Command | Ran | Time |
| --- | --- | --- |
| `check`, `fmt --check`, whole-tree clippy `-D warnings`, desktop `check` and clippy, CLI `check`, `check-cli-no-gpui.sh` | ok | |
| `--lib` | 1507 | 842 s |
| `--bin manyhands-cli`, `--test cli`, `read_contract`, `mutation_contract`, `mutation_replay`, `mutation_confirmation`, `mutation_bridges` | 300, 455, 72, 44, 384, 183, 166 | 573 s |
| `read_status`, `read_relationships`, `repository_enablement`, `remote_merge_recovery`, `comment_publication` | exit 0 | 547 s |
| `mutation_remote`, `document_promotion`, `ticket_closure`, `remote_polling`, `remote_synchronization`, `cli_remote` | 682, 79, 78, 84, 69, 84 SSH cases | 7298 s |

`origin/main` then moved to `0dfd5a68`, an `AGENTS.md` change only, and was merged (`0b04fd0c`) with no
conflict; `git merge-tree --write-tree HEAD origin/main` is clean.

### Decision 32 withdrawn, and main's AGENTS.md

**Decision 32.** The owner withdrew it, "Withdraw it, note the gap", on the facts above. Its premise came from
the controller's reading, not the owner's. Nothing implementing it was ever made, so the design had none to
remove. The Cycle keeps the entry, marked withdrawn. The design's Known Limits gains the contained context
whose remote branch has no tracking ref, which keeps that branch. Polls ignore it (decision 28).
Row: `c5_lifecycle_never_pushed_context_is_published_then_deleted`, both verbs. A never-pushed context's preview
says `remote_branch: true`, and the run's accepted updates of the context branch on the remote are exactly a
create, then a delete. **Follow-up candidate:** delete such a branch, guarded by the stored push evidence. Not
opened without the owner's go-ahead.

**Main's AGENTS.md (`0dfd5a68`, merged as `0b04fd0c`).** The plan is amended, caused by main:
- Global Constraints' test runs and checkpoint gate are the static gates across the whole tree, `AGENTS.md`'s
  fast tier verbatim with `--skip repository::local_sync::`, and the SSH tier selected by changed path.
- Task 15 Step 1 runs no full suite. It runs the static gates, the fast tier and every SSH-harness target, since
  the branch's diff touches `src/repository/transport/`. The owner confirmed on 2026-11-02 that Task 15 follows
  the table across the branch's full diff.
- Each remaining task's Verify keeps its filters, and adds the fast tier and its tier rows at checkpoints.
- The full `cargo test` and native CI run after merge.

### The scoped re-review of `79f08d10` (controller's rulings)

1. **`poll once` alone only for `primary_missing`.** For `primary_ahead_of_remote` primary may hold work never
   published, which a poll doesn't push: the way out is `poll once`, then `repo sync`, then a new prepare,
   with a note that says primary holds commits the remote's primary lacks as last read. `primary_missing` keeps
   the poll alone, with a note that the repository has no record of the remote's primary. Rows: the bin row
   `a_tracking_ref_behind_or_missing_names_the_poll`, rewritten first and seen failing; and
   `c5_polling_an_unpushed_primary_commit_names_poll_then_sync`, a real unpushed local commit: the prepare
   names both lines (golden `ticket_close_prepare_primary_ahead_text`), a poll pushes nothing and the prepare
   stays blocked, `repo sync` publishes primary, and the prepare is `ok`. The missing-ref golden's note changed.
2. No stale "stopped for a ruling" text remains for decision 32: the ledger's passages say it is withdrawn,
   and ticket comment `01M6F6D8DRGHFC8HJJSC7R5JHX` corrects comment `01M6F50Y…`, which stays as written.
3. `HELD_NOTE`: "prepare again once it has finished, or has stopped on a cleanup a new request can replace."
   The design's Known Limit says a new prepare is held after a generation raise over a retained child, and
   the request it names loops on `recovery_required`.
4. `resolve_way_on`'s original-request branch replaces `next action` again (`replaces_next_action: true`):
   the request line is the way on. `context_resolve_local_after_partial_text` is back as it was.
5. `document_promote_noop_text` is `document_promote_already_contained_text`.
6. `tracking_split_endpoints` asserts that a tracking ref exists before the push.
7. `promotion::unfinished_holder` reads through `read_session`, so a held registry answers as a read does,
   `busy`, and not `recovery_required`; `unfinished_record` takes a `Connection`. No row: the prepare's
   observation reads the registry first and meets the same contention, so no seam reaches the holder's read
   alone.
8. `offering_abandon` can't add `operation.abandon` to a held prepare. It offers one only where the answer is
   `recovery_required` or `external_change` and offers `repo.inspect`, or where the envelope names its own
   operation and request. A held prepare offers `request.retry` or `operation.resume` alone and names neither,
   so it returns the envelope unchanged. No change.

**Verified.** `--test cli` 455 (after the two goldens the rulings change, the held note and the partial
resolution's text, were updated and read); `--bin manyhands-cli` 300; `--lib promotion` 205;
`mutation_remote -- c5_cancel_` 99, `-- c5_resume` 58, `-- c5_lifecycle` 152 SSH cases;
`c5_polling_an_unpushed_primary_commit_names_poll_then_sync` and `_a_missing_tracking_ref_names_the_poll` 1
each; `fmt --check` and whole-tree clippy `-D warnings` ok. The fast tier, verbatim: `--lib` 1346 passed and 1
failed, `closure::checkpoint::tests::changed_identity_at_the_hook_is_external_change`, whose two records print
the same and differ in a field `Debug` leaves out. It passes alone, three runs of three, and touches no code
this round changed. It passed in the gate's `--lib` above, so it is recorded as a timing flake, not fixed.
That run stopped at `--lib`, so the tier was run again whole, with `--no-fail-fast`: exit 0, 79 test results,
4908 passed, none failed (`--lib` 1347).

## Task 12a: lifecycle exit evidence over SSH

Base `565523cc`; `origin/main` at `0dfd5a68` is contained. `tests/cli_remote/lifecycle.rs` holds the
`c5_lifecycle_` cases, each a fresh `CliPair` under the 100-second watchdog, registered as `lifecycle::BOTH`
and `lifecycle::ONE` in `tests/cli_remote.rs`. No library or bin file changed.

**Staging.** A CLI child can't take a failure point. Three rows run their first call in the test's own
process, on the clone's own data and with the confirmation the binary issued, and everything after it through
the binary: `PrimaryAdvanced` (`AfterContextSync`), the refused checkpoint merge (`AfterClosureCheckpoint`),
and a parent no request record holds (the domain's entry point stopped at `AfterConfirmed`, as `c5_resume`
begins one). The unlock row's middle call also runs there: the publication runs in this process
(`AfterPublication`), and the terminal's run is only the cleanup, because each connection decrypts the key
again and a whole close at a terminal outran the PTY helper's ten-second wait. (Corrected by the Task 12
review, which also gave that row a 40-second wait through an additive `PtyCli::exit_within`.)

**Rows (both verbs unless named).**

| Case | What it shows through the binary |
| --- | --- |
| `dirty_primary_blocks` | `recovery_required`, 3, blockers `[primary_dirty]`, `repo.inspect`, `Blockers: primary_dirty` in text; no ref moves at either end, no push. Clean again, ready. |
| `stale_consent_context_moved_close` | A second save between prepare and confirm: `external_change`, 3, `request.prepare`, no record, no merge, context kept; a fresh consent finishes once. One verb (decision 27: one digest). |
| `refused_primary_push_then_same_request` | `push_rejected`, 4, partial, integration complete, publication and cleanup pending, context and worktree kept, record accepted. `operation resume` answers `original_request_required`, 3, naming the request, with no record of its own. Refused again under the same request: one merge still. Accepted: `ok`, published, cleaned up, one merge, one checkpoint, `closed_at`/`closed_by` unchanged; the replay is identical with no connection. |
| `refused_deletion_then_same_request` | `push_rejected`, 4, published, `cleanup_pending: remote_deletion_rejected`, `request.retry` alone; the same request then deletes the branch; one merge, one checkpoint, fields kept; replay with no connection. |
| `primary_advanced_continues` | Stopped after the context's synchronization (published first, so nothing durable), a peer's commit polled in: `external_change`, 3, blocked, `request.retry`, `repo.inspect`, same operation, record accepted; the same request then merges once, on top of the peer's commit. |
| `resume_with_no_record_continues` | A new prepare is held, naming `operation.resume` with the operation (decision 33). The resume finishes in human mode; its JSON replay is the lifecycle's data with no connection; a fresh resume is `already_applied`; the same request ID for another operation is `request_mismatch`, 2; a malformed operation ID is `invalid_id`, 2. |
| `deleted_branch_request_route` | Decision 18: `remote_branch_deleted`, 3, `request.cancel`, `repo.inspect`; `request cancel`, the same request `cancelled` 130 with nothing pushed, `item republish` (one push), the same request `ok`, one merge. For a close, the four `next:` lines printed are exactly the commands run. |
| `deleted_branch_resume_route` | Decision 19: under `operation resume`, `remote_branch_deleted`, 3, `operation.resume` (naming the operation), `repo.inspect`, no record; `item republish`; the resume again `ok`, one merge. For a close, the two `next:` lines printed are the commands run. |
| `host_stop_then_same_request_close` | A rotated host key: `host_replacement_required`, 3, `request.retry`, `repo.inspect`, record accepted; `host replace` through the binary; the same request finishes once. One verb: the child's synchronization is shared. |
| `unlock_stop_then_same_request_close` (Unix) | A protected key with no terminal: `unlock_required`, 3, `request.retry`, record accepted. The same request's publication then runs **in this process** (stopped at `AfterPublication`), not at the terminal; with no terminal again, its cleanup's deletion stops the same way, 4, partial; at a PTY the same request asks for the passphrase and completes the cleanup: one merge, one checkpoint, published, `closed_at`/`closed_by` as the in-process publication wrote them; the passphrase is in no output or transcript. One verb, as the host row. (Wording corrected by the Task 12 review.) |
| `refused_checkpoint_merge_repaired_close` | The local recovery over SSH: stopped past its checkpoint, a peer's edit of the ticket polled in, the same request refused (4) with the four `context` lines; each run with only its placeholders filled; the same request then `ok`, published, `integration: merged`; primary's merges are the closure's one integration and the resolution's, and the remote's primary holds the checkpoint's `closed_at` and `closed_by`. `tests/cli/local_sync.rs` proves it with no remote. |

**Referenced, not repeated:** `c5_polling_promote_then_close_needs_no_poll` (both verbs published, then
`ticket_closed`), `c5_polling_item_sync_of_a_copy_the_remote_still_holds` (a refused deletion's first answer),
`tests/cli/lifecycle.rs` (Task 7's prepare, confirm, mismatch, local-only and sentinel rows), and the library
tables for the expired confirmation's stored clock and cache loss.

**Seen first.** No row here tests behaviour new in this task: Parts A and B implemented it, and the rows pin
it through the binary. 14 of the 20 first written passed on their first run. What failed:

- **`stale_consent_primary_moved` (both verbs) answered otherwise, and is not committed.** See Blockers.
- `deleted_branch_request_route_promote` and `deleted_branch_resume_route_promote`, on the human lines. See
  Blockers. The rows now follow the JSON actions for a promotion, and assert the printed lines for a close.
- Fixture faults, not answers: the refused checkpoint merge first moved only the ticket's title on primary,
  which Git merged cleanly with the checkpoint (the close answered `ok`), so the peer now edits the body line
  too; the unlock row's terminal run exceeded the PTY harness's ten seconds, hence the staging above.

**Blockers (for the owner), each left out of the commit as a row.** Both ruled: see "Task 12 rulings" below.

1. **A primary that moves by a clean fast-forward between prepare and confirm does not refuse the
   consent.** The Cycle's acceptance row says "A prepare whose context or primary then moves answers
   `external_change` at confirm". Through the binary, for both verbs: prepare (preview `primary_oid`
   `57dcd346`), a peer's commit of an unrelated file on the remote's primary, `poll once` fast-forwards
   primary to `95b3a04b`, then the confirmation answers **`ok`**: one merge `[95b3a04b, checkpoint]`,
   published. The library's tables never move primary before a confirmation (`stale_at_confirm` dirties
   primary; `crash_before_record_stale` edits the context). Not fitted; the row is out of the commit.
2. **A promotion's stop at its child carries no lifecycle data, so its human answer prints no route.**
   `document promote` meeting the deleted branch answers `remote_branch_deleted`, 3, with `request.cancel`,
   `repo.inspect` as designed, but `data: null` and every effect `not_requested`; a close's answer carries its
   data. `human_lifecycle` keys the route on the data, so a promotion prints only `next: request.cancel ...`
   and `next: repo.inspect ...`, and under `operation resume` it prints C4's comment note ("the request that
   started this publication, its comment add or item sync, still holds the reservation ..."), which is false
   for a lifecycle resume that let the child go. Decision 19 says Task 7's human lines print the route. The
   JSON routes walk to a finish for both verbs.

**Verified.** `--test cli_remote -- c5_lifecycle`: 18 SSH cases passed in 228 s (one case was changed after
it, the resume row's matrix cells, and that case was rerun alone for both verbs). `cargo fmt --check` and
whole-tree clippy with `-D warnings` pass.

## Task 12b: the six journeys

Base `30a37018`; `origin/main` at `0dfd5a68` is contained. `tests/cli_remote/journeys_c5.rs` holds the
design's journey table as eight `c5_journey_` cases through the binary over SSH, each asserting Git objects
and canonical content, with every output scanned. It reuses Task 12a's helpers (`lifecycle::Verb`,
`finished_once`, `closure_at`) and C4's (`collaboration`, `conflicts`). "Final save" differs by design: each
journey saves before it promotes or closes, and says so. No library or bin file changed.

| Case | PRD journey | What it shows |
| --- | --- | --- |
| `offline_edit_then_sync` | Offline edit, checkpoint and later sync | With the server dropping every connection, a save checkpoints and connects to nothing. `item sync` answers `transport_unavailable` naming the same request, the checkpoint kept; reachable again, that request publishes it byte for byte. Variant: the remote refuses the next checkpoint, `push_rejected` naming the same request, the work kept; accepted, the same request publishes it. |
| `conflict_recovery` | Concurrent conflict recovery (thin) | C4's `tickets_diverge`: B's sync stops on the conflict with B's commit kept; `conflict show`, then `conflict resolve` with the tokens shown; one resolution commit `[local, incoming]` holding the resolved sources, published; the stopped sync's own request then answers `ok` with that commit and pushes nothing. C4's `c4_conflicts_*` hold every other answer. |
| `background_update_and_discovery` | Background remote update and discovery | B publishes its context with a comment, a peer publishes primary with a new ticket. One `poll once` in A fast-forwards primary, materializes the context, makes exactly one worktree, and indexes the new ticket (`ticket show`) and the comment (`comment list`); a second poll finds both current, one worktree, nothing pushed. |
| `threaded_discussion` | Durable threaded discussion | Roots from both clones and two replies to one root, each published by `comment add`; every comment file on the remote's branch. After `index rebuild` in A, the list read from the item's own worktree (`--repo` the context worktree), from A's root and from B's is the same: roots in creation order, each reply beneath its parent in creation order. |
| `document_promotion` | Confirmed document promotion | Created and saved, the preview names branch, primary, remote, the added path, a merge commit, publication and all three cleanups, no closure. Confirmed: merged, published, cleaned up at both ends, one merge, the saved body on the remote's primary. B's poll brings it in; a fresh `document save` in A starts a new context whose parent is primary. |
| `document_promotion_local_only_then_published` | The same, local only | A clone of the server enabled with no publication remote and no key: the preview says `pending_no_remote` and no remote; the promotion exits 0, `publication_pending: no_publication_remote`, cleanup complete, nothing remote moved. Then `key import`, `key select`, `remote select` (the worktree clean after its commit, `01M669FJR2`), `host approve`, `repo sync`: the remote's primary is the integrated history, with no fixture seeding (decision 10). |
| `close_with_retryable_failure` | Confirmed close with retryable failure | A dependent is on primary. The preview names `closed_by`. A dirty primary after it, the injected prerequisite failure, refuses the confirmation (`external_change`, `request.prepare`), and the new prepare names `primary_dirty`, the failed step; context, branch, worktree and the remote are untouched, no merge. Corrected, the close is published and cleaned up, one merge and one checkpoint, `made_ready` is the dependent (`current`), and `ticket ready` lists it and not the closed ticket. |
| `close_local_only_then_published` | The same, local only | As the promotion's local-only case, for a close: one checkpoint, and the remote's primary holds the same `closed_at` and `closed_by` once `repo sync` publishes. |

**Referenced, not repeated:** the dirty, divergent, malformed and remotely deleted contexts of the
background journey are `c5_polling_dirty_context_preserved`, `_divergent_context_preserved`,
`_malformed_ref_names_nothing` and `_deleted_context_preserved`; the close's refused push and refused
deletion are Task 12a's `c5_lifecycle_refused_primary_push_then_same_request_close` and
`_refused_deletion_then_same_request_close`; conflict recovery's answers are C4's `c4_conflicts_*`.

**Seen first.** Every journey pins behaviour that exists, so none was expected to fail. Six passed on their
first run. Two failed on expectations of mine that no design row states, each read and corrected:

- `offline_edit_then_sync`: `item sync`'s `push_rejected` with nothing durable done is `blocked`, exit 3, not
  4; and a second request while the first, stopped, holds the reservation answers `busy` (C4's own note says
  so), so the journey goes on under the same request.
- `conflict_recovery`: the stopped sync's own request, sent again after the resolution, answers `ok` with the
  resolution's commit, not `already_applied`.

**Observed, for the owner (C4's behaviour, outside C5, not walked further).** In the offline journey's first
form, the remote refused the context's **first** publication (the branch did not exist there yet), and the
same `item sync` request, sent again once the remote accepted, answered `recovery_required`, 3, offering
`request.retry` and `repo.inspect`, with nothing published. The journey's variant now refuses a later
checkpoint of a published branch, which C4's comment row already shows finishing.

**The design's matrix, Task 12a's cells.** `document promote` and `ticket close`: replay and same-request
continuation, in the refused push and refused deletion rows. `operation resume` of a lifecycle: human and
JSON, input failure (`invalid_id`), no-op (`already_applied`), replay and mismatch in
`resume_with_no_record_continues`; continuation by a later resume in `deleted_branch_resume_route`.

**Verified.**

| Command | Ran | Time |
| --- | --- | --- |
| `--test cli_remote -- c5_journey` | 8 SSH cases | 69 s |
| `--test cli_remote -- c5_lifecycle` | 18 SSH cases (Task 12a) | 228 s |
| `--test cli_remote`, whole | 111 SSH cases, 0 failed | 814 s |
| AGENTS.md's fast tier, verbatim | 79 results, 4908 passed, 0 failed | 1510 s |
| `cargo check --all-features --locked`, `cargo fmt --check`, whole-tree clippy `-D warnings` | ok | |

## Task 12 rulings: the two blockers, and the C4 finding

The coordinator ruled on Task 12a's two blockers. Base `b441e715`.

**1. The stale-consent row: a correction, not a new decision.** Wave 02 Cycle 09's D6
(`docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md:250-256`, owner-approved, and closure
follows it) already says what a consent binds: with a remote the digest leaves out the context, local-primary
and tracking OIDs, and the item's change set and the D2 relation stand in; without one it binds both local
OIDs. The C5 Cycle's acceptance row overstated it. The controller's Cycle text is corrected to the Wave 02
rule: with a remote, a primary advance that leaves the item's change set unchanged is accepted, and the
confirm integrates onto it; a context change that changes the item's change set, any other change to the
change set, or any primary move without a remote answers `external_change` at confirm (wording corrected by
the Task 12 review: "a move of the context" overstated it the same way). Corrected in the Cycle (the acceptance row and Required
Contract item 1) and in the design's fact 7. Task 12a's Blocker 1 is closed by this.

Rows, all both verbs, since each verb's domain has its own digest encoding:

| Case | Answer |
| --- | --- |
| `c5_lifecycle_stale_consent_primary_moved` | A peer's commit of another file, polled in: the confirm answers `ok`, merges onto the peer's commit (first parent) and publishes, once. |
| `c5_lifecycle_stale_consent_item_path_moved` | A peer's commit of the item's own path, polled in (a ticket retitled, which Git merges cleanly; a document's path added with the bytes the context holds): `external_change`, 3, `request.prepare`, no record, no merge, context kept, nothing pushed. |
| `c5_lifecycle_stale_consent_local_only_primary_moved` | A `LocalOnly` clone (no publication remote), a person's own commit of another file on primary: `external_change`, 3, as above. |
| `c5_lifecycle_stale_consent_context_moved` | Was close only; now both verbs (a `document save` for a promotion), since the encodings are per verb. Unchanged in what it asserts. |

**Seen to fail with the other expectation.** Each of the three new rows was run with its expectation flipped
(`external_change` for the accepted case, `ok` for the two refused ones): all six failed by assertion at the
code check, and with the expectations restored all six pass. The accepted case also failed that way in Task
12a's first run.

`LocalOnly` moved from `journeys_c5.rs` to `lifecycle.rs`, with no change, so both modules use it.

**2. A promotion's child stop carries the same data as a close's.** In `bind/lifecycle.rs`, `stop_data`
returned no data for a promotion's stop that named no blockers (`None if V::REPORTS_MADE_READY => self.data(),
None => return None`), so a child's `remote_branch_deleted` came back with `data: null`. A stop whose child
stopped (`Stop::ContextSynchronization`) now names its item with the lifecycle DTO for both verbs, a
promotion's made-ready fields still null. That is the one library change. `human_lifecycle` keys decision 18's
and 19's routes on that data, so both verbs print them, and under `operation resume` the lifecycle arm renders,
so C4's comment note no longer prints for a lifecycle.

- **Narrowed to child stops, for a ruling.** The first version gave every promotion stop the DTO. The
  `mutation_remote` tier row on it: 683 passed and one failed by assertion, `c5_made_ready_promotion_stop_has_no_data`,
  Task 4's own pin that a promotion's stop before primary (`BeforeIntegrationRef`) "holds no data, as before".
  Changing that pin is a semantic edit no named edit covers, so the change is narrowed to what the ruling
  names, a promotion's child stops, and the pin holds unchanged. If every promotion stop should name its item,
  that row's edit is the owner's to approve; the wider patch is two lines (`None => self.data()`).
- **Test first.** The promotion's human-route check was un-skipped in `deleted_branch_request_route` and
  `deleted_branch_resume_route`, which now also assert the route's note (the reservation note, and the
  resume's own "this resume released ..." note, with no "comment add or item sync"). Against the unchanged
  binding both promotion rows failed by assertion at the route comparison, and the two close rows passed.
  With the fix all four pass. The request route also asserts the JSON stop names the item.
- **Library rows.** No library golden or row changed: the child's `remote_branch_deleted` rows
  (`c5_lifecycle_child_remote_branch_deleted`, `c5_cancel_walk_remote_branch_deleted`,
  `c5_resume_walk_remote_branch_deleted`) don't assert the data.

**3. Follow-up candidate, not opened without the owner's go-ahead (C4's `item sync`).** A refused **first**
publication of a context, then the same request, publishes nothing. The sequence, through the binary over
SSH (`CliPair`): A saves `ITEM` (its context never published); the server's bare `HEAD` is set to the
context's branch, which doesn't exist there yet, and receive-pack refuses the current branch; `item sync
--request-id R` answers `push_rejected`, 3, blocked, `request.retry`; the refusal is lifted and `HEAD` set back
to `main`; the same `item sync --request-id R` answers `recovery_required`, 3, offering `request.retry` and
`repo.inspect`, with nothing published. A later checkpoint of an already published branch, refused then
accepted, publishes under the same request, as C4's comment row and `c5_journey_offline_edit_then_sync` show.
Whether the fixture's unborn `HEAD` contributes was not established.

**Verified.**

| Command | Ran | Time |
| --- | --- | --- |
| `--test cli_remote`, whole | 118 SSH cases, 0 failed | 890 s |
| `--test mutation_remote -- c5_` | 426 SSH cases, 0 failed: `c5_cancel_` 99, `c5_lifecycle` 152, `c5_resume` 58, `c5_made_ready` 27, `c5_poll` 90 | 2090 s |
| `--test mutation_remote`, whole (the tier row), on the first, wider version | 683 passed, 1 failed (`c5_made_ready_promotion_stop_has_no_data`, above) | 4734 s |
| AGENTS.md's fast tier, verbatim (`--test cli` 455 within it) | 79 results, 4908 passed, 0 failed | 1426 s |
| `cargo check --all-features --locked`, `cargo fmt --check`, whole-tree clippy `-D warnings` | ok | |

The whole `mutation_remote` target was not run again on the narrowed version: the narrowing only takes back
data from non-child promotion stops, which is the state every row but the one passed on before this change,
and every `c5_` row ran on it. A fast-tier run on the wider version was stopped part way, and a `pkill` of it
by command line may have matched another session's process of the same command line; nothing showed one.

## Task 13a: interruption and no worker

Base `c94dd8e2`; `origin/main` at `0dfd5a68` is contained, so nothing was merged. Only
`tests/cli_remote/interruption.rs` changed: 30 `c5_interruption_` cases, registered in its `CASES`, each a
fresh `CliPair` under the watchdog. No library, bin, support or golden file changed.

**Staging.**

- **Lifecycle verbs and `poll once`: a PTY press while the server holds an advertisement,** as C4's
  cases do. Before integration, the first held connection is the context child's fetch. After
  integration, each connection is held in turn until primary holds the merge, which lands the press
  at the primary publication's fetch.
- **The two context mutations have no server to hold.** One of the run's own pipes is made one page
  long (`F_SETPIPE_SZ`) and pre-filled, so the run waits writing its next line:
  - **stderr, after `progress: accepted`.** Local sync writes a safe point's progress line before it
    reads the cancellation (`local_sync/control.rs`, `request_safe_point`), so the press lands before
    the first safe point's read.
  - **stdout, at the result's first write.** That write comes after the Ctrl-C window has closed, so it
    is a press after the last safe point.
  - The wait is read from `/proc/<pid>/wchan` (`pipe_write`) with `FIONREAD`. The press is `SIGINT` to
    the run's PID, which is what a terminal's line discipline sends on Ctrl-C. These rows are
    **Linux-only**, for the pipe size and `/proc`.
- **No resident worker.** A CLI child from `cli_command` (`setsid`) is collected by `collect_cli`,
  which proves the exit and both pipes' EOF within a second. Then no process may remain whose session
  is the child's PID: `/proc/*/stat` on Linux, and a null signal to the process group elsewhere on Unix.
  The positive control runs first: `sh -c 'sleep 60 … & exit 0'`, its pipes closed, exits and reaches
  EOF, and the check finds the sleeping process. That process is then killed by its PID.

**Rows (both verbs unless named).**

| Case | Answer through the binary |
| --- | --- |
| `press_before_integration` | `cancelled`, 130, `next: request.retry`, record accepted with no result, no merge, nothing pushed, no reservation held. A new ID's prepare is held, `recovery_required`, 3, naming the request (decision 33). The same request finishes once, and then a new prepare is not `ok`. |
| `press_after_integration` | `cancelled`, 130, `integration=complete`, primary not published, record accepted. Then the new ID is held, the same request publishes with one merge, and a new prepare is not `ok`. |
| `second_press_close` | Ended by `SIGINT`, no result printed, nothing merged or pushed, record accepted. The same request finishes once. One verb, since the window is the CLI's (decision 27). |
| `consent_prompt` | Interactive at the PTY, with no request ID or confirmation. The request is noted first, the preview shown, `Proceed? [y/N]` once, and `y` completes it once. |
| `press_at_the_consent_prompt` | The raw-mode byte is the prompt's refusal: `cancelled`, 130, terminal modes restored, `request_not_found` for the allocated ID, no ref moves at either end, no connection, and a fresh prepare `ok`. |
| `press_during_resume` | `operation resume` of a parent no request holds, pressed at the child's fetch: `cancelled`, 130, nothing merged or pushed, no record (`request_not_found`). A later resume finishes once. |
| `press_during_poll_once` | `cancelled`, 130, nothing fast-forwarded. The record is finished (R-18), the replay is the same with no connection, and a new request polls. |
| `primary_child_fails_with_cancellation_asked_close` | **The primary `Err` arm's row** (`promotion/run.rs`, `publish`). Pressed at the publication's fetch, the server then drops that connection, so the child fails rather than stops. The answer is `cancelled`, 130, `integration=complete`, with **no reservation held** (`poll status`'s `active_operation_id` is null) and the record accepted. The same request publishes once. Compare `c5_cancel_publication_child_released`, where the same failure without a cancellation keeps the reservation. One verb (decision 27). |
| `press_at_a_safe_point_of_sync_local` (Linux) | `cancelled`, 130, `progress: stopping`, nothing merged. The record is finished (the operation ends `cancelled`, `control.rs`'s `terminal`), and the replay with the same observation is the same. A new request merges. |
| `press_at_a_safe_point_of_resolve_local` (Linux) | `cancelled`, 130, not `complete`, record **accepted**. The same request with the same sources completes. This matches the gate's "effects kept, retry or abandon". |
| `request_cancel_of_sync_local`, `_of_resolve_local` (Linux) | `request cancel` from a second invocation while the run waits at its first safe point's line. The answers are as the two rows above. |
| `press_at_the_result_of_poll_once`, `_of_sync_local`, `_of_resolve_local` (Linux) | **After the last safe point:** ended by `SIGINT` at the result's write, outside the window. The work stands: the record is finished `ok`, the replay is identical, and there is no second commit or connection. |
| `request_cancel` | Lifecycle verbs, while the context child's fetch is held: the second invocation answers `requested: true`, then `cancelled`, 130, `request.retry`, record accepted, no merge, no push, no reservation. Then the held prepare, the finish, and a new prepare that is not `ok`. |
| `request_cancel_of_poll_once` | C4's `cancelled_while_held`: `cancelled`, then `finished` with the flag acknowledged. The replay makes no connection, and a new request polls. |
| `killed_poll_once_resent` | Killed by its handle while the fetch is held. The record stays accepted with one operation, and the resend under its ID takes up **that operation**, `ok`, and fast-forwards. |
| `lost_stdout` (both verbs), `_of_poll_once`, `_of_sync_local`, `_of_resolve_local` | Exit 1 with nothing on stderr, and the work is done. `request show` holds `ok`, and the replay matches it with no connection, push or second commit. |
| `no_resident_worker` | The control first. Then `poll once`, `ticket close` and `document promote`, each run to completion through `exited_alone`. |

**Seen first.** Every row pins behaviour that Parts A and B built, so none was expected to fail, and
all passed once their fixtures were right. The positive control is the no-worker check's failing case.
Fixture faults, none an answer:

- I expected `progress: stopping` from a lifecycle verb's cancellation. Lifecycle progress has no
  such stage (`accepted`, `connecting`), and nothing in the design names it.
- A sync-local replay sent a freshly read observation. That is another request, so it answered
  `request_mismatch`. The rows now resend the original input.
- My first receive race was armed before the context child's push, so it cut that push. It is now
  armed at the primary push only.
- Five rows failed by the watchdog with no assertion line while I compiled `--test cli` beside the run.
  Rerun alone, they passed: 22 s for the five lost-stdout rows, 11 s for no-worker.

**Blocker (for the owner): Review Focus 4 answers otherwise, and its row is not committed.** The
staging is C4's `c4_interruption_press_after_the_last_safe_point`, for both verbs:

1. Each connection is held until the integrated primary's push (`git-receive-pack`, with the merge on
   primary), and `reject_primary_updates(true)` is set.
2. A receive race is armed on primary, the server's answer is stalled 5 s, and the press is made once
   the push command has arrived.
3. The push is refused: one refused update of primary.

The design's rule is "A press after the last safe point keeps the domain's answer", which is
`push_rejected`, 4. **The binary answers `cancelled`, exit 130**, with `integration=complete`,
`Publication pending: cancelled`, `Cleanup pending: awaiting_publication`, `next: request.retry`, no
`retrying`, and the record accepted. Run once with 130 in place of 4 (not committed), the rest held for
both verbs: the record is accepted, primary is put back at the remote, and the same request then
publishes with one merge.

The reading: `publish`'s `Err` arm reads the cancellation after a failed primary child and releases it
(decision 18's "because its … primary child has just failed with cancellation asked"). That read is a
safe point after the push. So for these verbs no press after the push command is after the last safe
point, and the domain's `push_rejected` is never what such a press keeps. The owner must rule on which
holds:

- the row expects `cancelled`, and the design's sentence is narrowed to say that a refused push is
  followed by a safe point; or
- the `Err` arm's answer changes.

The probe is kept outside the tree. Its function and the two registrations are left out.

**Matrix cells.** `document promote`, `ticket close`:

- **PTY consent and a press at it:** above.
- **Ctrl-C:** above, except RF4.
- **`request cancel`:** above.
- **Lost stdout:** above.

`operation resume` of a lifecycle: Ctrl-C (`press_during_resume`). `poll once`:

- **Ctrl-C:** `press_during_poll_once`, and after the last safe point at its result.
- **`request cancel`:** `request_cancel_of_poll_once`.
- **Same-request continuation:** `killed_poll_once_resent`.
- **Lost stdout:** `lost_stdout_of_poll_once`.

`context sync-local`, `resolve-local`: Ctrl-C, `request cancel` and lost stdout, the first two
Linux-only.

**Not reached, and why.** A press after the last safe point of a context mutation or a poll, but still
inside the window, isn't reached: no line or wait comes between the last safe point's read and the end
of `execute`. On Unix outside Linux, the context mutations' presses and `request cancel` have no row,
since no portable way holds their pipes. The library rows pin them (`tests/mutation_replay/local_sync.rs`).

**Verified.** `--test cli_remote -- c5_interruption`: 30 SSH cases passed in 159 s, run alone.
`cargo fmt --check` and whole-tree `clippy --all-targets --all-features -D warnings` pass. The whole
`cli_remote` target and the fast tier run with Task 13b, below.

## Task 13b: recovery walk and exits

Base `a7fbf33d` (Task 13a); `origin/main` at `0dfd5a68` is contained. Files: `tests/cli_remote/lifecycle.rs`
(walk and evidence rows) and `tests/cli/collaboration_exits.rs` (E-8). `tests/cli/exits.rs` is C2's table and
needed no change: C5's codes are accounted for in E-8's new table. No library, bin, support or golden file changed.

### The walk: every `next:` line the C5 verbs print

Each line was followed through the binary, with only its placeholders filled. A walk ends when the reservation
is free and the work is done.

| Verb and stop | Lines printed | Walked by | Outcome |
| --- | --- | --- | --- |
| `ticket close`, refused checkpoint merge, request | four `context` lines, the same request | `c5_lifecycle_refused_checkpoint_merge_repaired_close` (12a); `the_refused_merges_route_finishes_the_close` (10b) | free, one close merge |
| `ticket close`, refused checkpoint merge, **under `operation resume`** | four `context` lines, the resume | **`c5_lifecycle_walk_refused_merge_under_resume_close`** (new) | free: the resume line merges once, published, checkpoint fields kept |
| either verb, child `remote_branch_deleted`, request | `request cancel`, same request, `item republish`, same request | `c5_lifecycle_deleted_branch_request_route_*` (12a) | free, one merge |
| either verb, child `remote_branch_deleted`, resume (decision 19) | `item republish`, the resume | `c5_lifecycle_deleted_branch_resume_route_*` (12a) | free, one merge |
| either verb, `push_rejected` (primary, deletion) | `request.retry` | `c5_lifecycle_refused_primary_push_*`, `_refused_deletion_*` (12a) | free |
| `ticket close`, host and unlock stops | `request.retry`, `repo.inspect` | `c5_lifecycle_host_stop_*`, `_unlock_stop_*` (12a) | free |
| either verb, `external_change` (`PrimaryAdvanced`, stale consent) | `request.retry` / `request.prepare` | `c5_lifecycle_primary_advanced_continues_*`, `_stale_consent_*` (12a) | free |
| either verb, `cancelled` | `request.retry` | `c5_interruption_press_*`, `_request_cancel_*` (13a) | free, one merge |
| either verb, prepare held by a request (decision 33) | the same request | `c5_interruption_press_before_integration_*`, `_request_cancel_*` (13a) | free |
| either verb, prepare held by an operation no request holds | the resume | `c5_lifecycle_resume_with_no_record_continues_*` (12a) | free |
| `ticket close`, prepare `integration_conflict` | four `context` lines, "prepare again" | **`c5_lifecycle_walk_prepare_conflict_close`** (new) | free: a new prepare is ready and closes once, keeping the context's side |
| `document promote`, prepare `integration_conflict` | the same four lines | not walked: see *Gap* below | the document's path is new to primary, so the conflict is an add/add, external only (decision 30) |
| `ticket close`, prepare `primary_ahead_of_remote` | `poll once`, `repo sync`, "prepare again" | **`c5_lifecycle_walk_primary_ahead_close`** (new) | free: the sync publishes primary, the close finishes once |
| `ticket close`, prepare `primary_missing` | `poll once`, "prepare again" | **`c5_lifecycle_walk_primary_missing_close`** (new) | free |
| `ticket close`, prepare `context_missing` | `ticket save … --input -`, "prepare again" | **`c5_lifecycle_walk_context_missing_close`** (new) | free: the save makes the context, the close finishes once |
| `poll once`, `busy` | `request.retry`, `operation list`, `operation resume --operation-id <operation-id>` | **`c5_lifecycle_walk_poll_busy_route`** (new) | free: the resume of the request's own operation (from `request show`) names only that request (`original_request_required`), and once the lease is let go the same request polls under its operation. **Finding:** `operation list` names nothing here, since the holder is a lease outside any operation, so the note's "operation list names it" doesn't hold for this holder. The row doesn't assert the list's content. |
| `poll once`, host and unlock stops (final, "a new request") | the stop's action, then a new request | `c5_polling_host_stop_is_final_then_a_new_request`, `_unlock_stop_*` (9) | free |
| `context sync-local`, conflict | `show-local`, `resolve-local` | Task 10b's walk; `c5_interruption_*_resolve_local` (13a) | free |
| `context sync-local`, `identity_required` | `repo.identity_set`, the same request | `no_identity_on_a_merge_is_identity_required_then_the_same_request` (10a) | free |
| `context resolve-local`, after a partial resolution | `request.retry` of the original | `a_partial_accepted_resolution_is_finished_by_the_same_request_and_source` (10b) | free |
| `context sync-local` stale, `resolve-local` invalid | `observe-local` / `show-local` | the 10a and 10b rows pin the line, and a fresh observation then goes on; this walk didn't repeat them | read again, as the row shows |
| either verb, `identity_required`; `discovery_pending` under a resume (`index.refresh`) | `repo.identity_set`; `index.refresh` | not through the binary here: the library's identity rows (Task 6) and `c5_resume_index_pending` | as the library rows |

**Gap, recorded.** A promotion's `integration_conflict` at prepare prints the four `context` lines, as a
closure's does (`way_out`, Task 7's review). The document's path is new to primary, so the conflict is an
add/add, and `context resolve-local` refuses it as external only (decision 30). So those lines lead to the
external repair, which the note doesn't say. This is the same follow-up candidate as decision 30's.

### STOP for the owner: no CLI route frees a retained-conflict close

The three stop-for-owner walks all stop at the same point. **Even with no fence, no CLI route frees a closure
whose context child has retained a conflict.** The sequence, through the binary over SSH (`CliPair`, one clone,
closure only):

1. `ticket save` of `ITEM` in A, then `item sync`: the context is published.
2. A peer's commit on the remote's context branch rewrites the ticket's body, and A then saves a different
   body.
3. `ticket close --prepare`: `ok`. Then `ticket close --request-id R4 --confirm …`: `recovery_required`, exit 3,
   blockers `[context_conflicted, conflict_pending, context_behind_remote]`, with `next: request.retry
   request_id=R4` and `next: repo.inspect root=…`. `operation list` shows the parent `close` (`confirmed`)
   and its child `synchronize_context` (`interrupted`, `next_action: resume`). `poll status` shows no
   `active_operation_id`.
4. Each line followed:
   - **`request.retry`:** the same answer, every time.
   - **`repo.inspect`:** registration, branches and the remote, with nothing about the conflict.
5. Every other route tried:
   - `ticket close --prepare`: decision 33's hold, `next: … --request-id R4`, back to step 4.
   - `conflict show --operation-id` of the child, or of the parent: `invalid_input`, exit 2. The C4
     resolution route can't address a lifecycle's child, and `conflict resolve` has no observation to send.
   - `operation resume` of the child, or of the parent: `original_request_required`, exit 3, naming R4.
   - `context observe-local`: `worktree_conflicted`, exit 3.
   - `item sync`: `busy`, exit 5, after C2's retries.
   - `request cancel R4`, then R4: `cancelled`, 130, once. Then R4 is `recovery_required` again: the retained
     conflict is kept, as `c5_cancel_retained_conflict_is_kept` pins.
   - `operation abandon --prepare` of the parent, or of the child: `operation_not_found`, exit 2.

**The fenced variants answer the same:**

- **`key clear` then `key select`** of the same key, through the binary: both `ok`. Then R4 is
  `recovery_required` with the same lines, and `conflict show` is `invalid_input`.
- **A poll's plan failure:** `poll once` with an invalid configuration answers `invalid_configuration`,
  so the CLI doesn't fence; the domain's automatic poll does (`c5_cancel_poll_plan_failure_…`). Then R4 is
  as above.

**A released resolution with its branch deleted can't be staged through the binary.** Making the resolution
needs `conflict show` and `conflict resolve` of the child, and step 5 shows the CLI refuses both. The library
stages it (`c5_cancel_fenced_released_resolution_close`, stuck, and `_released_resolution_publishes`).

**The only way on is outside the CLI:** a Git repair in the context worktree (see `context observe-local`'s
`worktree_conflicted`), then the same request. I didn't try it, since the walk stops here by the plan. The
exploratory rows weren't committed, because they would pin a stuck answer. For a ruling: should `conflict
show` and `conflict resolve` address a lifecycle child's synchronization, or should the answer name another
route, or is this a Known Limit?

### Exit classes (E-8)

`tests/cli/collaboration_exits.rs` gains C5's set equality, beside C4's:

- **The union.** `c5_union()` expands the audit's C5 result-set table with the C4 refresh's group
  definitions, and asserts each row's count against the table's Count column. That gives 46 library codes.
  With the CLI's `usage_error`, `invalid_id` and `command_unavailable`, there are 49 codes.
- **`C5_ROWS`.** Each row names a `#[test]` of `tests/cli`, or a `c5_` case of `cli_remote`, where a C5 verb
  reaches the code through the binary. A remote reference counts when its literal name is registered, or when
  it comes from lifecycle's `both!` macro.
- **`C5_REASONS`.** Each unreachable code has its reason:
  - `confirmation_expired`: the clock;
  - `remote_unavailable`: no fixture;
  - `unlock_failed`: C2's shared prompt, which C4's PTY case reaches;
  - `worktree_conflicted`: `poll once` connects first, and the walk's `observe-local` sighting isn't pinned;
  - `discovery_pending`: staged in-process by `tests/cli/local_sync.rs`;
  - `command_unavailable`: R-8;
  - `unlock_required`, off Unix only.
- **The other checks** are C4's: every exit matches its code's class unless the row is partial, and every
  class from 0 to 130 is reached.

**New rows that reach codes.**

- `tests/cli`, with `poll once`:
  - `c5_poll_once_refuses_each_root`: the seven root codes;
  - `_refuses_a_remote_or_key_it_cannot_use`: `no_selected_key`, `key_unavailable`, `invalid_remote`,
    and `transport_unavailable` against a closed port;
  - `_of_an_unreadable_configuration_is_invalid_configuration`;
  - `_on_a_corrupt_registry_is_index_unavailable`;
  - `_with_an_unopenable_data_directory_is_internal_error`.
- `cli_remote`: `c5_lifecycle_exit_evidence_close`, with:
  - `confirmation_used`: the spent consent under a new request;
  - `key_rejected` from `poll once`;
  - `poll_yielding` from `poll once` under a staged poll reservation, with no connection.

**Seen first.**

- **Candidates that answered otherwise:** two candidate rows answered other codes, and were changed or
  dropped before anything was committed.
  - `poll once` with the remote deleted from Git answered `publication_remote_required`, not
    `invalid_configuration`. The row now uses an unparseable configuration.
  - `poll once` over a conflicted index answered `transport_unavailable`, 5, so `worktree_conflicted` has a
    reason, not a row.

  Both were choices of staging, not design rows.
- **A count of mine was wrong.** My first set-equality assertion expected 47 library codes. The audit's own
  per-row counts all held, and a separate expansion gives 46, so 46 is right.
- **The rules were checked by breaking them.** With the `busy` row removed, the set equality failed. With
  a case name misspelled, the reference check failed. Both were restored.
- **No design row answered otherwise.** The walk rows pin routes Parts A and B built, and passed on their
  first complete run. The evidence row's first version finished the close after a staged poll reservation,
  which nothing owns, so every later remote call answered `poll_yielding`. It now comes last.

### Verified (Tasks 13a and 13b, on the final tree)

| Command | Ran | Time |
| --- | --- | --- |
| `--test cli_remote -- c5_interruption` | 30 SSH cases, 0 failed (Task 13a) | 159 s |
| `--test cli_remote -- c5_lifecycle_walk` | 6 SSH cases, 0 failed, run in two filters, `_poll` after its last change | ~90 s |
| `--test cli_remote -- c5_lifecycle_exit_evidence` | 1 | ~15 s |
| `--test cli -- exits` | 40 (C2's `exits`, C4's and C5's `collaboration_exits`) | 17 s |
| `--test cli_remote`, whole | 154 SSH cases, 0 failed. This ran before `c5_lifecycle_walk_poll_busy_route` was added, which was then run alone. | 878 s |
| AGENTS.md's fast tier, verbatim, with `--no-fail-fast` | 79 results, 4917 passed, 0 failed | 1419 s |
| `cargo check --all-features`, `cargo fmt --check`, whole-tree `clippy --all-targets --all-features -D warnings` | ok | |

**A first fast-tier run** stopped at `--lib`: one failure, `closure::checkpoint::tests::remote_primary_advance_from_context_synchronized_is_primary_advanced`,
whose two `ClosureRecord`s printed alike. No `src/` file changed in either task. It passed three times run alone,
and in the whole rerun above, so I read it as load: I was compiling `cli_remote` beside it. Recorded as a flake,
for whoever sees it again.

## Tasks 12a and 12b: the review's changes (controller's rulings)

Base `4df5be71` (Tasks 13a and 13b committed); `origin/main` at `0dfd5a68` is contained. One commit, with
Task 13a's Review Focus 4 row.

1. **No duplicate close.** `closure_checkpoints` now counts the commits on primary's first-parent line whose
   `closed_at`/`closed_by` differ from their first parent's: a second close, or a rewritten closure, is
   another. Every row that counts still finds one. **The rows listed to take the first stop's fields answered
   otherwise in their premise:** at the first stop of `primary_advanced_continues` (`AfterContextSync`),
   `resume_with_no_record_continues` (`AfterConfirmed`), both deleted-branch routes (the child's fetch) and the
   host and unlock rows (the child's first connection), no checkpoint is written yet, on the context's branch
   or on primary; their later stops before the finish (`external_change`, `cancelled`, the resume's second
   stop) don't write one either. So each now asserts, at those stops, that no closure field exists anywhere
   (`no_closure_yet`), which makes the finishing call's values the first and only ones, counted once. The
   unlock row's in-process publication is the one stop that writes them: its fields are captured there and
   the terminal's finish must keep them, on primary and on the remote.
2. **The conflict journey is the item's.** Both clones save `ITEM`, A publishes, and B's `item sync`
   (`conflicts::SYNC`, now `pub(super)`) stops `recovery_required`, 3, blocked, naming the item
   (`scope.item_id`), the repository (`scope.repository`) and the context (`scope.branch`, `scope.worktree`),
   with B's checkpoint kept. `conflict show` reads it (stage `context`, one path), `conflict resolve` with
   its tokens writes `[local, incoming]`, and the same sync request then answers `ok`, published. C4's
   primary conflicts are referenced in the doc.
3. **The close journey's injected failure.** The refused confirmation after a dirty primary names nothing:
   `external_change`, data null, human `next: request.prepare` only. So the journey now (a) refuses the
   prepare on a dirty primary, naming `primary_dirty` in its blockers, and (b) injects the failure after
   acceptance: the server drops the context synchronization's connection. That answer is
   `transport_unavailable`, 5, `request.retry` for the request, the lifecycle's data naming the item, and in
   human mode `progress: connecting` before `code: transport_unavailable`, which is where the step is named.
   The context, branch, worktree and remote are intact, and the same request then completes. The design's
   journey row says so.
4. **Stale-consent wording:** "a context change that changes the item's change set", in the Cycle row, the
   design's fact 7 and the earlier ledger section.
5. **Unlock wording:** the row's publication runs in this process (`AfterPublication`); the terminal's run is
   the cleanup. Corrected in the Task 12a staging paragraph and table row. The plan's text names no staging,
   so it is unchanged.
6. **Refused checkpoint merge:** the JSON resend asserts `recovery_required`, 4, partial, blockers
   `[integration_conflict]`, `recording: record`, `request.retry`, `repo.inspect`; the finished request's
   replay is identical with no connection. The four context lines now go through the shared `repaired`.
7. **Bytes:** both local-only journeys compare the item's file on the remote's primary with local primary's,
   and the promotion journey checks B's `document show` body and B's primary bytes.
8. **Background journey:** its doc says the peer writes the bare repository directly.
9. **`server.shutdown()`:** the local-only journey drops the server, as every other case does.
10. **Item path moved:** the ticket body is `ticket_source`'s. After the refusal a fresh prepare is `ok`,
    from the advanced primary. **For a ticket its `details.changes` do not differ, answering otherwise than
    the review's expectation:** the public change list holds path and category only, `modified` both times,
    and the blob that differs is D6's digest evidence with no DTO field. The row asserts they are equal for a
    ticket, and differ for a document, and says why; for the controller to confirm.
11. **PTY wait:** `PtyCli::exit_within` (additive; the default stays ten seconds) gives the unlock row 40 s.
12. **`c5_lifecycle_child_remote_branch_deleted`** asserts both verbs' data names the item, and a promotion's
    made-ready fields are null. Seen first: with the binding's child condition removed, the promotion case
    failed by assertion and the close passed; restored, both pass.
13. **Quality:** `ticket_path`, `ticket_source` and `own_commit` live once in `lifecycle.rs`;
    `collaboration::resume_args` is used directly; the moved-ticket source is `ticket_source`'s;
    `assert_refused_consent` takes the request.

**Review Focus 4, Task 13a's row, ruled.** Controller's ruling: a Ctrl-C after the last pre-publication safe
point whose push is then refused answers `cancelled`, 130, because the primary child's `Err` arm reads the
cancellation after the failed push, consistent with the RFC's classification order (cancellation first). The
design's `stops_on_ctrl_c` sentence is narrowed to say so. `c5_interruption_press_after_the_last_safe_point`
(both verbs), re-created from Task 13a's probe with the ruled expectation: 130, `integration=complete`,
`Publication pending: cancelled`, `Cleanup pending: awaiting_publication`, `next: request.retry`, no retry, one
refused primary push, the record accepted, then the same request publishes with one merge.

**Seen first.** Item 12 above. Items 2 and 3 were run against the code before writing their final
assertions, with the answers read (the refused confirmation's empty data is why item 3 changed). Item 1's
premise was probed at each stop before `no_closure_yet` was written. The rest pin what the code does.

**Verified.**

| Command | Ran | Time |
| --- | --- | --- |
| `--test cli_remote -- c5_lifecycle` | 32 SSH cases | 265 s |
| `--test cli_remote -- c5_journey` | 8 SSH cases | 56 s |
| `--test cli_remote -- c5_interruption` | 32 SSH cases (30 and RF4's two) | 180 s |
| `--test mutation_remote -- c5_lifecycle` | 152 SSH cases | 566 s |
| AGENTS.md's fast tier, verbatim | 79 results, 4917 passed, 0 failed | 1437 s |
| `cargo fmt --check`, whole-tree clippy `-D warnings` | ok | |

## Decision 34: the conflict verbs reach a lifecycle's child

Owner's decision 34 (2026-11-03), "Conflict verbs reach the child": `conflict show` and `conflict resolve`
accept a promotion's or closure's context child, using C4's remote-conflict read and resolution, and the
lifecycle's answer names them; then the same promote or close request finishes. Base `9aa3308a`;
`origin/main` at `0dfd5a68` is contained, so nothing was merged.

### What the library already did (probed before the design)

A probe row (not kept), on Task 13b's sequence in-process (`conflicted`, close):

- `show_conflict` of the **child's** ID: `conflict_pending`, `resolve`, one path. Of the **parent's**:
  `invalid_input` (its row's action is `close`, "no synchronization").
- `conflict resolve` of the child's ID, through the binding, a new request: `ok`, the resolution
  committed and the child published (row `completed`, with authority). Then the parent's same request:
  `ok`, cleanup complete, **one** merge on primary.
- Under `key clear` then `key select` (a raised generation): the read still says `resolve`, and the
  resolution answers `recovery_required`, `[repo.inspect]`, nothing written, because
  `reacquire_synchronization_conflict_in_window` requires the current generation. The parent stays
  `recovery_required`.

So the child's route works in the library through C4's code unchanged. What the binary met in Task 13b
(`invalid_input` for the child too) is examined by the walk below.

### Design

**How the read and the resolution identify a lifecycle's child.** As R-5 maps a comment's operation to its
synchronization, a lifecycle parent's operation ID maps to its **context child**: the `context_sync_id` its
own row (`document_promotions` or `ticket_closures`, same repository) holds now. The child's own ID keeps
working as before, since the child is a remote synchronization. One state helper reads the mapping
(`state::lifecycle_context_child`) and one the reverse (`state::lifecycle_parent_of`); both are plain reads
of the parent tables, with no new column.

- `conflict show` (`conflict_read::lookup`): after the local-sync check and the comment mapping, a parent's
  ID is looked up as its context child. A parent with no context child yet is a synchronization that
  recorded no conflict: `invalid_input`, as C4's row for that has it. A primary child is not mapped: the
  decision names the context child, and the primary child publishes without merging. The DTO is
  unchanged: `operation_id` is the child's ("the synchronization's ID, whichever ID was asked for"), and
  `comment_operation_id` stays a comment's only.
- `conflict resolve` (`ConflictResolveBinding::synchronization` and `settle_target`): the same mapping, so
  the request's identity and digest are of the child whichever ID names it, as for a comment. A request sent
  by the parent's ID and resent by the child's is the same request.

**Ownership, reservation and fencing.** The resolution takes nothing of the parent's:

- It runs C4's steps 1-7 on the child's row. `resolve_synchronization` reacquires the conflict under its
  reservation (`reacquire_synchronization_conflict_in_window`: the row interrupted or reconciling, the
  current generation, no flag on the row, the step's conflict digest, no other active operation, else
  `busy`). Its continuation is `sync::continued`, a restart with no `ChildExecution`, which bumps the
  owner epoch and so fences any older owner of the row, exactly as C4's continuation fences the
  synchronization's own request. Only one of the parent's runner and the resolution holds the reservation
  at a time; the other answers `busy`.
- **The parent's re-entry.** A published child has authority. `child_state` answers `Complete` for it, the
  runner adopts it through `stored_child_outcome`, and `ChildExecution::validate` accepts "an exact,
  current authoritative handoff" after any parent progress. A resolution checkpointed but not published
  (its continuation stopped) leaves a child with a resolution: `Retained` or, once released, `Unowned` with
  `promotion_child_has_resolution`, and the runner restarts it, as `c5_cancel_released_resolution_publishes`
  shows. A continuation cancelled at a safe point ends the row `cancelled` (C4's fact 56): the runner
  replaces it (`next_attempt` takes an unowned `cancelled` child), and the new attempt publishes the
  resolution commit the context branch holds.
- **Can a resolution finish a child the parent then can't adopt?** Not on any route found. While the
  conflict is retained the parent can't move off that child: `next_attempt` and
  `reset_context_after_primary_advance` refuse a retained child; release keeps it (below); a new prepare is
  held (decision 33); `operation abandon` doesn't name a parent. A parent that ended (abandoned or
  superseded) before its child conflicted has no child row to retain. So no STOP.
- **C4's R-38.** Unchanged: with a local-context sync unfinished anywhere in the repository, the
  resolution's reacquire refuses, `recovery_required`, `[repo.inspect]`, nothing written, and the attempt is
  kept for the same request. The parent's own call answers its own `recovery_required` meanwhile. No new
  check.
- **Decision 18's release guards.** Unchanged: a cancellation of the parent keeps a child with a retained
  conflict or a resolution, so a parent's `request cancel` never drops a resolution that is in hand.
- **Who stops the continuation (C4's holder rule).** C4 has the continuation read the flag of the request
  that holds the operation (`holder`). No request record holds a lifecycle's child, so the holder is the
  request whose record holds its **parent** (`records::operation_holder` of the parent, `Remote` family). So
  `request cancel` of the promote or close request stops a resolution's continuation at its next safe point,
  the flag left for that request's own next call, as C4 does for `comment add`. With no record holding the
  parent (a resume's), there is no holder, as before. A continuation that meets a stop the request can't get
  past offers `request cancel` of that holder (C4's `published`), which is decision 18's route.
- **Decision 25's effects.** Unchanged: a parent's stop reads its current context child's durable state. A
  resolution's merge on the context branch and its landed push are the child's, so a later stop of the
  parent reports them as its child's effects, and the finish counts one merge on primary.

**Which ID the user passes, and what the lifecycle's answer prints.** Either. The answer names the
**parent's**, which is the envelope's `operation_id`: it is stable while the parent mints child attempts, and
it is already in the answer, so neither the lifecycle DTO nor the recovery vocabulary changes (no
`RecoveryActionKind` may be added, as for the local repair). For a promote or close answer
`recovery_required` whose blockers include `conflict_pending`, the human output prints, before C2's lines:

```text
next: manyhands-cli conflict show --repo '<root>' --operation-id <parent>
next: manyhands-cli conflict resolve --repo '<root>' --operation-id <parent> --input - --request-id <new-request-id>
```

and a note: the context's synchronization stopped on a conflict; `conflict show` reads it (with `--json`
for the sides and tokens); when its next action is `resolve`, `conflict resolve` publishes it; when it is
`external_repair`, the repair is with Git in the context; then send this request again (or, under a resume,
resume again). C2's `request.retry` and `repo.inspect` lines follow, unchanged. Under `operation resume`
the resume's line closes the route.

**C4's contract for synchronizations** is unchanged: every C4 row keeps its answer, since only an ID that
named a lifecycle parent (until now `invalid_input`) reads differently, and a child's ID already read.
