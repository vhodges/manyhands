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

At the first Task 1 commit the runner's bound `run::PromotionShaped` pinned
the associated types to the promotion types. Fix round 1 below removes it.

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

### Fix round 1: one generic runner (C3, R-T1-2)

Ruling: the owner chose C3, one generic runner. `run::PromotionShaped` is
deleted. The runner is bounded on `L: Lifecycle` only. Its body no longer
names `PromotionRecord`, `PromotionStep` or `DocumentPromotionError`; only the
public `promote` and `promote_with_io` wrappers do.

- **`LifecycleRecord`** (implemented for `PromotionRecord`). Getters for
  `repository_id`, `operation_id`, `item_id`, `expectation`, `remote_name`,
  `context_sync_id` and `primary_sync_id`. Getters and setters for `step`,
  `base_oid`, `context_oid`, `integration_oid`, `integration_kind`,
  `remote_context_oid`, `publication`, `cleanup`, `cancel_requested` and
  `refusal`.
- **`ParentStep`**, the shared step view: the 13 steps the runner branches
  on, mapped one to one to `PromotionStep`. Closure's `closing` stays behind
  `pre_integration`.
- **`LifecycleError`** (implemented for `DocumentPromotionError`).
  - Constructors: `busy`, `operation_mismatch`, `recovery_required`,
    `interrupted`, `primary_advanced`, `context_synchronization`, and
    `From<RepositoryError>`.
  - Inspectors: `is_primary_advanced` and `is_consent_refusal`
    (`Blocked`/`ReconfirmationRequired`/`ExternalChange`).
- **`LifecycleRefusal`** (`external_change`, `reconfirmation_required`) and
  **`LifecyclePreview`** (`expectation`, `context_oid`, `tracking_context`,
  `integration`).
- **`Lifecycle` additions:**
  - observation: `preview_request`, `configuration`, `revalidate`, and
    `ready`, which turns a preflight into a ready preview or its `Blocked`
    refusal;
  - `refusal_error`, moved from `run.rs`;
  - integration: `integration_request`, `write_commit`, `resume_candidate`
    and `apply`;
  - outcome: `outcome`, `complete` and `index_pending`;
  - associated types `IntegrationRequest`, `Outcome` and `Output`.
- **`LifecycleRequest<E>`** is the runner's request.
  `promote_with_io` destructures `PromoteDocumentRequest` into it field for
  field.
- **R-T1-2.** `LeasedIntegration<'a, R>` gains `operation_id` and
  `record: Option<&R>`. The runner passes `Some(&r)` through
  `Lifecycle::prepare_integration`. Both are `None` only for the direct
  `integrate::prepare_integration` path used by unit tests.

**Fix round 1 tests and gate.**

- **Red.** `promotion_steps_map_to_the_shared_step_view` failed to compile
  before `ParentStep` and `LifecycleRecord` existed (16 errors).
- **Green.** `--lib promotion::lifecycle` passes 4 tests.
- **Policy.** Under the owner's 2026-10-17 test-policy change, the gate is
  the targets relevant to this change. The full suite runs once, after the
  last task.

| Args | Task 0 | Fix round 1 |
| --- | --- | --- |
| `--lib promotion` | 193 | 197 (193 + 4 lifecycle tests) |
| `--lib cleanup` | 42 | 42 |
| `--lib remote::reservation` | 45 | 45 |
| `--lib recovery` | 46 | 46 |
| `--test document_promotion` | 74 SSH | 74 SSH |

The sequential runner was already under way when the policy changed, so
these targets also ran:

| Args | Task 0 | Fix round 1 |
| --- | --- | --- |
| `--lib read::status` | 10 | 10 |
| `--test remote_reservation` | 13 | 13 |
| `--test read_status` | 23 | 23 |
| `--test remote_synchronization` | 63 SSH | 63 SSH |
| `--test remote_merge_recovery` | 25 SSH | 25 SSH |
| `--test remote_polling` | 84 SSH | 84 SSH |

`comment_publication` and `mutation_remote` were deferred to the final
verification. fmt is clean, and Clippy with `-D warnings` is clean.

## Task 2 — `ticket_closures` Record And Cross-Checks (2026-10-17)

### What landed

- **`closure/mod.rs`**: `ClosureBlocker` (the design's 21 variants, in
  design order) and `ClosureExpectation` (32 bytes, redacted `Debug`).
