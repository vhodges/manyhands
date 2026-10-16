---
title: "Wave 02 Cycle 09 Confirmed Managed-Document Promotion Execution Ledger"
date: 2026-10-15
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M500Q0ZEMW43YTJBVHFBH3X6"
---

# Confirmed Managed-Document Promotion Execution Ledger

## Planning Preflight — 2026-10-15

- **Worktree.** Reused the registered ticket worktree
  `.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH` on
  `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Status was clean.
- **Base.** `git fetch origin` observed `origin/main` at `89d04f9` (docs:
  future-ideas notebook). This follows the Cycle 08 merge `df73c54`, and
  Wave 03 F1 and F2 are present.
- **Rebase.** The single ticket checkpoint rebased without conflict from
  `f017c73` to `69176ba`. The published ticket ref still holds `f017c73`, so a
  later push must reconcile it. No force push is authorized.
- **Main checkout.** Its untracked `.superpowers/` was left untouched.

## Grounding

The Cycle and design are built on these source facts, each observed at
`89d04f9`:

| Fact | Location |
| --- | --- |
| `promote` already in the remote action `CHECK`, enum, parser and DTO; no executor | `src/repository/remote/state.rs:637,1499,2487,5075`; `read/dto.rs` |
| Promotion classified not resumable at the F2 boundary and in status text | `mutation/bind/resume.rs:177,289`; `read/status.rs:295`; `remote/state.rs:2898,2937` |
| No-remote synchronization reports `PublishPending` without local integration | `remote/sync.rs:6091-6104` |
| Non-writing in-memory merge | `remote/sync.rs:702` `prepare_clean_merge` |
| Guarded fast-forward with safe checkout | `remote/sync.rs:7332` `fast_forward` |
| Only exact fetch/push mappings admitted; no delete; no `push_negotiation` | `transport/remote.rs:173,473,696,720` |
| No worktree prune, context branch deletion or remote ref deletion anywhere | repository-wide search |
| Pending local journal row blocks remote reservations | `recovery.rs:633` `require_no_pending_local` |
| Parent/child binding precedent | `comment_publication_bindings`, `recovery.rs:47` |
| Edit refuses a leftover branch whose tip differs from the primary | `repository.rs:1764-1780` |
| Batch marks absent `observed_published` rows `remotely_deleted` | `remote/state.rs` observation batch |
| Fixture receipt proof cannot prove a deletion | `tests/support/ssh_server.rs:692` |
| Digest precedent uses BLAKE3 | `remote/merge.rs:219` `ExpectedConfiguration` |
| Documents have no lifecycle front matter | `canonical.rs:91`; canonical schema RFC |

## Decision Audit

| Area | Settled by | Open → decision |
| --- | --- | --- |
| Confirmation before the first step | Git RFC, MVP RFC, PRD COLLAB-007 | Domain digest form, internal ruling |
| Final save/checkpoint | PRD "when needed"; CLI RFC "save drafts first" | **D1** |
| Synchronize before integration when a remote exists | Git RFC, PRD | — |
| Unpublished local primary commits | — | **D2** |
| Non-fast-forward merge | MVP RFC | Subject `Promote document <ULID>`, internal ruling |
| Primary conflict without a remote | — | **D3** |
| New remote observation invalidates consent | Desktop RFC, Git RFC Wave 03 boundary | When synchronization brings new content, **D4** |
| Remote branch deleted before promotion | Git RFC republish rule | **D5** |
| Publish before cleanup; remote before local; remote OID precondition | Git RFC | — |
| No remote: local cleanup proceeds, publish pending | Git RFC, MVP RFC | — |
| Retry only unfinished work; cache loss → recovery | Git RFC, index RFC, CLI RFC | Reconciliation table, internal ruling |
| Consent binding of refs/OIDs vs a collaborator's primary advance | Git RFC Wave 03 boundary | **D6** |
| Ignored files in the context worktree at cleanup | — | **D7** |
| Partial worktree removal | — | Recovery required (mirrors Cycle 08 D2), internal ruling |
| Fresh context after promotion | MVP RFC, PRD | `retired_contexts` written at completion, internal ruling R1 |
| Front-end binding, expiry, preview schema | F2, Wave 03 track rule 6 | Deferred to C5/D5 |

## Planning Review — 2026-10-15

An independent reviewer read the first drafts against the code and the
authorities, without the author's reasoning. It confirmed:

- git2 0.20.4 provides `RemoteCallbacks::push_negotiation` (with
  `PushUpdate::src`/`dst`) and `Worktree::prune` (with `valid`,
  `working_tree` and `locked`);
- `require_no_pending_local` checks only `operation_records`, so child
  synchronizations can reserve;
- every sampled grounding line reference is accurate.

The author verified findings 3a and 6 in the source before revising.

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | Critical | A poll between children could advance `T(P)`, giving a merge with the context tree that drops primary commits | Under the integration lease: recompute the digest; with a remote, require base ≤ context, otherwise `PrimaryAdvanced` and re-synchronize with a new attempt. Tests in Tasks 3, 6, 7 |
| 2 | Critical | Records could strand the item: pre-integration refusals, a repeated `RemoteContextChanged`, cancelled children replaying as `Interrupted` | Terminal `abandoned` step; supersession also covers `remote_context_changed` and `ignored_files`; per-attempt child IDs with an end-state table; Task 0 characterization |
| 3 | Important | Deleting the history row does not give `never_published` when the repository history flag is set; resetting early allowed silent republish; the poll claim was wrong | R1 `retired_contexts`, written only in the final transaction; `promote` reservation held from F6 to completion; claim corrected |
| 4 | Important | Cache-loss path blocked by D5; `HistoryUnknown` stricter than `plan_clean_integration` | D5 now applies only when `T(C)` is absent; `AlreadyContained` is exempt; tests added |
| 5 | Important | Reconciling integration by the exact primary tip was too strict, and its error broke the "error means unchanged" invariant | Reachability search; recovery reported as a pending category, never as an error |
| 6 | Important | Ignored files would be pruned without a check; locked worktrees not considered | `include_ignored` pre-check (new D7), `locked` unset, tests |
| 7 | Important | Partial returns skipped the refresh | Refresh on every `Ok` return after integration |
| 8 | Important | Leaving OIDs out of the digest reinterprets the RFC's "refs/OIDs" binding; a stale `T(C)` made reconfirmation certain | Raised as owner decision D6; new `ContextBehindRemote` blocker |
| 9 | Minor | Deletion outcomes did not use the RFC's recovery-required wording | Mapping stated in the design |
| 10 | Minor | Cancellation after integration had no category | `cancelled` added to both pending enums |
| 11 | Minor | Boundary lists disagreed | Single F1–F12 list in the design, referenced by Cycle and plan |
| 12 | Minor | `ExactMapping` does not exist; OID wording; own reservation in cross-check; Task 4 dependency; superseded edge case | `SshDirection::Delete`; wording fixed; cross-check accepts own row; order fixed; edge case noted |

## Owner Approval — 2026-10-15

The owner approved the Cycle, the design and the implementation plan as
revised above. The owner accepted the recommended answer to D1–D7 and ruling
R1. Implementation authority, commits, pushes, native dispatch, pull request,
merge and closure were not granted by this approval. The owner asked for a
[fresh-session handoff brief](2026-10-15-wave-02-cycle-09-handoff.md).

## Implementation Preflight And Task 0 — 2026-10-15

**Status: blocked for owner amendment; Task 0 is incomplete.** The retained
conflict/interruption finding below contradicts the approved child-attempt
table. Stop before Task 1. Remaining characterizations are explicitly pending.

### Authority, Mode And Base

- The owner has now explicitly authorized implementation and checkpoint
  commits, using **subagent-driven development** with independent spec review
  followed by quality review. This Task 0 worker must return its uncommitted
  changes to the controller; the controller handles the checkpoint commit.
- The owner explicitly requires **targeted test runs only until Task 8**,
  superseding Task 0's full baseline and Task 6's full-library run. The full
  check/fmt/clippy/test gates are deferred to Task 8. No dependency change.
- Push, PR, native CI, merge, closure and worktree removal are not authorized.
- Controller preflight, supplied in the task brief: read AGENTS and the
  approved Cycle/design/plan/ledger; committed only the existing planning
  checkpoint as `d027d9896354eb2c9643c9e968d79ebda1cfd823`; then freshly ran
  `git fetch origin main`, observing
  `89d04f901636229392a311241f8e777f0a00962b`. The ticket-worktree rebase was a
  clean no-op: before and after HEAD both `d027d9896354eb2c9643c9e968d79ebda1cfd823`.
  Ancestry and clean status passed. The main checkout's unrelated
  `.superpowers/` was untouched.
- Worker independently read the ticket-worktree `AGENTS.md`, confirmed the
  exact branch and clean initial status, verified both full SHAs with
  `git rev-parse HEAD origin/main`, and rechecked
  `git merge-base --is-ancestor origin/main HEAD` (exit 0). No additional
  fetch/rebase was needed within this controller's Task 0 execution.
- D1–D7/R1 remain approved. No production change has been made. All work was
  performed inside the specified ticket worktree.

### Caller Re-scan / Wave 03 Compatibility

Re-scanned Rust source at this base for `synchronize_remote_observed`,
`prepare_context`, `RemoteOperationAction::Promote` and `remote_context_states`.

| Consumer | Compatibility item |
| --- | --- |
| `mutation/progress.rs:272-280` calls `synchronize_remote_observed`; `remote/sync.rs:5945-5955` is the ordinary wrapper | Preserve the F2 credentials/observer/settings adapter and existing synchronization behavior. Promotion must use its own merging children. |
| `repository.rs:1577,1612,1649` defines/provisions contexts; internal uses at `:1889,2310,2663`, plus `bridges/repair.rs:524` | Shared ticket/document/comment/repair provisioning must retain current behavior; fresh post-promotion edit remains to be characterized. No Wave 03 direct `prepare_context` caller was found. |
| `read/status.rs:202` maps the Promote action; `mutation/bind/resume.rs:177-180,289-293` leaves lifecycle operations `NotResumable`; `remote/state.rs:2895-2939` returns no synchronization target for Promote | Preserve the F2 resume boundary. Promotion's internal restart must not make the boundary resume it. |
| `remote/state.rs:89-100,2101,2292-2311,2487,5075,5202,5389` owns context table/schema/readback and Promote parsing/name mapping; `:1693` validates lifecycle targets | Keep existing snapshot/state consumers compatible with retirement semantics and deletion observations. No new direct Wave 03 SQL consumer of `remote_context_states` was found. |

Test-only callers in `sync_tests.rs`, `state_tests.rs`, local authoring,
recovery and real-SSH fixtures were also found. They do not introduce a new
production consumer beyond the planning grounding above.

### Characterization 1 — Guarded SSH Delete: Confirmed

Throwaway case `task0_delete_negotiation` was added to `tests/ssh_fixture.rs`
and executed through its existing isolated real-SSH harness. The owned server
advertised `ObservationRef::RemoteDocument`; the client pushed exactly
`:refs/heads/manyhands/document/01ARZ3NDEKTSV4RRFFQ69G5FAX` twice, first
refusing negotiation, then allowing it.

- Both negotiations contained one update: **`src()` = current remote OID,
  `dst()` = zero OID**. `src_refname()` was the empty string and
  `dst_refname()` the exact context ref. The approved source-OID guard is
  correct; OID direction must not be inferred from refname direction.
- Callback refusal returned an error, invoked no `push_update_reference`,
  preserved the server ref, and sent **zero parsed receive-update commands**.
  The receive-advertisement-only counter was exactly one after
  `wait_for_helper_audits()`; receive-pack authentication/helper startup still
  occurs, so "sends nothing" means no ref-update command, not no connection.
- Accepted deletion invoked `push_update_reference(context_ref, None)` once.
  The fixture parsed exactly one command with old=current OID/new=zero, the
  server ref was absent, and a fresh fetch advertisement omitted the ref.
- The existing receipt reports `accepted=false` for that successful deletion:
  its proof requires a ref pointing at `Some(new_oid)`, which cannot prove
  absence. This matches Task 1's planned deletion-proof extension.
- Source anchors: `tests/support/ssh_server.rs:475-490,693-707` (receipt proof),
  `tests/support/ssh_remote.rs:396-453` (receipts/counter/barrier). Dependency
  API confirmation: git2 0.20.4 `src/push_update.rs:46-53` defines src as
  current target and dst as new target. No production delete API exists yet.

### Characterization 2 — Raw Worktree Prune: Confirmed On Linux

Throwaway `task0_prune_characterization` used seven disposable repositories,
each with a real tracked file, tracked `.gitignore`, context branch and linked
worktree. Each prune used `valid(true).working_tree(true).locked(false)`.

| Starting state | Observed raw libgit2 outcome |
| --- | --- |
| Clean | Success; path and registration removed; branch retained at its original OID. |
| Ignored file present | Success; ignored file deleted with the worktree. `require_clean_target` passes despite the ignored file; explicit `include_ignored(true)` status detects it. D7's second check is essential. Production helpers must refuse, never delete these files. |
| Locked | Error; tracked file/path and registration remain. |
| Open tracked file (Linux) | Success; path/registration removed, branch retained; the already-open handle still reads the original bytes. Native Windows behavior remains unverified. |
| Partial deletion: tracked file already absent | Success; remaining path/registration removed, branch retained. Production must fail its clean pre-check rather than finish this partial deletion. |
| Partial deletion: `.git` link already absent | Success removes registration but **leaves the tracked worktree file/path behind**. This is a recovery-required state, not successful cleanup. |
| Whole path already absent, registration present | Success removes registration and retains the branch. Production reconciliation must instead report recovery required for this mixed state. |

The partial states were deterministically constructed before calling prune;
this does not claim an actual process kill inside libgit2 was tested. The
approved pre-check and reconciliation table cover these observations. Source
anchors: `remote/sync.rs:500-551` (`materialized_target`), `:554-622`
(`require_clean_target`); design `:471-495` (pre-check/prune/reconciliation).
No helper was added that could delete ignored files.

### Characterization 4 — BLOCKER T0-1: Interrupted Retained Conflict

**Contradiction:** design `:383` puts `Interrupted` in the **new ID** row,
while `:384` grants a retained-merge same-ID exception only to
`RecoveryRequired`. The lifecycle repeats interrupted → new attempt at
`:337-338`. A real child can return `Interrupted` with an owned, unfinished
conflict that requires its original operation ID. The parent cannot choose
its next attempt from the error category alone as currently specified.

Throwaway `task0_interrupted_retained_context` in
`tests/remote_merge_recovery.rs` called **`synchronize_remote_observed`
directly**, with default merging settings and a continuing observer. It used
the existing `Pair` two-clone SSH fixture, its disposable registries/keys and
real document saves. No transport or synchronization mock was used.

Reproduction and observed results:

1. Clone A saves and publishes a document edit; clone B independently saves a
   conflicting edit. B's document-context child returns `ConflictPending`
   with its operation ID. Actual linked-worktree HEAD, index conflict and
   merge metadata are attested by the existing `conflict_installed` helper.
2. During B's same-ID restart, a fixture SQL trigger on changed `owner_epoch`
   records a marker and sets `cancel_requested=1` on that row. The marker
   count is **one**. The call returns **`Interrupted`**.
3. The joined operation/integration row is exactly
   **`(phase='interrupted', cancel_requested=0, step.phase='conflict_pending')`**.
   The conflict's HEAD/index/metadata/worktree image is unchanged.
4. Same ID without restart returns **`RecoveryRequired`**.
5. Reopen the service. The design's proposed new-ID call returns
   **`TargetNotMaterialized`**, not a continuation of the retained conflict.
   The front-matter title conflict fails new-call canonical-context validation
   (`local_target` maps that validation to `TargetNotMaterialized`).
6. Original ID with `restart=true` returns **`ConflictPending` with the
   original operation ID**. It preserves the same physical conflict image,
   with **zero additional authentications and zero receive updates** across
   the replay/new-ID/restart probes.

Source explanation (line numbers after removing throwaway tests):

- `remote/reservation.rs:400-417,428-437`: cancellation preserves pending
  conflicts/owned resolution as a non-terminal interrupted row and clears
  the cancel request.
- `remote/sync.rs:6010-6014`: ordinary non-authoritative replay requires restart.
- `remote/sync.rs:6035-6085`: restart reconciles the child's owned merge before
  live configuration, cleanliness or transport.
- `remote/sync.rs:6111-6113,475-494` and `repository.rs:9095-9123`: a new ID
  performs ordinary canonical-target validation, not owned-merge recovery.
- Existing related evidence: `tests/remote_merge_recovery.rs:3322-3393`
  already exercises cancellation of a primary conflict and the same-ID
  recovery requirement; this throwaway probe establishes the behavior for a
  document-context child through the observed entry point.

**Owner amendment needed:** settle how a promotion retains/reacquires a child
that returns `Interrupted` while owning conflict/resolution evidence, and
make the attempt table/lifecycle consistent. No design or production behavior
was silently changed. Stop Task 0 and obtain owner review before continuing.

### Exact Targeted Commands And Results

All commands ran in the ticket worktree. The counts below describe actual
test executions; no full suite, check, clippy or dependency update ran.

| Exact command | Runs / result |
| --- | --- |
| `devenv shell -- cargo test --locked --test ssh_fixture push_round_trip -- --exact` | 1 run; baseline: **1 SSH case passed**. |
| `devenv shell -- cargo test --locked --test ssh_fixture task0_delete_negotiation -- --exact` | 2 runs: first compile failed because throwaway test used `unwrap_err` on non-Debug `Reference` and omitted `fixed` error conversion; corrected test-only API usage; second **1 SSH case passed**. |
| `devenv shell -- cargo test --locked --lib task0_prune_characterization -- --nocapture` | 3 runs: first **0 passed/1 failed/772 filtered** (incorrect throwaway `statuses(None)` assumption; it includes ignored entries), second **0 passed/1 failed/772 filtered** (discovered missing-`.git` partial-prune behavior), final **1 passed/0 failed/772 filtered**, seven matrix cases. First run also had one unused-mut warning, removed for subsequent runs. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_interrupted_retained_context -- --exact` | 5 runs: four failed cases while identifying the exact new-ID error (initial expectations `RecoveryRequired`, then `WorktreeConflicted`; fixed numeric probes classified the error as `TargetNotMaterialized`); final **1 SSH case passed** with every assertion above. Debug probes emitted only fixed numeric categories, never endpoints, keys or fixture paths. |

