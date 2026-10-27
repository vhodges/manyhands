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

**Draft, 2026-10-26.** Revised after the third review round, and re-grounded
on 2026-10-27 after the gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY` merged
(`a372edb`). Written for product-owner review. Nothing is built.
Implementation needs the gate merged (Cycle decision 2), which it now is, and
the owner's authorization.

## Intent And Approval Boundary

This design turns the [Cycle](../Cycles/wave-03-cli-04-discussion-and-sync.md)
into bindings, verbs, inputs, outcome tables and tests. The Cycle governs
this design, and this design governs the
[implementation plan](2026-10-26-wave-03-cli-04-discussion-and-sync-implementation.md).

Approval of this design covers:

- the library part of Cycle decisions 1, 3, 4, 7, 8, 9, 11, 12, 13, 17,
  18, 19 and 20, and decision 21's test edit;
- the per-verb grammar and input fields;
- the five required-outcome tables and the resume table;
- rulings R-1 to R-41;
- named edits E-1 to E-12;
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
- the comment checkpoint's author and parent check, and the release of a
  binding that wrote nothing (R-34);
- the durable state that lets the same request finish an interrupted
  resolution (decision 13, R-35);
- a push-only republish path (decision 11, R-18);
- the new data and preview DTOs and their schemas;
- the crate-private parameters these need.

No result code or recovery-action kind is added (decision 8).

## Baseline Facts

First read at branch head `219ec57` (main `0464c42` merged), and on the gate
branch at `0b75184`.

**Re-grounded on 2026-10-27 at `e920043`.** That commit merges `origin/main`
at `a372edb`, where the gate is merged and closed. Its `src/` and `tests/` are
identical to `a372edb`. Three read-only reviewers re-read every fact, and the
citations elsewhere in all three documents:
- facts 1, 3, 8–11, 19, 25, 30, 33, 49, 53, 64, 67, 72, 75 and 76 changed, and
  are rewritten below;
- the rest moved or hold, and their citations are updated;
- facts 80–86 are new.

The **(gate)** markers are resolved. Task 0 re-checks every fact again if main
moves before execution. A changed fact that alters a task stops there for a
controller ruling.

Facts 1–53 were read for the first draft. The first review round confirmed
facts 1–27, 29–35, 37–41 and 44–52 against the source, and corrected facts
28, 36, 43 and 53, as written below. Facts 54–66 were added after it, and the
second round confirmed them. Facts 67–74 were added after the second round;
the third round confirmed 67, 69, 70 and 73, and corrected 68, 71, 72 and 74.
Facts 75–79 were added after the third round.
Facts marked **(confirm)** rest on a reading that Task 0 must prove, and name
the check.

**The mutation boundary.**

1. `Command` (`src/repository/mutation/command.rs:37-70`) has `ItemSync`
   ("item sync"), `RepoSync` ("repo sync"), `OperationResume` ("operation
   resume") and `OperationAbandon` ("operation abandon"). The gate added
   `LocalContextSync` ("context sync-local") and `LocalContextResolve`
   ("context resolve-local"), so `Command::ALL` has 32 entries. There is no
   comment command and no remote conflict command. `Mutation` is at
   `src/repository/mutation/mod.rs:401`.
2. The inputs:
   - `ItemSyncInput { root, kind: AuthoringKind, id: ItemId }`
     (`bind/sync.rs:61`);
   - `RepoSyncInput { root }` (`:69`);
   - `OperationResumeInput { root, operation_id }` (`bind/resume.rs:63`);
   - `OperationAbandonInput { root, operation_id }` (`bind/abandon.rs:33`).

   `ItemSyncInput` and `RepoSyncInput` do not derive `Clone`.
3. `needs_confirmation` (`command.rs:447`) is a `const fn` of the command
   alone. It is true for `OperationAbandon`, and false for `ItemSync`,
   `RepoSync`, `OperationResume` and both local commands.
   `stops_at_safe_points` (`:275`) is true for `ItemSync`, `RepoSync`,
   `LocalContextSync` and `LocalContextResolve` (`:277`).
   `LocalContextResolve` has no operation (`:89-90`) but stops at safe points.
   Both local commands report `Observing`.
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
8. `SynchronizationMutationDto` (`dto.rs:658`) holds only host data: "A
   synchronization reports no other data". `MutationDataDto` is at
   `dto.rs:540`. Its first variant is now the gate's `LocalContextSync`.
   `MutationDataDto::authority()` (`dto.rs:977-992`) matches every variant.
9. `operation resume` (`bind/resume.rs:1-34`) takes up three kinds of work:
   - a local operation's owed index hand-off;
   - a remote synchronization, "and nothing is merged";
   - since the gate, a local-context sync. It is routed by operation ID into
     `bind/local_sync.rs:330-513`, from `prepare` (`resume.rs:826-830`),
     `run` (`:864-866`), `reenter` (`:887-896`) and `rerun` (`:952-954`). A
     local sync whose resolution request is unfinished answers
     `original_request_required`. It names `required_request_id`, with
     `LocalContextSyncMutationDto` data (`bind/local_sync.rs:355-383`).

   For the remote cases:
   - A comment's synchronization is left to inspect, so resume answers it
     `recovery_required` (`:217-232`).
   - An ended row answers `already_applied`, and a cancelled one is left to
     inspect, `recovery_required` (`:246-274`, `:464-467`).
10. `operation abandon` (`bind/abandon.rs`, unchanged by the gate):
    - it is confirmed, with `OperationAbandonPreviewDto`;
    - an ID the remote or key journal holds is `invalid_input`;
    - it calls `abandon_operation` (`bridges/abandon.rs:432`);
    - a `SubmitComment` row counts as `Writes::Item` (`:208`), and a
      `ContextSyncLocal` row is listed beside it (`:211`);
    - a bound local-sync operation goes to `local_sync::abandon_bound`
      (`bridges/abandon.rs:456-476`, `local_sync/control.rs:215-265`).
      - It succeeds only for an artifact-free `Prepared` or a proven-released
        `IndexPending`.
      - It finishes every associated accepted request as `cancelled`
        (`mutation/local_control.rs:274-325`).
      - Only the generic fall-through is refused (`bridges/abandon.rs:542-559`).
11. `cancel_request` (`progress.rs:366`) asks only a command whose
    `stops_at_safe_points` is true (`:383`). `request cancel` therefore does
    not reach a resume. It does reach `context resolve-local`, which has no
    operation: its acknowledgement is read from the gate's control counters
    (`progress.rs:427-433`, `records.rs:906-915`).
12. `prepare` (`confirm.rs:286`) answers `not_confirmable` when
    `mutation.confirmed()` is `None`. `CONFIRMATION_LIFETIME_SECONDS = 600`
    (`:47`). A confirmable command provides five things:
    - `needs_confirmation`;
    - a `Mutation::confirmed()` arm, which clones its input;
    - a binding implementing `Confirmed::{preview, row, observe_again}`;
    - a `PreviewDetailsDto` variant with its preview effects;
    - a preview schema of its own.
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
    `mod.rs:370`, and `cancel_request` at `progress.rs:366`.

**Comments.**

17. `submit_comment<P>(&self, PublishCommentRequest, &mut
    SessionCredentials<P>) -> Result<CommentSubmissionOutcome,
    CommentSubmissionError>` (`src/repository/comment_publication.rs:178`).
    - `PublishCommentRequest { comment, approval, confirmed_identity }`.
    - `SubmitCommentRequest { target: AuthoringTarget, comment_id,
      parent_id, body, expected_destination }` (`src/repository.rs:656`).
    - It takes no observer and no cancellation token.
18. Publication calls `synchronize_remote` on the context target, under the
    binding's child `synchronization_id` (`comment_publication.rs:499`). That
    function uses `SynchronizationSettings::default()`, which merges, and no
    observer (`remote/sync.rs:5267-5278`).
19. `checkpoint_comment_locally` (`src/repository.rs:2759`) refuses, in this
    order:
    - a context with a pending synchronization conflict (call `:2770`,
      function `:3642`);
    - an operation ID that a local-context sync owns, at `begin_lifecycle`
      (`:2776`, `recovery.rs:691-693`). The gate added this check;
    - a lifecycle-closed ticket (`refuse_closed_ticket`, function `:252`,
      call `:2810`);
    - an intent other than `Edit` (`:2817`);
    - a parent that is not a conforming comment of the same item, with
      `MissingAuthoringTarget` (`:2883-2895`), which becomes `item_not_found`
      (`outcome.rs:68`);
    - a new row whose `expected_destination` is not `Missing` (`:2914`).

    The binding row is inserted at `:2825-2826`, before the parent check.
20. `created_by` is set from the Git identity at the checkpoint:
    `comment.created_by = Some(format!("{} <{}>", identity.name,
    identity.email))` (`src/repository.rs:3064`).
    - With no identity, the answer is `IdentityRequired { context }`. That
      happens after the binding row is inserted and before the file is
      written.
21. The binding table `comment_publication_bindings` (`recovery.rs:47-62`)
    is keyed by `operation_ulid`. It holds one unique child
    `synchronization_ulid` (`:55`), and no body. `matches_request`
    (`:27-44`) compares kind, item, comment, parent and intent.
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
    `remote/sync.rs:5284`. `SynchronizeRemoteRequest { root, operation_id,
    target, approval, confirmed_identity, restart }` (`remote/state.rs:1730`)
    has no republish field.
25. Merging is `integrate_divergence` (`sync.rs:3252`). It stayed in
    `remote/sync.rs`. The gate moved only its helpers (`classify_integration`,
    `install_conflict`, `fast_forward`, `import` and the validation functions)
    into `src/repository/integration/{prepare,artifacts,validate}.rs`, and
    `sync.rs` keeps thin wrappers.
    - With `refuse_divergence: false` (`sync.rs:353`), a context target merges
      the remote context first (ordinal 0) and then primary (ordinal 1). A
      primary target merges primary only.
    - The subjects are "Merge remote context {id}", "Merge primary into
      {kind} {id}" and "Merge remote primary" (`:3441-3452`).
    - A conflict leaves index stages and `MERGE_HEAD` in the target worktree
      (`install_conflict`, `integration/artifacts.rs:232`). It releases the
      reservation (phase `interrupted`, `reservation.rs:1875-1879`), and
      returns `ConflictPending` (`sync.rs:3506-3540`). The import first
      re-checks the reservation's owner (`:3457-3462`), with the same behavior.
26. Restarting an operation:
    - the same operation without `restart` is `RecoveryRequired`;
    - with `restart` before a resolution, it is `ConflictPending`, offline;
    - after a resolution, it publishes the resolved commit
      (`tests/remote_merge_recovery.rs:1616-1649`).
27. While a conflict is pending:
    - every new synchronization reservation is `Busy`
      (`reservation.rs:1152-1161`);
    - authoring in that context is `RecoveryRequired`
      (`src/repository.rs:3665-3678`).
28. Merge identity:
    - `select_committing_identity` (`sync.rs:2766-2791`) returns
      `IdentityRequired { target, expected_configuration }`;
    - the integration step's intent is persisted before that check
      (`:3346-3351`);
    - `set_local_identity` is not refused by a pending operation: "An
      operation that stopped for want of an identity is taken up again once
      there is one" (`bridges/identity.rs:44-47`). It takes only the
      repository lease (`:717`);
    - `committing_identity` uses the effective identity, so a restart after
      `repo identity-set` merges.
29. `inspect_synchronization_recovery(root, operation_id) ->
    Result<SynchronizationConflictInspection, SynchronizationError>`
    (`sync.rs:4006`):
    - it runs SELECTs in IMMEDIATE transactions, and takes the repository
      lease;
    - it needs one step in `ConflictPending`, `ResolutionPrepared` or
      `CommitPrepared` (`:4035-4040`);
    - a HEAD other than the step's local parent, or an index fingerprint other
      than the stored conflict digest, is `ExternalChange` (`:4056-4063`). So
      at `ResolutionPrepared` the index stages are still intact, and the
      inspection still issues an observation and tokens.
30. `SynchronizationConflictInspection` (`sync.rs:262`) holds `operation_id`,
    `target`, `stage`, `local_parent`, `incoming_parent`, `observation` and
    `paths`. Each path is `SynchronizationConflictPath { token, eligibility }`
    (`:255`), so it carries no kind or structure.
    - Eligibility is one value for the whole set (`conflict_eligibility`,
      `integration/validate.rs:42-57`).
    - Each entry is classified by `classify_conflict_entry`
      (`validate.rs:59`), whose service wrapper is private to `sync`
      (`sync.rs:3938`).
    - The kinds are `ConflictEntryKind` (`Document`, `Ticket`, `Comment`,
      `Noncanonical`; `validate.rs:16-21`).
    - The structures are `ConflictStructure` (`RegularUtf8SamePath`, `Binary`,
      `Executable`, `Symlink`, `Rename`, `Delete`, `IdentityChanged`, and the
      gate's `IncompatibleClosures`; `:24-33`).
    - The gate classifies a ticket path whose sides carry different
      `closed_at`/`closed_by` as `IncompatibleClosures`, with kind still
      `Ticket` (`:166-184`). That makes the whole set external-only.
    - Both enums are `pub(in crate::repository)`. `remote::merge` re-exports
      them only under `cfg(test)` (`merge.rs:107-108`).
31. `ConflictObservation` and `ConflictPathToken` (`merge.rs:43`, `:85`)
    have only `pub(super)` fields and no `Serialize`. Two inspections of
    unchanged state compare equal. The fingerprint is index-based
    (`integration_conflict_digest`, `sync.rs:2622`; `conflict_digest`
    `:2574`), so removing markers in the worktree alone leaves it unchanged.
32. `read_synchronization_conflict(&ConflictPathToken) ->
    Result<EphemeralSynchronizationConflictSides, _>` (`sync.rs:4122`):
    - it returns base, local and incoming from index stages 1, 2 and 3;
    - `current` is always `None` (`:4189`);
    - non-UTF-8 content is `ExternalResolutionRequired` (`:4191-4199`).
33. `resolve_synchronization(ResolveSynchronizationRequest) ->
    Result<ResolveSynchronizationOutcome, SynchronizationError>`
    (`sync.rs:4664`) resolves "without transport or publication".
    - `ResolveSynchronizationRequest::new(root, synchronization_id,
      attempt_id, observation, resolutions, identity)` (`merge.rs:202`).
    - Its outcomes are `LocalCheckpointComplete { commit_oid }`,
      `IdentityRequired`, `StaleObservation`, `ValidationFailed` and
      `RecoveryRequired` (`merge.rs:236-243`).
    - An external-only set, including incompatible closures, is now
      `Err(ExternalResolutionRequired)` before any write
      (`sync.rs:4686-4695`). It used to be `Ok(ValidationFailed)`.
    - Re-acquiring the conflict calls `local_sync::record::require_no_pending`
      (`reservation.rs:1917`). So while any local-context sync in the
      repository is unfinished, both a fresh resolution (`sync.rs:4734`) and
      an identical retry of a retained candidate (`:4315`) fail with
      `Repository(RecoveryRequired)`, before any attempt row or write.
34. Resuming a resolution:
    - at `CommitPrepared` or `Applied`, `reconcile_resolution_candidate`
      (`sync.rs:4204`) accepts only the identical attempt and input digest;
    - at `ResolutionPrepared`, the attempt is reused when the inputs match
      (`state.rs:4543-4578`);
    - a new attempt after success is `RecoveryRequired`;
    - `remote_resolution_attempts` and `remote_resolution_paths`
      (`state.rs:818-851`) are durable.

    Every test of resumption reuses the in-memory request
    (`tests/comment_publication.rs:2913`; `sync_tests.rs:6651`).
35. An outside repair is accepted when:
    - HEAD has exactly the parents `[local, incoming]`;
    - the target is clean;
    - `validates_external_integration` passes (`integration/validate.rs:505`,
      wrapper `sync.rs:3983`). The parents and validation are checked at
      `sync.rs:2953-2961`.

    A restart then continues it (`reconcile_pending_candidate`, `:2859`).
    Removing markers alone, staging without a commit, or wrong parents are
    refused (`refused_false_repairs`, `tests/remote_merge_recovery.rs:2457`).
36. Remote deletion:
    - an absent, previously published context is `RemoteContextDeleted`
      (`refs.rs:262-268`), which becomes `remote_branch_deleted`;
    - the same state is refused at `sync.rs:5815-5834` and `:6209-6213`,
      which surface `RemoteContextDeleted`, `HistoryUnknown` or
      `RecoveryRequired` through `PushAbsenceBoundary::error`
      (`sync.rs:452-459`);
    - it is also refused at `reservation.rs:2690-2697` and `:2752-2759`,
      which surface `RecoveryRequired`;
    - a fetch marks an absent published context `remotely_deleted` in
      `remote_context_states` (`state.rs:89-99`, `:2431`), keeping
      `last_advertised_oid`, which can be null.
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
    - the `Boundary` impl passes `&CancellationToken::new()` to `execute`
      (`:60`);
    - `should_retry` (`:73-84`) retries `busy`, or a partial result whose
      first recovery action is `request.retry` for the same request;
    - a cancelled result is never retried.
41. The synchronization observer `Watch` (`bind/sync.rs:300`, cancelling at
    `:348` and `:405`) cancels at a safe point when the session's token is
    set or the request's flag is. Resume calls `sync::resumed` with that
    observer, so a token cancelled during `operation resume` stops it.
42. Nothing in `src/` installs a signal or console-control handler. Prompts
    use raw mode, which clears `ISIG` (`terminal/unix.rs:62`) and
    `ENABLE_PROCESSED_INPUT` (`terminal/windows.rs:98`). `libc = "0.2"` and
    `windows-sys` with `Win32_System_Console` and `Win32_Foundation` are
    dependencies (`Cargo.toml:23`, `:38`).
43. `PtyCli` (`tests/support/pty.rs:22`; `spawn` at `:79`) gives the child a
    controlling terminal:
    - `wait` and `expect` have 10-second deadlines (`:238`, `:294-309`);
    - `finish` expects an exit code and panics on death by signal (`:352`);
    - the module is `cfg(unix)` (`tests/support/mod.rs:35`).
44. The `cli_remote` harness:
    - `Case` is at `tests/support/ssh_harness.rs:48`;
    - each case has a 100-second watchdog (`:237`);
    - `World::new` (`tests/support/remote_world.rs:139`) builds one
      Manyhands clone and one raw peer, and sets a local identity
      (`:156-157`);
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
    people by a `created_by` containing `<noreply@`. AGENTS.md's existing
    exception covers `working-on-a-ticket`'s status edits and closure only.

**Inventories and tests.**

49. A new `Command` joins these closed inventories. The gate's two commands
    joined every one:
    - `command.rs`'s predicates (`:87`, `:155`, `:193`, `:232`, `:275`,
      `:324`, `:360`, `:404`, `:447`) and their tests (`:491-698`);
    - `Mutation` (`mod.rs:401`), `kind()` (`:442`), `confirmed()` (`:482`)
      and `into_binding()` (`:522`);
    - `recorded_effects` (`bind/mod.rs:903`, match `:930`), which now
      short-circuits on `local_control::effects` (`:904-906`);
    - `from_stored` (`dto.rs:996`). It has two `OperationResume` arms: one
      guarded on the stored `type` (`:1002-1007`, `local_context_sync`), and
      the general arm (`:1037`);
    - `MutationDataDto::authority()` (`dto.rs:977-992`), which the compiler
      enforces;
    - `CASES` in `tests/support/mutation_golden.rs:21`, and the read golden
      `CASES` in `tests/support/golden.rs:26`;
    - in `tests/mutation_contract.rs`: `DATA_ROOTS` (`:1510`, now 50 entries),
      `CODES_NO_MUTATION_GOLDEN_HAS` (`:1717`) and
      `CODES_NO_FIXTURE_CAN_PRODUCE` (`:1678`);
    - in `tests/read_contract.rs`: `contract_enumerations` (`:704`), now a
      `let mut` vector extended at `:1009-1036`, and
      `published_enumerations_list_exactly_the_registered_values` (`:1052`);
    - `HOMES` (`tests/support/conformance.rs:59`, now 32 entries) and each
      home's table, run by `mutation_bridges` among others. The per-target
      chain is `tests/mutation_replay/conformance.rs:12-18`;
    - `preview_schema` (`tests/support/confirmation.rs:48-61`);
    - the data accessors (`tests/support/mutation.rs:596-794`) and
      `tests/support/repair.rs:333`.

    The commit-reachability exemption (`replay.rs:41-54`) and the
    operation-action lists (`read_contract.rs:687`, `bridges/abandon.rs:211`)
    are command-specific, but they are not closed inventories. C4 adds
    nothing to them.

    `UNBOUND_COMMANDS` (`mutation_contract.rs:1455`) lists commands that
    `execute` does not bind, so a bound command never joins it.
50. The schema checker allows only `type`, `properties`, `required`,
    `additionalProperties`, `items`, `minimum`, `enum` and `$ref`
    (`tests/support/schema.rs:98-160`). It has no `oneOf` or `const`.
51. Tests that pin the bindings' refusal to merge are listed under named
    edit E-3.
52. The test targets C4 extends are already in the native list
    (`.github/workflows/build.yml:115`): `mutation_remote`,
    `mutation_bridges`, `mutation_contract`, `mutation_replay`,
    `mutation_confirmation`, `read_contract`, `cli` and `cli_remote`.

**The gate.**

53. As merged at `a372edb`, the gate:
    - adds `observe_local_context_sync` (`local_sync/observe.rs:360`) and
      `RepositoryOperation::ContextSyncLocal` (`src/repository.rs:1057`);
    - adds a request-control table, `local_context_sync_request_control`
      (`mutation/local_control.rs:17`). `accept` (`records.rs:614`), `enter`
      (`:852`) and `may_settle` (`:625`, `:666`) call it for every request.
      It asks nothing of a command that is not local: `accept` returns `Ok`
      (`local_control.rs:161`), and `enter` and `may_settle` pass through when
      there is no row (`:180-187`, `:244-251`);
    - moves merge helpers into `src/repository/integration/` (fact 25);
    - delivers the `LocalContextSync` and `LocalContextResolve` mutations
      (`bind/local_sync.rs:10-25`) and their reads (`read/local_sync.rs:39-125`).
      Its local resolution takes a top-level observation and a complete
      `{token, source}` set (`bind/local_sync.rs:16-25`). It checks syntax
      first and replays before it checks live tokens (`:240-256`, `:287-295`;
      `local_sync/resolve.rs:307-321`);
    - reads a local conflict's sides separately, with nullable `base`,
      `local` and `incoming`, and `historical: true` while a resolution
      attempt is accepted (`local_sync/conflict.rs:218-283`);
    - gives an attempt to one request only. Another request's active attempt
      is `external_change` (`local_sync/resolve.rs:335-339`), and resume
      answers `original_request_required` naming it (fact 9). There is no
      supersede;
    - checks a resolution's path count first, as `invalid_input`
      (`resolve.rs:435-437`), and then membership, as `external_change`
      (`:440-443`).

**Added after the first review round.**

54. A binding reaches credentials only through `trait Session`
    (`progress.rs:183-212`), whose `synchronize(service, request, settings,
    observer)` calls `synchronize_remote_observed` with the call's
    credentials. `CallSession<P>` keeps the credentials private
    (`:227-231`). `Watch` is private to `bind/sync.rs` (`:300`). It reads the
    stored push, `push_is_unsettled` and the fence by the accepted
    operation's ID (`:170-211`, `:635`). Resume builds an `Accepted` for a
    child operation at `resume.rs:647-655`.
55. `prepare` runs its binding with `session: &NoSession` (`confirm.rs:318-322`),
    whose `synchronize` returns `None` (`progress.rs:296-313`). A preview
    cannot contact the remote.
56. A cancellation is final. A restart that acknowledges a cancelled or
    interrupted row answers `Interrupted` (`sync.rs:5420-5433`). After a
    cancelled comment publication, `retry_comment_publication(restart: true)`
    answers `Pending(Synchronization(Interrupted))`. Only a new ordinary
    synchronization publishes the comment
    (`tests/comment_publication.rs:2990-3050`).
57. `ValidIdentity::of` (`src/repository/bridges/identity.rs:145-177`)
    requires non-empty values, no control characters, no leading or trailing
    whitespace, an `@` in the email, and a `git2` signature round trip.
58. The SSH harness filters by case name, by substring or `--exact`
    (`ssh_harness.rs:95-104`). `#[test]` functions never run in a
    `harness = false` target. No existing case name begins with `c4_`.
