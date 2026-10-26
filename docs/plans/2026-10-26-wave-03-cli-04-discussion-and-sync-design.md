---
title: "Wave 03 C4 CLI Discussion, Synchronization And Conflict Recovery Design"
date: 2026-10-26
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5X4YWQMNJJT3VXP0BY9JGG9"
---

# Wave 03 C4 CLI Discussion, Synchronization And Conflict Recovery Design

## Execution Status

**Draft, 2026-10-26.** Written for product-owner review. Nothing is built.
Implementation waits for the gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` to
merge (Cycle decision 2).

## Intent And Approval Boundary

This design turns the [Cycle](../Cycles/wave-03-cli-04-discussion-and-sync.md)
into bindings, verbs, inputs, outcome tables and tests. The Cycle governs
this design, and this design governs the
[implementation plan](2026-10-26-wave-03-cli-04-discussion-and-sync-implementation.md).

Approval of this design covers:

- the library part of Cycle decisions 1, 3, 4, 7, 8 and 9;
- the per-verb grammar and input fields;
- the five required-outcome tables;
- rulings R-1 to R-30;
- named edits E-1 to E-11;
- the document amendments listed below.

It does not authorize implementation, pushing beyond the ticket branch,
merging, closing or native CI.

C4 changes the library, as an approved exception to track rule 5 (decision
1). The change is limited to:

- three new mutation commands: `CommentAdd`, `ConflictResolve` and
  `ItemRepublish`;
- one new read, the conflict inspection;
- the merging settings of the two synchronization bindings;
- resume's routing;
- the new data and preview DTOs and their schemas;
- the crate-private parameters these need.

No result code or recovery-action kind is added (decision 8).

## Baseline Facts

Read at branch head `219ec57` (main `0464c42` merged), and on the gate branch
at `0b75184`. Facts marked **(gate)** may change when the gate merges. Task 0
re-checks every fact before any code, and a changed fact that alters a task
stops there for a controller ruling.

Facts 1–53 were read from the source for this draft. Facts marked
**(confirm)** rest on a reading that Task 0 must prove, and name the check.

**The mutation boundary.**

1. `Command` (`src/repository/mutation/command.rs:37-68`) has `ItemSync`
   ("item sync"), `RepoSync` ("repo sync"), `OperationResume` ("operation
   resume") and `OperationAbandon` ("operation abandon"). It has no comment
   or conflict command. `Mutation` is at `src/repository/mutation/mod.rs:398`
   **(gate)**.
2. The inputs:
   - `ItemSyncInput { root, kind: AuthoringKind, id: ItemId }`
     (`bind/sync.rs:61`);
   - `RepoSyncInput { root }` (`:69`);
   - `OperationResumeInput { root, operation_id }` (`bind/resume.rs:63`);
   - `OperationAbandonInput { root, operation_id }` (`bind/abandon.rs:33`).

   `ItemSyncInput` and `RepoSyncInput` do not derive `Clone`.
3. `needs_confirmation` (`command.rs:436`) is a `const fn` of the command
   alone. It is true for `OperationAbandon`, and false for `ItemSync`,
   `RepoSync` and `OperationResume`. `stops_at_safe_points` (`:268`) is true
   only for `ItemSync` and `RepoSync`.
4. The synchronization bindings pass
   `SETTINGS = SynchronizationSettings { refuse_divergence: true }`
   (`bind/sync.rs:101-103`) at `:643` and `:679`. The module doc (`:5-8`)
   says "Neither merges".
5. The bindings' request (`bind/sync.rs:583-594`) sets `approval: None` and
   `confirmed_identity: None`. `restart` is false in `run` and true in
   `rerun`, `resumed` and the acknowledgement call.
6. `synchronization_error_code` (`outcome.rs:197-231`):
   - `ConflictPending`, `ExternalResolutionRequired`, `Interrupted`,
     `HistoryUnknown` and `RecoveryRequired` become `recovery_required`;
   - `RemoteContextDeleted` becomes `remote_branch_deleted`;
   - `MergeRequired` becomes `merge_required`;
   - `IdentityRequired` becomes `identity_required`.

   The comment at `:215-217` rests this on "The bindings refuse divergence".
7. `recovery` (`outcome.rs:717`):
   - `merge_required` and `remote_branch_deleted` are
     `never_finished_by_the_same_request` (`:1002`), offered
     `[request.cancel]` only;
   - `identity_required` is offered `[repo.identity_set]`;
   - `recovery_required` is offered `[repo.inspect]`;
   - `discovery_pending` is offered `[operation.resume]`;
   - a synchronization left accepted has `request.retry` put first
     (`:993-996`).
8. `SynchronizationMutationDto` (`dto.rs:657`) holds only host data: "A
   synchronization reports no other data". `MutationDataDto` is at
   `dto.rs:540` **(gate)**.
9. `operation resume` (`bind/resume.rs:1-20`) takes up two kinds of work: a
   local operation's owed index hand-off, and a synchronization, "and nothing
   is merged". A comment's synchronization is left to inspect, so resume
   answers it `recovery_required` (`:217-232`) **(gate: the gate adds
   local-sync routing here)**.
10. `operation abandon` (`bind/abandon.rs`):
    - it is confirmed, with `OperationAbandonPreviewDto`;
    - an ID the remote or key journal holds is `invalid_input`;
    - it calls `abandon_operation` (`bridges/abandon.rs:428`);
    - a `SubmitComment` row counts as `Writes::Item` (`:208`);
    - **(gate)** the gate refuses generic abandonment of a local-sync
      operation.
11. `cancel_request` (`progress.rs:366`) asks only a command whose
    `stops_at_safe_points` is true (`:383`). `request cancel` therefore does
    not reach a resume.
12. `prepare` (`confirm.rs:286`) answers `not_confirmable` when
    `mutation.confirmed()` is `None`. `CONFIRMATION_LIFETIME_SECONDS = 600`
    (`:47`). A confirmable command provides five things:
    - `needs_confirmation`;
    - a `Mutation::confirmed()` arm, which clones its input;
    - a binding implementing `Confirmed::{preview, row, observe_again}`;
    - a `PreviewDetailsDto` variant with its preview effects;
    - a preview schema.
13. `PreviewDetailsDto` (`dto.rs:210-219`) is untagged with eight variants.
    It is matched exhaustively in `src/desktop/view/confirm.rs:492` and
    `src/bin/manyhands-cli/human_admin.rs:55`. `PreviewEffect`
    (`dto.rs:133`) is matched exhaustively in `human_admin.rs:77`.
14. `RecoveryActionKind` (`src/results.rs:401`) is matched exhaustively by
    the desktop (`src/desktop/model/operations.rs:529-564`).
15. `CancellationToken(Arc<AtomicBool>)` (`progress.rs:49`): `cancel`
    stores `true` with `SeqCst`.
16. `execute(&self, call: MutationCall, credentials, progress, cancel) ->
    Envelope<MutationDataDto>` is at `execute.rs:45`. `show_request` is at
    `mod.rs:367`, and `cancel_request` at `progress.rs:366`.

**Comments.**

17. `submit_comment<P>(&self, PublishCommentRequest, &mut
    SessionCredentials<P>) -> Result<CommentSubmissionOutcome,
    CommentSubmissionError>` (`src/repository/comment_publication.rs:178`).
    - `PublishCommentRequest { comment, approval, confirmed_identity }`.
    - `SubmitCommentRequest { target: AuthoringTarget, comment_id,
      parent_id, body, expected_destination }` (`src/repository.rs:653`).
    - It takes no observer and no cancellation token.
18. Publication calls `synchronize_remote` on the context target, under the
    binding's child `synchronization_id` (`comment_publication.rs:499`). That
    function uses `SynchronizationSettings::default()`, which merges, and no
    observer (`remote/sync.rs:6219`).
19. Before writing, submission refuses:
    - a context with a pending synchronization conflict
      (`src/repository.rs:3619`);
    - a lifecycle-closed ticket (`refuse_closed_ticket`, `:249`);
    - an intent other than `Edit` (`:2795`);
    - a parent that is not a conforming comment of the same item
      (`:2849-2873`);
    - a new row whose `expected_destination` is not `Missing` (`:2892`).
20. `created_by` is set from the Git identity at the checkpoint:
    `comment.created_by = Some(format!("{} <{}>", identity.name,
    identity.email))` (`src/repository.rs:3042`).
    - With no identity, the answer is `IdentityRequired { context }`. That
      happens after the binding row is inserted and before the file is
      written.
21. The binding table `comment_publication_bindings` (`recovery.rs:47-62`)
    is keyed by `operation_ulid`. It holds a unique child
    `synchronization_ulid`, and no body. `matches_request` (`:27-44`)
    compares kind, item, comment, parent and intent.
22. Retry and cancel are body-free:
    - `retry_comment_publication(RetryCommentPublicationRequest { root,
      operation_id, approval, confirmed_identity, restart })`
      (`comment_publication.rs:28`, `:330`) never writes a comment;
    - `cancel_comment_publication(root, operation_id) -> Result<bool, _>`
      (`:365`) returns `false` while a conflict is pending.
23. `CommentSubmissionOutcome` (`:102`) is one of:
    - `IdentityRequired`;
    - `Saved { receipt, publication, indexing, context, checkpoint }`, with
      `publication` one of `Published`, `AlreadyCurrent` or
      `Pending { reason }`.

    The pending reasons are `NoPublicationRemote`, `LocalRecoveryRequired`
    and `Synchronization(Box<SynchronizationError>)`.
    `CommentSubmissionError { kind, operation_id, .. }` is at `:117`.

**Synchronization and conflicts.**

24. `synchronize_remote_observed(request, session, settings, observer)` is at
    `remote/sync.rs:6236`. `SynchronizeRemoteRequest { root, operation_id,
    target, approval, confirmed_identity, restart }` (`remote/state.rs:1730`)
    has no republish field.
25. Merging is `integrate_divergence` (`sync.rs:3741`):
    - with `refuse_divergence: false`, a context target merges the remote
      context first (ordinal 0) and then primary (ordinal 1). A primary
      target merges primary only;
    - the subjects are "Merge remote context {id}", "Merge primary into
      {kind} {id}" and "Merge remote primary";
    - a conflict leaves index stages and `MERGE_HEAD` in the target
      worktree, releases the reservation (phase `interrupted`,
      `reservation.rs:1876-1879`) and returns `ConflictPending`
      (`sync.rs:3990-4033`).
26. Restarting an operation:
    - the same operation without `restart` is `RecoveryRequired`;
    - with `restart` before a resolution, it is `ConflictPending`, offline;
    - after a resolution, it publishes the resolved commit
      (`tests/remote_merge_recovery.rs:1616-1649`).
27. While a conflict is pending:
    - every new synchronization reservation is `Busy`
      (`reservation.rs:1152-1161`);
    - authoring in that context is `RecoveryRequired`
      (`src/repository.rs:3642-3655`).
28. Merge identity:
    - `select_committing_identity` (`sync.rs:3255-3280`) returns
      `IdentityRequired { target, expected_configuration }`;
    - the integration step's intent is persisted before that check
      (`:3835`);
    - `set_local_identity` (`bridges/identity.rs:703`) refuses while a
      remote reservation is held **(confirm: Task 0 reads it and records
      whether `repo identity-set` succeeds after a merge's
      `IdentityRequired`)**.
29. `inspect_synchronization_recovery(root, operation_id) ->
    Result<SynchronizationConflictInspection, SynchronizationError>`
    (`sync.rs:4935`):
    - it runs SELECTs in IMMEDIATE transactions, and takes the repository
      lease;
    - it needs one step in `ConflictPending`, `ResolutionPrepared` or
      `CommitPrepared`;
    - a HEAD other than the step's local parent is `ExternalChange`.
30. `SynchronizationConflictInspection` (`sync.rs:261`) holds `operation_id`,
    `target`, `stage`, `local_parent`, `incoming_parent`, `observation` and
    `paths`. Each path is `SynchronizationConflictPath { token, eligibility }`
    (`:254`). Eligibility is one value for the whole set (`merge.rs:171`).
31. `ConflictObservation` and `ConflictPathToken` (`merge.rs:75`, `:117`)
    have only `pub(super)` fields and no `Serialize`. Two inspections of
    unchanged state compare equal.
32. `read_synchronization_conflict(&ConflictPathToken) ->
    Result<EphemeralSynchronizationConflictSides, _>` (`sync.rs:5051`):
    - it returns base, local and incoming from index stages 1, 2 and 3;
    - `current` is always `None` (`:5111`);
    - non-UTF-8 content is `ExternalResolutionRequired`.
33. `resolve_synchronization(ResolveSynchronizationRequest) ->
    Result<ResolveSynchronizationOutcome, SynchronizationError>`
    (`sync.rs:5593`) resolves "without transport or publication".
    - `ResolveSynchronizationRequest::new(root, synchronization_id,
      attempt_id, observation, resolutions, identity)` (`merge.rs:280`).
    - Its outcomes are `LocalCheckpointComplete { commit_oid }`,
      `IdentityRequired`, `StaleObservation`, `ValidationFailed` and
      `RecoveryRequired` (`merge.rs:314-321`).
34. Resuming a resolution:
    - at `CommitPrepared` or `Applied`, `reconcile_resolution_candidate`
      (`sync.rs:5133`) accepts only the identical attempt and input digest;
    - at `ResolutionPrepared`, the attempt is reused when the inputs match
      (`state.rs:4542-4577`);
    - a new attempt after success is `RecoveryRequired`.

    Every test of resumption reuses the in-memory request
    (`comment_publication.rs:2913`; `sync_tests.rs:6651`).
35. An outside repair is accepted when:
    - HEAD has exactly the parents `[local, incoming]`;
    - the target is clean;
    - `validates_external_integration` (`sync.rs:4742`) passes.

    A restart then continues it (`reconcile_pending_candidate`, `:3348`).
    Removing markers alone, staging without a commit, or wrong parents are
    refused (`tests/remote_merge_recovery.rs`, `refused_false_repairs`).
36. Remote deletion:
    - an absent, previously published context is `RemoteContextDeleted`
      (`refs.rs:262-268`), which becomes `remote_branch_deleted`;
    - the same state is also refused at `sync.rs:6767-6786` and
      `:7161-7165`, and at `reservation.rs:2689-2696` and `:2751-2758`.
      Those four surface as `RecoveryRequired`;
    - `remote_context_states` (`state.rs:89-99`) keeps
      `last_advertised_oid`.
37. `show_item(&self, repo, id) -> Result<ItemDto, ReadError>`
    (`src/repository/read/items.rs:1784`) needs no kind.

**The CLI.**

38. The catalog lists seven verbs as `planned(…, "C4")`
    (`src/bin/manyhands-cli/catalog.rs:876-882`): `comment add`,
    `item sync`, `repo sync`, `conflict show`, `conflict resolve`,
    `operation resume` and `operation abandon`. `item republish` is not
    there.
39. `--operation-id` is declared inline in `operation show`
    (`catalog.rs:391-405`). `REQUEST_ID` is a shared const (`:92-97`).
40. The mutation driver:
    - `mutate::run` is at `mutate.rs:120`;
    - it passes `&CancellationToken::new()` to `execute` (`:60`);
    - `should_retry` (`:73-84`) retries `busy`, or a partial result whose
      first recovery action is `request.retry` for the same request;
    - a cancelled result is never retried.
41. The synchronization observer `Watch` (`bind/sync.rs:362-428`) cancels at
    a safe point when the session's token is set or the request's flag is.
    Resume calls `sync::resumed` with that observer **(confirm: Task 0 proves
    that a token cancelled during `operation resume` stops it at a safe
    point)**.
42. Nothing in `src/` installs a signal or console-control handler. Prompts
    use raw mode, which clears `ISIG` (`terminal/unix.rs:62`) and
    `ENABLE_PROCESSED_INPUT` (`terminal/windows.rs:98`). `libc = "0.2"` and
    `windows-sys` with `Win32_System_Console` are dependencies
    (`Cargo.toml:23`, `:38`).
43. `PtyCli` (`tests/support/pty.rs:79`) gives the child a controlling
    terminal. `finish` expects an exit code and panics on death by signal
    (`:352`).
44. The `cli_remote` harness:
    - `Case` is at `tests/support/ssh_harness.rs:48`;
    - each case has a 100-second watchdog (`:237`);
    - `World::new` (`tests/support/remote_world.rs:139`) builds one
      Manyhands clone and one raw peer;
    - `scan_output` (`tests/cli_remote.rs:62-81`) checks every output.
45. The help goldens list the seven planned C4 verbs:
    `tests/fixtures/cli_v1/help_root.json:1661-1716` and
    `help_root_text.txt:87-93`.
46. Two catalog tests:
    - `catalog_matches_every_command_and_delivering_cycle_without_duplicates`
      (`catalog_tests.rs:107`) compares the catalog with `DESIGN`'s C4 row
      (`:81-92`);
    - `catalog_repository_rules_and_planned_metadata_match_the_design`
      (`:131`) requires `--repo` to be `Required` on every available verb
      outside a fixed list.
47. `every_input_taking_command_has_exactly_one_schema_and_every_schema_is_reached`
    is at `tests/cli/input_schemas.rs:171`.
48. Comments are written as canonical files by instruction in:
    - `AGENTS.md:73-75`;
    - `.agents/skills/file-ticket/SKILL.md:138-153`;
    - `.agents/skills/planning-a-cycle/references/approval-and-handoff.md:49-51`;
    - `.agents/skills/working-on-a-ticket/references/formats.md:3-5` and
      `:42-69`;
    - `.agents/skills/implementing-a-cycle/SKILL.md:51-52`.

    `scripts/ticket-loop`'s `newest_comment` (`:118-130`) tells agents from
    people by a `created_by` containing `<noreply@`.

**Inventories and tests.**

49. A new `Command` joins these closed inventories **(gate)**:
    - `command.rs`'s predicates (`:85`, `:151`, `:188`, `:226`, `:268`,
      `:316`, `:351`, `:394`, `:436`) and their tests (`:491-651`);
    - `Mutation`, `kind()`, `confirmed()` and `into_binding()` in `mod.rs`;
    - `recorded_effects` (`bind/mod.rs:899`) and `from_stored`
      (`dto.rs:994`);
    - `CASES` in `tests/support/mutation_golden.rs:21`;
    - in `tests/mutation_contract.rs`: `UNBOUND_COMMANDS` (`:1455`),
      `DATA_ROOTS` (`:1510`) and `CODES_NO_MUTATION_GOLDEN_HAS` (`:1713`);
    - in `tests/read_contract.rs`: `contract_enumerations` (`:703`) and
      `published_enumerations_list_exactly_the_registered_values` (`:1018`);
    - `HOMES` (`tests/support/conformance.rs:59`) and each home's table;
    - `preview_schema` (`tests/support/confirmation.rs:49-62`);
    - the data accessors (`tests/support/mutation.rs:596-775`) and
      `tests/support/repair.rs:333`.
50. The schema checker allows only `type`, `properties`, `required`,
    `additionalProperties`, `items`, `minimum`, `enum` and `$ref`
    (`tests/support/schema.rs:98-160`). It has no `oneOf` or `const`.
51. Tests that pin the bindings' refusal to merge are listed under named
    edit E-3.
52. The test targets C4 extends are already in the native list
    (`.github/workflows/build.yml:115`): `comment_publication`,
    `remote_merge_recovery`, `mutation_remote`, `mutation_contract`,
    `mutation_replay`, `mutation_confirmation`, `read_contract`, `cli` and
    `cli_remote`.

**The gate.**

53. At `0b75184` the gate adds:
    - `observe_local_context_sync`;
    - a request-control table (`mutation/local_control.rs`), which
      `insert_request` and `enter_request` call for every request;
    - `OperationAction::ContextSyncLocal`.

    It plans `LocalContextSync` and `LocalContextResolve` mutations, with
    their own inventory edits. Task 0 re-reads them after it merges.

## Module Layout

**Library:**

- `src/repository/mutation/bind/comment.rs` (new): the `comment add`
  binding.
- `src/repository/mutation/bind/conflict.rs` (new): the `conflict resolve`
  binding.
- `src/repository/mutation/bind/republish.rs` (new): the `item republish`
  binding.
- `src/repository/remote/conflict_read.rs` (new): the conflict inspection
  read and string tokens. It sits inside `remote` because the tokens'
  fields are `pub(super)` (fact 31).
- Edited:
  - `bind/sync.rs` (merging settings and conflict data);
  - `bind/resume.rs` (routing);
  - `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs` and `bind/mod.rs`
    (inventories);
  - `comment_publication.rs` (the observed, authored submission);
  - `remote/sync.rs`, `remote/refs.rs`, `remote/reservation.rs` and
    `remote/state.rs` (republish consent and resolution resumption).
- New schemas under `schemas/v1/`:
  - `comment_mutation`;
  - `conflict` and `conflict_path`;
  - `conflict_next`;
  - `synchronization_conflict_mutation`;
  - `conflict_resolution_mutation`;
  - `republish_preview`.

**Desktop:** one arm in `src/desktop/view/confirm.rs` (decision 10).

**CLI (bin-private):**

- `collaboration.rs`: glue for the sync, republish, resume and abandon
  verbs.
- `discussion.rs`: `comment add`.
- `conflict.rs`: `conflict show` and `conflict resolve`.
- `interrupt.rs`, with `interrupt/unix.rs` and `interrupt/windows.rs`: the
  Ctrl-C window.
- `human_collaboration.rs`: human output.

Each new module has its `_tests.rs`, following C1's pattern.

**Tests:**

- library tests join `tests/mutation_remote.rs`,
  `tests/comment_publication.rs`, `tests/remote_merge_recovery.rs`,
  `tests/mutation_contract.rs` and `tests/read_contract.rs` as modules;
- CLI tests join `tests/cli/` and `tests/cli_remote/`;
- new support: `tests/support/cli_pair.rs` (R-24).

No new `[[test]]` target (R-29).

## Comment Submission (`comment add`)

`Mutation::CommentAdd(CommentAddInput)`, command `"comment add"`:

```rust
pub struct CommentAddInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub comment_id: ItemId,
    pub parent_id: Option<ItemId>,
    pub body: String,
    pub created_by: Option<String>,
}
```

It derives no `Debug`, `Display` or `Serialize`. It is not confirmed, and it
stops at safe points. The request digest covers every field, the body by
digest.

- **Target.** An `AuthoringTarget` with `intent: Edit` and the request's
  operation ID. A fresh request expects the comment path to be `Missing`
  (fact 19). A re-entered request uses the library's replay of its own row.
- **Author (decision 9, R-7).** A given `created_by` replaces the Git
  identity in the comment's front matter. It is written once. Without it,
  fact 20's identity applies, and a missing identity is
  `identity_required`. A given `created_by` does not remove the need for a
  commit identity for the checkpoint.
- **Publication (R-3).** The binding calls a crate-private observed
  submission that takes the binding's `Watch` observer and the merging
  settings. Publication therefore reports progress and stops at safe points.
  `submit_comment` and its public request types are unchanged (R-4).
- **Retry.** A re-entered request calls the observed retry with
  `restart: true`. It proves the receipt from Git, writes no comment and
  publishes (fact 22).
- **Data.** A `CommentMutationDto`, tagged `type: "comment"`, holding:
  - `item_id`, `comment_id`, `parent_id` and `created_by`;
  - `synchronization_id`;
  - `publication` (`published`, `current` or `pending`);
  - `pending_reason` (`no_publication_remote`, `recovery_required`,
    `synchronization` or null);
  - `conflict` (a `ConflictNextDto` or null; see The Next Step).

  It holds no body.

### Required outcomes: comment submission

Written as a test first (Task 2). A row that answers otherwise stops for a
ruling.

| Case | Code | Outcome, exit | Effects | Data, recovery |
| --- | --- | --- | --- | --- |
| Remote configured, publishes | `ok` | success, 0 | write written, checkpoint committed, publication published | publication `published` |
| No publication remote | `ok` | success, 0 | checkpoint committed, publication pending | `pending_reason: no_publication_remote` |
| Push refused by the server | the synchronization's code (`push_rejected`) | partial, 4 | checkpoint committed, publication pending | `pending_reason: synchronization`; `request.retry` first |
| Same request retried after the refusal, server now accepts | `ok` | success, 0 | publication published | One comment file, one comment checkpoint, `created_at` unchanged |
| Publication meets a canonical conflict | `recovery_required` | partial, 4 | checkpoint committed, integration pending, publication pending | `conflict: {operation_id: <synchronization_id>, next_action: resolve}` |
| Another item's conflict is pending | `busy` | partial, 4 after C2's bounded retry | checkpoint committed, publication pending | `pending_reason: synchronization` |
| This item's context has a pending conflict | `recovery_required` | blocked, 3 | nothing | none |
| Lifecycle-closed ticket | `ticket_closed` | blocked, 3 | nothing; no context or binding left | none |
| No identity and no `created_by` | `identity_required` | blocked, 3 | nothing written | `[repo.identity_set]`; after `repo identity-set`, the same request succeeds |
| `created_by` given, no Git identity | `identity_required` | blocked, 3 | nothing written | `[repo.identity_set]` |
| `created_by` malformed | `invalid_input` | input, 2 | nothing | none |
| `parent_id` names no comment of this item | the library's refusal, recorded by Task 0 | blocked or input | nothing written | none |
| `comment_id` already used by another file | `occupied_path` | input, 2 | nothing | none |
| Ctrl-C during publication | `cancelled` | cancelled, 130 | checkpoint committed, publication pending | `operation resume --operation-id <comment operation>` then publishes |
| Lost stdout after success | — | exit 1 | as success | `request show` returns `ok`; a replay writes and pushes nothing |

## Merging Synchronization (decision 3)

The bindings pass `refuse_divergence: false` (R-16). `item sync` and
`repo sync` then integrate divergent history as fact 25 describes.

- **Next step.** A conflict-pending result carries
  `SynchronizationConflictMutationDto`, tagged
  `type: "synchronization_conflict"`. It holds `conflict`, a
  `ConflictNextDto`. The binding reads the eligibility from the inspection
  after the conflict (R-8).
- **Effects.** `integration: pending` marks a merge begun and not finished.
  The checkpoint is not committed.
- **`merge_required`.** The bindings no longer return it. It stays a result
  code, listed with its reason in `CODES_NO_MUTATION_GOLDEN_HAS` (E-7).
- **Identity (R-17).** As in the table below.

### Required outcomes: divergent synchronization

For both `item sync` and `repo sync`, written as a test first (Task 1).

| Case | Code | Outcome, exit | Effects and data |
| --- | --- | --- | --- |
| Already current | `already_applied` | success, 0 | publication current |
| Remote behind, local ahead | `ok` | success, 0 | publication published |
| Local behind | `ok` | success, 0 | the branch fast-forwards; publication current or published |
| Clean divergence | `ok` | success, 0 | One merge commit with parents `[local, incoming]` and the fact 25 subject, published. No rebase; the remote keeps its earlier tip as an ancestor |
| Canonical conflict | `recovery_required` | blocked, 3 | integration pending; `conflict: {operation_id, next_action: resolve}`; the reservation is released |
| Conflict in a noncanonical path | `recovery_required` | blocked, 3 | integration pending; `next_action: external_repair` |
| Same request again, conflict still pending | `recovery_required` | blocked, 3 | the same conflict data; no transport |
| A new sync while another operation's conflict is pending | `busy` | transient, 5 after C2's bounded retry | nothing |
| Divergence with no usable identity | `identity_required` | blocked, 3 | `[repo.identity_set]` (with `request.retry` first if the record stays accepted) |
| Then `repo identity-set` | `ok` | success, 0 | the identity is set; it is not refused for a held reservation |
| Then the same sync request | `ok` | success, 0 | the merge is committed and published once |
| Remote context deleted | `remote_branch_deleted` | blocked, 3 | nothing recreated; human output names `item republish` |
| No publication remote | `publication_remote_required` | blocked, 3 | nothing |
| Ctrl-C at a safe point | `cancelled` | cancelled, 130 | effects name what completed |

## Republish (`item republish`, decisions 4 and 7)

`Mutation::ItemRepublish(ItemRepublishInput { root, kind, id })`, command
`"item republish"`. It is confirmed and stops at safe points. The input
derives `Clone`.

- **Preview.** `RepublishPreviewDto` holds:
  - `root`, `kind`, `item_id` and `branch`;
  - `last_published_oid`, from `remote_context_states.last_advertised_oid`;
  - `local_oid`.

  Its effect is the new `PreviewEffect::BranchRepublished`
  (`branch_republished`). The observation digest binds all of those, and the
  remote's advertisement of the branch's absence at preview time.
- **Execution.** The binding runs item sync's request with a crate-private
  consent value `{ branch, last_published, local }` (R-18).
  - At each of fact 36's refusal points, a matching consent lets the
    operation through, and the push recreates the branch.
  - An unmatched consent keeps the refusal.
  - Ordinary `item sync` passes no consent and still refuses.
- **Desktop.** One `PreviewDetailsDto::Republish` arm in
  `src/desktop/view/confirm.rs` shows "Branch", "Last published" and "Local
  commit" (decision 10, R-19).

### Required outcomes: republish

Written as a test first (Task 5).

| Case | Answer |
| --- | --- |
| `--prepare` on a deleted, previously published branch | Preview with `branch_republished`, the branch, `last_published_oid` and `local_oid`, exit 0 |
| `--confirm`, nothing changed | `ok`, exit 0: primary integrated as `item sync` does, the branch created once at the local tip, publication published |
| `--confirm` after the local branch moved | `external_change`, exit 3; nothing pushed |
| `--confirm` after the branch reappeared on the remote | `external_change`, exit 3; nothing pushed |
| `--prepare` on a branch that exists remotely, or was never published | A no-op preview, exit 0; confirm answers `already_applied`, exit 0 |
| Replay of a confirmed request | Its stored result; no second push |
| `item sync` after a successful republish | Ordinary synchronization; `already_applied` when current |
| Ctrl-C during the republish | `cancelled`, exit 130; a new prepare is needed |

## Conflict Inspection (`conflict show`)

`RepositoryService::show_conflict(&self, repo: &ResolvedRepository,
operation_id: OperationId) -> Result<ConflictDto, ReadError>`. It is a read
(R-12).

`ConflictDto`:

| Field | Type |
| --- | --- |
| `operation_id` | string: the synchronization's ID (R-5) |
| `comment_operation_id` | string or null: the comment's operation, when the synchronization is a comment's |
| `target` | `primary`, `document` or `ticket` |
| `item_id` | string or null |
| `stage` | `context` or `primary` |
| `state` | `conflict_pending`, `resolution_prepared`, `resolved`, `repaired`, `repair_refused` or `complete` |
| `next_action` | `resolve`, `retry_resolution`, `external_repair`, `resume` or `none` |
| `eligibility` | `eligible_canonical`, `external_resolution_required` or null |
| `local_parent`, `incoming_parent` | strings |
| `observation` | string or null (R-9) |
| `paths` | array of `ConflictPathDto` |

`ConflictPathDto` holds:

- `display_path`, escaped as the gate escapes it (R-11);
- `token`, a string (R-9);
- `content_available`, a boolean;
- `base`, `local` and `incoming`, each a string or null.

Sides are present only when the library returns them (fact 32, R-10).
Paths are ordered by raw path bytes.

### Required outcomes: conflict states

Written as a test first (Task 3).

| State of the operation | `state` | `next_action` | Paths and observation |
| --- | --- | --- | --- |
| Conflict pending, every path canonical | `conflict_pending` | `resolve` | every path with sides; observation present |
| Conflict pending, any path noncanonical | `conflict_pending` | `external_repair` | paths listed; sides only where readable; observation present |
| A resolution stopped after writing paths | `resolution_prepared` | `retry_resolution` | as the conflict; observation present |
| Resolved and checkpointed, not yet published | `resolved` | `resume` | empty; observation null |
| Repaired outside with an accepted two-parent commit | `repaired` | `resume` | empty; observation null |
| An outside attempt not accepted (markers removed, staged only, wrong parents) | `repair_refused` | `external_repair` | empty; observation null |
| Published | `complete` | `none` | empty; observation null |
| An unknown operation ID | `operation_not_found`, exit 2 | — | — |
| An operation that is no synchronization | `invalid_input`, exit 2 | — | — |

## Conflict Resolution (`conflict resolve`)

`Mutation::ConflictResolve(ConflictResolveInput)`, command
`"conflict resolve"`:

```rust
pub struct ConflictResolveInput {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub observation: String,
    pub resolutions: Vec<ConflictResolutionInput>,
}
pub struct ConflictResolutionInput {
    pub token: String,
    pub source: String,
}
```

These follow the gate's local-resolution shape. They derive no `Debug`,
`Display` or `Serialize`. The command is not confirmed and stops at safe
points. The digest covers the operation, the observation and every
token/source pair in token order. A duplicate token is `invalid_input`.

The binding works in five steps.

1. It re-derives the live conflict's observation and tokens (R-9), and
   compares them with the input.
   - A different observation is `external_change`.
   - An unknown token, a missing path or a duplicate is `invalid_input`.

   Nothing is written in either case.
2. It derives the resolution attempt from the request (R-14), and calls
   `resolve_synchronization`.
3. On `LocalCheckpointComplete`, it continues the operation's publication
   under the same request (R-15):
   - a comment's synchronization through the observed retry with
     `restart: true`;
   - any other synchronization through a restart with the merging settings.
4. It maps the remaining outcomes:
   - `StaleObservation` is `external_change`;
   - `ValidationFailed` is `invalid_input`, and nothing is written;
   - `IdentityRequired` is `identity_required` with `repo.identity_set`;
   - `ExternalResolutionRequired` is `recovery_required` with
     `next_action: external_repair`.
5. Its data is `ConflictResolutionMutationDto`, tagged
   `type: "conflict_resolution"`. It holds `operation_id`, `commit_oid`,
   `publication` and `conflict`. `conflict` is set when publication met a
   new conflict.

### Required outcomes: interrupted resolution

Written as a test first (Task 4), with library failure points as the
interrupting actor (C3's R-11 pattern). A row that answers otherwise stops at
a design checkpoint. The mechanism that lets a separate process finish a
resolution whose index or ref has moved is the implementer's (R-14). The
rows are the contract.

| Interrupted at | Retry of the same request, same input | Effects after the retry |
| --- | --- | --- |
| Before any path is written | `ok`, exit 0 | one resolution commit, published |
| After the paths are written (`ResolutionPrepared`) | `ok`, exit 0 | the same candidate commit; one commit; published |
| After the commit is prepared, before the ref moves | `ok`, exit 0 | the same commit OID; no second commit; published |
| After the ref moves, before publication | `ok`, exit 0 | the same commit OID; published |
| During publication | `ok`, exit 0 | published once |
| Any of the above, then a different `source` under the same request ID | `request_mismatch`; exit 4 when the stopped request recorded a durable effect, else 2 (`replay.rs:140-150`, `results.rs:282`) | nothing changes |
| After success, a new request with a different resolution | `recovery_required`, exit 3, `next_action: resume` or `none` | nothing written |
| After success, `operation resume` | `already_applied` or `ok` | nothing written twice |
| Ctrl-C during publication | `cancelled`, exit 130 | checkpoint committed; `operation resume` publishes |

## Resume (R-20)

`operation resume` adds three routes to fact 9's two. The gate's local-sync
routing stays as the gate delivers it.

| Operation | Resume does |
| --- | --- |
| A comment's synchronization, or the comment's own operation | The observed retry with `restart: true`: publishes the comment, never writes it |
| A synchronization with a resolution checkpointed or an accepted outside repair | A restart with the merging settings: publishes it |
| A synchronization whose conflict is still pending | `recovery_required`, exit 3, with the conflict data; no transport |

Resume is not confirmed, and `request cancel` still does not reach it
(fact 11). A Ctrl-C does reach it (fact 41, R-21).

## The Next Step (decision 8)

`ConflictNextDto { operation_id: String, next_action: ConflictNextAction }`,
where `next_action` is `resolve` or `external_repair` in mutation data. It
can be present in three DTOs:

- `CommentMutationDto`;
- `SynchronizationConflictMutationDto`;
- `ConflictResolutionMutationDto`.

The `recovery` array keeps F2's values (fact 7). Human output prints the
next command in full, for example
`next: manyhands-cli conflict show --repo <root> --operation-id <id>`.

Each new mutation DTO carries a single-value `type` enum. That keeps every
golden's data matching exactly one data root, which `DATA_ROOTS` requires
with no `oneOf` (fact 50).

## Command Catalog (R-1)

The seven planned verbs become `Available("C4")` in the task that
dispatches each one. Their summaries stay as cataloged. `item republish` is
added as `Available("C4")`, with the summary "Republish a deleted item
branch."

## Grammar And Per-Verb Rules

| Verb | Selectors | Input | Kind | Confirmed | Request ID in JSON |
| --- | --- | --- | --- | --- | --- |
| `comment add` | `--repo`, `--id` | required | mutation | no | required |
| `item sync` | `--repo`, `--id` | none | mutation | no | required |
| `repo sync` | `--repo` | none | mutation | no | required |
| `item republish` | `--repo`, `--id` | none | mutation | yes | required for `--confirm` and execution |
| `conflict show` | `--repo`, `--operation-id` | none | read | — | refused |
| `conflict resolve` | `--repo`, `--operation-id` | required | mutation | no | required |
| `operation resume` | `--repo`, `--operation-id` | none | mutation | no | required |
| `operation abandon` | `--repo`, `--operation-id` | none | mutation | yes | as C2's confirmed verbs |

- `--repo` is `Required` on all eight. That keeps fact 46's rule without an
  edit.
- `--operation-id` becomes a shared `OPERATION_ID` const, used by
  `operation show` unchanged.
- `--id` resolves the item's kind with one `show_item` read (fact 37, R-2).
  The read's failure passes through under the mutation's name, as C3's fill
  read does (C3 decision 5).
- The mutation verbs that take no input refuse `--input` as a usage error,
  as `index refresh` does.

## Input

- **`comment add`:**
  - `comment_id`: a canonical ULID string, required;
  - `body`: a string, required, which may be empty;
  - `parent_id`: a canonical ULID string, optional, and `null` means a root;
  - `created_by`: a string, optional, validated by R-7.
- **`conflict resolve`:**
  - `observation`: a string, required;
  - `resolutions`: an array of objects, required and non-empty. Each object
    has exactly `token` and `source`, both strings.

  The CLI adds an array-of-objects reader to `input.rs`, which C3's
  readers lack.

Schemas: `schemas/input/v1/comment_add.schema.json` and
`conflict_resolve.schema.json`. Examples:
`tests/fixtures/cli_input_v1/comment_add.json` and `conflict_resolve.json`.
The `conflict_resolve` example is validated by schema only. A live token
cannot be stored, so its run uses a token captured from `conflict show` in
the same test.

## Cancellation (R-21, R-22)

- **The window.** `interrupt::during(|| …)` installs a process-wide Ctrl-C
  handler for the duration of one `execute` of a stoppable verb, and removes
  it afterwards. The stoppable verbs are `item sync`, `repo sync`,
  `item republish`, `comment add`, `conflict resolve` and `operation resume`.
  The window holds a static clone of the `CancellationToken` that the driver
  passes to `execute` in place of fact 40's fresh token.
- **Unix.** `sigaction(SIGINT)` with a handler that calls `cancel()` on the
  first signal: an atomic store, which is async-signal-safe. On the second,
  the handler restores the default disposition and re-raises.
- **Windows.** `SetConsoleCtrlHandler` with a routine that cancels on the
  first `CTRL_C_EVENT` and returns `TRUE`. On the second it returns `FALSE`,
  so the default handler ends the process. The system runs this routine on
  a thread it creates. The CLI spawns none.
- **Prompts are unaffected.** They run before `execute`, in raw mode, and
  read 0x03 as a cancellation (fact 42).
- **Exit.** A cancelled result exits 130 under the RFC's order, ahead of
  partial effects. Its effects name what completed.
- **No retry.** C2's bounded retry never follows a cancellation (fact 40).

## Human Output

- Every verb prints C2's success and refusal lines, with `progress:` lines
  on stderr.
- `comment add` adds this note:
  `note: publishing the comment also publishes this item's other checkpointed work; unsaved drafts are not included.`
