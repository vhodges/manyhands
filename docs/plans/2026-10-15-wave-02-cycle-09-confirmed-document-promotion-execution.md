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

## Task 0 Resume — Owner-Approved T0-1 Amendment (2026-10-16)

The owner approved the controller's recommended ownership-first amendment
("yes"). Persisted child ownership takes precedence over error category:
`Interrupted` with retained conflict or owned resolution keeps the original
attempt ID and `restart=true`; confirmed absence permits a new ID; uncertain
ownership is not absence. Authoritative completion is never rerun, and
retained `RecoveryRequired` continues under the same ID. This applies to both
context and primary children. Cycle, design lifecycle/table, Task 6 and handoff
are updated consistently. The historical T0-1 finding/reviews above are kept.

Controller fresh preflight: clean status; `git fetch origin main` observed
`89d04f901636229392a311241f8e777f0a00962b`; ticket HEAD
`df4fe56ce0548290293c3f6d96f23f1ddbfc51a2`; ticket-worktree rebase onto
origin/main was a clean no-op (same before/after HEAD); ancestry passed.
`d027d98` is the planning checkpoint and `df4fe56` the independently reviewed
blocker checkpoint. Worker re-read AGENTS and confirmed clean branch status.
Implementation/checkpoint authority, SDD mode and targeted-tests-until-Task-8
override persist. This worker returns changes uncommitted for spec then
quality review; no Task 1 or outward action is authorized here.

Delete/prune evidence is reused without repeating those probes. Pending Task 0
characterizations are being resumed with throwaway tests and real service
entry points; results and cleanup will be recorded below.

### Resume Results — Characterizations 3 And 5

**Merge-tree equivalence confirmed.** Throwaway `task0_merge_tree` used two
fresh `Pair` SSH fixtures. B saved a document body edit on C; A published a
primary advance, either a separate file or a nonoverlapping title edit of the
same document. The test fetched that primary to B's actual `T(P)`. A temporary
bridge in the existing `#[cfg(test)]` sync-tests module called the real private
`prepare_clean_merge(T(P), C)` and imported its result to compute its tree OID.
The test then called real `synchronize_remote_observed` for B's document:

- actual commit parents were `[C, T(P)]` and its exact subject was
  `Merge primary into document <ULID>`;
- its tree OID equalled the prepared tree OID in both cases;
- the resulting diff from the new primary changed only the document path,
  not the primary-only addition. This supports the D4/D6 merge-equivalence
  premise without pretending to test the as-yet-unimplemented digest.

Anchors: `remote/sync.rs:702-782` (real preparation/import), `:3848-3923`
(real merge commit, subject and guarded application). The temporary bridge
wrote only fixture objects; no production visibility or behavior changed.

**Fresh edit context confirmed.** `task0_fresh_context` saved B's document
edit, installed that commit on the fixture primary, and added a later primary
commit. It pruned the old context worktree and deleted its branch, attested
that path/registration/branch were all absent, then called actual
`prepare_context(Edit)` with a new operation ID. The outcome was **Created**
at the deterministic path; both context HEAD and branch equalled the latest
primary OID, and the worktree held the primary's document bytes and its later
file. Anchors: `repository.rs:1708-1756,1783-1819`.

### Resume Results — Full Child Replay / Restart Matrix

The new probes called `synchronize_remote_observed` directly with default
merging settings and fixture `SessionCredentials`. The existing regression
cases call the real `synchronize_remote` wrapper, which delegates directly to
that entry point (`remote/sync.rs:5945-5955`). No synchronization/transport
mock or proposed promotion executor supplied a result.

`task0_transient_{primary,context}_{replay,replace}` covered seven end states
for **both Primary and Document targets**, with separate fixtures for same-ID
restart and new-attempt strategies: 28 fixture scenarios. Every tested
non-owned attempt had **zero integration-step rows**. Cancellation used the
actual observer `Cancel`; interruption set `yield_requested=1` at the observer
boundary; non-owned recovery used observer `Stop`; transport disconnected at
authentication; rejection used the receiver's checked-out-branch refusal
(including selecting the document HEAD for that case). Busy/yielding used
real manual/poll reservations. No sleeps.

In the following table, **replay** means same ID, `restart=false`; **restart**
means same ID, `restart=true`. A successful repaired restart is a capability
of the existing child, not a change to the parent's approved new-attempt
policy for non-owned failures.

| Actual child state | Replay | Restart | Evidence / approved parent selection |
| --- | --- | --- | --- |
| Authoritative Completed | Exact stored `Published` result, no authentication or push | Same stored result, no authentication or push | `task0_authority_and_lost_push`, both targets with normal and initially lost results. Parent consumes stored authority, never reruns. |
| ConflictPending | `RecoveryRequired`, offline | Same `ConflictPending` before resolution; `Published` after real canonical resolution, one merge/one push | Executed existing `two_clone_document_conflict_resolved_and_published`; same attempt and existing resolution flow. |
| IdentityRequired | `RecoveryRequired`, offline | Without confirmation: `IdentityRequired` with the same configuration digest. With confirmation: `Published`, one merge/one push | `task0_identity_and_owned_recovery`, both targets; existing primary identity regression also executed. Exactly one remote row exists and no candidate OID exists at the identity boundary. Keep original attempt. |
| External-resolution-only conflict | `RecoveryRequired`, offline | `ConflictPending` until external repair; a genuine whole-merge repair then publishes with the original ID | Executed `code_conflict_false_repairs_then_external_repair`, then extended it temporarily with explicit direct replay/restart assertions. `ExternalResolutionRequired` itself comes from the real resolver/inspection flow, not the synchronization call; do not manufacture that as a child return. |
| Cancelled, no retained ownership | `Interrupted` | `Interrupted`, terminal; a new ID publishes | Both-target transient matrix. The synchronization error enum has no Cancelled variant: the persisted cancelled state surfaces as Interrupted. Parent selects a new attempt. |
| Interrupted, no retained ownership | `RecoveryRequired`, offline | `Published` once the yield is cleared by restart | Both-target transient matrix. Separate replace probes confirmed a new attempt also publishes; parent selects the new attempt per the approved table. |
| PushRejected, no retained ownership | `RecoveryRequired`, offline | `Published` after receiver refusal is removed | Both-target transient matrix, including real context rejection. Separate replace probes published with a new ID after retiring an active old reservation. |
| Transport failure, no retained merge | `RecoveryRequired`, offline | `Published` after clearing the authentication disconnect | Both-target transient matrix. Separate replace probes also published under a new ID after retirement. |
| Busy | Busy while the competing manual reservation exists | Busy while it exists; Published after its release | Both targets; **zero child rows** were inserted. Fresh-ID strategy also succeeds after release. |
| PollYielding | PollYielding while the poll reservation exists | PollYielding while it exists; Published after acknowledgement/release | Both targets; **zero child rows** were inserted. Fresh-ID strategy also succeeds after release. |
| RecoveryRequired, no retained merge | RecoveryRequired, offline | Published when observer Stop is no longer supplied | Both-target transient matrix, zero integration rows. Separate replace probes retire an active old reservation then publish a fresh ID. |
| RecoveryRequired with retained clean candidate | RecoveryRequired, offline | Persistent `Repository(RecoveryRequired)` while the injected checkpoint failure remains; after removal, Published with the **same candidate OID**, one merge and one push | `task0_identity_and_owned_recovery`, both targets. Trigger rejected applied-phase observation after the ref had moved; candidate evidence was present. Same attempt retained. |
| RecoveryRequired with retained conflict / owned resolution | RecoveryRequired at unresolved evidence; same-ID restart inspects/reconciles, without adopting foreign state | Existing same-ID recovery preserves the original merge/resolution and publishes only after valid repair/continuation | Executed external false-repair regression and both `resolution_and_publication_seams_inventory` cases. The latter stop at resolution/Push/discovery checkpoints and count one resolution/one push. |
| Interrupted with retained conflict | RecoveryRequired | Original ConflictPending, offline | Reuse reviewed T0-1 document-context evidence above; owner amendment selects original ID. |
| Interrupted with owned resolution | Existing resolution remains owned, not terminal cancellation | Same-ID synchronization acknowledges interruption; identical resolution attempt subsequently completes | Executed library regression `cancellation_after_an_interrupted_resolution_is_a_recoverable_stop`, four actual owned stages (`resolution_prepared`, `commit_prepared`, and two applied-but-unreleased seams). This complements T0-1; it is a real primary synchronization over fixture Git/DB, not an SSH transport assertion. |

**Lost push result, one push:** `task0_authority_and_lost_push` disconnected
after receive-pack for both targets without any merge. The first call returned
Transport, the server withheld report-status, ordinary replay returned
RecoveryRequired, and same-ID restart recovered `Published`. The receive
counter stayed at **exactly one update**, and both later ordinary and restart
replays returned stored Published with zero further authentication, even with
authentication disconnected. Merge count remained zero. The executed existing
`resolved_merge_ambiguous_acceptance_and_persistence` adds the retained-
resolution cases, testing both a lost SSH result and lost `push_verified`
persistence with one accepted push and unchanged commit inventories.

### Task 6 Integration Notes From Actual Results

- **Do not infer restart from the outer error's terminal/non-terminal label.**
  Real clean-merge IdentityRequired retained **one remote child row** for
  both targets, even though it had no candidate; ordinary replay is not
  sufficient, and confirmed continuation must set restart on that original
  ID. No separate no-row IdentityRequired synchronization path was observed
  or fabricated. `committing_identity` is reached under an already reserved
  owner (`remote/sync.rs:3214-3230,3884`). If an outer operation reports an
  identity issue before invoking/reserving a child, that is not evidence of
  a child row. Read the actual child row: absent means first execution, not
  lost ownership; a failed/unreadable lookup is uncertain ownership, not absence.
- Busy/PollYielding really insert no row. Their tested same-ID `restart=true`
  calls after release run as new executions, not resumed work. The parent can
  still mint a new ID as specified; its restart flag follows actual row
  existence, not the previous error category.
- **Retiring an attempt includes its active reservation.** The replace probes
  first proved no integration ownership. When the failed child still held an
  active reservation, they called existing `cancel_remote_operation`, then
  the original-ID synchronization with restart to acknowledge it. That call
  returned **RecoveryRequired while durably changing the row to cancelled**
  (the replay arm in `sync.rs:6133-6169`), not Interrupted. A new ID then
  published for every mode/target. A later cancelled-row replay is Interrupted.
  This is an implementation detail of the approved retirement/new-attempt
  rule; treating that acknowledgement's error alone as failed retirement
  would be incorrect. Retained/uncertain ownership must not enter this path.
- Historical applied rows are not automatically unfinished ownership. Existing
  reservation logic distinguishes pending conflict/resolution and applied
  resolution whose release evidence is incomplete
  (`remote/reservation.rs:400-417`). Authoritative completion wins first.
- Source anchors for the measured matrix: `remote/sync.rs:5984-6014`
  (authority/cancelled/ordinary replay), `:6035-6085` (local owned-merge restart),
  `:6133-6169` (reservation restart), `:4038-4058` (observer requests);
  `remote/reservation.rs:856-904` (replay/arbitration/no new row), `:952-963`
  (cancellation request), `:2099-2168` (restart/acknowledgement).

### Resume Exact Commands And Results

All commands ran in the ticket worktree. Only targeted tests were selected;
the full gates remain deferred to Task 8. Initial failures below were
throwaway test assumptions/setup or harness limits, corrected without any
production change. Systematic-debugging guidance was applied.

| Exact command | Actual results |
| --- | --- |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_` | Two runs of the original three probe cases: first **1 passed/2 failed**; second **2 passed/1 failed**. Merge-tree assertion initially omitted the actual subject's ULID suffix (tree/parents already matched). Fresh-context passed both runs. Transient setup initially attempted reservation before binding the live endpoint configuration; priming through a real AlreadyCurrent synchronization fixed setup. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_transient_matrix -- --exact` | Four runs before splitting: original 14-scenario matrix **1 passed**; expanded 28-scenario matrix **0 passed/1 failed** on the cancellation-acknowledgement error assumption; two more expanded runs **0 passed/1 failed** via the isolated-child/watchdog failure path, without an assertion line, partway through the 25th setup. The harness has a fixed 100-second per-case bound (`tests/support/ssh_harness.rs:237`); splitting the long combined probe into seven-scenario cases below established all 28 scenarios within the unchanged bound. No application failure was reproduced by those bounded cases. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_transient_context_replace -- --exact` | **1 passed**, seven scenarios. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_transient_primary_replay -- --exact` | **1 passed**, seven scenarios. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_transient_primary_replace -- --exact` | **1 passed**, seven scenarios. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_transient_context_replay -- --exact` | **1 passed**, seven scenarios. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_identity_and_owned_recovery -- --exact` | **1 passed**, four scenarios (identity/retained candidate × primary/document). |
| `devenv shell -- cargo test --locked --test remote_merge_recovery task0_authority_and_lost_push -- --exact` | **1 passed**, four scenarios (normal/lost result × primary/document). |
| `devenv shell -- cargo test --locked --test remote_merge_recovery two_clone_document_conflict_resolved_and_published -- --exact` | **1 passed**; canonical conflict resolution then original-child publication and authoritative replay. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery confirmed_identity_completes_a_divergent_primary_merge -- --exact` | **1 passed**; unchanged identity regression including stale-confirmation refusal. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery code_conflict_false_repairs_then_external_repair -- --exact` | Two runs, **1 passed each**; second added throwaway explicit ordinary-replay/original-ID-restart assertions before the real external repair. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery resolved_merge_ambiguous_acceptance_and_persistence -- --exact` | **1 passed**, lost transport and lost verified-checkpoint persistence, one accepted push each. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery resolution_and_publication_seams_inventory` | **2 passed** (primary and context), retained-resolution/publication/discovery seams. |
| `devenv shell -- cargo test --locked --lib cancellation_after_an_interrupted_resolution_is_a_recoverable_stop` | **1 passed/0 failed/771 filtered**, four owned-resolution stages. One dead-code warning for the throwaway merge bridge unused by this library-only selector; the bridge is removed before handoff. |

### Task 0 Resume Handoff / Self-Review

- The approved amendment and SDD/targeted-testing overrides are reflected in
  Cycle, design, implementation plan and handoff. Historical blocker evidence
  and approval history are preserved. The caller scan and Linux delete/prune
  baseline from the reviewed checkpoint are reused; production did not change
  between checkpoints.
- **All remaining Task 0 characterization categories now have real-path
  execution evidence. No new design contradiction was found.** The measured
  child capabilities are distinguished from the parent's chosen retry policy;
  reservation retirement and identity-row details are recorded above for Task 6.
- Removed this worker's merge bridge, eight new SSH cases/helpers and extra
  external-conflict assertions with `apply_patch`. No production, dependency,
  retained-test or Cargo changes are intended. No Task 1, commit or outward
  action has been performed. Controller owns independent spec then quality
  review and the checkpoint commit.
- Remaining limits are the already-recorded native Windows open-file/prune
  gap and deterministic partial-deletion simulation rather than a process kill
  inside libgit2. Full Rust/CLI/native gates are not claimed; Task 8 owns the
  full local gate. Unknown-ownership handling of the future promotion executor
  is a Task 6 test obligation, not a delivered implementation in this probe.
- Final cleanup verification: `git diff --exit-code -- src tests Cargo.toml
  Cargo.lock devenv.nix devenv.lock` exited 0, as did `git diff --check`.
  Status showed exactly the five updated Cycle/planning documents and new
  canonical comment `01M513H8Z0N4Q6S8V0X2Z4B6DK`. HEAD remains the supplied
  `df4fe56ce0548290293c3f6d96f23f1ddbfc51a2`; no worker commit.

### Task 0 Resume Reviews — Documentation Consistency Corrections

Spec review passed. Quality review requested three documentation corrections;
each was verified against the current source and approved contract before
editing. These complete the approved ownership-first amendment/invariants;
they add no production behavior or new implementation scope.

| Finding | Verification and resolution |
| --- | --- |
| Important: primary-child recovery had a pending mapping but was omitted from the exhaustive publication category list | Cycle outcome invariant (`An error means nothing reached the primary`) and design Public Interface require `Ok` after integration. Added fixed redacted `recovery_required` to the publication category list; persistent recovery with retained primary-child ownership and uncertain ownership yield `Ok` partial publication pending synchronization recovery, cleanup awaiting publication, never post-integration `Err`. Made the attempt-table mapping explicit and added matching Task 6 outcome/redaction/ownership/refresh expectations. |
| Minor: Task 6/table described ExternalResolutionRequired as a child synchronization result | Source `remote/sync.rs:4777-4805,5320-5349` locates this outcome in conflict read/resolve; Task 0's external-conflict evidence confirms synchronize retains ConflictPending. Changed the table to ConflictPending whose inspection requires external resolution, and Task 6 to the actual inspect/read/resolve → external repair → original-ID restart flow. Explicitly prohibit injecting the impossible synchronize return. |
| Minor: implementation-plan self-review still claimed nothing had run | Replaced the stale ending with executed Task 0 characterizations/targeted baseline and recorded T0-1 amendment, distinguishing the still-planned implementation/verification of Tasks 1–8. |

Only design/plan prose and review evidence changed in this correction pass.
No tests were needed or run for these documentation-only corrections.
`git diff --check` passed (exit 0); `git diff --exit-code -- src tests Cargo.toml
Cargo.lock devenv.nix devenv.lock` also exited 0. No commit; the controller
sends the corrections for quality re-review before checkpointing.

Quality re-review approved the checkpoint with all three findings resolved
and no remaining findings. Task 0 is complete for the local implementation
gate; Task 1 may begin. Native gaps and the deferred full Task 8 gate remain
as recorded above. The controller owns the authorized checkpoint commit.

## Task 1 — Guarded Remote Document-Context Delete (2026-10-16)

**Status: implementation and targeted verification complete; ready for independent
spec review, then quality review. Uncommitted.** The fresh implementer used the
controller's completed preflight and Task 0 reviews, read the worktree AGENTS,
and confirmed clean initial status on the exact ticket branch at `25514a3`.
No additional fetch/rebase, delegation or outward action was performed. The
controller owns the checkpoint commit after reviews. The targeted-only override
continues through Task 8.

### Delivered Interfaces And Decisions

- `SshDirection::Delete` resolves/authenticates against the existing **Push**
  endpoint, including a distinct configured push URL. Only the exact mapping
  `:refs/heads/manyhands/document/<ULID>` for the selected document context is
  admitted; primary, ticket, malformed, wildcard, force, duplicate and empty
  mappings fail closed. Invalid scope is checked before a transfer reconnect.
- `AuthenticatedSshRemote::delete_exact(plan, target, expected: git2::Oid) ->
  Result<DeleteObservation, SshTransportError>` and crate-visible
  `transport::DeleteObservation::{Deleted, StillPresent(Oid), Changed(Oid)}`.
  Zero expected OIDs are refused. There is no new public promotion operation.
- Every call first freshly advertises. Already-absent contexts return `Deleted`
  without sending a delete. For a present context, at most one delete push is
  attempted. The callback requires exactly one update, `src() == expected`,
  zero `dst()`, and the exact destination ref. It emits only fixed refusal text.
  The mismatch test deliberately reaches negotiation rather than short-circuiting
  on the pre-advertisement's different OID: zero parsed update commands, four
  advertisement-only helpers (initial authentication, pre-observation, refused
  negotiation, post-observation).
- Rejection or ambiguous transport return is followed by a fresh authenticated
  advertisement: absent → `Deleted`, expected → `StillPresent`, different →
  `Changed`. Failure to obtain that advertisement remains an error. A later
  retry advertises before any push, and never blindly repeats an accepted delete.
  Existing selected-key, host-trust and endpoint rechecks run on each reconnect;
  policy failures are propagated rather than converted into observation proof.
  Reservation/generation/safe-point orchestration remains the later caller's
  responsibility, using the existing authenticated-scope interfaces.
- Test-only `Checkpoint::DeleteAttemptReturned` identifies the interval before
  post-send observation. It proves that a collaborator's replacement wins over
  successful callback status, and that lost status plus failed observation
  remains an error until a subsequent observe-first retry proves absence.
- Fixture hooks: `reject_context_deletion(bool)` uses native receive-pack
  checked-out-context deletion refusal; `race_context_update(ObservationRef,
  expected, competing)` reuses the existing deterministic receive race, after
  advertisement and before forwarding the complete command to receive-pack.
- Fixture proof snapshots each parsed command's old **direct** OID before
  forwarding/racing. A deletion receipt requires successful helper completion,
  non-noop old/new OIDs, a matching old snapshot and actual `NotFound` afterwards.
  A dangling symbolic ref is not absence. Missing snapshot evidence fails closed.
  Ordinary push proof retains its existing non-noop/new-tip rule. Snapshot reads
  are skipped when no new command was parsed, including opaque pack chunks.
- `SshTransportError` keeps its existing typed context fields/API but now formats
  Debug using only direction and fixed error kind, omitting authority, root and
  remote name. Display remains fixed category text. New observations contain only
  a variant and optional OID. No endpoint, content or secret enters the new
  records, callback diagnostics or Debug output.

### Exact RED / GREEN Commands And Results

Tests were authored first; initial missing-API compilation failures are separated
from runnable behavioral RED evidence below. Before implementing the mapping,
callback, post-push observation or deletion proof, the required library command
had run, as had both SSH selectors. A declaration-only Delete direction and
exhaustive endpoint arms then permitted the runnable mapping/privacy RED run;
the Delete mapping still returned an empty set at that point. The new fixture
hooks ran before the receipt-proof change, exposing the successful-deletion
receipt bug. No dependency change or full-suite/check/clippy gate ran.

| Exact command | Actual runs and results, in execution order for that command |
| --- | --- |
| `devenv shell -- cargo test --locked --lib transport` | **4 runs:** initial compile RED, 7 missing-Delete diagnostics, zero tests executed; runnable RED **36 passed / 2 failed / 736 filtered** (missing delete mapping and endpoint/path-leaking Debug); GREEN **38 passed / 0 failed / 736 filtered**; final GREEN **38 passed / 0 failed / 736 filtered**. |
| `devenv shell -- cargo test --locked --test ssh_fixture context_delete` | **5 runs:** initial compile RED, 3 missing-hook/proof-helper diagnostics; runnable RED **2 passed / 1 failed** (successful deletion not accepted by receipt); expanded proof RED **2 passed / 2 failed** (receipt and valid guarded-absence proof); GREEN **4 SSH cases passed**; final GREEN **4 SSH cases passed**. Negative proof checks include still-present, unmatched old OID, zero/zero no-op and dangling symbolic ref. |
| `devenv shell -- cargo test --locked --test ssh_transport exact_delete` | **5 runs:** initial compile RED, 16 missing direction/API/hook/type diagnostics; compile failure with 3 non-exhaustive matches in existing endpoint test helpers, corrected to share Push's cases; GREEN **7 SSH cases passed**; after adding the post-send case, **7 passed / 1 failed** at its incorrect transport-category assertion; final GREEN after receipt hardening **8 SSH cases passed**. |
| `devenv shell -- cargo test --locked --test ssh_transport exact_delete_post_send_observation -- --exact` | **2 runs:** compile RED for missing checkpoint; final GREEN **1 SSH case passed**, two post-send scenarios. The intervening runnable failure above established that the auth-boundary disconnect is classified by existing backend policy as `KeyRejected`, not `TransportUnavailable`; corrected only the test expectation. Systematic debugging used the auth fault location and existing `backend_failure` classification; no transport policy change. |
| `devenv shell -- cargo test --locked --test ssh_transport exact_` | **1 run: 15 SSH cases passed** (8 delete cases plus 7 existing exact fetch/push/policy cases). |
| `devenv shell -- cargo test --locked --test remote_synchronization` | **2 runs:** first shell timeout at 600,000 ms after **52 cases reported passed**, no case failure reported and no completed target result; rerun with 1,800,000 ms shell allowance completed, **63 SSH cases passed**, exit 0. Expected fixture unwind observation remained a passing case. |
| Unsupported continuation command below | **1 run:** runner rejected `--skip` options, exit 2, **zero cases executed**. No source or runner behavior was changed to accommodate it; the complete target rerun above supplied the regression gate. |
| `devenv shell -- cargo test --locked --test remote_synchronization fixture_` | **1 final run: 6 SSH cases passed** after the self-review missing-snapshot guard/opaque-pack fast path. |
| `devenv shell -- cargo test --locked --test remote_synchronization receiver_` | **1 final run: 8 SSH cases passed**; substring selection also includes five fixture receipt/controller cases. |
| `devenv shell -- cargo fmt` | **2 runs**, exit 0; formatter only, not the deferred full Task 8 gate. |

Exact rejected continuation command (retained so no attempted verification is
silently omitted):

```sh
devenv shell -- cargo test --locked --test remote_synchronization -- --skip ignored_ --skip fixture_ --skip synchronization_ --skip hostile_endpoint_redaction --skip receiver_ --skip primary_ --skip context_ --skip distinct_push_ancestry_and_tracking --skip binary_primary_conflict_side_read_is_refused --skip symlink_primary_conflict_side_read_is_refused --skip mixed_mode_primary_conflict_side_read_is_refused --skip worktree_symlink_is_never_followed_during_conflict_side_read --skip remote_change_during_fetch --skip verified_context_immediate_deletion --skip incompatible_generation_history_unknown --skip post_accept_ --skip merge_candidate_race_continues_through_appended_publication --skip merge_candidate_continuation_ambiguous_acceptance_verifies_same_attempt --skip merge_candidate_publication_stop_at_settlement --skip merge_candidate_publication_stop_at_window_append --skip merge_candidate_continuation_cancel_before_push --skip merge_candidate_continuation_takeover_after_push_return --skip merge_candidate_publication_stop_at_stage_observation --skip merge_candidate_publication_stop_at_prepared
```

### File Scope, Self-Review And Handoff

- Transport implementation/tests: `src/repository/transport/{remote.rs,
  callbacks.rs, operation_tests.rs}`; fixtures: `tests/support/{ssh_server.rs,
  ssh_remote.rs}`; covering cases in existing `tests/ssh_fixture.rs` and
  `tests/ssh_transport/exact.rs`.
- Necessary enum/API plumbing: `src/repository/transport/{mod.rs, endpoint.rs,
  operation.rs}`, `src/repository/remote/observation.rs` and
  `tests/ssh_transport/endpoints.rs`. Each new exhaustive Delete arm uses the
  existing Push endpoint/direction. `transport/error.rs` owns the tested Debug
  redaction. These additions are needed because the approved new enum is shared
  beyond the task's primary files; no front-end behavior was implemented.
- Durable evidence: this ledger and canonical ticket comment
  `01M5157Z00N4Q6S8V0X2Z4B6DN`. The implementation plan is not marked reviewed
  or committed by this worker.
- Self-review covered exact mapping validation before reconnect, libgit2 OID
  direction, one-command/no-command receive evidence, post-send state overriding
  callback status, failed re-advertisement, observe-first retry, distinct Push
  endpoint, privacy, non-delete proof compatibility and changed-file scope.
  Final structural verification: `git diff --check` exited 0, as did
  `git diff --exit-code -- Cargo.toml Cargo.lock devenv.nix devenv.lock`.
  `git rev-parse HEAD` remains `25514a312370863c7d2fb3082c53c6e7cc14a985`;
  `git branch --show-current` confirms the exact ticket branch. Status shows
  only the named source/test files, ledger and new canonical comment; no
  unrelated initial work existed or was overwritten.
- No new dependencies, GPUI, sleeps, global config, developer keys or real app
  data. Fixture faults use the existing owned SSH backend, counters, helper-audit
  barriers and synchronous checkpoints. No approved-contract contradiction found.
- **Review boundary:** compare the uncommitted diff to `25514a3`. Task 1 is ready
  for controller-owned independent spec then quality review. The ticket remains
  open. Full Rust gates/CLI smoke and native platform coverage remain at Task 8;
  no native delete proof is claimed. Promotion orchestration, durable cleanup
   pending/recovery mapping and generation/reservation integration are later tasks.

### Task 1 Independent Reviews

Spec review passed with no findings after inspecting the complete diff and
independently verifying 38 transport unit tests, four fixture deletion cases,
15 exact transport cases and 63 synchronization cases. Quality review then
approved with no findings; it inspected the race, authentication, privacy and
fixture proof paths without repeating those passing tests. Task 1 is complete
at the local task gate; the full Task 8 and native gates remain pending.

## Task 2 — Promotion Observation And Read-Only Preview (2026-10-16)

**Current status: Task 2 complete at the local gate; both amended spec and quality
reviews PASS. T2-1 resolved, prior spec fixes retained, 53 targeted tests GREEN.
Uncommitted; controller checkpoint pending. Task 3 is next.** Historical blocker,
amendment and implementation evidence remain below, followed by the review results.
Initial implementation evidence follows. Fresh worker read this worktree's
AGENTS and the TDD/Cycle implementation skills, approved design/Task 2 and prior
execution evidence. Initial status was clean at
`3d470272cbb78cbe614340dbdef56746bfdb85a5` on
`manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. The controller-supplied current
`origin/main` preflight (`89d04f9`) is reused; no second rebase or delegation.
Implementation/checkpoint authorization and targeted-tests-until-Task-8 remain
in force; the controller owns commits after both independent reviews.

### Delivered Interfaces And Decisions

- New `promotion/{mod.rs,observe.rs,observe_tests.rs}` defines the requested
  preview request, preflight/blockers, accessor-only preview/path changes,
  planned integration/publication/cleanup, and opaque Copy/Eq expectation with
  byte round-trip and redacted Debug. Preview/path-change Debug exposes only
  IDs/OIDs/categories, not paths, branch names, remote names or content.
- Internal `observe::observe_promotion(repository, root, config, item,
  snapshot, pending_conflict) -> PromotionObservation` is the reusable actual-Git
  observation for preview and later post-sync/leased checks. The caller supplies
  a coherent remote-state view; `into_preflight` emits every independently
  observable blocker in enum declaration order. No executor/records/integration
  mutation was added. `RepositoryOperation::Read` is used for repository errors.
- The public preview reads configuration and the registry read-only, in a short
  cache-read-guarded SQLite read transaction. It calls the existing snapshot
  reader/publication-evidence method and pending-conflict query. It takes no
  repository lease, reserves nothing, does not refresh, and has no transport or
  credentials path. Actual Git tracking refs, not cached tracking OIDs, decide
  ancestry and the remote integration base.
- Context conformity consumes the real `observe_root` active-context discovery
  rule and `materialized_target`; both targets use `require_clean_target`.
  Missing context branches are classified before cleanliness, avoiding spurious
  staged-dirty classification of an unborn HEAD. Primary missing/not checked out
  are separate. Existing shared primary merge metadata makes **both** targets
  conflicted; an isolated primary index conflict tests PrimaryConflicted alone.
- D2/D3/D5 and ContextBehindRemote follow the approved relations and exception.
  Result selection is base tree / context tree / clean prepared merge. D1 parses
  the unique managed result document through the canonical-tree snapshot and
  validates its regular-blob mode, including duplicate/metadata-invalid cases;
  any `.manyhands/worktrees` entry is rejected.
- Prepared-tree import runs **only on a separate memory-ODB handle**. The same
  memory-backend protection surrounds the existing cleanliness helper's index
  tree verification. A real same-document nonoverlapping merge creates a new
  blob in memory: its exact OID is digest-bound, but it remains absent from the
  on-disk ODB. Full fixture Git/worktree/index/ref/object-file digests and all
  registry table rows remain unchanged, including with the repository lease held.
- Rename detection stays off; all added/modified/deleted paths are sorted,
  including comments, noncanonical files and document moves. Digest encoding uses
  the exact `manyhands promotion v1` domain, u64 big-endian length/count fields,
  fixed enum/option/cleanup tags, new OIDs (none for deletions), and local-only
  primary/context OIDs. Remote moving OIDs are deliberately excluded as approved
  D6; primary-only advances and context-primary merges preserve the digest.
- `repository.rs` declares/re-exports the API. Existing sync helper/type/field
  visibility widens to `pub(in crate::repository)` without behavior changes.
  Two necessary extra visibility-only lines are in `remote/mod.rs` (sync module
  reachable from its sibling) and `remote/state.rs` (pending-conflict query).
  No dependency, GPUI, global configuration, developer data, sleeps or native
  workflow change. All edits used apply_patch inside this ticket worktree.

### Exact RED / GREEN Commands And Results

The initial 38 tests and declaration-only preview/digest stubs preceded all
production observation/digest behavior. The second required-selector run was
runnable behavioral RED for every original test. Five additional edge/side-effect
cases were added during self-review; they confirmed existing behavior. A sixth
self-review case for a missing context branch supplied a further RED before its
cleanliness-order fix. Systematic debugging was used for fixture and test
assumptions; no existing sync behavior was changed to satisfy them.

| Exact command | Runs/results in execution order |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::observe` | **6 runs:** (1) **0 passed / 38 failed / 774 filtered**, fixture setup failed because enablement's configuration needed a fixture checkpoint (one unused-mut warning); (2) required behavioral **RED: 0 passed / 38 failed / 774 filtered**, every assertion reached the empty-blocker stub, no warnings; (3) implementation **32 passed / 6 failed / 774 filtered**, one unused-import warning; (4) **GREEN: 38 passed / 0 failed / 774 filtered**, no warnings; (5) expanded **43 passed / 0 failed / 774 filtered**, no warnings; (6) final **44 passed / 0 failed / 774 filtered**, no warnings, 8.82 seconds. |
| `devenv shell -- cargo test --locked --lib promotion::observe::tests::context_branch_missing_is_blocked_without_panicking -- --exact` | **1 run, 0 tests / 818 filtered**: exact selector omitted the `repository::` prefix; not counted as verification. |
| `devenv shell -- cargo test --locked --lib repository::promotion::observe::tests::context_branch_missing_is_blocked_without_panicking -- --exact` | **1 run, RED: 0 passed / 1 failed / 817 filtered**: observed `[ContextMissing, ContextDirty]`, expected only ContextMissing. Reference observation was moved before cleanliness; the final 44-test run above verifies the fix. |
| `devenv shell -- cargo fmt` | **2 runs**, exit 0. |
| `devenv shell -- cargo fmt --check` | **1 run**, exit 0 after the final formatting pass. |

The six intermediate failures were test/setup distinctions: an
`observed_published` history fixture must retain its last advertised OID to pass
the existing snapshot invariants; primary common-directory merge metadata is
shared by both targets, so the isolated-primary test now uses a stage-1 index
entry; and the registry preservation assertion now compares all actual table
rows rather than transient SQLite reader bookkeeping files. The unused import
was removed. The final side-effect cases still compare the complete Git/root
file map, all registry rows and zero accepted local TCP connections.

### Self-Review And Controller Handoff

- All 16 blocker categories have isolated coverage, with separate tracked,
  staged and untracked context dirtiness, ahead/diverged ancestry cases, D5
  positive/negative/contained exceptions, fixed multiple-blocker ordering,
  all tree-selection branches, sorted full-path/move changes, digest field and
  new-blob binding, local/remote OID policy and redaction. **44 unit tests added.**
- Self-review additionally checked unleased missing-ref classification,
  generated-object isolation, canonical duplicate/metadata validity, actual-ref
  authority over stale cache, typed accessor visibility and the visibility-only
  existing-code diff. No material approved-contract contradiction was found.
- Remaining gates: controller-owned independent spec/quality reviews, and the
  approved Task 8 full Rust/CLI gate and separately authorized native coverage.
  The local observation remains a snapshot, not authorization for a later write;
  future integration must freshly observe and compare consent under its lease.
- Evidence is this ledger and canonical ticket comment
  `01M5167Z00N4Q6S8V0X2Z4B6DP`. HEAD stays at the supplied Task 1 base. No commit,
  push, PR, CI dispatch, merge, ticket closure, cleanup or later-task work.

### Task 2 Spec Review Corrections — 2026-10-16

The controller returned two spec findings for verification and targeted TDD
correction. Both were confirmed against the approved contract and actual source
before changes. This supersedes the initial self-review's claim of complete
contract conformity; the initial execution evidence above remains historical.

| Finding | Verification | Resolution |
| --- | --- | --- |
| Preview/digest used the integrated document path instead of its context path | Cycle 09 lines 199–201 explicitly require the path **on the context**. `observe.rs` validated the result path and then assigned that path to the preview. A context at `docs/document.md` already contained by a primary that moved it to `docs/moved.md` reproduced the wrong value. | Result-tree path is now used solely for D1 validity. Preview and expectation use the discovered context document path. The regression asserts AlreadyContained, empty changes, the unchanged context path, digest equality with that path and inequality with the primary's moved path. The public accessor documents this meaning. |
| Valid executable regular documents were blocked by a poll-only mode restriction | Discovery `collect_source` accepts regular non-symlink files without checking executable permissions; canonical parsing has no mode input. Cycle 08 execution ledger lines 2904–2916 explicitly distinguish the poll-specific non-executable policy from discovery and the canonical-tree snapshot. Promotion's approved D1 requires canonical document validity, not a non-executable restriction. | Result validation accepts both regular blob modes `100644` and `100755`. New tests cover an executable context and an executable AlreadyContained primary result while the context remains `100644`. A negative test retains a valid document's bytes in a `120000` entry and proves symlinks remain invalid. Existing duplicate, invalid-metadata and forbidden-worktree tests remain GREEN. |

Each finding received a failing targeted regression before its production fix.
No owner amendment, existing discovery/poll behavior change, new dependency or
later-task work was needed. Only `promotion/observe.rs`,
`promotion/observe_tests.rs`, the context-path accessor documentation in
`promotion/mod.rs`, this ledger and the new canonical review-response comment
were changed in this correction pass.

| Exact command | Actual RED / GREEN results |
| --- | --- |
| `devenv shell -- cargo test --locked --lib repository::promotion::observe::tests::contained_primary_move_preserves_context_document_path_and_digest_binding -- --exact` | **2 runs:** RED **0 passed / 1 failed / 818 filtered**, actual `docs/moved.md` versus required `docs/document.md`; after the path fix, GREEN **1 passed / 0 failed / 818 filtered**. No warnings. |
| `devenv shell -- cargo test --locked --lib promotion::observe::tests::document_mode_` | **2 runs:** RED **1 passed / 2 failed / 819 filtered**, executable context and contained-result cases each incorrectly blocked by DocumentInvalidAfterIntegration, symlink rejection already passed; after accepting regular executable blobs, GREEN **3 passed / 0 failed / 819 filtered**. No warnings. |
| `devenv shell -- cargo fmt` | **1 correction-pass run**, exit 0. |
| `devenv shell -- cargo fmt --check` | **1 correction-pass run**, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion::observe` | **1 final correction-pass run: 48 passed / 0 failed / 774 filtered**, no warnings, 9.32 seconds. Includes all original 44 cases and four review regressions. |

Review-response comment: `01M5177Z00N4Q6S8V0X2Z4B6DQ`. Changes remain uncommitted
at base `3d47027` in the same ticket worktree, for the **same spec reviewer's
recheck** before quality review. No full Task 8 gate or native result is claimed;
no commit, outward lifecycle action, cleanup or other task was performed.

### Task 2 Quality Review — BLOCKER T2-1: Resulting Git Mode Not Bound (2026-10-16)

Historical pre-approval checkpoint. The owner approved the amendment after this
stop; see the resume section below for its implementation and verification.

**Status: independently characterized and CONFIRMED; STOP pending owner
encoding amendment.** The controller reports that Task 2 spec review passed
after its context-path and executable-document corrections. Those findings
remain resolved. Quality review then identified a possible material gap in the
approved encoding itself. This worker verified the contract/source and ran
temporary real-Git fixture probes before recording the blocker. No production
hash, policy or approved design was changed.

#### Contract And Source Evidence

- Approved design lines 248–258 explicitly encode sorted changes as **path,
  change, new blob OID**, with local primary/context OIDs added only without a
  remote. Lines 260–267 deliberately omit moving remote OIDs (D6), while stating
  that new context content will change the change set/digest (D4).
- `promotion/observe.rs:465–484` preserves only path, Added/Modified/Deleted and
  the new OID. A Git Typechange is normalized to Modified. Resulting Git mode
  and its type bits are discarded.
- `promotion/observe.rs:535–550` hashes only those retained change fields;
  `:557–560` includes primary/context OIDs only for local-only previews. A Git
  blob's OID binds its bytes, not whether its tree entry is regular, executable
  or a symlink. Mode/type changes can therefore collide in remote-backed consent
  even with a completely unchanged primary and a clean, valid managed document.
- This is a **material design encoding gap**, not permission to add fields
  silently: the implementation follows the explicit approved encoding. The
  executable-document spec correction must not be reverted to hide this issue;
  noncanonical paths are in the approved promotion change set too.

#### Temporary Characterization And Actual Results

Four `#[cfg(unix)]` probes used the existing `Fixture::new(true)`, real Git
commits, actual filesystem symlinks/chmod, and the public preview method. The
noncanonical path was `extra`, initially a regular `100644` file containing the
literal bytes `ordinary.txt`. A symlink to `ordinary.txt` has those same Git blob
bytes; an executable-bit change preserves them as well. The managed document was
untouched and remained valid.

For Modified cases, the fixture first committed `extra` with different bytes on
primary, updated actual `T(P)`, and based the context on that primary while
preserving its document edit. Primary was then frozen before both observations.
Added cases left `extra` absent from primary. These avoid a changing-primary
explanation or add/add merge conflict.

Every probe asserted, **before its final expected inequality**:

- both previews returned Ready and the context commit OID changed;
- exact primary HEAD OID, tree OID, index bytes, tracking-primary OID and root
  working-file digest map were unchanged (Git admin/context paths excluded from
  that working-file map and primary metadata asserted separately);
- `extra` retained the same path and category, with the **entire** preview change
  vector equal, plus equal primary/tracking OIDs, integration kind and context
  document path;
- the result-context entry had the intended changed mode and the same new blob
  OID **`5ef2f07219e9778bc697b47b11acb0d807837eb9`**, verified against a Git hash of
  `ordinary.txt`; both entry objects were blobs.

| Temporary test | Old → new mode | Category relative to unchanged primary | Final result |
| --- | --- | --- | --- |
| `t2_1_added_regular_to_symlink_changes_digest` | `100644` → `120000` | Added → Added | Expected `assert_ne!(expectation)` **FAILED**, digests equal |
| `t2_1_added_executable_bit_changes_digest` | `100644` → `100755` | Added → Added | Expected inequality **FAILED**, digests equal |
| `t2_1_modified_regular_to_symlink_changes_digest` | `100644` → `120000` | Modified → Modified | Expected inequality **FAILED**, digests equal |
| `t2_1_modified_executable_bit_changes_digest` | `100644` → `100755` | Modified → Modified | Expected inequality **FAILED**, digests equal |

All four emitted their completed precondition evidence before failing at the
same final digest inequality. There were no compilation/setup failures or
warnings. These are Linux/Unix characterization results, not native-platform
verification. No transport, process sleeps, global config or developer data was
used.

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::observe::tests::t2_1_ -- --nocapture` | **1 RED run: 0 passed / 4 failed / 822 filtered**, 0.77 seconds, exit 101. Every failure was the final `left != right` expectation assertion after all evidence assertions passed. |
| `devenv shell -- cargo test --locked --lib promotion::observe` | After removing only the temporary helper/four probes with apply_patch: **1 GREEN run: 48 passed / 0 failed / 774 filtered**, no warnings, 9.22 seconds. This confirms preservation of the existing suite, **not a fix for T2-1**. |

#### Proposed Owner Amendment — NOT APPROVED / NOT IMPLEMENTED

Recommended: bind the **resulting Git entry mode including type bits** for every
Added/Modified change, alongside path/category/new OID, in a specified fixed-width
encoding. Define an explicit deletion sentinel (for example reserved zero mode)
or an unambiguous absence tag for Deleted entries. Preserve D6's primary-only
invariance: bind the contributed result entry modes, not moving commit/tree OIDs
or unrelated primary-only paths. Add retained RED/GREEN regressions for both
mode transitions and both categories after approval, while retaining the existing
primary-advance/context-primary-merge invariance tests.

The owner must approve the encoding amendment and its consistent Cycle/design/
plan updates before production implementation. This recommendation is only a
proposal; the approved documents and production encoding remain unchanged.

#### Preservation And Handoff

- Removed only the temporary helper and four failing probes. All pre-existing
  Task 2 production/spec-fix WIP and its 48 tests remain intact. No intentionally
  failing tests remain in the working suite.
- Updated the fresh-session handoff: Task 0 complete, Task 1 reviewed/committed at
  `3d47027`, Task 2 uncommitted/spec passed/**quality BLOCKED T2-1**. Canonical
  blocker comment: `01M5187Z00N4Q6S8V0X2Z4B6DR`.
- Stop here pending owner decision; no Task 3, commit, push, PR, native dispatch,
  merge, ticket closure or worktree cleanup. Full Task 8 gates remain deferred.

### Task 2 Resume — Owner-Approved T2-1 Encoding Amendment (2026-10-16)

The owner approved the controller's T2-1 mode/type encoding recommendation
(**"yes"**, conveyed in the continuation instruction). Implementation authority
covers this amendment to the existing Task 2 WIP; the controller still owns the
checkpoint commit after review. The earlier context-path/executable-document
spec fixes and all 48 tests are preserved. Historical blocker and review evidence
above is unchanged apart from its explicit historical label.

**Preflight reused for this ongoing task:** the controller fetched `origin main`
this turn, still `89d04f901636229392a311241f8e777f0a00962b`, verified it is an
ancestor of HEAD `3d470272cbb78cbe614340dbdef56746bfdb85a5`, and inspected only
the expected WIP. The worker independently confirmed that HEAD/branch/status.
No dirty rebase, additional checkpoint or delegation was needed or performed.

#### Approved Encoding And Implementation

- Cycle, design and implementation plan now consistently record T2-1 as
  **approved**, including the exact byte placement and deletion sentinel.
- For every sorted path/category change, encode the existing optional new OID,
  **then exactly four bytes of u32 big-endian resulting Git entry mode including
  type bits**. Added/Modified read both OID and mode from the integrated result
  Git tree entry, including memory-ODB prepared results; neither comes from
  filesystem permission/type assumptions. Deleted has absent new OID and the
  explicit **zero u32** mode sentinel, not its old entry mode.
- `PromotionChangeEntry` is private ephemeral encoding evidence. The public
  preview/path-change DTO fields are unchanged. No dependency or other production
  module changed in this amendment pass.
- Keep domain **`manyhands promotion v1`**: no earlier digest has shipped, so
  this finalizes its unshipped encoding. Other field ordering/lengths/tags remain
  unchanged. Mode follows the OID before the next change or cleanup tags.
- D6 still excludes remote-backed moving commit/tree OIDs and unrelated primary
  paths. Existing primary-only advance and context-primary merge invariance is
  verified in the final targeted selector. Local-only OID binding remains intact.

#### Retained TDD Evidence

Restored and **retained** the four real-Git mode/type collision cases from the
historical probe, with the same unchanged-primary, same-blob, Ready-preview and
equal-path/category assertions. They now require distinct digests after the mode
transition. The Unix filesystem setup exercises actual symlink/chmod state;
production encoding reads the resulting Git tree, not those setup assumptions.

Added `t2_1_deletion_encoding_has_zero_big_endian_u32_mode`, an independent
two-change protocol vector (modified document plus deleted `ordinary.txt`). It
requires the regular mode bytes `[0,0,0x81,0xa4]`, an absent deletion OID followed
by `[0,0,0,0]`, and proves that omitting the deletion word or substituting the old
regular mode gives a different digest. Existing direct field-binding tests were
adapted to private entry evidence and now also vary only its mode; they retain
their previous field/new-blob/cleanup assertions.

| Exact command | Actual runs/results |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::observe::tests::t2_1_` | **2 runs:** before any production hash change, runnable **RED: 0 passed / 5 failed / 822 filtered**, no warnings, 0.89 seconds. All four collision cases failed at the final inequality; the byte-vector test failed because the old encoding omitted mode words. After implementation, **GREEN: 5 passed / 0 failed / 822 filtered**, no warnings, 0.93 seconds. |
| `devenv shell -- cargo fmt` | **1 amendment-pass run**, exit 0. |
| `devenv shell -- cargo fmt --check` | **1 amendment-pass run**, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion::observe` | **1 final amendment-pass run: 53 passed / 0 failed / 774 filtered**, no warnings, 11.60 seconds. All original 48 cases plus the five retained T2-1 tests pass, including deletion encoding, both spec fixes, primary-only/context-primary-merge invariance and read-only memory-ODB behavior. |

#### Self-Review And Return

Self-review checked tree-derived type bits/OIDs, matching sorted evidence and
public paths, Deleted's absent-entry sentinel, fixed-width endianness, unchanged
domain/public DTOs, preserved historical approvals and all current planning
documents' encoding consistency. No additional contract contradiction found.

Current handoff and canonical comment `01M5197Z00N4Q6S8V0X2Z4B6DS` record the
approved amendment, implementation, exact RED/GREEN and pending quality recheck.
**Ready for the same quality reviewer, not an independently approved Task 2
checkpoint.** HEAD remains the Task 1 commit; all Task 2 changes are uncommitted.
No Task 3, full-suite/Task 8 gate, dependency change, commit, push, PR, native CI,
merge, closure or cleanup was performed.

### Task 2 Final Independent Reviews — Local Gate Complete (2026-10-16)

The controller conveyed both final review results for the amended Task 2 code
and documents:

- **Same quality reviewer: PASS.** Approved T2-1 as resolved with **no critical,
  important or minor findings** remaining.
- **Same spec reviewer: PASS.** Rechecked the amended code/documents and confirmed
  the context-path and executable-document spec fixes remain intact.

**Task 2 is complete at the local gate; Task 3 is next after the controller's
checkpoint.** The approved mode/type amendment, earlier spec corrections and all
53 passing targeted tests are retained. Task 8 full local checks and separately
authorized native coverage remain deferred; these task reviews do not claim
those gates or completion of the whole Cycle.

This coordination-only pass updates the current ledger/handoff status, adds the
Task 2 plan completion summary and canonical comment
`01M51A7Z00N4Q6S8V0X2Z4B6DT`. The review results are controller-reported evidence;
no new reviewer test execution is inferred. **No source changes or test reruns**
were made. Prior exact RED/GREEN evidence remains the verification record.
No commit was made; the controller owns the immediate checkpoint. No Task 3 or
outward lifecycle action was performed. Historical blocked/pending-review
sections above are preserved as checkpoint history, superseded by this result.

## Task 3 Entry — BLOCKER T3-1: Confirmation Helper Requires A Sync Owner (2026-10-16)

**Historical entry, superseded by the Task 3 internal ruling/resume below.**

**Status: stopped before implementation; identity policy needs an explicit
decision. Task 3 is not complete or review-ready as an implementation.** Fresh
worker read this worktree's AGENTS, TDD and Cycle implementation skills, approved
Cycle/design/plan, Task 0 characterizations and Task 2 interfaces/evidence. Initial
status was clean on the exact ticket branch at
`6d4ad1664462c42c24b4a820024d76d7c86ca8db`. Controller's completed preflight is
reused; local `origin/main` remains
`89d04f901636229392a311241f8e777f0a00962b`, and its ancestry check passed. No
additional fetch/rebase, delegation or commit was performed.

### Required Helper Versus Actual Source

The Task 3 brief and approved plan require consuming `committing_identity`,
including observation-bound confirmed identity, with only minimal shared-helper
changes and no synchronization behavior change. Integration must also work
without a remote. Task 5 storage is expressly outside this task; only immutable
typed integration evidence/request inputs and a future persistence seam are
authorized now.

The existing helper is not a standalone identity selector:

- `remote/sync.rs:3214-3220`: it requires a real `RemoteReservation` and a
  `SynchronizeRemoteRequest`, besides the service/root/repository.
- `:3221-3239`: complete effective Git identity wins; otherwise absence of a
  confirmation returns IdentityRequired with the configuration digest. Empty/NUL
  identity fields or a stale digest return ExternalChange.
- `:3240-3273`: accepting a confirmation prepares a durable identity intent,
  records the effect boundary, writes repository-local `user.name`/`user.email`,
  and records the observed configuration digest. It does not merely use the
  supplied signature for one commit.
- `remote/reservation.rs:1734-1743,1896-1923`: these journal calls require an
  actually owned remote operation. `owned` (`:218-238`) validates service scope,
  repository, operation row, generation, owner epoch and active phase.
- `remote/state.rs:4303-4328`: preparing the identity intent first calls
  `synchronization_record`. At `:3153-3159`, that guard admits only
  SynchronizePrimary/SynchronizeContext, explicitly excluding Promote. Widening
  helper visibility or supplying a future promote reservation cannot fix this.

Consequently the local-only confirmed-identity path cannot call the specified
helper with just the authorized immutable Task 3 input. Reusing a completed
context child's owner would also fail ownership checks, and no context child
exists in local-only promotion. Creating a synthetic synchronization operation
or bypassing its journal would change lifecycle/ownership behavior, not helper
visibility. No such workaround was implemented.

### Decision Needed — Proposed, Not Approved

Recommended narrow amendment: extract the existing **read-only identity
selection/confirmation validation** into a shared helper. Keep synchronization's
current journaled repository-config effect unchanged around that selection.
Explicitly authorize promotion to use the validated confirmed identity for its
commit signature **without persisting Git identity config**. Preserve complete
effective-identity precedence, exact configuration-digest comparison and
empty/NUL rejection. This introduces no new promotion identity journal or
synthetic sync owner.

If promotion instead must persist identity configuration exactly as the existing
helper does, its durable ownership/intent/recovery boundary must be specified
and scheduled explicitly; the current Task 3 request-value seam does not provide
it. The approved contract was not silently amended in either direction.

### Verification, Self-Review And Return

- Static call-chain evidence above establishes the dependency/side effects; no
  runtime failure or failing integration test is claimed. No integration tests,
  stubs, production behavior or helper visibility edits were authored before
  resolving this policy boundary.
- Required RED command `devenv shell -- cargo test --locked --lib
  promotion::integrate`: **not run**; the module does not yet exist. No GREEN or
  Rust verification is claimed. Tasks 0–2's recorded test evidence is unchanged.
- Self-review distinguishes this confirmed helper incompatibility from an
  approved identity-policy choice. Integration-base, digest, checkout and
  reconciliation tests remain Task 3 obligations after the decision.
- `git diff --check` and `git diff --exit-code -- src tests Cargo.toml
  Cargo.lock devenv.nix devenv.lock` both exited 0. Status showed only this
  ledger and the new canonical comment; HEAD remains the supplied Task 2 base.
- Durable changes are this ledger and canonical ticket comment
  `01M51B7Z00N4Q6S8V0X2Z4B6DV`. Return uncommitted for controller/owner review of
  T3-1, not implementation spec/quality approval. Task 8/full/native gates remain
  deferred; the ticket remains open.

## Task 3 Resume — Internal Ruling And Primary Integration (2026-10-16)

**Status: implementation and targeted verification complete; uncommitted and
ready for controller-owned independent spec review, then quality review.** HEAD
remains `6d4ad1664462c42c24b4a820024d76d7c86ca8db`. The exact ticket
worktree/branch and completed controller preflight were retained. No subagents
were used. The initial T3-1 ledger/comment are preserved as historical evidence,
not the current decision or a remaining owner-approval gate.

### T3-1 Superseding Internal Engineering Ruling

The controller conveyed independent authority/source review: **T3-1 is helper
composition, not an unsettled owner policy.** Git workflow RFC Identity
(`:69-73`, reiterated `:362-363`) and MVP RFC Identity (`:483-488`) already require
effective identity first and, when missing, explicit confirmation followed by
repository-local config persistence before committing. The historical
commit-signature-only proposal conflicts with these authorities and is
**rejected/superseded**, not implemented. No owner amendment is needed.

Source review also identified the already-approved journal-free atomic bridge
(`bridges/identity.rs` original module contract `:10-47`, original implementation
`:529-620`; F2 design `:3401-3503`). Its public setter's own lease, observation
and stricter validation are not reusable as promotion policy. Extracting its
lease-held atomic primitive and the synchronization helper's read-only selection
is permitted by the approved shared-helper extraction rule.

Implemented ruling:

- Complete effective identity wins, ignoring supplied confirmation. Missing and
  unconfirmed returns IdentityRequired without writing. Otherwise compare the
  exact effective name/email configuration digest and apply the existing
  empty/NUL rejection, without bridge-specific email/whitespace rules.
- Check context/primary evidence, blockers, consent and remote base ancestry
  before accepting any config write. AlreadyContained needs no identity.
- Under the integration lease and Git config lock, freshly re-observe effective
  identity/confirmation before editing and immediately before installation.
  Install both local identity keys by one atomic rename; preserve initials and
  unrelated settings. A foreign config lock is refused and never removed.
- Config is its own atomic recovery evidence; no promotion identity journal,
  record table or synthetic sync owner is introduced. Installed config stays
  installed if a later integration step fails. Error means **primary** unchanged,
  not identity-config rollback.
- Synchronization retains its current confirmation intent/effect journal and
  config writes unchanged around the extracted read-only selection.
- Narrow design/Task 3 plan wording now describes this composition and the
  immutable evidence/candidate boundary; historical recommendations remain in
  this ledger and comment `01M51B7Z00N4Q6S8V0X2Z4B6DV` only.

### Delivered Interfaces And Boundaries

- New `promotion/integrate.rs`: `integrate(service, root, &IntegrationRequest,
  Option<&ConfirmedCommitIdentity>) -> Result<Integrated, DocumentPromotionError>`.
  `IntegrationRequest::from_preview` captures consent and post-sync observation;
  the future runner can also reconstruct these internal typed values from its
  record. This is a fresh-attempt primitive; an integrating replay reconciles
  persisted evidence before deciding to prepare again.
- `prepare_integration` takes exactly one repository lease, compares old local
  primary and synchronized context, and calls Task 2's read-only observation
  afresh under it. Remote primary/context names, frozen remote and OIDs are
  checked; digest is recomputed. D2 (local primary ≤ stored tracking primary)
  is enforced by observation; remote merge additionally requires base ≤ context
  before object/config effects. Poll advance returns PrimaryAdvanced with no
  object/ref/index/worktree writes. Integration conflicts require reconfirmation.
- `PreparedIntegration` owns the lease and exposes immutable `IntegrationEvidence`
  (base, item/context/primary branch, old local primary, integration kind). The
  runner must persist **integrating before `write_commit`**. That method returns
  a lease-owned `IntegrationCandidate` with a commit object but no ref effect
  (F3). `apply` uses the existing safe/ref-locked `fast_forward` for one old-local-
  primary → merge update; the returned outcome is F4's persistence seam. No
  Task 5 storage, migrations or F3/F4 failure-point stubs were added.
- Trees use context when base ≤ context, or the imported in-memory prepared
  merge for local divergence. Exact commit subject is `Promote document <ULID>`,
  author/committer use the resolved identity and parents are ordered [base,
  context]. AlreadyContained returns the base and writes no commit/ref.
- Ignored path/ancestor collisions are checked through an in-memory result tree
  before importing objects or writing a commit. Final application still uses
  existing safe checkout with ignored overwrites disabled. No sync algorithm
  was rewritten.
- `reconcile_integration(repository, base, &IntegrationContext) -> IntegrationState`
  walks **first parents**, matching exact subject and exactly [base, context].
  A merge at tip or behind later FF/sync commits returns Integrated; no match
  with local primary ≤ base returns NotIntegrated, ignoring dangling objects.
  Missing/uncertain/other history returns the **pending** RecoveryRequired state,
  never an operation error. A fast-forward error is reconciled before returning;
  an applied checkout without a successful ref update also returns pending
  recovery, with no rollback. Deterministic hooks verify both post-effect cases.
- `DocumentPromotionError` defines only current integration refusals plus
  IdentityRequired/Repository; Debug/Display emit fixed categories (Repository
  Debug shows only its kind). Remaining lifecycle variants/re-exports belong to
  Task 6. Internal stage dead-code allowance is scoped to this as-yet-uncomposed
  module and should disappear with its runner.
- `LocalConfigEdit` extracts Git lock/private-copy/source-compare/atomic-install
  ownership from the identity bridge. The bridge retains its own validation,
  expected observation, idempotence and failure checkpoint. Config handles drop
  before the private scratch directory (Windows-compatible ownership order).
- Shared helpers retain repository-only visibility. The real-SSH source-included
  target nests the library under `production`; absolute restricted paths from
  Task 2 and the initial extraction failed to compile there. Equivalent
  `pub(in super::super)` scope fixes those declarations in sync/state/identity
  without widening their audience or changing behavior. `fast_forward` alone
  additionally widens from remote-only to repository-only as required by Task 3.

### Exact RED / GREEN And Regression Evidence

All original 23 integration tests were written before production behavior.
Declaration-only panic stubs enabled runnable RED, then were fully replaced.
The privacy regression was added and observed RED before custom error formatting.
The four later tests extend self-review coverage of post-effect reconciliation,
D2 and config-lock re-observation; they did not require a behavior fix. All
commands ran in the exact ticket worktree through Devenv. No full Task 8 gate ran.

| Exact command | Actual runs/results, in execution order for that command |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::integrate` | **7 runs:** (1) compile RED, 45 missing-API/test-API diagnostics, one unused-import warning, no tests; (2) **0 passed / 23 failed / 827 filtered**, 22 stub failures and one collision-fixture PrimaryDirty setup error, one unused-import warning; (3) corrected runnable **RED: 0 passed / 23 failed / 827 filtered**, all at the missing behavior stubs, no warnings; (4) first implementation **23 passed / 1 failed / 827 filtered**, ignored-collision preservation detected one dangling commit; (5) **GREEN: 24 passed / 0 failed / 827 filtered** after the collision pre-check; (6) expanded **27 passed / 0 failed / 827 filtered** including post-effect hooks and D2; (7) final **28 passed / 0 failed / 827 filtered**, no warnings, 5.43 seconds, after config-lock concurrency coverage, evidence/checkpoint assertions and formatting. |
| `devenv shell -- cargo test --locked --lib promotion::integrate::tests::integration_errors_redact` | **1 RED run: 0 passed / 1 failed / 850 filtered**; derived Repository error Debug exposed sentinel path/message. One transient unused Confirmed payload warning while extraction was not yet consumed; final promotion code consumes it. Fixed custom formatting, verified by final GREEN. |
| `devenv shell -- cargo test --locked --lib promotion::integrate::tests::ignored_checkout_collision` | **1 diagnostic RED: 0 passed / 1 failed / 850 filtered**, narrowed file-image diagnostics showed exactly one new `.git/objects/...` commit; primary/ref/index/worktrees unchanged. |
| `devenv shell -- cargo test --locked --lib promotion` | **1 GREEN run: 80 passed / 0 failed / 774 filtered**, no warnings, 16.84 seconds. All 53 approved observation/T2-1 tests plus the then-current 27 integration tests. The additional config-lock test is covered by the final 28-test integration run above. |
| `devenv shell -- cargo test --locked --test mutation_bridges identity::` | **1 GREEN run: 42 passed / 0 failed / 87 filtered**, no warnings. Existing atomic/name-email/initials/idempotence/validation/lock/link/mode/privacy/boundary regressions unchanged. |
| `devenv shell -- cargo test --locked --test mutation_bridges identity::placement::` | **1 final GREEN run: 5 passed / 0 failed / 124 filtered**, no warnings, after making config-before-directory drop order explicit. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery confirmed_identity_completes_a_divergent_primary_merge -- --exact` | **2 runs:** first compile failure, 19 restricted-visibility path diagnostics in the source-included library (including Task 2 declarations); equivalent relative repository scope fixed it. Final **1 SSH case passed**, no warnings, exercising the existing sync confirmation/config-persistence/journal path. |
| `devenv shell -- cargo fmt` | **2 runs**, exit 0. |
| `devenv shell -- cargo fmt --check` | **2 runs**, exit 0. |

Systematic debugging traced the initial collision fixture's PrimaryDirty result
to overwriting `info/exclude` and thereby removing the fixture's worktree ignore;
the fixture now appends its collision rule. The later dangling-object failure
was separate: git2 0.20.4 `CheckoutBuilder::dry_run` selects CHECKOUT_NONE, while
libgit2's ignored conflict action is skipped under NONE (`build.rs:350-353`,
`checkout.c:500-513`). The final read-only ignored-path check avoids relying on
that dry-run behavior. Preservation snapshots include all fixture Git object,
ref, reflog, index, registration and working-file content hashes; no production
algorithm/dependency workaround was used. Tests never use sleeps, developer
keys or global config writes.

### Self-Review And Controller Return

Self-review checked every original Task 3 case against the tests: poll-style
base advance writes no objects/ref and keeps full images; leased digest mismatch
precedes identity persistence; remote parent order/tree/behind-local-primary,
local generated-blob import and canonical validity; contained/no identity;
missing/confirmed/stale/empty/NUL/effective-wins identity; atomic/no-half-write,
unrelated settings/initials and foreign locks; changed primary/context; ignored
collision; first-parent reachable/dangling/no-match/uncertain reconciliation;
and post-effect pending outcomes. The seam test proves one held lease, no object
before the future integrating checkpoint and a usable registry transaction after
observation. The new config-race test covers both stale partial identity and a
complete effective identity appearing under the config lock, with foreign bytes
preserved. Debug redaction is explicit.

No unresolved product-policy contradiction remains. Native Windows execution
and the full Rust/CLI checks remain Task 8/later authorization gates; Linux
targeted success is not native proof. Task 5 must wire durable integrating
evidence before `write_commit`; Task 6 must map pending integration recovery to
the approved partial `Ok` outcome, reconcile integrating replay before preparing
again, and compose the rest of the lifecycle. These are the planned downstream
obligations, not stubbed Task 3 behavior.

Current canonical checkpoint comment: `01M51C7Z00N4Q6S8V0X2Z4B6DW`. All changes
remain uncommitted for the controller's independent **spec then quality review**;
the controller owns the authorized checkpoint commit. No push, PR, native CI,
merge, closure, cleanup, dependency or global configuration change was performed.

Final inventory checks: `git diff --check` exited 0; `git diff --exit-code --
Cargo.toml Cargo.lock devenv.nix devenv.lock .github/workflows/build.yml` exited 0.
Status contains the two new integration files, promotion module, identity bridge,
sync/state helper changes, three plan/design/ledger documents and the historical
plus current canonical comments (11 intended paths). HEAD is still the supplied
Task 2 base. No implementation stub remains in `integrate.rs`.

## Task 3 Quality Review Correction — Q3-1: Proposed Identity Include Context (2026-10-16)

**Status: quality finding verified and fixed with targeted TDD; uncommitted for
the same quality reviewer's recheck.** The controller conveyed **SPEC PASS** and
one important quality finding. No independent quality approval is claimed by
this implementer. Base remains `6d4ad1664462c42c24b4a820024d76d7c86ca8db` in the
same exact ticket branch/worktree. All prior uncommitted Task 3 changes and
historical evidence are retained.

### Verification Of Finding

The reviewer supplied a concrete include-resolution counterexample. Verified:
`LocalConfigEdit` edits a copy at `.git/manyhands-identity-.../config`, so an
ordinary relative include is resolved from the wrong directory there. The
pre-install `select()` in promotion re-read the **original current config**,
which is still missing identity; it did not establish what the **proposed**
config would mean after installation. The public identity bridge's rejection
of included/ambiguous identity meant its existing tests did not exercise this
new promotion use.

The retained regression uses original empty local name/email followed by
`[include] path = identity.inc`; `.git/identity.inc` holds an empty user.email.
With a valid confirmation the original implementation returned Merged, although
the installed real include overrode the confirmed email. A second regression
established the same problem for an active onbranch conditional include with a
nested relative include. Both were observed failing before any repair.

### Narrow Repair And Invariants

- Added `LocalConfigEdit::would_persist_identity`, called by promotion under its
  existing repository lease and config lock immediately before `install`.
  Missing/confirmed selection and validation remain the same shared sync policy.
- Keep the existing protected scratch directory and atomic installation. Moving
  the edited file beside the original would fix ordinary relative paths alone:
  standalone git2 Config parsing lacks repository context for conditional
  includes (`config_file.c:721-728`). No scratch relocation, include-path rewrite,
  external Git process, new dependency or new identity validation policy is used.
- Snapshot libgit2's effective configuration in the **original repository
  context**, and read its local backend's ordered root/included entries. Merge
  the actual proposed root name/email fields/order with that native include
  expansion. Every other proposed root field/directive must exactly match;
  otherwise refuse the unprovable edit. Git evaluates the relative, nested,
  onbranch and gitdir includes; the new code does not parse include conditions.
- Account for unchanged higher-priority worktree/app backend identity values.
  Accept only when the proposed effective name/email exactly equal the selected
  confirmed values. A mismatch refuses before config replacement or tree/commit
  creation, keeping config, primary and all fixture file images unchanged.
  Includes that do not override the proposed identity still work: no blanket
  rejection of includes or included identity was added.
- Factor the existing source-byte comparison into `require_unchanged_source` so
  proposal checking and final atomic installation both detect a noncooperating
  writer. The public bridge still uses its original observation/validation and
  installation path; it does not invoke the promotion-specific proposal check.

### Exact RED / GREEN And Regression Evidence

Three focused tests were added before the repair. Besides the supplied ordinary
relative include, they cover active/inactive onbranch includes, an exact active
gitdir condition, nested relative includes, and positive cases where include
data is unrelated or overridden by the final local user section. Accepted cases
read the actual installed effective identity through a fresh repository handle;
refused cases compare the complete Git/worktree/config file image.

| Exact command | Correction-pass runs/results |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::integrate::tests::proposed_identity_` | **4 runs:** initial RED **0 passed / 3 failed / 855 filtered** (ordinary/conditional bugs reproduced; positive fixture also had an unintended empty earlier user section); corrected runnable RED **1 passed / 2 failed / 855 filtered**, both failures were erroneous Merged outcomes; first repair **2 passed / 1 failed / 855 filtered**, reaching a previously unexecuted gitdir-fixture activation assertion; after correcting the exact gitdir pattern, **GREEN: 3 passed / 0 failed / 855 filtered**, 1.09 seconds. No warnings. |
| `devenv shell -- cargo test --locked --lib promotion::integrate` | **1 final GREEN: 31 passed / 0 failed / 827 filtered**, no warnings, 6.39 seconds. All previous 28 Task 3 tests plus the three quality regressions. |
| `devenv shell -- cargo test --locked --test mutation_bridges identity::` | **1 GREEN: 42 passed / 0 failed / 87 filtered**, no warnings, 3.26 seconds. The shared source-compare factoring and public bridge's existing atomic/ambiguous/include/placement/privacy behavior are covered without running unrelated bridge tests. |
| `devenv shell -- cargo fmt` | **1 correction-pass run**, exit 0. |
| `devenv shell -- cargo fmt --check` | **1 correction-pass run**, exit 0. |

Systematic debugging separated both fixture issues from the production bug:
libgit2 writes into the first existing user section, so the positive fixture now
has exactly one user section at the intended side of the include. The gitdir
fixture originally used a trailing slash, which means a directory-prefix match
rather than the exact Git directory; the exact condition now omits that slash,
and its active marker is asserted before attempting promotion. Neither required
a production policy change. No full/local/native gate or unrelated SSH suite
was rerun for this correction.

### Self-Review And Return

Checked ordered native include expansion against actual proposed root-entry
placement, all non-identity root-field equality, nested include depth, active and
inactive repository conditions, higher-backend priority, raw optional values
(including valueless entries), preserved atomic/source-change checks and fixed
redacted errors. Complete effective identity still bypasses confirmation/editing;
missing identity still writes nothing; confirmed config from an accepted earlier
step still follows the existing no-rollback rule. The new pre-install refusal
adds no public policy or journal.

This correction changes only `bridges/identity.rs`, `promotion/integrate.rs`,
`promotion/integrate_tests.rs`, this ledger and canonical comment
`01M51D7Z00N4Q6S8V0X2Z4B6DX`, on top of the prior uncommitted Task 3 work. Ready
for the **same quality reviewer** to verify Q3-1 resolved. No commit, subagent,
push, PR, CI, merge, closure, cleanup, dependency or global-config action was
performed. Full Task 8/native checks remain deferred.

Correction inventory checks: `git diff --check` exited 0, and `git diff
--exit-code -- Cargo.toml Cargo.lock devenv.nix devenv.lock
.github/workflows/build.yml` exited 0. HEAD is unchanged at the supplied Task 2
base; the current uncommitted Task 3 inventory adds only the quality-response
comment to the previous path list.

## Task 3 Final Independent Reviews — Local Gate Complete (2026-10-16)

The controller conveyed the final review results for the current uncommitted
Task 3 change set, including the Q3-1 include-context repair:

- **Spec review: PASS**, previously reported and retained.
- **Same quality reviewer: APPROVED. Q3-1 resolved; no findings remain.**

**Task 3 is reviewed complete at the local gate. Task 4 is next after the
controller's checkpoint.** Exact review base and unchanged HEAD are
`6d4ad1664462c42c24b4a820024d76d7c86ca8db` (`6d4ad16`). The review boundary is
the full current uncommitted Task 3 source/documentation change set against that
base, including all retained tests and the quality correction; no new commit or
commit range is implied. The controller owns explicit staging and the checkpoint
commit.

T3-1 remains a **resolved internal engineering ruling**, with RFC-compliant
atomic repository-local identity persistence before commit. There is no pending
owner-policy question. Its historical commit-only suggestion is preserved but
superseded. Q3-1's root-context include validation and regression evidence are
retained unchanged. Previous blocked/review-ready entries are historical states,
superseded by these final review results.

This coordination-only pass updates the handoff to Task 3 complete/Task 4 next,
adds the plan's Task 3 completion summary, records this ledger status and adds
canonical comment `01M51E7Z00N4Q6S8V0X2Z4B6DY`. **No source changes or Rust test
reruns** were made. Verification remains the recorded final 31 passing integration
tests, three passing focused include regressions, 42 passing shared bridge
identity regressions and prior observation/synchronization evidence. Review
approvals are controller-reported; no additional reviewer test execution is
inferred.

The full Task 8/local/native gates and later cleanup/storage/runner work remain
due. This is Task 3 completion, not completion or closure of the Cycle. No commit,
Task 4 implementation or outward lifecycle action was performed by this status
worker; the ticket stays open.

Coordination verification: `git diff --check` exited 0. `git status --short
--untracked-files=all` lists 14 intended Task 3 paths (six source files, four
planning documents and four canonical comments) for the controller's explicit
staging. No files were staged and HEAD remains the exact review base above.

## Task 4 Entry — BLOCKER T4-1: Git/SQLite Completion Atomicity (2026-10-16)

**Historical entry status (superseded by owner-approved resume below): Task 4
STOP at T4-1; owner decision pending. Independent SPEC
confirmed a material contradiction requiring an owner amendment; QUALITY approved
the blocker evidence without findings. These are read-only evidence reviews, not
implementation approval; no reviewer reruns occurred.** The task brief expressly
requires a stop on a material approved-contract contradiction rather than
silently weakening all-three failure preservation.

### Entry And Contract

- Reused the controller's completed preflight. Read this worktree's AGENTS, TDD
  and Cycle implementation skills, approved Cycle/design/plan and prior evidence.
  Initial status was clean on the exact ticket branch. HEAD is
  `3b925e141bec9ae309024bdbfbf73aa6ab48caae`; `origin/main` is
  `89d04f901636229392a311241f8e777f0a00962b`. The ancestry check exited 0.
  Tasks 0–3 and their reviews remain intact. No fetch/rebase was repeated.
- Task 4 plan `:401-403,421-425` requires tracking-ref deletion, state-row deletion
  and retirement insertion in one transaction, with an injected transaction
  failure leaving **all three unchanged**. Design `:389-390,570-576` also puts
  completion and reservation finish in that final transaction; Cycle
  `:300-304` requires the same boundary.
- `remote/state.rs:5556-5591` supplies a **SQLite-only** transaction. Its contract
  explicitly says it is never held across Git work; the callback runs before
  `transaction.commit()`. The requested `retire_context(tx, ...)` receives a
  borrowed SQL transaction, so it cannot decide whether its caller later rolls
  back or whether that later SQL commit succeeds.
- Git ref transactions commit separately (`remote/sync.rs:7371-7377,7405-7410`).
  No existing enlistment mechanism makes Git refs part of SQLite's rollback or
  commit. A repository lease and remote reservation exclude cooperating actors;
  they do not merge those two durability domains. Existing poll code explicitly
  treats a surviving Git effect as independent of SQL rollback
  (`remote/poll.rs:1791-1807`).

### Real-Git/SQLite Characterization

A throwaway test in `remote/state_tests.rs` used two isolated existing state
fixtures, actual Git repositories/commit objects/tracking refs, actual
`configure`/`complete_batch`/`with_transaction`, and a test-only SQL table named
`task4_retirement_probe`. That table represented the SQL retirement effect; it
was **not** a production `retired_contexts` migration or a test of a delivered
`retire_context` implementation. No mock, dependency, global config, developer
key, network or sleep was used.

1. Seed a real tracking ref at the fixture commit OID and a real
   `observed_published` context-state row. Lock the ref and stage its removal.
2. **Git-first scenario:** inside `with_transaction`, delete the state row,
   commit the Git ref transaction, then attempt the retirement-effect INSERT.
   A deterministic SQLite `BEFORE INSERT` trigger raises `ABORT`; the closure
   returns the existing fixed RecoveryRequired error. SQL rollback restores the
   original state row and leaves zero retirement-effect rows, but a newly opened
   Git repository reports the tracking ref **NotFound**. The three resources are
   not preserved together.
3. **Unapplied-Git scenario:** stage and then drop the Git ref transaction without
   committing; commit the SQL deletion/insertion through `with_transaction`.
   The reopened Git ref remains at the original OID, while SQL has zero state
   rows and one retirement-effect row. This deterministically represents the
   unapplied Git side of a SQL-first protocol; it does **not** claim an actual
   process kill was executed.

Executing all SQL statements before committing Git can avoid the first trigger
ordering, but cannot make a subsequent SQL commit failure/outer rollback undo a
committed Git deletion. Committing SQL before Git has the opposite gap. A
best-effort compensating ref write is a further fallible Git effect, not an
atomic rollback. These observations establish the missing cross-resource
mechanism; they do not claim that every possible recovery protocol was tested.

### Exact Commands And Counts

All Rust commands used Devenv in the specified ticket worktree. The first two
selectors were incorrectly qualified for `--exact`: the actual nested test
module is `persistence_tests`, not `tests`. Their zero-test results are explicitly
not verification.

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::state::tests::task4_retirement_cross_resource_characterization -- --exact` | **0 passed / 0 failed / 859 filtered**; no test selected. |
| `devenv shell -- cargo test --locked --lib repository::remote::state::tests::task4_retirement_cross_resource_characterization -- --exact` | **0 passed / 0 failed / 859 filtered**; no test selected. |
| `devenv shell -- cargo test --locked --lib task4_retirement_cross_resource_characterization` | **1 passed / 0 failed / 858 filtered**, 0.15 seconds, no warnings; both real-resource scenarios above. This is characterization evidence, not Task 4 implementation GREEN. |

The planned `cleanup`, `remote::state` and `remote::refs` implementation RED/GREEN
runs have **not** run. No stubs or production behavior were written. The blocker
was found before starting the implementation TDD cycle, and the passing probe
must not be reported as a passing Task 4 gate. Full Task 8 checks remain deferred.

### Required Resolution And Self-Review

Controller/contract review must specify an achievable completion durability
boundary before Task 4 resumes: either supply a real mechanism satisfying the
literal cross-resource atomicity/failure guarantee, or explicitly amend it to a
recoverable ordered Git effect plus atomic SQL finalization. An amendment must
define pre-effect failures, post-Git/pre-SQL failure and crash reconciliation,
and how applicable existing publication history/reservation ownership remain
until completion.
That alternative is **proposed, not approved**; the all-three preservation rule
has not been relaxed. No Task 5/6 journal or runner was implemented to bridge it.

Self-review: the contradiction is the final cross-resource boundary, not D7,
R1's missing-row semantics or the clean-worktree design. Those implementation
obligations remain pending, including generic ticket/document coverage,
migration/audit/inventory, ignored/locked/partial preservation and fresh Edit.
The original Linux prune characterizations are retained; Windows open-handle
behavior remains unverified. No claim of native execution is made.

Removed the entire throwaway test using `apply_patch`. `git diff --exit-code --
src tests Cargo.toml Cargo.lock devenv.nix devenv.lock` exited 0 after removal.
At the initial blocker return, durable changes were only this ledger and canonical
comment `01M51F7Z00N4Q6S8V0X2Z4B6DZ`, returned uncommitted. The ticket stays open. No
subagents, staging, commit, push, PR, CI dispatch, merge, closure or worktree
cleanup occurred.

Initial blocker-return verification: `git diff --check` and `git diff --exit-code
-- src tests Cargo.toml Cargo.lock devenv.nix devenv.lock` both exited 0. `git status --short
--untracked-files=all` listed exactly the modified ledger and new canonical
comment above. No source/dependency changes or staged files remain.

### Independent Reviews And Proposed Minimal Amendment — Owner Decision Pending

The controller conveyed the completed independent reviews:

- **SPEC:** confirmed T4-1 is a material approved-contract contradiction and
  requires an **owner amendment** before implementation resumes.
- **QUALITY:** approved the blocker evidence **without findings**.
- Both reviewers worked read-only over the blocker evidence/current diff and
  relevant source. No reviewer test reruns or code changes occurred. Prior probe
  counts remain the evidence; neither review clears Task 4 for implementation.

Task 3 is committed as `3b925e141bec9ae309024bdbfbf73aa6ab48caae` (`3b925e1`),
the unchanged current HEAD. Tasks 0–3 remain reviewed complete at their local
gates. **Task 4 remains STOP at T4-1; the owner has not approved an amendment.**

**Proposed minimal amendment — NOT APPROVED:**

1. Persist the existing planned **`branch_deleted` intent** durably before the
   tracking-ref effect. No new phase or table is proposed.
2. Under the repository lease, validate ownership and frozen configuration, then
   delete the tracking ref guarded by the expected **direct OID**, or proceed
   only on verified true absence. Wrong/new conflicting values, symbolic refs
   (including dangling symbolic refs) and unreadable refs stop cleanup; none is
   interpreted as eligible absence.
3. After the guarded Git effect, perform **one SQL transaction** that deletes
   publication history, inserts `retired_contexts`, marks the promotion complete
   and finishes the applicable reservation. Git deletion is a separate durable
   effect, not part of this SQL transaction's rollback guarantee.
4. In the split interval, the tracking ref may be absent while applicable
   **existing** SQL history stays intact (`observed_published` when present), no
   retirement exists, the durable promotion record remains pending and the
   applicable reservation stays held. Do not manufacture `observed_published`
   history for local-only or cache-loss cases. SQL statement failure followed by
   rollback of the uncommitted transaction preserves SQL state, **not an
   already-deleted Git ref**. An uncertain commit result does not prove SQL is
   unchanged: reopen and reconcile actual completion. If completion and applicable
   reservation finish committed, replay the stored outcome. Git deletion may
   remain whether SQL rolled back or committed. Do not reset history or grant
   first-publication eligibility early.
5. Failures after integration return **`Ok` with cleanup pending
   `RecoveryRequired` and refresh**, rather than a post-integration error. A
   same-ID retry rechecks actual refs, ownership and configuration and can finish
   SQL finalization without repeating completed lifecycle work, or replay the
   stored outcome when actual completion already committed. **Cache loss or
   ref absence alone is not retirement proof**; the retained durable intent and
   required actual checks are essential.
6. Add tests for the newly explicit boundaries **inside F10/F11**, including SQL
   statement failure and SQL commit failure/uncertainty, split-interval existing
   history/retirement/pending-record/reservation state, same-ID reconciliation, and wrong/conflicting/
   symbolic/unreadable ref refusals. Cover reopening/reconciling uncertain commit
   results and replay of committed completion/reservation finish. Test rollback
   of uncommitted SQL separately from the surviving Git deletion; do not retain
   the impossible all-three rollback assertion as a claim about this proposed protocol.

This proposal is recorded for owner decision only. The approved Cycle/design/
implementation plan have not been amended, and no production, schema, test,
phase or recovery protocol change was made. The handoff and canonical comment
now carry the actual review status and proposal. No tests were rerun for this
documentation-only update. The controller owns the pending checkpoint of the
reviewed blocker evidence; this worker makes no commit.

Review-status update verification: `git diff --check` and `git diff --exit-code
-- src tests Cargo.toml Cargo.lock devenv.nix devenv.lock` exited 0. `git status
--short --untracked-files=all` lists exactly three pending paths: this ledger,
`docs/plans/2026-10-15-wave-02-cycle-09-handoff.md`, and canonical comment
`01M51F7Z00N4Q6S8V0X2Z4B6DZ`. No code/test changes, reruns or staged files.

## Task 4 Resume — Owner-Approved T4-1 (2026-10-16)

The owner approved (“yes”) the controller's final Git-first amendment, including
the uncertain-SQL-commit and applicable-existing-history qualifications above.
Historical blocker/reviews/proposal are preserved; their STOP/NOT APPROVED status
is superseded. Full original Task 4 scope is authorized now. Cycle/design/plan
and handoff are being updated consistently before source implementation.

Controller fresh preflight: clean fetch/rebase onto origin/main
`89d04f901636229392a311241f8e777f0a00962b`; no-op rebase and ancestry passed at
`79463c53beb400d95a311e558e64caf8efd2beac` (reviewed blocker evidence). Worker
confirmed clean entry, exact branch/SHAs and reread AGENTS/TDD. No extra rebase,
subagents or commits. Targeted tests only until Task 8. Task 4 exposes separate
guarded Git deletion and private validated evidence for SQL retirement; Task 5/6
will compose durable branch_deleted intent, completion and reservation finish.
No synthetic promotion storage/phase is authorized or needed here.

### Task 4 RED — before production behavior

All eight new cleanup tests and five state tests plus the refs real-SQL/Git test
were authored first. Declaration-only unimplemented cleanup stubs enable runnable
RED; they are not implementation. Inventory expectation now includes retirement.

| Exact command | Actual result before implementation |
| --- | --- |
| `devenv shell -- cargo test --locked --lib cleanup` | First compile RED: 25 missing cleanup API diagnostics, one unused-import warning, no tests. After declaration-only stubs: **8 passed / 8 failed / 856 filtered**; all eight new tests stop at the missing cleanup behavior; existing matching tests passed. No warnings in runnable RED. |
| `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::retirement` | **0 passed / 5 failed / 867 filtered**, missing retired_contexts schema. |
| `devenv shell -- cargo test --locked --lib remote::refs::tests::retired_context` | **0 passed / 1 failed / 871 filtered**, missing retired_contexts schema. |

The cleanup matrices cover both kinds, full Git images on refusals, ignored/
locked preservation, partial/mixed recovery, other worktrees, branch OID and
all-checkout guards, Git-first split state/SQL trigger rollback/retry, tracking
conflicts/symbolic/dangling/unreadable refs and fresh prepare_context(Edit).
State/refs tests cover additive atomic/idempotent/audited migration, schema loss,
orphan/corrupt rows, R1-only eligibility under unknown history, unchanged invalid
never-published-row invariant, observation absence and atomic retirement removal.

### Delivered Task 4 Interfaces / Decisions

- `cleanup::ContextCleanup::acquire(root)` owns the actual repository lease.
  `precheck_context(kind,id,oid)`, `remove_context_worktree(kind,id,oid)` and
  `delete_context_branch(kind,id,oid)` are generic for document/ticket reuse by
  Cycle 10. Precheck uses the existing canonical/materialization/clean helpers,
  plus direct branch/OID, own gitdir/common-dir identity, ignored status and lock
  checks. Removal repeats it under the same lease and prunes with
  valid=true/working_tree=true/locked=false. Both path/admin absent is idempotent;
  mixed states or missing tracked files/.git require recovery without further
  removal. Dirty, ignored, locked, changed-branch/OID and nonconforming cases keep
  full file-content/directory snapshots unchanged.
- Branch deletion reads direct refs and holds a real ref transaction for the
  compare/remove; scans primary and every registration's own HEAD. It refuses
  foreign locks, indirect refs and unreadable evidence. Reading registration HEAD
  directly is essential: `open_from_worktree` still followed a replaced worktree
  .git link in the regression. An already-absent branch returns without taking a
  ref lock, which otherwise recreates empty ref directories.
- `delete_tracking_ref(plan,kind,id,expected,operation_id,oid)` runs Git first,
  refuses wrong/symbolic/dangling/unreadable refs, and accepts only the expected
  direct OID or verified absence. It returns non-Clone private
  `RetirementEvidence`, bound to the lease/root/plan/kind/item/operation/OID, holding
  an absent-slot ref lock after deletion. Keep evidence until SQL commit or
  reconciliation; ordinary Git writers cannot recreate the slot meanwhile. A
  writer bypassing Git locks entirely is still checked again before SQL writes.
- `retire_context(tx,repository_id,&evidence)` contains only SQL finalization
  pieces and pre-write root/plan/actual-ref rechecks. It deletes applicable state
  and inserts the retirement. Its caller must propagate errors/rollback, retain
  evidence through commit, and reopen/reconcile an uncertain commit result.
  Actual deferred-constraint commit failure and outer callback rollback were
  tested with reopened SQL state; neither restores the already-deleted Git ref.
  Local-only retirement writes no fabricated observed-published row, including
  with history_unknown set. Fresh Edit provisions at the latest primary HEAD.
- Deleting a state row while leaving its current advertised observation violates
  the existing snapshot invariant. Deleting that observation in-place also breaks
  a sync window pinned to the exact current batch. Final SQL therefore rotates
  the current projection to a new batch excluding only the retired item, retaining
  every original batch/observation as historical and every other item's current
  evidence. A real pinned-window regression was observed RED and now passes;
  startup audit and unchanged snapshot/window validation remain intact.
- `state::migrate` now adds `retired_contexts` and marker
  `cycle_09_retired_contexts` inside the existing service-opening migration TX.
  Strict schema/value/FK audit, rollback, idempotence, partial/lost schema and
  table inventory are covered. Existing migration/audit hooks already call this
  module; no discovery/recovery opener rewrite was needed. Columns are exactly
  repository_id/kind/item_id/operation_id/retired_oid, with per-repository-kind-item
  primary key and no paths/content/endpoint fields.
- Snapshot policy holds private retired-item evidence; a missing state row reads
  NeverPublished for that exact retired kind/item even under history_unknown.
  State rows retain precedence and the invalid never_published-row invariant is
  unchanged. Observed publication deletes retirement in the same batch SQL TX;
  an absent batch never invents a deleted state for a retirement. Neither the
  global flag nor unrelated items are reset.
- `plan_clean_integration` and synchronization already consume
  `publication_evidence_for`, so no production refs/sync/poll/merge rewrite or
  eligibility duplication was needed. Remote `mod.rs` widens only a cfg(test)
  Git-metadata path-formatting helper to repository scope for native-shaped
  fixtures. No public API/DTO, dependency or GPUI change.

### Exact GREEN / Additional RED And Regression Commands

All commands ran through Devenv in the specified worktree. Initial RED above
preceded production behavior. Counts below include intermediate failed runs,
not merely the final result. All final successful executions were warning-free.

| Exact command | Actual runs/results in execution order |
| --- | --- |
| `devenv shell -- cargo test --locked --lib cleanup` | **9 runs total:** compile RED (25 missing-API diagnostics, one unused import); runnable RED **8 passed/8 failed/856 filtered**; initial implementation **15/1/856**; fixture-corrected **16/0/856**; expanded/ref-lock **22/1/856** (test dropped owned evidence before snapshot); compile failure (one test-only missing borrow); stronger directory snapshot **22/1/856** (absent branch lock recreated empty directories); **23/0/857** after absent-branch/pinned-batch corrections; final **23 passed/0 failed/857 filtered**, 13.23 seconds, after formatting and other-worktree-admin preservation coverage. |
| `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::retirement` | **3 runs:** schema RED **0/5/867**; initial implementation **2/3/867**; corrected fixture history flag/migration observation order **5/0/867**. |
| `devenv shell -- cargo test --locked --lib remote::refs::tests::retired_context` | **3 runs:** schema RED **0/1/871**; initial implementation **0/1/871**; corrected unknown-history fixture **1/0/871**. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests` | **1 run: 12 passed/1 failed/864 filtered**, discovering that a foreign .git link with identical content/branch/OID could pass reused clean checks. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_foreign_git_link` | **4 runs:** compile failure (one test-helper visibility diagnostic); runnable RED **0/2/876** (unsafe precheck and hidden checkout); **1/1/876** (`open_from_worktree` also followed the foreign link); final **2 passed/0 failed/876 filtered** after own-gitdir validation and actual registration-HEAD inspection. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_retirement_evidence_holds_ref_lock` | **1 RED run: 0/1/878** before evidence retained the absent-slot lock. GREEN is included in final cleanup selector. |
| `devenv shell -- cargo test --locked --lib remote::state::persistence_tests::retirement_preserves_pinned` | **2 runs:** RED **0/1/879** (retirement damaged the current immutable pinned pass); GREEN **1/0/879** after retaining the original pass and rotating the current projection in SQL. |
| `devenv shell -- cargo test --locked --lib remote::state` | **2 GREEN runs:** **66/0/813**, then final **67 passed/0 failed/813 filtered**, 8.37 seconds, including pinned-pass regression and all existing state invariants/migrations/audits unchanged. |
| `devenv shell -- cargo test --locked --lib remote::refs` | **1 GREEN run: 10 passed/0 failed/869 filtered**; existing graph/evidence rules plus new actual Git/SQLite R1 eligibility case. |
| `devenv shell -- cargo test --locked --test repository_enablement registry_creates_repository_and_discovery_metadata_schema -- --exact` | **1 GREEN run: 1 passed/0 failed/77 filtered**; normal service-opening inventory includes retirement. |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::task6_absent_known_context_preserves_tracking_and_local_files` | **2 runs:** compile failure from the same test-only missing borrow (no tests); then **1 passed/0 failed/878 filtered**. |
| `devenv shell -- cargo test --locked --lib remote::poll::tests::poll_ignored_only_is_clean_and_remote_absence_wins_over_dirty` | **2 runs:** same missing-borrow compile failure (no tests); then **1 passed/0 failed/878 filtered**. |
| `devenv shell -- cargo test --locked --test remote_synchronization context_first_current_fast_forward_local_ahead -- --exact` | **1 GREEN run: 1 SSH case passed**; unchanged first-publication/current/FF/local-ahead behavior. |
| `devenv shell -- cargo test --locked --test remote_synchronization incompatible_generation_history_unknown -- --exact` | **1 GREEN run: 1 SSH case passed**; non-retired unknown history still refused. |
| `devenv shell -- cargo fmt` | **2 runs**, exit 0. |
| `devenv shell -- cargo fmt --check` | **1 run**, exit 0. |

Systematic investigation separated test setup from implementation defects:
configure's boolean forces a configuration fence, not history_unknown; fixtures
now explicitly set that flag. A migration test must snapshot before removing its
schema. Discovery can synthesize a cached local context until the later refresh;
the retirement assertion checks its R1 evidence, not premature cache removal.
The ref-lock/empty-directory/foreign-link/pinned-pass defects above were fixed in
cleanup only, after failing regression evidence. Tests use actual Git/SQLite and
isolated service authoring, no mocks, sleeps, developer keys or global config.

### Self-Review / Return For Independent Spec Then Quality Review

**Status: Task 4 implemented and targeted-verified; uncommitted, review-ready
against `79463c53beb400d95a311e558e64caf8efd2beac`.** There is no pending owner
decision on T4-1. Historical blocker reviews do not review this implementation.
The controller owns independent implementation spec/quality review and checkpoint.

Original Task 4 clauses are mapped to retained tests: clean/dirty/staged/untracked/
new-OID/other-branch/missing-registration/conflict/nonconforming precheck matrices;
ignored and locked preservation; full Git file-content/directory images; worktree
path/admin removal/idempotence and other worktree/admin preservation; partial/
mixed refusal; branch OID/all-checkout/absence/lock guards; direct tracking
expected/absent/conflicting/symbolic/dangling/unreadable/new-after-delete cases;
SQL trigger/outer rollback/deferred commit and retry; private evidence root/plan/
lock binding; exact R1 eligibility and global invariant; observation consumption/
absence; additive atomic audited migration/inventory; fresh Edit from primary.
There are **15 new cleanup tests, 6 new state tests and 1 new refs test**, using
scenario matrices where noted. Existing affected tests were not edited.

Task 5/6 must persist the real branch_deleted intent, validate actual completed
worktree/branch state and owned generation/configuration before effects, and
compose promotion complete plus applicable reservation finish with
`retire_context` in one SQL TX. Retain evidence through commit/reconciliation;
reopen uncertain commit results and replay committed completion, rather than
assuming rollback or redoing lifecycle work. They also own post-integration Ok
cleanup pending RecoveryRequired plus refresh and actual F10/F11 record/reservation
tests (Tasks 5–7). No fake journal/table/phase was added here. Cache loss/absence
alone does not authorize retirement; the helper's documented durable-intent and
ownership preconditions remain the runner's obligation.

Limits: Linux tests prove the named cases, not native Windows open-handle prune
behavior. Partial deletion is constructed deterministically, not a process kill
inside libgit2. Full check/clippy/all-features suite/CLI smoke and native gates
remain deferred to Task 8/authorization. No commits, staging, subagents, push, PR,
CI dispatch, merge, closure or worktree cleanup. Ticket remains open.

Final inventory/whitespace verification: `git diff --check` exited 0.
`git diff --exit-code -- Cargo.toml Cargo.lock devenv.nix devenv.lock
.github/workflows/build.yml src/repository/remote/sync.rs
src/repository/remote/poll.rs src/repository/remote/refs.rs
src/repository/remote/merge.rs` exited 0. `git status --short
--untracked-files=all` lists exactly **14 intended paths**: five Cycle/planning
documents, repository module plumbing, cleanup source/tests, cfg(test) remote
helper visibility, state source/tests, refs tests, inventory test and new canonical
comment `01M51G7Z00N4Q6S8V0X2Z4B6E0`. No staged files; HEAD stays `79463c5`.

## Task 4 SPEC1-P2 — Complete Registration Inventory (2026-10-16)

Spec reviewer finding verified against the actual locked libgit2: after removing
the owned context, a second registration still holds HEAD at the target branch,
but removing its commondir or gitdir makes Repository::worktrees omit it. The
existing branch guard then incorrectly deletes the branch. Both independent
regressions assert the omission, require Changed and snapshot all Git/worktree
file contents and directories. Each covers document and ticket kinds.

Before production correction:

- `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_branch_guard_keeps_checkout_with_missing`
  → **0 passed / 2 failed / 880 filtered**, both returned Deleted instead of Changed.
- `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_branch_guard`
  → **0 passed / 3 failed / 880 filtered**, including the expanded invalid/incomplete
  evidence matrix (first failure: missing HEAD also omitted by libgit2).

No production correction preceded these RED runs. Scope is the deletion guard's
inventory only; worktree precheck/removal/pruning behavior is retained. No commit
or delegation; return to the same spec reviewer for recheck after targeted GREEN.

Correction: `branch_is_checked_out` now enumerates the physical common-dir
worktrees inventory with read_dir instead of the filtered libgit2 list. Every
entry must be a real directory with readable, ordinary commondir/gitdir/HEAD
members; metadata links must resolve to this common directory and the registered
worktree's Git link. Missing/invalid/unreadable members, non-directory inventory
entries, symlinks/reparse points and enumeration/read errors refuse deletion.
Each complete registration's own HEAD remains authoritative. A verified absent
inventory is the only no-registration case. Ref/OID checks and the ref transaction
remain intact; worktree precheck/removal/pruning code was not changed.

The added matrix also covers missing/invalid HEAD, invalid commondir/gitdir,
a directory substituted for a metadata member, a file inventory entry and a file
substituted for the inventory itself. All preservation checks include directory
entries, files, branch refs, worktree bytes, indexes and registration metadata.

| Exact command | Actual correction-pass results |
| --- | --- |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_branch_guard_keeps_checkout_with_missing` | **1 RED run: 0 passed/2 failed/880 filtered**, before production correction. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_branch_guard` | **2 runs: RED 0 passed/3 failed/880 filtered; GREEN 3 passed/0 failed/880 filtered**, 2.09 seconds. |
| `devenv shell -- cargo fmt` | **1 run**, exit 0. |
| `devenv shell -- cargo fmt --check` | **1 run**, exit 0. |
| `devenv shell -- cargo test --locked --lib cleanup` | **1 GREEN run: 26 passed/0 failed/857 filtered**, 14.25 seconds, including existing worktree/prune/branch matrices and all three new regressions. |

No warnings. No deps, mocks, sleeps, global config or developer-data changes.
This pass updates only cleanup source/tests, this ledger and canonical comment
`01M51G7Z00N4Q6S8V0X2Z4B6E0`; the rest of the existing uncommitted Task 4 inventory
is retained. `git diff --check` passed. HEAD stays `79463c5`, no staged files or
commit. **SPEC1-P2 correction is implemented and targeted-verified, ready to return
to the same spec reviewer for recheck; reviewer acceptance is pending.** No new
review approval or reviewer test execution is inferred. Full/native gates remain
deferred as before.

## Task 4 QUALITY2-P1 — Hidden Index Edits / Recursive Submodule Safety

Verified both data-loss findings before production changes. Real Git update-index
flags on tracked noncanonical `ordinary` hide edits from status while the index
tree equals HEAD. Both assume-unchanged and skip-worktree incorrectly prechecked
Ok. Real populated submodules, built locally with tracked .gitmodules ignore=all,
also prechecked Ok despite ignored/untracked/tracked data hidden below gitlinks.
No clone, network, global or production config changes are involved in fixtures.

Pre-correction exact runs:

- `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_hidden_index`
  → **0 passed/2 failed/888 filtered**; both incorrectly returned Ok.
- `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_submodule`
  → **1 passed/3 failed/886 filtered**; recursive clean control passed, ignored,
  hidden dirty and embedded-Git-data refusal cases failed at incorrect Ok.
- `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_raw_prune_has_no_submodule`
  → **1 passed/0 failed/889 filtered**: raw prune actually deleted a populated
  submodule's ignored fixture file and worktree/registration. External child Git
  metadata survived. This proves raw prune supplies no ignored-submodule safety;
  it is not an assumed backend property.
- `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_submodule_unknown_or_advanced`
  → **0 passed/1 failed/890 filtered**, advanced unpublished nested HEAD wrongly Ok.

All regression tests preceded production correction. The intended correction is
cleanup-only: refuse hidden index flags without changing index state, explicitly
inspect populated gitlinks recursively under the existing lease, bound inspection
and fail closed when safe deletion cannot be proved. Shared synchronization policy
and the approved spec are unchanged. Return to the same quality reviewer for
recheck; no commit or delegation.

Implemented a shared **cleanup-only** recursive clean proof, reused by precheck
and the immediately-before-prune recheck:

- Refuse on-disk assume-unchanged/skip-worktree index entries before status/tree
  cleanliness checks. Never clear flags, stage bytes or write an index to make
  the worktree appear safe. Both hidden-edit tests assert status empty/index tree
  equal HEAD before the refusal and preserve exact index bytes, edited bytes,
  registration and full directory/file snapshots afterward.
- Enumerate gitlinks from the actual index, not the superproject's status report.
  For each populated submodule, validate its local path/Git identity and HEAD at
  the recorded gitlink OID, then apply the same index-flag/clean/untracked/ignored
  proof recursively regardless of parent ignore configuration. Inspect ignored
  descendants explicitly. Missing/inaccessible/unopened/advanced/redirected state
  is Changed. A foreign linked-worktree registration is not owned by cleanup.
- All inspection remains under the existing ContextCleanup lease, with no child
  lease, reservation, transport or configuration mutation. A visited-gitdir set,
  depth bound and total bound of 64 repositories fail closed on cycles/oversize
  graphs. The clean nested control asserts the outer lease remains held and zero
  remote-operation rows before/after; byte snapshots show no cleanup-side registry
  or external Git metadata changes.
- A clean worktree alone does not prove embedded submodule Git history disposable.
  Refuse if the child's gitdir/common-dir would be removed with the context path
  or registration. Canonicalized removal roots make that containment check native
  path-representation independent. Clean nested submodules with retained external
  Git metadata still pass and prune successfully; embedded Git data is preserved.

The new actual-Git fixtures cover depth one/two ignored data, hidden untracked and
modified tracked bytes, both hidden index flags inside submodules, unpublished
nested HEAD advance and missing Git linkage. These tests snapshot all working
files/directories plus external child indexes/objects/config/refs, and preserve
the original context registration on refusal. The raw-prune characterization is
retained using disposable ignored data only. No synchronization/poll/merge policy,
approved contract, dependency or public interface change was made.

| Exact command | Actual correction-pass results |
| --- | --- |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_hidden_index` | **2 runs: RED 0 passed/2 failed/888 filtered; GREEN 2 passed/0 failed/889 filtered**. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_submodule` | **2 runs: RED 1 passed/3 failed/886 filtered; GREEN 5 passed/0 failed/886 filtered**, including the subsequently added unknown/advanced case. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_raw_prune_has_no_submodule` | **1 pre-correction characterization run: 1 passed/0 failed/889 filtered**, proving ignored submodule data is actually deleted by raw prune. Also included in final cleanup GREEN. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests::cleanup_submodule_unknown_or_advanced` | **1 RED run: 0 passed/1 failed/890 filtered**; GREEN is covered by the expanded submodule selector and final cleanup. |
| `devenv shell -- cargo test --locked --lib cleanup` | **2 runs: 33 passed/1 failed/857 filtered**, then final **34 passed/0 failed/857 filtered**, 16.67 seconds. The intermediate failure was test-only WAL/SHM snapshot timing after adding SQL reservation-count probes; production/Git preservation checks were already green. Moving the snapshot between the test's SQL probes preserved the full assertion and excluded the probes' own connection checkpoint/removal effects. |
| `devenv shell -- cargo fmt` | **2 runs**, exit 0. |
| `devenv shell -- cargo fmt --check` | **2 runs**, exit 0. |

Final runs warning-free. `git diff --check` passed, as did unchanged dependency/
toolchain and production sync/poll/refs/merge checks. This correction pass changes
only cleanup source/tests, this ledger and comment `01M51G7Z00N4Q6S8V0X2Z4B6E0`;
the existing 14-path Task 4 inventory remains uncommitted. No full gate, staging,
commit, subagents or outward action. Native Windows execution remains unclaimed.
**QUALITY2-P1 is implemented and targeted-verified, ready for the same quality
reviewer's recheck; acceptance remains pending.** No reviewer approval or rerun
is inferred. The approved never-discard/ignored rules already require this proof;
no owner-policy/spec amendment is needed or introduced.

## Task 4 Final Independent Rechecks — Local Gate Complete (2026-10-16)

The controller conveyed the final review results for the full current
uncommitted Task 4 change set:

- **SPEC recheck PASSED**, including SPEC1-P2's complete physical registration
  inventory and preservation correction.
- **QUALITY recheck APPROVED**, both P1 findings resolved and **no findings
  remaining**, including hidden index flags and recursive submodule safety.

**Task 4 is reviewed complete at the local gate. Task 5 is next after the
controller's checkpoint.** Exact review base and unchanged HEAD are
`79463c53beb400d95a311e558e64caf8efd2beac` (`79463c5`). Reviews cover the full
current uncommitted Task 4 source/tests/documentation change set, including all
SPEC1-P2/QUALITY2-P1 corrections, not just the last correction. Controller owns
explicit staging and the pending checkpoint commit. No new commit/range is implied.

T4-1 remains **owner-approved**, with guarded Git-first tracking deletion, separate
atomic SQL retirement/completion and honest split-state/uncertain-commit
reconciliation. Historical blocker, owner approval, original review findings,
RED/GREEN evidence and prior pending-recheck statuses remain preserved; this
result supersedes the latter. No owner decision or implementation review finding
remains open for Task 4.

This coordination-only pass updates ledger, handoff, Task 4 plan summary and
canonical comment `01M51G7Z00N4Q6S8V0X2Z4B6E0`. **No source/test changes, test
reruns or commit.** Review results are controller-reported; no additional reviewer
test execution is inferred. Prior evidence is the final **34 passing cleanup
tests**, **67 state tests**, **10 refs tests**, inventory and focused poll/real-SSH
regressions. Exact prior commands/counts remain the verification record.

Full Task 8/local/native gates remain deferred. Task 5/6 still compose the real
durable branch_deleted intent, owned generation/configuration fencing, completion
and applicable reservation finish, plus F10/F11 lifecycle/uncertain-result tests
in Tasks 5–7. This is Task 4 local-gate completion, not Cycle closure. No Task 5
implementation, staging, commit, push, PR, CI, merge, closure or worktree cleanup
was performed by this status worker; ticket remains open.

Coordination verification: `git diff --check` passed. `git hash-object` checks of
all eight pending source/test files before and after this pass matched exactly.
`git status --short --untracked-files=all` retains exactly 14 intended Task 4
paths, with no staged files. Only the four coordination documents/comment changed
in this pass; HEAD remains the full review base above. No Rust command was run.

## Task 5 — Promotion Storage And Reservation Composition (2026-10-16)

**Status: implemented and targeted-verified; uncommitted, ready for controller's
independent spec review then quality review. Neither review is claimed here.**
Task 4's reviewed checkpoint is now `f641f917113cc252f429860ba5092efcbbfa260c`.
This fresh worker read worktree AGENTS, approved design/Task 5 and existing seams,
used TDD inline and spawned no subagents. The controller supplied its freshly
completed fetch/no-op rebase onto `89d04f901636229392a311241f8e777f0a00962b`;
the worker confirmed the exact ticket branch, clean initial tree, both SHAs and
successful origin/main ancestry. It did not repeat that controller preflight or
touch the main checkout. All edits/commands were in the requested ticket worktree.

### Delivered Contract And Engineering Decisions

- Added `document_promotions`, marker `cycle_09_document_promotions` and the
  partial unique unfinished-item index in the existing migration transaction.
  Marked missing/malformed tables or indexes and unmarked partial creation fail
  closed. A repeated migration audits without writes. Startup checks exact
  schema, decoded values, foreign keys and cross-journal identities. Legacy
  registration/local-journal/retirement rows are retained. Normal inventory lists
  the new table; Task 4's retirement migration is reused unchanged.
- Typed steps/categories, immutable identity and integration evidence, stored
  abandoned refusals, Busy for another unfinished item, and supersession only for
  context_changed/ignored_files/remote_context_changed. Internal RepositoryBusy
  is the storage representation for the future runner's public Busy mapping.
  Ordinary saves cannot mark complete or superseded or reopen terminal rows.
- **Narrow schema additions:** `refusal_category` is a closed CHECKed enum of
  blocked/external_change/reconfirmation_required; `refusal_blockers` is an
  optional 1..65535 bitset of the existing 16 declaration-ordered blockers.
  Relational CHECKs bind these fields to abandoned/blocked. This persists exact
  content-free refusal replay without a free-text payload or another table.
- Child IDs are minted internally, collision-checked and stored with checked
  monotonically increasing attempt numbers before execution. `child_state`
  reads authoritative child completion first; retained candidates/conflicts/
  unreleased resolutions keep their original ID independent of the outer error.
  Missing ownership and failed/unreadable/foreign-scope ownership are distinct.
  Replacement refuses completed/retained/uncertain/active children. The explicit
  PrimaryAdvanced reset is allowed only from context_synchronized, clears stale
  context evidence and persists a newly minted context attempt atomically.
  Cancellation flags parent and applicable active children/reservation; explicit
  continuation clears only the parent's stop, not child ownership.
- Parent/local/remote/current-child/comment-binding IDs are cross-checked under
  the existing begin/reservation transactions, including foreign repository,
  action, priority and item scope. Own promote reservations are accepted. Child
  local refresh is allowed only for exact authoritative synchronization handoff;
  the pre-existing local/sync/poll/comment guards remain in force.
- Promote uses existing manual arbitration: automatic polls are Busy, running
  polls yield and running synchronizations stay Busy. Restart checks generation,
  increments owner epoch and returns a newly fenced token, also after reopening
  the service (verified by the final focused reservation run). A recoverable promote
  cancellation retains the slot and does not write polling history/policy.
  Superseding a stopped parent atomically terminalizes/fences **only its own
  promote reservation**, so the replacement is not stranded behind that old slot.
  Neither path changes synchronization or poll cancellation/restart behavior.
- **T4-1 composition:** `complete_record(tx, service, repository_id, operation,
  &evidence, owner)` requires prior branch_deleted intent, matching private
  retirement evidence, frozen configuration and applicable owned reservation.
  It calls SQL retirement, marks the parent complete and calls transaction-only
  `finish_promotion` in that one transaction. The public remote finisher is not
  used. `RetirementEvidence::matches_promotion` is a read-only bound-evidence
  predicate; it exposes no constructor, clone, lock or detached proof.
- The caller retains the existing ContextCleanup/evidence lease and tracking-slot
  lock through SQL commit and the transaction-scoped `reconcile_completion`.
  Failure at the last SQL statement preserves every field in eight completion
  tables, including observed_published history and the held reservation, while
  the Git ref stays absent. Competing ref recreation remains refused through
  rollback, actual-state reconciliation and successful commit; dropping evidence
  finally releases it. Retry finalizes SQL without repeating a Git deletion.
- Uncertain completion reads the actual parent/reservation rows in a new SQL
  transaction. It does not demand that mutable retirement/history projections
  remain unchanged forever: later observed publication legitimately removes
  retirement, without revoking historical completion. Local-only and contained,
  absent-remote-context cleanup can complete without an applicable reservation;
  an actually present reservation must still finish in the same transaction.
  Neither case fabricates observed-published history.
- Added lifecycle FailurePoints with the exact design names. F7 reuses the
  already-existing `AfterRemoteDeletion` variant; the other eleven are new.
  Their runner call sites remain Task 6. Status projects the parent row's step
  once, suppressing its reservation duplicate, with no resume offer/new DTO field.
  F2's `NotResumable` implementation is untouched.

### Exact RED / GREEN Evidence

All Rust commands used Devenv. Missing-API compile RED and test-authoring errors
are distinguished below from behavioral RED. Test-first declarations initially
had no storage/reservation implementation. Subsequent added cases demonstrated
the missing completion, cross-scope, status, cancellation, supersession and
no-applicable-reservation behavior before their corresponding fixes. No entire
library/full all-feature gate ran. Counts are passed/failed/filtered.

| Exact command | Actual runs/results in order for that command |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::record` | **13 runs:** compile RED (62 diagnostics, 2 unused-import warnings; includes test-only Busy variant/Debug assumptions); compile failure (3 private read_operation diagnostics, 1 unused import); runnable **9/2/894** (local-only replay binding and completion stub); compile failure (1 test-only shadowed plan call); **10/3/894** (missing reservation cross-check/finalization plus missing fixture observed_at); compile failure (1 missing finish helper after a patch-envelope duplicate-file section was corrected); GREEN **13/0/894**; added status/cancellation RED **14/2/895**; foreign-child-scope RED **17/1/897**; GREEN **19/0/897**; GREEN **20/0/897**; expanded SQL-image/startup GREEN **20/0/897**; final transaction-signature GREEN **20/0/897**. All final runs warning-free. |
| `devenv shell -- cargo test --locked --lib remote::reservation` | Two runs: initial compile RED (59 missing-API diagnostics, 1 unused import); GREEN **45/0/871** including all existing reservation tests and five new promotion cases. |
| `devenv shell -- cargo test --locked --lib recovery` | Two runs: initial compile RED (59 missing-API diagnostics, 1 unused import); GREEN **43/0/873**, including the new parent-ID begin guard and existing comment/key/local recovery guards. |
| `devenv shell -- cargo test --locked --lib remote::reservation::tests::promotion` | Three runs: restart-stub RED **2/1/902**; added held-slot cancellation RED **4/1/910**; final reopened-service/epoch GREEN **5/0/912**. Broad reservation GREEN above also covers the corrections. |
| `devenv shell -- cargo test --locked --lib promotion` | One run: **104/1/807**; new completed-replay regression intentionally removed retirement after completion and exposed the overly strict projection requirement. Corrected reconciliation to trust actual committed parent/reservation authority; final record GREEN covers it. Prior observation/integration cases passed in this run. |
| `devenv shell -- cargo test --locked --lib promotion::record::tests::supersession_fences` | RED **0/1/915** (old promote owner stranded the replacement); final record GREEN covers atomic fencing/release. |
| `devenv shell -- cargo test --locked --lib promotion::record::tests::finalization_without_applicable` | RED **0/1/916** (remote-configured contained/absent-context case incorrectly demanded a reservation); final record GREEN covers local-only and contained no-reservation completion with no fabricated history. |
| `devenv shell -- cargo test --locked --test repository_enablement registry_creates_repository_and_discovery_metadata_schema -- --exact` | GREEN **1/0/77**, required inventory target. |
| `devenv shell -- cargo test --locked --lib read::status` | GREEN **10/0/907**, existing status contracts. The new parent-step/deduplication case is in record GREEN. |
| `devenv shell -- cargo test --locked --lib cancellation_after_an_interrupted_resolution_is_a_recoverable_stop` | GREEN **1/0/916**, existing real owned-resolution cancellation regression. |
| `devenv shell -- cargo fmt` | Five runs, exit 0. Formatting only; full Task 8 gates remain deferred. |

The ownership matrix exercises both context and primary bindings against actual
stored child seams: absent, retained clean candidate, interrupted retained
conflict, authoritative completed publication and corrupted/unreadable ownership.
It asserts unchanged child IDs/attempts and refusal to replace ownership. The
existing owned-resolution regression covers its four real resolution boundaries;
full parent runner retry/network/error mapping is still Task 6's obligation.
Test-authoring corrections did not alter existing policy: complete replay must
not synthesize a remote target for a local-only parent; fixtures must supply
observed_at and last advertised evidence; loop replay must use the current row's
ID. Debugging used concrete compiler/SQL errors and snapshot invariants, not
changed preservation expectations or sleeps.

### Full File Inventory And Review Handoff

Review the complete uncommitted Task 5 change set against **f641f91**, including
new files (not just tracked `git diff` output):

1. `src/repository/promotion/record.rs` — schema, typed storage, audit, attempts,
   identity guards, cancellation, finalization and committed-state reconciliation.
2. `src/repository/promotion/record_tests.rs` — 20 new record/composition tests.
3. `src/repository/promotion/mod.rs` — record module registration only.
4. `src/repository/remote/reservation.rs` — scoped ID guard, ownership probe,
   promote restart/cancellation/transaction-only finish.
5. `src/repository/remote/reservation_tests.rs` — five new promotion tests.
6. `src/repository/remote/state.rs` — schema/read visibility and startup audit hook.
7. `src/repository/recovery.rs` — local begin identity guard and one regression.
8. `src/repository/discovery.rs` — migration call in existing transaction.
9. `src/repository.rs` — F1–F12 variants (shared existing F7).
10. `src/repository/read/status.rs` — parent step projection/deduplication only.
11. `src/repository/cleanup.rs` — private evidence matching predicate only.
12. `tests/repository_enablement.rs` — table inventory entry.
13. This execution ledger.
14. Canonical comment `01M51H7Z00N4Q6S8V0X2Z4B6E1`.

**Task 6 handoff, not implementation authorization:** compose the actual runner
with these storage seams and Task 3's real prepare/candidate/apply interfaces;
commit integrating intent before effects and branch_deleted before tracking
deletion; hold the private evidence through the outer SQL transaction's commit
and reopened-state reconciliation. Treat storage RecoveryRequired after integration
as Ok pending plus refresh. Consume ChildState::Complete without rerunning, retain
owned/uncertain child IDs, and acknowledge unowned active child cancellation using
the existing synchronization before asking for a replacement attempt. Preserve
published history until final SQL. Exercise actual runner F10/F11 process/commit
uncertainty and F1–F12/SSH boundaries in Tasks 6/7. No run.rs or runner stub added.

No new owner decision is pending. Remaining review/native/full-gate risks are the
explicitly deferred Task 6/7 orchestration/SSH proof, Task 8 full local gates and
native Windows open-handle behavior. Targeted results do not claim those gates.
No dependencies, GPUI, global configuration, developer keys, sleeps, staging,
commit, push, PR, CI dispatch, merge, ticket closure or worktree cleanup. Ticket
remains open; controller owns reviews and checkpoint commit afterward.

Final preservation checks: `git diff --check` and `git diff --exit-code --
Cargo.toml Cargo.lock devenv.nix devenv.lock src/repository/remote/sync.rs
src/repository/remote/poll.rs src/repository/remote/refs.rs
src/repository/promotion/integrate.rs src/repository/mutation/bind/resume.rs
.github/workflows/build.yml` passed. HEAD remains the exact Task 5 base above.

## Task 5 SPEC P2 — Guard The Pre-Effect Refresh Check (2026-10-16)

**Status: correction implemented and targeted-verified, returned to the same SPEC
reviewer for recheck. SPEC acceptance is pending; no quality approval is claimed.**
The entire Task 5 change remains uncommitted against
`f641f917113cc252f429860ba5092efcbbfa260c`. No Task 6 work or checkpoint commit.

### Verified Finding And Narrow Fix

The reviewer identified a real pre-effect hole. `begin_or_reconcile` called the
new promotion identity guard, but `recovery::require_refresh_can_begin` did not.
The actual `create_folder` bridge asks that earlier question before marking the
registration stale and making directories. With a confirmed promotion parent ID
or a minted, not-yet-run child ID, it could therefore create the directory/change
the cache and then convert the refresh's late OperationMismatch into a partial
folder result. The earlier comment-publication guard already checked both paths.

Added three focused tests through **actual `RepositoryService::create_folder`**,
using enabled disposable repositories and repository-local fixture identity:

- `spec_p2_create_folder_parent_id_preserves_directory_and_cache`;
- `spec_p2_create_folder_unrun_child_id_preserves_directory_and_cache`;
- `spec_p2_create_folder_authoritative_child_keeps_exact_refresh_handoff`.

Both negative tests snapshot every directory/file beneath docs and every row of
every registry table, with refresh_required proven current beforehand. Before
production changes, each reported **directory unchanged=false and cache
unchanged=false**. After the correction they return OperationMismatch with
left_directories=false, preserve both complete snapshots and leave the entire
requested ancestor path absent. This verifies the bridge's real side-effect
boundary, not just a call to the guard.

The positive case uses real reservation/checkpoint/classification seams to create
authoritative **context and primary** children bound to their promotion. Their
actual folder calls still return Created(Current), and the actual refresh leaves
the matching local row exactly refresh / empty target / completed. Wrong-root
pre-effect use, wrong local action and a nonempty refresh target remain refused.

Production fix: call the existing `promotion::record::validate_local_identity`
from `recovery::require_refresh_can_begin`, before its existing comment guard and
poll/pending-operation checks, with the same root, refresh action, ID and empty
target used by the eventual refresh. Updated the guard's documentation. No new
identity policy or duplicate implementation; the exact authoritative-child refresh
exception is retained. Production folder, promotion storage and synchronization
code are unchanged by this correction.

### Exact Correction-Pass Verification

| Exact command | Actual results |
| --- | --- |
| `devenv shell -- cargo test --locked --lib spec_p2_create_folder` | Three runs: initial **0 passed/3 failed/917 filtered** (two genuine preservation failures, plus a test-only incorrect expectation that an additional already-completed same-ID refresh returns Refreshed); corrected test-only replay assertion, still before production changes, **1 passed/2 failed/917 filtered**, both directory/cache snapshots changed; after the narrow guard fix **3 passed/0 failed/917 filtered**. The positive case already proves its refresh through Created(Current) and the completed exact local row, so no redundant subsequent replay assertion is used. |
| `devenv shell -- cargo fmt` | One run, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion::record` | Final **23 passed/0 failed/897 filtered**, including strengthened current-cache and wrong-root assertions. |
| `devenv shell -- cargo test --locked --lib recovery` | **43 passed/0 failed/877 filtered**, retaining comment-child pre-effect/handoff, poll priority and local/key recovery guards. |
| `devenv shell -- cargo test --locked --test mutation_bridges folder::domain` | **7 passed/0 failed/122 filtered**, actual existing folder bridge regressions. |

All runs used Devenv and were warning-free. The first RED's positive-case failure
was traced to `refresh_repository_target` returning IndexPending when an already
completed row cannot be claimed again; only that redundant test assertion was
removed. The pre-effect preservation expectations were never weakened. No sleeps,
developer keys, dependency, GPUI or developer/global Git configuration changes.

### Correction Scope And Return

Only these four paths changed during this correction pass:

- `src/repository/recovery.rs` — shared pre-effect guard call and documentation;
- `src/repository/promotion/record_tests.rs` — actual bridge regressions/helpers;
- this execution ledger;
- canonical comment `01M51H7Z00N4Q6S8V0X2Z4B6E1` — appended correction evidence.

The prior full Task 5 file inventory and deferred full/native/Task 6–8 obligations
remain in force. Return the complete current Task 5 diff against f641f91 to the
**same SPEC reviewer for P2 recheck**, then quality review after SPEC acceptance.
No staging, commit, push, PR, CI, merge, closure, cleanup or new delegation.
Ticket remains open; controller owns the checkpoint after review.

## Task 5 Final Reviews And Nonblocking Lookup Minor (2026-10-16)

**Task 5 is reviewed complete at the local gate. Controller checkpoint pending;
Task 6 is next after that checkpoint.** The controller conveyed **SPEC PASS**,
including the P2 pre-effect guard correction, and **QUALITY APPROVED**, with no
fatal issues and one nonblocking lookup minor worth fixing before checkpoint.
That minor is now resolved and targeted-verified. Review scope is the full
current uncommitted Task 5 change set against
`f641f917113cc252f429860ba5092efcbbfa260c`, not just the last correction. These
controller-reported approvals supersede pending-SPEC/recheck statuses above;
historical evidence remains intact. No additional reviewer execution or separate
minor-fix re-review is inferred.

### Verified Minor And Narrow Resolution

Source inspection confirmed both unnecessary full-history reads:

- begin_record loaded/decoded records(tx, repository_id), then searched for the
  requested item's unfinished row. It now queries repository_id + item_id with
  the **exact partial-index predicate**
  `step NOT IN ('complete','abandoned','superseded')`, decoding only that row.
  Existing same-ID replay/collision checks still run first; Busy, permitted
  supersession and stored abandoned refusals keep their existing semantics.
- promotion_operations loaded/decoded all repository promotions even for one
  explicit operation ID. That path now uses the existing typed read_record
  lookup with the same repository scope. Terminal rows still show explicitly;
  missing rows stay absent. List and startup audit retain the full-history path.

Only `promotion/record.rs` and `read/status.rs` changed in production during this
pass. No schema, identity guard, refusal encoding, child ownership, transaction,
reservation, DTO or F2 behavior changed. The requested existing focused tests were
used; no tests mirroring query implementation were added.

### Exact Verification And Coordination

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo fmt` | One run, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion::record` | **23 passed/0 failed/897 filtered**, including exact identity/abandoned replay, Busy/supersession, parent-step status and SPEC P2 authoritative-child/side-effect guards. |
| `devenv shell -- cargo test --locked --lib read::status` | **10 passed/0 failed/910 filtered**. |

All runs warning-free. Earlier targeted verification remains recorded; no broad
or full/native gate was rerun. The implementation plan's Task 5/current-progress
summary, fresh-session handoff and canonical comment
`01M51H7Z00N4Q6S8V0X2Z4B6E1` now reflect reviewed-complete Task 5 and Task 6 next
after the controller's checkpoint. Task 4's committed base is f641f91.

This pass changes six paths: the two production files above, this ledger,
implementation-plan coordination, handoff coordination and the existing canonical
comment. All prior Task 5 pending source/tests remain present. No other source
change, new test, dependency, GPUI, global config, developer key, sleep, subagent,
staging, commit or outward lifecycle action. HEAD stays f641f91, ticket open;
Task 6 has not started. Full Task 8/native gates remain deferred.

## Task 6 — STOP T6-1: Process-Exit Leaves The Tracking Absence Lock (2026-10-16)

**Task 6 is incomplete and blocked. The blocker evidence is ready for independent
review; the implementation is not spec/quality-review-ready as a completed task.**
The owner explicitly requires a stop on a real contract contradiction. This is
the required process-exit case inside F10, not an ordinary failure-injection unwind
or a hypothetical concern. No recovery policy was silently weakened and no stale
Git lock was removed automatically.

### Entry And Scope

- Reused the supplied controller preflight, clean exact ticket worktree and branch
  at `8d06577` (Tasks 0–5 reviewed/committed), `origin/main=89d04f9`. Verified clean
  initial status, worktree root, recent history and ancestry (exit 0). No new
  fetch/rebase was required within the controller's preflight boundary.
- Read worktree AGENTS, approved implementation/design and actual Task 1–5 seams,
  Task 0's measured ownership table, TDD/executing-plans/cycle/debugging guidance.
  No subagents. All Rust commands used Devenv and targeted selectors. Full gates
  remain deferred to Task 8. No dependency, GPUI, CLI/desktop/F2 adapter, global
  configuration, developer key, sleep, staging, commit or outward action.
- Implemented only in the ticket worktree; every source change below remains
  uncommitted and unreviewed. Preserve this work when resuming; do not replay the
  reviewed Tasks 0–5.

### Reproduction And Actual Durable State

`promotion::run_tests::process_exit_f1_through_f12_and_inside_f10_reopens_without_repeated_effects`
uses real repositories, actual promotion storage, Task 3 integration, Task 4 local
cleanup/retirement and Task 5 reservation/completion. Only child transport calls
are injected, as the Task 6 plan permits. Each child uses real reservation rows
and stores authoritative completion. Counters in the disposable fixture registry
count child publication/delete calls and actual local-helper effects. The service
in a subprocess is `open_at_with_exit_point_for_testing`; no destructor runs.

1. The twelve published F1–F12 exit cases (test indices 0–11) each reopen and
   finish under the original operation ID, with exactly one promotion commit,
   one context/primary publication, one remote-delete call, one actual worktree
   removal and one actual branch deletion. Stored completion replays without
   further child/deletion calls.
2. Test index **12** installs an internal-F10 test-only exit callback immediately
   after `ContextCleanup::delete_tracking_ref` returns its private evidence, and
   before `complete_record` executes SQL. The subprocess exits with the expected
   `FAILURE_POINT_EXIT_STATUS`, using the same actual service/cleanup path.
3. Reopened actual state is: tracking ref absent; promotion **branch_deleted**;
   existing history **observed_published**; **zero retired_contexts**; Promote
   reservation still **advertising**. These assertions pass before retry. The
   tracking slot's `.lock` file exists and its exact contents are **empty**.
4. A fresh service and the original request return
   `IndexPending(outcome { publication: Published, cleanup: Pending(RecoveryRequired) })`.
   No completed lifecycle work is repeated, but the required same-ID completion
   fails. The test remains RED; the lock is not erased to force a green result.

Root cause: `cleanup.rs::delete_tracking_ref` commits the Git removal, then creates
a **second git2 ref transaction** and locks the absent slot. `RetirementEvidence`
keeps that transaction through SQL completion. On normal return or rollback Rust
drops it and removes the lock, which is why unwind/reopen tests pass. Process exit
leaves the filesystem `.lock` behind; it is not an OS-only lock released by exit.
On retry, the same helper's `transaction.lock_ref` refuses it before SQL. Its
empty contents carry no operation/owner/identity evidence distinguishing this
orphan from a legitimate foreign Git writer's lock. Durable branch_deleted intent
and ref absence alone are not such evidence. Blind deletion would weaken the
guard against ordinary Git writers. A safe recoverable absence-guard ownership
mechanism is missing from the approved composition seams.

The current safe refusal preserves T4-1's history/reservation invariant, but does
not satisfy the required inside-F10 process-exit/same-ID-finalization acceptance.
Controller/spec review must resolve that contract/seam gap before implementation
continues. No amendment, lock adoption, new phase/table or cleanup algorithm has
been implemented. Normal helper composition and the settled identity policy do
not need reopening; this evidence is specifically the crash-persistent ref lock.

### Provisional Implementation And File Inventory

- `src/repository/promotion/run.rs` (new): provisional lifecycle, durable replay,
  ownership-first children, integration candidate reuse, publication/cleanup
  pending categories, cancellation, retirement composition and fresh-ID refresh.
  Consumes existing algorithms; no duplicate merge or cleanup algorithm.
- `src/repository/promotion/run_tests.rs` (new): 25 named tests/helpers, including
  actual local lifecycle, no-identity contained cleanup, abandoned replay/new ID,
  conflict at execution, identity, lost-result authority, a 24-case both-role
  structural ownership-policy matrix, uncertainty, primary-only/new-path consent,
  F2 primary advance, cleanup preservation, rejected publication/deletion retry,
  SQL rollback, wrong/symbolic refs, cache loss, cancellation, refresh and process
  exits. The structural matrix seeds real **prepared child ownership**; it must
  not be described as actual installed conflict/resolution/external-repair proof.
- `src/repository/promotion/mod.rs`: approved request/result/outcome/accessor and
  error shapes, content-free formatting, public promote/cancel entry points.
  `repository.rs` already re-exports `promotion::*`; no edit is necessary there.
- `src/repository/promotion/integrate.rs`: narrow candidate-OID accessor and
  validated `resume_candidate`, reusing the existing prepared tree/commit evidence
  after F3 instead of creating a second commit with a changed timestamp.
- `src/repository/remote/{mod.rs,observation.rs,state.rs}`: repository-scope access
  to observation configuration/fence and authority OID, without algorithm changes.
- `src/repository/remote/sync.rs`: extracted the existing authenticated scope and
  frozen-configuration fence into `with_scoped_action_remote`, retaining the
  synchronization-owned boundary via its wrapper. Promotion supplies its own
  reservation boundary; no synthetic synchronization owner for local integration.
- `src/repository/remote/reservation.rs`: cfg(test)-only bridge seeds real prepared
  child ownership without exposing production synchronization internals.
- `docs/research/wave-03-api-audit.md`: preview is present, provisional execution
  and retry remain explicitly blocked; C5/D5 prepare/execute binding, ten-minute
  expiry, preview schema and F2 NotResumable boundary remain pending.
- This ledger and canonical comment `01M51J7Z00N4Q6S8V0X2Z4B6E2` record the stop.

### Exact Commands And Results

All counts below are actual targeted executions, not full-library gates. Initial
compile failures and fixture errors are distinguished from behavior RED.

| Exact command | Runs/results in execution order |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion` | Initial compile RED: 16 missing-API/import diagnostics, zero tests. Declaration-only runner runnable RED: **112 passed/6 failed/807 filtered** (five new lifecycle failures; one scaffold error-conversion regression in the existing foreign-config-lock test, reverted). Final blocked verification: **137 passed/1 failed/807 filtered**, 75.83s, warning-free. The sole failure is the actual process-exit lock blocker above. No full Task 6 GREEN is claimed. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests` | Runnable RED **0/11/920**; compile failure (private authority OID); **9/2/920**; GREEN **11/0/920**; expanded **14/3/920**; GREEN **17/0/920**; compile failure (test-private generation helper); expanded **23/2/920**. Green subsets do not clear the later expanded gate. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::post_child_new_path` | Four diagnostic failures while fixing a test fixture's cross-handle parent objects: ContextBehindRemote, then actual merge ancestry/zero-parent evidence, finally before-reset parent-count failure. Find both parents through the committing root handle; subsequent 17-case runner GREEN includes both consent scenarios. No production digest/merge algorithm changed. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::ownership_first` | Compile failure (private test-only child journal types); runnable RED **0/1/939** (Busy reused the bound no-row attempt); two **0/1/939** fixture-diagnostic failures (prepared retained child's fake completion forgot reconciliation_required=0); GREEN **1/0/939**, 20.84s, all 24 both-role scenarios. Parent now persists a fresh attempt for no-row Busy/yielding retries; retained attempts keep same-ID restart. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::process_exit` | Four failed runs: missing split-state history exposed an unconfigured fixture endpoint generation; nested key lookup exposed a fixture cache-lock recursion; after moving snapshot acquisition outside the TX, two runs reached the actual inside-F10 pending result (the latter names boundary 12 and the surviving lock). Final broad targeted run reproduces the same blocker with exact empty-lock/history/retirement/reservation assertions. |
| `devenv shell -- cargo fmt` | One run, exit 0. |

Other diagnosed failures: new remote fixtures initially changed root config without
checkpointing/merging it; fixture setup now keeps primary/context config coherent.
Cancellation changed the durable parent after the executor's snapshot, so runner
saves re-read the cancellation bit in the same SQL transaction rather than
overwriting it. Invalid live configuration on a retry after integration originally
escaped as Err; the new pre-read/validated stored authority returns Ok pending.
Both fixes have passing targeted coverage in the final 137 passing cases.

The deletion-pending reservation correctly prevents the existing refresh gate
from acquiring its index claim; those results are IndexPending with discovery
marked stale, not a claimed successful refresh. Full results and publication
pending paths without that reservation refresh normally. No bypass of refresh
arbitration was introduced.

### Remaining Work / Return

Task 6 is **not complete**. Resolve T6-1 before proceeding. Still due: genuine
installed context conflict canonical/external resolution and owned-resolution
composition cases (the current matrix is structural); persistent retained-recovery
and more cancellation/refresh assertions; runner-level supersession and explicit
early-history RemoteContextDeleted coverage; SQL commit-result uncertainty versus
actual committed completion, and complete boundary/negative-case review. Task 7
real SSH and Task 8/full/native gates are untouched. The provisional API audit
must not be changed to delivered until the complete Task 6 gate passes.

All current work is preserved uncommitted for controller review of the blocker.
No independent reviewer was spawned or approval inferred. Ticket remains open.

Final inventory verification: `git diff --check` exited 0. `git diff --exit-code --
Cargo.toml Cargo.lock devenv.nix devenv.lock .github/workflows/build.yml src/bin
src/desktop src/repository/mutation` exited 0. Status contains exactly the twelve
paths inventoried above (nine source files, audit, ledger and new comment), with
no staged changes. HEAD remains `8d06577f6af17e2532d072d7ee14419dbf8a84b4`.

## Task 6 Resume — Owner-Approved T6-1 (2026-10-16)

The owner reports explicit approval via the question tool: Cycle 09 inherits
Cycle 06's backend-lock exception. Ambiguous crash locks are preserved with `Ok`
cleanup pending RecoveryRequired plus refresh/IndexPending. Only after an operator
quiesces writers and verifies/repairs affected backend metadata may the identical
request resume unfinished SQL. No automatic deletion/adoption, new lock journal
or further owner question. Rare inside-F10/F11 recovery explicitly costs operator
intervention. Independent review confirmed the existing refusal safe, citing
Cycle 06 contract 228–242 and approved 2026-10-08 Task 4 protocol 36–46; the actual
fixture-only operator simulation at sync_tests 15283–15303 was verified in source.

Historical T6-1 blocker/evidence above are preserved, superseded as a blocker by
this approval. Cycle/design/plan/handoff now consistently qualify inside-F10/F11
acceptance. All T4-1 SQL history/retirement/reservation and once-only effects remain.
No production lock mechanism or removal is needed or authorized.

Resume status/branch/HEAD/origin-main remain the supplied exact worktree at 8d06577
and 89d04f9. Existing uncommitted work is preserved; no stash/rebase/commit, subagent
or outward action. Read AGENTS/TDD/review guidance and actual predecessor recovery
precedent. All original Task 6 obligations remain in scope. Targeted checks only.

Post-approval RED, before qualifying the retained crash test:
`devenv shell -- cargo test --locked --lib promotion::run_tests::process_exit`:
**0 passed/1 failed/944 filtered**, 11.05s, warning-free. It reproduces the old
immediate-finish expectation's failure at internal boundary 12; this is now a
contract-qualified recovery result, not a production automatic-repair defect.

## Task 6 Complete At Targeted Local Gate — Independent Reviews Next (2026-10-16)

**The whole bounded Task 6 implementation is ready for independent spec then
quality review, uncommitted against `8d06577f6af17e2532d072d7ee14419dbf8a84b4`.**
No independent approval is inferred, and no Task 7/8 gate is claimed. T6-1 is
owner-approved and resolved as a contract qualification; the historical blocker
and failed immediate-retry test remain recorded above. No other blocker remains.
Controller owns review/checkpoint; this worker did not stage/commit or delegate.

### Approved Crash Behavior And Real Evidence

The retained real-process test runs **fourteen** scenarios: F1–F12, exit after
tracking deletion/before SQL, and exit after SQL commit/before the absence guard
is dropped. It uses actual Git, Task 3/4/5 helpers, process exit (no destructors),
reopened services and disposable-registry effect counters. Child network/publication
calls alone use the approved injected seam; local child integration tests separately
execute the real synchronization merge/reconciliation/resolution stack.

- Before **any** repair in the pre-SQL crash case, retry returns IndexPending with
  cleanup pending RecoveryRequired, preserves the complete Git/worktree byte image,
  empty ambiguous lock, applicable history/current observations, zero retirement,
  branch_deleted intent and held reservation. No child/delete/removal counters
  repeat; discovery is stale after the attempted refresh. This is expected T6-1.
- Only in the parent-owned disposable fixture, with its subprocess reaped and no
  active writers, assertions verify exact ref absence, retained SQL/intent/history
  and the exact regular empty lock. The test simulates operator removal of that
  one known lock. The **identical request** then finishes the one SQL transaction,
  with one of each effect and stored replay. Production never removes/adopts the lock.
- In the post-SQL-commit exit case, actual complete/retirement/reservation finish
  are durable. The identical request replays that authority with no child/delete
  calls, preserves the surviving backend lock, and refreshes only as required.
  Ref/cache/lock absence is never used as completion or retirement authority.
- Real deferred-FK SQL commit failure and an earlier statement-trigger rollback
  each retain existing SQL history/observations, no retirement and pending parent/
  held reservation while the independently committed Git ref remains absent.
  Retry finishes without repeated lifecycle effects. A test-only lost commit-result
  seam returns an error **after actual commit**; actual-record reconciliation proves
  complete/finished and replays instead of reporting a fabricated rollback.

### Final Interfaces And Narrow Composition Decisions

The approved public API is implemented in promotion/mod.rs: PromoteDocumentRequest,
promote_document with caller SessionCredentials, cancellation returning bool,
DocumentPromotionResult::{Complete,IndexPending}, outcome accessors and separate
integration/publication/cleanup enums with fixed pending categories. Errors and
Debug output are content/path/endpoint/secret-free. The existing repository.rs
`pub use promotion::*` already exports every new type; no redundant export edit.

The runner resolves registered roots/configuration, binds/replays item+digest,
stores terminal abandoned refusals, respects indexed Busy/three-category
supersession, persists attempt IDs before children, consumes completed child
authority without rerunning, and honors actual ownership before errors. Busy/
yielding/no-row transients receive fresh attempts; active non-owned transients
are cancelled/acknowledged and **durable** cancellation is checked even when the
acknowledgement returns RecoveryRequired. Uncertain ownership never mints a replacement.

Actual canonical resolution exposed an important composition distinction: after
resolution releases its local artifacts, the **original child still owns its
publication continuation**. The narrow reservation::promotion_child_has_resolution
query reuses existing integration-window/resolution evidence for same-ID restart;
it does not misclassify every historical applied step as unfinished ownership.
Genuine canonical/external repair, interrupted retained conflict/owned resolution,
real primary identity confirmation and applied-but-uncheckpointed clean candidates
are tested, in addition to the original both-role structural policy matrix.

Integration uses the existing prepared/candidate lease stages and persists intent
before any ref. A validated candidate is reused across F3. The recorded base's
immutable configuration identifies the original primary on post-effect retries,
so live primary renames cannot redirect cleanup. Reachability and shared
primary_clean proof gate pre-integration errors: a dirty partial checkout is
pending RecoveryRequired, even if the ref did not move. Lease/SQL/cancellation-clear
failures after a matching effect become Ok pending plus refresh. Superseded
post-effect IDs report their stored partial outcome without resuming cleanup.

Publication uses manual merging children, passing host approval/confirmed identity
and a cancellation observer. The exact-delete adapter reuses the extracted existing
authenticated-scope/endpoint fence, reaches BeforeTransport/AfterAdvertisement,
and checks generation/ownership. Cleanup runs the actual strengthened Task 4
precheck before reservation **and again after F6 immediately before transport**;
its lease is dropped before credentials/network. Each local step rechecks live
configuration, ownership and actual state under the cleanup lease. Pending deletion
keeps history and all local state; typed deletion failures cover configuration,
key, unlock, host, transport and rejection. No merge/prune/deletion algorithm is
duplicated, and no synthetic synchronization owner is used for local integration.

Every Ok post-effect path marks discovery stale and attempts fresh-internal-ID
index-only refresh outside the lease. A held Promote reservation correctly makes
the existing refresh arbitration defer the claim as IndexPending. Existing
discovery deliberately leaves active items to their contexts; partial-refresh
tests verify the updated primary HEAD and actual canonical primary document while
that active-item override survives. Completed cleanup is verified as a single
primary-only public item. No discovery/F2 read-policy redesign was introduced.

### Task 6 Obligation → Passing Evidence

Runner test names below are under `promotion::run_tests`; existing Task 2/3/5
selectors in the final promotion run retain their original coverage.

| Obligation | Named passing evidence |
| --- | --- |
| Local full lifecycle, ordered parents, no remote and contained | local_full_lifecycle_and_exact_replay; runner_local_divergence_preserves_prepared_primary_tree_and_complete_discovery; contained_cleanup_needs_no_identity_or_new_merge |
| External digest, new blocker, execution-only conflict and abandoned replay/new ID | changed_digest_is_durably_abandoned_and_new_id_succeeds; new_blocker_is_stored_and_replayed_before_any_child; execution_only_local_conflict_abandons_without_primary_effect |
| Identity/local atomic persistence and operation mismatch | identity_and_mismatch_do_not_strand_the_item; all existing promotion::integrate identity/include/config-lock tests |
| Cache loss/AlreadyContained without republish/remerge, fresh Edit/R1 | cache_loss_contained_context_cleans_without_another_merge_or_child; contained_remote_cache_recovery_skips_deleted_context_child_and_does_not_remerge; later_edit_after_runner_completion_provisions_fresh_context_and_first_publication |
| Both-role ownership matrix/identity/conflict/transients/Busy/yielding/lost result, no strand | ownership_first_attempt_matrix_both_children_has_no_permanent_strand (24 scenarios); uncertain_child_ownership_fails_closed_without_replacement_and_primary_returns_pending |
| Genuine conflict inspect/read/resolve/external repair; interrupted conflict/owned resolution and original IDs | genuine_context_conflict_resolves_canonical_and_external_and_reuses_owned_resolution (four scenarios); genuine_primary_owned_resolution_stays_pending_on_recovery_then_publishes_same_child |
| Actual retained clean candidate after ref effect/checkpoint failure | actual_applied_child_candidate_is_reused_after_lost_checkpoint_for_both_roles |
| Actual primary IdentityRequired and confirmation on original child | actual_primary_identity_required_keeps_original_child_for_confirmed_continuation |
| Fresh T(P), context new path versus primary-only merge, PrimaryAdvanced reset | post_child_new_path_reconfirms_but_primary_only_merge_preserves_consent; advance_after_f2_resets_context_attempt_and_integrates_once; existing Task 3 base/no-object tests |
| Rejected publication, retained/uncertain recovery Ok pending, current/publication replay | publication_rejection_is_pending_and_retry_only_publishes_and_cleans; genuine_primary_owned_resolution_stays_pending_on_recovery_then_publishes_same_child; uncertain_child_ownership_fails_closed_without_replacement_and_primary_returns_pending; already_current_publication_is_stored_and_replayed_without_children_or_deletion |
| Changed/StillPresent/ambiguous deletion, category redaction, tracking evidence preserved | deletion_changed_and_still_present_keep_context_and_retry; ambiguous_delete_result_is_pending_then_observe_only_retry_finishes; deletion_failures_are_fixed_redacted_categories_and_keep_history; wrong_and_symbolic_tracking_refs_stop_before_any_retirement_sql |
| Dirty/untracked/moved/ignored preservation before delete and F6 recheck, supersession | cleanup_precheck_preserves_dirty_untracked_moved_and_ignored_before_remote_delete; f6_rechecks_dirty_or_ignored_context_under_lease_before_deletion; supersession_finishes_new_committed_context_and_fences_old_remote_changed_owner |
| History not reset early; explicit deleted-publication barrier | early_remote_delete_keeps_history_and_explicit_deleted_publication_barrier (real history and shared production graph planner returns RemoteContextDeleted; held reservation still excludes transport) |
| F1–F12 and inside F10/F11 process exits, at most one actual local helper effect/candidate/publication/delete | every_interruption_reopens_with_one_candidate_and_only_unfinished_children; process_exit_f1_through_f12_and_inside_f10_reopens_without_repeated_effects (14 real exits, qualified T6-1) |
| Statement rollback, real SQL commit failure, lost committed result, stored complete/finish | tracking_git_sql_split_retains_history_parent_and_owner_and_retries_same_id; final_sql_commit_failure_rolls_back_all_history_and_retry_finishes; lost_completion_commit_result_reopens_actual_complete_authority; existing promotion::record finalization/rollback/no-fabrication cases |
| Partial prune/checkout refusal; no stub finishing or post-effect Err | partial_worktree_prune_requires_recovery_and_never_finishes_a_stub; integrating_retry_with_partial_checkout_is_pending_until_verified_baseline_repair; post_ref_retry_lease_failure_and_cancel_clear_failure_never_become_errors |
| Cancellation before integration, at child safe points, after integration between whole steps, retry reset | cancellation_before_and_after_integration_preserves_and_retry_finishes; cancel_at_pre_ref_and_each_local_boundary_stops_before_the_whole_step; consent_identity_cancellation_forwarding_and_network_seams_never_hold_a_lease; genuine owned-resolution interruption cases |
| Live valid/invalid configuration changes, endpoint generation, no further cleanup | valid_configuration_changes_after_integration_stop_before_any_cleanup_effect; integrated_configuration_failure_is_ok_pending_even_when_configuration_is_invalid |
| Refresh success/failure/partial authoritative result/primary-only completion | refresh_failure_keeps_identical_authoritative_outcome_and_replay_repairs_index; partial_refresh_failure_preserves_outcome_and_later_refresh_shows_integrated_primary; later_edit_after_runner_completion_provisions_fresh_context_and_first_publication; all full/partial runner cases |
| Exact consent/session forwarding and no lease across network seams | consent_identity_cancellation_forwarding_and_network_seams_never_hold_a_lease |
| Content-free request/error/outcome formatting | fixed_error_and_request_formatting_never_includes_content_paths_or_endpoints; local_full_lifecycle_and_exact_replay; existing promotion Debug/column inventory tests |

### Resume Exact RED / GREEN And Verification

All commands below ran through Devenv in the specified worktree. No Task 7 target,
entire-library selector, dependency command, full check/clippy/test gate or native
workflow ran. Counts distinguish fixture assumptions/missing test bridges from
behavioral regressions; no failed crash case was removed or skipped.

| Exact command | Resume runs/results in execution order |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::process_exit` | Post-approval old-expectation RED **0/1/944**, 11.05s; qualified pre-repair preservation/operator-fixture-repair GREEN **1/0/944**, 12.28s. Later final gate includes the additional actual-committed F11 exit. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::genuine_context` | Initial compile RED: missing test-only local-driver bridge and incorrect external-resolution enum assumption; then **0/1/945** on cached fixture index; **0/1/945** on real canonical resolution replacing its child ID; **0/1/945** on test assumption that safe plain-code side-read must itself reject (inspection/resolve correctly require external repair); GREEN **1/0/945**, 4.54s, four scenarios. Same-ID composition fixed from the real resolution evidence. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests` | Expanded **29/1/920** (valid primary rename redirected cleanup; one not-yet-used test-seam warning); compile RED missing test-only graph-plan bridge; **31/5/920** (pre-ref cancellation, deletion categories, contained deleted-context child, post-ref lease error, supersession fixture missing real primary merge); GREEN **36/0/920**, 63.25s; expanded **39/2/920**, 64.76s (late-F6 dirty precheck and discovery test assumption). All final corrections pass the final 159-case selector. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::lost_completion` | Strengthened actual-seam-execution assertion RED **0/1/949**, then wired lost-return seam/reconciliation GREEN **1/0/949**, 0.95s. An earlier expanded test had not proved hook execution; no false positive is retained. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::valid_configuration` | GREEN **1/0/949**, 1.60s, three valid-config/endpoint scenarios after fixing the immutable primary/live cleanup fence. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::partial_refresh_failure` | Diagnostic **0/1/960**: public discovery selected active context; diagnostic **0/1/960**: observe_root intentionally filters that active item from root item rows as well; corrected assertion verifies primary HEAD/canonical file and primary-only list after completion; GREEN **1/0/960**, 0.87s. No discovery production change. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::f6_` | GREEN **1/0/960**, 1.35s, dirty and ignored late-F6 scenarios after adding the second actual leased precheck before transport. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::actual_applied` | Initial compile failure in new test assertions treating cancel_remote_operation's unit result as bool; corrected fixture API use; GREEN **1/0/964**, 1.90s, both real applied candidate/checkpoint-failure cases. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::genuine_` | GREEN **2/0/963**, 4.85s, after adding actual cancellation acknowledgement with owned resolutions to both real conflict cases. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::integrating_retry` | Behavioral RED **0/1/965**, 0.41s (dirty partial checkout escaped as Err); shared Task 3 primary_clean proof now gates that error; GREEN **1/0/965**, 0.56s. |
| `devenv shell -- cargo fmt` | Two resume runs, both exit 0. |
| `devenv shell -- cargo fmt --check` | Two resume runs, both exit 0. |
| `devenv shell -- cargo test --locked --lib promotion` | First 158-case run hit the outer shell's 120-second combined fmt/build/test timeout, **no final result claimed**. Reran unchanged selector with a 300-second command timeout: **158 passed/0 failed/807 filtered**, 119.70s. After the meaningful partial-checkout regression/correction and superseded replay check, final **159 passed/0 failed/807 filtered**, **116.65s**, warning-free. This remains a targeted promotion selector, not the entire library gate. |

### Complete Uncommitted File Inventory

Nine source paths:

1. `src/repository/promotion/run.rs` — new complete parent runner/Direct adapter.
2. `src/repository/promotion/run_tests.rs` — new 46 named runner tests/helpers,
   real-process harness, real local-child integration seam, counters and fixtures.
3. `src/repository/promotion/mod.rs` — public types, redacted formatting and service
   entry points. Existing repository.rs wildcard re-export is sufficient.
4. `src/repository/promotion/integrate.rs` — candidate OID/resume seam and shared
   primary_clean proof visibility; existing preparation/apply algorithms reused.
5. `src/repository/remote/mod.rs` — repository-scope observation module visibility.
6. `src/repository/remote/observation.rs` — narrow snapshot/fence visibility.
7. `src/repository/remote/reservation.rs` — released-resolution continuation query
   using existing typed evidence; cfg(test)-only structural child-seeding bridge.
8. `src/repository/remote/state.rs` — narrow authority-OID/generation visibility.
9. `src/repository/remote/sync.rs` — shared scoped authenticated-action fence
   extraction; cfg(test)-only adapters to **existing** local child integration/
   reconciliation and publication planning, not production mock algorithms.

Six documentation paths:

- `docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md`;
- `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md`;
- `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md`;
- this execution ledger;
- `docs/plans/2026-10-15-wave-02-cycle-09-handoff.md`;
- `docs/research/wave-03-api-audit.md` — headless preview/promote/cancel/results/
  retry now delivered at the local gate; C5/D5 boundary prepare/execute, ten-minute
  expiry, preview schema/serialization and current F2 NotResumable remain pending.

Three canonical comments under `.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/`:

- `01M51J7Z00N4Q6S8V0X2Z4B6E2.md` — historical blocker, retained;
- `01M51K7Z00N4Q6S8V0X2Z4B6E3.md` — owner approval/resume checkpoint;
- `01M51P7Z00N4Q6S8V0X2Z4B6E4.md` — complete local-gate review handoff (uncommitted comment ID corrected to the ULID alphabet; content/history retained).

### Review / Remaining Gates

Self-review checked the full Task 6 diff against 8d06577 for public contract,
ownership-first retry, before-ref durable intent, original primary binding,
post-effect error conversion including partial checkout, configuration/generation
fences, double precheck before remote deletion, safe-point cancellation, T4-1 SQL
atomicity, T6-1 operator exception, fresh-ID unleased refresh and redaction. Original
Task 6 obligations are mapped above; there is no deferred Task 6 implementation
item. **Independent spec and quality reviews remain pending**, followed by the
controller's authorized checkpoint. No worker approval/commit is claimed.

Remaining risks/gates are explicitly bounded: real SSH transport/counter and
poll/concurrency acceptance in Task 7; full/local/native gates in Task 8; the
approved rare ambiguous-backend-lock interval needs operator intervention; held
reservation can defer indexing as IndexPending. C5/D5 own the frontend boundary.
No dependency, GPUI, CLI/desktop/F2 binding, global configuration, developer key,
sleep, subagent, staging, commit, push, PR, CI, merge, closure or cleanup action.
Ticket remains open; HEAD remains 8d06577 and all eighteen paths are uncommitted.

Final handoff checks: `git diff --check`, `git diff --cached --exit-code` and
`git diff --exit-code -- Cargo.toml Cargo.lock devenv.nix devenv.lock
.github/workflows/build.yml src/bin src/desktop src/repository/mutation` all
exited 0. Status lists exactly the eighteen intended paths above. Source has not
changed since the final 159-test GREEN; subsequent edits are documentation and
canonical comments only. Exact HEAD remains
`8d06577f6af17e2532d072d7ee14419dbf8a84b4`.

## Task 6 SPEC Corrections — Three Findings Resolved, Same SPEC Recheck Next (2026-10-16)

**All three supplied SPEC findings were verified against the approved contract and
actual seams, reproduced before production edits, and corrected. Return the full
uncommitted Task 6 diff against 8d06577 to the same SPEC reviewer before quality.**
This is implementer verification, not SPEC approval. Existing source/planning work
and the reviewed Tasks 0–5 checkpoints remain intact; no commit, new delegation,
Task 7 target or full-library gate was performed.

### Verification Against Contract And Corrections

1. **P1 — terminal cancellation after fully released resolution.** The ownership-
   first table permits a new attempt for a cancelled child only with proven absence
   of retained ownership; historical resolution alone is not unfinished ownership.
   Actual `cancellation_after_a_fully_released_resolution_stays_terminal` source
   confirms terminal cancellation after release. The runner unconditionally preferred
   `promotion_child_has_resolution` even for an already classified **Unowned,
   Cancelled** row. That repeatedly restarted a terminal ID, stranding the parent.

   The released-resolution continuation exception now applies only to nonterminal
   children. Terminal Cancelled/Failed rows follow the existing acknowledged
   retirement/new-attempt policy after actual ownership absence is established.
   Completed authoritative results and retained conflict/owned resolution are still
   classified **first**, preserving the required original-ID continuation even for
   interrupted retained ownership. No child synchronization behavior was changed.

   The two new context/primary runner regressions install a real conflict via the
   existing local child stack, fully resolve/release it through the actual resolver,
   then invoke real `synchronize_remote_observed` on the continuation with an observer
   cancelling at **BeforeFetch**. They prove that point was reached, actual durable
   phase is Cancelled and child ownership is Unowned, then retry the same parent
   request to completion with a **new child ID**, one promotion merge and no strand.
   Cancelled-ID probes use actual synchronization replay, not a fabricated error.

2. **P2 — refused replacement superseded a live operation.** The Cycle requires a
   **new confirmed promotion** before supersession. Previously begin_record's SQL
   transition fenced/cancelled the original Promote owner and superseded the parent
   before the runner checked new blockers/digest, then abandoned the replacement.

   For new requests, the runner now validates the local preflight/digest **before**
   the atomic supersession/confirmed insertion, retaining its repository lease
   through the short storage transition and dropping it before child/network work.
   Refused requests use a separate `refuse_record` path that inserts a terminal
   Abandoned row directly, compatible with the partial unique index, without
   touching the eligible original parent/reservation. The indexed unfinished lookup,
   cross-journal ID checks and insertion/encoding are factored/shared; an ineligible
   unfinished parent still yields Busy. No schema or confirmation-policy amendment.

   Both stale-digest and blocker tests start from a real runner's RemoteContextChanged
   stop with a live manual Promote reservation. They compare **every column** of the
   old parent/owner row (including epoch, phase, timestamps and flags), the full Git/
   worktree byte image, all history/retirement/current-observation rows and effect
   counts before/after refusal. The new row is Abandoned, the original remains a live
   continuation, and a subsequently valid confirmed replacement still supersedes
   and completes. Thus refusal cannot revoke consent for the prior operation.

3. **P2 — BranchDeleted replay trusted completed local cleanup without observing it.**
   The local cleanup contract requires re-observation under the lease before each
   step and T4-1 requires actual checks before tracking deletion/final SQL. Previously
   a retry from BranchDeleted could retire history after a branch/worktree/admin entry
   reappeared, because that step skipped the earlier destructive helper guards.

   Task 4's lease-owning ContextCleanup now exposes the read-only
   `require_local_cleanup_absent(kind,id)` capability seam. It checks actual direct
   branch absence (symbolic refs refuse), deterministic worktree/admin absence and
   the existing complete checkout-inventory guard. Reappeared state is Changed;
   mixed/unreadable evidence is RecoveryRequired. It never deletes that state or
   repeats previously completed destructive steps. The runner calls it under the
   cleanup lease **before tracking deletion**, and rechecks inside the final SQL
   closure immediately before complete_record while the same lease/absence guard
   remain held. A late reappearance after the independent Git effect therefore
   prevents all SQL retirement/completion/owner-finish writes via rollback.

   Actual reopen cases cover recreated branch at the old OID, a moved OID, symbolic
   branch, path-only state, admin-only state and a real newly registered worktree.
   All remain pending at BranchDeleted with byte-identical Git/local state, existing
   publication history, no retirement and a held reservation; completed effect counts
   do not repeat. A second regression creates a branch in the test-only post-tracking
   seam and proves final SQL also refuses while Git's already committed tracking
   deletion remains (T4-1). A positive complete-record replay test creates a fresh
   Edit context **after** completion, then proves stored replay preserves it with no
   child/deletion effects. Complete-record replay bypasses unfinished cleanup checks.

### Exact Correction-Pass RED / GREEN

Only new regressions were selected first, before any production correction. All
commands ran through Devenv in the same ticket worktree. No compilation/setup
failure was hidden as behavioral RED; the initial regressions compiled warning-free.

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec3_` | Initial behavioral RED **1 passed/6 failed/966 filtered**, 3.52s. Both real terminal-cancellation continuations strand; both refused replacements mutate the old parent/owner; BranchDeleted reopen and the post-tracking reappearance incorrectly complete. Positive stored-complete/fresh-context replay already passes and stays as a control. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec3_terminal` | After finding 1 correction, **2 passed/0 failed/971 filtered**, 1.63s, context and primary real cancellation after release. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec3_stale` | After finding 2 correction, **1 passed/0 failed/972 filtered**, 1.15s. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec3_blocked` | **1 passed/0 failed/972 filtered**, 1.18s. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec3_` | After all three corrections, **7 passed/0 failed/966 filtered**, 8.38s, including the six actual BranchDeleted reopen variants and the late final-SQL guard. |
| `devenv shell -- cargo fmt` | One correction-pass run, exit 0. |
| `devenv shell -- cargo fmt --check` | One correction-pass run, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion` | Broadened only after new regressions passed: **166 passed/0 failed/807 filtered**, **132.69s**, warning-free. Existing retained/interrupted/owned-resolution, record arbitration/identity, T6-1 crash, rollback and full runner cases all remain green. Not the entire library suite. |

### Correction Scope And Return

Correction source paths: `src/repository/promotion/{run.rs,run_tests.rs,record.rs}`
and `src/repository/cleanup.rs`. Storage helpers share prior identity/index/refusal
logic; cleanup adds a lease-owned observation seam without changing existing prune/
branch-delete algorithms. No new table/column/dependency or production child mock.
The seven new named regressions bring runner cases/helpers to 53. Latest plan,
handoff and API-audit local-gate summaries are updated to 166; the previous 159-test
handoff/history above remains historical evidence, not the current gate.

Canonical comment `01M51M7Z00N4Q6S8V0X2Z4B6E5` records this response. All prior
uncommitted Task 6 paths remain present, with record.rs, cleanup.rs and the new
response comment added to the previous full inventory. **Same SPEC recheck next;
quality only after SPEC acceptance.** The owner-approved T6-1 exception and pending
Task 7/8/C5/D5 gates remain unchanged. No worker commit/staging, subagent, dependency,
GPUI, global config, developer key, sleep, CLI/desktop/F2 binding, full gate, push,
PR, CI, merge, closure or cleanup action. HEAD remains 8d06577; ticket stays open.

Correction-pass final inventory checks: `git diff --check`, `git diff --cached
--exit-code` and the protected-path diff for Cargo/Devenv lockfiles/configuration,
native workflow, CLI/desktop and mutation/F2 paths all exited 0. The full pending
inventory is now 21 intended paths (prior 18 plus cleanup.rs, record.rs and this
response's canonical comment). Source is unchanged since the 166-test GREEN;
later edits are only documentation/comments. Exact HEAD remains
`8d06577f6af17e2532d072d7ee14419dbf8a84b4`.

## Task 6 Additional SPEC P2 — Discovery Outside The Common Lease (2026-10-16)

**Prior three findings reported resolved by SPEC recheck. The additional discovery/
lease P2 is now corrected, ready for the same SPEC reviewer before quality.** No
approval is inferred for this correction. The existing Cycle Bounds/global plan
rule forbids filesystem discovery while holding the common repository lease; no
owner amendment or policy exception was sought or introduced.

### Verified Finding And Complete Call-Path Audit

The previous supersession correction acquired `preflight_lease` and then invoked
preview, which reached `discovery::observe_root` over root and active contexts.
Task 3 `prepare_integration` independently acquired its lease before preview too.
Both violated the existing rule. Audit also found full filesystem canonical-context
validation underneath Task 4 cleanup prechecks and the real synchronization child's
`inspect_synchronization_local`/local target helpers. Merely moving one preview
call or dropping the lease before SQL would leave both scan violations and a
supersession TOCTOU window. All of these reachable promotion paths were addressed.

### Shared Preparation And Bounded Leased Revalidation

- `observe::prepare` performs full root/active-context discovery **without the
  common lease**, derives the selected item's discovery facts, and captures a
  process-only guard before/after discovery to reject unstable observations.
  Public preview still acquires no lease and remains read-only/offline.
- PreparedObservation::revalidate opens fresh Git handles under the caller's lease,
  checks checkout/common-dir/registration locators, local HEAD/refs, configuration
  bytes, index/state/status, and metadata of known canonical/index paths. It does
  not enumerate filesystem discovery sources or rebuild active-context discovery.
  Ignored canonical statuses are included so namespace changes cannot hide behind
  stale facts; known canonical metadata catches stat/index-flag-hidden changes.
  Checks use existing managed-entry bounds. Unix device/inode and the existing
  Win32 directory/file identity backend seal locator identity; no new dependency.
  Configuration bytes are bound even for an existing configuration indirection.
- The same actual-Git predicate consumes the sealed discovery facts and fresh
  tracking/history/conflict state. Canonical result validation reads Git trees.
  Tracking refs are re-observed rather than frozen blindly: a primary-only race
  still yields the approved PrimaryAdvanced refusal before owned objects/refs.
  The public expectation encoding and public preview schema are unchanged.
- New confirmation prepares outside, then takes the lease for bounded revalidation
  and the short atomic refusal/confirmation/supersession transaction. It also
  captures the exact indexed predecessor and owner before discovery and compares
  that guard in SQL (repository identity, whole parent record, owner epoch/phase/
  generation/requests/timestamp). A changed guard inserts only a terminal refusal
  where permitted; it never supersedes/fences the old owner. Read failures propagate
  fail-closed rather than being interpreted as a matching/absent predecessor.
- Task 3 integration uses that same prepare/revalidate split before identity/object/
  ref work. No full filesystem discovery occurs in its retained preparation lease.
- The existing canonical policy is extracted into shared source/item validation,
  bounded context identity and commit-tree validation. Cleanup uses identity plus
  the recorded clean commit's canonical tree and its existing strong clean/ignored/
  recursive-gitlink/lock checks. Synchronization preserves strict filesystem
  inspection outside its lease, then revalidates the clean target/HEAD/identity/
  canonical commit inside. Its lease-held local/recovery helpers use the bounded
  Git form. No full filesystem canonical-context scan remains in those intervals;
  target identity/cleanliness semantics remain shared rather than duplicated.
- Index-only refresh was already outside the lease and remains so. The first three
  SPEC corrections, ownership-first attempt policy, T4-1 completion and T6-1 backend
  lock exception remain unchanged and pass their existing regressions.

### Actual Scan-Entry Proof And Race Tests

Cfg(test)-only hooks are at **the actual** discovery::observe_root entry and
canonical_context_at filesystem scanner entry, not a mock preview wrapper. Each
probe uses the existing non-waiting `leases_are_free`/try-lock mechanism on the
fixture's actual common directory and a counter. No sleeps, global keys/config,
production observer bypass or transport mock pretending to be discovery.

Six new `promotion::run_tests::spec4_` cases cover:

1. New success, digest refusal and blocker: every actual root/selected-context scan
   sees a free common lease; probes must execute. Success includes integration,
   cleanup and refresh, so the audit is not limited to the initial preview.
2. Actual Task 3 preparation independently proves its discovery precedes its lease.
3. A real synchronization-context child reaches/cancels at BeforeFetch, with actual
   canonical filesystem scanner entry unleased. No credential/network is needed.
4. Seven outside/inside supersession races change primary ref, context ref, untracked
   status, configuration, checkout identity, predecessor row or owner epoch after
   prepared discovery and before lease-held revalidation/SQL. All refuse ExternalChange
   before any child/new effect, preserve the intentionally raced Git bytes/history/
   counters and preserve the old parent/owner instead of superseding them.
5. Four actual integration preparation races change context ref/status/configuration/
   identity in that interval. All refuse, preserve the post-race byte image and
   create no promotion commit or other owned object/ref effect.
6. A tracking-primary advance in the same interval returns PrimaryAdvanced, not a
   stale integration or dropped primary content; post-race images stay unchanged.

### Exact RED / GREEN And Affected Verification

| Exact command | Actual runs/results |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec4_` | Initial behavioral RED **0 passed/3 failed/973 filtered**, 0.90s: both new preflight and Task 3 preparation fail at the actual scan-entry try-lock; the supersession race returns too-late ReconfirmationRequired. One subsequent compile failure was a moved read-view helper's `&Path`/PathBuf conversion, corrected with no tests executed. Initial split GREEN **3/0/973**, 5.90s; expanded audited GREEN **4/0/973**, 5.50s; full six-case GREEN **6/0/973**, 6.10s. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec4_real_context` | Additional audit regression RED **0/1/976**, 0.51s: the real child's canonical filesystem scan still held the common lease after the first split. Fixed the shared child inspection/leased-target path, not the test; included in expanded/final GREEN above. |
| `devenv shell -- cargo test --locked --lib promotion` | Three passing broadened runs after meaningful guard refinements: **172/0/807**, 158.78s; **172/0/807**, 141.90s after explicit configuration-byte binding; final **172 passed/0 failed/807 filtered**, **134.56s**, after reusing the existing Win32 identity backend for locator seals. All warning-free. Prior seven SPEC regressions remain present and green. |
| `devenv shell -- cargo test --locked --lib repository::cleanup::tests` | **26 passed/0 failed/953 filtered**, 14.85s; targeted affected Task 4 clean/ignored/partial/identity/submodule/retirement regressions. |
| `devenv shell -- cargo test --locked --lib remote::sync::tests::local_` | **11 passed/0 failed/968 filtered**, 8.21s; targeted affected local binding, actual-state/replay, safe-point and outside-lease reconciliation cases. |
| `devenv shell -- cargo test --locked --lib remote::sync::tests::materialized_context` | **1 passed/0 failed/978 filtered**, 0.35s; existing materialized context/authoring identity and cleanliness contract. |
| `devenv shell -- cargo fmt` / `devenv shell -- cargo fmt --check` | Three correction-pass runs each, all exit 0. |

No entire-library selector, full Rust gate, Task 7 real-SSH target or native CI ran.
Linux checks do not claim Win32 runtime verification; the added Windows locator
seal reuses the already existing native backend, with native gates still deferred.

### Correction Inventory / Same-SPEC Return

Source changes for this finding are:

- `src/repository/promotion/{observe.rs,integrate.rs,run.rs,record.rs,run_tests.rs}`;
- `src/repository/{cleanup.rs,discovery.rs}`;
- `src/repository/remote/sync.rs`;
- `src/repository.rs` (shared canonical identity/content split and test-only actual
  filesystem scanner entry hook; existing promotion re-export remains unchanged).

Latest design/plan/handoff/API-audit summaries and this ledger record the actual
split/current **172**-test gate and same-SPEC recheck. Comment
`01M51N7Z00N4Q6S8V0X2Z4B6E6` records this response. A prior uncommitted handoff
comment's Crockford-excluded `L` in its ID/filename was corrected to valid
`01M51P7Z00N4Q6S8V0X2Z4B6E4`, with references updated and historical body preserved;
the original T6-1 blocker comment is unchanged. No published identity was rewritten.

All previous Task 6 changes remain uncommitted against exact HEAD
`8d06577f6af17e2532d072d7ee14419dbf8a84b4`; full pending inventory is now 25 paths
(prior 21 plus observe.rs, discovery.rs, repository.rs and the new response comment).
**Return to the same SPEC reviewer before quality.** Prior three findings are
controller-reported resolved, this correction awaits acceptance, and no review
approval or worker checkpoint is inferred. Ticket stays open. No subagent,
dependency, GPUI, CLI/desktop/F2 binding, global config, developer key, sleep,
staging, commit, push, PR, CI, merge, closure or worktree cleanup action.

Additional-P2 final checks: `git diff --check`, `git diff --cached --exit-code`
and the protected-path diff for Cargo/Devenv configuration/locks, native workflow,
CLI/desktop and mutation/F2 paths all exited 0. Status lists the 25 intended pending
paths. No source edit followed the final 172-test GREEN; subsequent changes are
documentation/comments only. Exact HEAD is still 8d06577 above.

## Task 6 Remaining SPEC P2 — Canonical Tree Entry Mode/Type (2026-10-16)

**The supplied final SPEC finding was verified, reproduced and corrected. Return
to the same SPEC reviewer for final recheck before quality.** No SPEC acceptance,
quality approval, owner policy amendment or commit is inferred.

### Finding And Minimal Correction

The shared validate_context_tree consumed canonical_tree_sources' raw byte sources
without checking the corresponding entry's mode/type. That snapshot intentionally
retains symlink Blob entries and their link bytes for whole-tree shape/invariant
analysis. Thus link bytes that happened to be valid canonical front matter could
fulfill the target, unlike the old filesystem collector which skips symlinks.

The canonical **item-source** projection in validate_context_tree now accepts a
source only when snapshot.entries proves **Blob AND mode 100644 or 100755**.
It filters neither the underlying tree nor the shared raw snapshot/entry map.
Symlinks (120000) and gitlinks cannot fulfill the target; regular executable blobs
remain valid under the already approved T2 policy. Canonical parsing/policy and
all other constraints are unchanged. Whole-tree `.manyhands/worktrees/` checks
still inspect the original complete tree, before their canonical source projection.
No scan/lease behavior, identity guard, expectation encoding, schema or dependency
changed. The production correction is confined to `src/repository.rs`.

### Tests First — Actual Cleanup And Synchronization Paths

Three new `promotion::run_tests::spec5_` cases use actual Git objects and linked
worktrees. A memory index changes the canonical document entry's recorded mode,
commits it and checks it out; assertions verify the exact tree mode/object kind.
For 120000 the Blob's bytes are the fixture's valid canonical document, and on
Unix the actual worktree entry is verified to be a symlink with those exact link
bytes (not an ordinary file containing an invalid link string).

- `spec5_cleanup_symlink_front_matter_and_gitlink_cannot_fulfill_target`:
  both 120000 and 160000 fail the real ContextCleanup precheck and removal attempt;
  HEAD/index/link presentation and registration remain preserved. This directly
  exercises the shared validator used before promotion's destructive cleanup.
- `spec5_sync_symlink_front_matter_and_gitlink_cannot_fulfill_target`:
  both modes return TargetNotMaterialized through the existing test adapter's real
  bounded local synchronization stack, before local integration effects. The
  worktree/index/HEAD image remains unchanged and no promotion commit is created.
- `spec5_cleanup_and_sync_accept_regular_and_executable_canonical_blobs`:
  both 100644 and 100755 pass cleanup precheck and the real synchronization path
  with the same clean OID/image. This prevents an overcorrection to 100644-only.

The generic snapshot reader is unchanged because other consumers need full shape
evidence. No filesystem fallback or leased discovery was added. All earlier
scan-entry try-lock/race, predecessor/supersession, terminal-cancellation and
completion-absence tests remain in the final promotion selector.

### Exact RED / GREEN And Final Targeted Gate

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec5_` | Initial behavioral RED **1 passed/2 failed/979 filtered**, 1.26s, warning-free. Cleanup incorrectly returned Ok for the 120000 front-matter link, and synchronization also accepted it; regular/executable control passed. No production fix preceded this run. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::spec5_` | After the mode/type filter, GREEN **3 passed/0 failed/979 filtered**, 1.43s, warning-free; symlink/gitlink negatives and both regular-mode positives execute. |
| `devenv shell -- cargo fmt` | One correction-pass run, exit 0. |
| `devenv shell -- cargo fmt --check` | One correction-pass run, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion` | **175 passed/0 failed/807 filtered**, **152.80s**, warning-free. Includes all six actual lease-free/race probes, all seven prior SPEC regressions, executable-document/digest tests, `result_forbids_worktrees_entries`, and original integration/record/runner coverage. Not the entire library suite. |

### Scope And Final-SPEC Return

Correction code/test paths: `src/repository.rs` and
`src/repository/promotion/run_tests.rs`. Latest design/plan/handoff/API-audit local
gate summaries plus this ledger and canonical comment
`01M51Q7Z00N4Q6S8V0X2Z4B6E7` record the response. The runner now has 62 named
tests/helpers. The complete prior Task 6 diff is retained uncommitted against
`8d06577f6af17e2532d072d7ee14419dbf8a84b4`; full pending inventory is 26 paths,
adding only the new response comment to the preceding full inventory. Same SPEC
recheck is next; quality waits for acceptance. T6-1 and Task 7/8/C5/D5 gates remain.
No subagent, dependency, GPUI, global config, developer key, sleep, CLI/desktop/F2
binding, Task 7/full gate, staging, commit, push, PR, CI, merge, closure or worktree
cleanup action. Ticket stays open; HEAD is unchanged.

## Task 6 QUALITY1 P1 — Atomic Parent/Child Start Fence (2026-10-16)

**QUALITY1's concurrent no-row attempt orphan was verified, reproduced for both
roles with real conflicts/candidates, and corrected. Return the full uncommitted
Task 6 diff against 8d06577 for quality recheck.** No review approval or checkpoint
is inferred. Prior SPEC corrections and T6-1 remain intact and green.

### Actual Race And TDD Evidence

The approved no-row Busy/yielding policy permits minting a new attempt. Before this
correction, A could persist C1 and pause before synchronization, B could bind C2
while neither child had a row, and A could start C1 after the generic ID validator
no longer found its parent binding. That made C1 an ordinary synchronization capable
of owning a conflict/candidate that the parent no longer tracked. Remote-owner
epochs cannot fence an executor which had not reserved anything yet.

The new barrier regressions use **independent RepositoryService instances and
SQLite connections** on the same disposable registry, and real Git/local child
integration helpers, not fabricated conflict labels. Channels pause A after durable
ID assignment and pause B after rebinding C2 but before either child reserves.
A is released/reaped first, then B. Both are always released/reaped before assertions.
No sleeps, common Git lease held during a pause/network, per-service execution mutex,
developer key/config, or real SSH transport is used. The ordinary transport seam
alone is injected, as approved for Task 6.

Four cases cover context/primary × installed conflict/applied clean candidate.
Two further cases pause A **after early binding validation, immediately before
reservation SQL**, proving that an entry-only check is insufficient. Initial
behavioral RED created an actual stale C1 remote row in **all six cases**, with
retained merge state; B's parent tracked C2 instead. This is ownership/effect proof,
not an outer error-category assertion.

### Private Execution Capability And SQL Linearization

The production parent handoff now carries private ChildExecution through
PromotionIo into `synchronize_promotion_child_observed`, over the existing observed
synchronization engine. It contains only existing repository/parent/item/expectation,
role, attempt count and child ID metadata, with private fields and no Debug/serde
implementation. It holds no lease, lock, transaction or credential/session content.

The child checks request identity and the current parent binding at entry. The
same fence is checked **inside the immediate reservation transaction**, before
the ordinary identity/arbitration/replay/insert logic, and inside restart and
configuration-write transactions. If a peer already replaced C1, C1 fails before
reservation/effects even though no current ID-index row points back to C1. A private
parent-issued request never silently switches to the ordinary-sync path or to
local-only binding. If reservation wins first, attempt replacement sees the actual
row/ownership in SQL, and the existing remote-owner epoch/safe-point fences protect
subsequent effects. Binding replacement and reservation cannot pass each other on
inconsistent no-row observations. No process-local mutex is the authority; existing
SQLite transaction serialization supplies the cross-process durability boundary.

The fence also checks role-appropriate parent phase and refuses new unstarted work
when the parent is cancelled/terminal/superseded. Exact **current authoritative**
child handoff remains valid after parent progress/completion and replays without
transport. Ordinary public synchronization uses the unchanged unfenced public API
with its existing ID/target/index/cancellation guards; only the private parent
adapter requires this additional fence. A child-error return while the parent has
requested cancellation reports Interrupted, retaining existing parent cancellation
outcomes rather than misclassifying start refusal as publication recovery.

No persistent claim field, table, new operation phase, lock journal or migration is
introduced. Existing audited parent attempt/binding columns are sufficient, so
schema/fingerprints remain unchanged and all existing migration/audit/privacy tests
pass. The approved no-row Busy/yielding **new ID** policy is preserved, as are
ownership-first same-ID recovery and complete authoritative/index handoff. This is
an execution-fencing correction, not a product-policy amendment or whole-parent
parallel-execution lock.

### New Passing Ownership/Concurrency Cases

All nine new names are under `promotion::run_tests::quality1_`:

- context/primary `*_conflict_rejects_stale_unstarted_attempt`;
- context/primary `*_candidate_rejects_stale_unstarted_attempt`;
- context/primary `*_atomic_reservation_rechecks_after_early_validation`;
- `parent_cancellation_fences_unstarted_children_at_both_handoff_windows`;
- `confirmed_supersession_fences_late_children_without_ordinary_fallthrough`;
- `current_authority_handoff_and_ordinary_synchronization_remain_valid`.

The six race cases now prove C1 has **no remote row and no Git/object/ref/worktree
effect**, only C2 starts, and every actual retained child in the database is exactly
the parent's current C2. Conflict cases request cancellation through the parent,
prove retained recovery remains addressable by its current ID, inspect/resolve
through the existing resolver, and retry to completion under C2. Candidate cases
remove the fixture checkpoint fault and retry/reconcile the same candidate. Each
has one promotion merge and no orphan. A stopped installed conflict is already
Interrupted, so existing cancellation need not set a live-child flag; the test
checks the parent request and actual reachable retained ownership instead.

Additional cancellation cases cover both roles at both barriers: no stale row or
effect appears after cancellation, the parent retains its stop request, and an
explicit retry resets cancellation and finishes with a new child ID. Valid confirmed
supersession cases finish a replacement while A is paused and prove the late old
frame cannot create an ordinary child afterward. Positive controls call the real
private observed entry for current completed authority (including the removed
context), reject a mismatched private request ID, and exercise legitimate ordinary
public synchronization through BeforeFetch cancellation without credentials/network.

### Exact RED / GREEN And Affected Targeted Checks

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality1_` | Initial test-hook compile failure: OperationId has no Ord for BTreeMap (3 diagnostics, no tests); corrected the test-only key to its canonical String. Behavioral RED then **0 passed/6 failed/982 filtered**, 2.24s: every stale C1 became an actual remote row. No production fence preceded this RED. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality1_` | After fencing: **3 passed/3 failed/982 filtered**, 2.80s. All orphan/effect assertions passed; three conflict cases reached a later incorrect fixture assertion expecting cancel_requested on an already inactive Interrupted child. Corrected that assertion to parent cancellation plus actual reachable retained ownership, without changing production cancellation policy. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality1_` | Six race cases GREEN **6/0/982**, 4.29s. After adding cancellation/supersession/ordinary-authority controls, expanded GREEN **9/0/982**, 9.54s. |
| `devenv shell -- cargo fmt` / `devenv shell -- cargo fmt --check` | One correction-pass run each, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion` | Final **184 passed/0 failed/807 filtered**, **163.02s**, warning-free. Includes no-row Busy/yielding policy, all ownership/resolution/cancellation/replay, lease-free/race, canonical-mode, SQL retirement and T6-1 cases. Not the entire library gate. |
| `devenv shell -- cargo test --locked --lib remote::reservation` | **45 passed/0 failed/946 filtered**, 11.62s; affected ordinary/manual/poll/sync arbitration, epoch/cancellation, private-ID/index handoff and privacy/migration tests remain green. |
| `devenv shell -- cargo test --locked --lib remote::sync::tests::local_` | **11 passed/0 failed/980 filtered**, 8.13s; affected ordinary/local-only binding, replay, collision, safe-point and local reconciliation behavior remains green. |

### Correction Inventory And Quality Return

Correction source paths: `src/repository/promotion/{record.rs,run.rs,run_tests.rs}`
and `src/repository/remote/{reservation.rs,sync.rs}`. The test-only local-child
adapter and existing injected child adapters pass the same private fence into the
actual reservation/restart gate; this is not a production mock fix. Test barriers
are outside SQL/common-lease ownership. The public synchronize request/settings/
observer signatures are unchanged.

Design, latest implementation/handoff/API-audit summaries, this ledger and comment
`01M51R7Z00N4Q6S8V0X2Z4B6E8` record the private protocol and current 184-test local
gate (71 runner cases/helpers). All prior uncommitted Task 6 work is retained;
full inventory is now 27 paths, adding only this response comment to the preceding
26. **Return to quality recheck; acceptance/checkpoint remain pending.** Exact HEAD
is `8d06577f6af17e2532d072d7ee14419dbf8a84b4`. No Task 7/full/native gate, new
dependency, GPUI, global config, developer key, sleep, subagent, CLI/desktop/F2
binding, staging, commit, push, PR, CI, merge, closure or worktree cleanup action.
Ticket stays open; T6-1 and later Task 7/8/C5/D5 gates are unchanged.

QUALITY1 final checks: `git diff --check`, `git diff --cached --exit-code` and
protected Cargo/Devenv/native-workflow/CLI/desktop/F2 diffs all exited 0. Status
lists the 27 intended pending paths, with nothing staged. Source is unchanged
since the 184-test GREEN and affected 45/11-test checks; later edits are only
documentation/comments. Exact HEAD remains 8d06577 above.

## Task 6 Quality P2 — Durable Completion Before Cancellation (2026-10-16)

**QUALITY1's original child-binding concurrency P1 is reported resolved. The
subsequent completion/cancellation P2 is verified and corrected; return to the
same quality reviewer.** No acceptance or checkpoint for this response is inferred.

The shared child helper checked `result.is_err() && parent cancellation` before
probing durable ChildState::Complete. Thus a completed Published/AlreadyCurrent
child whose outer return was lost could be mislabeled Interrupted. For primary
publication that incorrectly yielded publication pending Cancelled even though
the database already held authoritative completion; cleanup alone was unfinished.

The helper now probes authoritative completion **first** for both roles. Only the
non-complete branch classifies a lost/error return as cancellation. Stored completion
kind and exact OID win; cancellation cannot undo them. The primary path records
Published/AlreadyCurrent then stops cleanup as pending Cancelled. The context path
records ContextSynchronized, then returns Interrupted before integration. Unknown
state-read failures remain fail-closed rather than proving absence of completion.
No private execution-fence, ownership/retry policy, public type, schema or migration
changed; the production correction is solely `promotion/run.rs`.

Four new `promotion::run_tests::quality2_` regressions cover context/primary ×
Published/AlreadyCurrent. They reuse the existing injected child result seam and
real fenced reservation/durable authority storage: first complete the child,
assert the actual row is Completed with authority, call the real parent cancellation
API, then inject a lost transport return. The primary fixture advances to a real
later primary tip before completion, so the expected publication OID differs from
integration and cannot pass by returning a fallback OID.

Assertions verify primary kind/exact OID, cleanup pending Cancelled, Published
parent step, preserved context and zero early deletions. Context cases verify
ContextSynchronized/exact context OID, Interrupted and unchanged primary before
integration. All cases retry the identical parent request, preserve the child ID,
perform no second invocation/publication for that role (zero pushes for AlreadyCurrent,
one for Published), complete cleanup and retain exactly one promotion merge.

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality2_` | Behavioral RED **0 passed/4 failed/991 filtered**, 1.21s, warning-free: both primary cases incorrectly report publication pending Cancelled, and both context cases remain Confirmed instead of recording completed synchronization. No production edit preceded RED. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality2_` | After completion-first ordering, GREEN **4 passed/0 failed/991 filtered**, 1.97s, warning-free. |
| `devenv shell -- cargo fmt` / `devenv shell -- cargo fmt --check` | One correction-pass run each, exit 0. |
| `devenv shell -- cargo test --locked --lib promotion` | Final **188 passed/0 failed/807 filtered**, **188.09s**, warning-free. All nine QUALITY1 concurrency cases, prior SPEC/mode/lease-free/ownership/cancellation/replay/SQL/T6-1 cases retained. Not the full-library gate. |

Correction code/test files: `src/repository/promotion/{run.rs,run_tests.rs}`.
Latest design/implementation/handoff/API-audit evidence summaries, this ledger and
canonical comment `01M51S7Z00N4Q6S8V0X2Z4B6EA` record the response. There are now
75 named runner tests/helpers; full pending inventory is 28 paths, adding only the
new response comment to the prior 27. All prior Task 6 changes remain uncommitted
against `8d06577f6af17e2532d072d7ee14419dbf8a84b4`. Same quality recheck next;
no Task 7/full/native gate, dependency, GPUI, global config, developer key, sleep,
subagent, CLI/desktop/F2 binding, staging, commit, push, PR, CI, merge, closure or
worktree cleanup action. Ticket remains open; later Task 7/8/C5/D5 gates unchanged.

P2 final checks: `git diff --check`, `git diff --cached --exit-code` and protected
Cargo/Devenv/native-workflow/CLI/desktop/F2 diffs all exited 0. The 28 intended paths
remain pending with nothing staged; source is unchanged since the 188-test GREEN.
Only documentation/comments changed after that run. Exact HEAD remains 8d06577.

## Task 6 Final Reviews — Reviewed Complete At Local Gate (2026-10-16)

The controller/owner conveyed final review results for **the entire uncommitted
Task 6 change against exact checkpoint base
`8d06577f6af17e2532d072d7ee14419dbf8a84b4`**:

- **SPEC: PASS.**
- **QUALITY: FINAL APPROVE. No remaining findings.**
- The private child execution/concurrency fence and authoritative completion-before-
  cancellation fix are explicitly accepted, with all earlier corrections included.

**Task 6 is reviewed complete at the local gate. Controller checkpoint is next;
Task 7 follows that checkpoint.** No current Task 6 owner decision, review request,
finding or implementation item remains pending. Approvals are controller-reported
from the current instruction; no separate reviewer execution or new test evidence
is inferred. Review scope is the full Task 6 diff against 8d06577, not just the last
P2 correction. All earlier blocked/review-ready states above are historical and
superseded by this final acceptance.

Recorded verification remains **188 passed/0 failed/807 filtered**, 188.09s,
warning-free, for `devenv shell -- cargo test --locked --lib promotion`, plus the
previously recorded targeted RED/GREEN, affected reservation/local checks and
formatter/check results. **No source edit, Rust command or test rerun occurred in
this status-only pass.** No full/local/native/real-SSH gate is newly claimed.

Historical T6-1 blocker, owner approval and qualified backend-lock exception are
preserved unchanged. Rare ambiguous backend recovery requiring verified/quiesced
operator repair is the accepted contract, not a pending owner question. T4-1's
history/retirement/reservation atomicity and all accepted recovery/publication
invariants remain in force.

Current handoff and Task 6 plan summary now say reviewed complete/Task 7 after
controller checkpoint. The API audit marks headless preview/promote/cancel/results/
retry **delivered and reviewed complete**, supported only by the recorded targeted
188-test local gate. Remaining domain verification is Task 7 real SSH and Task 8
full/native gates; C5/D5 still own boundary prepare/execute, ten-minute expiry,
preview schema values/serialization and current F2 NotResumable resume binding.

This coordination pass changes only this ledger, the current handoff, implementation
plan's Task 6/final summaries, API audit and new canonical approval comment
`01M51T7Z00N4Q6S8V0X2Z4B6EB`. All previous pending Task 6 source/docs/comments are
retained for controller staging; full pending inventory is now **29 paths**. The
worker did not stage/commit, push, create a PR, dispatch CI, merge, close the ticket,
remove the worktree, delegate or start Task 7. Ticket stays open, HEAD unchanged.

Documentation-only source-integrity witnesses, captured before this pass:

- tracked `git diff --binary -- src | git hash-object --stdin`:
  `32a2db497beeaa16c91d4d2d6e54e815ac49ef0c`;
- untracked `src/repository/promotion/run.rs` Git blob:
  `2083f0252e34336b57c24631769f8dd8405a4da9`;
- untracked `src/repository/promotion/run_tests.rs` Git blob:
  `5b73bcbbeb7b328198da17051e8dc41bdac30998`.

Final staging inventory/diff checks and source-integrity comparison are reported
with this return. The controller owns the pending reviewed Task 6 checkpoint.

Final coordination verification: `git diff --check` and `git diff --cached
--exit-code` exited 0; status lists all 29 intended pending paths, nothing staged.
Both source-integrity witnesses were rechecked and match the before-pass values
exactly (tracked source diff and both untracked Rust files). No source edit/test
rerun/commit occurred. Current HEAD remains the exact checkpoint base above.

## Task 7 — Public-API SSH Evidence And Contract Blockers (2026-10-16)

**Status: BLOCKED / INCOMPLETE, not review-ready for the complete Task 7.**
The new target retains two ordinary failing acceptance cases. Neither is ignored,
converted into an expected failure, nor counted as an accepted contract. Owner
decisions below are required before claiming the requested complete evidence.
The deletion-OID correction is independently reviewable as an uncommitted fix;
it does not resolve those policy contradictions or finish Task 7.

### Authority / preflight / scope

- Fresh worker read worktree AGENTS, implementing-a-cycle/TDD instructions, approved
  implementation plan, ledger, public promotion/remote APIs and existing SSH
  harnesses; systematic-debugging was used for failures. No subagents.
- Exact worktree: `.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH`, branch
  `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Initial status was clean;
  Tasks 0–6 checkpoints are retained, Task 6 at
  `c0e972655f04ea67bb9411877b0627e7eb8439ac`.
- `git fetch origin main`, then `git rebase origin/main` in that worktree:
  no-op, before/after HEAD both c0e9726; origin/main remains
  `89d04f901636229392a311241f8e777f0a00962b`. Ancestry succeeded, status clean.
  Published ticket branch divergence was observed, not modified.
- Only targeted Rust commands through Devenv. No dependency, GPUI, global config,
  developer key, sleeps, watchdog changes, staging, commit, push, PR, native
  workflow dispatch, merge, closure or worktree removal. Task 8 gates remain due.

### T7-1 — Real deletion guard used fetched rather than published OID (fixed locally)

The first runnable case saved a document, previewed and called actual
`promote_document` against the owned SSH server. Both context and primary were
published, but promotion returned cleanup pending RecoveryRequired and
IndexPending; its `remote_context_oid` was absent. No deletion reached receive-pack.
This is a real lifecycle failure before cache loss, not an authentication fault.

Root cause: synchronization's fetched T(C) may be absent on first publication or
remain at the earlier checkpoint on subsequent publication. The parent saved
that tracking observation and passed it as the remote delete guard. Task 6's
injected deletion adapter did not check that guard against an actual server.

Current minimal correction in `promotion/run.rs`: retain `remote_context_oid`
as the bound fetched tracking evidence for Git-first local retirement, but obtain
the remote delete guard from the context child's **durable completed publication
authority** through existing `record::child_state`. Missing/non-complete authority
fails closed as pending recovery. No new field/schema/API or synchronization
behavior is introduced. Configuration/ownership fences still surround the call.

An initial attempted correction stored the published OID in the tracking field.
That broke 16 existing tests with genuinely older tracking refs; the attempt was
replaced, not retained. Tracking and verified remote authority are separate facts.
The final targeted library run restores all 188 tests without modifying them.

Public SSH regressions (actual library import, no source inclusion/private API):

- `first_publication_records_verified_context_oid`: first publish with absent
  T(C), exact promotion subject and ordered `[base, context]` parents, server
  primary equals merge, exactly three accepted post-setup commands (context push,
  primary push, exact-old-OID delete), worktree/local branch/remote context absent.
  Requires Complete, not a silently consumed IndexPending.
- `later_publication_deletes_verified_not_stale_tracking_oid`: publish an initial
  save, publicly synchronize to fetch its tracking tip, save a distinct new edit,
  promote and require accepted deletion guarded by the **new** checkpoint, with
  cleanup complete. This covers the stale-present case separately from absence.

### T7-2 — Cache-loss / R1 contradiction (owner decision required)

`cache_loss_fresh_edit_first_publication` executes real promotion/cleanup, attests
one retirement row, absent context branch/worktree, then removes the actual
registry and sidecars. It uses the existing public corrupt-cache replacement and
rebuild path to obtain the required unknown-history state, re-registers/selects
the fixture key, creates a fresh Edit context at the merged primary and performs
a distinct **public save**. The retirement count is now zero. The actual public
snapshot returns HistoryUnknown; actual synchronization returns HistoryUnknown
and leaves the context unadvertised. Both before/after workflow privacy scans pass.
No SQL history/retirement insertion, synchronization mock or fabricated result.

The original missing-store-only variant observed NeverPublished and could publish:
opening a merely absent registry treats it as a new store, so it does **not**
establish the plan's `history_unknown` premise. The retained test explicitly uses
the established corrupt-loss/rebuild path, as in `tests/remote_observation.rs`,
rather than manufacturing the flag with SQL. This distinction is important to
the evidence; plain deletion alone is not claimed to have set that flag.

Source coherence:

- R1's sole exception reads `retired_contexts` from the registry
  (`remote/state.rs`, `read_retired_contexts`, `publication_evidence_for`).
- Cache rebuild cannot recover a row erased with that registry. The global
  marker carries no per-item retirement proof.
- T4-1 explicitly states cache loss/ref absence alone is not retirement proof.
  Inferring a retirement from a reachable old promotion commit would silently
  weaken that approved rule; a later collaborator branch could have been deleted.

**Recommended minimal amendment, NOT approved:** after total registry loss with
unknown history, fresh publication remains HistoryUnknown until explicitly
recovered; R1's automatic exception requires retained durable retirement evidence.
Alternatively the owner can authorize a new independently durable proof/recovery
design. No such policy, persistence mechanism or automatic inference was added.

### T7-3 — Literal whole-store title privacy conflicts with discovery (owner decision)

`document_title_absent_from_whole_stores` saves/promotes the canary-title document,
checks workflow rows, creates a VACUUM INTO backup, and scans the whole database,
WAL and backup. Discovery has one `discovered_items.title` row equal to the title.
The literal whole-store requirement fails. The first run found title bytes in the
checkpointed registry and backup; the strengthened retained case pins a reader
before authoring and proves them in the WAL and backup (file indices 0=DB, 1=WAL,
2=backup; final observations `31 0 0`, `31 1 1`, `31 2 1`, `30 1 2`). No title or
other canary is emitted in parent output.

This is established discovery policy, not a title inserted by promotion storage:
`discovery.rs` defines `discovered_items.title TEXT NOT NULL` and populates it for
ordinary document discovery. Removing all title metadata is a broader read/cache
contract change. Promotion/remote rows and formatted public output scan clean.

**Recommended clarification, NOT approved:** titles remain permitted discovery
metadata; exclude them only from the *whole-cache* prohibition. Keep titles absent
from promotion/remote workflow rows and diagnostics/output, and keep bodies,
comments, endpoint/passphrase/key canaries absent from every whole store. If whole
cache title secrecy is literal policy, authorize the required discovery redesign.

The test helper makes this distinction explicit: each case scans ALL workflow
row columns with ALL probes (including title/worktree paths), and whole generated
stores with non-title secrets; the separate ordinary **failing** title case keeps
the literal acceptance gate red. This is diagnostic decomposition, not an approved
privacy waiver or full privacy acceptance. Comment bodies are output canaries
only so far; a real promoted comment remains part of unfinished case 1.

Five child-only negative controls deliberately emit title, body, comment,
passphrase and endpoint respectively; `capture_privacy_controls_fail_closed`
requires OutputPrivacy for all five. It also runs bounded raw-output/overflow
controls. Child raw bytes are scanned before forwarding; only fixed numeric
observations appear. A/B registries, WAL/sidecars and VACUUM backups are scanned.

### Exact commands / RED, investigation, GREEN

All commands below ran in the ticket worktree, with unchanged locked dependencies.
Initial test-authoring/setup failures are separated from behavioral RED. Temporary
fixed numeric production instrumentation used to locate the boundary was removed.

| Exact command | Actual runs/results |
| --- | --- |
| `devenv shell -- cargo test --locked --test document_promotion cache_loss_fresh_edit_first_publication -- --exact` | 15 runs: initial compile error (test mistakenly used RemotelyDeleted as publication evidence); 14 failed runnable cases during diagnosis. Initial setup correctly refused PrimaryAheadOfRemote because the fixture's anonymous seed push had not updated T(P); corrected with public primary syncs in both clones. Further runs localized the real absent deletion guard, plus one test-only wrong SQL column name. After the initial attempted guard fix the missing-store variant observed NeverPublished (not unknown); the final corrupt-loss run observed **HistoryUnknown + actual synchronization refusal**, with the title-policy scan still failing at that historical point. This selector has no claimed GREEN. |
| `devenv shell -- cargo test --locked --test document_promotion first_publication_records_verified_context_oid -- --exact` | 3 failed runs: lifecycle pending-recovery RED, then explicit absent-record assertion RED, then initial guard-fix lifecycle assertions passed but strict title store scan failed. Final green executions are in the complete target runs below; the final regression checks actual delete guard/cleanup, not an incorrectly conflated record field. |
| `devenv shell -- cargo test --locked --lib promotion` | First **172 passed / 16 failed / 807 filtered**, 172.47s with the rejected tracking-field correction. Final **188 passed / 0 failed / 807 filtered**, 160.50s, with the separate-authority fix. No existing unit tests edited. |
| `devenv shell -- cargo test --locked --lib genuine_context_conflict_resolves_canonical_and_external_and_reuses_owned_resolution` | **1 passed / 0 failed / 994 filtered**, 4.58s after the corrected guard, before the final 188-test run. |
| `devenv shell -- cargo test --locked --test document_promotion` | Two runs, both **3 SSH cases passed / 2 failed**, warning-free. First established first/stale guard GREEN, five-control GREEN and both ordinary policy failures. Final strengthened exact-merge/receive assertions and pinned-WAL proof retain the same **3/2** result. Failures are title whole-store absence and unknown-history fresh publication, not watchdog/transport issues; numeric panic locations and source assertions identify them. |
| `devenv shell -- cargo fmt --all` | Two executions, exit 0 (second immediately precedes final target run). |
| `devenv shell -- cargo fmt --check` | Exit 0. |

### Remaining full acceptance mapping / honest handoff

Numbering follows the owner's full Task 7 brief. None of the following pending
items is covered merely by the existing 188 unit tests or earlier Cycle targets.

| Owner case | Current real-SSH evidence / remaining obligation |
| --- | --- |
| 1 happy A + comment + B preserved poll | Partial: exact merge/parents/server equality/three receives/local and remote cleanup. Real comment, discovery uniqueness, second-parent Git log and materialized B poll still pending. |
| 2 local-only then configure/publish | Pending. |
| 3 AlreadyContained/fresh save/second promotion | Pending; the stale-tracking regression is a separate context publication, not a second promotion. |
| 4 all offline blockers/full A+B+server snapshots | Pending. Fixture primary observation is setup, not blocker acceptance. |
| 5 four post-integration preservation/supersession cases | Pending. |
| 6 cache-loss/unknown-history + interrupted F7 cleanup | First part reproduced as **failing T7-2**; F7 real-SSH exit/cache-loss recovery pending. |
| 7 D4 versus primary-only advance | Pending. |
| 8 real divergent conflict/public resolve | Pending for this SSH target; targeted genuine local regression passed but is not SSH proof. |
| 9 primary rejection/offline at publication boundary | Pending. |
| 10 delete rejection/raced update/tracking preservation | Pending; no claim based on successful deletion alone. |
| 11 F2 poll primary advance and retry | Pending. |
| 12 cancelled/non-owned versus retained interrupted children | Pending for real SSH. |
| 13 actual process exits F1–F12/inside F10/T6-1 repair | Pending for real SSH. Existing unit exit tests were re-run by the 188-test selector, not recast as SSH proof. |
| 14 genuine held-poll priority, inter-step polls, Busy and post-completion poll | Pending. |
| 15 remote rename/config fences | Pending. |
| 16 privacy/negative controls | Partial workflow-row/store-secret/output/control proof; whole-store title requirement is **failing T7-3**; actual comment and all remaining lifecycle states still pending. |

`Cargo.toml` declares the new harness=false target; the manual native workflow
only appends `--test document_promotion`, leaving its existing four matrix entries
and workflow_dispatch-only trigger unchanged. **Native execution pending**; no
dispatch. Required final affected SSH targets remote_synchronization,
remote_merge_recovery, remote_polling and comment_publication have **not** run in
this Task 7 session: the complete acceptance target is not ready. Task 8 broad
gate/CLI/native approval and fresh complete spec/quality reviews remain pending.

Changed files at this checkpoint: `tests/document_promotion.rs`, `Cargo.toml`,
`.github/workflows/build.yml`, `src/repository/promotion/run.rs`, this ledger,
implementation-plan status and canonical comment `01M51V7Z00N4Q6S8V0X2Z4B6EC`.
Risks: two unresolved contract decisions; incomplete requested SSH matrix; deletion
guard change requires fresh spec/quality review; all platform/native gates remain
unverified. The helper does not assert the full title secrecy requirement green.
Return is an uncommitted **blocked evidence checkpoint**, not completion or the
requested full Task 7 review-ready handoff. Ticket remains open at c0e9726.

Final checkpoint checks: `git diff --check`, `git diff --cached --exit-code`,
and `git diff --exit-code -- Cargo.lock devenv.nix devenv.lock` exited 0.
`git status --short --untracked-files=all` lists exactly the seven intended paths
above, nothing staged. Source diff contains only the separate delete-authority
lookup/argument change; no temporary production instrumentation remains. Native
workflow diff changes only its test-list line. No additional Rust run is inferred
from these documentation/status checks.

## Historical Task 7 Resume — Approved Bounds, 63-Case Review Handoff

**Superseded evidence claim:** SPEC3-P2 later identified incomplete proof in
groups 9 and 16 despite all 63 assertions passing: irreversible offline scenarios
ended without recovery, title controls only positively exercised A and a synthetic
backup probe table, and the rejected title never crossed production. Do not treat
this historical section as the corrected complete mapping. The test-only SPEC3-P2
section below records the actual repairs and final 64-case proof; original RED/
GREEN counts and implementation decisions here remain historical evidence.

**2026-10-16: IMPLEMENTATION / TARGETED EVIDENCE READY FOR FRESH SPEC THEN
QUALITY REVIEW. Not reviewed/approved/complete yet.** Full change is uncommitted
against `c0e972655f04ea67bb9411877b0627e7eb8439ac` in the specified ticket branch/
worktree. Historical T7-2/T7-3 RED and earlier partial mappings above remain as
history; their STOP/owner-pending status is superseded by the approval and evidence
here. All original Task 7 journeys now have real-SSH proof. Task 8 is still deferred.

### Owner approval and execution constraints

The owner conveyed approval of **both bounded amendments through the question
tool**. Independent review confirmed actual contradictions with Cycles 04/05
history policy and Cycle 08's accepted-discovery-title exception, not harness bugs:

1. **Retained R1 only.** Unknown-history automatic first publication requires a
   retained validated retirement row and no masking state. Registry loss erases
   that proof: HistoryUnknown persists across reopen pending explicit recovery/
   republication. Absence and old promotion merges never reconstruct it. No new
   republication API is delivered.
2. **Narrow title metadata exception.** Only existing accepted
   `discovered_items.title` metadata and DB/WAL/backup copies may contain titles.
   All workflow/promotion/retirement/remote/recovery/operation rows, formatted
   results/diagnostics and child output stay title-free. Raw whole stores,
   sidecars and backups forbid bodies/comments/endpoints/passphrases/credentials/
   key material and rejected non-indexed title canaries. No new title-bearing
   workflow write is introduced.

Cycle, design R1/recovery/privacy, plan, handoff and canonical approval comment
`01M51W7Z00N4Q6S8V0X2Z4B6ED` now record these bounds consistently. Old failures are
not reclassified as bugs or silently deleted. The renamed loss case now proves
the approved negative contract; checkpointed/pinned-WAL title cases positively
attest the permitted metadata and negatively attest workflow contamination.

This was continuation of the same active task/worktree, retaining its recorded
fresh no-op fetch/rebase at c0e9726 onto origin/main 89d04f9. No rebasing/stashing
of dirty work, no subagents, no dependency/GPUI/global-config/developer-key change,
no sleeps or watchdog changes. Every Rust command used Devenv. No staging, commit,
push, PR, native dispatch, merge, closure or worktree cleanup. The native workflow
remains workflow_dispatch-only with the same four entries and just the test added.

### Actual bugs found by the new journeys and bounded fixes

The first guard correction T7-1 above remains part of the review scope. Additional
failures were investigated before changing behavior; no violated contract was
marked expected, ignored or skipped:

| Finding | Behavioral RED / root cause / correction |
| --- | --- |
| **T7-1 deletion guard** | First-publication and stale-tracking SSH cases prove that the remote delete must use completed child publication authority, not fetched T(C). Keep fetched evidence for local retirement. Historical rejected tracking-field correction and 16 unit failures remain above; final implementation separates them. |
| **T7-4 retained R1 versus immutable Push history** | `contained_and_fresh_second_promotion` and `retained_r1_unknown_history_and_consumption` genuinely returned RemoteContextDeleted despite an earned retirement row and no state. The Push-absence scan still saw pre-retirement publication envelopes. It now validates the snapshot/retained row/no masking state and exempts only candidates contained in the row's actually retired tip. New ambiguous/verified attempts still fence absence. No ULID/time ordering, absence or historical merge reconstructs a row. |
| **T7-4 verified publication consumption** | The real first publication succeeded but retirement count remained 1. Consuming only on a later Fetch observation was insufficient. Retirement is now consumed atomically at legacy/new-attempt Push verification and authoritative classification; no consumption at intent. Additional RED controls stopped actual synchronization before classification and after a new Push intent: both initially failed. Final tests prove permission is gone at verified Push and new ambiguous work is not mistaken for old retired history. |
| **T7-5 deletion race tracking preservation** | After correctly arming Task 1's receive race after primary publication, cleanup reported RemoteContextChanged/server kept B, but A's tracking ref stayed old. Direct deletion now reuses existing exact scoped Fetch to preserve peer objects/tracking on a shared Fetch/Push endpoint, under the same owner/config fences, outside the lease. Distinct Push endpoints do not contaminate Fetch tracking. No merge or republish is performed. |
| **T7-6 allowed pre-F6 poll** | `poll_between_publication_and_f6` preserved context/primary bytes and sent no receive command, but identical retry stopped at local retirement because the poll had fetched the already-verified context into T(C). Cleanup now admits either its recorded fetched OID or that exact durable context-child authority, compared under the existing lease before guarded deletion. Arbitrary/newer/symbolic/unreadable refs still refuse; existing unit negatives passed unchanged. |
| **T7-7 F11 refresh handoff** | Real F11 process exit committed completion before discovery was marked stale. Reopen replayed Complete without refreshing obsolete discovery. Final completion SQL now also sets the existing repository refresh_required flag in the same transaction. Refresh stays outside the lease. Real F11 and after-SQL-commit/surviving-lock exits now prove primary-only discovery after replay. |
| **T7-8 source-inclusion compilation** | The required remote_synchronization command failed with 44 E0433 diagnostics: `pub(in crate::repository)` does not resolve through its source-included `production` wrapper. All 51 same-scope qualifiers in promotion record and remote helpers now use equivalent `pub(in super::super)` repository ancestry. Visibility is not widened, no public capability added, and no old test changed. All four affected targets subsequently compiled and passed. |

Two hidden FailurePoint variants support genuine public-library process exits
inside the existing F10/F11 boundary: `PromotionAfterTrackingDeletion` and
`PromotionAfterCompletionCommit`. The first runs with Git-first deletion done,
SQL not begun and evidence lock retained. The second runs after actual SQL commit
before its return is consumed/lock released; an injected non-exit error feeds the
existing actual-completion reconciliation. These add no lifecycle phase/table,
mock child, lock ownership inference or automatic lock removal.

### Fixture/oracle investigations (not production bugs)

- D5 requires absent T(C). A first push alone does not establish A's fetched
  history, and Cycle 08 polling preserves tracking. The blocker fixture now
  observes publication through a second public sync, has B delete, observes A's
  deleted state, then performs an ordinary fixture Git prune of that exact ref
  before testing D5 offline. The required zero-connection/full-snapshot assertion
  begins after this setup; present-T(C) precedence is not weakened.
- A new preview after a published-primary push needs its fetched T(P) current;
  preservation/supersession and fresh-context fixtures use an allowed ordinary
  inter-step/post-completion poll first. The original D2 policy is retained.
- Publication selection intentionally preserves the live index, per existing
  `publication_repeated_changes_accept_the_expected_stale_live_index`. The local-
  only journey starts without publication configuration, later calls the public
  selector and has the user's fixture Git tooling align only that committed config
  entry before primary sync. Earlier attempts failed WorktreeNotClean, not a
  promotion failure. No publication-selector behavior was changed.
- `reject_context_deletion` checks out the fixture context and therefore must be
  armed after its first publication. The receive race applies to the next exact
  command, so it is armed after F5, not before the primary push. Both setup mistakes
  were corrected before treating the tracking mismatch as T7-5 RED.
- A failed ordinary HistoryUnknown sync may retain its reservation; loss replay
  uses the original ID with explicit restart, and the unrelated-item control
  publicly cancels/acknowledges its failed attempt before testing another target.
- Cancelling an already parked context before its early replay can return the
  original ConflictPending. For the actual interrupted-retained cases, the known
  T0-1 SQL-trigger seam requests cancellation at the real child's new owner epoch,
  counts exactly one trigger execution, and asserts actual interrupted phase and
  unchanged conflict bytes. Both targets then resolve publicly under the original
  ID. No result or integration record is fabricated.
- A reset may mint a bound-but-unrun ID which T0 permits replacing. The first full
  target's only failure was an added over-specific `attempt == 2` oracle. It now
  requires a new ID, attempt > 1 and **exactly two executed context child rows**,
  preserving the stronger real-work assertion. No runner behavior was changed.
- A checkpointed WAL may be absent: the checkpoint variant requires the actual
  DB/backup and records absence, while the pinned reader variant requires/scans
  its retained WAL. Neither omission is accepted in the pinned variant.

### Historical owner-case mapping (63 passed; groups 9/16 had SPEC3-P2 gaps)

Every ordinary case runs in the existing pre-thread/short-path isolated child
with output privacy. Only existing public library APIs and owned fixture Git/
SQL fault seams are used; no source-included library or synchronization mock in
the new target. Existing fixture holds provide genuine concurrency without sleeps.

| Owner case | Named tests and concrete proof |
| --- | --- |
| **1 complete happy + collaborator** | `happy_comment_history_and_collaborator_poll`: actual public document save and public submit_comment, exact merge subject/ordered parents/server primary equality, doc/comment blobs on primary, local/remote branch/worktree absence, one primary discovery item, actual `git log <merge>^2` and revwalk retain edit/comment. B was materialized first; poll fast-forwards B primary and reports RemotelyDeleted while preserving B's context bytes. A's subsequent poll neither reports nor rematerializes it. `first_publication_deletes_verified_context_oid` adds exactly three accepted lifecycle commands; stale-present guard has its own separate case. |
| **2 local-only** | `local_only_then_primary_publication`: public removal of the fixture's origin, an asserted empty Git remote list and zero new connections during save/promotion, Merged, Pending(NoPublicationRemote), cleanup Complete; later public AddRemote/remote selection plus aligned committed config entry and actual primary synchronization publish a primary containing the original merge, still one promotion merge. This is genuinely zero Git remotes, not just an unselected publication remote. |
| **3 contained + fresh second promotion** | `contained_and_fresh_second_promotion`: fresh Edit starts at primary, contained promotion creates no merge, a distinct new save has that primary parent and publishes first, then a second real promotion yields exactly two promotion merges and primary-only discovery. |
| **4 all blockers** | Seven `blocker_*` cases: dirty/conflicted/not-checked-out/ahead primary, dirty context, behind fetched context, observed remote deletion with absent T(C). Both preview and execute refuse; zero accepted sockets and exact recursive content/name/type hashes of **both clones and the bare server**, including Git indexes/refs/registrations, remain unchanged. |
| **5 post-integration preservation** | Four `preserve_*_then_supersede` cases stop after publication, add tracked edit/untracked/locally-excluded ignored file/worktree lock, retry old ID, require ContextChanged/IgnoredFiles, server context intact and full context/worktree administration/branch unchanged. User saves the edit, removes the file or unlocks; allowed poll/new preview/new ID supersedes the old row and completes. |
| **6 retained/lost history** | `retained_r1_unknown_history_and_consumption`: actual public corrupt-cache recovery while context is published, real sync/promotion earns retirement, reopen/fresh Edit/distinct save successfully publishes with unknown flag still 1, retirement consumed. Other item in same repo remains HistoryUnknown/no receives; subsequent remote deletion blocks ordinary republication. `cache_loss_fresh_edit_remains_unknown_across_reopen`: actual loss erases retirement, public rebuild/key restore/fresh Edit/save stays HistoryUnknown, no receive update, remote absent/local context and bytes preserved; original-ID reopen/restart remains unknown/no row. `f7_cache_loss_new_confirmed_cleanup_earns_retirement`: actual F7 process exit, loss/rebuild, contained cleanup with a new confirmation/ID performs no second merge or push, earns a new row and permits a distinct subsequent first publication. Extra `r1_*` controls prove verified-before-authority consumption and no exemption for a new ambiguous Push intent. |
| **7 D4 versus primary-only** | `new_context_content_reconfirms`: B pushes a new path after A preview; A's real child synchronizes but promotion refuses before primary integration; new preview lists B path and changes digest, then succeeds once. `primary_only_advance_preserves_consent`: B's already-published other-path primary change is integrated without reconfirmation and retained on server. |
| **8 genuine conflict** | `real_context_conflict_public_resolution`: actual divergent context merge/index conflict; public inspect/read/resolve chooses the local canonical side, resolved two-parent checkpoint is checked, identical parent request retries once and produces one promotion merge. No mocked child or private resolver. |
| **9 publication failure/offline boundary** | `primary_rejection_retry_only_unfinished`: receiver rejects primary, server has no merge, local/remote context retained; clearing fault/retry gives one promotion and exactly three accepted lifecycle updates overall. Historical offline cases proved only the pre-/post-integration refusal/pending boundary, then ended with the fixture shut down; they did **not** prove restoration/identical-request retry. SPEC3-P2 closes that gap below. |
| **10 rejection and actual deletion race** | `delete_rejection_retry_preserves_local_state` uses Task 1 native receiver fault, preserves bytes/ref and retries successfully. `delete_context_race_preserves_peer_and_tracking` uses B's real commit and Task 1 advertise/receive race, requires RemoteContextChanged, B OID on server **and A tracking**, unchanged local context. |
| **11 automatic F2 poll** | `poll_f2_primary_advanced_retry_integrates_once` uses an actually eligible Automatic invocation with an explicitly unlocked session after F2. B's other-path published commit advances T(P)/primary; integration refuses PrimaryAdvanced with primary unchanged, no promotion object; retry executes a new context child (exactly two real child rows total), creates one promotion and keeps B's tree/server primary. |
| **12 both child roles** | `cancelled_{context,primary}_attempt_replaced_once` publicly cancels the actual bound child during credential callback, requires resumable outcome/new ID/attempt 2 and exactly one accepted push per branch plus delete. `interrupted_retained_{context,primary}_keeps_child_id` uses actual parked conflict and T0 epoch cancellation seam, asserts interrupted durable phase/original ID/attempt 1, then public resolution and successful same-ID retry. |
| **13 process exits and SQL split** | Twelve distinct `exit_f1_*` through `exit_f12_*` cases launch the actual public-library exit service in a separate process, require exit 86, assert exact record step/ref/worktree/server/candidate inventory and pre-exit receive counts, reopen and retry only unfinished work, then replay equality/one merge/one push each/one delete/absent local context/primary-only discovery. Both inside-F10/F11 exits and all SQL fault cases first publicly fetch an **existing T(C)** after F5, assert it equals the published context, and exercise its actual guarded Git deletion (not just pre-existing absence). `exit_inside_f10_git_sql_split_operator_repair` first preserves existing history/no retirement/held reservation/backend lock with Ok pending RecoveryRequired and unchanged receives; only then removes the exact verified empty lock in the quiesced disposable fixture and completes identical retry. `exit_after_sql_commit_replay_preserves_lock` proves stored completion replays without removing the surviving lock. Statement/deferred-commit rollback cases preserve history/retirement/parent/reservation separately from already-deleted Git tracking; unknown commit-result case reconciles actual committed completion. Normal exits also cover verified-absence reconciliation. |
| **14 genuine priority and inter-step polls** | `automatic_poll_advertisement_yields_to_promotion` uses a real hold and scoped thread, observes first child PollYielding, poll interruption and successful new attempt. `poll_between_publication_and_f6` proves no relied-on checkout/primary mutation/receive command and successful identical retry. Five `poll_busy_at_f6` through `f10` cases require Automatic poll Busy/zero connections/full clone/server snapshots until completion; post-completion polls have no deleted/rematerialized A context. |
| **15 configuration/generation fence** | `remote_rename_before_integration_fences` and `remote_rename_after_integration_is_pending`: real Git remote rename/config commit between steps, exact after-change images/receive/socket counts unchanged by continuation, pre-effect Interrupted versus post-effect Ok cleanup RecoveryRequired. |
| **16 bounded privacy, historical partial proof** | Generic helpers scanned A/B workflow/backup rows with all probes and raw stores with non-title secrets. The two positive title cases only attested **A** live/backup/WAL, their negative control only used A's synthetic remote_title_probe table, and REJECTED_TITLE was only a declared/direct scanner probe. Actual comments and five output/overflow controls were covered, but bilateral positive/actual-workflow-row controls and a production-rejected title input were missing. SPEC3-P2 supplies those proofs below without widening any scanner exception. |

### Exact commands and counts — resumed work

Commands ran in the specified worktree. The same expanded `-- --exact` commands
were used inside ordered shell loops; loops stopped at the first failing case,
and an unexecuted tail is not reported as a test run. No watchdog increase,
blanket skip, dependencies or ignored tests were used. Historical pre-approval
commands/counts above are retained separately. Two case labels were clarified at
the final checkpoint: first_publication_records_verified_context_oid →
first_publication_deletes_verified_context_oid; cache_loss_fresh_edit_first_publication
→ cache_loss_fresh_edit_remains_unknown_across_reopen.

| Exact command / selector | Actual resumed results, in execution order |
| --- | --- |
| `devenv shell -- cargo test --locked --test document_promotion privacy_` | **2 passed / 1 failed**: checkpoint WAL absence setup; pinned-WAL and five-control case passed. |
| `devenv shell -- cargo test --locked --test document_promotion happy_comment_history_and_collaborator_poll -- --exact` | Compile failure: three test-only API/IndexEntry diagnostics; then **1 passed**, initially one unused race-hook Result warning, fixed before later commands. Final full-target runs are warning-free. |
| `devenv shell -- cargo test --locked --test document_promotion blocker_` | **6 passed / 1 failed**, D5 present-history/tracking prerequisite setup. |
| `devenv shell -- cargo test --locked --test document_promotion blocker_remote_deleted -- --exact` | **0/1**, then **1/0** after authentic observed-publication and explicit exact-ref prune setup. |
| `devenv shell -- cargo test --locked --test document_promotion preserve_` | **0 passed/4 failed**, then **4/0** after T(P) was publicly observed before a new confirmation. |
| `devenv shell -- cargo test --locked --test document_promotion preserve_untracked_then_supersede -- --exact` | **0/1**, fixed numeric checkpoints located the new-preview D2 setup issue. |
| `devenv shell -- cargo test --locked --test document_promotion privacy_metadata_checkpointed -- --exact` | **1/0**, checkpointed DB/backup with permitted absent WAL. |
| `devenv shell -- cargo test --locked --test document_promotion cache_loss_fresh_edit_first_publication -- --exact` | **0/1** on a fresh-ID retry against a held failed reservation; then **1/0** with actual original-ID restart after reopen. Final label is the negative loss/reopen name above. |
| `devenv shell -- cargo test --locked --test document_promotion local_only_then_primary_publication -- --exact` | Seven **0/1** investigations, then **1/0**. Numeric diagnostics separated initial configured-then-disabled setup, actual no-publication initial configuration, and the documented stale live index after public remote selection. A later stronger zero-Git-remotes/public AddRemote control also passed **1/0**. No selector contract/production change. |
| `devenv shell -- cargo test --locked --test document_promotion contained_and_fresh_second_promotion -- --exact` | Three **0/1** runs (D2 setup, then actual old Push-history refusal with earned retirement); then **1/0** after T7-4. |
| `devenv shell -- cargo test --locked --test document_promotion retained_r1_unknown_history_and_consumption -- --exact` | Four **0/1** runs: actual old Push-history refusal; retained permission not consumed on successful Push (two probes, count 1); other-item failed reservation blocking a different target. Then **1/0** with all retained-R1/other-item/deletion controls. Additional GREEN executions occurred in the `r1_` filter/full target, not silently counted as extra distinct cases. |
| `devenv shell -- cargo test --locked --test document_promotion r1_` | **1 passed/2 failed** (new verified-before-authority and ambiguous-new-intent controls RED); then **3/0** after verified-checkpoint consumption/retired-tip qualification. |
| `devenv shell -- cargo test --locked --test document_promotion new_context_content_reconfirms -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion primary_only_advance_preserves_consent -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion real_context_conflict_public_resolution -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion primary_rejection_retry_only_unfinished -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion offline_before_context_is_preintegration_error -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion offline_after_integration_is_publication_pending -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion delete_rejection_retry_preserves_local_state -- --exact` | **0/1** (fault armed before context existed), then **1/0** after setup correction. |
| `devenv shell -- cargo test --locked --test document_promotion delete_context_race_preserves_peer_and_tracking -- --exact` | **0/1** (race consumed by primary push), **0/1 actual tracking-preservation RED**, then **1/0** after T7-5. |
| `devenv shell -- cargo test --locked --test document_promotion poll_f2_primary_advanced_retry_integrates_once -- --exact` | **1/0**; after full-target attempt-number oracle correction, **1/0** again. Final full target additionally proves actual Automatic invocation, not just explicit polling. |
| `devenv shell -- cargo test --locked --test document_promotion cancelled_` | Compile failure: three test API-return diagnostics; then **2/0**, both public child-role cancellation cases. |
| `devenv shell -- cargo test --locked --test document_promotion interrupted_retained_` | **1 passed/1 failed** (context early replay returned ConflictPending); final **2/0** at the actual T0-1 epoch-interruption boundary. |
| `devenv shell -- cargo test --locked --test document_promotion interrupted_retained_context_keeps_child_id -- --exact` | **0/1** characterization confirmed original ConflictPending before choosing the real epoch seam. |
| `devenv shell -- cargo test --locked --test document_promotion automatic_poll_advertisement_yields_to_promotion -- --exact` | **1/0**, real held advertisement/scoped worker. |
| `devenv shell -- cargo test --locked --test document_promotion poll_between_publication_and_f6 -- --exact` | **0/1 actual tracking-advance RED**; one compile error in the correction (typed map_err closure); final **1/0** after T7-6. |
| `devenv shell -- cargo test --locked --test document_promotion poll_busy_` | **5/0**, all five actual Automatic Busy boundaries. |
| `devenv shell -- cargo test --locked --test document_promotion remote_rename_` | **2/0**, pre-/post-integration outcomes. |
| `devenv shell -- cargo test --locked --test document_promotion exit_inside_f10_git_sql_split_operator_repair -- --exact` | Compile RED: three missing inside-F10/F11 FailurePoint uses; then **1/0**, genuine subprocess exit/lock-preservation/operator-repair/identical retry. A later strengthened existing-T(C)-deletion setup also passed **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion exit_f` | **11 passed/1 failed**: real F11 stale-discovery crash bug. |
| `devenv shell -- cargo test --locked --test document_promotion exit_f11_after_completion -- --exact` | **0/1** boundary localization, then **1/0** after atomic stale marker. |
| `devenv shell -- cargo test --locked --test document_promotion exit_after_sql_commit_replay_preserves_lock -- --exact` | **1/0**; later strengthened existing-T(C)-deletion setup **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion f7_cache_loss_new_confirmed_cleanup_earns_retirement -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion completion_statement_rollback_preserves_history -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion completion_commit_rollback_preserves_history -- --exact` | **1/0**, actual deferred SQL commit failure/rollback. |
| `devenv shell -- cargo test --locked --test document_promotion completion_unknown_commit_result_reconciles -- --exact` | **1/0**, actual committed SQL plus lost return. |
| `devenv shell -- cargo test --locked --test document_promotion completion_` | **3/0** with existing-T(C) actual deletion in statement rollback, deferred commit rollback and lost commit-return cases. |
| `devenv shell -- cargo test --locked --test document_promotion privacy_metadata_` | **2/0** after strengthening the title allowance to exactly one cache column and backup-row contamination controls. |
| `devenv shell -- cargo test --locked --lib promotion` | **188 passed / 0 failed / 807 filtered**, 182.11s, after all behavioral fixes. Later relative-visibility-only changes preserve the same repository scope and were compiled by the affected source-included targets/public-library target; no extra unit runtime rerun is inferred. |
| `devenv shell -- cargo test --locked --test document_promotion` | Six resumed full runs: **62 passed/1 failed** (attempt-number oracle), then five runs of **63 passed/0 failed**. Reruns were justified by that failure, relative-visibility fix plus stronger per-column privacy controls, actual Automatic F2 invocation and LF-independent/plain+encrypted-fragment probes, strengthening inside-F10/F11/SQL cases to prove deletion of an existing fetched tracking ref, and the final zero-Git-remotes/public AddRemote input control. No unchanged-tree blanket repetition. Final source tree passed; no further Rust edits followed. |
| `devenv shell -- cargo test --locked --test remote_synchronization` | First compile failed with **44 E0433 diagnostics**, no tests; after equivalent relative scopes **63 SSH cases passed**, warning-free. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery` | **25 SSH cases passed**, warning-free, one complete run. |
| `devenv shell -- cargo test --locked --test remote_polling` | **84 SSH cases passed**, warning-free, one complete run. |
| `devenv shell -- cargo test --locked --test comment_publication` | **41 SSH cases passed**, warning-free, one complete run. |
| `devenv shell -- cargo fmt --all` | Five resumed executions, exit 0; last immediately before final format-check/63-case run. |
| `devenv shell -- cargo fmt --check` | Four resumed executions, all exit 0. |
| `devenv shell -- cargo clippy --locked --test document_promotion -- -D warnings` | Optional targeted probe **FAILED in existing library code with 22 pre-existing lints**, before it could finish test-target linting: 14 result_large_err, four needless_borrow, three collapsible_if, one unnecessary_unwrap. Unchanged identity/integrate/observe/mod/inspection code and unchanged Task 6 control-flow sections match the baseline; no lint suppression, broad gate, or Clippy GREEN is claimed. Carry to Task 8, whose required full gate was explicitly deferred. |

### Changed files, review boundary and remaining gates

Review the **entire uncommitted Task 7 diff against c0e9726**, not merely the
last fix. Intended inventory (21 paths after both new comments):

- `tests/document_promotion.rs` and `tests/document_promotion/{journeys,advanced,exits}.rs`.
- `Cargo.toml`, `.github/workflows/build.yml` (new harness=false target / appended
  manual native test only; same four matrix entries).
- `src/repository.rs` (two hidden inside-F10/F11 test seams).
- `src/repository/promotion/{run,record}.rs` (authority/tracking/race/refresh/seams
  and equivalent record visibility).
- `src/repository/remote/{observation,reservation,sync,state}.rs` (endpoint equality,
  verified-R1 consumption/old-new history qualification; equivalent relative
  scopes; **state has visibility-only changes**).
- Cycle 09, design, implementation plan, this ledger and handoff.
- Canonical comments EC (historical blocker), ED (approved bounded amendments),
  EE (this full review-ready checkpoint) under the existing ticket directory.

No old SSH regression test, lockfile, dependency, GPUI module or global config
was edited. All 16 original obligation groups are covered above; no Task 7 product
policy question remains. Fresh independent spec review followed by quality review
is still required for these implementation changes. The controller owns the
checkpoint commit only after those reviews. **Task 7 is review-ready, not complete.**

Remaining risks/gates are explicit: native Windows/macOS/other architectures have
not run; the current four-target workflow is only wired, never dispatched. Rare
ambiguous backend locks still require the approved operator intervention. Native
file-handle/long-path/locking behavior remains unproved by Linux. Task 8 must run
full Rust/CLI/merge integration checks, resolve the carried baseline Clippy findings
and complete final review/approval. No full all-features check/test/Clippy or CLI
smoke, push, PR, native dispatch, merge, closure or cleanup was performed here.

Final handoff checks (after the last 63-case run; documentation only changed
afterward): `git diff --check` and `git diff --cached --exit-code` exited 0.
`git diff --exit-code -- Cargo.lock devenv.nix devenv.lock
tests/remote_synchronization.rs tests/remote_merge_recovery.rs
tests/remote_polling.rs tests/comment_publication.rs
src/repository/bridges/identity.rs src/repository/promotion/integrate.rs
src/repository/promotion/observe.rs src/repository/promotion/mod.rs` exited 0,
confirming unchanged lockfiles/original regression tests and untouched sections
containing the carried library lints. Status lists exactly the 21 intended paths,
nothing staged. HEAD remains c0e972655f04ea67bb9411877b0627e7eb8439ac and origin/main
89d04f901636229392a311241f8e777f0a00962b. No Task 7 commit/review approval is inferred.

## Historical SPEC3-P2 — Test-Only Evidence Corrections (2026-10-16)

SPEC subsequently passed, as conveyed by the owner. QUALITY2 then found two
implementation issues and a fixture-image minor; the current correction/recheck
section below supersedes this 64-case handoff without erasing its actual results.

**Current: corrections implemented and targeted-verified; resubmit to SPEC, then
QUALITY. Not approved/complete.** The earlier 63-case all-covered claim is
withdrawn for its identified evidence gaps, not retrospectively marked complete.
Final full `document_promotion` target now passes **64 cases**, including every
prior journey and one new real rejected-input case. No product-policy or production
code change, shared fixture edit, dependency, skip, mock child or sleep.

### Review verification and integrity boundary

Read the review against actual `offline_before`/`offline_after` and
`privacy_title_policy`: offline cases called shutdown and ended; B never received
the accepted title in these privacy variants; negative contamination was synthetic
and A-backup-only; REJECTED_TITLE was never submitted. These were real coverage
gaps. The existing fixture already supports a reversible SSH advertisement
disconnect, and existing canonical authoring already rejects NUL in titles; no
new outage fixture API or validation policy was needed.

Re-read worktree AGENTS and used receiving-code-review/TDD/debugging/verification
guidance. This is continued work on the same dirty ticket branch at exact base
c0e972655f04ea67bb9411877b0627e7eb8439ac, preserving earlier work and preflight.
Before/after `git diff -- src | git hash-object --stdin` both equal
**a9d8060139dd15f7a76b3e125ae31e08d900b7d4**. `git diff --exit-code -- tests/support`
exited 0. Thus this correction changes no production bytes or shared fixtures;
prior affected-target and 188-unit evidence is retained, not falsely rerun here.

### P2-1 reversible real SSH outages and identical-request recovery

The RED first extended both old shutdown cases with restoration/identical-request
completion assertions: both failed. Clearing a transport fault cannot revive a
stopped listener. The GREEN uses **existing** persistent
`disconnect_at(FixtureBoundary::Advertisement)` and `clear_fault`, preserving the
same listener endpoint, repository and server identity/key. It exercises actual
SSH/helper connections; no mocked child or replaced endpoint.

Both pre-context and after-integration cases now:

- Capture the original parent request (same operation ID/digest/public approval),
  endpoint/host identity, server primary, local primary/context and worktree bytes.
- Assert the real transport failure's appropriate outcome: pre-context error with
  zero promotion merges and absent remote context; post-F4 Ok publication
  Pending(Transport)/cleanup AwaitingPublication with the existing server context
  preserved at its exact OID and server primary still at the original base.
- Assert no receive update during the outage, unchanged local primary/context/
  bytes, actual failed role child ID/attempt 1 and zero integration/resolution
  rows belonging to that child. Scan pending workflow/store privacy.
- Clear the actual fault, assert same endpoint/server key, and retry the identical
  parent request to completion. Actual stored role child ID changes and attempt
  becomes 2; old child is retired into a nonactive failed/cancelled/interrupted
  phase. There is exactly **one promotion merge** and exactly **three accepted
  lifecycle commands overall**: one context push, one primary push, one delete.
  No rejected/no-op/extra receive command is accepted as unfinished-work proof.
- Require final local/remote context cleanup, server primary equality, primary-
  only discovery and final privacy. The pre-context retry is covered too.

### P2-2 bilateral accepted metadata and actual workflow-row negative controls

Both checkpointed and pinned-WAL variants now author/save/promote in A, then use
B's **public poll** to discover the accepted title independently. Reader snapshots
are created for **both** clone databases before production writes when pinning;
the checkpointed variant requires successful actual WAL truncation checkpoints.

Before any negative injection, each variant asserts:

- Exactly one accepted `discovered_items.title` row in live A, live B, backup A and
  backup B. Both backups are actual VACUUM INTO copies of their respective stores.
- Actual title bytes in both checkpointed DB files/both backups, or both retained
  WAL files/both backups while the pre-write readers are pinned. Numeric witness
  `[31, side, file, present]` uses side 0=A/1=B and file 0=DB/1=WAL/2=backup. The
  checkpoint variant permits absent/empty WAL; the pinned variant requires it.
- Existing all-column workflow scans/all-cache scans and raw non-title-secret
  whole-store/sidecar/backup scans pass in both clones.

After dropping the pinning readers, the same negative helper runs on all four
datasets. It selects a **real existing `operation_records` journal row** and updates
its free-text `target` metadata to a fixture string containing TITLE. It does not
change action/state/identity/fixed categories, insert synthetic workflow rows,
disable constraints or add a probe table. UPDATE must succeed for exactly one row
and the selected row must actually contain TITLE. Both unchanged scanners must
reject it with the complete probe set, including title; the discovery-title
exception is still limited to `discovered_items.title`.

Each helper begins with a clean scan and hashes **all columns of all operation
rows** as a known snapshot, checks it differs while poisoned, explicitly rolls
back the transaction, then requires exact snapshot and original selected-field
restoration plus clean scans. No service call runs against poisoned metadata.
Numeric `[32, side, store]` witnesses show all four controls, with store 0=live,
1=backup. A test-led intermediate synthetic-table implementation deliberately
failed the new actual-operation-row witness even though its scanner rejection
would otherwise pass; the final code has no synthetic probe-table control.

Raw body/comment/endpoint/passphrase/plain+encrypted key/fragment and rejected-title
probes, both row-scanner variants, all five output negatives and raw/overflow
controls remain strict and unchanged. Positive title copies are inspected before
negative injection so test-written bytes cannot masquerade as production metadata.

### P2-3 rejected title crosses public canonical authoring validation

New ordinary case `privacy_rejected_nonindexed_title_input` first establishes a
valid saved document, then submits a real public `save_document` edit whose title
is REJECTED_TITLE plus NUL (and whose body carries the existing body/comment
canaries). Existing canonical validation returns **InvalidPath** before writing.
The test formats/scans the actual domain error, never the private request/draft,
and checks public SaveDocument operation/Display for the specific title/NUL
validation reason, rather than accepting an unrelated InvalidPath refusal.
It attests a journal row for the exact attempted operation, unchanged source
bytes/branch OID, no rejected title in either public discovery snapshot or either
discovered-items table, then scans both stores/sidecars and fresh backups with
the full strict rejected-title/secret probe set. The sentinel is no longer merely
a declared probe or ignored fixture file. Initial test-evidence RED was zero
records for the as-yet-unsubmitted request; invoking the public boundary supplies
the real rejection witness. This is an evidence-workload RED, not a claim that
production canonical validation was broken.

### Corrected current mapping — all owner groups retained, 64 passing cases

| Owner group | Current proving cases / correction |
| --- | --- |
| 1 happy/comment/collaborator | `happy_comment_history_and_collaborator_poll`, first/stale deletion guards; unchanged and passed in the 64-case run. |
| 2 local-only/later publication | `local_only_then_primary_publication`; unchanged zero-Git-remotes/public AddRemote proof passed. |
| 3 contained/fresh second merge | `contained_and_fresh_second_promotion`; unchanged and passed. |
| 4 offline blockers/full snapshots | All seven `blocker_*`; unchanged and passed. |
| 5 preservation/supersession | All four `preserve_*_then_supersede`; unchanged and passed. |
| 6 retained/lost/F7 R1 | Retained R1, new verified/ambiguous Push guards, loss-across-reopen and F7 newly earned retirement; unchanged and passed. |
| 7 D4/primary-only invariance | `new_context_content_reconfirms`, `primary_only_advance_preserves_consent`; unchanged and passed. |
| 8 actual conflict/resolution | `real_context_conflict_public_resolution`; unchanged and passed. |
| **9 rejected/offline publication** | Receiver rejection remains. **Both** `offline_before_context_is_preintegration_error` and `offline_after_integration_is_publication_pending` now use reversible real SSH disconnects, attest server/local preservation while pending and restore/complete identical requests with actual retired/new child IDs and exact once-only receives/merge count. |
| 10 deletion rejection/race | Both existing deletion fault cases; unchanged and passed. |
| 11 actual Automatic F2 poll | `poll_f2_primary_advanced_retry_integrates_once`; unchanged and passed. |
| 12 both child-role cancellation/retained interruption | Four existing cases; unchanged and passed. |
| 13 F1–F12/inside-F10/F11/SQL/T6-1 | All existing exit and SQL-fault cases, including actual existing-T(C) deletion, pending-before-operator-repair and committed replay retaining locks; unchanged and passed. |
| 14 genuine priority/inter-step/Busy polls | Held advertisement, pre-F6 poll and five Busy cases; unchanged and passed. |
| 15 pre-/post-effect config fence | Both remote rename cases; unchanged and passed. |
| **16 bounded privacy** | Both `privacy_metadata_*` now prove positive metadata in **all four datasets** and checkpoint/pinned-WAL copies, plus real existing operation-row poisoning/rejection/rollback in **all four datasets**. New `privacy_rejected_nonindexed_title_input` exercises public canonical rejection and strict error/discovery/raw-store absence. Five output negatives/overflow controls and every-case strict row/raw-store scans remain. |

### Exact SPEC3-P2 commands, RED/GREEN and rerun scope

All Rust commands used Devenv in the specified worktree. No broad gate, original
fixture/watchdog change, expected-failure conversion or extra policy amendment.

| Exact command | Actual results |
| --- | --- |
| `devenv shell -- cargo test --locked --test document_promotion offline_` | RED **0 passed / 2 failed** with retry assertions on the old irreversibly shut-down fixture; GREEN **2 passed / 0 failed** after using/restoring the existing real SSH disconnect fault and adding server/attempt/receive preservation proofs. |
| `devenv shell -- cargo test --locked --test document_promotion privacy_` | Three runs: RED **1 passed / 3 failed** (B positives missing, new rejected-input operation not submitted; output controls passed); intermediate **2 passed / 2 failed** (bilateral positives and actual rejection now passed, synthetic control failed actual-operation-row witness); GREEN **4 passed / 0 failed** with all four real-row mutation/rollback controls. |
| `devenv shell -- cargo fmt --all` | Two executions, exit 0 (final reason assertion checked through the public error API). |
| `devenv shell -- cargo fmt --check` | Three executions, all exit 0. |
| `devenv shell -- cargo test --locked --test document_promotion` | One final complete execution after all corrections: **64 SSH cases passed / 0 failed**, warning-free. All earlier journeys remained enabled and passed; new case raises count from 63 to 64. |
| `devenv shell -- cargo test --locked --test document_promotion privacy_rejected_nonindexed_title_input -- --exact` | After the full run, the narrower field-reason assertion initially hit one test-authoring compile error (RepositoryError.message is private). It now uses only public operation/Display; **1 passed / 0 failed**, warning-free. No production accessor or policy was added. All other cases unchanged, so no redundant full-target rerun. |
| `git diff -- src \| git hash-object --stdin` | Before/after both **a9d8060139dd15f7a76b3e125ae31e08d900b7d4**; existing Task 7 production changes preserved byte-for-byte, no new production policy/code edits. |
| `git diff --exit-code -- tests/support` | Exit 0, shared SSH fixture/harness/privacy helpers untouched. |

The four required affected SSH targets were **not rerun** in this correction:
neither their production code nor shared fixtures changed. Their existing results
remain 63 synchronization / 25 merge recovery / 84 polling / 41 comment publication;
188 promotion unit results are also retained, not newly executed. Task 8/full/
native gates and the carried baseline Clippy findings remain pending.

### Files and review return

This correction changes only `tests/document_promotion.rs`,
`tests/document_promotion/journeys.rs`, the five Cycle 09 evidence/plan/design/
handoff documents, and canonical review-correction comment
`01M51Y7Z00N4Q6S8V0X2Z4B6EF`. The earlier full uncommitted Task 7 change is retained;
total intended pending inventory is now **22 paths**, including four new test
files and four canonical comments. No changes to advanced/exits test journeys,
shared fixtures, production, dependencies, lockfiles, GPUI or global config in
this revision. No subagents, staging, commit, push, PR, native dispatch, merge,
closure or worktree cleanup. Return the whole uncommitted Task 7 diff against
c0e9726 for fresh SPEC recheck followed by QUALITY; no review approval inferred.

Final SPEC3-P2 checks: `git diff --check` and `git diff --cached --exit-code`
passed; unchanged shared fixtures/lockfiles/original four affected test sources
were verified with `git diff --exit-code`. A broader invariance probe including
Cargo/native wiring showed their already-pending original target additions, not
new changes; the unchanged-file check excludes those intentionally dirty paths.
Status is the expected 22 paths, nothing staged. HEAD remains c0e9726 and the
production diff fingerprint still matches the before-work witness above.

## QUALITY2 — Owner Fencing, All Verified R1 Routes And Framed Images

**2026-10-17 UTC: all two important findings and the minor implemented and
targeted-verified; ready for QUALITY recheck, uncommitted.** SPEC PASS was conveyed
on the preceding 64-case revision. No approval of this new correction is inferred.
Final full target: **74 SSH cases passed**, with all prior SPEC3-P2 journeys still
enabled. The final production correction was followed by fresh **63/25/84/41**
affected SSH target passes. Task 8/full/native gates remain deferred.

### Verification before implementation / execution boundary

Read the reported sites and the existing claim/epoch/heartbeat/persist protocol.
The scalar refresh_required update could indeed be cleared by an older still-valid
index owner after its two observations. The generic verified-checkpoint and
advance wrapper consumed R1, but legacy reconciliation, settlement's direct state
advance and already-current candidate binding did not. The image helper really
concatenated unframed names/payloads and gave same-byte symlink/regular nodes the
same representation, without execution-mode evidence. Each was reproduced before
its implementation correction; no policy amendment, mock return or blanket skip.

This is continued work in the same ticket worktree/branch at exact HEAD
`c0e972655f04ea67bb9411877b0627e7eb8439ac`, retaining the earlier active-task
preflight and all pending work. Read AGENTS and used the already-loaded review/
TDD/debugging/verification guidance. No subagents, dependencies, GPUI/global config,
developer keys, sleeps, watchdog changes, staging/commit/push/PR/native dispatch/
merge/closure/worktree cleanup. The new test threads/processes are actual harness
actors, not subagents. Every Rust command used Devenv and locked targeted tests.

### Important 1 — atomic observation-owner fencing, not just a stale Boolean

`record::complete_record` now calls `recovery::invalidate_index_observations` in
the **same transaction** as retirement, refresh_required=1, completion and applicable
reservation finish. It finds only this repository's currently indexing rows by
registered root, checked-increments their existing index_owner_epoch and records
fixed state **index_invalidated**. IDs, operation identities, targets, authoritative
steps and journal records are preserved. No row is silently deleted or made to
look completed, and another root's live owner in the same registry is untouched.
Overflow/failed guarded updates refuse the transaction instead of guessing.

An old token fails the existing `require_index_owner` check before context writes
or clearing the stale mark. `refresh_repository_claimed` may perform **one bounded
fresh pass** only after a CAS reclaim of that same record/root at the exact
one-epoch invalidation boundary, excluding abandoned/reclaimed/completed rows.
It discards both old observations, starts a heartbeat for the new epoch and scans
outside the repository/cache claim guards. The old heartbeat cannot touch the
new owner. Generic claim_indexing also recognizes the explicit resumable state.
Other recovery/error classes retain the existing handling; there is no resident
retry loop, timeout-based ownership inference or new generation column/table.

The new hidden `set_refresh_persist_hook_for_testing` is after **both observations
and the owner touch**, before persistence, with no repository lease/cache guard/
transaction held. It does not move the existing between-observations hook or
change its failure-point contract. Its mutex failure retains fixed InjectedFailure
with RefreshRepository/root classification.

Real, barrier-driven public-library regressions:

- **quality2_f11_fences_older_refresh**: a contained cleanup is stopped at F5;
  an external refresh observes primary plus materialized context twice and pauses
  before persistence; an actual F11 subprocess completes cleanup. After release,
  the old record is explicitly fenced/reclaimed rather than persisting its obsolete
  context. Epoch witness is **1 → 2 → 3**, identity/authoritative step retained,
  original record completed, primary-only discovery and identical promotion replay
  correct. Another simultaneously paused root **shares A's registry** and remains
  epoch 1/unfenced, then finishes normally.
- **quality2_f11_preserves_active_child_handoff**: the paused owner belongs to
  the real bound primary child's authoritative index handoff. A concurrent same-ID
  promotion executor consumes stored child authority and reaches actual F11. The
  child performs its fresh owned pass, clears remote index_pending, completes its
  original record, and the older parent returns stored completed authority rather
  than its obsolete unfinished-step outcome. `finish_integrated` now checks that
  matching stored completion before reporting a stale executor's recovery category.
- **quality2_f11_does_not_adopt_reclaimed_owner**: another legitimate executor
  of the ORIGINAL refresh identity claims the explicit fence and completes first.
  The paused old token returns IndexPending, does not adopt/overwrite that owner
  or completed record, and preserves the fresh primary-only index/epoch 3.

All three retain one promotion merge/no new receive command and prove a subsequent
ordinary refresh can proceed. The race uses a legitimate AlreadyContained/no-remote-
deletion-reservation path, so the older refresh is truly live while completion can
commit; a merged promotion's remote reservation/local-journal guard would otherwise
stop some interleavings before this window. No lock or owner is removed by a test
or production shortcut. Existing focused discovery concurrency, injected-failure
and lease-release tests also pass unchanged.

### Important 2 — every verified write consumes R1 atomically

`state::consume_retirement_on_publication` is the common primitive. It derives the
repository/kind/item from the immutable context envelope ID, not a separately
supplied scope. It is called in the same transaction as:

1. Normal legacy PushVerified checkpoint writes.
2. `reconcile_synchronization` upgrading its legacy checkpoint to PushVerified.
3. `bind_publication_candidate(... verified=true)` for an already-current candidate.
4. `advance_publication_attempt(... Verified)` itself, covering settlement's
   direct Action::Verify **and** the ordinary advance wrapper.

The wrapper-only duplicate is removed. Classification records authority but is
**not another publication observation** and no longer consumes retirement. Legacy
reconciliation which has freshly re-proved the exact verified candidate proceeds
only to classification instead of re-running a Fetch-only first-publication plan
after consuming R1 on a distinct endpoint.

Three actual two-endpoint tests earn retirement through public recovery/sync/real
promotion under history_unknown=1, then configure a distinct authenticated Push
server. The selected fixture key/server trust is verified publicly. Fetch never
advertises the new context, so its observation cannot consume the permission:

- **quality2_r1_distinct_reconcile_verified**: successful Push STOP after return;
  restart STOP after verification takes the legacy reconciliation upgrade route.
- **quality2_r1_distinct_prepare_already_current**: legacy stopped before Push;
  real peer publication/primary advancement creates an open attempt and an owned
  integration window whose final candidate is already present at Push. Binding
  verified=true records AlreadyCurrent without another push.
- **quality2_r1_distinct_settlement_verified**: real peer/primary changes create
  an appended attempt; its actual push stops after return. Restart's equal-current
  settlement directly verifies that attempt before classification.

Each first forces a DELETE-on-retirement SQL failure: retirement remains and the
verified checkpoint/candidate/attempt write rolls back to push_returned/open/
returned respectively. After clearing the fixed trigger, STOP before classification
requires retirement count 0, no authority yet, unknown flag still 1 and no additional
receive update. This proves co-transactional verification/consumption, not merely
eventual deletion. Same-ID continuation then completes with no repeated push.
The original finding is an **incomplete permission-consumption invariant**, not a
claim that an unauthorized repeat push was demonstrated; existing journal absence
guards already fenced that class of replay.

Verified Push journals now supply qualified **ObservedPublished** read evidence
even before classification, while advertised/tracking OIDs stay absent and Fetch
state remains HistoryUnknown. `fetch_publication_evidence_for` privately preserves
the original Fetch-only evidence for graph/preflight/poll decisions; no Fetch row
or ref is fabricated. Only already-visible identities are looked up; newest proof
is decoded/audited under the current generation/exact plan. Retained validated R1
still suppresses old proof and masking Fetch state still governs eligibility.

The additive partial index **remote_context_publication_evidence**, marker
**cycle_09_context_publication_evidence**, bounds those context-only lookups away
from retained poll history. No table/column or DTO field was added. Its public
acceptance control simulates the preceding schema with business rows preserved,
reopens twice idempotently, then requires RecoveryRequired on marked index loss.
State tests' bounded-read controls pass. One legacy Cycle 04 unit fixture initially
failed because it dropped/rebuilt the index's underlying table but retained the
future index marker. That downgrade fixture now clears only this new marker;
terminal-row/no-inferred-publication assertions and production marked-loss refusal
remain intact. No audit was relaxed to accommodate the fixture.

The audit also exposed a related inverse edge, proven before its final fix:
**quality2_late_classification_keeps_new_retirement** pauses actual context sync
after verified publication, deletes remotely with peer Git, and completes a new
confirmed contained cleanup through actual F11. Old classification then used to
delete that newly earned row. Removing classification-time consumption preserves
the new permission; a distinct fresh edit subsequently publishes and consumes it
at the proper NEW verification. No synthetic retirement insertion or mock child.

### Minor — injective fixture framing and mode/type evidence

`image` now encodes a fixture-only v2 domain, explicit node-kind tags, fixed-width
mode/permission/readonly metadata and length-delimited names/payloads/entry counts.
It never follows symlinks or opens special FIFO/device/socket payloads. Git regular
versus executable mode uses the Unix owner-execute bit; native permission bits are
also included, and Windows retains its available readonly/type evidence. Root
directory metadata is included without binding unrelated temporary root names.
The production consent digest/protocol is **unchanged**.

Meaningful negative controls prove distinct `a`+`bc` versus `ab`+`c` layouts do not
collide, same-byte regular/symlink nodes differ, execute-bit changes differ and
restoring identical files/modes restores the same image. These are counterexamples,
not expected hash vectors mirroring the encoder. Type/exec control is Unix-only;
Linux's final target has **74** cases, Windows is expected to omit that one case.
No native execution has occurred, so platform counts/behavior are not claimed as
observed native results.

### Current mapping additions

All 16 corrected SPEC3-P2 groups remain exercised by the final full target. The
existing outage/bilateral privacy/rejected-input cases remain enabled and passed.
The following supplements close the QUALITY2 implementation gaps:

| Finding / owner obligation | New named proof |
| --- | --- |
| Q1; groups 13/14/refresh | quality2_f11_fences_older_refresh; both observations captured, actual F11 exit, old observations rejected, exact same record reclaims epoch 3 and re-observes; foreign root in same registry unchanged. |
| Q1 compound handoff/no permanent busy | quality2_f11_preserves_active_child_handoff; real bound primary child completes its original refreshed index handoff and clears index_pending, older parent returns stored complete authority. |
| Q1 no owner adoption | quality2_f11_does_not_adopt_reclaimed_owner; original-ID legitimate takeover completes first, expired old token returns IndexPending without changing that record/cache. |
| Q2; groups 3/6/verified metadata | Three quality2_r1_distinct_* routes; rollback proves verified write and retirement removal atomic; STOP before classification proves retired gone/ObservedPublished despite no Fetch context; original-ID completion with unchanged receipts. |
| Q2 additive read support | quality2_publication_index_additive_audited; idempotent prior-schema migration preserves data, marked index loss refuses; state bounded-history tests pass. |
| Q2 retirement lifecycle ordering | quality2_late_classification_keeps_new_retirement; later confirmed cleanup's new row survives an older classifier and permits its next real fresh publication. |
| Minor; groups 4/5/10/14 preservation snapshots | quality2_image_name_payload_boundaries and Unix quality2_image_type_and_git_mode; collision/type/mode controls plus all existing framed-image preservation cases in the 74-case target. |

### Exact QUALITY2 commands / RED and GREEN

All commands ran through Devenv in the ticket worktree. Ordered `&&` chains stopped
on failure; later commands in a failed chain are not counted as executed. Shell
timeouts covered whole custom targets; no per-case watchdog change or sleep added.

| Exact command | Actual QUALITY2 results |
| --- | --- |
| `devenv shell -- cargo test --locked --test document_promotion quality2_image_` | Three runs: RED **0 passed/2 failed** (actual old framing/type collisions); GREEN **2/0**; after precise Git owner-exec mapping refinement **2/0**. |
| `devenv shell -- cargo test --locked --test document_promotion quality2_f11_` | Compile RED: two missing persist-hook method diagnostics; after seam only runnable RED **0/2** (epoch 1→1→1 obsolete writer, active-child old result incomplete); after fencing/handoff corrections **2/0**; strengthened same-registry foreign-root control **2/0**. |
| `devenv shell -- cargo test --locked --test document_promotion quality2_r1_` | RED **0/3**, retired count still 1 at all three real verified routes; after atomic consumption **0/3** on missing qualified snapshot read evidence; GREEN **3/0** after audited Push projection/Fetch separation. |
| `devenv shell -- cargo test --locked --test document_promotion quality2_` | **8/0**, including SQL rollback controls on all routes and additive/marked-loss index control. Later takeover and late-classification cases run below and in final target. |
| `devenv shell -- cargo test --locked --test document_promotion quality2_f11_does_not_adopt_reclaimed_owner -- --exact` | **1/0**. |
| `devenv shell -- cargo test --locked --test document_promotion quality2_late_classification_keeps_new_retirement -- --exact` | RED **0/1**: old classifier deleted new row; GREEN **1/0** after consuming only at true verified transitions. |
| `devenv shell -- cargo test --locked --lib later_edit_after_runner_completion_provisions_fresh_context_and_first_publication` | **1 passed/0 failed/994 filtered**, 1.03s, after final classification-only correction. |
| `devenv shell -- cargo test --locked --lib promotion` | **188 passed/0 failed/807 filtered**, 189.51s. Later production change only removes the now-redundant classification deletion; focused fresh-context test above and final 74-case/four affected runs verify it. |
| `devenv shell -- cargo test --locked --lib remote::state` | Initial **66 passed/1 failed/928 filtered**, 7.96s, on the intentionally downgraded Cycle 04 fixture retaining the new marker; corrected authentic fixture final **67/0/928**, 8.16s. No terminal-row assertions changed. |
| `devenv shell -- cargo test --locked --lib cycle04_poll_migration_preserves_terminal_rows_without_inferred_publication` | **1/0/994**, 0.14s, after fixture version-marker correction. |
| `devenv shell -- cargo test --locked --lib remote::reservation` | **45/0/950**, 13.22s. |
| `devenv shell -- cargo test --locked --test discovery_rebuild concurrent_refreshes_with_the_same_operation_id_have_one_index_owner -- --exact` | **1/0/77**, 0.28s. |
| `devenv shell -- cargo test --locked --test discovery_rebuild refresh_after_observation_failure_preserves_prior_rows_for_retry -- --exact` | **1/0/77**, 0.27s; original injected-failure/prior-cache retry classification retained. |
| `devenv shell -- cargo test --locked --test discovery_rebuild refresh_scan_releases_repository_lease_before_observation -- --exact` | **1/0/77**, 0.25s. |
| `devenv shell -- cargo test --locked --test repository_enablement registry_creates_repository_and_discovery_metadata_schema -- --exact` | **1/0/77**, 0.07s. |
| `devenv shell -- cargo test --locked --test document_promotion` | First **73/0**; the subsequently discovered/proven late-classification permission-loss edge justified another production correction/new regression, then final **74 SSH cases passed/0 failed**, warning-free. All original cases and all three review findings covered; no unchanged-tree repetition. |
| `devenv shell -- cargo test --locked --test remote_synchronization` | **63/0**, then final **63/0** after classification-only correction. |
| `devenv shell -- cargo test --locked --test remote_merge_recovery` | **25/0**, then final **25/0** after classification-only correction. |
| `devenv shell -- cargo test --locked --test remote_polling` | **84/0**, then final **84/0** after classification-only correction. |
| `devenv shell -- cargo test --locked --test comment_publication` | **41/0**, then final **41/0** after classification-only correction. |
| `devenv shell -- cargo fmt --all` | Three QUALITY2 executions, exit 0. |
| `devenv shell -- cargo fmt --check` | Three QUALITY2 executions, all exit 0. |

The affected reruns are justified by the changed shared refresh-owner protocol,
legacy/attempt verification, qualified read projection, and final classification
ordering. No full all-features gate or optional Clippy rerun was performed here;
the previously recorded baseline lints and any final-gate findings remain Task 8.

### Review scope / remaining gates

Review the whole uncommitted Task 7 diff against c0e9726, including new quality
regressions. This QUALITY2 correction adds/changes:

- `src/repository.rs`: post-two-observation test seam and bounded owned fresh pass.
- `src/repository/recovery.rs`: explicit root-scoped index invalidation/CAS reclaim
  using existing fields, with generic claim support for the fixed resumable state.
- `src/repository/promotion/{record,run,observe}.rs`: atomic index-owner fence,
  concurrent stored-completion priority, Fetch-specific eligibility.
- `src/repository/remote/{state,reservation,sync,poll}.rs`: all verified transitions,
  no late classification consumption, qualified indexed Push evidence/read overlay,
  preserved Fetch-only planning and verified-only legacy continuation.
- `src/repository/remote/state_tests.rs`: authentic legacy fixture marker reset only.
- `tests/document_promotion/quality.rs`: ten real/small negative-control cases;
  journeys registration/framed image and exits launch reuse visibility. Shared
  `tests/support` and the original four SSH regression sources remain untouched.
- Cycle/design/plan/handoff/this ledger and canonical comment
  `01M51Z7Z00N4Q6S8V0X2Z4B6EG` (UTC date reflects the long-running session).

Total intended pending inventory is **28 paths**, with no staging or commit.
Final production diff witness `git diff -- src | git hash-object --stdin` is
**d539ccf43b89be9112dc62a3ae9ee56f17c842ab**. Prior SPEC3 fingerprint is historical;
production intentionally changed for these QUALITY2 fixes. New schema is only an
audited additive partial index/marker, not a new data table/column. No consent
digest, public DTO field, CLI/desktop API, new republish API or product policy change.

No known review finding is left unaddressed, but **QUALITY recheck/approval is
pending**, not inferred from tests. Task 8/full/native gates and recorded Clippy
carryover remain pending. Backend lock repair exception remains unchanged, no
automatic lock removal added. Linux counts do not prove Windows/macOS behavior;
the Unix-only image control will not run on Windows. Native workflow is still
manual-only, same four entries, never dispatched. Ticket stays open; no commit,
push, PR, native action, merge, closure, subagent or worktree cleanup.

Final QUALITY2 handoff checks: `git diff --check` and `git diff --cached
--exit-code` passed; `git diff --exit-code -- Cargo.lock devenv.nix devenv.lock
tests/support tests/remote_synchronization.rs tests/remote_merge_recovery.rs
tests/remote_polling.rs tests/comment_publication.rs tests/discovery_rebuild.rs
tests/repository_enablement.rs` passed. Status is exactly the 28 intended paths,
nothing staged, HEAD c0e9726. The final source-diff hash was rechecked and matches
d539ccf43b89be9112dc62a3ae9ee56f17c842ab; only evidence prose changed after the last
test run. No full/native gate or review approval is newly claimed.

## QUALITY Follow-up — Invalidated Authoritative Local Handoffs Remain Resumable

**2026-10-17 UTC: narrow compatibility correction targeted-verified, uncommitted
at c0e972655f04ea67bb9411877b0627e7eb8439ac; return for QUALITY recheck.**

The controller accepts the R1 correction, fixture-image minor and F11 mechanism.
Its remaining important finding is valid: generic refresh could reclaim
`index_invalidated`, but public `operation.resume` and recovery advice used
`HAND_OFF_STATES` without that entry. After executor loss an authoritative local
handoff therefore demanded its original request, or recovery/abandon when there
was no owning request record, and kept unrelated work blocked.

### Scope and preflight

All work stayed in the existing ticket worktree/branch. `git fetch origin main`
succeeded; `git merge-base --is-ancestor origin/main HEAD` returned 0. HEAD remains
c0e972655f04ea67bb9411877b0627e7eb8439ac and freshly fetched main remains
89d04f901636229392a311241f8e777f0a00962b. Existing 28-path Task 7 work was already
dirty and preserved. The current-main base is still an ancestor, so no stash,
checkpoint commit or dirty-tree rebase was needed for this same-task review fix.

Only new production behavior in this follow-up is the extra `index_invalidated`
entry in `HAND_OFF_STATES` plus an explanatory comment. The existing
`LocalRow::owes_only_its_hand_off` authoritative-step/pending-state proof is
unchanged. The remote-first resume binding is unchanged; there is no new F2
promotion binding, API or feature.

### Regressions and evidence boundaries

- The existing state/step classification matrix now includes `index_invalidated`
  across authoritative positives and all existing non-authoritative/no-step
  negatives. Added comment-checkpoint-intent and bare invalidated-refresh
  negatives ensure state alone cannot establish completed work. Existing
  pre-handoff, completed, rejected and abandoned negatives remain.
- `a_resume_finishes_an_invalidated_authoritative_hand_off_after_executor_loss`:
  a real public save commits and owes indexing. The test explicitly seeds the
  durable fence/epoch left by invalidation, then reopens without a running
  executor. Another request offers operation.resume; public resume advances the
  epoch once and completes the same row/step, reports only discovery current,
  preserves the saved commit/content, replays its result, lets the original save
  settle without a second commit, and unblocks the previously refused request.
- `an_invalidated_authoritative_hand_off_needs_no_original_request_record`:
  public domain add_remote leaves a real `remote_changed` handoff without any
  request record. After the seeded fence and reopen, public resume succeeds,
  completes the same step under a fresh epoch and preserves Git config bytes.
- `an_invalidated_state_without_an_authoritative_step_still_needs_the_original_request`:
  an actual save interrupted before its write stays OriginalRequestRequired
  after a seeded invalidation/reopen. Neither request/journal nor content/commit
  changes. The matrix separately covers a missing step.
- `a_resume_does_not_take_up_promotion_through_an_invalidated_local_hand_off`:
  a seeded unfinished promote remote row remains InvalidInput/NotResumable even
  with a same-ID authoritative invalidated local shadow; both rows are unchanged
  and no resume request is recorded. This tests remote-first classification,
  not execution of promotion through the mutation boundary.

These recovery cases follow the existing resume suite's durable-state fixture
pattern: no claim of a new genuine process-kill journey. The three real F11
SSH races were rerun and still prove actual invalidation/epoch fencing, fresh
observation, foreign-root protection and exact-owner takeover. Together the
tests cover real fence creation and the previously missing public recovery path.

### Exact RED/GREEN commands

All Cargo commands ran through Devenv in the ticket worktree, with no warnings.

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib a_row_owes_only_its_hand_off_once_its_work_is_recorded_and_it_has_handed_off` | RED: 1 failed/994 filtered; `authoring_checkpoint_observed in index_invalidated` was not recognized. |
| `devenv shell -- cargo test --locked --test mutation_replay invalidated` | RED: 1 failed/1 passed/304 filtered; authoritative save offered only inspect/abandon, missing operation.resume; non-authoritative control passed. Initial GREEN: 2 passed/304 filtered after adding the state. |
| `devenv shell -- cargo test --locked --test mutation_replay resume::an_invalidated_authoritative_hand_off_needs_no_original_request_record -- --exact` | RED with only the new state entry temporarily removed: Blocked/RecoveryRequired instead of Success/Ok, 1 failed/306 filtered. Entry restored immediately. |
| `devenv shell -- cargo test --locked --lib recovery::abandoned_row_tests` | GREEN: 12 passed/0 failed/983 filtered. |
| `devenv shell -- cargo test --locked --test mutation_replay resume::` | Final GREEN: 33 passed/0 failed/275 filtered, 7.61s. Earlier 31-case intermediate run also passed; see fixture correction below. |
| `devenv shell -- cargo test --locked --test document_promotion quality2_f11` | GREEN: 3 actual SSH cases passed, each observed owner epochs 1 → 2 → 3. |
| `devenv shell -- cargo test --locked --test mutation_contract an_operation_resume_matches_the_envelope_schema_and_its_goldens -- --exact` | GREEN: 1 passed/0 failed/42 filtered, 0.75s. |
| `devenv shell -- cargo fmt --all` | Exit 0 after each test-edit checkpoint. |

One intermediate 33-case run had **32 passed/1 fixture error**: the new promotion
negative tried to change a synchronization's immutable target into `promote`.
The existing SQL trigger correctly refused `immutable remote operation target`.
The fixture now inserts the intended promote row directly with valid context
fields; no trigger/production behavior was changed or bypassed. Final 33-case
run above includes the corrected negative and all existing resume cases.

### Handoff

This correction touches only `src/repository/recovery.rs`,
`tests/mutation_replay/resume.rs`, existing Cycle/design/plan/handoff/ledger and
canonical comment `01M5207Z00N4Q6S8V0X2Z4B6EH`. Total intended pending inventory
is now **30 paths**, all unstaged/uncommitted. Production diff witness
`git diff -- src | git hash-object --stdin` is
**5f4704976c6dcae4fad323727042b3329acafc5c**.

Earlier full 74-case/four-affected-SSH-target runs remain historical; only the
focused F11 selection was rerun here. No full gate or native execution was
requested or performed. Existing Task 8 gates/lint carryover remain pending.
Return the whole uncommitted Task 7 diff for controller QUALITY recheck; approval,
Task 7 completion and ticket closure are not inferred. No staging, commit,
subagent, dependency, GPUI/global configuration, push/PR/native dispatch, merge
or worktree cleanup action.

Final follow-up checks: `devenv shell -- cargo fmt --check`, `git diff --check`
and `git diff --cached --exit-code` all passed. `git diff --exit-code --
Cargo.lock devenv.nix devenv.lock tests/support tests/remote_synchronization.rs
tests/remote_merge_recovery.rs tests/remote_polling.rs tests/comment_publication.rs
tests/discovery_rebuild.rs tests/repository_enablement.rs
src/repository/mutation/bind/resume.rs` passed. Full untracked-file status confirms
30 intended unstaged paths; the final source-diff witness still matches the value
above. Only documentation/comment edits followed the final test run.

## Task 7 Final Reviews Accepted — Controller Checkpoint, Then Task 8

**2026-10-17 UTC: reviewed complete at the local gate.** The controller reports
**Task 7 SPEC PASS and QUALITY FINAL APPROVE, no remaining findings**, including
the final `index_invalidated` public-resume correction. The review boundary is
the entire uncommitted Task 7 change against exact base
**c0e972655f04ea67bb9411877b0627e7eb8439ac**, not only the final classifier edit.
These are controller-reported final approvals; this coordination pass does not
run an additional review or infer approval from tests. Earlier pending-review
handoffs are historical and superseded. No current Task 7 owner question or
review finding remains open.

Task 7 is delivered and reviewed complete at the local gate. **Next action is
the controller's checkpoint commit, then Task 8.** The worker makes no commit,
starts no Task 8 implementation and does not close the ticket. Full Rust/CLI,
whole-branch and native gates remain pending. The **22 carried Clippy findings
must be fixed in Task 8**, not waived as baseline warnings. Task 7 approval is
not full/native readiness or Cycle completion.

### Retained verification qualifications

The delivered Task 7 evidence is **74 real-SSH promotion cases**, plus affected
targets **63 synchronization / 25 merge recovery / 84 polling / 41 comment
publication**, and the recorded **188** promotion-library tests. The complete
target runs precede the last narrow resume correction. That final correction
has focused GREEN evidence: **33 public resume / 12 recovery / 3 real SSH F11 /
1 resume envelope-schema** tests. The full targets were not rerun after that
correction. Recovery tests explicitly model the durable invalidation fence and
reopen; real F11 race tests separately establish actual fence creation. The
Unix-only image test remains a platform qualification, not Windows/macOS proof.
Historical RED/GREEN, fixture corrections and all approved policy boundaries
remain recorded above.

### Coordination-only pass and exact staging boundary

Only this ledger, handoff, implementation-plan Task 7 summary/next-step checklist,
Cycle evidence and canonical comment `01M5217Z00N4Q6S8V0X2Z4B6EJ` were changed in
this pass. No source, tests, dependency files or workflow wiring changed; no
Cargo/formatter/test command was run. HEAD and exact review base remain
c0e972655f04ea67bb9411877b0627e7eb8439ac. The source diff witness stays
**5f4704976c6dcae4fad323727042b3329acafc5c**. The tracked non-document diff witness
(`src tests Cargo.toml Cargo.lock .github devenv.nix devenv.lock`) stays
**9c4d45eee4710000122cc1096b220015834acb70**. Hashes of the five untracked SSH
test files were also compared before/after to cover files absent from `git diff`.

**All 31 pending paths below are intended for the controller's Task 7 checkpoint;
nothing is staged.** Paths are relative to the ticket worktree:

```text
.github/workflows/build.yml
Cargo.toml
docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md
docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md
docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-execution.md
docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md
docs/plans/2026-10-15-wave-02-cycle-09-handoff.md
src/repository.rs
src/repository/promotion/observe.rs
src/repository/promotion/record.rs
src/repository/promotion/run.rs
src/repository/recovery.rs
src/repository/remote/observation.rs
src/repository/remote/poll.rs
src/repository/remote/reservation.rs
src/repository/remote/state.rs
src/repository/remote/state_tests.rs
src/repository/remote/sync.rs
tests/mutation_replay/resume.rs
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M51V7Z00N4Q6S8V0X2Z4B6EC.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M51W7Z00N4Q6S8V0X2Z4B6ED.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M51X7Z00N4Q6S8V0X2Z4B6EE.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M51Y7Z00N4Q6S8V0X2Z4B6EF.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M51Z7Z00N4Q6S8V0X2Z4B6EG.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M5207Z00N4Q6S8V0X2Z4B6EH.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M5217Z00N4Q6S8V0X2Z4B6EJ.md
tests/document_promotion.rs
tests/document_promotion/advanced.rs
tests/document_promotion/exits.rs
tests/document_promotion/journeys.rs
tests/document_promotion/quality.rs
```

The design, source/test files, wiring and six earlier comments are existing
reviewed pending work, not edits made by this approval-recording pass. New
canonical approval comment brings the prior 30-path inventory to 31. No staging,
commit, push, PR, native dispatch, merge, closure, subagent or worktree cleanup.

Coordination verification: **`git diff --check` passed**;
**`git diff --cached --exit-code` passed (nothing staged)**. Full untracked-file
status confirms exactly the 31 paths above. HEAD matches the exact review base.
Both tracked-diff witnesses and all five untracked SSH-file hashes match their
pre-edit values, confirming source/tests/wiring unchanged in this pass. These
are Git/document checks only, not Rust/test/full/native gate reruns.

## Historical Task 8 — Pre-Quality-Correction Full Verification (2026-10-17 UTC)

**Superseded as final-tree evidence by the post-review final gate below.** The
3,049-pass run and pending-review status in this section describe the earlier
tree; the subsequent five-regression correction changed source. Retain the
commands/failure history, full acceptance mapping and desktop evidence as history.

**Local verification complete; READY for the controller's independent final spec
and whole-branch quality reviews. Those reviews and final approval are pending.**
Tasks 0–7 were reviewed and committed before this worker started. Task 7 checkpoint
and unchanged worker HEAD: **504cb44aa7f8e1eebd5996666851ebe34c47f78d**. All Task 8
changes are unstaged/uncommitted. Ticket remains **open**. The owner explicitly
authorized the saved full run now, and assigned checkpointing only to the controller
after independent reviews. No subagents, commit, push, PR, native dispatch, merge,
closure or worktree cleanup occurred.

### Boundary, Integration And Lockfile Evidence

- Read this worktree's `AGENTS.md`, approved Cycle/design/plan/history and relevant
  verification/debugging skills. Initial `git status --short --branch` was clean
  on exact branch `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. All edits and
  commands were in `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH`.
- Reused the controller's completed boundary rebase. As explicitly requested,
  **`git fetch origin main`** ran without another rebase/merge/checkout; current
  **origin/main = 89d04f901636229392a311241f8e777f0a00962b**, unchanged from the
  controller's last fetch. No new main compatibility delta exists.
- **`git merge-tree --write-tree HEAD origin/main`**, twice, exit 0, tree
  **9fa0a194c03bd43640b82b9bf0bfa59f67a18d7e**. This proves committed-branch
  integration at the actual SHAs above; it does not pretend to include uncommitted
  edits. The Task 8 diff is based on that HEAD, and final review must include it.
  No index or checkout mutation. Main checkout and its `.superpowers/` untouched.
- **`sha256sum Cargo.lock devenv.lock`** at start and after all Rust runs/smokes:

  | File | Start SHA-256 = end SHA-256 |
  | --- | --- |
  | `Cargo.lock` | `8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8` |
  | `devenv.lock` | `d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e` |

  No dependencies, toolchain/global configuration, developer keys or GPUI
  dependencies changed. No sleeps or lint suppressions added.

### Carried Lints And Full-Run Failures Resolved

The earlier **22** diagnostics were an incomplete targeted probe, not a waiver.
Fresh `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
failed (exit 101) with **23 library / 28 total diagnostics**: 17 large-error,
five needless-borrow, four collapsible-if, one unnecessary-unwrap and one test
type-complexity diagnostic. Root causes and narrow refactors:

- Box only `DocumentPromotionError::Transport`'s large payload; preserve its
  typed transport error and redacted Display/Debug. Update the existing privacy
  test's construction. This is the new domain API's payload representation;
  existing synchronization/transport error APIs are unchanged.
- In three private inspections whose callers already discarded all error detail,
  use `Result<_, ()>` instead of constructing/carrying large synchronization
  errors. Preserve success/invalid-document distinction and fail-closed results.
- Remove five redundant borrows, collapse four short-circuit nested conditions,
  bind the optional cleanup plan with `if let`, and name the existing test image
  tuple. No new lint-only tests, blanket `allow`, or behavior change.

Four executions of the **exact full test command** were needed; each repeat had
a concrete unsuccessful prior run or changed test tree, never a passing unchanged
tree. All used Devenv and the unchanged 100-second per-SSH-case watchdog:

1. **Outer shell timeout at 1,800,000ms**, not a passing gate. Compilation took
   5m45s; library **995/0** (540.26s), canonical **62/0**, discovery **78/0**,
   comment publication **41 SSH passes** completed. Promotion was progressing
   through `exit_f2_context_synchronized` with no assertion failure when the outer
   tool killed the run. Process inspection confirmed no remaining test process.
   The complete suite is much longer than the new promotion target. Subsequent
   outer allowance was 10,800,000ms; no per-case/harness timeout was increased.
2. **Exit 101:** reached recovery foundation, **50 passed / 1 failed** in
   `failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly`.
   A focused RED with a table-name assertion confirmed the sole mismatch was
   `registry_migrations`. Cycle 9 now commits this shared table/its three markers
   in the earlier registry-schema transaction (`discovery.rs`), before the
   independent recovery transaction fails on the deliberately NULL legacy time.
   The test now retains all three recovery-table absence checks, exact legacy
   columns/two operation/two context rows, redaction, repair/retry/idempotence
   checks, and asserts **exactly the three committed Cycle 9 markers**, thereby
   proving neither failed recovery marker leaked. No production migration change.
   Focused GREEN: **1 passed / 0 failed / 50 filtered**, 0.12s.
3. **Exit 101:** recovery now **51/0**; all later targets executed through SSH
   transport, which reported **120 passed / 2 failed**. Each failed deterministically
   at a snapshot-evidence equality assertion, not its watchdog:
   `synchronization_distinct_push_context_deleted` (old line 1072) and
   `synchronization_context_absence_boundaries` (old line 675). Focused RED
   reproduced both. Their old `NeverPublished` assertions predated Task 7's
   reviewed direction-qualified Push-history projection (`attach_verified_publications`
   and separate Fetch-only planning). Updated only these three assertions via
   a shared assertion helper: public/item evidence **ObservedPublished**, while
   advertised/tracking OIDs remain **None** and Fetch state **HistoryUnknown**.
   Preserved actual server absence, no-overwrite/unchanged local-image, reopen,
   `RemoteContextDeleted` and unknown-history refusal controls. Each focused
   GREEN passed **1 SSH case**. No new policy or production correction.
4. **Final full gate: exit 0**, **3,049 passes / 0 failures**, detailed below.
   One existing opt-in read performance characterization is ignored by its
   pre-existing annotation; no failure was ignored and no test disabled.

Captured full outputs (local harness evidence, not committed artifacts) are
`tool_147c27eac001qdhUATrCRqCLRj` (outer timeout),
`tool_147da1e91001odwV3ifYWng43o` (migration RED),
`tool_148069e06001Y8X3uCAkUz0u03` (SSH assertion RED), and
**`tool_1484c204d001O5WsMcEt1cKaB1`** (final GREEN), under
`/home/vhodges/.local/share/opencode/tool-output/`. Counts below are read from
this worker's final full output, including the late Task 7 corrections, not
inferred from earlier worker reports.

### Exact Commands And Final Results

| Exact command | Executions / actual result |
| --- | --- |
| `devenv shell -- cargo fmt` | Twice during refactoring; exit 0. |
| `devenv shell -- cargo check --all-features --locked` | Twice, both exit 0; final 0.35s after all Rust edits. |
| `devenv shell -- cargo fmt --check` | Three times, all exit 0; final after all Rust edits. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Initial RED above, then three GREEN runs (38.10s, 1.12s, final 3.31s); final exit 0, no warnings. Later repeats follow test corrections. |
| `devenv shell -- cargo test --all-features --locked` | Four runs as above; final exit 0, no failed target, through all 35 integration targets and doctests. Final library 434.00s. |
| `devenv shell -- cargo test --all-features --locked --test recovery_foundation_gate failed_recovery_migration_leaves_no_partial_schema_and_retries_cleanly -- --exact` | Focused RED **0/1/50 filtered**, then GREEN **1/0/50 filtered**. |
| `devenv shell -- cargo test --all-features --locked --test ssh_transport synchronization_distinct_push_context_deleted -- --exact` | Focused RED **0 passed / 1 failed**, then GREEN **1 SSH pass**. |
| `devenv shell -- cargo test --all-features --locked --test ssh_transport synchronization_context_absence_boundaries -- --exact` | Focused RED **0 passed / 1 failed**, then GREEN **1 SSH pass**. |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | Exit 0, no application output and no window. Actual CLI remains a transport-initialization skeleton, with no no-argument help/error grammar and no comment command. |
| `git diff --check` | Exit 0 after source/test changes and after final documentation/comment edits. One intermediate documentation-only trailing space was detected and removed before final success. |

Final standard harness counts (all **zero failed**, zero filtered):

| Target | Passed | Ignored |
| --- | ---: | ---: |
| library | 995 | 0 |
| desktop binary / CLI binary | 0 / 0 | 0 |
| canonical_foundation | 62 | 0 |
| discovery_rebuild | 78 | 0 |
| editor_feasibility | 9 | 0 |
| journal_rejection | 26 | 0 |
| key_material | 39 | 0 |
| key_storage | 2 | 0 |
| local_authoring | 140 | 0 |
| mutation_bridges | 129 | 0 |
| mutation_confirmation | 154 | 0 |
| mutation_contract | 43 | 0 |
| mutation_relationships | 27 | 0 |
| mutation_replay | 308 | 0 |
| read_boundary | 20 | 0 |
| read_characterization | 0 | 1 (pre-existing opt-in performance characterization) |
| read_comments | 22 | 0 |
| read_contract | 71 | 0 |
| read_credentials | 25 | 0 |
| read_items | 76 | 0 |
| read_relationships | 17 | 0 |
| read_repository | 33 | 0 |
| read_status | 23 | 0 |
| recovery_foundation_gate | 51 | 0 |
| remote_reservation | 13 | 0 |
| repository_enablement | 78 | 0 |
| session_credentials | 11 | 0 |
| shared_key_registry | 39 | 0 |
| doctests (compile-fail privacy/API contracts) | 11 | 0 |

Final custom SSH harness counts, all **zero failed** (not libtest counts):

| Target | SSH cases passed |
| --- | ---: |
| comment_publication | 41 |
| document_promotion | 74 |
| mutation_remote | 88 |
| remote_merge_recovery | 25 |
| remote_observation | 15 |
| remote_polling | 84 |
| remote_synchronization | 63 |
| ssh_fixture | 35 |
| ssh_transport | 122 |
| **SSH total** | **547** |

Totals: **995 library + 1,496 standard integration + 547 SSH + 11 doctests =
3,049 passes**, zero failures, one existing ignored characterization.

### Desktop Smoke And Native Limitations

Environment inspection found `DISPLAY=:1`, `WAYLAND_DISPLAY=wayland-1`,
`XDG_RUNTIME_DIR=/run/user/1000`; the Wayland socket was present and
`timeout 10s xprop -root _NET_SUPPORTING_WM_CHECK` returned a window ID (exit 0).
An attempted Python inspection was unavailable (`python3: command not found`);
shell/socket/X inspection supplied the evidence instead, without installing tools.

- `timeout --signal=TERM 180s devenv shell -- cargo run --locked --features desktop --bin manyhands`
  compiled in 1m22s, reached `Running target/debug/manyhands`, and emitted no
  application startup error. The smoke was bounded, not left running.
- To substantiate actual window/surface creation rather than only a running
  process: `timeout --signal=TERM 10s env WAYLAND_DEBUG=client devenv shell -- cargo run --locked --features desktop --bin manyhands`;
  capture via `desktop_smoke_status=$?; declare -p desktop_smoke_status` reported
  **124**, expected deliberate timeout termination. Protocol evidence shows
  `xdg_surface.get_toplevel`, configure/ack, non-null `wl_surface.attach(wl_buffer)`
  and `wl_surface.commit`, with no startup panic/error. Captured in
  `tool_14891a89200127mCFuppfBB2WU`. **Desktop launch/surface smoke passed**;
  no human visual/button-interaction assertion is made. Only owned smoke processes
  were terminated; process inventory checked afterward.
- **Native CI remains pending authorization**, never dispatched. Task 7 wired
  `--test document_promotion` into the existing manual-only Build workflow.
  The actual current matrix has **four** targets: Linux x86_64/aarch64,
  Windows x86_64, macOS aarch64. The AGENTS five-target wording is historical;
  Windows aarch64 was owner-dropped in Cycle 08. Local Linux evidence does not
  establish native Windows open-handle/prune or other native path/permission
  behavior. The Unix-only image case is not claimed on Windows.

### Every Cycle Acceptance Row → Named Passing Tests

All names below passed in **this worker's final full gate**. `O::` abbreviates
`repository::promotion::observe::tests::`, `I::` =
`repository::promotion::integrate::tests::`, `R::` =
`repository::promotion::run_tests::`; unprefixed names are cases in
`tests/document_promotion.rs` unless a target is explicitly given. Row numbers
follow the Cycle acceptance table in order; none relies only on a task label.

| # / Acceptance | Named fresh passing evidence |
| --- | --- |
| 1. Offline read-only preflight, all blockers, exact/stable bound digest | `O::preview_is_read_only_offline_and_does_not_take_repository_lease`, `O::multiple_blockers_have_fixed_order`, `O::change_set_includes_all_paths_and_moves_without_rename_detection`, `O::digest_is_stable_and_debug_is_redacted`, `O::digest_binds_each_field_change_blob_and_cleanup_member`, `O::digest_binds_every_change_entry`, `O::t2_1_added_executable_bit_changes_digest`, `O::t2_1_added_regular_to_symlink_changes_digest`, `O::t2_1_modified_executable_bit_changes_digest`, `O::t2_1_modified_regular_to_symlink_changes_digest`, `O::t2_1_deletion_encoding_has_zero_big_endian_u32_mode`; `blocker_dirty_primary`, `blocker_dirty_context`, `blocker_context_behind_fetched` count transport and snapshots. |
| 2. Digest/blocker/new-context refusal, D6 invariance | `R::changed_digest_is_durably_abandoned_and_new_id_succeeds`, `R::new_blocker_is_stored_and_replayed_before_any_child`, `new_context_content_reconfirms`, `primary_only_advance_preserves_consent`, `O::remote_digest_excludes_moving_oids`. New context content can finish its deliberate child synchronization; refusal is before **promotion integration**, not a claim of zero prior child effects. |
| 3. Poll advance cannot drop primary commits | `poll_f2_primary_advanced_retry_integrates_once`, `I::poll_advance_requires_resynchronization_without_writing_objects_or_any_git_bytes`, `R::advance_after_f2_resets_context_attempt_and_integrates_once`. |
| 4. Complete remote lifecycle/history/ordered merge | `happy_comment_history_and_collaborator_poll`, `first_publication_deletes_verified_context_oid`, `later_publication_deletes_verified_not_stale_tracking_oid`, `I::remote_merge_has_exact_subject_parent_order_and_context_tree_from_behind_primary`. |
| 5. Local-only then later primary publication | `local_only_then_primary_publication`, `R::local_full_lifecycle_and_exact_replay`, `R::runner_local_divergence_preserves_prepared_primary_tree_and_complete_discovery`. |
| 6. Contained/no new merge/cache-loss cleanup | `contained_and_fresh_second_promotion`, `f7_cache_loss_new_confirmed_cleanup_earns_retirement`, `R::cache_loss_contained_context_cleans_without_another_merge_or_child`, `O::contained_absent_tracking_context_is_cleanup_only`, `I::contained_records_base_without_identity_commit_or_primary_update`. |
| 7. Fresh edit/R1 retained-only/registry loss/new earned retirement | `contained_and_fresh_second_promotion`, `retained_r1_unknown_history_and_consumption`, `cache_loss_fresh_edit_remains_unknown_across_reopen`, `f7_cache_loss_new_confirmed_cleanup_earns_retirement`, `r1_verified_push_consumes_before_authority`, `r1_new_ambiguous_push_is_not_old_retired_history`, `quality2_r1_distinct_reconcile_verified`, `quality2_r1_distinct_prepare_already_current`, `quality2_r1_distinct_settlement_verified`, `quality2_late_classification_keeps_new_retirement`. R1 requires retained validated retirement with no masking state; cache loss/absence/old merge does not reconstruct permission. |
| 8. Dirty/conflicted/ahead/wrong-primary blockers | `blocker_dirty_primary`, `blocker_conflicted_primary`, `blocker_primary_ahead`, `blocker_primary_not_checked_out`; `O::primary_diverged_from_remote`. |
| 9. D3/D4/D5 refusal | `O::local_integration_conflict`, `R::execution_only_local_conflict_abandons_without_primary_effect`, `new_context_content_reconfirms`, `blocker_remote_deleted`, `O::remote_history_unknown`, `O::present_tracking_context_overrides_unknown_history`. D4 qualification is the same as row 2. |
| 10. Context conflict, actual resolution then once-only promotion | `real_context_conflict_public_resolution`, `R::genuine_context_conflict_resolves_canonical_and_external_and_reuses_owned_resolution`; existing `remote_merge_recovery::code_conflict_false_repairs_then_external_repair`. |
| 11. Failed primary publication preserves cleanup; retry unfinished | `primary_rejection_retry_only_unfinished`, `offline_after_integration_is_publication_pending`, `offline_before_context_is_preintegration_error`, `R::publication_rejection_is_pending_and_retry_only_publishes_and_cleans`. Same endpoint/key restored for identical requests. |
| 12. Cleanup edits/untracked/ignored/locked/moved, delete failures/races, supersession | `preserve_edited_context_then_supersede`, `preserve_untracked_then_supersede`, `preserve_ignored_then_supersede`, `preserve_locked_then_supersede`, `delete_rejection_retry_preserves_local_state`, `delete_context_race_preserves_peer_and_tracking`, `R::cleanup_precheck_preserves_dirty_untracked_moved_and_ignored_before_remote_delete`. A rejected/changed remote delete preserves **all local state and the observed server state**, not the already-published primary's old value. |
| 13. Ownership-first interrupted/cancelled children never strand | `cancelled_context_attempt_replaced_once`, `cancelled_primary_attempt_replaced_once`, `interrupted_retained_context_keeps_child_id`, `interrupted_retained_primary_keeps_child_id`, `R::ownership_first_attempt_matrix_both_children_has_no_permanent_strand`, `R::actual_primary_identity_required_keeps_original_child_for_confirmed_continuation`, `R::genuine_primary_owned_resolution_stays_pending_on_recovery_then_publishes_same_child`. Retained/uncertain ownership never implies a new attempt. |
| 14. F1–F12, inside-F10/F11, SQL rollback/uncertain commit/no duplicate effects | `exit_f1_confirmed`, `exit_f2_context_synchronized`, `exit_f3_before_integration_ref`, `exit_f4_after_integration_ref`, `exit_f5_after_publication`, `exit_f6_before_remote_deletion`, `exit_f7_after_remote_deletion`, `exit_f8_worktree_removing`, `exit_f9_before_branch_deletion`, `exit_f10_before_completion`, `exit_f11_after_completion`, `exit_f12_before_refresh`, `exit_inside_f10_git_sql_split_operator_repair`, `exit_after_sql_commit_replay_preserves_lock`, `completion_statement_rollback_preserves_history`, `completion_commit_rollback_preserves_history`, `completion_unknown_commit_result_reconciles`, `R::lost_completion_commit_result_reopens_actual_complete_authority`. **T6-1 qualification retained:** ambiguous backend lock first preserves history/reservation/retirement/effect counters and returns pending recovery plus refresh; only quiesced verified operator repair permits the identical retry. No unqualified automatic-resume claim. |
| 15. Automatic poll yields; no rematerialization during/after cleanup | `automatic_poll_advertisement_yields_to_promotion`, `poll_between_publication_and_f6`, `poll_busy_at_f6`, `poll_busy_at_f7`, `poll_busy_at_f8`, `poll_busy_at_f9`, `poll_busy_at_f10`, `happy_comment_history_and_collaborator_poll` (promoter and collaborator dispositions). |
| 16. Every post-integration Ok refreshes or IndexPending | `R::partial_refresh_failure_preserves_outcome_and_later_refresh_shows_integrated_primary`, `R::refresh_failure_keeps_identical_authoritative_outcome_and_replay_repairs_index`, `quality2_f11_fences_older_refresh`, `quality2_f11_preserves_active_child_handoff`, `quality2_f11_does_not_adopt_reclaimed_owner`, `exit_f12_before_refresh`; `mutation_replay::resume::a_resume_finishes_an_invalidated_authoritative_hand_off_after_executor_loss`, `resume::an_invalidated_authoritative_hand_off_needs_no_original_request_record`, `resume::an_invalidated_state_without_an_authoritative_step_still_needs_the_original_request`, `resume::a_resume_does_not_take_up_promotion_through_an_invalidated_local_hand_off` (all four in mutation_replay target). The resume fixtures model durable fence/reopen; real F11 cases prove fence creation. |
| 17. Content-free workflow/diagnostics; accepted discovery-title exception only | `privacy_metadata_checkpointed`, `privacy_metadata_pinned_wal`, `privacy_rejected_nonindexed_title_input`, `capture_privacy_controls_fail_closed`, `happy_comment_history_and_collaborator_poll`, `R::fixed_error_and_request_formatting_never_includes_content_paths_or_endpoints`, `repository::promotion::record::tests::privacy_column_inventory_for_both_new_tables`. Includes actual promoted public comment, bilateral live DB/sidecar/backup positive and actual-row rollback negative controls. Only accepted `discovered_items.title` metadata/copies allow titles; rejected titles, bodies/comments/endpoints/credentials/key material remain excluded from raw stores/output. |
| 18. Required Devenv gates; honest native evidence | Exact final command/results/counts/hashes above: check/fmt/clippy/test/CLI and bounded active-display desktop smoke. Four-target manual workflow includes promotion target. **Native execution pending authorization**, not a passing native exit gate. |

### Final Review Boundary / Remaining Gates

The controller must independently review **the entire `origin/main..HEAD` branch
plus all uncommitted Task 8 changes**, not `HEAD~1`. Use actual base
`89d04f901636229392a311241f8e777f0a00962b` and tip
`504cb44aa7f8e1eebd5996666851ebe34c47f78d`, then include the working diff.
Task-specific spec/quality approvals for Tasks 0–7 stand; this worker does not
grant whole-branch/final approval. Review the boxed new transport payload and the
three stale-assertion corrections as part of the full change. All approved
T0/T2/T3/T4/T6/T7 rulings remain intact.

Task 8 file boundary: eight lint-refactor files (`src/repository.rs`,
`src/repository/bridges/identity.rs`, `src/repository/promotion/{integrate,mod,observe,run,run_tests}.rs`,
`src/repository/remote/reservation.rs`), two existing regression files
(`tests/recovery_foundation_gate.rs`, `tests/ssh_transport/synchronization.rs`),
Cycle/ledger/plan/handoff and canonical review-ready comment. No new test case
was added. Final review, controller checkpoint, separately authorized native CI,
published-ref reconciliation and any later outward/closure actions remain gated.

Final return checks: `git diff --check` and `git diff --cached --exit-code` pass;
nothing staged. `git diff --exit-code -- Cargo.toml Cargo.lock devenv.nix
devenv.lock .github/workflows/build.yml
.manyhands/tickets/01K7F6H9J2N4Q6S8V0X2Z4B6DH/ticket.md` exits 0 (including the
unchanged open ticket). End lockfile hashes/HEAD/main match those recorded above.
`ps -C manyhands -o pid,ppid,etimes,comm` lists no remaining desktop smoke process.
Final status is exactly **14 modified tracked paths + the one new canonical
comment**, all within this ticket worktree. Only documentation changed after
the final passing full Rust gate; no redundant Rust rerun for bookkeeping.

## Final Whole-Branch Quality2 — Publication Evidence And Retry Cancellation (2026-10-17 UTC)

**Both supplied IMPORTANT findings verified, reproduced before production edits,
and corrected. Return to the same quality reviewer before the controller's final
gate rerun. No whole-branch approval or new full-gate result is inferred.** Exact
entry/review base and unchanged HEAD:
`504cb44aa7f8e1eebd5996666851ebe34c47f78d` (Task 7 committed).

### Entry And Task 8 WIP Preservation

The worktree was intentionally dirty on entry: fourteen tracked Task 8 source/
legacy-test/documentation paths plus canonical comment `01M5227Z00N4Q6S8V0X2Z4B6EK`.
Inspected the actual working diff, including run.rs's lint folds/concurrent-complete
handoff and run_tests.rs's boxed transport payload/type alias/lint refactor. Layered
this fix onto those files; no reset, checkout, stash, rebase or replacement from HEAD.
The existing Task 8 lint work, two legacy regression files and documentation are
preserved. The previous 3,049-test full gate above remains **prior evidence before
this correction**, not a claimed final gate for the changed source.

All unrelated WIP was byte-hashed before and after the source/test edits and
formatting; hashes match exactly:

| Preserved WIP path | Git blob witness |
| --- | --- |
| `docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md` | `cc889dcdb34b1793e3fd65c0be3646ad9955fa01` |
| `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md` | `ff434b98e1bcf12e2f5db753c7579deb42eaa18f` |
| `docs/plans/2026-10-15-wave-02-cycle-09-handoff.md` | `d6acc5c9d024cd89267c59514230f36604facae9` |
| `src/repository.rs` | `0bf8c442de0f9c5787e40f2e2b1c284ee94fdfa2` |
| `src/repository/bridges/identity.rs` | `b082b22daf6e6faee8a5b50846eed34c4018ab6f` |
| `src/repository/promotion/integrate.rs` | `2aa242e7689c9b2cd59da377bd598bb2f061a794` |
| `src/repository/promotion/mod.rs` | `fcaeab05c7db2cb571c88a6d10b79a4052d4e825` |
| `src/repository/promotion/observe.rs` | `450a61009fb7e9916b541c7e7be19946ac169032` |
| `src/repository/remote/reservation.rs` | `dc9637dc4267a3e9932f7fb7222927023d6d11ae` |
| `tests/recovery_foundation_gate.rs` | `6bba8f665a3144b0ed13c01a68573153632f5614` |
| `tests/ssh_transport/synchronization.rs` | `3a81aeb5e3cc6732fb66632bd0245228e28d82b6` |
| `.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M5227Z00N4Q6S8V0X2Z4B6EK.md` | `118f7d14a3b94f6f6ac59955ba2e02f739521c82` |

This ledger is append-only for this pass. Existing run.rs/run_tests.rs WIP remains
in the combined working diff; only the focused authority/cancellation changes and
new tests are layered onto it. No dependency/schema/configuration or other source
path was edited.

### Finding 1 — Never Substitute Integration M For Publication P

Verified make_outcome discarded primary authority read errors via
`.ok().flatten().unwrap_or(integration_oid)`. The read used an immediate SQLite
writer transaction, so even a writer lock could turn actual later publication P
into a falsely exact Published/AlreadyCurrent claim for integration M.

Correction:

- Carry the exact primary child authority already known by this execution frame
  through finish/refresh/outcome construction. The process-only witness includes
  the child ID, so it is not reused for a peer's different bound attempt.
- Existing durable primary authority is read in a coherent **read-only/deferred**
  registry transaction, with registered-root/repository-ID checks and existing
  typed child identity/authority validation, not an immediate writer reservation.
- make_outcome derives publication **kind and OID** from actual authority, never
  from integration. If a frame lacks known authority and lookup fails or cannot
  corroborate a recorded publication, return Ok with explicit synchronization
  RecoveryRequired publication pending. No error or invented OID. Durable complete
  cleanup is not undone, and no parent/child authority row is rewritten to guess.
- Complete-ID replay remains effect-free and accurate: an unavailable read can
  explicitly report recovery pending; a later identical replay reads the original
  P and restores the exact stored outcome without another child/publication/delete.

The existing completion/cancel/lost-return fixture creates a real later primary
tip P distinct from integration M and stores actual child authority. Two new
complete-replay tests fail the **actual outcome authority lookup boundary** through
a one-shot test-only read-unavailable seam (which must execute), for both completion
kinds. Before the fix both report M. Afterward they report only exact known P or
explicit recovery pending, keep the complete record, and replay the exact P outcome
when the read recovers, with identical child ID/effect counts. A live-frame control
for both kinds proves exact known P is carried without a redundant outcome lookup.
The seam models the unavailable query itself, not an invented completed result.

### Finding 2 — Consume Existing Completion Before The Outer Cancel Gate

Verified finish_steps's outer cancellation check ran before child(), so a retry
could persist publication pending Cancelled without inspecting a primary which
had already completed durably before executor loss. The earlier inside-child
lost-return/cancel tests did not cover this retry-entry interval.

Before configuration/cancellation classifies publication as unfinished, an
Integrated/PublicationPending parent now probes **existing** primary authority.
If present, it records Published/AlreadyCurrent and the frame's exact witness,
persists the Published parent step, then cancellation stops cleanup with Cancelled.
This probe does not call the child IO, mint an attempt, reserve/restart or start
transport. Known absence follows existing cancellation/no-execution semantics.
Unreadable ownership/authority remains publication synchronization RecoveryRequired
with cleanup AwaitingPublication, not an assumed completion or cancellation.

Two deterministic retry-window regressions cover both kinds, each from **both
Integrated and PublicationPending**. A disposable fixture executor unwinds after
the actual primary row is durably complete but before the parent can acknowledge
it. Raw assertions prove this intermediate parent/child state and P != M. The
real cancellation API sets an initial stop; a fixture SQL trigger posts a second
cancellation immediately after the identical retry's initial clear, before
finish_steps. This matches a concurrent cancellation in that interval (all child
rows are completed/inactive). The tests assert exact kind/P, Published parent step,
cleanup Cancelled, preserved local context/child ID and no repeated child call;
after the stop is removed, identical retry completes cleanup without republication.
They also assert durable parent progress so render-time correction alone cannot
hide a still-wrong stored publication category.

### Exact Targeted RED / GREEN

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality3_` | Initial behavioral RED **0 passed/5 failed/995 filtered**, 2.01s, warning-free: replay/live lookup faults fabricate M instead of P, and both retry windows report publication pending Cancelled. No production fix preceded this run. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality3_` | After evidence correction alone, **3 passed/2 failed/995 filtered**, 2.86s: exact-OID tests pass, retry-window tests still fail the durable Published-step assertion. Confirms the second finding is not merely hidden by outcome rendering. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality3_` | After both findings corrected, **5 passed/0 failed/995 filtered**, 3.97s. Each completion kind and both pending-parent starting steps execute. |
| `devenv shell -- cargo test --locked --lib promotion` | First broadened run **192 passed/1 failed/807 filtered**, 198.35s. Existing uncertain-ownership regression exposed an early-probe error mapped to cleanup RecoveryRequired rather than approved AwaitingPublication. No test/expectation was weakened. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::uncertain_child_ownership` | Focused reproduction **0/1/999**, 1.53s. Corrected early-probe read failure to publication RecoveryRequired + cleanup AwaitingPublication, without replacement/transport; focused GREEN **1/0/999**, 1.80s. |
| `devenv shell -- cargo test --locked --lib promotion::run_tests::quality3_` | After that compatibility correction, **5/0/995**, 4.11s. |
| `devenv shell -- cargo fmt` / `devenv shell -- cargo fmt --check` | Two focused-correction formatting/check runs each, exit 0; unrelated WIP hashes still match. No blanket lint allow or lint rollback. |
| `devenv shell -- cargo test --locked --lib promotion` | Final **193 passed/0 failed/807 filtered**, **200.36s**, warning-free. Prior concurrency fence, inside-child authority priority, context/no-execution cancellation, lease-free, mode/type, SQL/T6-1 and other promotion tests retained. |

Only focused and promotion selectors ran. No check/Clippy/full-suite/Task 7 SSH/
native workflow or CLI smoke was rerun. The fixture unwind panics are intentional
and caught to model executor loss after durable completion, not waived failures.

### Exact Correction Scope / Review Return

Changed by this correction worker only:

1. `src/repository/promotion/run.rs` — exact child-bound publication witness,
   read-only authority reader, no OID fallback, existing-authority-first retry gate;
   existing Task 8 lint folds and concurrent complete-record handoff retained.
2. `src/repository/promotion/run_tests.rs` — five new meaningful read-failure/live-
   witness/retry-window tests and their local hooks/fixtures; existing Task 8 boxed
   transport test, ModeImage alias and barrier lint refactor retained.
3. This execution ledger — appended this correction record; Task 8 evidence above
   preserved, including its prior 3,049-test full gate.
4. New canonical comment `01M5237Z00N4Q6S8V0X2Z4B6EM` — quality recheck handoff.

No persisted field, schema or migration was added; existing publication evidence
is used. All other Task 8 WIP paths remain byte-identical to entry, verified by
the table above. Full current pending inventory is the original fourteen modified
tracked paths plus the original Task 8 comment and this new response comment.
**Same quality recheck next, then controller-directed final gate rerun.** No
policy change, subagent, staging, commit, push, PR, CI, merge, closure or worktree
cleanup. Exact base/current HEAD remains 504cb44 above; ticket stays open.

Final correction checks: `git diff --check` and `git diff --cached --exit-code`
exit 0; nothing staged. Status is the original fourteen modified tracked paths
plus the preserved Task 8 comment and this new response comment. All twelve
unrelated-WIP blob witnesses were rechecked after formatting/documentation and
still match entry exactly. Source is unchanged since the final 193-test GREEN;
only the ledger append/comment followed. No broad gate or commit was performed.

## Task 8 Final Post-Review Full Gate — Verified / Ready For Controller Checkpoint

**2026-10-17 UTC: final local gate COMPLETE.** The controller reports whole-branch
**SPEC PASS** and the **same final QUALITY reviewer APPROVE, no findings** after
the two IMPORTANT authority/cancellation findings were corrected and all five
new regressions passed. The preceding correction handoff/comment's pending-recheck
language is historical and superseded by this controller-reported approval.
No new review was dispatched by this worker; no approval is inferred from tests.
The reviewed scope is the entire `origin/main..HEAD` plus the uncommitted Task 8
work and quality corrections, not `HEAD~1`. Owner review, native authorization/
execution and later outward actions remain gated; ticket stays **OPEN**.

### Final-Tree Boundary And One New Lint

This pass resumed the intentionally dirty exact ticket worktree/branch at
**504cb44aa7f8e1eebd5996666851ebe34c47f78d**. Re-read worktree AGENTS, correction
ledger and comment `01M5237Z00N4Q6S8V0X2Z4B6EM`. Preserved all existing WIP.
Only source edit by this verification pass: remove an immediately invoked closure
in `promotion/run.rs::make_outcome` after the initial all-target/all-feature Clippy
probe reported **one `redundant_closure_call` diagnostic**, exit 101. Explicit
`cfg(test)` read-hook chaining and `cfg(not(test))` direct authority read preserve
the reviewed behavior and fault seam. No suppression, product behavior change,
new test, dependency, schema or global configuration edit. The already-approved
exact-child-bound authority, read-only recovery-pending fallback and completion-
before-outer-cancel corrections remain intact. Full testing followed this narrow
lint fix; no additional focused or broad test runs were needed in this pass.

### Exact Fresh Commands And Results

Every Cargo command used Devenv in the specified ticket worktree. Required final
gates below ran after the lint fix. **Full suite ran exactly once in this pass**,
with a 10,800,000ms outer tool allowance, unchanged per-case watchdog, no pipeline,
filter, failure bypass or target omission. It completed normally with exit 0.

| Exact command | Actual result |
| --- | --- |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Initial probe exit 101, one new closure-call lint; after narrow refactor, required final gate exit 0, warning-free, 19.04s. |
| `devenv shell -- cargo check --all-features --locked` | Exit 0, 5.16s. |
| `devenv shell -- cargo fmt --check` | Exit 0; no formatter rewrite needed. |
| `devenv shell -- cargo test --all-features --locked` | **Exit 0; 3,054 passed / 0 failed**. Compile 1m38s; library 1,000 passed in 564.39s. All 35 integration targets and doctests completed. One existing ignored opt-in read performance characterization; no disabled/waived failure. |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | Exit 0; build 11.33s; no application output/window, as expected for the existing skeleton. |
| `git diff --check` | Exit 0 after source edit and final evidence edits. |

Full output: `/home/vhodges/.local/share/opencode/tool-output/tool_148be2566001dEBnpP9rvo3yYa`.
These are fresh results observed by this worker, not the prior 3,049-run or the
correction implementer's targeted 193-test report. The full run independently
executes the **193 promotion tests**, including the five named quality cases below.

| Final standard target | Passed | Ignored |
| --- | ---: | ---: |
| library | **1,000** | 0 |
| desktop binary / CLI binary | 0 / 0 | 0 |
| canonical_foundation | 62 | 0 |
| discovery_rebuild | 78 | 0 |
| editor_feasibility | 9 | 0 |
| journal_rejection | 26 | 0 |
| key_material | 39 | 0 |
| key_storage | 2 | 0 |
| local_authoring | 140 | 0 |
| mutation_bridges | 129 | 0 |
| mutation_confirmation | 154 | 0 |
| mutation_contract | 43 | 0 |
| mutation_relationships | 27 | 0 |
| mutation_replay | 308 | 0 |
| read_boundary | 20 | 0 |
| read_characterization | 0 | 1 (existing opt-in characterization) |
| read_comments | 22 | 0 |
| read_contract | 71 | 0 |
| read_credentials | 25 | 0 |
| read_items | 76 | 0 |
| read_relationships | 17 | 0 |
| read_repository | 33 | 0 |
| read_status | 23 | 0 |
| recovery_foundation_gate | 51 | 0 |
| remote_reservation | 13 | 0 |
| repository_enablement | 78 | 0 |
| session_credentials | 11 | 0 |
| shared_key_registry | 39 | 0 |
| doctests | 11 | 0 |

| Final SSH target | Passed |
| --- | ---: |
| comment_publication | 41 |
| document_promotion | 74 |
| mutation_remote | 88 |
| remote_merge_recovery | 25 |
| remote_observation | 15 |
| remote_polling | 84 |
| remote_synchronization | 63 |
| ssh_fixture | 35 |
| ssh_transport | 122 |
| **SSH total** | **547** |

All targets have **zero failures**. Total: **1,000 library + 1,496 standard
integration + 547 SSH + 11 doctests = 3,054 passes**.

### Acceptance Mapping And Review Findings — Final Run

The **18-row named-test acceptance mapping in the historical Task 8 section is
retained and revalidated by this final full run**: those named tests all passed
on the corrected tree. Its native row still explicitly records pending native
execution, not platform success. All approved T0/T2/T3/T4/T6/T7 qualifications
remain unchanged (ownership-first, mode/type digest, RFC identity persistence,
Git-first/SQL-atomic finalization, verified operator-repair backend-lock exception,
retained-only unknown-history R1 and accepted discovery-title exception).

Add these fresh named regressions to acceptance **rows 11, 13, 16** (accurate
publication, once-only retry/cancel priority and refresh/outcome evidence):

| Named test, prefix `repository::promotion::run_tests::` | Final observed proof |
| --- | --- |
| `quality3_published_replay_read_failure_never_uses_integration_oid` | PASS; completed replay with unavailable outcome read reports explicit recovery pending, never substitutes integration M for exact Published P; identical later replay recovers P without effects. |
| `quality3_current_replay_read_failure_never_uses_integration_oid` | PASS; same guarantee for actual AlreadyCurrent authority, including exact kind/OID. |
| `quality3_live_known_authority_does_not_depend_on_a_second_outcome_lookup` | PASS; both actual completion kinds carry the child-bound known P through live refresh/render without redundant outcome lookup. |
| `quality3_retry_window_cancel_preserves_published_authority` | PASS; durable primary Published completion acknowledged/persisted before concurrent outer retry cancellation, cleanup pending Cancelled, no child IO/republication, both starting parent steps. |
| `quality3_retry_window_cancel_preserves_current_authority` | PASS; same durable-step/once-only guarantee for AlreadyCurrent completion. |

The existing uncertain-ownership regression also passed in this full run,
preserving publication RecoveryRequired + cleanup AwaitingPublication. The
reviewer's two IMPORTANT findings are controller-reported **resolved/approved**;
the five regressions plus the original lifecycle/concurrency tests now have a
fresh complete-gate result. No pending independent-review redispatch remains.

### Fingerprint, Lockfiles And Integration Proof Boundary

Before the full run and again after it/CLI, this exact read-only command returned
the same Git blob witness of the complete verification-scope diff:

```sh
git diff --binary origin/main -- src tests Cargo.toml Cargo.lock devenv.nix devenv.lock .github | git hash-object --stdin
```

**`2549bc99929448e16b837045fcbc2e40127ce09a`**. Scope includes full tracked
source/tests/manifests/toolchain/workflow relative to actual main, including
uncommitted lint and authority/cancel corrections. It excludes evolving evidence
documents/comments. This is a **diff fingerprint, not a committed Git tree OID**.
No source/test changes followed the full gate; final edits are evidence only.

`sha256sum Cargo.lock devenv.lock` before/after the full gate, unchanged:

| File | Start = end SHA-256 |
| --- | --- |
| `Cargo.lock` | `8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8` |
| `devenv.lock` | `d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e` |

Fresh **`git fetch origin main`** still observes
**89d04f901636229392a311241f8e777f0a00962b**; no new main delta. No mid-task
rebase/merge/stage/checkout. **`git merge-tree --write-tree HEAD origin/main`**
again exits 0, tree **9fa0a194c03bd43640b82b9bf0bfa59f67a18d7e**; HEAD stays
**504cb44aa7f8e1eebd5996666851ebe34c47f78d**. **This proves HEAD only and does
not include uncommitted final edits. Post-checkpoint final-HEAD merge-tree proof
is explicitly pending for the controller.** No temporary commit/index/ref was
created to overstate that proof. Main is an ancestor of the unchanged HEAD;
the controller will verify the final committed descendant at checkpoint.

### Completion / Remaining Gates / Exact Pending Paths

Local implementation/final verification and independent final reviews are
complete. Ready for the controller's final checkpoint; after committing, run
the final-HEAD integration proof. Ticket stays **OPEN awaiting owner review**
and applicable native/outward authorization. Native four-target manual execution
remains pending, not dispatched. Prior actual Wayland surface smoke is retained:
the changes are domain-only, with no GUI change requiring a repeat. This pass
ran only the required CLI smoke. No dependency/global config/developer key/GPUI
change, sleep, subagent, staging, commit, push, PR, CI, merge, closure or cleanup.

All pending paths, relative to the ticket worktree (**14 tracked modifications,
three untracked canonical comments; nothing staged**):

```text
docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md
docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-execution.md
docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md
docs/plans/2026-10-15-wave-02-cycle-09-handoff.md
src/repository.rs
src/repository/bridges/identity.rs
src/repository/promotion/integrate.rs
src/repository/promotion/mod.rs
src/repository/promotion/observe.rs
src/repository/promotion/run.rs
src/repository/promotion/run_tests.rs
src/repository/remote/reservation.rs
tests/recovery_foundation_gate.rs
tests/ssh_transport/synchronization.rs
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M5227Z00N4Q6S8V0X2Z4B6EK.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M5237Z00N4Q6S8V0X2Z4B6EM.md
.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/01M5247Z00N4Q6S8V0X2Z4B6EN.md
```

## Controller Final Checkpoint And Committed Integration Proof — 2026-10-17

- Committed every reviewed, final-gate-verified implementation/evidence path as
  `62e8e207aa31b72ec87e27dac8699378d0d7c70f`
  (`fix: finalize promotion publication authority and verification`).
- Fresh `git fetch origin main` still observed
  `89d04f901636229392a311241f8e777f0a00962b`.
- **Committed final implementation proof:** `git merge-tree --write-tree HEAD
  origin/main` exited 0 with tree
  `7077b29a643f785f8577f351dbc90318d218c4ec`;
  `git merge-base --is-ancestor origin/main HEAD` exited 0. This supersedes the
  historical HEAD-only proof that excluded Task 8 WIP. `git status --porcelain`
  was empty after the checkpoint.
- Controller inspected the retained final full-run output: all 1,000 library,
  1,496 standard integration, 547 SSH and 11 doctest passes, including the 74
  promotion SSH cases; no failed result. The pre-existing opt-in ignored
  characterization remains explicitly qualified. No redundant Cargo run.
- This final documentation/comment checkpoint changes no tested Rust source,
  tests, lockfile, manifest or workflow. All Tasks 0–8 are complete at the local
  implementation/review/verification gate. Ticket remains **OPEN for owner
  review**. Native execution is pending separate authorization; no push, PR,
  workflow dispatch, merge, closure or worktree cleanup occurred.
- The published ticket ref remains divergent from its planning-time rebase.
  Any future authorized push must reconcile it without an unapproved force push.

## Publication And First Native Dispatch — 2026-10-17

- Owner authorized pushing the ticket branch and running Build once. After the
  actual divergence was inspected, owner explicitly approved a history-only merge
  preserving the original published checkpoint, followed by a normal push.
- Fresh remote ticket SHA was `f017c738161b8acc7b54b88055471cf7ae551765`.
  It was the sole remote-exclusive commit. Its stable patch ID and rebased
  `69176ba` both equal `e656e8a4e97f26efa2b8a52c91bd4d9143d23158`.
- History-only merge `5161f1d8882bb30204dba18576469a200a9b4693` has exactly the
  pre-merge verified tree `e5ad7abdb9475b6ecc4008f58318dae29cd97b8e`.
  `git diff --exit-code HEAD^ HEAD` and remote ancestry both passed.
- Normal push to `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` succeeded;
  `git ls-remote` verified that exact SHA. No force push.
- `gh workflow run build.yml --ref manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`
  created [Build run 39023714430](https://github.com/vhodges/manyhands/actions/runs/39023714430).
  Readback confirmed `workflow_dispatch`, the correct branch and exact SHA above,
  and all four native jobs queued/running. This is dispatch evidence, not a pass.
- The following documentation/comment-only checkpoint records this receipt; the
  implementation tested by that run remains unchanged. No second run is authorized
  or dispatched here. Ticket remains open; PR, merge, closure and worktree cleanup
  have not been authorized or performed.

## Native Failure Checkpoint — 2026-10-17

**Status: two corrections ready for controller review/native verification;
macOS initiating failure still needs the focused native diagnostic run.**

### Authority, Base And Native Evidence

- Work remains exclusively in ticket worktree
  `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH`,
  branch `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`, HEAD `a5a2e90`.
  Its only controller change since published `923cac8` is the temporary
  three-platform/exact-22-test workflow narrowing. The restoration baseline is
  `923cac8:.github/workflows/build.yml`.
- Controller already fetched unchanged main `89d04f9`, aborted its plain rebase
  that tried to flatten approved merge history, and completed
  `git rebase --rebase-merges origin/main` without changing `923cac8`.
  That completed preflight was retained; no second rebase was performed here.
- Owner authorized investigation/correction and focused publication followed by
  restoration/full verification. Controller owns review, commits, pushes and
  dispatches. This pass performed none of those and used no subagents.
- Initial native run **39023714430**, implementation SHA **5161f1d**: Linux
  x86_64 passed; Linux aarch64 failed one mutation replay case; macOS aarch64
  failed one F11 SSH case; Windows x86_64 failed 20 unit cases. These are actual
  failures, superseding the preceding dispatch-only/pending receipt.
- Source log:
  `/home/vhodges/.local/share/opencode/tool-output/tool_149eb917e001RXpK15aiSUDN7L`.
  Read ranges: 13412–13538 (Windows), 4988–5001 (ARM), 9117–9121 (macOS).
  No native result after the corrections is available yet.

### Windows: Full Snapshot Of An Owned Mandatory-Locked File

All 20 native panics are `OsError 33` at filesystem snapshot reads: 17 in
`cleanup_tests.rs:59`, one in `integrate_tests.rs:72`, two in
`observe_tests.rs:1095`. Source tracing identifies the held common-directory
lease, `.git/manyhands-operation.lock`: `coordination::repository_lease` locks
it exclusively; `acquire` creates without truncation and writes no bytes.
Each failing snapshot opens a second handle while that lease is alive.
Windows mandatory locking rejects that read even within the same process.
Earlier snapshot patterns remove that exact coordination path, but these
assertions can preserve stronger coverage without removing any entry.

The shared **test-only** `coordination::snapshot_file_hash` recognizes only
`root/.git/manyhands-operation.lock`, proves it is a regular zero-length file
using `symlink_metadata`, and records its empty-content digest. Existence remains
part of every snapshot map; nonempty/nonregular files fail. All other paths
still require successful full reads. No arbitrary errors, `.git`, registry,
index, refs, user files or namesake lock files are excluded. Lease lifetime and
preview/no-acquisition assertions remain intact. Production locking is unchanged.

Portable regression
`repository::coordination::tests::held_lease_snapshot_checks_all_protected_files_and_empty_lease`
was RED with the old read-only helper (unexpected nonempty coordination content
was not rejected), then GREEN. It also proves index/ref/config/registry/user
and nested namesake bytes change their digests, missing coordination files fail,
and snapshotting leaves the lease held. Native Windows verification remains
required; Linux does not emulate mandatory locking.

### Linux ARM: Lost Request Commit During Duplicate-Owned Discovery

Native failure was not a permissible null/no-op: `discovery_pending` returned
`write=unchanged`, `checkpoint=unchanged`, `commit_oid=null` while the same
request's duplicate had committed and owned discovery. The original assertion
requiring the authoritative commit stays unchanged.

The deterministic channel-barrier regression is included inside the existing
exact selected two-process case, before its original two-process exercise.
It pauses the original call after acceptance, lets a duplicate commit and pause
at the existing observation hook, then lets the original enter the domain.
`save_ticket_with_effective_config` correctly protects the active index owner
by returning `IndexPending/NoChange` for this call. The first-call mapper
incorrectly treated its empty domain claim as proof the entire request made
no commit. The regression reproduced the native null-versus-commit failure
locally before the correction (0 passed, 1 failed).

`mutation/bind/save.rs::run` now handles this unclaimed pending-discovery result
with the same journal and intended-content/range evidence used by re-entry.
It never substitutes HEAD or loosens ownership guards; unreadable evidence
remains unconfirmed. The test also verifies one commit, duplicate/replay OID
agreement, resume recovery, and no remaining pending journal row. Eight exact
affected/ownership-guard cases passed locally after the correction.

### macOS: Initiating Failure Unresolved; Diagnostic Deadlock Corrected

The saved native output contains only the 100-second isolated-child/watchdog
failure. Adjacent ordinary-refresh and takeover F11 cases passed. The exact
active-child case passes locally. A temporary Unix symlink-root probe also
passed with the expected owner epochs; that probe was removed, not used as
evidence of a macOS fix. Production promotion preflight canonicalizes the root,
and the root-scoped F11/shared-index fences have not been weakened.

Source inspection did establish an independent test-harness deadlock: after a
worker fails before invoking its hook, that hook's receiver stays alive in the
service. Cleanup's zero-capacity `send` then waits forever for a receiver that
will never execute. Hook-entry sends can likewise strand a late worker after
the receiving side has timed out. The local regression holds a receiver alive
without entering the hook and proves cleanup must return before it receives;
with the original rendezvous channel it failed at its explicit assertion
(isolated case: 0 passed, 1 failed; privacy-safe line observation 101).

Queued entry/release signals now avoid those unbounded sends. Existing bounded
hook waits, outer watchdog, privacy scan, actual F11 child exit, index-epoch,
foreign-root isolation and publication assertions remain in force. All three
affected F11 cases passed locally, including the cleanup regression. This fixes
the diagnostic deadlock, **not a proven cause of the original macOS worker
failure**. The next focused native run is needed before further production edits.

Numeric diagnostics use existing `ssh_harness::observation` and its captured
output privacy checks; no raw result, endpoint, path, OID or fixture data is
printed. Prefix **2110** identifies these checkpoints:

| Checkpoint | Meaning / trailing value |
| --- | --- |
| 0 | Fixture ready; root already canonical (0/1) |
| 1 | Initial promotion interruption established |
| 2 / 3 | Older / foreign owner entered persist hook |
| 4 | Compound worker returned; fixed result category |
| 5 / 6 | Controller received older / foreign hook entry |
| 7 | Both owner-row captures succeeded (0/1) |
| 8 | Actual F11 subprocess exited as expected |
| 9 | Completion/fence capture succeeded (0/1) |
| 10 / 11 | Older / foreign release sent |

Checkpoint 4 categories: 0 complete cleanup, 1 complete envelope with cleanup
pending, 2 index pending; 10 repository error, 11 transport, 12 blocked,
13 external change, 14 reconfirmation, 15 primary advanced, 16 identity,
17 context synchronization, 18 busy, 19 poll yielding, 20 interrupted,
21 operation mismatch, 22 recovery required. Existing **2100** still records
compound flag and before/fenced/final owner epochs (locally **1/2/3**).

### Commands And Observed Local Results

All Rust commands used Devenv from this ticket worktree. No full library/suite,
Clippy, desktop, dependency or configuration gate was run for this native-failure
pass, per the affected-tests-only rule. Final focused results: **21 unit + 8
mutation replay + 3 SSH cases passed**, zero ignored in these selections.

Windows: first ran the new coordination regression alone with
`devenv shell -- cargo test --locked --lib repository::coordination::tests::held_lease_snapshot_checks_all_protected_files_and_empty_lease -- --exact`
(RED). After correction ran the following exact selection (GREEN, **21 passed,
980 filtered out**); repeated after the save correction and additional lock-held
assertions with the same final count:

```sh
devenv shell -- cargo test --locked --lib -- \
  repository::coordination::tests::held_lease_snapshot_checks_all_protected_files_and_empty_lease \
  repository::cleanup::tests::cleanup_branch_deletion_checks_oid_and_every_checkout \
  repository::cleanup::tests::cleanup_branch_guard_keeps_checkout_with_missing_commondir_registration \
  repository::cleanup::tests::cleanup_branch_guard_keeps_checkout_with_missing_gitdir_registration \
  repository::cleanup::tests::cleanup_branch_guard_requires_complete_admin_evidence \
  repository::cleanup::tests::cleanup_foreign_git_link_cannot_authorize_prune \
  repository::cleanup::tests::cleanup_foreign_git_link_cannot_hide_registered_checkout \
  repository::cleanup::tests::cleanup_hidden_index_assume_unchanged_preserves_edit_and_index \
  repository::cleanup::tests::cleanup_hidden_index_skip_worktree_preserves_edit_and_index \
  repository::cleanup::tests::cleanup_ignored_and_locked_are_preserved \
  repository::cleanup::tests::cleanup_partial_and_mixed_worktrees_require_recovery_without_more_removal \
  repository::cleanup::tests::cleanup_precheck_clean_and_dirty_cases_preserve_full_git_image \
  repository::cleanup::tests::cleanup_submodule_embedded_git_data_is_not_proved_safe_to_remove \
  repository::cleanup::tests::cleanup_submodule_hidden_dirty_files_are_preserved_recursively \
  repository::cleanup::tests::cleanup_submodule_ignored_files_are_preserved_recursively \
  repository::cleanup::tests::cleanup_submodule_unknown_or_advanced_state_is_not_safe_to_remove \
  repository::cleanup::tests::cleanup_tracking_guards_missing_expectation_zero_and_foreign_locks \
  repository::cleanup::tests::cleanup_tracking_ref_refusals_preserve_git_and_sql \
  repository::promotion::integrate::tests::persistence_seam_holds_one_lease_and_creates_no_commit_before_integrating_evidence \
  repository::promotion::observe::tests::prepared_generated_blob_is_bound_without_odb_or_other_writes \
  repository::promotion::observe::tests::preview_is_read_only_offline_and_does_not_take_repository_lease --exact
```

ARM deterministic RED:
`devenv shell -- cargo test --locked --test mutation_replay reenter::two_processes_submitting_the_same_new_request_make_one_commit -- --exact`.
GREEN (**8 passed, 300 filtered out**):

```sh
devenv shell -- cargo test --locked --test mutation_replay -- \
  reenter::two_processes_submitting_the_same_new_request_make_one_commit \
  reenter::a_second_call_that_read_its_evidence_before_the_first_committed_reports_that_commit \
  reenter::a_second_call_whose_save_is_refused_after_the_first_committed_reports_that_commit \
  reenter::a_second_call_that_runs_before_the_first_reaches_the_domain_commits_and_finishes \
  reenter::a_save_that_changed_nothing_does_not_take_a_later_commit_of_the_same_content \
  reenter::identical_content_from_another_request_is_already_applied_and_not_this_request_s_commit \
  reenter::a_request_that_never_ran_does_not_take_another_request_s_identical_commit \
  reenter::a_retry_that_cannot_read_git_changes_nothing_and_claims_nothing --exact
```

macOS investigation used
`devenv shell -- cargo test --locked --test document_promotion quality2_f11_preserves_active_child_handoff -- --exact`
for the unchanged local case (1 passed), temporary alias probe (1 passed), and
cleanup regression with original rendezvous channel (RED, 1 failed). After the
channel correction and diagnostics:
`devenv shell -- cargo test --locked --test document_promotion quality2_f11 --`
selected exactly the three named F11 cases and reported **3 SSH cases passed**.

`devenv shell -- cargo fmt --check` initially identified formatting only; after
applying its changes with `apply_patch`, the final check passed.
`git diff --check` passed. `git diff --exit-code -- .github/workflows/build.yml`
passed: workflow remains exactly the controller's temporary narrowing.

### Review / Native Next Step

Controller can review these uncommitted corrections and publish/dispatch the
existing focused workflow under the recorded authorization. It already selects
the deterministic ARM regression and macOS cleanup regression via their existing
test names. The separate coordination-helper regression runs locally and will
be included by the restored full library selection; the focused Windows matrix
still selects exactly the original 20 failures. Inspect the macOS numeric
checkpoints if it fails; do not declare native fixed from these Linux passes.
Only after focused native verification is green should the controller restore
the full workflow from `923cac8` in its own commit and run full native coverage.
Ticket remains OPEN; no closure or outward actions occurred in this pass.

### Quality Recheck P2 — Canonical Snapshot Fixture Root

Specification review passed; quality review identified a portable fixture-path
defect in the new held-lease snapshot regression. `tempfile::tempdir()` retained
an aliased TMPDIR spelling while `repo.commondir()` canonicalized it. The helper's
deliberately exact path comparison therefore missed the lease file, and the
nonempty-content rejection assertion failed at line 249. This also applies to
macOS `/var` versus `/private/var` spellings.

Reviewer-provided RED: the exact test with
`TMPDIR=/proc/self/root/tmp/opencode` reported **0 passed, 1 failed**; the
canonical `/tmp/opencode` control reported **1 passed**. Source inspection
confirmed that mismatch. The fixture now uses
`tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap())`, matching
the existing cleanup fixture. The helper's exact-path, empty-file and
regular-file requirements are unchanged.

Fresh verification after the one-line fixture correction:

```sh
devenv shell -- env TMPDIR=/proc/self/root/tmp/opencode cargo test --locked --lib repository::coordination::tests::held_lease_snapshot_checks_all_protected_files_and_empty_lease -- --exact
devenv shell -- env TMPDIR=/tmp/opencode cargo test --locked --lib repository::coordination::tests::held_lease_snapshot_checks_all_protected_files_and_empty_lease -- --exact
```

Both commands passed: **1 passed, 0 failed, 0 ignored, 1000 filtered out** each.
P2 correction is ready for the same quality recheck. This is fixture portability
evidence, not native-platform verification; the preceding native gates remain.
No workflow changes or commits were made.
