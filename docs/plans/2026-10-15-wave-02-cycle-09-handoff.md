# Wave 02 Cycle 09: Fresh-Session Handoff

## Current: Task 6 reviewed complete at local gate; Task 7 after controller checkpoint (2026-10-16)

Tasks 0–5 are reviewed/committed; Task 5 checkpoint is `8d06577`, the current
unchanged HEAD. Full Task 6 work is uncommitted in this ticket worktree and has
**SPEC PASS and QUALITY FINAL APPROVE, with no remaining findings**, conveyed by
the controller/owner. Review covers the complete Task 6 change against exact base
**`8d06577f6af17e2532d072d7ee14419dbf8a84b4`**, including the concurrency fence,
authoritative-completion fix, helper extractions and all earlier corrections,
not merely the last diff. No Task 6 review or owner decision remains pending.
Final recorded targeted gate:
`devenv shell -- cargo test --locked --lib promotion` **188 passed/0 failed/807
filtered**, 188.09s, warning-free. Formatter/check pass. No Task 7/full/native gate
or worker commit; **controller checkpoint is next, then Task 7**. Exact file
inventory, changes, RED/GREEN, acceptance mapping and final reviews are recorded
in the execution ledger. This coordination pass changes only documentation and
a canonical comment; it runs no Rust commands/tests and makes no commit. Earlier
handoff states below are historical and superseded by this reviewed-complete status.

The owner approved T6-1 through the question tool: inherit Cycle 06's existing
backend-lock exception (Cycle 06 contract 228–242; approved 2026-10-08 Task 4
resolution protocol 36–46). Independent review confirmed the current refusal is
safe. Ambiguous libgit2 locks after process exit must be preserved with `Ok`
cleanup pending RecoveryRequired plus refresh/IndexPending. Only after an operator
quiesces writers and verifies/repairs metadata may the identical request resume
unfinished SQL. No automatic lock removal/adoption, new lock journal or new owner
question. The rare inside-F10/F11 interval explicitly costs operator intervention.

Retain the real crash case and assert full preservation/no repeated effect
counters before **any** repair. In the disposable fixture only, simulate quiesced
verified operator removal of the exact known lock, then complete under the same
request. Existing SQL history/retirement/reservation and committed-outcome replay
invariants remain those of T4-1. This qualifies every inside-F10/F11 immediate-
retry/finish requirement below; normal recovery remains automatic. Task 6's complete
implementation and qualified recovery tests are now reviewed accepted; T6-1 is
approved history, not a pending owner question. Task 7 starts only after the
controller checkpoint; Task 8/full/native and C5/D5 frontend gates remain later work.
Exact approval, RED/GREEN and final reviews are in the execution ledger.

## Historical planning and Task 5 handoff (superseded)