- **`closure/record.rs`**: the `ticket_closures` table, its
  `ticket_closures_unfinished` index and marker `cycle_10_ticket_closures`,
  migrated all or nothing inside the registry-schema transaction
  (`discovery.rs`) and audited at startup (`remote/state.rs`
  `audit_registry`). `ClosureStep` adds `closing`; `ClosureKind` is
  `record`/`existing`. `ClosureRecord` carries promotion's fields plus
  `closure_kind`, `closed_at` (RFC 3339, `Z`, whole seconds) and
  `closure_oid`; `merged_context()` is `closure_oid.or(context_oid)`. Record
  functions mirror promotion's, with their own SQL, plus `begin_closing` and
  `replace_unmoved_closure`. `CLOSURE_BLOCKERS` fixes the 21-bit refusal
  mask (`CHECK` 1..2097151).
- **R-T1-5.** `ClosureRecord` implements `LifecycleRecord`; `ParentStep` has
  no closure variant. `closing` reads as `ContextSynchronized`; writing that
  view back keeps `closing`; `save_record` accepts only `integrating` and
  `abandoned` from `closing` (promotion's pattern: the save refuses).
  `Lifecycle` itself is not implemented for closure (Task 6).
- **R-P2.** `closing → abandoned` is permitted at the record layer;
  `closing → superseded` is not, and a `closing` or `integrating` row is
  never superseded whatever its cleanup category (new ID is `Busy`).
- **Cross-checks naming both parent tables.**
  - `promotion/record.rs`: the begin-record collision query, `mint_attempt`
    and the audit's local-collision query name `ticket_closures`;
    `validate_remote_identity` checks promotion bindings and then closure
    bindings (`closure::record::validate_remote_identity`), so every
    reservation site in `reservation.rs` covers both tables unchanged.
  - `closure/record.rs`: the symmetric queries name `document_promotions`;
    its remote collision admits only `action='close' AND kind='ticket'`.
  - `recovery.rs`: both local-journal entry points call promotion's and
    closure's `validate_local_identity`.
  - `read/status.rs`: closure parents list as `action: close` with the
    stored step (including `closing`) as state and completed step; their
    `Close` reservation is hidden.
- **Lint.** The module-wide `too_many_arguments` allow is gone from
  `promotion/lifecycle.rs` (nothing there needed it). Item-level allows with
  a reason: `closure::record::{refuse_record, insert_record}` and
  `ChildExecution::new`.
- **Small promotion-side changes.** The category `parse` and
  `CleanupCategory::supersedable` are visible to the repository module;
  `ChildExecution` stores its parent's digest as bytes and gains `new` for
  the closure parent (its fence still refuses `TicketClosure` until Task 5).

### Decisions for later tasks

- `closure_kind` is `Option`: a refusal recorded at insertion (for example
  `Blocked([NotATicket])`) has no observed closure effect. Every other step
  requires it (schema and read validation). `begin_record` takes
  `ClosureKind`; `refuse_record` takes `Option<ClosureKind>`. Task 6 must
  supply them through the `Lifecycle` begin/refuse calls.
- `remote_context_oid` is not required at `context_synchronized` with a
  remote. Promotion records the fetched tracking context, which can be
  absent (Cycle 09 design: "possibly absent"), and the deletion-reservation
  rule depends on that absence. The invariant enforced is promotion's: only
  with a remote.
- `integration_kind` stays required from `integrating` on, as for promotion
  (the shared runner sets it with the base before that save); the design's
  table lists it from `integrated`.
- `complete_record` fails closed until Task 5 adds the kind-checked
  `RetirementEvidence::matches`; `finish_parent(TicketClosure, ..)` still
  refuses.
- `begin_closing` refuses a sub-second `closed_at` and stores another
  offset as the same instant in UTC.

### Tests

- **Red.** `devenv shell -- cargo test --locked --lib closure::record` with
  only the tests: 243 compile errors (record types and functions missing).
  `--test read_status status_lists_closure_parent_and_hides_its_reservation`
  failed: `no such table: ticket_closures`.
- **Green.** 15 tests in `closure/record_tests.rs`: the brief's 13, plus
  `closure_step_round_trips_through_parent_view` (R-T1-5) and
  `service_start_audits_closure_rows_and_schema`.
- **Existing tests.** Only named edit 3 (marker list in
  `tests/recovery_foundation_gate.rs`), the new table in the
  `tests/repository_enablement.rs` inventory, and the new
  `tests/read_status.rs` case.

| Args | Before | Task 2 | Wall time |
| --- | --- | --- | --- |
| `--lib closure::record` | — | 15 | 7 s |
| `--lib promotion` | 197 | 197 | 228 s |
| `--lib read::status` | 10 | 10 | 1 s |
| `--lib recovery` | 46 | 46 | 13 s |
| `--test read_status` | 23 | 24 (+1) | 4 s |
| `--test repository_enablement` | — | 78 | 22 s |
| `--test recovery_foundation_gate` | — | 51 | 14 s |
| `--test document_promotion` | 74 SSH | 74 SSH | 1012 s (concurrent with the other targets) |

`cargo fmt --check` is clean. `cargo clippy --all-targets --all-features
--locked -- -D warnings` is clean.

## Task 3 — Closure Observation, Blockers, Preview And Digest (2026-10-17)

### What landed

- **Public API** (`closure/mod.rs`, re-exported from `repository`):
  `PreviewTicketClosureRequest`, `TicketClosurePreflight`
  (`Ready` / `Blocked { blockers, identity_configuration }`),
  `TicketClosurePreview` (accessors only), `PlannedClosure`
  (`Record { closed_by }` / `Existing { closed_at, closed_by }`, `Debug`
  shows the variant only) and `RepositoryService::preview_ticket_closure`.
  The execution types (`CloseTicketRequest`, `TicketClosureError`,
  `TicketClosureResult`, `TicketClosureOutcome`, `ClosureRecording`) are left
  to Task 6, which first uses them.
- **`closure/observe.rs`**: `ClosureObservation` (no `Debug`),
  `PreparedClosureObservation`, `prepare_observation` (outside the lease),
  `revalidate_under_lease` (no discovery; guard, registration, pending work,
  Git and identity re-read), `closed_by_text`, `closed_ticket_bytes` with
  `ClosureContentError`, `CLOSURE_DIGEST_DOMAIN` and the digest.
- **`closed_ticket_bytes` (R-P1, C10, R-T0-2)** is complete here, with its
  round-trip and refusal tests. It compares after canonical normalization:
  every parsed field, the body and every unknown key must survive, except
  that `deps` and `parent` may take their canonical form. Preview runs it on
  the context ticket and on the pre-closure result ticket.
- **Shared promotion helpers** (`promotion/observe.rs`, no behavior
  change): `ObservationSeal` (the guard before/after discovery and the
  revalidation reopen), `open_observation`, `read_view_with` (one registry
  transaction plus caller reads), `target_cleanliness`/`Cleanliness`,
  `context_presence`, `primary_presence`, `remote_facts` (D2),
  `remote_context_history` (D5), `planned_result`, `ChangeEvidence` and
  `change_evidence`, and the `digest` encoders (D6 `local_oids`). Promotion's
  `observe_git`, `PreparedObservation` and `expectation` now call them with
  byte-identical results. `PromotionPathChange`'s fields are `pub(super)`;
  `bridges::abandon::authoring_item` is `pub(in super::super)` and reused
  for C12 (R-T0-4), not copied.
- **Task 2 leftover.** `steps_permit_only_designed_transitions` now asserts
  `ClosureStep::Closing.permits(Superseded, ..)` is false for both kinds,
  with and without a remote, and that `Closing → Abandoned` is permitted.

### Decisions and conservative choices

- **Guard scope.** `LifecycleKind::guarded_directory()` stays
  `.manyhands/tickets`. The design says the guard "hashes the ticket
  directory, not docs", parallel to the documents directory, and does not
  name `.manyhands/tickets/<ULID>`. The wider directory is a superset; the
  index and status walks already cover every canonical path under it.
- **Base closure** is read from Git at the base (the tracking primary with a
  remote, else the local primary), never from a worktree. A file naming
  either closure key counts as closed, so a malformed closure never admits
  a merge.
- **`AlreadyClosed`** replaces `ContextMissing` when the base is closed; it
  is also reported for "not contained, closed" without `IntegrationConflict`
  (no merge is ever attempted into a closed ticket).
- **D5** applies only to a context that is not contained in the base: a
  contained context runs no context child, so nothing could recreate the
  deleted remote branch (promotion's `AlreadyContained` exemption, extended
  to the contained-open row). Remote-branch cleanup is planned unless the
  context is contained and its tracking ref is absent.
- **`TicketInvalidAfterClosure`** is reported when the pre-closure result
  has a `.manyhands/worktrees/` entry, not exactly one conforming ticket at
  `.manyhands/tickets/<ULID>/ticket.md` as a `100644`/`100755` Blob, when
  `closed_ticket_bytes` refuses the context or result ticket (`Record`), or
  when the result does not carry exactly the planned `Existing` fields. The
  context ticket itself must be a canonical Blob, else
  `ContextNonconforming`.
- **C12.** A ticket's comment whose own local row is unfinished is reported
  as both `LocalOperationPending` and `CommentPublicationPending` (its row's
  target names the ticket). A binding whose publication row was never
  created is not unfinished (controller ruling). A remote row is unfinished
  by the status read's rule (`state.rs`): an active phase, an interrupted or
  failed synchronization, `index_pending` or `reconciliation_required`.
- **Identity.** A confirmation for another configuration is treated as no
  confirmation: the fresh expected configuration is returned with
  `IdentityRequired`. For `Existing`, an identity is selected when available
  and never blocks. Preview uses a fixed stand-in `closed_by` for the
  closed-ticket check while the identity is missing (any valid text gives
  the same verdict).
- **`needs_context_child`** is `remote && !contained`.

### Tests

- **Red.** The observation code was drafted before its tests, so the red
  run is a mutation run: with the C12 local check disabled, the relationship
  normalization exemption removed and `AlreadyClosed` no longer replacing
  `ContextMissing`, `--lib closure::observe::tests::` failed 6 of 52
  (`closed_ticket_bytes_normalizes_relationships_without_refusing`,
  `local_operation_pending`, `pending_local_operation_blocks`,
  `pending_comment_publication_blocks`,
  `every_blocker_is_reported_in_fixed_order`,
  `table_no_context_base_closed_is_already_closed`). The source was then
  restored byte for byte.
- **Green.** 52 tests in `closure/observe_tests.rs`: each blocker alone
  (plus `context_branch_missing`, `primary_tracking_missing`,
  `context_symlink_ticket_is_nonconforming`), all together in fixed order,
  `IdentityRequired` only alone and only for `Record`; the integration table
  (eight tests over the seven rows); `pending_local_operation_blocks` and
  `pending_comment_publication_blocks` (both rechecked through
  `revalidate_under_lease`); `context_with_closure_fields_previews_existing`;
  `preview_lists_whole_branch_changes`;
  `preview_and_execution_share_closed_ticket_check` (a CRLF hand-written
  ticket previews; a clean merge of two valid tickets that duplicates a key
  is refused by `closed_ticket_bytes` and is `TicketInvalidAfterClosure`);
  the digest (stable, redacted, each bound fact, mode-only changes, D6, time
  of day); `effective_identity_wins_over_confirmation`,
  `confirmation_must_match_configuration`; `preview_is_read_only`;
  `closed_ticket_bytes` round trip (LF and CRLF), relationship
  normalization and refusals.

| Args | Before | Task 3 | Wall time |
| --- | --- | --- | --- |
| `--lib closure::` | 15 | 67 (+52) | 28 s |
| `--lib closure` (name filter) | — | 71 | 33 s |
| `--lib promotion` | 197 | 197 | 247 s |
| `--test document_promotion` | 74 SSH | 74 SSH (unchanged) | 1009 s |

`cargo fmt --check` is clean. `cargo clippy --all-targets --all-features
--locked -- -D warnings` is clean (it builds every integration target, so
the source-included `production` wrappers compile).

### Task 3 fix round 1

The review changes are listed by item.

- **D5 contained exception.** Three tests cover it:
  `contained_closed_with_deleted_remote_is_cleanup_only`,
  `contained_closed_with_unknown_remote_history_is_cleanup_only` and
  `contained_open_with_deleted_remote_records_without_child`. Mutation red:
  without `!contained` all three fail; with the condition narrowed to
  `integration != AlreadyContained`, the contained-open test fails.
- **`preserved`.** It now compares `deps` as a set without repeats (null or
  empty means none) and `parent` with null treated as absent. A lost
  dependency, or a changed or dropped non-null parent, is refused.
- **Golden vector.** `closure_digest_golden_vector` pins
  `a585a50f6038f601a962c5ca2051657a80030a17a572fed3b415cbb35f03590c`. The
  test builds the wire bytes by hand from the design's encoding.
- **Identity selection.** It runs only for a `Record` effect.
  `ClosureObservation.identity` is `None` for `Existing`.
- **`already_closed`.** The test is now the C4 case: no context at all and
  the ticket closed on the primary.
- **Gate.** `--lib closure::` 73, `--lib closure` 77, `--lib promotion` 197.
  fmt and Clippy are clean.