- A result with conflict data prints `conflict:` and `next:` lines.
- `remote_branch_deleted` prints
  `next: manyhands-cli item republish --repo <root> --id <id> --prepare`.
- `conflict show` prints the state, the next step, and each path with its
  token. A side is printed only with `--json`, so a terminal is never
  repainted by content. Paths are escaped as C3's `path observe` escapes
  them.

## Exit Statuses

C2's classes apply unchanged. Each code keeps its own class
(`src/results.rs:534-675`), and `failure_class` (`:282`) makes any partial
outcome 4 and any cancelled one 130:

| Exit | Class | Codes |
| --- | --- | --- |
| 0 | success or no-op | `ok`, `already_applied`, including a comment with publication pending and no remote |
| 2 | input | `invalid_input`, `usage_error`, `operation_not_found`, `occupied_path`, `request_mismatch` |
| 3 | blocked | `recovery_required`, `external_change`, `identity_required`, `remote_branch_deleted`, `publication_remote_required`, `push_rejected`, `ticket_closed`, the host and unlock codes |
| 4 | incomplete | any of these after a durable effect, such as a comment checkpointed with publication failing |
| 5 | transient | `busy`, `transport_unavailable` |
| 130 | cancelled | `cancelled` |
| 1 | internal | `internal_error`, output write failure |