59. About 26 test targets mount `tests/support/mod.rs` without
    `ssh_remote`. A module added to it must compile in all of them.
60. The SSH harness sets `GIT_CONFIG_NOSYSTEM` and the Git search paths
    (`ssh_harness.rs:29-41`, `:224-226`), so no system identity reaches an
    SSH case.
61. `Boundary::execute` (`mutate.rs:40-48`) takes no token, and `Plan`
    literals appear in `mutate_tests.rs:199`, `:368` and `:412`, with a
    `FakeBoundary` at `:103` (impl `:111`).
62. `tests/cli_remote.rs` uses the `manyhands` crate (`:2`), not the library
    as a module, so it reaches public items only. `mutation_remote`,
    `comment_publication` and `remote_merge_recovery` mount `src/lib.rs` as
    a module. `mutation_remote` also mounts `golden.rs`,
    `mutation_golden.rs`, `schema.rs`, `conformance.rs` and
    `remote_world.rs` (`tests/mutation_remote.rs:6-28`). The other two
    mount none of the golden support.
63. A fast-forward of a behind local branch with nothing to push answers
    `AlreadyCurrent` (`refs.rs:281-285`; `sync.rs:6254`, `:6743`), which is
    `already_applied`.
64. No library `FailurePoint` precedes the first conflicted path write in a
    remote resolution (`src/repository.rs:971-977`). The first remote point
    follows a path write (`sync.rs:4981`). The gate's 18 new `LocalSync*`
    points serve the local executor only. All remote resolution points and
    the attempt and reconcile code are in `sync.rs:4625-5253` and
    `state.rs`. `integration/` has none.
65. After a comment's `IdentityRequired` no checkpoint exists, so
    `retry_comment_publication` fails to prove a receipt
    (`comment_publication.rs:338-343`, `:548-555`). The submit path continues
    from an existing receipt with `restart: false` (`:216-224`).
66. `should_retry` also re-runs a partial `push_rejected` and a partial
    `recovery_required` whose first action is `request.retry`, up to three
    more times, under the same request.

**Added after the second review round.**

67. Settlement (`settle.rs:40-75`) keeps a record for `Owed`, `Unconfirmed`
    and an unread journal. It finishes one whose journal row was cancelled or
    retained, and deletes a stopped record whose row is completed and not in
    flight. A command with no operation reads its row as `Absent`
    (`bind/mod.rs:518-529`), so any stop deletes its record.
    - `read_journal_row` asks `local_control::journal` first (`:522-524`).
      That returns `None` for every command that is not local, so a remote
      `conflict resolve` still reads `Absent`.
    - `settled` replays early for local-control requests only
      (`settle.rs:86-94`).
68. `bind_comment_publication` refuses a new binding whose comment ID is
    already bound in the repository (`recovery.rs:292-297`, unique
    `(root_digest, comment_ulid)` at `:61`). A comment request whose record
    was deleted, and that comes back under a new operation ID, is refused.
    The local row is completed on `IdentityRequired` (`src/repository.rs:3186-3189`).
69. For a context target, the clean plan makes the final commit descend
    cleanly from both the remote context and remote primary
    (`refs.rs:247-286`, `clean_descendant` `:289-302`). So item sync's path
    always integrates primary, and fast-forwards to it when the context is an
    ancestor.
70. `remote_resolution_paths` stores a `path_digest`, not the path
    (`state.rs:838-851`). `reconcile_resolution_candidate` hashes each
    token's path into its input digest (`sync.rs:4256-4273`).
71. `prepare_resolution_attempt_in_window` accepts a new attempt while a step
    is `ResolutionPrepared`, `CommitPrepared` or `Applied`
    (`state.rs:4521-4530`, insert at `:4588`).