You are picking up Wave 02 Cycle 09, **Confirmed Managed-Document
Promotion**. It is tracked by Manyhands ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DH`.

The owner approved the Cycle, the design and the implementation plan on
2026-10-15. The owner also accepted the recommended answer to owner decisions
D1–D7 and internal ruling R1.

Implementation and checkpoint commits are authorized in subagent-driven
development mode, with independent spec then quality review. Targeted tests
only until Task 8 override the full Task 0 baseline and Task 6 library run.
Task 0's delete/prune probes are verified; its interrupted-retained-conflict
blocker was reviewed and the owner approved the ownership-first amendment.
The remaining Task 0 characterizations are recorded in the ledger, including
the full replay/restart matrix, merge-tree equivalence and fresh edit context.
Task 0 passed independent spec and quality review and is committed as `25514a3`.
Task 1 guarded remote context deletion passed both reviews and is committed as
`3d47027`. Task 2 observation/preview, including its spec corrections and approved
T2-1 mode/type amendment, passed both final reviews and is committed as
`6d4ad1664462c42c24b4a820024d76d7c86ca8db`.

**Task 3 primary integration is reviewed complete at the local gate and committed
as `3b925e141bec9ae309024bdbfbf73aa6ab48caae` (`3b925e1`).** Its prior
spec review PASS is retained; the same quality reviewer approved Q3-1 resolved
with no findings. Those reviews covered the Task 3 change set, including the
include-context correction, against base `6d4ad16` (full SHA above); the controller
has since committed it. **T4-1 is owner-approved; Task 4 passed both final reviews
and is committed as `f641f917113cc252f429860ba5092efcbbfa260c`. Task 5 is now
reviewed complete at the local gate, uncommitted for the controller's checkpoint.
Task 6 is next after that checkpoint.** Current HEAD and full Task 5 review base
are `f641f917113cc252f429860ba5092efcbbfa260c`.

Final controller-reported Task 5 implementation reviews:

- **SPEC PASS**, including the P2 pre-effect refresh identity correction.
- **QUALITY APPROVED**, no fatal issues; its single nonblocking lookup minor is
  resolved in the current tree and targeted-verified.

The review boundary is the full uncommitted Task 5 change set against that exact
base, not merely the last correction. Prior pending-SPEC states are superseded;
historical RED/GREEN and review evidence remain in the execution ledger and
canonical comment `01M51H7Z00N4Q6S8V0X2Z4B6E1`. Approvals are controller-reported;
no additional reviewer execution or minor-fix re-review is inferred. Full Task
8/native gates remain deferred; controller owns the pending checkpoint commit.

### Task 5 delivery and Task 6 handoff

Task 5 delivers audited document_promotions storage, typed steps/categories and
stored refusals, persisted child attempts and ownership-first retry seams,
cross-journal IDs, promote arbitration/restart/epoch fencing, F1–F12 names and
parent-step read status. T4-1 completion composes retirement, complete and
applicable reservation finish in one SQL transaction, with actual-state commit
reconciliation and the private Git evidence/ref lock retained by the caller.
SPEC P2 closes create_folder's pre-effect guard hole while preserving the exact
authoritative child refresh exception.

The final quality minor replaces full-history work in two bounded lookups:
begin_record selects the requested item's unfinished row with the partial-index
predicate; explicit-ID status uses read_record. Listing and startup audit retain
full history. Identity guards, stored refusal replay and outcome semantics stay
unchanged. Latest exact checks:

- `devenv shell -- cargo fmt` — exit 0.
- `devenv shell -- cargo test --locked --lib promotion::record` — **23 passed,
  0 failed, 897 filtered**.
- `devenv shell -- cargo test --locked --lib read::status` — **10 passed,
  0 failed, 910 filtered**.

Earlier 45 reservation, 43 recovery, required inventory and focused actual-folder
checks remain recorded evidence; no broad rerun is inferred. No test was added
to mirror the query implementation, as requested. Task 6 has not started. After
the controller's checkpoint, compose the real runner with Task 3's integration
stages and Task 5's storage/transaction seams; preserve ownership-first child
selection, integrating/branch_deleted-before-effect ordering and evidence lifetime
through SQL commit/reconciliation. Tasks 6/7 still prove real F1–F12/F10/F11 and
SSH orchestration; Task 8 owns the full local gates.

### Task 4: historical blocker and owner-approved recovery boundary

The original completion transaction required atomicity across a Git
tracking-ref deletion and SQLite history/retirement/completion state. Existing
Git and SQLite transactions commit independently. The temporary real-resource
probe confirmed SQL rollback cannot restore an already-deleted Git ref; its
exact commands/counts and source anchors are retained in the ledger. The probe
was removed before implementation. The historical reviews below apply to that
blocker evidence; Task 4's new RED/GREEN evidence is in the ledger's resume entry.

- **Independent SPEC confirmed** a material contract contradiction requiring an
  **owner amendment**.
- **Independent QUALITY approved the evidence without findings.**
- Both reviews were read-only; no reviewer reruns or code changes occurred.
  These are blocker-evidence reviews, not implementation approval. Full Task 8
  and native gates remain deferred.
- The controller checkpointed this reviewed evidence as `79463c5`. Its historical
  STOP/NOT APPROVED status is superseded by the owner's approval below. See the
  ledger and canonical comment `01M51F7Z00N4Q6S8V0X2Z4B6DZ` for the original record.

**T4-1 amendment — OWNER APPROVED 2026-10-16.** The owner answered “yes” to
the controller's final Git-first proposal. The historical STOP/review evidence
above and in the ledger is superseded by this approval. Resume the full original
Task 4 test/helper/R1 scope. Fresh controller fetch/rebase was a clean no-op at
`79463c53beb400d95a311e558e64caf8efd2beac`; origin/main remains `89d04f9`,
ancestry passed. No additional worker rebase is needed in this execution.

1. Persist the existing planned durable **`branch_deleted` intent** before the
   tracking-ref effect. No new phase or table is needed by this amendment.
2. Under the lease and ownership/frozen-configuration checks, delete the tracking
   ref only at the expected **direct OID**, or proceed on verified true absence.
   Wrong/new conflicting OIDs, symbolic refs (including dangling symbolic refs)
   and unreadable refs stop cleanup.
3. Then use **one SQL transaction** for history deletion, retirement insertion,
   promotion completion and applicable reservation finish. This transaction does
   not atomically include or roll back the preceding Git effect.
4. During the split interval the ref may be absent, but applicable **existing**
   SQL history remains intact (`observed_published` when present), no retirement
   exists, the durable record is pending and the applicable reservation stays
   held. Do not manufacture `observed_published` history for local-only or
   cache-loss cases. SQL statement failure followed by rollback of the uncommitted
   transaction preserves SQL, not an already-deleted Git ref. An uncertain commit
   result does not prove SQL unchanged: reopen and reconcile actual completion.
   If completion and applicable reservation finish committed, replay the stored
   outcome. Git deletion may remain whether SQL rolled back or committed.
5. After-integration failures return **`Ok` with cleanup pending
   `RecoveryRequired`, plus refresh**. Same-ID retry performs actual ref,
   ownership and configuration checks and finalizes SQL without repeating
   completed lifecycle work, or replays the stored outcome when actual completion
   already committed. **Cache loss or ref absence alone is not retirement
   proof**; do not infer retirement without the required durable intent/checks.
6. Add tests for split boundaries **inside F10/F11**, SQL statement and commit
   failures/uncertainty, split-state existing-history preservation, same-ID
   finalization and wrong/conflicting/symbolic/unreadable ref refusals. Cover
   reopening/reconciling uncertain commit results and replay of committed
   completion/reservation finish. Tests for rollback of uncommitted SQL must
   distinguish SQL preservation from the surviving Git deletion.

This is the approved recovery boundary. Task 4 supplies helpers/private evidence
and R1 migration; Tasks 5/6 later supply durable intent/completion/reservation
composition, with inside-F10/F11 lifecycle tests in Tasks 5–7. Do not implement
fake Task 5 storage in Task 4. That reviewed implementation is committed as
`f641f91`; Task 5 supplies the real storage/completion composition above.

Delivered Task 4: lease-owning generic cleanup helpers for both document/ticket
contexts; ignored/untracked/locked/partial preservation; guarded branch deletion;
Git-first direct tracking deletion with private evidence/absent-slot ref lock;
SQL-only retirement; additive migrated/audited R1 table and per-item publication
evidence. Observed publication consumes retirement in the same SQL batch; other
history rules stay unchanged. Current observation projection rotates to a new
batch excluding the retired item, preserving the original pinned synchronization
pass. Synchronization/poll/merge production code was not changed. Standalone tests
also prove SQL trigger/outer rollback/deferred-commit failure, fresh Edit from
primary HEAD, foreign .git-link refusals and unchanged snapshot invariants.
Reviewed corrections also preserve branches when registrations are incomplete
through an unfiltered physical admin inventory, reject hidden index flags without
index mutation, and explicitly prove populated submodule cleanliness recursively
under the existing lease with bounded/fail-closed inspection.

The runner must retain evidence/ref lock through SQL commit/reconciliation and
compose real branch_deleted intent, ownership/generation/configuration fencing,
promotion completion and reservation finish. Task 5 supplies the SQL composition;
Tasks 6/7 still own actual runner F10/F11 and uncertain-result outcome/refresh
tests. Task 4 does not
implement those records or claim native Windows pruning was executed. Exact
commands/counts and final review approvals are in the ledger and comment
`01M51G7Z00N4Q6S8V0X2Z4B6E0`.
Retained evidence includes final **34 cleanup tests passed**, **67 state tests**,
**10 refs tests**, inventory and focused poll/SSH regressions. These are prior
executions, not reruns by this status update.

### Task 3: reviewed integration and identity composition

- Delivered leased re-observation/digest/base guards, exact merge subject/tree/
  parent order, one guarded primary update, contained handling and first-parent
  reconciliation with pending recovery after uncertain effects.
- Immutable request/evidence and lease-owned preparation/candidate stages are
  the later storage/runner boundary. Task 5 persists integrating evidence before
  `write_commit`; Task 6 composes the lifecycle and maps pending recovery.
- **T3-1 is resolved as an internal engineering ruling, not a pending owner
  decision.** Existing RFC policy requires atomic local identity persistence
  before commit. The historical commit-only suggestion is superseded. Shared
  selection retains sync validation/journaling; promotion uses the bridge's
  lease-held atomic primitive without its differing public-setter policy.
- **Q3-1 is reviewed resolved.** Before installation, proposed identity is checked
  against native include expansion in the original repository context, including
  relative/nested/conditional includes. Preserve the approved correction.
- Reuse recorded evidence: final integration **31 passed / 0 failed**, focused
  include regressions **3 passed / 0 failed**, shared bridge identity **42 passed /
  0 failed**. The earlier promotion selector passed all 53 Task 2 observation
  tests plus then-current integration tests, and the existing sync identity SSH
  regression passed. Exact commands/intermediate results are in the ledger;
  these are prior executions, not reruns or inferred reviewer test results.
- Final review status is recorded in the ledger and comment
  `01M51E7Z00N4Q6S8V0X2Z4B6DY`. Full Task 8/native gates remain deferred.

### Retained Task 2 history: T2-1 approved, implemented and reviewed resolved

The original approved digest encoded changed path/category/new blob OID but not resulting
Git mode/type. With a remote, moving context OIDs are deliberately excluded.
Temporary real-Git probes confirmed equal digests for `extra` changing from
regular `100644` to symlink `120000`, or to executable `100755`, with the same
blob OID. Both Added and Modified cases were reproduced against an exactly
unchanged primary; both previews stayed Ready and their path/category change
vectors were equal. All four expected digest-inequality assertions failed.

Exact RED: `devenv shell -- cargo test --locked --lib promotion::observe::tests::t2_1_ -- --nocapture`
→ **0 passed / 4 failed / 822 filtered**. The temporary probes were removed;
`devenv shell -- cargo test --locked --lib promotion::observe`
→ **48 passed / 0 failed / 774 filtered**. That historical preservation pass did
not clear T2-1; the owner subsequently approved the encoding amendment below.

**Approved and implemented:** immediately after each optional new OID, bind the
resulting Git entry mode including type bits as a four-byte **u32 big endian**.
Added/Modified use the result Git tree entry; Deleted encodes absent OID and
**zero u32**, not its old mode. Evidence is private; public DTOs are unchanged.
Keep domain `manyhands promotion v1`, finalizing an unshipped encoding. D6 still
excludes moving commit/tree OIDs and unrelated primary-only paths.

Retained regression RED/GREEN:
`devenv shell -- cargo test --locked --lib promotion::observe::tests::t2_1_`
→ **0 passed / 5 failed / 822 filtered** before the hash fix, then **5 passed /
0 failed / 822 filtered**. This includes all four mode transitions plus an
independent fixed-width/deletion-sentinel protocol vector. Final
`devenv shell -- cargo test --locked --lib promotion::observe`
→ **53 passed / 0 failed / 774 filtered**, preserving prior spec fixes and
primary-only/context-primary-merge invariance. Formatter/check passed.

Cycle/design/plan now include the approved amendment. Full historical blocker,
approval, implementation and verification evidence is in the execution ledger;
comments `01M5187Z00N4Q6S8V0X2Z4B6DR` (blocker) and
`01M5197Z00N4Q6S8V0X2Z4B6DS` (approved delivery). The prior spec path/mode-policy
findings remain resolved. Final quality and spec rechecks both passed, recorded
in the ledger's final review section and comment `01M51A7Z00N4Q6S8V0X2Z4B6DT`.
That historical coordination-only update changed no source and reran no tests.

### Where things are

| Item | State |
| --- | --- |
| Worktree | `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Work only here, never in the main checkout. |
| Branch | `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` |
| Current HEAD / Task 5 review base | `f641f917113cc252f429860ba5092efcbbfa260c`: reviewed Task 4 checkpoint. Task 5's full change set is uncommitted against it. |
| Task 0 resume base | `df4fe56ce0548290293c3f6d96f23f1ddbfc51a2` (reviewed blocker evidence); planning checkpoint `d027d98`; `origin/main` remains `89d04f901636229392a311241f8e777f0a00962b`. Fresh controller fetch/rebase was a clean no-op; ancestry passed. |
| Published ticket ref | `origin/manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` still at the pre-rebase `f017c73`. A push must reconcile it. Never force-push without explicit owner authority bound to the observed remote SHA. |
| Checkpoints | Tasks 0–4 reviewed/committed (`25514a3`, `3d47027`, `6d4ad16`, `3b925e1`, `f641f91`); T4-1 owner-approved. Task 5 reviewed complete: SPEC PASS, QUALITY approved, nonblocking lookup minor resolved/verified. Controller checkpoint pending; Task 6 next afterward. |
| Ongoing-task preflight | Reuse the controller's completed fetch/rebase/ancestry preflight against `origin/main` at `89d04f901636229392a311241f8e777f0a00962b`. This status pass performs no fetch, rebase or checkpoint commit. |
| Main checkout | It has an unrelated untracked `.superpowers/`. Leave it alone. |