Task 0's audit refresh lists every code each C4 verb can return. Task 13's
exit table maps each to a test, or to a reason no fixture reaches it (R-27).

## Dogfooding (R-26)

In the final task, and only with the owner's go-ahead at that step:

1. **Preflight.**
   - The C4 worktree is clean, and pushed.
   - Local `main` equals `origin/main`.
   - Record whether the main checkout's registration has a publication
     remote, and whether the ticket's context is registered there.
2. **Build.** The branch-built CLI adds the C4 ticket's review-ready comment:
   - `comment add --repo /home/vhodges/work/src/manyhands --id 01M4CC0VMWEQQ9G4W0DAG7ADPY --input -`;
   - `created_by` is `Claude Opus 5.5 <noreply@anthropic.com>`;
   - the request ID is recorded in the ledger before it is sent.
3. **If a publication remote is configured,** the comment's publication
   merges local primary into the ticket context and pushes the ticket
   branch. That is what the go-ahead covers. A refusal or partial result
   stops for the owner. It is resumed only under the recorded request ID.
4. **Check by value.** `comment list --id 01M4CC0VMWEQQ9G4W0DAG7ADPY` shows
   the comment, with its ID, parent and `created_by`.
5. **The switch.** The files of fact 48 direct comments to `comment add`,
   with agents passing their noreply `created_by`.
   - `working-on-a-ticket` and `scripts/ticket-loop` keep their canonical
     procedure, under AGENTS.md's existing exception. The skill's comment
     format gains a note that `comment add` with `created_by` is equivalent.
   - The switch has its own independent review.