72. `validate_child_id` (`state.rs:4446-4462`) refuses a child ULID equal to
    any operation, confirmation or attempt ULID. Since the gate it also
    refuses a ULID a local-context sync owns (`:4450`), as `OperationMismatch`
    rather than `recovery_required`. The same local-ownership check runs:
    - on every non-refresh local begin (`recovery.rs:691-693`);
    - on a comment's child ID (`recovery.rs:299`);
    - in `local_sync/record.rs:946-970`.
73. `checkpoint_comment_locally` is called directly by existing tests:
    `src/repository/ticket_guard_tests.rs:331`,
    `src/repository/remote/sync_tests.rs:2404`, `:2484`, `:2491`, `:2547`
    and `:6868`, and `tests/remote_merge_recovery.rs:955`. Its one production
    caller is `comment_publication.rs:232`. The gate added none.
74. A call that begins with an unsettled push from an earlier call, and is
    then cancelled, answers `not_confirmed`: `recovery_required` with
    `Standing::Final`, not `cancelled` (`bind/sync.rs:811-819`). `unsettled`
    is set once, at the start of a call (`:635`, `:207-211`). Within one call,
    a Ctrl-C after the push returns is `cancelled`, with the publication read
    from the stored push (`:357-359`).

**Added after the third review round.**

75. At service start, `audit_registry` (`state.rs:5333-5337`) checks:
    - the exact column lists of four tables: `remote_integration_steps`,
      `remote_identity_confirmations`, `remote_resolution_attempts` and
      `remote_resolution_paths` (`validate_merge_evidence_schema`,
      `:1284-1369`, compared at `:1366`);
    - the SQL of `MERGE_SCHEMA_OBJECTS` (`:1261`);
    - foreign keys on 17 remote tables (`:5364-5381`).

    So a new column on an existing table makes main's binaries answer
    `recovery_required` on a registry the branch has migrated. A new table is
    not checked by binaries that do not list it, and no code enumerates
    unknown tables.

    The gate added no column. It added eight tables, each audited by its
    exact SQL on every registry migration (`recovery.rs:630`, `records.rs:162`,
    `:174`):
    - `local_context_sync_records`, `_steps`, `_resolutions` and `_execution`
      (`local_sync/record.rs:14-90`);
    - `_conflicts` and `_resolution_cancellation`
      (`local_sync/conflict_record.rs:7`, `:12`);
    - `_images` (`local_sync/image_proof.rs:7`);
    - `_request_control` (`mutation/local_control.rs:17`).
76. While a resolution attempt row exists, an outside repair is refused.
    `observe_external_integration` refuses when
    `integration_has_resolution_attempt` (`state.rs:3405-3416`, helper
    `:3384-3394`), and `sync.rs:3023-3030` relies on it.
77. `push_exact` has no guard on the remote ref's old value
    (`transport/remote.rs:184-199`). Only deletion has one
    (`callbacks.rs:178-188`). `src/repository/transport/` is outside C4's
    library files.
78. A remote operation's `action` is one of poll, `synchronize_context`,
    `synchronize_primary`, promote and close (`state.rs:729`), so a republish
    is recorded as `synchronize_context`. Resume routes any pending such row
    to `sync::resumed` (`resume.rs:253-266`). `records::operation_holder`
    names the request that holds an operation (`resume.rs:186-197`).
79. A request record holds only `accepted.operation()` (`execute.rs:307`).
    A comment's child synchronization ID is created later
    (`recovery.rs:298`, with the gate's local-ownership check at `:299`), so
    `request cancel` on a `comment add` request sets that request's flag but
    never calls `cancel_remote_operation` for the child.

**Added after re-grounding on the merged gate (2026-10-27).**

80. An unfinished local-context sync anywhere in the repository blocks
    remote work. A local sync counts as unfinished when its state is not
    `complete`, `cancelled` or `abandoned` (`local_sync/record.rs:1082-1089`).
    - Every remote reservation answers `recovery_required`, through
      `require_no_pending_local` (`reservation.rs:1146`). That check runs
      before the `Busy` checks.
    - Beginning a comment's lifecycle answers `recovery_required`
      (`recovery.rs:1348`, `:791`).
    - Re-acquiring a remote conflict answers `recovery_required` (fact 33).

    No CLI or desktop verb can start a local sync before C5 or D5 delivers
    `context sync-local` (decision 19). Until then only the library can make
    this state.
81. Two tests pin the registry's exact set of tables:
    - `the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`
      (`tests/mutation_replay.rs:400`). It drops `REQUEST_TABLES` (`:347`,
      four entries), and asserts that no remaining object's name contains
      `request` (`:442`). After migration it asserts that exactly
      `REQUEST_TABLES` was added (`:464`).
    - `registry_creates_repository_and_discovery_metadata_schema`
      (`tests/repository_enablement.rs:475`) asserts the exact table list
      (`:581-624`). **It fails on `a372edb`**, because the list lacks the
      gate's eight tables (fact 75): `local_context_sync_conflicts`,
      `_execution`, `_images`, `_records`, `_request_control`,
      `_resolution_cancellation`, `_resolutions` and `_steps`. Nothing else
      differs. This was run on 2026-10-27, as the one test of its target; it
      was the only Cargo run in planning. C4's E-12 adds them (Cycle decision
      21).
82. The remote state tests drop and re-create the resolution tables
    (`src/repository/remote/state_tests.rs:1898`, `:2008`, `:2071`, `:2114`,
    `:2203`, `:2470`).
83. The gate's private `display_path` hex-escapes every byte outside printable
    ASCII, valid UTF-8 included, and doubles backslashes
    (`local_sync/request.rs:317-330`).
84. `bind/sync.rs` says "The bindings never merge." (`:100`) and "Only a merge
    commits, and nothing here merges." (`:590`).
85. The gate amended C4's authorities in `4e3a67a` and `baea8ee`:
    - **CLI RFC**, "Conflict and cancellation behavior" (`cli-contract.md:753-781`):
      - `conflict show` and `conflict resolve` remain "a future CLI adapter
        contract";
      - "C4/D5 must adapt this contract deliberately and still provide
        separate remote-conflict wire bindings";
      - the local shapes, among them nullable sides plus `historical`, and
        escaped display paths;
      - "A partial accepted resolution must resend the same R
        request/source when `required_request_id` names R";
      - "Safe-point cancellation preserves recoverable Accepted requests";
      - "abandonment requires artifact-free proof".
    - **Git RFC**: the "Dedicated Local-Primary Into Context Synchronization"
      section, and a "Local recovery amendment". The amendment says:
      - "partial R without a fully prepared candidate requires its original
        source bytes";
      - "`operation resume` names R through `original_request_required`";
      - incompatible closures are external-only, and "choosing one
        incompatible closure is not proof of completion".
    - **Wave**: the gate note, which says "C4/D5 must adapt the local boundary
      and still provide the separate remote-conflict wire boundary". It also
      rewrote the capability row "Conflict inspection read", which now begins
      "Local boundary: closure-recovery gate; remote boundary: first of C4 or
      D5".
    - **API audit**: the section "closure-recovery-library-amendment-2026-10-27".
      Among other things, it says reads "do not take the write lease".
86. **(confirm)** A remote resolution writes each path in the worktree, and
    stages its result into the locked authoritative index in memory
    (`resolution::stage_result`). It writes a tree from that index
    (`sync.rs:4940-5000`). The failure point `ResolutionAfterPathWrite`
    fires before that tree is written. Read from the code, an attempt stopped
    at `ResolutionPrepared` leaves the index stages on disk intact. The
    inspection then still matches the stored digest (fact 29), and the sides
    are still readable. Task 3's `resolution_prepared` row proves this, and a
    different answer stops there.

## Module Layout

**Library:**

- `src/repository/mutation/bind/comment.rs` (new): the `comment add`
  binding.
- `src/repository/mutation/bind/conflict.rs` (new): the `conflict resolve`
  binding.
- `src/repository/mutation/bind/republish.rs` (new): the `item republish`
  binding.
- `src/repository/remote/conflict_read.rs` (new): the conflict read and the
  string tokens. It sits inside `remote` because the tokens' fields are
  `pub(super)` (fact 31). It gets each path's kind and structure by calling
  `crate::repository::integration::validate::classify_conflict_entry` on
  each index conflict (fact 30). That call is read-only, and it edits nothing
  in `integration/`.
- Edited:
  - `bind/sync.rs` (merging settings, conflict data, `Watch` made
    `pub(super)`);
  - `bind/resume.rs` (routing);
  - `command.rs`, `mod.rs`, `dto.rs`, `outcome.rs` and `bind/mod.rs`
    (inventories);
  - `comment_publication.rs` (the synchronizer-driven submission);
  - `src/repository.rs`: a crate-private `checkpoint_comment_locally_with`
    (author, the parent check before the binding row, the binding released
    on `IdentityRequired`), which the unchanged `checkpoint_comment_locally`
    delegates to, so fact 73's callers are untouched;
  - `mutation/progress.rs`: a crate-private `Session::republish` method
    beside `synchronize` (R-18);
  - `remote/mod.rs` (module registration);
  - `remote/sync.rs`, `remote/refs.rs`, `remote/reservation.rs`,
    `remote/state.rs` and `remote/merge.rs`, for the push-only republish
    path, and for resolution association and resumption. Every function C4
    edits stayed in these files (facts 25 and 64). C4 edits nothing in
    `integration/` or `local_sync/`, which the gate's local executor shares.
- New schemas under `schemas/v1/`:
  - `comment_mutation`;
  - `conflict` and `conflict_path`;
  - `conflict_next`;
  - `synchronization_conflict_mutation`;
  - `conflict_resolution_mutation`;
  - `item_republish_preview`.

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

- **Library:** Part A's tables are modules of `tests/mutation_remote.rs`,
  which mounts the golden, schema and conformance support (fact 62). Each
  case is an `ssh_harness` case named `c4_<module>_<case>`, registered in
  the target's `CASES` (R-31).
- **CLI:** tests join `tests/cli/` and `tests/cli_remote/`.
- **New support:** `tests/support/cli_pair.rs`, mounted with `#[path]` only
  from `tests/cli_remote.rs` (fact 59). `tests/support/pty.rs` gains one
  additive helper that returns a signal death instead of panicking.

No new `[[test]]` target (R-29).

## Command Predicates (R-32)

| Predicate | `CommentAdd` | `ConflictResolve` | `ItemRepublish` |
| --- | --- | --- | --- |
| `has_operation` | true | false, as `OperationResume`: it acts on an existing operation | true |
| `was_accepted_without_operation` | false | false | false |
| `restores_index` | false | false | false |
| `creates_its_repository` | false | false | false |
| `stops_at_safe_points` | true | false, as `OperationResume` (R-35) | true |
| `finishes_as_a_synchronization` | true | false | true |
| `stage` | `Writing` | `Writing` | `Connecting` |
| `hands_off_to_any_refresh` | false | false | false |
| `needs_confirmation` | false | false | true |

- `finishes_as_a_synchronization` equals "stops at safe points, or resume",
  as `command.rs:574`'s test asserts (`:576-580`).
- `ConflictResolve` does not stop at safe points, so `request cancel` asks
  nothing of it (fact 11). That is C4's choice (R-32, R-35), not a
  consequence of holding no operation. The gate's `LocalContextResolve` also
  holds no operation, yet stops at safe points, and `request cancel` reaches
  it through the gate's control counters (facts 3 and 11). C4 does not reuse
  those counters, because they belong to the local executor. The Ctrl-C
  window still covers `conflict resolve` (R-21).
- `CommentAdd` reports `Writing`. The stopping commands' stages are now
  `Connecting` for the remote synchronizations and `item republish`,
  `Observing` for the gate's two local commands, and `Writing` for
  `comment add`. E-6 covers the test that pins them.

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

It derives no `Debug`, `Display` or `Serialize`. The request digest covers
every field, the body by digest.

- **Target.** An `AuthoringTarget` with `intent: Edit` and the request's
  operation ID. A fresh request expects the comment path to be `Missing`
  (fact 19).
- **Author (decision 9, R-7).** A given `created_by` replaces the Git
  identity's text in the comment's front matter, in
  `checkpoint_comment_locally_with`. It is written once. Without it, fact
  20's identity applies. A commit identity is still needed for the
  checkpoint either way.
- **Parent check.** `checkpoint_comment_locally_with` checks the parent
  before it inserts the binding row, so a refused parent leaves no row
  (fact 19).
- **Publication (R-3).** The binding passes the submission a crate-private
  synchronizer. That is a closure over
  `call.session.synchronize(service, request, MERGING, &mut watch)`.
  - `watch` comes from a new `pub(super)` constructor of `Watch`, given an
    `Accepted` that carries the child `synchronization_id` (fact 54).
  - Its owner is the `comment add` request. `request cancel` on that request
    sets its flag, which this `Watch` reads at the next safe point. Its
    `acknowledged` stays false, because the record does not hold the child
    operation (fact 79).
  - Publication therefore merges, reports progress and stops at safe
    points. `submit_comment` and its public request types are unchanged
    (R-4).
- **Standing (R-34).**
  - **`IdentityRequired` with nothing written.** If this operation recorded
    no write step and the comment file is absent, one registry transaction
    deletes the binding and marks the local row `rejected`. The record stays
    accepted. A retry under the same request ID then begins the same
    operation again as new, and binds afresh (`recovery.rs:804-820`). A
    checkpoint that landed in the context meanwhile changes nothing (fact
    68).
  - **`IdentityRequired` after a write.** If an earlier attempt of this
    operation wrote the file, the binding and the row are kept. The same
    request checkpoints the written file once after `repo identity-set`.
  - **Pending publication.** A `Saved` result with publication pending is
    `Standing::Owed`, so the record stays accepted. The exceptions are a
    child row that is final, such as one cancelled, and a code that is never
    finished by the same request, such as `remote_branch_deleted`. Those are
    `Standing::Final`.
  - `ended_without_checkpoint` answers `SaysNothing`.
  - The public `submit_comment` never releases a binding, so its behavior is
    unchanged. The release happens only when the binding asks for it, by a
    parameter of `checkpoint_comment_locally_with`.
