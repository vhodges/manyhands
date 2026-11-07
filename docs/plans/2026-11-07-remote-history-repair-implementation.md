# Remote History Repair Implementation Plan

> **For agentic workers:** Revision 1, 2026-11-07. **Draft, awaiting owner
> approval**; no implementation before it. Work serially, task by task.
> Re-read the design before each task, and record every checkpoint on ticket
> `01M66HFV44YXNQ7ANJVA16TQ1H` (vh-hyksw). Steps use checkbox (`- [ ]`)
> syntax.

**Goal:** A key change or a passing configuration failure no longer marks
remote history unknown. A real remote change marks only the contexts that
existed then. An explicit repair, `remote history` plus `item
resolve-history`, clears what is unknown after checking the remote. Stuck
registries recover through a migration.

**Spec:** [design](2026-11-07-remote-history-repair-design.md), approved
2026-11-07 with all nine decisions as recommended. "§n" and "Decision n"
refer to it. Follow-on tickets, **out of scope here**:
- `01M6W5GZ3HCFREB7AYED504MP8` (vh-ez4y9), the desktop repair sheet;
- `01M6W5HFES1K2PPFB3NQ64B5TE` (vh-vsfbp), `busy` naming its holder, and
  `next:` lines that are runnable commands.

**Tech stack:** Rust 2024, `rusqlite`, `git2`, Devenv. No new dependency.

## Global Constraints

- **Where.** In `.manyhands/worktrees/01M66HFV44YXNQ7ANJVA16TQ1H` on
  `manyhands/ticket/01M66HFV44YXNQ7ANJVA16TQ1H`. Every Rust command is
  `devenv shell -- cargo … --locked`.
- **Main.** Merge `origin/main` at Task 0 and whenever main moves; never
  rebase. Check `git merge-tree --write-tree HEAD origin/main` before
  review-ready.
- **Boundary.**
  - **Library:** `src/repository/remote/`, `src/repository/mutation/`,
    `src/repository/promotion/observe.rs`, `src/repository/closure/observe.rs`,
    `src/repository/recovery.rs` (marker deletion only), `src/repository.rs`
    (the public service surface only).
  - **CLI:** `src/bin/manyhands-cli/`.
  - **Desktop:** guidance text only, in `src/desktop/view/lifecycle/prepare.rs`
    and `src/desktop/model/publication.rs`.
  - A gap outside these stops the task and goes to the owner.
- **Schema.** Additive only: new columns on `remote_polling_state`, and one
  `registry_migrations` marker, `remote_history_per_context_v1`. No change
  to the CHECK constraints on existing columns, except the
  `remote_poll_dispositions.disposition` CHECK, which gains `unpublished`
  (Task 4). That table is rebuilt through the existing pattern in
  `local_sync/record.rs:200-212`.
- **Privacy.** No locator, key path or key id in a new column, digest or
  output. The endpoint digest stays blake3 over the URLs, as now.