### Read in this order

1. `AGENTS.md` (repository rules), then this brief.
2. `docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md`: the
   contract, the decisions with their accepted costs, and the acceptance
   table.
3. `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md`:
   the interfaces, the change set and digest, the records, the F1–F12
   lifecycle, the reconciliation, and the child-attempt table.
4. `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md`:
   Tasks 0–8. Follow it exactly.
5. `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-execution.md`:
   the grounding facts, planning reviews, Task 0 evidence/T0-1 approval, Task 1
   reviewed delivery, Task 2's approved T2-1 correction, and Task 3's internal
   ruling, Q3-1 correction and final independent review results, then Task 4's
    historical T4-1 blocker/reviews, owner approval and full Task 4 resume evidence.
   Record your evidence here as you go.

Cycle 08's documents (`docs/Cycles/wave-02-cycle-08-*` and
`docs/plans/2026-10-11-*`) are the closest precedent for style, ledger entries
and the shape of the real-SSH test target.

### What is being built (one paragraph)

Two operations are being added:

- `preview_document_promotion`. It is local and read-only: no network, no
  lease, no writes. It returns either every blocker or an exact preview: the
  primary-path change set, the integration kind, the publication and cleanup
  effects, and a BLAKE3 `PromotionExpectation`.
- `promote_document`. It is bound to that digest. With a publication remote,
  it does the following in order:
  1. Runs a context-synchronization child.
  2. Re-checks under the lease.
  3. Makes one `Promote document <ULID>` merge with parents `[base,
     context]`.
  4. Runs a primary-synchronization child.
  5. Deletes the remote context branch, guarded by `push_negotiation`.
  6. Removes the worktree.
  7. Deletes the local branch.
   8. After durable branch_deleted intent and fresh guards, delete tracking first;
      then atomically retire history, insert `retired_contexts`, record completion
      and finish applicable reservation in SQL (approved T4-1).
  9. Refreshes the index on every `Ok` return after integration.

