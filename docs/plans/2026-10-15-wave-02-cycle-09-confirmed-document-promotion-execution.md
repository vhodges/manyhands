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