- **Re-entry.** A re-entered request calls the same submission with its full
  input and a crate-private `restart: true` (fact 65). It proves the receipt
  and publishes without writing again.
- **Data.** A `CommentMutationDto`, tagged `type: "comment"`, holding:
  - `item_id`, `comment_id`, `parent_id` and `created_by`;
  - `synchronization_id`;
  - `publication`: a `PublicationEffect`, which is never `not_requested`
    here;
  - `pending_reason` (`no_publication_remote`, `recovery_required`,
    `synchronization` or null);
  - `conflict` (a `ConflictNextDto` or null).

  It holds no body.

### Required outcomes: comment submission

Written as a test first (Task 2b). A row that answers otherwise stops for a
ruling.

| Case | Code | Outcome, exit | Effects | Data, recovery |
| --- | --- | --- | --- | --- |
| Remote configured, publishes | `ok` | success, 0 | write written, checkpoint committed, publication published | publication `published` |
| No publication remote | `ok` | success, 0 | checkpoint committed, publication pending | `pending_reason: no_publication_remote` |
| Push refused by the server | `push_rejected` | partial, 4, after C2's retry re-runs it three more times (fact 66) | checkpoint committed, publication pending | `pending_reason: synchronization`; `request.retry` first |
| The same request retried after the refusal, the server now accepting | `ok` | success, 0 | publication published | one comment file, one comment checkpoint, `created_at` unchanged |
| Publication meets a canonical conflict | `recovery_required` | partial, 4, after C2's retry re-runs it offline (facts 26, 66) | checkpoint committed, integration pending, publication pending | `conflict: {operation_id: <synchronization_id>, next_action: resolve}` |
| Another item's conflict is pending | `busy` | partial, 4, after C2's bounded retry | checkpoint committed, publication pending | `pending_reason: synchronization` |
| This item's context has a pending conflict | `recovery_required` | blocked, 3 | nothing | `[repo.inspect]` |
| A local-context sync anywhere in the repository is unfinished (fact 80, R-38) | `recovery_required` | blocked, 3 | nothing; no binding row | `[repo.inspect]` |
| Lifecycle-closed ticket | `ticket_closed` | blocked, 3 | nothing; no context or binding left | none |
| No identity and no `created_by` | `identity_required` | blocked, 3 | nothing written; binding released and row rejected; record kept (R-34) | `[request.retry, repo.identity_set]`; after `repo identity-set`, the same request ID succeeds and writes once |
| `created_by` given, no Git identity | `identity_required` | blocked, 3 | as the row above | `[request.retry, repo.identity_set]` |
| A checkpoint lands in the context between `identity_required` and the retry | `ok` | success, 0 | the comment is written once on the new tip | as the first row |
| An earlier attempt wrote the file and stopped before its checkpoint; the retry finds no identity | `identity_required` | partial, 4: the file is written | binding and row kept | `[request.retry, repo.identity_set]`; after `repo identity-set`, the same request checkpoints the written file once |
| Publication meets a remotely deleted context | `remote_branch_deleted` | partial, 4 | checkpoint committed, publication pending; the record finished | human output names `request cancel` for the request, then `item republish` |
| `created_by` malformed | `invalid_input` | input, 2 | nothing | none |
| `parent_id` names no comment of this item, or the comment itself | `item_not_found` | input, 2 | nothing; no binding row | none |
| `comment_id` already used by another file | `occupied_path` | input, 2 | nothing | none |
| Ctrl-C during publication, before a push | `cancelled` | cancelled, 130 | checkpoint committed, publication pending | The operation is final (fact 56): a new `item sync` publishes the comment |
| Ctrl-C after the push returned, before it is verified | `cancelled` | cancelled, 130 | checkpoint committed; publication as the stored push says (fact 74) | a new `item sync` verifies it |
| A later request that begins with that unsettled push, then is cancelled | `recovery_required` | partial, 4 | publication pending: not confirmed (fact 74) | a new `item sync` verifies and completes it |
| No publication remote at submission, one added later | — | — | — | `operation resume` is not the route: no child synchronization ran. A new `item sync` publishes the comment |
| Lost stdout after success | — | exit 1 | as success | `request show` returns `ok`; a replay writes and pushes nothing |

## Merging Synchronization (decision 3)

The bindings pass `refuse_divergence: false` (R-16). `item sync` and
`repo sync` then integrate divergent history as fact 25 describes.

- **Next step.** A conflict-pending result carries
  `SynchronizationConflictMutationDto`, tagged
  `type: "synchronization_conflict"`. It holds `conflict`, a
  `ConflictNextDto`. The binding reads the eligibility from one inspection
  after the conflict (R-8).
- **Effects.** `integration: pending` marks a merge begun and not finished.
  The checkpoint is not committed.
- **`merge_required`.** The bindings no longer return it. It stays a result
  code, listed with its reason in `CODES_NO_MUTATION_GOLDEN_HAS` and
  `CODES_NO_FIXTURE_CAN_PRODUCE` (E-7).
- **Identity (R-17).** `repo identity-set` is not refused by the stopped
  merge (fact 28). The same request then restarts and merges. No workaround
  of `01M4R19PC0T6ZPKM39X9DF26HX` is involved.

### Required outcomes: divergent synchronization

For both `item sync` and `repo sync`, written as a test first (Task 1).

| Case | Code | Outcome, exit | Effects and data |
| --- | --- | --- | --- |
| Already current | `already_applied` | success, 0 | publication current |
| Remote behind, local ahead | `ok` | success, 0 | publication published |
| Local behind | `already_applied` | success, 0 | the local branch fast-forwards; nothing pushed (fact 63) |
| Clean divergence | `ok` | success, 0 | One merge commit with parents `[local, incoming]` and the fact 25 subject, published. No rebase; the remote keeps its earlier tip as an ancestor |
| Canonical conflict | `recovery_required` | blocked, 3 | integration pending; `conflict: {operation_id, next_action: resolve}`; the reservation is released |
| `item sync`: the remote context merged (stage 0), then primary conflicts (stage 1) | `recovery_required` | partial, 4: the stage-0 merge commit is durable | integration pending; the stage-0 commit kept; `next_action` from the stage-1 conflict |
| Conflict in a noncanonical path | `recovery_required` | blocked, 3 | integration pending; `next_action: external_repair` |
| The same request again, conflict still pending | `recovery_required` | blocked, 3 | the same conflict data; no transport (fact 26) |
| A new sync while another operation's conflict is pending | `busy` | transient, 5, after C2's bounded retry | nothing |
| A local-context sync anywhere in the repository is unfinished (fact 80, R-38) | `recovery_required` | blocked, 3 | nothing; `[repo.inspect]`. This is checked before the `busy` row's check |
| Target worktree not clean | `worktree_not_clean` | blocked, 3 | nothing |
| Target worktree in an unfinished outside merge | `worktree_conflicted` | blocked, 3 | nothing |
| Advertisement moved between fetch and push | `external_change` | blocked, 3 | nothing pushed |
| Push rejected, no merge made | `push_rejected` | blocked, 3 | nothing durable |
| Push rejected after a merge commit | `push_rejected` | partial, 4, after C2's retry (fact 66) | the merge commit stays; publication pending |
| Index refresh fails after publication | `discovery_pending` | partial, 4 | publication published, discovery pending; `[operation.resume]` |
| Divergence with no usable identity | `identity_required` | blocked, 3 | `[request.retry, repo.identity_set]` (fact 7) |
| Then `repo identity-set` | `ok` | success, 0 | the identity is set; not refused by the stopped merge (fact 28) |
| Then the same sync request | `ok` | success, 0 | the merge is committed and published once |
| Remote context deleted | `remote_branch_deleted` | blocked, 3 | nothing recreated; human output names `item republish` |
| No publication remote | `publication_remote_required` | blocked, 3 | nothing |
| Ctrl-C at a safe point | `cancelled` | cancelled, 130 | Effects name what completed. The operation is final (fact 56): a new request completes it |
| Ctrl-C after the push returned, before it is verified | `cancelled` | cancelled, 130 | publication as the stored push says (fact 74); a new request verifies it |
| A later request that begins with that unsettled push, then is cancelled | `recovery_required` | blocked, 3, or partial, 4, after a merge commit | publication not confirmed (fact 74) |
| Ctrl-C during a blocked network call | `cancelled`, exit 130, at the next safe point; or `transport_unavailable`, exit 5, or 4 after a merge commit, with effects intact (decision 14) | — | Task 7's spike records which. Anything that misreports effects stops |

## Republish (`item republish`, decisions 4, 7 and 11)

`Mutation::ItemRepublish(ItemRepublishInput { root, kind, id })`, command
`"item republish"`. It is confirmed and stops at safe points. The input
derives `Clone`.

- **Closed tickets (decision 17).** A lifecycle-closed ticket is refused with
  `ticket_closed` at prepare and at execution.
- **Preview.** Prepare cannot contact the remote and holds no reservation
  (fact 55), so it reads stored evidence through a new reservation-free read.
  That read runs over the same records the refusal points use:
  - `remote_context_states` marked `remotely_deleted` by a prior fetch (fact
    36);
  - the earlier operations' push evidence, as
    `synchronization_push_absence_boundary` classifies it
    (`sync.rs:6498-6553`), without taking a reservation.

  Only a `Deleted` classification makes a preview. `Ambiguous`, `Unknown` or
  no evidence gives the no-op preview, which says to run `item sync` first.

  `ItemRepublishPreviewDto` holds:
  - `root`, `kind`, `item_id`, `branch` and `remote`;
  - `last_published_oid`, which can be null;
  - `local_oid`.

  Its effect is the new `PreviewEffect::BranchRepublished`
  (`branch_republished`). The observation digest binds every field and the
  stored deletion state.
- **Execution (decision 11, R-18).** A push-only domain path, not item
  sync's clean plan, because that plan always integrates primary (fact 69).
  The binding calls it through a new crate-private `Session::republish`.
  1. It fetches.
  2. It refuses any change since the preview, before any push:
     - the context advertised again is `external_change`;
     - a local tip other than `local_oid` is `external_change`.
  3. It pushes exactly `local_oid`. The push leaves out the push-absence
     boundary (`sync.rs:6209-6213`) and the per-record refusal
     (`reservation.rs:2742-2759`), which are what refuse a deleted branch;
     the consent replaces them. It keeps the reservation's other checks,
     the unfinished local-sync refusal among them (fact 80).
  4. It records the publication, so the context is published again.

  - It integrates nothing, and the next `item sync` integrates primary as
    usual. Ordinary `item sync` still refuses at fact 36's points.
  - The push cannot be guarded as a create (fact 77). A branch recreated
    elsewhere between the fetch and the push is refused as `push_rejected`.
    One recreated at an ancestor in that window is fast-forwarded; that is a
    known limit.
  - `operation resume` on an operation held by an `item republish` request
    answers `original_request_required`, naming it (fact 78). Resume never
    takes a republish's operation through item sync's merging path.
- **Records.** An `external_change` finishes the record, so its recovery is
  `[request.prepare]`: a new preview. A transport stop leaves it accepted,
  and the same request continues.
- **Data.** None: `data` is null, and the effects report the publication.
- **Desktop.** One `PreviewDetailsDto::Republish` arm in
  `src/desktop/view/confirm.rs` shows "Branch", "Last published" and "Local
  commit" (decision 10, R-19).

### Required outcomes: republish

Written as a test first (Task 5).

| Case | Answer |
| --- | --- |
| `--prepare`, deletion recorded by a fetch | Preview with `branch_republished`, the branch, the remote, `last_published_oid` and `local_oid`, exit 0 |
| `--prepare`, deletion shown only by push-absence evidence classified `Deleted` | The same preview, exit 0 |
| `--prepare`, push-absence evidence `Ambiguous` or `Unknown` | The no-op preview, exit 0, saying to run `item sync` first |
| `--confirm`, the branch recreated elsewhere between the fetch and the push | `push_rejected`, exit 3; nothing pushed |
| `--confirm` stopped after the push returned; then `operation resume` on its operation | `original_request_required`, exit 3, naming the republish request; nothing merged or pushed |
| `--prepare` with no deletion evidence: the branch exists, was never published, or the deletion is not yet observed | A no-op preview, `already_applied`, exit 0, with no confirmation, saying to run `item sync` first |
| `--prepare` on a lifecycle-closed ticket | `ticket_closed`, exit 3 (decision 17) |
| `--confirm`, nothing changed | `ok`, exit 0: the branch created once at `local_oid`, no merge, publication published |
| `--confirm` after remote primary moved since the context's last integration | `ok`, exit 0: exactly `local_oid` pushed, no merge |
| `--confirm` where the context is an ancestor of primary | `ok`, exit 0: exactly `local_oid` pushed, not primary's tip |
| `--confirm` after the local branch moved | `external_change`, exit 3; nothing pushed; `[request.prepare]` |
| `--confirm` after the branch reappeared on the remote | `external_change`, exit 3; nothing pushed; `[request.prepare]` |
| `--confirm` while the refused `item sync` request still holds its reservation (`01M4R19PC0T6ZPKM39X9DF26HX`) | `busy`, exit 5, after C2's bounded retry; human output names `request cancel` for that request first |
| `--confirm` while a local-context sync in the repository is unfinished (fact 80, R-38) | `recovery_required`, exit 3; nothing pushed; `[repo.inspect]` |
| `--confirm` meeting a transport, unlock or host stop | the code `item sync` gives it, such as `transport_unavailable` (5), `unlock_required` (3) or `host_approval_required` (3); nothing pushed; the same request continues |
| Replay of a confirmed request | Its stored result; no second push |
| `item sync` after a republish, primary unchanged | `already_applied` |
| Ctrl-C during execution | `cancelled`, exit 130. The operation is final (fact 56); a new prepare is needed |
| Ctrl-C at the interactive consent prompt | `cancelled`, exit 130, before execution; nothing reserved |

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
| `state` | `conflict_pending`, `resolution_prepared`, `resolved`, `repaired`, `repair_refused`, `cancelled` or `complete` |
| `next_action` | `resolve`, `retry_resolution`, `external_repair`, `resume`, `synchronize` or `none` |
| `request_id` | string or null: the request whose resolution attempt is unfinished. That request may continue it, and under decision 18 a new request may supersede it (R-35) |
| `eligibility` | `eligible_canonical`, `external_resolution_required` or null |
| `local_parent`, `incoming_parent` | strings |
| `observation` | string or null (R-9) |
| `historical` | boolean: true when an attempt has written paths, so the sides are the recorded conflict's and the worktree may already hold results (R-39) |
| `paths` | array of `ConflictPathDto` |