Without a remote, it merges locally, cleans up, and reports publication
pending. The parent record `document_promotions` binds child attempts by ID,
the same way comment publication binds its child synchronization.

### Accepted decisions, in brief

| ID | Accepted answer |
| --- | --- |
| D1 | Promotion takes no draft. It requires a clean, conforming context and validates the document in the integrated tree. |
| D2 | A local primary that is ahead of or diverged from `T(P)` blocks (`PrimaryAheadOfRemote`). |
| D3 | Without a remote, the merge is computed in memory, and a conflict is refused before the primary changes. |
| D4 | New context content arriving during synchronization gives `ReconfirmationRequired`, and the record ends `abandoned`. |
| D5 | A deleted, or history-unknown, remote context blocks only when `T(C)` is absent. An already-contained context is exempt. |
| D6 | The digest binds the change set and the D2 relation, not the moving OIDs (when a remote is configured). This refines the Git RFC's "refs/OIDs" binding, and the owner accepted it. |
| D7 | Ignored files in the context worktree block cleanup (`pending(ignored_files)`). Never delete them. |
| R1 | `retired_contexts` lets a fresh context of a promoted item publish for the first time, even when repository `history_unknown` is set. It is written only in the completion transaction. |

### Invariants that the review proved easy to break

- **Integration base.** With a remote, require `base ≤ context_oid` under the
  integration lease. A poll can advance `T(P)` between the children. Without
  this check, a merge that takes the context tree silently drops primary
  commits. The failure is `PrimaryAdvanced`, and the retry re-synchronizes
  with a new attempt.