- **Tests** (AGENTS.md "Testing Strategy"):
  - Between steps: `cargo check`, plus the tests just written, by exact
    name (`--lib <module>::<test> -- --exact`).
  - Tables over cases, an injected clock, local `git2` repositories and
    `file://` remotes.
  - **No SSH case** (the design's Testing section says why).
  - The fast tier and the SSH targets run **once**, at Task 13.
- **Ledger.** Record in the ticket, at each checkpoint (Tasks 0, 5, 9, 13),
  the tests that ran and their results, as a Manyhands comment on
  `01M66HFV44YXNQ7ANJVA16TQ1H`.

## File Map And Dependency Order

| Task | Files (main ones) | Depends on |
| --- | --- | --- |
| 1 Reasons and known identity | `remote/state.rs`, `remote/observation.rs`, `remote/poll.rs`, `keys/registry.rs` (call sites), `repository.rs:3456` | 0 |
| 2 Carry-forward | `remote/state.rs` | 1 |
| 3 Per-context unknown | `remote/state.rs` | 1 |
| 4 `unpublished` disposition | `remote/poll.rs`, `remote/state.rs` (CHECK), `mutation/bind/poll.rs`, `mutation/dto.rs`, CLI poll output | 3 |
| 5 Migration (checkpoint) | `remote/state.rs` | 1, 3 |
| 6 Repair read | new `remote/history.rs`, `repository.rs` | 3 |
| 7 Resolve verb (library) | new `mutation/bind/resolve_history.rs`, `mutation/mod.rs`, `command.rs`, `dto.rs`, `remote/state.rs`, `recovery.rs` | 6 |
| 8 Final refusal and way out | `remote/sync.rs`, `remote/refs.rs`, `mutation/outcome.rs`, `mutation/way_out.rs`, `promotion/observe.rs`, `closure/observe.rs`, `bind/lifecycle_verb.rs` | 3, 7 |
| 9 CLI (checkpoint) | `src/bin/manyhands-cli/` (dispatch, catalog, collaboration/admin, human_*) | 6, 7, 8 |
| 10 Desktop text | `src/desktop/view/lifecycle/prepare.rs`, `src/desktop/model/publication.rs` | 8 |
| 11 Named test edits | listed in the design | 1–8 |
| 12 Documents | `docs/RFC/cli-contract.md`, Wave 02 Cycle 04 doc | 9 |
| 13 Gates, review-ready | none | all |

## Task 0: Baseline

- [ ] `git fetch origin && git merge origin/main` in the worktree.
- [ ] `devenv shell -- cargo check --all-features --locked` is green.
- [ ] Run once, to learn the baseline, the existing tests the design names
  for change. Record which pass:
  - `--test remote_polling history_unknown -- --exact`
  - `--lib remote::poll_tests::task6_config_and_selected_key_source_after_fetch_fence_before_batch -- --exact`
  - `--lib remote::state_tests`
  - `--lib remote::refs_tests`

  `document_promotion` is an SSH target: don't run it here.
- [ ] Settle the open question from the design's Testing section: is the
  reset at `closure/test_support.rs:203` dead? Temporarily make it a
  `debug_assert` that the flag is already 0, run
  `--lib closure::observe_tests`, record the answer, and revert.
- [ ] Ticket comment: the plan has started, plus the baseline results.

## Task 1: `ConfigurationChange` And Known Identity (§1)

- [ ] **Test first.** In `remote/state_tests.rs`, one table-driven test,
  `configure_reasons_fence_and_forget`. Rows:

  | Prior state | Call | Generation bumps | Forgets | Flag |
  | --- | --- | --- | --- | --- |
  | no remote | first plan | yes | no | 0 |
  | plan A | `Credentials` | yes | no | 0 |
  | plan A | `Unavailable` | yes | no | 0 |
  | after `Unavailable` | plan A again | yes | no | 0 |
  | after `Unavailable` | plan B | yes | yes | 0 |
  | plan A | a new primary branch | yes | yes | 0 |
  | digest X | digest Y | yes | yes | 0 |
  | digest X | digest X | no | no | 0 |
  | marker-set flag | `Credentials` | yes | no | stays 1 |

  "Forgets" means the context rows are rewritten to unknown (Task 3). In
  Task 1, assert only the generation, the flag, and the
  `known_remote_name`, `known_primary_branch` and `known_endpoint_digest`
  values.
- [ ] Add the columns (`known_remote_name TEXT`, `known_primary_branch TEXT`,
  `known_endpoint_digest BLOB` with the same CHECK as `endpoint_digest`)
  through the `PRAGMA table_info` additive pattern in `state::migrate`
  (`remote/state.rs:113-160`).
- [ ] Add `pub(in super::super) enum ConfigurationChange { Credentials,
  Unavailable, Plan }`. `configure(tx, id, plan, change)`:
  - **`Unavailable`** (plan `None`): fences, sets `remote_name` and
    `primary_branch` NULL as now, and leaves the known columns alone.
  - **`Plan`** with `Some(plan)`: forgets only if the known columns are set
    and differ from the plan. The first plan sets them.
  - **`Credentials`**: fences and forgets nothing.
  - **No path** sets `history_unknown`. Delete the `CASE` at `:2302`.
- [ ] `configure_endpoints` and `configure_poll_endpoints` compare the digest
  with `known_endpoint_digest`, so a digest unchanged after `Unavailable`
  forgets nothing. A changed digest forgets. A `NULL` known digest, which
  is the pre-migration state, takes today's `has_history` rule for whether
  to fence, and doesn't forget (Task 5 initializes it).
- [ ] Call sites:
  - `invalidate_key_selection`: `Credentials`.
  - `observation.rs:150` and `poll.rs:1321`: `Unavailable`.
  - `observation.rs:270`: `Plan` when `current_plan != plan`, otherwise
    `Credentials`.
  - `repository.rs:3456`: `Plan`.
  - Test call sites (`cleanup_tests.rs`, `refs_tests.rs`, `record_tests.rs`)
    get the reason that matches their intent.
- [ ] Until Task 2 exists, a fence that forgets nothing still deletes the
  rows. Mark the spot `// Task 2` and keep the tree green.
- [ ] Run the new test and `cargo check`.

## Task 2: Carry The Evidence Forward (§1)

- [ ] **Test first**, `credentials_fence_carries_batch_and_rows`. Set up a
  current batch with an observed context row and a `remotely_deleted` row,
  run `Credentials`, then check:
  - `read_snapshot` succeeds;
  - the new current batch has the new generation and the same observations;
  - both rows are kept;
  - the old batch is gone, or kept un-current if a window pins it.

  Add a second row to the table where a window pins the batch, and check
  that the startup audit (`read_snapshot_rows(.., true)`) passes.
- [ ] In `configure`, for a fence that doesn't forget:
  1. Insert a copy of the current batch at the new generation, with
     `is_current=1`, and copy its `remote_ref_observations`.
  2. Un-current the old batch, then delete it unless pinned, as now.
  3. Keep `remote_context_states`.
- [ ] Check `persist_complete_advertisement`'s precondition
  (`policy.generation == generation`) against the carried batch, and keep
  the order of operations inside the one transaction.
- [ ] Run the test, then Task 1's test, which now asserts that the rows
  survive.

## Task 3: Unknown Per Context (§2)

- [ ] **Test first**, extending Task 1's table with a "Forgets" assertion:
  after a forgetting change, every `kind='active'` context has an
  `('history_unknown','history_unknown')` row, and a context registered
  afterwards reads as `NeverPublished`. Add
  `evidence_defaults_by_flag_and_rows`, a table over flag ∈ {0, 1} × row ∈
  {none, `never_published`, `history_unknown`, `observed_published`} ×
  retired ∈ {no, yes}, asserting `fetch_publication_evidence_for` and
  `read_snapshot` acceptance.
- [ ] For a forgetting change: delete the rows, then insert an unknown row
  for each active context, using the `SELECT branch,item_id FROM contexts
  WHERE kind='active'` read at `:6301-6305` and `RemoteRefPlan::context`
  for the refs under the **new** plan.
- [ ] `read_snapshot_rows` `:6254`: the `never_published` rejection stays
  only while the flag is set. That covers cache loss alone now, because
  no path through `configure` sets the flag any more.
- [ ] Re-run the existing `state_tests` that cover R1 retirement and the
  invariants (`:13`, `:143`, `:1735-1743`, `:1800-1849`). Pin each one's
  outcome in the ledger, and update only those whose premise was a flag set
  by `configure`.

## Task 4: The `unpublished` Disposition (§2, Decision 6)

- [ ] **Test first**, in `remote/poll_tests.rs`, one table: absent and
  `NeverPublished` gives `unpublished`; absent and `HistoryUnknown` gives
  `history_unknown`; absent and previously `ObservedPublished` gives
  `remotely_deleted`.
- [ ] `PollTrackingState`, `PollDisposition` and the guidance text (`poll.rs:69-71`,
  `:216-217`, `:1696-1710`).
- [ ] Persistence: add `unpublished` to the `remote_poll_dispositions` CHECK
  (`state.rs:320`) by rebuilding the table, with marker
  `remote_poll_dispositions_unpublished_v1`, then the name and parse arms
  (`:441`, `:461`).
- [ ] DTO `"unpublished"` (`mutation/bind/poll.rs:301`, `dto.rs:1617`) and
  CLI human output, plus the contract note (Task 12).

## Task 5: Migration For Stuck Registries (§5), Checkpoint

- [ ] **Test first**, `history_per_context_migration`, a table:

  | Flag | Marker file | Context rows | After |
  | --- | --- | --- | --- |
  | 1 | absent | some missing | unknown rows written, flag 0 |
  | 1 | present | — | unchanged |
  | 0 | — | — | unchanged |
  | migrated twice | — | — | idempotent |

  Every row also checks that the known columns are initialized from
  `remote_name`, `primary_branch` and `endpoint_digest`.
- [ ] Implement it in `state::migrate` under marker
  `remote_history_per_context_v1`. The marker check uses
  `recovery::remote_history_lost` on the registry's directory, as `:39-46`
  does.
- [ ] **Checkpoint:**
  - `cargo clippy --all-targets --all-features --locked -- -D warnings`;
  - Tasks 1–5's tests by name;
  - `--lib remote::state_tests`;
  - `--lib remote::poll_tests`;
  - `--lib remote::refs_tests`.

  Write a ticket comment with the results, merge main if it moved, and push.

## Task 6: Library Repair Read (§3)

- [ ] New `src/repository/remote/history.rs` with `RepositoryService::remote_history(root,
  session) -> Result<RemoteHistoryReport, RemoteObservationError>`:
  1. Run the one-shot observation (the `poll once` path, reserved, a
     complete fetch).
  2. Read the snapshot.
  3. For each context whose fetch evidence is `HistoryUnknown`, or that the
     fetch just turned `observed_published` from an unknown row, build a
     `RemoteHistoryEntry` with:
     - kind, id and branch;
     - the local head;
     - whether a tracking ref exists;
     - the advertised OID, or `None`;
     - the relation (`Same | LocalAhead | RemoteAhead | Diverged`, through
       `poll::classify_relation`) and `ahead`/`behind` counts from
       `git2::Repository::graph_ahead_behind`;
     - `resolved: bool`;
     - `choices: {publish, release}`, which are offered only when the
       context is absent from the remote and unresolved.

  The report also carries `cache_loss: bool` (the flag) and the generation.
- [ ] Capture the "resolved by this fetch" set inside the observation
  transaction, from the rows before and after. Don't infer it later.
- [ ] **Tests**, in `remote/history_tests.rs`, one table over a `file://`
  remote with an injected clock. Cases:
  - absent and unknown;
  - present and same;
  - local ahead 2;
  - remote ahead 1;
  - diverged 1/1;
  - a cache-loss flag with a context that has no row.

  Assert the entries and the choices.

## Task 7: `item resolve-history` In The Library (§3, Decisions 2, 4, 7)

- [ ] Write the binding `mutation/bind/resolve_history.rs` from
  `mutation/bind/republish.rs`:
  - `ItemResolveHistoryInput { root, kind, id, choice: Publish | Release }`.
  - `observe()` reads, with no reservation, the context's branch and head,
    whether the tracking ref exists, the advertised OID from the
    **current** batch, the generation, and its evidence.
  - `digest()` covers all of them plus the choice.
  - **Preview cases:**
    - **publish:** `Previewed`, with effect `HistoryResolved`, when the
      context is unknown and absent;
    - **release:** previews nothing itself. It refuses with a way out naming
      `context release` (Decision 2), with no effects;
    - **already resolved:** `AlreadyApplied`;
    - **present on the remote:** `AlreadyApplied`, naming `item sync`;
    - **`remotely_deleted`** (published, then seen gone): refused, with a
      way out naming `item republish`. `resolve-history` never stands in
      for a republish confirmation.
- [ ] Domain `RepositoryService::resolve_history(..)`, in one transaction
  under the repository's lease. It re-reads, refuses `ExternalChange` if the
  digest's inputs moved, and writes the context's
  `('never_published','history_unknown')` row. If the cache-loss flag is
  set and no context still reads unknown afterwards, it clears the flag.
  After the commit it deletes the marker file through a new
  `recovery::retire_remote_history_marker` (Decision 7). A failure to
  delete the file is reported but doesn't undo the commit: the migration
  ignores a marker when the flag is 0 and no rows say unknown.
- [ ] Registration:
  - `Command::ItemResolveHistory => "item resolve-history"` with
    confirmation;
  - `Mutation::ItemResolveHistory`;
  - DTOs for the input, preview and result;
  - `next:` is `item sync` after publish.
- [ ] **Tests**, one table in the binding's tests:
  - prepare and preview per case above;
  - a confirm after the batch changed (simulate a new
    `persist_complete_advertisement`) gives `ExternalChange` plus a fresh
    prepare;
  - publish writes `never_published`, and a following
    `plan_clean_integration` succeeds;
  - the last resolution clears the cache-loss flag and calls the marker
    deletion;
  - replaying the request gives the recorded result.

## Task 8: Final Refusal And Truthful Way Out (§4)

- [ ] **Test first**, one table across sync, promotion and closure:
  - unknown from evidence: the sync is final, the reservation is released,
    and recovery is `[RemoteHistory]`; the lifecycle way out is
    `ResolveHistory`;
  - unknown from push proof of an earlier generation: resumable, and names
    `poll once`, as now.
- [ ] Split the cause: `CleanIntegrationError::HistoryUnknown` (evidence)
  versus the `PushAbsenceBoundary::Unknown` path (`sync.rs:452-457`,
  `:6225`, `:6605`). Evidence-based unknown becomes a terminal
  `SynchronizationError::HistoryUnresolved` → `ResultCode::RecoveryRequired`
  with a final record and recovery `[RemoteHistory]` (`outcome.rs:231`,
  `:819-828`). Put no `request.retry` first for it.
- [ ] `LifecycleWayOut::ResolveHistory` (`way_out.rs:247-262`) when every
  blocker is `HistoryUnknown` from evidence. Promotion and closure's
  `remote_context_history` carries the cause (`promotion/observe.rs:755-764`,
  `closure/observe.rs:790-806`).
- [ ] Update `mutation/c5_lifecycle.rs:3235` (a named edit) to expect the
  new way out.

## Task 9: CLI, Checkpoint

- [ ] `remote history --repo <path> [--json]`: catalog entry,
  dispatch, human output. One line per entry, for example
  `01M…  ticket  vh-xxxxx  absent  local-only  choices=publish,release`, or
  `present  diverged ahead=1 behind=1  resolved`, plus a `Cache loss:`
  line. `next:` lines are full `manyhands-cli item resolve-history …`
  commands.
- [ ] `item resolve-history --repo --id --as publish|release` through the shared
  confirmed flow (`collaboration.rs` `item()`/`run()`). Human preview and
  result.
- [ ] Human lifecycle output: `ResolveHistory` renders `next: manyhands-cli
  remote history --repo …` (`human_lifecycle.rs:284-311`), and
  `UNKNOWN_NOTE` is rewritten to match.
- [ ] **CLI tests** (`tests/cli`, `file://` remote), two journeys:
  1. Key select after a remote is set, then `item sync` of a new context
     publishes.
  2. Change the remote's URL, then `remote history` (lists the old context),
     then `item resolve-history --as publish --prepare` and `--confirm`,
     then `item sync` publishes. A context created after the change syncs
     with no repair.

  Add one golden fixture for `remote history --json`.
- [ ] **Checkpoint:** clippy, plus Tasks 6–9's tests by name. Then a ticket
  comment, merge main, and push.

## Task 10: Desktop Guidance Text (Decision 8)

- [ ] `src/desktop/view/lifecycle/prepare.rs:76` and `:97-99`: for
  `ResolveHistory`, say that the item's shared copy can't be confirmed and
  to run `manyhands-cli remote history` (the sheet is vh-ez4y9).
  `src/desktop/model/publication.rs`: show the same guidance for a final
  `recovery_required` from unresolved history.
- [ ] A model test asserts the way out is mapped to that guidance, not the
  text itself (the desktop rule). The desktop tests compile:
  `cargo check --features desktop`.

## Task 11: Named Edits To Existing Tests

Each edit names its test in the commit message.

- [ ] `tests/remote_polling.rs:1613`: a key selection now gives
  `unpublished`.
- [ ] `tests/document_promotion/journeys.rs:780-830` `unknown()`: keep
  `lose_registry`; the step through `select_key` asserts the flag is still
  1 because of the loss, not the key. **SSH target:** edit it here and run
  it at Task 13.
- [ ] `remote/poll_tests.rs:1990`: assert the carry-forward.
- [ ] `tests/remote_synchronization.rs:1069` and
  `tests/ssh_transport/synchronization.rs:1113`: the push-proof cases, which
  should pass unchanged. Check them at Task 13.
- [ ] Remove each test-only flag reset that Task 0 found dead
  (`closure/test_support.rs:203`, `promotion/run_tests.rs:256`,
  `integrate_tests.rs:167`, `observe_tests.rs:375`).

## Task 12: Documents

- [ ] `docs/RFC/cli-contract.md`:
  - remove the key-selection known limit (around line 1506);
  - add `remote history`, `item resolve-history` and the `unpublished`
    disposition;
  - amend the `history_unknown` guidance (around line 1591).
- [ ] `docs/Cycles/wave-02-cycle-04-remote-observation-and-recovery.md`:
  one amendment line saying that the "later explicit recovery" of the
  2026-10-05 rule is this repair.

## Task 13: Gates And Review-Ready

- [ ] Merge `origin/main`, then check `git merge-tree --write-tree HEAD
  origin/main`.
- [ ] Static gates: `cargo check --all-features --locked`,
  `cargo fmt --check`, and `cargo clippy --all-targets --all-features
  --locked -- -D warnings`.
- [ ] The fast tier, exactly as main's `AGENTS.md` lists it.
- [ ] The SSH tier, once each:
  - for `src/repository/remote/`: `ssh_transport`, `remote_observation`,
    `remote_synchronization`, `remote_merge_recovery`, `remote_polling`,
    `cli_remote`;
  - for `promotion/`: `document_promotion`;
  - for `closure/`: `ticket_closure`.

  Stop any command at about 15 minutes, narrow it, and note it in the
  ticket.
- [ ] Owner check: on the real registry, `remote history` lists nothing
  unresolved, and a `key select` of the same key followed by `item sync` of
  a new ticket publishes.
- [ ] Ticket comment: review-ready, with every test command and result.
  Closure is through `ticket close`, after review approval.

## Plan Self-Review

- **Design coverage:**
  - §1 → Tasks 1–2; §2 → Tasks 3–4; §3 → Tasks 6–7, 9; §4 → Tasks 8, 10;
    §5 → Task 5.
  - Decisions 1–7 are covered by those tasks.
  - Decisions 8 and 9 are the follow-on tickets.
- **Ticket acceptance:**
  - key select, then publish, promote and close: Tasks 1, 8, 9 (journey 1);
  - remote change: Task 3 and journey 2;
  - repair lists and resolves: Tasks 6, 7;
  - prepare fetches, relation table, no overwrite: Tasks 6, 7;
  - stale confirm: Task 7;
  - truthful refusal: Task 8;
  - deleted-after-publish still refused: Task 4's table and Task 7's
    `AlreadyApplied` and refusal rows;
  - stuck registry: Task 5;
  - contract: Task 12.
- **Riskiest steps:** Task 2 (carry-forward against the startup audit) and
  Task 8 (splitting the two causes of `HistoryUnknown`). Both are
  test-first, with their own tables.