`ConflictPathDto` holds:

- `display_path`, escaped by C4's rule (R-11);
- `token`, a string (R-9);
- `kind`: `document`, `ticket`, `comment` or `noncanonical` (decision 12);
- `structure`: `regular`, `binary`, `executable`, `symlink`, `rename`,
  `delete`, `identity_changed` or `incompatible_closures` (fact 30, R-40);
- `content_available`, a boolean;
- `base`, `local` and `incoming`, each a string or null.

Sides are present only when the library returns them (fact 32, R-10). Paths
are ordered by raw path bytes. Eligibility stays one value for the whole set
(decision 12). A conflict with any path that is not canonical, or any
`incompatible_closures` path, is repaired outside as a whole.

`show_conflict` takes remote synchronizations only (decision 19). A
local-context sync's operation ID is `invalid_input`. It is found in the
gate's records before the remote lookup, so it is never reported as
`operation_not_found` (R-37).

### Required outcomes: conflict states

Written as a test first (Task 3).

| State of the operation | `state` | `next_action` | Paths and observation |
| --- | --- | --- | --- |
| Conflict pending, every path canonical with structure `regular` | `conflict_pending` | `resolve` | every path with kind and sides; observation present |
| Conflict pending, any path noncanonical or structural | `conflict_pending` | `external_repair` | every path with kind; sides only where readable; observation present |
| Conflict pending, two sides of a ticket closed differently (fact 30) | `conflict_pending` | `external_repair` | that path with kind `ticket` and structure `incompatible_closures`; observation present. `conflict resolve` is refused for the whole set (R-40) |
| Markers removed in the worktree, stages intact | `conflict_pending` | as the rows above | as above (fact 31) |
| A resolution stopped after writing paths | `resolution_prepared` | `retry_resolution`, with `request_id` | as the conflict, with `historical: true` and the observation present (fact 86). The same request continues it. A new request is admitted only by decision 18's supersede rule (step 3), never beside it (fact 71) |
| A resolution stopped with its commit prepared, the ref not yet moved | `resolution_prepared` | `resume`: `operation resume` finishes it from durable state (decision 18) | empty; observation null |
| Resolved and checkpointed, not yet published | `resolved` | `resume` | empty; observation null |
| Resolved, then a local save in the context before publication | `resolved` | `resume`, which publishes the branch tip, containing the resolution | empty; observation null |
| Repaired outside with an accepted two-parent commit | `repaired` | `resume` | empty; observation null |
| Staged without a commit, or committed with wrong parents | `repair_refused` | `external_repair` | empty; observation null |
| The merge aborted outside: HEAD at the local parent, no `MERGE_HEAD`, no stages | `repair_refused` | `external_repair` | empty; observation null. The operation cannot be abandoned (decision 6, `01M4H33R34Z7C7EEKTY1ZCT950`) |
| Resolved, then the continuation's publication cancelled | `cancelled` | `synchronize`: a new `item sync` or `repo sync` publishes the resolution commit (fact 56) | empty; observation null |
| Published | `complete` | `none` | empty; observation null |
| The repository lease is held | `busy`, exit 5 | — | — |
| An unknown operation ID | `operation_not_found`, exit 2 | — | — |
| An operation that is no synchronization | `invalid_input`, exit 2 | — | — |
| A local-context sync's operation ID (decision 19) | `invalid_input`, exit 2 | — | — |

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

The input shape matches the gate's local resolution: a top-level
observation and a complete `{token, source}` set (fact 53). The binding also
orders its checks as the gate does: syntax first, then replay. It departs from
the gate where the remote operation needs it, and Document Amendments records
each departure:
- decision 18's supersede (step 3);
- sides given inline, with `historical` (R-10, R-39);
- the lease (R-12).

The types derive no `Debug`, `Display` or `Serialize`. The command is not confirmed. Like `operation
resume`, it holds no operation of its own and does not stop at safe points
in the library's sense (R-32, R-35). The Ctrl-C window covers it. The digest
covers the resolved synchronization's ID, the observation and every
token/source pair in token order.

The binding works in seven steps.

1. **Syntax.** A malformed observation or token, an empty set or a
   duplicate token is `invalid_input`, before anything is read. A
   local-context sync's operation ID is `invalid_input` too (decision 19,
   R-37).
2. **Replay first (decision 13, R-35).** If this request has a recorded
   attempt for this synchronization, the binding reconciles it before it
   checks any live token, as the gate orders it. The comparison is with the
   input digest stored with the association, not with a rebuilt observation,
   so a later change to `config.toml` does not stop the same request:
   - a matching input finishes the attempt with its recorded candidate, and
     skips straight to the continuation if the candidate is already applied;
   - a different input under the same request ID is `request_mismatch`.
3. **Association (decisions 13 and 18).** If another request's attempt is
   unfinished:
   - at `ResolutionPrepared`, a new request supersedes it if every path the
     attempt wrote still holds exactly what it wrote, as the attempt's
     recorded digests show. The old attempt is closed, and the new one
     proceeds. The gate's local resolution has no such rule. Decision 20
     keeps it for remote, and scopes the gate's "resend the same R"
     sentences to the local operation;
   - at `ResolutionPrepared`, with a written path since changed, the answer
     is `original_request_required`, naming the request;
   - at `CommitPrepared` or `Applied`, the answer is `recovery_required`
     with `next_action: resume`, because `operation resume` finishes it from
     durable state.

   A new request never starts a second attempt beside an unfinished one
   (fact 71).
4. **Validate.** It re-derives the live conflict's observation and tokens
   (R-9), and compares them with the input. It checks in this order, and
   answers the first that applies:
   1. an operation whose conflict is already resolved is
      `recovery_required`, with `[repo.inspect]` (fact 34);
   2. a different observation is `external_change`;
   3. an external-only set, including `incompatible_closures`, is
      `recovery_required` with `next_action: external_repair`. The gate
      answers `external_change` for its local equivalent, and Document
      Amendments records the difference;
   4. a count of resolutions other than the conflict's path count is
      `invalid_input`. A missing path falls here;
   5. a well-formed token that is not in this conflict is `external_change`,
      as the gate has it (R-33). Count before membership is the gate's order
      too (fact 53).

   Nothing is written in any of these cases.
5. **Resolve.** It calls `resolve_synchronization` under the derived
   attempt.
6. **Continue.** On `LocalCheckpointComplete`, it checks the token, then
   continues the operation's publication under the same request (R-15):
   - a comment's synchronization through the comment's synchronizer-driven
     retry, with `restart: true`;
   - any other synchronization through a restart with the merging settings.

   The continuation's `Watch` owner is this request, whose flag nothing sets
   (fact 11). So the Ctrl-C token stops it. So does `request cancel` on the
   original synchronization request, which still holds the operation
   (`progress.rs:448-462`), and the resolve then reports `cancelled`. A
   Ctrl-C seen before the continuation starts stops after the checkpoint,
   with the synchronization left `resolved` and resumable.
7. **Map the other outcomes, and give the data:**
   - `StaleObservation` is `external_change`;
   - `ValidationFailed` is `invalid_input`, and nothing is written;
   - `IdentityRequired` is `identity_required` with `repo.identity_set`;
   - `ExternalResolutionRequired`, now an `Err` before any write (fact 33),
     is `recovery_required` with `next_action: external_repair`;
   - `Repository(RecoveryRequired)` while a local-context sync is unfinished
     (fact 80) is `recovery_required` with `[repo.inspect]`, and nothing is
     written (R-38);
   - the data is `ConflictResolutionMutationDto`, tagged
     `type: "conflict_resolution"`. It holds `operation_id`, `commit_oid`,
     `publication` and `conflict`, and `conflict` is set when publication
     met a new conflict.

### Required outcomes: interrupted resolution

Written as a test first (Task 4a). The interrupting actor is a library
failure point. Before the first path write, where no failure point exists
(fact 64), it is an SQL trigger. Each retry uses a fresh `RepositoryService`
on the same data directory, so no in-memory state carries over. Task 12b's
binary journeys cover separate processes. A row that answers otherwise
stops at a design checkpoint (decision 13).

| Interrupted at | Retry of the same request, same input | Effects after the retry |
| --- | --- | --- |
| Before any path is written | `ok`, exit 0 | one resolution commit, published |
| After the paths are written (`ResolutionPrepared`) | `ok`, exit 0 | the same candidate commit; one commit; published |
| After the commit is prepared, before the ref moves | `ok`, exit 0 | the same commit OID; no second commit; published |
| After the ref moves, before publication | `ok`, exit 0 | the same commit OID; published |
| During publication | `ok`, exit 0 | published once |
| Any of the above, then a different `source` under the same request ID | `request_mismatch`, exit 2: the command records no operation of its own (R-32), so the refusal carries no effects | nothing changes |
| After the paths are written, the first request lost, and the written paths unchanged: a new request | `ok`, exit 0: it supersedes the attempt (decision 18) | one resolution commit, published |
| After the paths are written, a written path since edited by hand: a new request | `original_request_required`, exit 3, naming the first request | nothing written |
| After the commit is prepared or the ref moved, the first request lost: `operation resume` | `ok`, exit 0, finishing from durable state (decision 18) | the prepared commit, no second one; published |
| After the commit is prepared or the ref moved: a new `conflict resolve` request | `recovery_required`, exit 3, `next_action: resume` | nothing written |
| `config.toml` changed after the paths are written, then the same request | `ok`, exit 0: the stored input digest matches | as row 2 |
| Published by another request (`operation resume` or a sync), then the same request | `already_applied`, exit 0, with the same commit | nothing written or pushed |
| After success, a new request with any resolution | `recovery_required`, exit 3, `[repo.inspect]` (fact 34) | nothing written |
| Any of the above, retried while a local-context sync in the repository is unfinished (fact 80) | `recovery_required`, exit 3, `[repo.inspect]` | nothing written; the attempt kept, so the same retry finishes it once the local sync ends |
| After success and before publication, `operation resume` | `ok`, exit 0 | published |
| After publication, `operation resume` | `already_applied`, exit 0 | nothing |
| Ctrl-C during the local resolution, before publication | `cancelled`, exit 130 | the checkpoint is committed; the synchronization stays `resolved`, and `operation resume` publishes it |
| Ctrl-C during publication | `cancelled`, exit 130 | The checkpoint is committed. The synchronization is final (fact 56): a new `item sync` or `repo sync` publishes the resolution commit |

## Resume (R-20)

`operation resume` adds four routes to fact 9's remote cases. The gate's
local-sync routing stays as the gate delivers it (decision 19). C4 changes
nothing in it, and gives the CLI human output for its data (R-37).

| Operation | Resume does | Data |
| --- | --- | --- |
| A comment's synchronization, or the comment's own operation, with its checkpoint present and publication pending, not cancelled | The synchronizer-driven retry with `restart: true`: publishes the comment, never writes it | `CommentMutationDto` |
| A comment request that stopped at `identity_required` | Nothing to take up: its binding was released (R-34). Resume answers its completed local row as F2 does, `already_applied`; the same `comment add` request ID writes the comment | none |
| A synchronization with a resolution checkpointed or an accepted outside repair | A restart with the merging settings: publishes it | null |
| A synchronization whose resolution attempt has its commit prepared, or its ref moved, unpublished | Finishes the attempt from durable state alone, with no input (decision 18), then publishes it | null |
| A synchronization whose resolution attempt has written paths only (`ResolutionPrepared`) | `original_request_required`, exit 3, naming the attempt's request, as the gate's local resume does (decision 20). Human output adds that a new `conflict resolve` may supersede it while the written paths are unchanged (decision 18). Nothing is written | none; the `original_request` recovery action names the request, as for a republish |
| An operation held by an `item republish` request | `original_request_required`, naming that request (fact 78) | none |
| A synchronization whose conflict is still pending | `recovery_required`, exit 3, with no transport | `SynchronizationConflictMutationDto` |
| A cancelled operation | As F2: `recovery_required`, left to inspect (fact 9). A new request completes the work (fact 56) | none |
| A local-context sync | The gate's routing, unchanged (fact 9) | the gate's `LocalContextSyncMutationDto` |

`from_stored`'s `OperationResume` arm accepts these data shapes. The gate's
arm guarded on `type: local_context_sync` stays first (fact 49). C4's shapes
are matched by their own `type` values after it. Resume is not confirmed,
and `request cancel` still does not reach it (fact 11). A Ctrl-C does
(fact 41, R-21).

## The Next Step (decision 8)

`ConflictNextDto { operation_id: String, next_action: ConflictNextAction }`,
where `next_action` is `resolve` or `external_repair` in mutation data. The
read's own `next_action` enum (`ConflictReadNextAction`) is wider, because a
read also sees resolved, repaired, cancelled and published states. It
can be present in three DTOs:

- `CommentMutationDto`;
- `SynchronizationConflictMutationDto`;
- `ConflictResolutionMutationDto`.

The `recovery` array keeps F2's values (fact 7). Human output prints the
next command in full, for example
`next: manyhands-cli conflict show --repo '<root>' --operation-id <id>`, with
the root quoted and escaped. An identity need is reported by its code and
`repo.identity_set`, not by a `set_identity` next action as the gate has.
That departure is recorded under Document Amendments.

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
- The five unconfirmed mutations answer `not_confirmable` to `--prepare`.
  `conflict show`, a read, refuses `--prepare` as a usage error.

## Input

- **`comment add`:**
  - `comment_id`: a canonical ULID string, required;
  - `body`: a string, required, which may be empty;
  - `parent_id`: a canonical ULID string, optional, and `null` means a root;
  - `created_by`: a string, optional, validated by R-7.

  A malformed ULID inside the input is `invalid_input`, as C3's amendment
  has it. `invalid_id` is for flags.
