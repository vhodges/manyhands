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
