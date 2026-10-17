---
title: "Wave 02 Cycle 10 Confirmed Ticket Closure Execution Ledger"
date: 2026-10-17
status: active
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M55JWY66M5BCCAWCGEQX9N9V"
---

# Confirmed Ticket Closure Execution Ledger

This ledger records the execution of the
[implementation plan](2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-implementation.md)
for the [design](2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-design.md)
and the [Cycle](../Cycles/wave-02-cycle-10-confirmed-ticket-closure.md).
Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DJ`.

## Task 0 — Base, Decisions, Characterization, Baseline (2026-10-17)

**Status: blocked on one design contradiction (characterization 4).** Every
other finding agrees with the design. No source or test change remains.

### Owner approvals

Recorded from the owner's 2026-10-17 approval:

- The Cycle document, the design and the implementation plan, with owner
  decisions C1–C12.
- Named existing-test edit 1: `tests/local_authoring.rs`
  `ticket_edit_preserves_unknown_closure_and_exact_body` becomes a refusal
  test (Task 7). Named edits 2–4 are part of the approved plan.
- Implementation authority; execution mode subagent-driven development; task
  checkpoint commits on the ticket branch.
- Native CI runs after merge, on main; it is not a merge gate.
- Push, pull request, native dispatch, merge, ticket closure and cleanup each
  still need their own authorization. Nothing was pushed.

### Base

- `git fetch origin`: `origin/main` is still
  `37655241276c44557d191e56708b17042d233b7b` (`3765524`), unchanged since
  planning. No merge was needed.
- `HEAD` is `e8af026214286c66b32fbae445962f54c833c653` (approved planning
  checkpoint). `git merge-base --is-ancestor origin/main HEAD` holds. Status is
  clean apart from the untracked `.superpowers/`. `AGENTS.md` is unchanged.

### Re-scan of the source boundary

**Calls `promotion/run.rs` makes** (Task 1 Step 1 maps each to a trait method):

| Function | Lines in `run.rs` |
| --- | --- |
| `record::read_record` | 155, 167, 318, 403 |
| `record::save_record` | 170 |
| `record::begin_record` | 323, 424 |
| `record::replacement_guard` | 360, 405 |
| `record::refuse_record` | 414 |
| `record::clear_cancellation` | 450 |
| `record::reset_context_after_primary_advance` | 582 |
| `record::child_state` | 686, 739, 788, 897, 1111, 1271 |
| `record::child_execution` | 733, 772 |
| `record::next_attempt` | 763 |
| `record::complete_record` | 1307 |
| `record::reconcile_completion` | 1317 |
| `observe::preview` | 187 |
| `observe::prepare` | 368 |
| `integrate::reconcile_integration` | 275, 483 |
| `integrate::primary_clean` | 280 |
| `integrate::prepare_integration` | 573 |

Twelve distinct `record::` functions over 23 call sites (the design says "about 20"),
two `observe::` and three `integrate::` functions. Types used:
`record::ChildExecution` (the `PromotionIo` trait, lines 24 and 54),
`integrate::{IntegrationRequest, Integrated, IntegrationContext,
IntegrationState}`. The 13 `AuthoringKind::Document` literals in `run.rs`
match the design: lines 102, 130, 806, 1017, 1054, 1100, 1177, 1198, 1227,
1259, 1278, 1291 and 1305.

**Callers of the named sites:**

| Site | Callers (non-test unless noted) |
| --- | --- |
| `finish_promotion` | `reservation.rs:779` (definition), `promotion/record.rs:1032` |
| `restart_remote_promotion` | `reservation.rs:836`, `run.rs:1064`; tests `reservation_tests.rs:146`, `record_tests.rs:941` |
| `RetirementEvidence` | `cleanup.rs:52,61,369,413,583`; `promotion/record.rs:10,985` |
| `matches_promotion` | `cleanup.rs:64`, `promotion/record.rs:990` |
| `ChildExecution` | `promotion/record.rs:659–689`; `run.rs:24,54`; `reservation.rs:974,996,2344,2353`; `sync.rs:10,4040,4050,4242,4254,6236`; `run_tests.rs` (11 sites) |
| `reserve_promotion_child` | `reservation.rs:969`, `sync.rs:4245`; `run_tests.rs:333,877` |
| `RemoteOperationAction::Close` | `state.rs:1786` (`for_context`), `2628` (parser), `5236` (name); `remote/mod.rs:426` (test); `read/status.rs:203`; `state_tests.rs:3391`; `tests/read_status.rs:1261` |
| `save_ticket_with_effective_config` | `repository.rs:2342` (definition), `2351`, `2358` |
| `prepare_context` (public) | `repository.rs:1662`; `prepare_context_unlocked` at `1697`, `1734`, `1974`, `2395`, `2748` and `bridges/repair.rs:524` |
| `checkpoint_comment_locally` | `repository.rs:2677`, `comment_publication.rs:232`; tests in `sync_tests.rs` and `tests/remote_merge_recovery.rs:955` |
| `repair_item` | `bridges/repair.rs:448` (definition); `mutation/bind/repair.rs:593` |

**SQL literals naming `document_promotions`:** 27 in `promotion/record.rs`
(schema, index, migration, reads, collision and identity queries, updates,
audit); `reservation.rs:792` (finisher join); `read/status.rs:366,425,432`
(`has_table` and existence checks). `recovery.rs` names the table nowhere; its
local-ID cross-checks reach it through
`promotion::record::validate_local_identity` (`recovery.rs:684,1404`). Tests:
`tests/document_promotion*.rs`, `tests/recovery_foundation_gate.rs`,
`tests/repository_enablement.rs`.

**Document-only sites beyond the design's list** (each joins Task 1's list):

1. `transport/remote.rs:776` `exact_mappings`: the `SshDirection::Delete` arm
   matches only `AuthoringKind::Document`; every other target gets no
   mappings. The design lists this as kind-generic. See characterization 4.
2. `transport/remote.rs` unit test
   `transport_delete_scope_is_exact_document_context_only` asserts that a
   ticket context delete mapping is **not** admitted. Changing site 1
   changes this existing test semantically, which no named edit covers.
3. `reservation.rs:2339` `restart_promotion_child`, and
   `restart_synchronization_guarded`, take a `ChildExecution` and call
   `validate_remote_identity` (also at `reservation.rs:1009`, `2364`). The
   design names `reserve_promotion_child` only.
4. `promotion/record.rs:455`: the begin-record collision query hard-codes
   `action!='promote' OR kind!='document'` for `remote_operation_records`.
5. `promotion/observe.rs:150` (item lookup by `AuthoringKind::Document`),
   `:533`, `:601` (`plan.context(Document, ..)` tracking ref) and `:672`
   (`fetch_publication_evidence_for(Document, ..)`), in addition to the four
   observe sites the design names.
6. `run.rs:1053`: `RemoteOperationAction::Promote` literal in the cleanup
   reservation target.
7. `discovery.rs:1732` (`promotion::record::migrate`) and `state.rs:5337`
   (`promotion::record::audit`): Task 2 already names both.

Kind-neutral by content despite their names: `retains_promotion_child`,
`promotion_child_has_resolution`, the `PROMOTION_RESERVATION_HOOKS` test hook,
and `sync.rs` `promotion_test_*` helpers.

### Characterizations

Each was a throwaway test, run once and then deleted.

1. **Canonical re-serialization with closure fields — agrees.**
   `devenv shell -- cargo test --locked --test t0_char_canonical -- --nocapture`.
   Checked 20 tickets: the 18 `.manyhands/tickets/*/ticket.md` (8 already
   closed, round-tripped unchanged; 10 open, closed with the two fields), one
   CRLF ticket with `manyhands_*` keys first, quoted scalars and `wave`/`cycle`,
   and one ticket with unknown nested keys, `slug`, `parent` and sorted `deps`.
   For all 20, parse, set fields, `serialize_item`, re-parse gave the original
   plus the two fields. **0 failures.** Observed edge (not a repository
   ticket): `serialize_item` normalizes relationships, so a ticket with
   unsorted `deps`, or `parent: null`, re-parses with a different `unknown`
   map. A literal whole-struct comparison would report those as
   `TicketInvalidAfterClosure`. Task 3/4 needs a ruling on whether
   relationship normalization counts as "unchanged".
2. **An unreferenced ODB commit is visible to `prepare_clean_merge` —
   agrees.** `devenv shell -- cargo test --locked --lib t0_char -- --nocapture --test-threads=1`.
   A `Close ticket` commit written with `commit(None, ..)` (no ref)
   merged cleanly with an advanced primary. `import_prepared_tree` produced a
   tree holding the closed ticket and the primary's change. A conflicting
   unreferenced commit gives `MergePreparation::Conflict`. A missing commit
   gives `Err(RecoveryRequired)`.
3. **`sync::fast_forward` on the context worktree's repository — agrees.**
   Same command. A repository opened at a linked worktree on
   `refs/heads/manyhands/ticket/<ULID>` moved its branch from base to the ODB
   checkpoint. The context worktree status was clean, and the primary's HEAD
   and worktree were untouched. **Simulated stop** (the real `fast_forward`,
   with the existing `observation_tests` safe-point hook panicking at
   `BeforeLocalMutation`, between checkout and ref update): the ref stays at
   `context_oid`; the file holds the checkpoint bytes; the **index** holds the
   checkpoint Blob, so status is `INDEX_MODIFIED` on the ticket path only; no
   ref lock file remains. A second `fast_forward(base → checkpoint)` then
   succeeds with a clean status. The design's reconciliation row ("worktree
   differs only at the ticket path, bytes equal the stored Blob") must treat
   staged-and-worktree equality with that Blob as the same state.
4. **Ticket context remote delete — CONTRADICTS the design.** Same command.
   For `SynchronizationTarget::Context { kind: Ticket, .. }`,
   `exact_mappings(.., SshDirection::Delete)` returns `[]`, and
   `mappings_match_scope(.., Delete, [":refs/heads/manyhands/ticket/<ULID>"])`
   returns `false`, so `delete_exact` would refuse a ticket target. The push
   mapping for a ticket is exact as expected. The existing unit test
   `transport_delete_scope_is_exact_document_context_only` asserts this
   refusal. The design's "Kind-generic and reusable as is" list includes
   "the guarded remote delete (`SshDirection::Delete`, `delete_exact`,
   `exact_mappings` built from the target kind)". That is false at the base.
   Fixture: a throwaway case running the deletion receipt proof with
   `refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAV`
   (`devenv shell -- cargo test --locked --test ssh_fixture -- --exact t0_ticket_delete_proof`):
   **1 SSH case passed**, so the fixture's `update_effect_proven` is ref-name
   generic and accepts a ticket ref.
5. **Running one case of a `harness = false` target.**
   `devenv shell -- cargo test --locked --test <target> -- --exact <case>` runs
   one case (verified: `--test ssh_fixture -- --exact context_delete_proof_guards`
   reported `1 SSH cases passed`). Without `--exact`, the first free argument is
   a substring filter. Other accepted options: `--list`, `--test-threads N`,
   `--nocapture`, `--show-output` and `--quiet`. `--skip` is rejected with exit
   status 2 ("unsupported SSH runner options").
6. **Columns identifying C12 targets — agrees, with an implementation note.**
   `devenv shell -- cargo test --locked --test mutation_replay t0_char -- --nocapture --test-threads=1`.
   A ticket save stopped at `AfterOwnedWriteBeforeLifecyclePersistence` left
   `operation_records` row `action=save_ticket`,
   `target="authoring-context-v1/ticket/<ULID>/Edit\0"`, **`item_id` NULL**,
   `state=worktree_observed`. The `item_id` column is not written by
   lifecycle rows. The target item comes from parsing `target`, as
   `bridges/abandon.rs` `authoring_item` does. Pending means
   `state!='completed' OR completed_step='comment_registration_pending'`
   (`require_no_pending_local`, `comment_local_pending`).
   `comment_publication_bindings` names the ticket directly in
   `kind`/`item_ulid` (schema `recovery.rs:47`, insert `recovery.rs:320`). The
   binding has no state column. Whether it is unfinished is read through
   `local_record_id → operation_records` (same pending rule; its `target` is
   `authoring-context-v1/ticket/<ULID>/Edit\0.manyhands/comments\0<comment>`)
   and `synchronization_ulid → remote_operation_records`. A comment's own
   pending local row therefore also matches the ticket's `target` prefix.
7. **F2 `ticket save` replay after closure — agrees; Task 7 changes no
   binding answer.** Same command.
   - Finished request replayed after a closure in the context: the stored
     envelope is returned unchanged (`Ok`). No `operation_records` row is
     added and the tip is unchanged, so the domain is not called.
   - Request accepted with its output lost (`BeforeRequestSettlement`), then
     closed, then replayed: re-entry settles it from Git evidence as `Ok` with
     its commit, adds no row and finishes the record. The domain is not
     called.
   - A **new** request asking for the already-applied state on a closed
     ticket: `ticket_closed` (`Blocked`), with no operation and no record. The
     binding's `refuse_closed` in `prepare_save` runs before the
     already-applied rule.
   - A pending-row request (stopped after its write) retried after closure on
     the primary: the domain runs and commits (`Ok`). The base domain has no
     guard, and the design exempts resumed rows; C12 blocks closure while such
     a row exists.

### Targeted baseline (base `e8af026`)

Pre-build `devenv shell -- cargo test --locked --no-run --lib --tests`: 172 s.
Each row is `devenv shell -- cargo test --locked <args>`, run in the
background; all exited 0.

| Args | Result | Wall time |
| --- | --- | --- |
| `--lib promotion` | 193 passed | 288 s |
| `--lib cleanup` | 42 passed | 39 s |
| `--lib remote::reservation` | 45 passed | 17 s |
| `--lib read::status` | 10 passed | 1 s |
| `--lib recovery` | 46 passed | 9 s |
| `--test document_promotion` | 74 SSH cases passed | 835 s |
| `--test remote_synchronization` | 63 SSH cases passed | 725 s |
| `--test remote_merge_recovery` | 25 SSH cases passed | 246 s |
| `--test remote_polling` | 84 SSH cases passed | 200 s |
| `--test comment_publication` | 41 SSH cases passed | 216 s |
| `--test remote_reservation` | 13 passed | 4 s |
| `--test read_status` | 23 passed | 4 s |
| `--test mutation_remote` | 88 SSH cases passed | 880 s |

`devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`:
clean (exit 0, 162 s).

### Stop

Characterization 4 contradicts the design's source boundary. Generalizing the
delete scope to tickets needs an owner amendment. It means a production change
to `transport/remote.rs` `exact_mappings` (Task 1 or 5) and a semantic edit to
the existing unit test `transport_delete_scope_is_exact_document_context_only`,
which is not among the four named edits. Task 1 waits for the ruling. Open
point for the same ruling: characterization 1's relationship normalization
edge.

### Rulings after Task 0

- **R-T0-1, named edit 5.** Owner approved named edit 5 (2026-10-17). Task 5
  adds a ticket-context Delete arm to `transport/remote.rs` `exact_mappings`,
  with the same exact `:<context ref>` shape.
  `transport_delete_scope_is_exact_document_context_only` keeps all its
  document and malformed refusals; only its ticket assertion flips to
  accepted, and it gains refusals for a kind mismatch and a bad ticket ID.
  Task 1 does not change `remote.rs`.
- **R-T0-2.** The post-closure validity check compares after canonical
  normalization; normalized `deps` order or a dropped `parent: null` is not
  `TicketInvalidAfterClosure`.
- **R-T0-3.** Task 4 reconciliation accepts the ticket path staged in the
  index as well as in the worktree after a stop between checkout and ref
  update.
- **R-T0-4.** The C12 `LocalOperationPending` check finds a pending row's
  ticket by parsing `operation_records.target`.

## Task 1 — Lifecycle Seam (2026-10-17)

### Step 1: call map and document-only sites

**Runner calls → `Lifecycle` methods.** Every call below now goes through the
runner's type parameter `L` (`L::method(..)`, or `lifecycle.method(..)` for
the two methods taking `&self`). `DocumentPromotionLifecycle` forwards each
one to the unchanged promotion function, with the same arguments.

| Base call in `run.rs` (sites) | Trait method |
| --- | --- |
| `record::read_record` (4) | `Lifecycle::read_record` |
| `record::save_record` (1) | `Lifecycle::save_record` |
| `record::begin_record` (2) | `Lifecycle::begin_record` |
| `record::replacement_guard` (2) | `Lifecycle::replacement_guard` |
| `record::refuse_record` (1) | `Lifecycle::refuse_record` |
| `record::clear_cancellation` (1) | `Lifecycle::clear_cancellation` |
| `record::reset_context_after_primary_advance` (1) | `Lifecycle::reset_context_after_primary_advance` |
| `record::child_state` (6) | `Lifecycle::child_state` |
| `record::child_execution` (2) | `Lifecycle::child_execution` |
| `record::next_attempt` (1) | `Lifecycle::next_attempt` |
| `record::complete_record` (1) | `Lifecycle::complete_record` |
| `record::reconcile_completion` (1) | `Lifecycle::reconcile_completion` |
| `observe::preview` (1) | `Lifecycle::preview` |
| `observe::prepare` (1) | `Lifecycle::prepare` |
| `integrate::reconcile_integration` (2) | `Lifecycle::reconcile_integration` (default: `integrate::reconcile_integration_for(Self::KIND, ..)`) |
| `integrate::primary_clean` (1) | `Lifecycle::primary_clean` (default) |
| `integrate::prepare_integration` (1) | `Lifecycle::prepare_integration(&self, ..)` → `integrate::prepare_integration_for(self, ..)` |
| (new) | `Lifecycle::pre_integration(&self, &mut LeasedIntegration)`; promotion returns `Ok(PreIntegration::None)` |

Associated types: the brief's `Record`, `Error`, `Preview`, `Expectation`,
plus `Preflight`, `PreviewRequest`, `Observation`, `Refusal` and
`ReplacementGuard`, which today's argument and return types of the calls
above need (`DocumentPromotionPreflight`, `PreviewDocumentPromotionRequest`,
`observe::PreparedObservation`, `record::Refusal`,
`record::ReplacementGuard`).

**Document-only sites and their Task 1 treatment.**

| Site | Replacement | Task |
| --- | --- | --- |
| `run.rs` 13 `AuthoringKind::Document` literals | `L::KIND.authoring()` (two in the `Direct` delete adapter use the kind it carries) | 1 |
| `run.rs:1053` `RemoteOperationAction::Promote` | `L::KIND.action()` | 1 |
| `run.rs` `restart_remote_promotion` call | `restart_remote_lifecycle(L::KIND, ..)` | 1 |
| `run.rs` `synchronization_target`, `stored_child_outcome` | take a `LifecycleKind` | 1 |
| `integrate.rs` context ref | `L::KIND.context_ref(item)` | 1 |
| `integrate.rs` subject `Promote document <ULID>` (commit, resume check, reconciliation) | `kind.merge_subject(item)`; `reconcile_integration_for(kind, ..)` searches for it; `reconcile_integration` stays as a promotion wrapper | 1 |
| `observe.rs:150` item lookup by `Document`, `:145` branch name | `kind.authoring()`, `kind.context_branch(item)` | 1 |
| `observe.rs` `observation_guard` `root/docs`, `context/docs` and the document ref | `kind.guarded_directory()` (`docs` / `.manyhands/tickets`), `kind.context_ref(item)` | 1 |
| `observe.rs:533`, `:601`, `:672` target, tracking ref, publication evidence | `kind.authoring()` | 1 |
| `observe.rs` `DiscoveryFacts.not_document`, `result_document_path`, `NotADocument` | unchanged; `prepare_for(TicketClosure, ..)` returns `recovery_required` | 3 |
| `reservation.rs` `finish_promotion` | `finish_parent(kind, ..)`: today's check and SQL for `DocumentPromotion` (action from `kind.action()`); `TicketClosure` returns `recovery_required` | 1 (Close arm: 5) |
| `reservation.rs` `restart_remote_promotion` | wrapper over `restart_remote_lifecycle(kind, ..)`; `TicketClosure` returns `recovery_required` | 1 (Close arm: 5) |
| `reservation.rs` `reserve_promotion_child`, `restart_promotion_child`, `restart_synchronization_guarded` (child fence) | `ChildExecution` carries its parent's `LifecycleKind`; `validate` and `validate_request` return `recovery_required` for `TicketClosure`; the context target kind is `kind.authoring()` | 1 (ticket admission: 5) |
| `sync.rs` child fence calls (`validate_request`, `validate`) | unchanged source; the kind travels inside `ChildExecution` | 1 |
| `reservation.rs` `acknowledge` hold-the-slot and cancel arms (`Promote` only) | unchanged | 5 |
| `promotion/record.rs:455` collision SQL `action!='promote' OR kind!='document'`, `validate_remote_identity`, `existing_record`, `mint_attempt`, ID cross-checks | unchanged: promotion's own SQL; the closure table and cross-table queries come with the closure record | 2 |
| `cleanup.rs` `RetirementEvidence::matches_promotion` | unchanged | 5 |
| `transport/remote.rs:776` `exact_mappings` Delete arm and its unit test | unchanged (R-T0-1) | 5 |
| `read/status.rs` promotion parent listing | unchanged | 2 |
| `recovery.rs` local-ID cross-checks (via `validate_local_identity`) | unchanged | 2 |
| `discovery.rs:1732` migrate, `state.rs:5337` audit | unchanged | 2 |

The runner's generic bound is `run::PromotionShaped`, which pins the
associated types to the promotion types because the runner body still reads
`PromotionRecord` fields, `PromotionStep` values and
`DocumentPromotionError` variants. Task 6 must lift those (see the Task 1
report's concerns).

### Steps 2–5: tests and gate

- **Red.** `devenv shell -- cargo test --locked --lib promotion::lifecycle`
  with only the test file failed to compile (11 errors: `LifecycleKind`,
  `DocumentPromotionLifecycle`, `LeasedIntegration`, `PreIntegration`
  unresolved).
- **Green.** The same command passed 3 tests:
  `document_kind_matches_legacy_constants`, `ticket_kind_constants` and
  `document_promotion_has_no_pre_integration_effect`.
- **Gate.** No existing test file changed. Every Task 0 baseline count is
  unchanged:

| Args | Task 0 | Task 1 | Wall time |
| --- | --- | --- | --- |
| `--lib promotion` | 193 | 196 (193 + 3 new lifecycle tests) | 220 s |
| `--lib cleanup` | 42 | 42 | 31 s |
| `--lib remote::reservation` | 45 | 45 | 17 s |
| `--lib read::status` | 10 | 10 | 1 s |
| `--lib recovery` | 46 | 46 | 10 s |
| `--test document_promotion` | 74 SSH | 74 SSH | 829 s |
| `--test remote_synchronization` | 63 SSH | 63 SSH | 729 s |
| `--test remote_merge_recovery` | 25 SSH | 25 SSH | 247 s |
| `--test remote_polling` | 84 SSH | 84 SSH | 202 s |
| `--test comment_publication` | 41 SSH | 41 SSH | 218 s |
| `--test remote_reservation` | 13 | 13 | 5 s |
| `--test read_status` | 23 | 23 | 5 s |
| `--test mutation_remote` | 88 SSH | 88 SSH | 870 s |

  `cargo fmt --check` is clean. `cargo clippy --all-targets --all-features
  --locked -- -D warnings` is clean.