- **`conflict resolve`:**
  - `observation`: a string, required;
  - `resolutions`: an array of objects, required and non-empty. Each object
    has exactly `token` and `source`, both strings.

  The CLI adds an array-of-objects reader to `input.rs`, which C3's
  readers lack.

Schemas: `schemas/input/v1/comment_add.schema.json` and
`conflict_resolve.schema.json`. Examples:
`tests/fixtures/cli_input_v1/comment_add.json` and `conflict_resolve.json`.

- The `comment_add` example runs through the binary, as C3's
  `every_c3_example_input_runs` does.
- The `conflict_resolve` example is validated by schema only. A live token
  cannot be stored, so its run uses a token captured from `conflict show`
  in the same test.

## Cancellation (decision 14, R-21, R-22)

- **The window.** The `Boundary` impl in `mutate.rs` installs a process-wide
  Ctrl-C handler around `RepositoryService::execute` when the call's
  mutation is one of the six stoppable verbs, matched on the `Mutation`
  variant: `item sync`, `repo sync`, `item republish`, `comment add`,
  `conflict resolve` and `operation resume`. It passes that window's token
  in place of fact 40's fresh token, and removes the handler on return.
  Neither `Boundary`'s signature nor `Plan` changes (fact 61).
- **One latch per run (R-36).**
  - The press count and the cancellation persist across the execute calls
    of C2's bounded retry within one CLI run.
  - The retry loop stops once a press is latched, so no retry follows a
    Ctrl-C that landed after the last safe point.
- **Unix.** `sigaction(SIGINT)` with `SA_RESTART`. The token is held in an
  `AtomicPtr` slot.
  - The first signal calls `cancel()`, an atomic store, which is
    async-signal-safe.
  - The second restores the default disposition and re-raises, so the
    process dies by SIGINT.
  - Teardown restores the disposition, clears the slot, then drops the
    token. The handler checks for null, and a drop guard covers an unwind.
- **Windows.** `SetConsoleCtrlHandler` with a routine that cancels on the
  first `CTRL_C_EVENT` and returns `TRUE`. On the second it returns `FALSE`,
  so the default handler ends the process. The system runs this routine on
  a thread it creates. The CLI spawns none.
- **The spike first.** Task 7 begins with a spike: a Ctrl-C during a held
  transfer (`hold_transfer_after`, `hold_advertisement`). It records whether
  the result is `cancelled` at the next safe point, or `transport_unavailable`
  because a blocking call returned `EINTR`. Either is accepted with its
  effects intact (decision 14). Anything that misreports effects stops for
  the owner.
- **Prompts.** C2's consent and identity prompts run before `execute`. The
  unlock prompt runs inside `execute`, in raw mode, which clears `ISIG`, so
  0x03 there is read as the prompt's cancellation. That stays
  `unlock_required`, exit 3, as C2's amendment has it, and a test proves it
  with the handler installed.
- **After a cancellation.** The operation is final (fact 56). A same-ID
  retry returns the stored `cancelled` result. A new request completes the
  work:
  - `item sync` or `repo sync` for a synchronization, and for a comment's or
    a resolution's publication;
  - a new prepare for `item republish`, because its operation ended with its
    confirmation used.
- **Exit.** A cancelled result exits 130 under the RFC's order, ahead of
  partial effects. Its effects name what completed.
- **No retry.** C2's bounded retry never follows a cancellation (fact 40).

## Human Output

- Every verb prints C2's success and refusal lines, with `progress:` lines
  on stderr.
- `comment add` adds this note:
  `note: publishing the comment also publishes this item's other checkpointed work; unsaved drafts are not included.`
- A result with conflict data prints `conflict:` and `next:` lines.
- `remote_branch_deleted` prints two `next:` lines, because the refused
  request still holds its reservation (`01M4R19PC0T6ZPKM39X9DF26HX`):
  `next: manyhands-cli request cancel --request-id <id>`, then
  `next: manyhands-cli item republish --repo '<root>' --id <id> --prepare`.
- Roots in `next:` lines are quoted and escaped as C3's `path observe`
  escapes paths.
- `conflict show` prints the state, the next step, and each path with its
  kind, structure and token. `display_path` is printed as the library
  escaped it, without escaping it again. A side is printed only with
  `--json`, so a terminal is never repainted by content.
- An `incompatible_closures` path prints
  `note: two clones closed this ticket differently; repair it outside Manyhands.`
- `operation resume` and `operation abandon` on a local-context sync print the
  gate's `LocalContextSyncMutationDto` (R-37):
  - its `state`, `reason` and `next_action`;
  - for `original_request_required`, a `next:` line naming
    `required_request_id`.

  No local verb is named, because C5 delivers them (decision 19).

## Exit Statuses

C2's classes apply unchanged. Each code keeps its own class
(`src/results.rs:534-675`), and `failure_class` (`:282`) makes any partial
outcome 4 and any cancelled one 130:

| Exit | Class | Codes |
| --- | --- | --- |
| 0 | success or no-op | `ok`, `already_applied`, including a comment with publication pending and no remote |
| 2 | input | `invalid_input`, `usage_error`, `operation_not_found`, `occupied_path`, `item_not_found`, `request_mismatch` |
| 3 | blocked | `recovery_required`, `external_change`, `identity_required`, `original_request_required`, `remote_branch_deleted`, `publication_remote_required`, `push_rejected`, `ticket_closed`, `worktree_not_clean`, `worktree_conflicted`, `confirmation_not_found`, the host and unlock codes |
| 4 | incomplete | `discovery_pending`, and any code after a durable effect, such as a comment checkpointed with publication failing |
| 5 | transient | `busy`, `transport_unavailable` |
| 130 | cancelled | `cancelled` |
| 1 | internal | `internal_error`, output write failure |

Task 0's audit refresh lists every code each C4 verb can return. Task 13's
exit table maps each to a test, or to a reason no fixture reaches it (R-27).

## Dogfooding (R-26, decision 15)

In the final task, and only with the owner's go-ahead at that step:

1. **Preflight.** Each check stops for the owner if it fails:
   - the C4 worktree is clean and pushed;
   - local `main` equals `origin/main`;
   - the ticket's context is registered in the main checkout's index, so the
     comment does not provision a second one;
   - the selected key can be used. A protected key makes non-interactive
     mode answer `unlock_required`, so the owner runs the step at a terminal;
   - C4 adds no migration of the shared application database, or main's CLI
     still opens it after the branch-built CLI has. Another session uses
     main's CLI against the same registry;
   - every edit in the C4 worktree is committed. That worktree is the
     ticket's context, and `comment add` checkpoints into it, as AGENTS.md
     requires before `ticket save`.

   Record whether a publication remote is configured.
2. **A verification comment through the CLI.** The branch-built CLI adds a
   short verification comment on the C4 ticket:
   - `comment add --repo /home/vhodges/work/src/manyhands --id 01M4CC0VMWEQQ9G4W0DAG7ADPY --input -`;
   - `created_by` is `Claude Opus 5.5 <noreply@anthropic.com>`;
   - the request ID is recorded in the ledger before it is sent.

   With a publication remote, the comment's publication merges local
   primary into the ticket context and pushes the ticket branch. The
   go-ahead covers that. A refusal or partial result stops for the owner,
   and is resumed only under the recorded request ID.
3. **Check by value.** `comment list --id 01M4CC0VMWEQQ9G4W0DAG7ADPY` shows
   the comment, with its ID, parent and `created_by`.
4. **The switch.** The files of fact 48 direct comments to `comment add`,
   with agents passing their noreply `created_by`. AGENTS.md states the
   extended exception explicitly (decision 15): `working-on-a-ticket` and
   `scripts/ticket-loop` keep canonical comment files, with the noreply
   `created_by`, until that skill is revised. The switch has its own
   independent review.
5. **Review-ready.** After the documents are marked review-ready and
   `merge-tree` is clean, the review-ready comment is also added through the
   CLI, the same way.

## Testing

- **Library:**
  - each outcome table as a test first, in the task that owns it, written as
    `c4_` cases in `tests/mutation_remote/` (R-31);
  - conformance rows for the three new commands, run by `mutation_bridges`,
    `mutation_remote` and `mutation_replay`;
  - goldens for every new DTO, preview and refusal;
  - privacy: no body, source, token or path in a diagnostic, a request
    store or a `Debug` rendering;
  - hostile conflict paths, as `hostile_conflict_paths_are_redacted` does,
    in a new case for the new read.
- **CLI:**
  - each verb in JSON and human mode;
  - input schemas and examples;
  - help goldens (E-1);
  - exits;
  - a `not_confirmable` test for the five unconfirmed mutations, and a usage
    error for `conflict show --prepare`;
  - Ctrl-C through the pseudo-terminal against a held fixture, in
    `cli_remote` (Task 13);
  - lost stdout, with a closed pipe.
- **Two clones (R-24, R-33):**
  - `CliPair` seeds the ticket on primary before both clones are made, so
    both index it. Each clone makes its own context with a first write
    before it synchronizes the item;
  - every setup step goes through the binary;
  - each case stays under the 100-second watchdog, and the PTY's 10-second
    deadlines bound any hold (fact 43).
- **Second actor (track rule 4):** a raw Git peer pushes divergent history
  and deletes a branch.
- **An unfinished local sync (R-38):** the library rows that fact 80 adds
  are seeded through the gate's own `context sync-local` mutation, left
  stopped in a conflict. They run in `tests/mutation_remote/`, which mounts
  the library as a module (fact 62). No CLI journey covers them, because no
  C4 verb starts a local sync.
- **Shared code:** the tasks that edit `remote/` also run `--lib remote`,
  `--lib local_sync` and `--lib integration`, because the gate's local
  executor calls into the same modules.
- **Redaction:** every `cli_remote` case runs through `scan_output`.
- **Platforms:** the PTY cases are `#[cfg(unix)]` (fact 43). The Windows
  console handler is a native manual check, owed after merge and recorded in
  the ledger and the Wave.

### Mutating-Cycle evidence by verb (track rule 7)

"N/A" gives the reason. Numbers are plan tasks.

| Verb | Progress | Consent | Ctrl-C | `request cancel` | Redaction | Stale write | Replay | `request_mismatch` | Completion of a stopped request |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `comment add` | 8, 12a | N/A: unconfirmed | 13 | 13 | 12a | N/A: a new comment has no observation; `occupied_path` in 8 | 8 | 2b | 2b: a refused push, then the same request |
| `item sync` | 12a | N/A: unconfirmed | 13 | 13 | 12a | 1: `external_change` | 13 | 12a | 1: identity |
| `repo sync` | 12a | N/A: unconfirmed | 13 | 13 | 12a | 1: `external_change` | 13 | 12a | 1: identity |
| `item republish` | 12b | 12b: prepare, confirm, interactive | 13 | 13 | 12b | 5, 12b | 12b | 12b | 12b: a transport stop, then the same request. A cancelled republish is final (fact 56) |
| `conflict resolve` | 12b | N/A: unconfirmed | 13 | N/A: no operation of its own (R-35) | 12b | 4a, 12b | 4a | 4a | 4a |
| `operation resume` | 12a | N/A: unconfirmed | 13 | N/A: fact 11 | 12a | N/A: no observation | 7 | 7 | N/A: resume is itself the completion |
| `operation abandon` | 7 | 7 | N/A: no safe point; Ctrl-C at its prompt in 7 | N/A: no safe point | 7 | 7: `external_change` | 7 | 7 | N/A: no safe point |
| `conflict show` | N/A: a read | N/A | N/A | N/A | 9 | N/A | N/A | N/A | N/A |

## Named Edits To Pre-Existing Tests

