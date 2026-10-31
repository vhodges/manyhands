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
- `Binding::prepare` checks nothing, so a state blocked since its preview is `external_change` with no record. `Blocked` at run is reached only after acceptance, and a case stages it with the request hook.
- The domain reports an unknown ID as `ContextMissing`, not `NotA*`. The binding asks the index: `ContextMissing` with `show_item` answering `item_not_found` is `item_not_found`. So an item on primary with no context, under a stale index, answers `item_not_found` until `index refresh`, as `ticket show` does (Task 0). The context-missing row refreshes first.
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
