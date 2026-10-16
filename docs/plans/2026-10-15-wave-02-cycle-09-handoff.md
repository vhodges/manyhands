# Wave 02 Cycle 09: Fresh-Session Handoff

## Current: Task 0 reviewed; Task 1 next (2026-10-16)

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
Independent spec review passed; quality review approved after three
documentation consistency fixes. No production code or retained test/Cargo
changes exist yet. Task 1 is next.

### Where things are

| Item | State |
| --- | --- |
| Worktree | `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Work only here, never in the main checkout. |
| Branch | `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` |
| Task 0 resume base | `df4fe56ce0548290293c3f6d96f23f1ddbfc51a2` (reviewed blocker evidence); planning checkpoint `d027d98`; `origin/main` remains `89d04f901636229392a311241f8e777f0a00962b`. Fresh controller fetch/rebase was a clean no-op; ancestry passed. |
| Published ticket ref | `origin/manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` still at the pre-rebase `f017c73`. A push must reconcile it. Never force-push without explicit owner authority bound to the observed remote SHA. |
| Checkpoints | Planning and reviewed blocker evidence are committed. Task 0 resume evidence and amendment passed spec and quality review and are ready for the authorized checkpoint commit. |
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
   the grounding facts, planning reviews, Task 0 evidence and T0-1 approval.
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
  8. In one final transaction: deletes the tracking ref, deletes the history
     row, inserts `retired_contexts`, and records `complete`.
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
2. Task 0 passed independent spec then quality review. Before subsequent
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
3. Then work Tasks 1–8 in order. Each task starts with a failing test and its
   recorded red run. Each ends with a commit (only when authorized), a ticket
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