| Edit | Task | File and change |
| --- | --- | --- |
| E-1 | 7–10 | `tests/fixtures/cli_v1/help_root.json` and `help_root_text.txt` are regenerated as each C4 verb becomes available, and for `item republish`'s new row. The diff may change only the verb's availability, repository rule and options, plus the one added row. |
| E-2 | 10 | `src/bin/manyhands-cli/catalog_tests.rs`: `DESIGN`'s C4 row (`:81-92`) gains the string `"item republish"`. |
| E-3 | 1 | `tests/mutation_remote.rs`: the binding-level tests that assert a refusal to merge are rewritten. The synchronization tests go to the divergent-synchronization table's outcomes: `a_divergent_remote_is_merge_required_and_holds_its_reservation` (`:1324`), `a_lost_push_the_remote_has_since_built_on_is_merge_required_and_leaves_nothing_of_a_merge` (`:2697`) and `merge_required_is_not_finished_by_the_same_request_after_the_branch_is_rebased` (`:2754`). The resume tests go to the resume table's, because `resumed` shares the binding's settings (`bind/sync.rs:516-519`): `a_resume_never_merges_and_leaves_nothing_of_a_merge` (`:3614`) and `a_resume_does_not_continue_a_merge_made_some_other_way` (`:3654`). Tests that call the domain with their own `REFUSING` settings (`:365`), such as `the_sibling_refuses_divergence_and_the_existing_function_merges_the_same_fixture` (`:493`) and `the_sibling_does_not_continue_a_merge_it_is_set_to_refuse` (`:590`), test the domain setting and stay unchanged. Task 0 classifies each. |
| E-4 | 1 | `tests/fixtures/mutation_v1/repo_sync_merge_required.json` is replaced by `repo_sync_recovery_required.json`, the conflict-pending golden, and its registration (`tests/support/mutation_golden.rs:143`) is renamed. |
| E-5 | 1, 5 | `src/repository/mutation/outcome_tests.rs`: `every_synchronization_error_has_its_code` (`:705`, table `:646-673`) for the conflict rows; and `an_external_change_to_a_confirmed_command_is_offered_a_new_preview_or_an_inspection` (`:1536`), because `item republish` is the first confirmed command that stops at safe points. The tests at `:1565` and `:1633` stay unchanged: `never_finished_by_the_same_request` does not change. The loops over `Command::ALL` take the new commands unchanged. |
| E-6 | 2b, 4a, 5 | `src/repository/mutation/command.rs`'s tests (`:491-698`) gain the three commands, as the Command Predicates table gives them. `only_a_synchronization_stops_at_safe_points_and_none_needs_confirmation` (`:537`) already treats the gate's two local commands apart: `LocalContextResolve` stops with no operation, and both report `Observing`. It keeps that. It gains `comment add` and `item republish` as stopping commands with an operation: `comment add` reports `Writing`, and `item republish` reports `Connecting` and is the only stopping command that needs confirmation. `ConflictResolve` is checked as not stopping. The test is renamed `the_stopping_commands_and_their_stages_and_confirmation`, because a stopping command can now need confirmation. `only_a_command_whose_work_no_journal_records_has_no_operation` (`:614`) gains `ConflictResolve`; its `names_another_s` list already includes `LocalContextResolve` (`:625-628`). |
| E-7 | 1–5 | `tests/mutation_contract.rs`: `DATA_ROOTS` (`:1510`), `CODES_NO_MUTATION_GOLDEN_HAS` (`:1717`) and `CODES_NO_FIXTURE_CAN_PRODUCE` (`:1678`) gain the new entries, with `merge_required`'s reason. `UNBOUND_COMMANDS` (`:1455`) is unchanged (fact 49). |
| E-8 | 1–5 | `tests/read_contract.rs`: `contract_enumerations` (`:704`, extended at `:1009-1036`) gains the new enums, so `published_enumerations_list_exactly_the_registered_values` (`:1052`) counts them. |
| E-9 | 2b, 4a, 5 | `tests/support/conformance.rs`'s `HOMES` (`:59`) and each home's table gain rows for the three commands. `tests/support/confirmation.rs`'s `preview_schema` (`:48-61`) gains `item republish`. |
| E-10 | 1, 2b, 4a | `tests/support/mutation.rs`'s data accessors (`:596-794`) and `tests/support/repair.rs:333` gain arms for the new `MutationDataDto` variants. |
| E-11 | 4b | The resume tests that assert a comment's synchronization is left to inspect, in `tests/mutation_replay/resume.rs` (`a_resume_of_a_comment_s_synchronization_is_not_taken_up_and_is_not_offered`, `:1090`) and `src/repository/mutation/bind/` unit tests, change to publication (R-20). Task 0 finds them with the plan's grep. |
| E-12 | 4a | `tests/repository_enablement.rs`: `registry_creates_repository_and_discovery_metadata_schema`'s exact table list (`:581-624`) gains the gate's eight `local_context_sync_*` tables, which it lacks on `a372edb` (fact 81, Cycle decision 21), and `remote_resolution_owners` (R-41), each in sorted order. The test then passes. If main has fixed the gate's part before Task 4a, E-12 adds only `remote_resolution_owners`. |

