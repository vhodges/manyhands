# Wave 02 Cycle 09: Fresh-Session Handoff

## Current: Task 3 committed; Task 4 STOP at T4-1, owner decision pending (2026-10-16)

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
as `3b925e141bec9ae309024bdbfbf73aa6ab48caae` (`3b925e1`), the current HEAD.** Its prior
spec review PASS is retained; the same quality reviewer approved Q3-1 resolved
with no findings. Those reviews covered the Task 3 change set, including the
include-context correction, against base `6d4ad16` (full SHA above); the controller
has since committed it. **Task 4 is stopped at T4-1 pending an owner amendment.**
No Task 4 production changes remain. This status update changes no source/tests
and reruns no Rust tests.

### Task 4: reviewed blocker T4-1 — owner decision pending

The approved final completion transaction requires atomicity across a Git
tracking-ref deletion and SQLite history/retirement/completion state. Existing
Git and SQLite transactions commit independently. The temporary real-resource
probe confirmed SQL rollback cannot restore an already-deleted Git ref; its
exact commands/counts and source anchors are retained in the ledger. The probe
was removed; Task 4 implementation RED/GREEN has not started.

- **Independent SPEC confirmed** a material contract contradiction requiring an
  **owner amendment**.
- **Independent QUALITY approved the evidence without findings.**
- Both reviews were read-only; no reviewer reruns or code changes occurred.
  These are blocker-evidence reviews, not implementation approval. Full Task 8
  and native gates remain deferred.
- **STOP: no amendment is approved.** The controller will checkpoint the reviewed
  blocker evidence; this status worker makes no commit. See the ledger and
  canonical comment `01M51F7Z00N4Q6S8V0X2Z4B6DZ`.

**Proposed minimal amendment — NOT APPROVED:**

1. Persist the existing planned durable **`branch_deleted` intent** before the
   tracking-ref effect. No new phase or table is needed by this proposal.
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

This is a proposal for owner decision, not an amendment to the approved contract.
Do not implement it or continue Task 4 until that decision is recorded.

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
This latest coordination-only update changed no source and reran no tests.

### Where things are

| Item | State |
| --- | --- |
| Worktree | `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Work only here, never in the main checkout. |
| Branch | `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` |
| Current HEAD / Task 3 checkpoint | `3b925e141bec9ae309024bdbfbf73aa6ab48caae`: reviewed and committed Task 3 primary integration. Its review base was `6d4ad1664462c42c24b4a820024d76d7c86ca8db`. |
| Task 0 resume base | `df4fe56ce0548290293c3f6d96f23f1ddbfc51a2` (reviewed blocker evidence); planning checkpoint `d027d98`; `origin/main` remains `89d04f901636229392a311241f8e777f0a00962b`. Fresh controller fetch/rebase was a clean no-op; ancestry passed. |
| Published ticket ref | `origin/manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` still at the pre-rebase `f017c73`. A push must reconcile it. Never force-push without explicit owner authority bound to the observed remote SHA. |
| Checkpoints | Tasks 0–3 are reviewed and committed (`25514a3`, `3d47027`, `6d4ad16`, `3b925e1`). Task 4 STOP at T4-1: SPEC confirmed owner amendment required; QUALITY approved evidence without findings. Owner decision pending; no production changes. Controller checkpoint of blocker evidence pending. |
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
   T4-1 blocker, read-only reviews and unapproved minimal amendment proposal.
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
  8. The approved contract calls for one final transaction deleting the tracking
     ref/history row, inserting `retired_contexts` and recording `complete`.
     **T4-1 blocks this cross-resource boundary; see the unapproved proposal
     above. Do not treat that proposal as the current approved contract.**
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
- **History stays `observed_published` until completion.** A cleanup that
  stops early must leave the context under the RFC's explicit-republish rule.
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
2. **Task 3 passed both reviews and is committed as `3b925e1`. Task 4 is STOP at
   T4-1; owner decision pending.** SPEC confirmed a material contradiction requiring
   owner amendment; QUALITY approved the evidence without findings. Both reviews
   were read-only, with no reruns. Reuse the retained evidence; do not redispatch
   completed work or start implementing the unapproved proposal. The controller
   owns the pending checkpoint of reviewed blocker evidence.
3. Tasks 0–3 passed their independent reviews. Before subsequent
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
4. Only after the owner resolves T4-1 and the approved documents consistently
   record that decision, resume Task 4 and work Tasks 4–8 in order. Each task
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