## Testing

- **Library:**
  - outcome tables 1–5 as tests first, each in the task that owns it;
  - conformance rows for the three new commands;
  - goldens for every new DTO, preview and refusal;
  - privacy: no body, source, token or path in a diagnostic, a request
    store or a `Debug` rendering;
  - `hostile_conflict_paths_are_redacted` extended to the new read's human
    output.
- **CLI:**
  - each verb in JSON and human mode;
  - input schemas and examples;
  - help goldens (E-1);
  - exits;
  - the Ctrl-C window, through the pseudo-terminal against a held fixture
    (`stall_at` / `hold_transfer_after`, fact 44);
  - lost stdout, with a closed pipe.
- **Two clones (R-24):** `tests/support/cli_pair.rs` gives two Manyhands
  clones of one `SshRemoteFixture`. Each has its own `CliEnv`, data
  directory, enabled repository, selected key and approved host. The
  Wave's two-clone rows are `cli_remote` cases. Each case stays under the
  100-second watchdog, splitting a journey where needed.
- **Second actor (track rule 4):** a raw Git peer or a library-level actor
  pushes divergent history and deletes a branch.
- **Redaction:** every `cli_remote` case runs through `scan_output`.

## Named Edits To Pre-Existing Tests

| Edit | Task | File and change |
| --- | --- | --- |
| E-1 | 7–10 | `tests/fixtures/cli_v1/help_root.json` and `help_root_text.txt` are regenerated as each C4 verb becomes available, and for `item republish`'s new row. The diff may change only the verb's availability, repository rule and options, plus the one added row. |
| E-2 | 10 | `src/bin/manyhands-cli/catalog_tests.rs`: `DESIGN`'s C4 row (`:81-92`) gains `("item", "republish")`. |
| E-3 | 1 | `tests/mutation_remote.rs`: the binding-level tests that assert a refusal to merge are rewritten to the merging outcomes of table 2. Candidates, from planning: `a_divergent_remote_is_merge_required_and_holds_its_reservation` (`:1324`), `a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge` (`:2697`), `merge_required_is_not_finished_by_the_same_request_after_the_branch_is_rebased` (`:2754`), `a_resume_never_merges_and_leaves_nothing_of_a_merge` (`:3614`), `a_resume_does_not_continue_a_merge_made_some_other_way` (`:3654`) and `the_sibling_does_not_continue_a_merge_it_is_set_to_refuse` (`:590`). A test that calls the domain with its own `REFUSING` settings (`:365`), such as `the_sibling_refuses_divergence_and_the_existing_function_merges_the_same_fixture` (`:493`), tests the domain setting and stays unchanged. Task 0 classifies each. |
| E-4 | 1 | `tests/fixtures/mutation_v1/repo_sync_merge_required.json` is replaced by `repo_sync_conflict_pending.json`, and its registration in `tests/support/mutation_golden.rs` is renamed. |
| E-5 | 1, 2, 4 | `src/repository/mutation/outcome_tests.rs`: `every_synchronization_error_has_its_code` (`:705`, table `:646-673`) for the conflict rows. Also `a_synchronization_that_stopped_with_its_reservation_held_is_offered_the_same_request` (`:1565`) and `a_blocked_synchronization_with_no_way_forward_is_offered_its_cancellation` (`:1633`), for `merge_required`. The loops over `Command::ALL` take the new commands unchanged. |
| E-6 | 1, 2, 4, 5 | `src/repository/mutation/command.rs`'s tests (`:491-651`) gain the three commands in their sets. `only_a_synchronization_stops_at_safe_points_and_none_needs_confirmation` (`:525`) becomes "and only `item republish` needs confirmation". |
| E-7 | 1–5 | `tests/mutation_contract.rs`: `UNBOUND_COMMANDS` (`:1455`), `DATA_ROOTS` (`:1510`) and `CODES_NO_MUTATION_GOLDEN_HAS` (`:1713`) gain the new entries, with `merge_required`'s reason. |
| E-8 | 1–5 | `tests/read_contract.rs`: `contract_enumerations` (`:703`) gains the new enums, so `published_enumerations_list_exactly_the_registered_values` (`:1018`) counts them. |
| E-9 | 2, 4, 5 | `tests/support/conformance.rs`'s `HOMES` (`:59`) and each home's table gain rows for the three commands. `tests/support/confirmation.rs`'s `preview_schema` (`:49-62`) gains `item republish`. |
| E-10 | 1, 2, 4 | `tests/support/mutation.rs`'s data accessors (`:596-775`) and `tests/support/repair.rs:333` gain arms for the new `MutationDataDto` variants. |
| E-11 | 2, 4 | The resume tests that assert a comment's synchronization is left to inspect, in `tests/mutation_replay/resume.rs` and `src/repository/mutation/bind/` unit tests, change to publication (R-20). Task 0 finds them with the grep in the plan. |