The failures were characterization/test-authoring findings, not production
fixes. Systematic-debugging skill was used; only throwaway tests changed.
Tests used fixture counters, a SQL-trigger marker and helper-audit barriers,
with no sleeps or external application data/global key/config changes.

### Pending, Cleanup And Review Handoff

- **Pending characterization 3:** prepared preview merge tree versus actual
  `Merge primary into document` synchronization (D4/D6).
- **Pending characterization 4:** the full completed/identity/external
  resolution/cancelled/interrupted-without-retained-merge/push-rejected/
  transport/busy/yielding/recovery-with-and-without-retained-merge matrix;
  resolution followed by restart; and lost-push-result single-push replay.
  Only conflict replay/restart and interrupted-with-retained-conflict were
  established in the probe above. Do not treat the entire table as verified.
- **Pending characterization 5:** fresh `prepare_context(Edit)` after branch
  and worktree removal with the document already on primary.
- **Native gap:** Windows open-file pruning; no native CI was dispatched.
- Removed only this worker's throwaway additions from `sync_tests.rs`,
  `ssh_fixture.rs` and `remote_merge_recovery.rs` using `apply_patch`. No
  production or dependency edits remain. Durable changes are this ledger and
  ticket comment `01M5125XP0N4Q6S8V0X2Z4B6DJ`.
