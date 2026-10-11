---
title: "Wave 02 Cycle 08 One-Shot Polling And Materialization Execution Ledger"
date: 2026-10-11
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M4NM26RWRM6ATDHN16HV1M9S"
---

# One-Shot Polling And Materialization Execution Ledger

## Planning Authorization And Preflight — 2026-10-11

The owner asked for the Wave 02 Cycle 08 Cycle document, design and detailed
implementation plan, a review of them, and presentation for review and
approval, on existing ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DG`. Authorization is
planning only. No implementation, commit, push, CI dispatch, merge, closure or
cleanup was authorized or performed.

- Repository: `/home/vhodges/work/src/manyhands`.
- Verified registered ticket worktree:
  `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DG`.
- Branch: `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DG`.
- Ticket worktree was clean before the rebase. The main checkout had unrelated
  untracked `.superpowers/`; left untouched. Other worktrees untouched.
- Fresh `git fetch origin` succeeded; local main and `origin/main` agreed at
  `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17` (Cycle 07 merge, pull request 17).
- Before ticket HEAD: `5539c60eeb4022c71b1dc13716aeb2d2545a7375`.
- Rebase onto the fetched base succeeded without conflicts (386 commits behind,
  one ticket checkpoint replayed).
- After ticket HEAD: `27d5ef54c67a116a590602d86b7086f8602c0c62`.
- `merge-base --is-ancestor` passed; post-rebase status was clean. Main was
  never rebased or switched.
- The published ticket ref still holds the original checkpoint, so the branch
  reads ahead 387 and behind 1. A later push must reconcile that ref; a force
  push is not authorized.
- `manyhands-cli` is startup-only with no ticket or comment operation, so
  ticket and comment updates use the canonical filesystem paths.

## Grounding

Read `starting-a-cycle` with its preflight reference, `review-cycle-docs` with
its matrix, `AGENTS.md` and the ticket. Read Wave 02 in full; the polling,
remote-ref, coordination and reservation sections of the Git workflow, index,
MVP, runtime and test RFCs; the CLI contract's poll rows; the Wave 03 API
audit's polling gaps; and the Cycle 04 and Cycle 07 documents, design and
ledger for contract and format. Inspected `remote/{observation,reservation,
state,sync,refs}.rs`, `transport/{remote,operation,callbacks}.rs`,
`repository.rs` context creation, `discovery.rs`, `recovery.rs`,
`read/{status,dto}.rs`, `results.rs`, the SSH fixture and its test targets,
`Cargo.toml` and the native workflow.

Source inspection used three delegated read-only passes plus direct reads of
the reservation finish logic, the journal-begin guard, the operation-table
schema, the snapshot reader and the failure-point seam. No Rust command was
run. Findings that shaped the design are listed under Current Source Boundary
in the design.

## Artifacts

- [Cycle](../Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md)
- [Design](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md)
- [Detailed implementation plan](2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md)

## Skill-Based Self-Review

Applied `review-cycle-docs` and every row of its matrix to the three documents
against the authorities and the inspected source.

| Source | Concern / promised behavior | Classification | Resolution / ruling | Proof or follow-up |
| --- | --- | --- | --- | --- |
| Git RFC, Remote Refs | "Before a poll prunes stale tracking refs" implies pruning; a tracking ref can be the only local reference to a collaborator's commits | **Decision D1** | Recommended: never delete a tracking ref in this Cycle; cost is accumulating stale refs until Cycles 09/10 | Owner answer; Task 3 no-deletion tests |
| Wave 02 "materializes exactly once"; Git RFC "never clean up" | An interrupted worktree creation can leave a partial directory; finishing or removing it are both recovery policy | **Decision D2** | Recommended: resume only branch-created-with-no-path; report anything else and remove nothing; cost is a rare manual repair | Owner answer; Task 0 characterization; Task 5 interruption tests |
| Cycle 06/07 conflict rule; reservation code | A pending conflict makes every synchronization `Busy` but does not stop a poll, and a poll moves tracking refs | **Decision D3** | Recommended: refuse before transport with a typed non-failure result; cost is no background discovery until the conflict is resolved, with no abandon path yet | Owner answer; Task 0 characterization; Tasks 2, 6, 7 |
| MVP and Git RFC manual priority; `recovery.rs` guard | A local save or comment begun during a poll is refused with `RecoveryRequired`; the full poll widens that window from an advertisement to a fetch and checkout | **Decision D4** | Recommended: against a poll-priority record, request yield and return `RepositoryBusy`; cost is a narrow change to a Wave 01 guard | Owner answer; Task 2 and Task 7 tests |
| Wave 02 gate "obeys stored policy"; existing observe ignores policy | Whether the domain or the caller enforces eligibility | Ruling | Ruling: the poll gates automatic invocations itself through one pure function also exposed read-only — the Wave gate requires it and Wave 03 needs the same answer — cost if wrong: Wave 03 double-checks harmlessly | Tasks 1, 6, 7 |
| Runtime RFC "Explicit Poll now bypasses automatic pause/backoff" | The RFC names pause and backoff; `enabled = 0` and recovery suspension are not named | Ruling | Ruling: explicit bypasses all four because each gates automatic polling, and clears none — matches the existing explicit observation — cost if wrong: one extra deliberate fetch; revisit if the owner wants `enabled = 0` to forbid explicit polls | Task 1 and Task 7 policy cases |
| Runtime RFC clock-rollback clamp | A stateless clamp never becomes due | Ruling | Ruling: persist the clamped deadline once — only a stored bound counts down — cost if wrong: one extra write in an otherwise read-only check | Task 1 clamp tests |
| Runtime RFC "host failure never becomes ordinary network backoff" | Next eligible time after key, unlock and host failures | Ruling | Ruling: store the category, keep backoff, use the interval; session suspension stays Wave 03 state — cost if wrong: a background attempt per interval that returns without prompting | Tasks 1, 2, 7 |
| Git RFC poll refspecs | Wildcards would download malformed family names and give them tracking refs | Ruling | Ruling: explicit mappings for names that parse, equivalent to the RFC set restricted to recognized contexts — nothing may use a malformed ref — cost if wrong: a malformed ref's objects are not local | Task 3 |
| Cycle 04 observation model | Reuse the advertisement-only operation or add one | Ruling | Ruling: new operation plus stored poll kind; observe unchanged — its callers rely on no local change — cost if wrong: one column and a rule fork | Task 2 frozen-behaviour tests |
| Operation table `CHECK` lists | New phase or safe-point names force a table rebuild | Ruling | Ruling: reuse `local_prepared`, `before_local_mutation`, `before_discovery`; verified the schema has no action-to-phase constraint — cost if wrong: status text shared with synchronization | Task 2 |
| Restart contract | Cycle 04 refuses restart at `BeforeLocalMutation` | Ruling | Ruling: `update` polls may restart from any step because each local step acts only on state re-observed under the lease — cost if wrong: an extra fetch | Tasks 2, 6, 7 reopen cases |
| Poll ID versus local ID rule | A poll cannot share its ID with the refresh | Ruling | Ruling: durable refresh-required flag before completion, then a refresh under a fresh internal ID; no `index_pending` on the poll record — cost if wrong: a stale index repaired by any refresh | Task 6 index tests |
| Index RFC "appear as recovery problems" | Poll exceptions written to `problems` would be erased by the refresh | Ruling | Ruling: store fixed dispositions with fixed guidance in remote state, exposed on the remote snapshot — the refresh owns `problems` — cost if wrong: Wave 03 maps dispositions into its problem view | Task 6 snapshot tests; Wave 03 read DTO obligation |
| MVP RFC "local tracking branch" | Whether to write upstream branch configuration | Ruling | Ruling: none; no existing context has any and tracking derives from the ref plan — cost if wrong: an additive later write | Task 5 |
| MVP RFC primary fast-forward | A primary branch not checked out could be moved safely | Ruling | Ruling: leave it, as synchronization does, and report `inaccessible` — cost if wrong: one unchanged branch | Tasks 4, 7 |
| Git RFC item synchronization step 2 | Integrating primary into a context | Settled | A poll compares a context with its own tracking ref only; anything else is a merge | Task 4 |
| Remote rename | No rename signal exists in Git | Settled | Old name `remotely_deleted`, new name `nonconforming` when the item ID no longer matches; local context kept | Task 7 rename case |
| Authentication RFC; Cycle 03/04 session model | Prompts and trust during an automatic poll | Settled | Existing automatic-scope behaviour: no prompt when the session is blocked; host approval typed and never implied | Tasks 3, 7 |
| Privacy contract | New tables and columns | Settled | IDs, ref names, OIDs, timestamps and fixed enumerations only; scans extended | Tasks 1, 6, 7 |
| Runtime RFC transport limits | No total poll deadline | Settled | Per-call limits stand; in-transfer stop is added through the progress callback as the Git RFC requires | Task 0, Task 3, native evidence |
| Test strategy RFC | Fixture fidelity | Settled | Real in-process SSH with Git helpers for every transport claim; no fake transport; library tests cover only post-fetch logic on real local repositories | Tasks 3, 6, 7 |
| Native platforms | Progress-callback stop, worktree checkout failure and path length differ by platform | Feasibility gap | Not provable by inspection; Task 0 characterizes on Linux, native runs need separate dispatch authorization | Task 0; Task 8 native record |
| Lifecycle | Closure and publication | Settled | Ticket stays open; closure in the final pre-merge push after approval; every outward step separately authorized | Task 8 |

At review time four product and recovery questions were open for the owner
(D1–D4); all four were decided as recommended, recorded below. Task 0
characterizations and native evidence are open **gates**, not decisions.

### Corrections Made During Self-Review

- The first plan draft relied on a "fixture transport seam" for library tests.
  None exists. Task 6 now isolates the post-fetch stage as one function that
  library tests drive on a real local repository, and leaves every transport
  claim to real SSH.
- The first plan draft said "the existing failure-point seam" covers
  materialization. The seam is the public `FailurePoint` enumeration, which
  needs new poll variants. Task 5 now says so.
- Checked the operation table for a constraint tying `local_prepared` to
  synchronization actions before relying on it; there is none, and the active
  reservation index and the local journal guard already include that phase.
- The design first compared the clamp statelessly. It now persists the bound.
- Confirmed the three documents agree on dispositions, the result table, the
  decision markers and the task that proves each acceptance row.

## Verification And Approval Checkpoints

Planning checks only: internal links resolve, document IDs are unique canonical
ULIDs, and the new files have no whitespace errors. No Rust or native test was
run; all verification in the plan is future work.

Approval needed before implementation: the Cycle, design and plan; answers to
D1–D4; and separate implementation authorization. Proposed lifecycle
checkpoints are one ticket comment per task, an independent review per task,
the full local gate, a whole-branch review, native runs when authorized, and
closure in the final pre-merge push after approval.

## Document Approval And Commit Authorization — 2026-10-11T14:39:22Z

The owner approved the Cycle, design and implementation plan, approved the
recommended answer to each open decision, and authorized committing this
planning checkpoint.

| Decision | Answer |
| --- | --- |
| D1 stale tracking refs | Never delete a tracking ref in this Cycle |
| D2 half-created worktree | Resume only branch-created-with-no-path; report anything else and remove nothing |
| D3 polling during a pending conflict | Refuse before transport with a typed non-failure result |
| D4 local action during an automatic poll | Request yield and return retryable `RepositoryBusy` |

Document status changed from proposed to approved in all four artifacts. The
approval does not authorize implementation, push, CI dispatch, merge, closure
or cleanup. The planning commit is local; the published ticket ref still holds
the pre-rebase checkpoint and was not force-pushed.