No other pre-existing test changes. `tests/support/pty.rs` gains an additive
helper and changes no existing function. The new table's name avoids
`request`, so `tests/mutation_replay.rs:400` needs no edit (fact 81, R-41).
It holds no foreign key into the tables that `state_tests.rs` drops, so
those tests need none either (fact 82). Task 4a's runs of `--test
mutation_replay` and `--lib remote` prove both.

Task 0 proves the list with the plan's greps, re-run on the main it
starts from. Those greps were re-run on `a372edb` by the 2026-10-27
re-grounding. They found no gate test that asserts a refusal to merge, pins
the C4 verbs, or pins help. A hit outside this table stops for an owner
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
- `conflict show`'s per-path `kind` and `structure`, and set-wide
  resolution (decision 12);
- after an outside repair, `conflict show` reports it and `operation resume`
  continues, in place of "`operation show` re-observes";
- the `comment add` example, using real codes in place of
  `publication_pending`, and `created_by`;
- republish through `item republish` (decisions 7 and 11), refused for a
  lifecycle-closed ticket (decision 17);
- conflict data in place of a recovery action (decision 8);
- Ctrl-C's window, a cancellation's finality and lost stdout (decision 14,
  R-21, R-22);
- the C4 rows of the input-schema table, and the evidence paragraph;
- **the gate's paragraph (fact 85, decisions 19 and 20).** Its
  "Library-boundary amendment, 2026-10-27" (`cli-contract.md:753-781`) is
  amended in place:
  - `conflict show` and `conflict resolve` are delivered by C4 for remote
    synchronizations. They are no longer a "future CLI adapter contract";
  - "C4/D5 must adapt this contract" becomes C5/D5 for the local verbs.
    C4 delivers the separate remote-conflict wire bindings;
  - the sentences on resending the same R request and source, on
    cancellation preserving Accepted requests, and on abandonment proof are
    scoped to the local operation. A remote attempt that has only written
    paths may be superseded (decision 18), and a remote cancellation is final
    (fact 56);
  - C4's remote departures from the local shapes are listed: inline sides
    with `historical` (R-10, R-39), the lease (R-12), identity reported by
    code and `repo.identity_set` rather than `set_identity` (The Next Step),
    and an external-only set answered `recovery_required` rather than
    `external_change` (Conflict Resolution, step 4).

**Git workflow RFC:**

- the bindings merge (decision 3);
- a conflict's next step is in data;
- republish is its own confirmed command, pushing exactly the previewed
  commit, and the next item sync integrates primary (decision 11);
- the gate's "Local recovery amendment" is scoped to the local operation,
  with the remote rule beside it: a new request may supersede a remote
  attempt that has only written paths, while those paths are unchanged
  (decisions 18 and 20). Its resume rule, naming R through
  `original_request_required`, applies to remote too (Resume);
- an incompatible-closure conflict's limit on remote: it is external-only,
  and an outside repair that chooses one closure is not promised to be
  accepted (fact 85). Abandonment stays on `01M4H33R34Z7C7EEKTY1ZCT950`
  (decision 6).

**Canonical content and comment schema RFC:** `created_by` comes from the
caller's validated input when given, else the Git identity (decision 9).

**F2 design:** a dated departure under "Departures from the RFCs". The
Comment Author rule "A caller cannot supply it" is replaced by decision 9.

**Wave 03:**

- the C4 section, "Amended 2026-10-26 by C4", records decisions 1–21, the
  delivery and the known limits;
- the track-rule-5 exception (decision 1);
- the D4 section records that D4 waits for C4 (decision 5). The D5 section
  records that D5 consumes C4's conflict read and resolution through D4;
- the C5 section records that C5 exposes the gate's local-sync verbs, and
  any local conflict inspection they need (decision 19);
- the gate's "Final gate checkpoint, 2026-10-27" paragraph: its "C4/D5 must
  adapt the local boundary" becomes C5/D5. It keeps "C4/D5 … still provide
  the separate remote-conflict wire boundary", and records that C4 delivered
  that (decision 19);
- the Capability Ownership rows:
  - "Comment author (`created_by`)" (decision 9);
  - "Conflict inspection read": the gate's rewritten row is kept. Only its
    remote half is updated, to "remote boundary: C4", and its local half is
    left as the gate wrote it;
  - "Discussion/sync/conflict and resume", whose bindings C4 adds;
- the entry-gate item 3 departure (decision 16);
- the Windows Ctrl-C manual check, owed after merge.

**The gate's shapes.** The departures listed under the CLI contract RFC
above are recorded in the RFC itself, and also in the design's rulings:
- R-9: the token inputs;
- R-10 and R-39: the sides;
- R-12: the lease;
- The Next Step: `set_identity`.

The gate's own Cycle, design, implementation and execution documents are
not edited. Its amendments to C4's authorities are edited as listed here.

**API audit:** the C4 refresh section (Task 0), and the C4 rows updated in
place. The gate's section "closure-recovery-library-amendment-2026-10-27"
gains a note naming C4's remote departures, and it is not otherwise
changed.

**AGENTS.md and the skills of fact 48:** the switch and the extended
exception (decision 15), in Task 15 after the verification comment
succeeds.

**The C4 ticket:** its planning boxes are ticked at plan approval, through
`ticket save` with the owner's go-ahead then. That is planning-session work,
not a plan task.

## Rulings

Each ruling is listed with its cost if wrong.

- **R-1 Catalog.** Literal `Available("C4")` entries, flipped in the task
  that dispatches each verb, plus the new `item republish` entry. *Cost:*
  none.
- **R-2 Kind lookup.** `--id` verbs resolve the kind with one `show_item`
  read before the request is built. Its failure passes through as C3's fill
  read does. *Cost:* one extra read. A failed read can mask a stored result
  on retry, as C3's limit describes; `request show` still has it.
- **R-3 Synchronizer-driven submission.**
  - A crate-private submission and retry take a synchronizer closure built
    from `call.session.synchronize` with the merging settings and a
    `Watch`.
  - `Watch` gains a `pub(super)` constructor that takes an `Accepted`
    carrying the child `synchronization_id` (fact 54). Its owner is the
    `comment add` request.
  - The checkpoint goes through a crate-private
    `checkpoint_comment_locally_with`, which takes the author and whether to
    release a binding that wrote nothing. The existing
    `checkpoint_comment_locally` delegates to it with neither, so its
    behavior and fact 73's callers are unchanged.
  - Re-entry calls the submission with its full input and `restart: true`
    (fact 65).
  - The public functions keep their behavior, and call it with the
    unobserved synchronizer.

  *Cost:* internal entry points shared by two callers.
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
- **R-6 Comment codes.** As the comment-submission table. With no remote,
  the code is `ok` with publication pending, as the RFC's exit-0 rule says.
  *Cost:* a script must read `effects.publication`.
- **R-7 `created_by` validation.**
  - The form is `Name <email>`, at most 256 bytes.
  - The name and email each meet `ValidIdentity::of`'s rules (fact 57), and
    the name holds no `<` or `>`.
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
    observation digest, the path's position in the inspection's `paths`, the
    path bytes, and the three sides' OIDs and modes.
  - **Their encoding:** each field is domain-separated and length-delimited,
    as the gate's tokens are.
  - **Their life:** both are re-derived from the live conflict on every
    call and never stored. They are valid only while that conflict is
    unchanged.
  - **The departure from the gate:** the gate's token hashes the parent
    operation and a snapshot generation. This one hashes the observation
    digest and the path's position, which bind the same operation, window
    and conflict.

  *Cost:* a token from before any change is refused, which is the intended
  staleness rule.
- **R-10 Sides inline.** `conflict show` returns sides in its data, and only
  for paths the library can read as UTF-8 canonical text. The gate reads
  local sides separately (fact 53), and this departure is recorded in the CLI
  RFC. *Cost:* large conflicts make a large response. Items are already
  bounded by their limits.
- **R-11 Display paths.** C4's conflict read escapes each path byte outside
  printable ASCII as `\xNN`, and doubles a backslash, which is the rule the
  gate's `display_path` applies (fact 83). That function is private to a gate
  file C4 does not edit, so C4 writes its own copy of the rule. Task 3's test
  pins it byte for byte on a control byte, a backslash, a non-ASCII UTF-8
  character and an invalid byte. A display path is never used as a writable
  path. *Cost:* two copies of one rule. A later change to the gate's copy
  is not caught by C4's test.
- **R-12 `conflict show` is a read.**
  - It takes no request ID and writes nothing.
  - It takes the repository lease while it reads Git, as fact 29's
    inspection does, and a test proves the database unchanged.
  - A held lease answers `busy`, exit 5.
  - This departs from the gate's local reads, which "do not take the write
    lease" (fact 85). The departure is recorded in the CLI RFC.

  *Cost:* a read can answer `busy`.
- **R-13 States.** As the conflict-states table. *Cost:* a new state later
  is a new enum value.
- **R-14 The attempt and its durable state (decision 13).**
  - **The attempt ID.** It is a ULID made from a domain-separated BLAKE3
    digest of the request ID, the synchronization's ID, the integration window
    and the stage ordinal. So a reused request ID never meets another
    conflict's attempt. `validate_child_id` refuses an ID equal to an
    operation, confirmation, attempt or local-sync ULID (fact 72), and a
    digest-derived ID is not expected to meet one. If it ever does, the
    refusal is reported as the library maps it, and Task 4a asserts that
    nothing was written.
  - **What the library keeps.** It finishes an attempt at
    `ResolutionPrepared`, `CommitPrepared` or `Applied` from its durable rows
    (fact 34), without storing a body:
    - at `ResolutionPrepared`, with the presented input and the live
      inspection, because no candidate exists yet;
    - at `CommitPrepared` or `Applied`, from durable state alone, which also
      serves `operation resume` (decision 18).
  - **Rebuilt tokens.** The library rebuilds its tokens only for
    `reconcile_resolution_candidate`, which checks window, ordinal and
    operation (`sync.rs:4226-4231`). Each path's bytes come from hashing
    every path of the candidate tree and matching `path_digest` (fact 70).
    A path resolved to its local side is found too. The binding compares the
    presented input with the digest stored with the association, never with
    a rebuilt observation (fact 31's configuration digest is not stored).
  - **The contract.** The interrupted-resolution table is the contract, and
    the binding replays first.

  *Cost:* the largest library change in C4. If a row cannot be met, Task 4a
  stops.
- **R-15 Continuation.** `conflict resolve` publishes under its own request,
  with progress and cancellation. *Cost:* one command does two phases. Its
  effects distinguish them.
- **R-16 Merging settings.** `refuse_divergence: false` in both bindings.
  `merge_required` becomes unreachable from them. *Cost:* the E-3 to E-5
  edits.
- **R-17 Identity during a merge.** No workaround. `repo identity-set` is
  not refused by the stopped merge (fact 28), and the same request restarts
  and merges. *Cost:* none.
- **R-18 Republish (decisions 11 and 17).**
  - A push-only domain path, reached through a new crate-private
    `Session::republish(service, request, consent, observer)`. It fetches,
    proves the absence, refuses an advertised context or a moved local tip
    as `external_change`, then pushes exactly `local_oid`.
  - It does not use the clean plan (fact 69). Its push leaves out the
    push-absence boundary (`sync.rs:6209-6213`) and the per-record refusal
    (`reservation.rs:2742-2759`), which the consent replaces.
  - The preview uses a new reservation-free read of the same evidence, and
    accepts only a `Deleted` classification.
  - Resume answers `original_request_required` for an operation a republish
    holds (fact 78).
  - A lifecycle-closed ticket is `ticket_closed`.

  *Cost:* a second push path to keep consistent with item sync's evidence.
  The table's tests cover each case.
- **R-19 Republish preview.** `ItemRepublishPreviewDto`, the effect
  `branch_republished` and one desktop arm. *Cost:* a small cross-track
  merge with D3 (decision 10).
- **R-20 Resume routes.** As the resume table. A remote attempt that has
  only written paths answers `original_request_required`, naming its
  request, as the gate's local resume does. *Cost:* resume can now publish a
  comment. That is what the RFC's example shows.
- **R-21 The Ctrl-C window.** Installed by the `Boundary` impl around
  `execute` of the six stoppable verbs, matched on the `Mutation` variant,
  with `SA_RESTART`. The first Ctrl-C cancels, and a second ends the
  process. *Cost:* a Ctrl-C between execute calls of C2's bounded retry,
  outside a window, ends the process at once. That is interruption, as
  before C4.
- **R-22 Cancellation's exit.** 130 even after a checkpoint, as the RFC's
  order puts cancellation first. *Cost:* a script must read effects after
  130.
- **R-23 Retry.** C2's bounded retry is unchanged.
  - It re-runs a partial whose first action is `request.retry` for the same
    request, including `push_rejected` and a comment's conflict (fact 66).
  - A re-run is the same request, so it duplicates nothing. It is not new
    lifecycle intent, which the Wave's exclusion is about (C5's promotion
    and closure).
  - It never follows `cancelled`, a blocked result, or a latched Ctrl-C
    (R-36).
  - The sync, republish and comment verbs run with `Plan { retry: true }`.

  *Cost:* up to three extra push attempts on a refused push.
- **R-24 Two-clone support.** `tests/support/cli_pair.rs`, new, built on
  `SshRemoteFixture` and `CliEnv`, mounted only from `tests/cli_remote.rs`.
  Cases are split to stay under the watchdog. *Cost:* a new support module.
- **R-25 Lost stdout.** Proven with a closed stdout pipe on `item sync` and
  `comment add`. The process exits 1, then `request show` and a replay
  follow. *Cost:* none.
- **R-26 Dogfooding.** As the Dogfooding section, with the owner's go-ahead
  at that step. *Cost:* two comment paths coexist until
  `working-on-a-ticket` is revised (decision 15).
- **R-27 Exit accounting.** As C3's decision 6: every exit reached through
  the binary, and the unreachable codes listed with reasons. Set equality
  with Task 0's audit union. *Cost:* none.
- **R-28 Golden names.**
  - **CLI goldens:** `<resource>_<verb>_ok`, at least one
    `<resource>_<verb>_<code>`, and `<resource>_<verb>_text`;
  - **mutation goldens:** `<resource>_<verb>` for success, as the existing
    set has it (`operation_resume`), and `<resource>_<verb>_<code>`;
  - **previews:** `item_republish_preview`, `item_republish_preview_noop` and
    `operation_abandon_preview`;
  - **read goldens:** `conflict_show_<state>`.

  *Cost:* none.
- **R-29 No new test target.** New tests are modules of existing targets,
  which are already in the native list (fact 52). *Cost:* none.
- **R-30 C3's stuck save.** `operation abandon` is shown closing C3's
  interrupted-and-edited save (C3's known limit). Its file stays as it is.
  *Cost:* none.
- **R-31 SSH case names.** Every new case in a `harness = false` target is
  named `c4_<module>_<case>`, registered in the target's `CASES`, and run
  with `-- c4_<module>`. Each verification step checks the number of cases
  run. *Cost:* none.
- **R-32 Predicates.** As the Command Predicates table. *Cost:*
  `request_mismatch` on `conflict resolve` is always exit 2.
- **R-33 Fixture premise and gate alignment.** `CliPair` seeds the item on
  primary before the clones are made. A well-formed foreign token is
  `external_change`, as the gate has it. *Cost:* none.
- **R-34 Comment standing.** As the Comment Submission section's Standing:
  - a binding that wrote nothing is released in one transaction with the
    row's rejection, and the record stays, so the same request binds afresh
    (fact 68);
  - a written file keeps the binding;
  - a pending publication is `Owed`, except a final child or a code never
    finished by the same request.

  *Cost:* releasing a binding is a new library path. The comment table's
  identity rows test it.
- **R-35 Resolution association.**
  - **No record to replay.** `conflict resolve` holds no operation of its
    own, so settlement deletes its record on any stop (fact 67). The durable
    state is the resolution attempt.
  - **A new table.** The attempt's association lives in a new table,
    `remote_resolution_owners` (R-41), with the attempt ID, the request ID
    and the presented input's digest, as the gate keeps its own in a new
    table. No audited table changes, so main's binaries still open the
    registry (fact 75).
  - **Who may continue.** A retry under that request ID finds and continues
    the attempt. Any other request is answered by step 3's rules (decision
    18).
  - **Reporting.** `ConflictDto.request_id` names the request for
    `retry_resolution`.
  - **Its `Watch`.** The continuation's owner is the resolve request itself.
  - **Replay.** A replayed finished resolve offers no recovery action, since
    `finishes_as_a_synchronization` is false. `conflict show` names the next
    step.

  *Cost:* a new table, and two ways out (decision 18). An ID or a digest is
  not a body.
- **R-36 One Ctrl-C latch per run.** The window's press count and
  cancellation persist across C2's bounded retry within one CLI run, and the
  retry loop stops once a press is latched. `mutate::run` resets the latch
  on entry. Every test that raises a signal or sets the latch runs in a
  child process, so no in-process test, `mutate_tests` included, sees it set.
  *Cost:* none.
- **R-37 Local-sync operations in C4's verbs (decision 19).**
  - `conflict show` and `conflict resolve` look an operation ID up in the
    gate's local-sync records before the remote ones, and answer
    `invalid_input` for a local sync.
  - `operation resume` and `operation abandon` pass a local sync to the
    gate's routing and abandonment, unchanged (facts 9 and 10).
  - The CLI's human output renders `LocalContextSyncMutationDto` (Human
    Output). Its JSON passes through unchanged.
  - Task 9's human-output tests render the gate's goldens
    `tests/fixtures/mutation_v1/local_sync_*.json`, because no C4 verb can
    start a local sync.

  *Cost:* a local conflict has no CLI inspection until C5.
- **R-38 An unfinished local sync (fact 80).** Every C4 verb that reserves,
  begins a comment or re-acquires a conflict answers the library's
  `recovery_required` with `[repo.inspect]`, and writes nothing. The four
  tables carry that row, and the library tests seed it through the gate's
  own `context sync-local` mutation, as `ConflictWorld::with_incoming` does
  (`tests/mutation_replay/local_sync.rs:164-212`). *Cost:* none; C4 adds no
  check of its own.
- **R-39 `historical`.** `ConflictDto.historical` is true when an attempt has
  written paths, which is the `resolution_prepared` state with paths. The
  sides are then the recorded conflict's, and the worktree may already hold
  results. This matches the gate's local content shape (fact 53) at
  `conflict show`'s level. *Cost:* one boolean.
- **R-40 Incompatible closures.** `structure` gains `incompatible_closures`
  (fact 30). The set is external-only, `next_action` is `external_repair`,
  and human output says why. `conflict resolve` answers `recovery_required`
  with `next_action: external_repair` (step 4). *Cost:* a remote closure race
  has no in-product way out (Known Limits).
- **R-41 The association table's name and shape.** It is named
  `remote_resolution_owners`, not `…_requests`. Two things follow from the
  name and shape:
  - `tests/mutation_replay.rs:442` asserts that no object surviving the drop
    of `REQUEST_TABLES` has `request` in its name (fact 81);
  - it holds the attempt and request IDs as values, with no foreign key, so
    `state_tests.rs`'s drop-and-recreate tests are unaffected (fact 82).

  It is created by its own migration step, so a registry made before C4
  gains it on open. E-12 adds it to the enablement test's list. *Cost:* the
  attempt row's lifecycle is enforced by the binding's transactions, not by
  SQLite. Task 4a's rows test it.

## Known Limits

- A pending remote conflict cannot be abandoned (decision 6). It blocks
  other synchronizations and authoring in that context until it is resolved
  or repaired.
- A conflict with any noncanonical or structural path is repaired outside as
  a whole. Its canonical paths are not resolved in-app (decision 12).
- A remote conflict in which two clones closed one ticket differently is
  external-only (R-40). The Git RFC does not promise that an outside repair
  choosing one closure is accepted, and the operation cannot be abandoned
  (decision 6). Such a conflict may have no way out until
  `01M4H33R34Z7C7EEKTY1ZCT950` delivers abandonment.
- While a local-context sync is unfinished anywhere in the repository, every
  C4 verb that reserves, begins a comment or re-acquires a conflict answers
  `recovery_required` (R-38). No C4 verb can start or finish a local sync
  (decision 19), so before C5 this happens only to a sync the library or a
  later build started.
- `conflict show` does not inspect a local-context sync (decision 19).
- A cancelled operation is final. A new request completes the work
  (fact 56).
- A Ctrl-C during a blocked network call may answer `transport_unavailable`
  instead of `cancelled`, with effects intact (decision 14), as Task 7's
  spike records.
- A later request that begins with an unsettled push and is cancelled
  answers `recovery_required`: the publication is not confirmed, and a new
  request verifies it (fact 74).
- A merge aborted outside leaves the synchronization unresolvable in-app.
  The merge must be redone and repaired outside, and it cannot be abandoned
  (`01M4H33R34Z7C7EEKTY1ZCT950`). Once a resolution attempt exists, outside
  repair is refused (fact 76). So an aborted merge with an unfinished
  attempt whose written paths were also changed has no in-product way out.
- An unfinished resolution whose written paths were edited by hand can be
  continued only by its own request (decision 18's supersede rule needs the
  written paths unchanged). Undoing the hand edits restores the supersede
  route.
- `request cancel` does not reach `conflict resolve`, which holds no
  operation (R-35). Ctrl-C does, and so does `request cancel` on the
  original synchronization request.
- The republish push cannot be guarded as a create (fact 77). A branch
  recreated at an ancestor between the fetch and the push is fast-forwarded,
  and reported as republished.
- A refused `item sync` holds its reservation until that request is
  cancelled, so `item republish` first needs `request cancel`
  (`01M4R19PC0T6ZPKM39X9DF26HX`).
- `request cancel` does not reach a resume. Ctrl-C does.
- A wrong passphrase reports `transport_unavailable`
  (`01M5E6DYJ6GN7DMW4424TH3V78`).
- An unusable identity is refused late (`01M5MK57JKCSQFXBNZ8DBE548K`). The
  divergent-synchronization table's identity rows cover the missing case
  only.
- A blocked synchronization holds its reservation until its request is
  cancelled (`01M4R19PC0T6ZPKM39X9DF26HX`).
- Operation reads name no item for comment rows
  (`01M4W5SCBV6WFJP11Y7PJF5VS8`).
- An interrupted save whose row recorded no step is closed as `rejected`
  (`01M4PB7YZMBFAFFM94AZY08HWS`).
- `created_by` is not authenticated (R-7).
- A string token is valid only while its conflict is unchanged (R-9).
- The Windows Ctrl-C handler is first compiled and checked natively, after
  merge.

## Alternatives Rejected

- **A separate library ticket (track rule 5).** The owner chose to carry the
  library part in C4 (decision 1).
- **A `conflict.show` recovery action.** It would edit the desktop's
  exhaustive match (decision 8).
- **`item sync --republish`.** Confirmation is fixed per command (fact 12,
  decision 7).
- **Integrating primary inside a republish.** The pushed commit would differ
  from the preview (decision 11).
- **Per-path eligibility.** A Wave 02 merge-model change (decision 12).
- **Masking SIGINT and checking it at safe points.** A Ctrl-C would wait
  silently, and could not force a quit (decision 14).
- **Serializing the library's tokens.** Their fields are private by design.
  String tokens are re-derived (R-9).
- **A separate content read, as the gate has.** One `conflict show` call
  gives a CLI user everything to write a resolution (R-10).
- **Local-sync IDs in `conflict show` and `conflict resolve`.** C5 owns the
  local verbs (decision 19).
- **No supersede, as the gate's local resolution has.** A remote attempt with
  only written paths would then have no way out once its request is lost
  (decisions 18 and 20).
- **An identity prompt in sync.** C3 has none for saves, and the RFC routes
  identity through `repo identity-set`.

## Risks

| Risk | Control |
| --- | --- |
| The gate changed the inventories and records C4 extends | Done: the gate merged at `a372edb`, and the 2026-10-27 re-grounding re-read every fact and named edit there. Task 0 re-greps E-1 to E-12 if main moves again |
| Main is red: `registry_creates_repository_and_discovery_metadata_schema` lacks the gate's tables (fact 81) | C4's E-12 adds them (Cycle decision 21). Task 0's full suite records that one failure as expected, and Task 4a's run proves it fixed |
| C4's `remote/` edits break the gate's local executor, which shares those modules | Tasks that edit `remote/` also run `--lib local_sync` and `--lib integration` (Testing) |
| Finishing an interrupted resolution needs more library change than planned | Decision 13's table, written first, with a stop |
| Merging changes the premise of many F2 tests at once | E-3 to E-5 list them; Task 0 classifies each before code |
| A Ctrl-C handler interrupts blocking I/O | Task 7's spike; decision 14 bounds the acceptable answers |
| A Ctrl-C handler leaks past `execute` | The window is scoped, and a unit test proves the disposition restored |
| The Windows handler is not compiled before merge | Recorded as an owed native check; native CI runs after merge |
| Two-clone CLI journeys exceed the watchdog | Cases are split (R-24) |
| The dogfooding comment publishes local main into the ticket branch | The preflight stops unless local main equals `origin/main`, and the go-ahead is at that step |
| The branch-built CLI migrates the shared registry that main's CLI uses | The preflight stops unless main's CLI still opens it |
| D3 and C4 both edit `src/desktop/view/confirm.rs` | One arm; whichever merges second resolves it (decision 10) |