No other pre-existing test changes. Task 0 proves it with the plan's greps,
run after the gate merges. A hit outside this table stops for an owner
ruling.

## Document Amendments

Task 14 makes these, except where noted.

**CLI contract RFC**, "Amended 2026-10-26 by C4":

- the taxonomy rows of the eight verbs, with selectors and input, and the
  new `item republish` row;
- "C4 verbs are available; C5 remains planned";
- `conflict resolve`'s input `{observation, resolutions: [{token,
  source}]}`, replacing `{path, observation, source}`, with the string
  tokens' lifetime (R-9);
- the `comment add` example, using real codes in place of
  `publication_pending`, and `created_by`;
- republish through `item republish` (decision 7);
- conflict data in place of a recovery action (decision 8);
- Ctrl-C's safe-point window (R-21) and lost stdout;
- the C4 rows of the input-schema table, and the evidence paragraph.

**Git workflow RFC:** the bindings merge (decision 3). A conflict's next
step is in data. Republish needs `item republish`'s confirmation.

**Canonical content and comment schema RFC:** `created_by` comes from the
caller's validated input when given, else the Git identity (decision 9).

**Wave 03:**

- the C4 section, "Amended 2026-10-26 by C4", records decisions 1–10, the
  delivery and the known limits;
- the D4 section records that D4 waits for C4 (decision 5);
- the track-rule-5 exception (decision 1).

**API audit:** the C4 refresh section (Task 0), and the C4 rows updated in
place.

**AGENTS.md and the skills of fact 48:** the switch, in Task 15 after the
dogfooding comment succeeds.

**The C4 ticket:** its planning boxes are ticked (Task 15).

## Rulings

Each ruling is listed with its cost if wrong.

- **R-1 Catalog.** Literal `Available("C4")` entries, flipped in the task
  that dispatches each verb, plus the new `item republish` entry. *Cost:*
  none.
- **R-2 Kind lookup.** `--id` verbs resolve the kind with one `show_item`
  read before the request is built. Its failure passes through as C3's fill
  read does. *Cost:* one extra read. A failed read can mask a stored result
  on retry, as C3's limit describes; `request show` still has it.
- **R-3 Observed submission.** A crate-private `submit_comment_observed` and
  `retry_comment_publication_observed`, which take the observer and the
  settings, serve the binding. The public functions keep their behavior.
  *Cost:* two internal entry points to keep in step.
- **R-4 No new fields on literal-built public structs.**
  `SubmitCommentRequest`, `PublishCommentRequest`,
  `RetryCommentPublicationRequest`, `SynchronizeRemoteRequest` and
  `SynchronizationSettings` are built by struct literal in existing tests.
  New behavior enters through crate-private parameters. *Cost:* slightly
  wider internal signatures. Adding fields would force unnamed test edits.
- **R-5 The conflict's operation ID.** `conflict show`, `conflict resolve`
  and `operation resume` accept the synchronization's ID, or a comment's
  operation ID, resolved through its binding. Data always reports the
  synchronization's ID. *Cost:* two accepted forms.
- **R-6 Comment codes.** As table 1. With no remote, the code is `ok` with
  publication pending, as the RFC's exit-0 rule says. *Cost:* a script
  must read `effects.publication`.
- **R-7 `created_by` validation.**
  - The form is `Name <email>`, at most 256 bytes.
  - The name is non-empty, with no `<`, `>`, control character or NUL.
  - The email has one `@`, no whitespace and no `<` or `>`.
  - It is the same check `repo identity-set` applies to its fields, plus
    the form.
  - It is written verbatim.

  *Cost:* any caller can claim any author. That is already true of a
  hand-written file (decision 9).
- **R-8 Conflict data.** `ConflictNextDto` in three tagged DTOs. The
  synchronization binding reads eligibility from one inspection after the
  conflict. *Cost:* one extra local read on a conflict.
- **R-9 String tokens.**
  - **The observation:** `conflict-v1:` plus a BLAKE3 digest over the
    observation's fields.
  - **A path token:** `conflict-path-v1:` plus a BLAKE3 digest over the
    observation digest, the ordinal, the path bytes and the three sides' OIDs
    and modes.
  - **Their life:** both are re-derived from the live conflict on every
    call and never stored. They are valid only while that conflict is
    unchanged.

  *Cost:* a token from before any change is refused, which is the intended
  staleness rule.
- **R-10 Sides inline.** `conflict show` returns sides in its data, and only
  for paths the library can read as UTF-8 canonical text. *Cost:* large
  conflicts make a large response. Items are already bounded by their
  limits.
- **R-11 Display paths.** Escaped for control and non-UTF-8 bytes, as the
  gate's `display_path`. Never used as a writable path. *Cost:* none.
- **R-12 `conflict show` is a read.** It takes no request ID and writes
  nothing. It takes the repository lease while it reads Git, as fact 29's
  inspection does, and a test proves the database unchanged. A held lease
  answers `busy`. *Cost:* a read can answer `busy`.
- **R-13 States.** As the conflict-states table. *Cost:* a new state later
  is a new enum value.
- **R-14 Deterministic attempt and cross-process resumption.**
  - The resolution attempt is derived from the request ID, so a retry of
    the same request reuses it.
  - The library keeps what it needs to finish an attempt whose index or ref
    has moved, without the in-memory request, and without storing a body.
  - The interrupted-resolution table is the contract, written first. A row
    that cannot be met stops at a design checkpoint.

  *Cost:* the largest library change in C4. If the rows cannot be met, the
  owner decides between a narrower guarantee and more library work.
- **R-15 Continuation.** `conflict resolve` publishes under its own request,
  with progress and cancellation. *Cost:* one command does two phases. Its
  effects distinguish them.
- **R-16 Merging settings.** `refuse_divergence: false` in both bindings.
  `merge_required` becomes unreachable from them. *Cost:* the E-3 to E-5
  edits.
- **R-17 Identity during a merge.** Table 2's identity rows are the
  contract. If `repo identity-set` is refused while the merge's reservation
  is held (fact 28), the binding must release the reservation on
  `IdentityRequired`, or check identity before reserving when the history
  diverges. The implementer chooses, test-first, and stops if neither meets
  the rows. *Cost:* a stop.
- **R-18 Republish consent.** A crate-private consent value checked at each
  of fact 36's refusal points against the observation the confirmation
  bound. *Cost:* five refusal points must agree. The table's tests cover
  each.
- **R-19 Republish preview.** `RepublishPreviewDto`, the effect
  `branch_republished` and one desktop arm. *Cost:* a small cross-track
  merge with D3 (decision 10).
- **R-20 Resume routes.** As the resume table. *Cost:* resume can now publish
  a comment. That is what the RFC's example shows.
- **R-21 The Ctrl-C window.** Installed only around `execute` of the six
  stoppable verbs. The first Ctrl-C cancels, and a second ends the process.
  *Cost:* a Ctrl-C between execute calls of C2's bounded retry ends the
  process at once, which is interruption, as before C4.
- **R-22 Cancellation's exit.** 130 even after a checkpoint, as the RFC's
  order puts cancellation first. *Cost:* a script must read effects after
  130.
- **R-23 Retry.** C2's bounded retry unchanged. It never follows
  `cancelled` or `recovery_required`. *Cost:* none.
- **R-24 Two-clone support.** `tests/support/cli_pair.rs`, new, built on
  `SshRemoteFixture` and `CliEnv`. Cases are split to stay under the
  watchdog. *Cost:* a new support module.
- **R-25 Lost stdout.** Proven with a closed stdout pipe on `item sync` and
  `comment add`. The process exits 1, then `request show` and a replay
  follow. *Cost:* none.
- **R-26 Dogfooding.** As the Dogfooding section, with the owner's
  go-ahead at that step. `working-on-a-ticket` and `ticket-loop` keep their
  procedure. *Cost:* two comment paths coexist until that skill is revised.
- **R-27 Exit accounting.** As C3's decision 6: every exit reached through
  the binary, and the unreachable codes listed with reasons. Set equality
  with Task 0's audit union. *Cost:* none.
- **R-28 Golden names.** `<resource>_<verb>_ok`, at least one
  `<resource>_<verb>_<code>` and `<resource>_<verb>_text`, plus
  `item_republish_preview` and `operation_abandon_preview`. *Cost:* none.
- **R-29 No new test target.** New tests are modules of existing targets,
  which are already in the native list (fact 52). *Cost:* none.
- **R-30 C3's stuck save.** `operation abandon` is shown closing C3's
  interrupted-and-edited save (C3's known limit). Its file stays as it is.
  *Cost:* none.

## Known Limits

- A pending remote conflict cannot be abandoned (decision 6). It blocks
  other synchronizations and authoring in that context until it is resolved
  or repaired.
- `request cancel` does not reach a resume. Ctrl-C does.
- A wrong passphrase reports `transport_unavailable`
  (`01M5E6DYJ6GN7DMW4424TH3V78`).
- An unusable identity is refused late (`01M5MK57JKCSQFXBNZ8DBE548K`).
  Table 2's identity rows cover the missing case only.
- Operation reads name no item for comment rows
  (`01M4W5SCBV6WFJP11Y7PJF5VS8`).
- `created_by` is not authenticated (R-7).
- A string token is valid only while its conflict is unchanged (R-9).

## Alternatives Rejected

- **A separate library ticket (track rule 5).** The owner chose to carry the
  library part in C4 (decision 1).
- **A `conflict.show` recovery action.** It would edit the desktop's
  exhaustive match (decision 8).
- **`item sync --republish`.** Confirmation is fixed per command (fact 12,
  decision 7).
- **Serializing the library's tokens.** Their fields are private by design.
  String tokens are re-derived (R-9).
- **A separate content read, as the gate has.** One `conflict show` call
  gives a CLI user everything to write a resolution (R-10).
- **An identity prompt in sync.** C3 has none for saves, and the RFC routes
  identity through `repo identity-set`.

## Risks

| Risk | Control |
| --- | --- |
| The gate's Task 7 changes the inventories and records C4 extends | Decision 2: C4 starts after it merges; Task 0 re-greps E-1 to E-11 |
| Cross-process resolution resumption needs more library change than planned | R-14's table, written first, with a stop |
| Merging changes the premise of many F2 tests at once | E-3 to E-5 list them; Task 0 classifies each before code |
| A Ctrl-C handler leaks past `execute` | The window is scoped and tested to be removed; a Ctrl-C after it ends the process as before |
| Two-clone CLI journeys exceed the 100-second watchdog | Cases are split (R-24) |
| The dogfooding comment publishes local main into the ticket branch | The preflight records local main against `origin/main`, and the go-ahead is at that step |
| D3 and C4 both edit `src/desktop/view/confirm.rs` | One arm; whichever merges second resolves it (decision 10) |