- Final cleanup verification: `git diff --exit-code -- src tests Cargo.toml
  Cargo.lock devenv.nix devenv.lock` and `git diff --check` both exited 0.
  `git status --short` showed only the ledger and the new ticket comment.
- Self-review: the blocker is supported by the actual observed entry point,
  real Git state and offline replay counters; prune crash behavior is scoped
  to constructed partial states, and all outstanding probes remain marked
  pending. Ready for controller's independent spec then quality review of
  the evidence, **not** a passing Task 0 gate. No worker commit was made.

### Independent Reviews And Proposed Amendment

- **Spec review: passed for the blocked evidence checkpoint.** A fresh
  reviewer checked the current diff, caller boundaries, reservation and sync
  source, and existing conflict-recovery tests. It confirmed T0-1 and the
  required stop; it did not mark Task 0 complete.
- **Quality review: approved for checkpointing, no findings.** A second fresh
  reviewer checked accuracy, reproducibility, pending/native limits, privacy
  and canonical comment schema. Neither reviewer reran the characterization
  tests; the execution counts above are the implementer's recorded results.
- **Proposed owner amendment, not yet approved:** persisted child ownership
  takes precedence over the returned error category. An `Interrupted` child
  retaining a conflict or owned resolution keeps its attempt ID and uses
  `restart=true`. An interrupted child with confirmed absence of retained
  ownership gets a new attempt/ID. Uncertain ownership is not proof of absence.
  Authoritative completion still replays without repeating work; retained
  `RecoveryRequired` still uses same-ID restart.
- If approved, update the design lifecycle and attempt table (including the
  primary-publication reference), Cycle context-synchronization rule, and Task
  6 attempt expectations consistently, then finish the pending Task 0 probes.
  The approved design and production code remain unchanged at this checkpoint.