- **Nothing strands.** Pre-integration refusals end `abandoned`.
  `context_changed`, `ignored_files` and `remote_context_changed` rows are
  superseded by a newly confirmed promotion. For context and primary children,
  authoritative completion is never rerun. Retained conflict/owned-resolution
  evidence takes precedence over errors: an interrupted retained child keeps
  its original attempt ID with restart; retained `RecoveryRequired` also
  restarts that ID. Only confirmed absence of retained ownership permits a
  new attempt for cancelled/interrupted/rejected/offline/busy/yielding or
  recovery failures. Uncertainty is not absence. Task 0's evidence confirms
  the real child behavior and passed independent reviews.
- **Applicable existing history stays until completion** (`observed_published`
  when present). Never fabricate it in local-only/cache-loss cases. Early cleanup
  must not grant new first-publication eligibility.
- **Errors mean the primary is unchanged.** Integration reconciliation
  searches for a reachable matching merge. Recovery after integration is a
  pending category, never an error.
- **Lease rules.** No network call, prompt or discovery scan holds the lease.
  Every local step re-observes under the lease first.
- **Records stay content-free.** They hold only IDs, ref names, OIDs,
  timestamps and fixed categories. Never a title, body, path content,
  endpoint or secret.

### How to resume

1. Preserve existing authorization and reviewed evidence; read the latest
   execution ledger. The controller owns checkpoint commits after reviews.
