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
  `manyhands.sqlite3` held both tables when opened read-only. **The guards differ:**
  records take `cache_write_guard` over the data directory, and `with_transaction`
  takes `cache_read_guard` over the root. Task 5a's boundary path takes the write
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

**Tests first.** 31 `c5_resume_` cases and the E-10 row ran against the unchanged library. 29 cases and the E-10 row failed by assertion on the answer. `pending_child`, both verbs, passed: it pins what exists. `no_record_past_integration` and `complete_with_record_accepted` first failed on fixture faults (below). Once those were fixed they were run against the old library again, with its three files put back for the run, and failed by assertion on the code (`operation_not_found`'s line).

**Fixture faults, none a changed answer.**

- A domain stop past integration is an outcome with cleanup pending, not an error, so `begun_outside` doesn't assert an error.
- A call that dies at `BeforeRequestSettlement` names no operation, so that row reads the ID from the parent table.
- Past integration, one further accepted update is the context's deletion. The row asserts `pushes + 1`, and that primary doesn't move.
- `grants_no_new_consent` first expected `request.prepare`. A resume offers nothing there, because its command has no confirmation. **For review:** the row pins an empty list, so the answer doesn't say that a new prepare of the verb is the way on.
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
- **`request cancel` of a resume that is continuing a parent stops it,** because that call's safe points read the resume's flag. By reading only, with no row. The binding's doc says so.

**Deviation from the task's file list.** One comment in `bind/lifecycle.rs`'s header, which still listed resume as Task 3's to do. It is within the library scope.

**The Ctrl-C row.** `ctrl_c` runs at the library, by a token cancelled in the request hook: `cancelled`, no record, `operation.resume`, then a finish with one merge. `stops_on_ctrl_c` for `operation resume` is Task 7's.

**Risks, for review.**

- **A resume's `discovery_pending` offers nothing.** The parent is complete by then, so `offering_resume` drops the action, and the next resume is `already_applied`. Pinned by `index_pending`. Nothing in the answer says how the hand-off gets done.
- **A child's `remote_branch_deleted` under a resume has no row.** By reading, `request.cancel` is dropped, because a resume's envelope names no operation, which would leave `repo.inspect` alone. Decision 18's release is reached through a request record's cancel.

**Left.** Task 4: made-ready in the resume's data, so the plan's "pending child … and its made-ready" row is only half done here. Task 7: `stops_on_ctrl_c`.