2. **Task 5 is reviewed complete at the local gate against `f641f91`: SPEC PASS,
   QUALITY approved and the nonblocking lookup minor resolved/verified.**
   Controller checkpoint is pending; **Task 6 is next after it**. Reuse evidence
   and approvals rather than redispatching completed work. T4-1 remains approved;
   no pending owner decision. This worker makes no checkpoint commit or Task 6 change.
3. Tasks 0–5 passed their independent reviews. Before subsequent
   implementation in a fresh session:
   1. Fetch, then rebase the ticket branch onto `origin/main` from inside
      the worktree. Never rebase main onto the branch.
   2. Re-scan callers if the source changed.
   3. Reuse the recorded guarded-delete, prune, in-memory merge equivalence,
      fresh-context and child replay/restart findings. Review the execution
      ledger's Task 6 integration notes, especially row existence, cancellation
      acknowledgement when retiring an active reservation, and ownership.
   4. Run targeted tests only until Task 8.

   A characterization that contradicts the design stops work for an owner
   amendment. Do not adapt silently.
4. After the controller's Task 5 checkpoint, continue Tasks 6–8
   in order, retaining the approved T4-1 boundary. Each task
   starts with a failing test and its recorded red run. Each ends with a commit
   (only when authorized), a ticket
   comment, and an independent review before the next task. The plan's
   dependency notes allow Tasks 1, 2, 4 and 5 to proceed independently after
   Task 0.

### Repository rules you must follow

- Run every Rust command as `devenv shell -- cargo …`. The full Task 8 gate is:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

  Until Task 8, the owner requires targeted tests only, including Tasks 0
  and 6. Do not run the cargo gate on Sprites: that experiment was
  abandoned.
- **Ticket comments.** Write them directly as canonical files at
  `.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/<ULID>.md`, with the same
  front matter as the existing comments. Add one at every plan checkpoint:
  progress, decisions, blockers, verification and review-ready. Use the
  Manyhands CLI instead once it supports comments.
- **Native CI.** `.github/workflows/build.yml` runs only by manual dispatch.
  Ask the owner before **each** run, since approval does not carry over. The
  current matrix has four targets: Windows aarch64 was dropped by owner
  decision during Cycle 08.
  - On a platform failure: narrow the matrix in a commit of its own, fix the
    failure, run only the affected tests locally, then re-dispatch.
  - Once green, restore the matrix in its own commit and run the full set
    once more.
  - Add `--test document_promotion` to the workflow's test list (Task 7).
- **Before calling the Cycle review-ready,** run `git merge-tree` against
  current `origin/main` to prove the branch still integrates.
- **Each needs its own owner authorization:** commit, push, pull request,
  native dispatch, merge, closure and worktree cleanup. The ticket closes
  only after code-review or pull-request approval, normally in the final
  pre-merge push.

### Known downstream items (not this Cycle)

- **Wave 03 C5/D5 (whichever comes first)** binds this operation at the F2
  boundary: `prepare`/`execute`, the ten-minute expiry, the preview schema
  values, and changing `bind/resume.rs` `NotResumable` for promote. Task 6
  updates `docs/research/wave-03-api-audit.md` to say so.
- **Cycle 10 (ticket closure)** reuses `src/repository/cleanup.rs` and
  `retired_contexts` unchanged.
- **Collaborators' clones.** A collaborator's clone keeps its local context
  after promotion, and its poll reports it `remotely_deleted`. Explicitly
  discarding that context is later work.
