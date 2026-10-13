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

## Task 0 Execution — 2026-10-11

### Authority, Scope And Base

The owner explicitly authorized rebase, Tasks 0–8 implementation, local commits
and canonical ticket/comment/ledger updates. This implementer was assigned
**Task 0 only**, inline with no subagents, followed by independent spec and
quality reviews. The 2026-10-11 document approval and D1–D4 answers at
`2026-10-11T14:39:22Z` were re-read, not reopened:

- D1: never delete tracking refs.
- D2: resume only an intent's branch at its recorded OID with no path and no
  registration; retain and report all half-created worktrees.
- D3: a pending synchronization conflict blocks the new full poll before
  transport, with a typed non-failure result.
- D4: a local action during an automatic poll requests yield and receives
  retryable `RepositoryBusy`.

All shell commands used the assigned workdir
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DG`
and branch `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DG`. Read current
`AGENTS.md`, the implementing skill's preflight/verification references, all
three approved documents, this ledger and the ticket's existing comments.
The CLI remains startup-only, so canonical files were used for bookkeeping.

Preflight, before mutation:

- `git worktree list --porcelain` confirmed the assigned registration and branch.
- Ticket status was clean. Main status was read for preflight only: unrelated
  untracked `.superpowers/` was preserved; other worktrees were untouched.
- `git rev-list --left-right --count main...origin/main` returned `0 0`.
- Fresh `git fetch origin main` succeeded. Rebase onto `origin/main` was a
  clean no-op, with no conflicts and no `AGENTS.md` change.
- Base: `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`.
- Before and after rebase / **Task 0 base HEAD**:
  `094e5734d1d4480ee29e6f675aaba4a8883b9060`.
- `git merge-base --is-ancestor origin/main HEAD` returned success;
  post-rebase `git status --short --branch` showed a clean ticket branch.

### Caller Re-Scan And Wave 03 Compatibility

Re-scanned `src/` and `tests/` for `observe_publication_remote`,
`reserve_remote_operation*`, `remote_safe_point`, `finish_remote_operation`
and `RemoteSnapshot`; also re-read the Wave 03 API audit's C2/C4, E-observe,
E-policy, E-reserve and `poll once` rows. `git diff origin/main -- src tests
docs/research/wave-03-api-audit.md` was empty before characterization: no new
caller or changed production assumption since the approved planning base.

Production consumers remain observation orchestration in `remote/observation.rs`,
reservation/safe-point consumers in `remote/sync.rs`, and snapshot construction,
read and re-exports in `remote/state.rs`, `remote/mod.rs` and `repository.rs`.
The other direct callers are existing library and acceptance tests (including
comment publication and read-status tests). Neither frontend calls the remote
operations, and no implemented Wave 03 consumer was found.

Carry these compatibility items into the later tasks:

1. Wave 03 W3-02/W3-12 maps service-owned reservation tokens and durable
   cancel/yield acknowledgement into bounded-worker progress; a request is not
   an immediate cancellation of a blocking SSH call.
2. E-observe remains advertisement-only, with its existing replay, policy and
   safe-point behavior unchanged. The new full poll must be a separate API.
3. E-policy/E-reserve and W3-06/W3-12 `poll once` need the new eligibility,
   disposition and attempt-time accessors later; existing `RemoteSnapshot`
   publication/history fields and stored-row validation must retain their meaning.
4. The desktop scheduler/session attention state and unsaved-draft protection
   remain downstream obligations. This Linux characterization is not native
   matrix proof.

### Five Executed Throwaway Characterizations

Temporary assertions were added only to existing test targets. No production
behavior, dependency, target or workflow was changed. Real transport claims
used the in-process `russh` fixture and Git upload-pack, isolated homes and
pre-thread initialization. No fake transport, sleeps in test/controller code,
developer keys or global Git configuration were used; transfer pacing is the
existing fixture facility. No `cfg(test)` hook was used by a harness-false target.

**1. Stored tracking OID and snapshot equality — PASS.**
`task0_tracking_batch_equality` in `remote/reservation_tests.rs` created two
real local commits, committed an observation with tracking OID A, then moved
the actual context tracking ref to B before another batch. The ordinary
snapshot and a freshly reopened service still returned the unchanged A
snapshot. Committing the next batch with tracking OID B put B in both its
observation and context snapshot. A negative control changed only the stored
`remote_context_states.tracking_oid` back to A: snapshot returned
`RecoveryRequired`. Thus tracking OID is the value captured when the observation
is constructed, not a live ref or an expected pre-fetch value. The equality
check in `state.rs:4567–4582` compares the two **stored** rows, not Git refs.
Writing tracking refs before constructing/committing the complete batch is
compatible, and a crash between those stages does not itself corrupt a snapshot.

**2. Interrupted and failed synchronization after tracking movement — PASS.**
`task0_restart_moved_tracking` in `tests/remote_synchronization.rs` ran two
real-SSH two-clone scenarios. A fixture SQL trigger requested yield on
`AfterFetch`; the service returned `Interrupted` after fetching A, with local
main still at the base. The interrupted variant retained that state. The
failed variant explicitly reacquired the reservation, reached `BeforeFetch`
and used the public `finish_remote_operation(ProtocolRejected)` transition;
SQL confirmed literal `failed` rather than merely a failed call. This is a
synthetic durable failure boundary, not a claim about a network failure.
In both variants the fixture rewound the tracking ref to the base and the
peer published a newer B whose commit was absent locally. A reopened service
with the same operation ID and `restart=true` opened **13 fresh Git helpers**,
downloaded B, advanced local main and tracking to B, and returned
`Complete(AlreadyCurrent { oid: B })`. No receiver update occurred during the
restart. No stale result was replayed, and no deletion was inferred.
Runner evidence: `0 13` (interrupted) and `1 13` (failed).

**3. Parked conflict resolution and tracking refs (D3) — PASS.**
`task0_conflict_tracking_movement` in `tests/remote_merge_recovery.rs` parked
a real canonical ticket conflict with recorded local/incoming parents L/R.
It captured the inspection token, then moved both primary and context tracking
refs to L. Inspection still returned the same observation token; the incoming
side still matched the blob at R. `resolve_all` made a local checkpoint with
ordered parents `[L, R]`, kept both tracking refs at L, and made zero new
authenticated connections and zero receiver updates. Resolution uses the
recorded integration parents and conflict evidence, not current tracking tips.
Source grounding: `resolve_synchronization`/recovery inspection in `sync.rs`
and the fixed HEAD/branch/packed-ref storage checks in `RefLogSnapshot`.
This finding does **not** repeal D3: the approved full poll still refuses before
transport during a pending conflict. Runner evidence: `1 1 1` for unchanged
token, frozen-parent resolution and no transport.

**4. Part-way git2 worktree checkout failure (D2) — PASS.**
`task0_partial_worktree_checkout` used real `git2::Repository::worktree` with
the same default `WorktreeAddOptions::reference` as local authoring. The tree
contained ordinary file `a` and symlink `z` with a 5,000-byte target, causing a
real Linux filesystem failure after `a` was written. `worktree` returned
`GenericError`; the path, `a`, worktree `.git` file, admin directory and worktree
registration all remained. `z` was not created. The admin directory contained
exactly `gitdir`, `HEAD`, `commondir` (no index); the branch remained at its
created OID. Observed output:
`path=true a=true z=false gitfile=true admin=true registered=true`.
A targeted recheck replaced the initial directory predicate for `z` with
`symlink_metadata(...).kind() == NotFound`, asserted the exact sorted admin
entry set, and opened the registered worktree to assert its original symbolic
branch and OID. All those explicit assertions passed.
This is a retained half-created worktree, not the branch-only resumable case.
No cleanup/adoption was attempted. This proves the Linux failure shape only,
not Windows path-length behavior or every possible checkout failure.

**5. Transfer-progress false return during paced SSH download — PASS.**
`task0_progress_abort` in `tests/ssh_fixture.rs` put a deterministic
incompressible 320,000-byte blob on remote main and paced outgoing fixture
chunks at 50 ms. It called real `Remote::download` with the exact forced
primary mapping, FETCH_HEAD disabled, tags off, prune off, a selected fixture
key and an exact host-key callback. `transfer_progress` returned false once
received bytes reached at least 16,384. The final measured run reached the
callback once at **65,931 bytes**, returned in **61 µs after false** and
**362 ms from download start**, with `GenericError` (enum ordinal 0). Assertions
proved: incomplete download; zero local refs; no FETCH_HEAD; advertised commit
absent locally; exactly one helper; zero receiver updates. No guarded tracking
write stage was entered. Runner tuple:
`1 65931 61 362 0 1 0` = callbacks, bytes, stop-to-return µs, elapsed ms, refs,
helpers, error-code ordinal. This is callback-to-return latency on this fixture,
not a universal 250-ms wall-clock cancellation bound. Existing 10-s TCP/30-s
blocking SSH limits remain; no total deadline was added.

### Characterization Commands And Honest Trial History

All used `devenv shell -- cargo`:

| Command suffix | Final result |
| --- | --- |
| `test --all-features --locked --lib task0_ -- --nocapture` | 2 passed, 504 filtered; 0.11 s |
| `test --all-features --locked --lib task0_partial_worktree_checkout -- --nocapture` | 1 passed, 504 filtered; 0.01 s; explicit absence/admin/HEAD recheck |
| `test --all-features --locked --test remote_synchronization task0_` | 1 SSH case passed; both interrupted/failed scenarios |
| `test --all-features --locked --test remote_merge_recovery task0_` | 1 SSH case passed |
| `test --all-features --locked --test ssh_fixture task0_` | 1 SSH case passed |

Five named tests executed, covering all five gate questions. Initial throwaway
compilation failed on a missing qualified `AuthoringKind` and an assumed
`WorktreeAddOptions::checkout_options` method that git2 0.20.4 does not expose.
The test was corrected to use the real filesystem checkout failure above.
The first three SSH cases passed but arbitrary stdout was filtered by the
runner, so numeric observations were added and the cases rerun. An extra
speculative assertion that callback abort returns `ErrorCode::User` failed at
line 143; investigation measured `GenericError` instead. That unsupported
assertion was removed; the actual abort/no-ref/latency assertions passed in
both the original and final run. The approved design does not require a
libgit2 `User` code, so this is not a design contradiction; later transport
code must not identify cancellation solely by that code.

One parallel Devenv invocation printed an enter-shell `load-exports` chmod
race although Cargo and its case passed. All three SSH commands were then run
sequentially; their final passing evidence had successful shell setup. No
production fix or design amendment was made for a test assumption or shell race.

All temporary tests and registrations, including the explicit worktree
recheck, were removed using patches. The post-removal
`git diff --exit-code -- src tests Cargo.toml Cargo.lock devenv.lock`
passed: the original Rust/tests/lockfiles are byte-for-byte
the Task 0 base. The final commit contains no characterization test.

### Baseline On The Original Rust Tree

| Required command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | PASS; successful retry finished in 1m 35s |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS; 1m 31s |
| `devenv shell -- cargo test --all-features --locked` | PASS; 1,568 passed, 0 failed, 1 existing ignored characterization |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS; exit 0, no window; build 1m 35s |

Test breakdown: 504 library; 775 ordinary integration; 280 real-SSH cases
(comment publication 41, merge recovery 25, observation 15, synchronization 63,
SSH fixture 31, SSH transport 105); 9 compile-fail doctests. Both binary test
targets had zero tests. The existing ignored test is the thousand-item read
characterization. Existing sync/merge/comment suites passed without edits.

The initial check reached the 120-s tool timeout during compilation, then
passed with a longer tool limit. The initial full test command compiled in
6m 23s, passed library and preceding suites, then hit the 1,200-s **tool**
timeout part-way through merge recovery; it did not report a test failure.
The complete rerun used a 3,600-s tool limit and exited 0 through doctests.
These tool-limit changes do not change runtime transport limits.

Lockfile SHA-256 (unchanged):

- `Cargo.lock`: `8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8`.
- `devenv.lock`: `d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e`.

### Checkpoint And Review Handoff

No gate contradiction or unresolved blocker was found. Task 0 is complete and
ready for independent spec review, then quality review; Tasks 1–8 were not
started. The ticket stays open and native evidence remains pending separate
authorization. The local checkpoint changes only this ledger, the ticket and
its Task 0 comment `01M4NV86AZMAVANZ2G8RJQM6FZ`. Its exact head SHA is reported
by the final handoff (`git rev-parse HEAD` after commit), avoiding a self-referential
commit identifier in its own contents.

`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` validated/rendered the open
ticket and all four canonical comments successfully. `git diff --check` passed;
status, full diff and `git log --oneline -10` were inspected before staging
only these three evidence files for the local checkpoint.

## Task 0 Independent Reviews And Owner Pause — 2026-10-11T20:01:51Z

The owner reported both independent reviews passed and instructed:
**"Pause after task 0 and before task 1"**. This checkpoint records review
bookkeeping only; Task 1 has not started and execution is paused until owner
direction to resume.

Both reviews inspected the full Task 0 range:

- Base: `094e5734d1d4480ee29e6f675aaba4a8883b9060`.
- Head: `11f7dbd6b99ee7f0a838c0faa972cb0c794c763f`.

| Review | Session ID | Scope and result reported by owner |
| --- | --- | --- |
| Independent spec | `ses_ed4470e17ffeiE6Bsc6av876RR` | Full range plus current source/ref/schema inspection; spec compliant, no blockers or findings requiring action |
| Independent quality | `ses_ed443ab4effebJy3h12aBe4kYO` | Full range plus relevant source and locked backend; hashes, counts and schema checked; **APPROVED**, zero Critical, Important or Minor issues |

Neither reviewer replayed the removed throwaway assertions or reproduced the
historical baseline. Both explicitly accepted the recorded evidence limitations.
The characterizations and baseline remain the implementer's historical Linux
evidence; review approval is not a new test run or native matrix proof. Native
evidence remains pending separate authorization.

Both Task 0 review checkboxes are ticked on the open ticket, which now states
Task 0 is completed, independently reviewed and paused before Task 1. Canonical
comment `01M4P8XCDN3ZBFEXBJ3W865M7N` records the review results and pause.
The bookkeeping base is `11f7dbd6b99ee7f0a838c0faa972cb0c794c763f`; only the
ticket, this ledger and that comment are included in the local checkpoint.
No Rust tree change or broad baseline repeat is needed for this bookkeeping.

## Task 1 Execution Resumed — 2026-10-13

The controller conveyed the owner's authorization to resume remaining execution
and assigned this fresh implementer **Task 1 only**, inline, no subagents, with
one local task commit and canonical bookkeeping, then return for independent spec
and quality review. The controller (not the owner) reported the approved Task 0
reviews and performed the resume fetch/rebase. Task 0's five characterizations
and historical baseline are trusted and were not replayed.

- All operations use the assigned ticket worktree and branch.
- Controller's fresh fetch of origin main and ticket rebase were a clean no-op:
  main `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`; before/after HEAD
  `4c1590d73b9dbab17030670989bd8825328f1f55` (also Task 1 base).
- Implementer confirmed that exact HEAD and clean branch status. No further
  fetch/rebase is required per task. D1–D4 and the approved design remain settled.
- Current AGENTS, Task 1 plan/design, ledger, ticket and latest comments read.
  The CLI has no ticket/comment commands; canonical files remain the supported
  bookkeeping path. No publication, native dispatch or lifecycle closure authority.

### Test-First Red Evidence (Before Production Behavior)

All Rust commands use Devenv, sequentially to avoid the previously recorded
shell-setup race:

| Command | Observed red result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::state` | exit 101; 42 passed, 3 failed, 463 filtered. Missing `latest_attempt_at` on migration/range queries; startup wrongly accepted a half-migrated table. |
| `devenv shell -- cargo test --locked --lib read::status` | exit 101; 9 passed, 1 failed, 498 filtered. Stored deadline expected `2026-01-01T00:00:00Z`, got `None`. |
| `devenv shell -- cargo test --locked --lib remote::poll` | exit 101; compile-red for missing eligibility types/functions/service method and snapshot accessors. Initial trial also omitted imports of existing policy types; corrected those test imports and reran: 45 missing-new-API errors, including the additional transaction-writer test. No runtime poll-test pass is claimed. |

The new tests precede production behavior. Poll module initially contains only
its test registration. Pure table tests cross all nine existing outcome
categories with automatic/explicit, including cancellation and capped retries;
transaction tests count writes, verify persisted countdown and policy precedence,
and exercise timestamp/outcome rollback. Busy/yield/interrupted/conflict/not-eligible
have no stored outcome category and must bypass the finishing writer in Task 2.

After recording compile-red, added API signatures with `todo!` bodies (no policy
behavior) so poll assertions could execute: the same `remote::poll` command
reported **0 passed, 8 failed, 509 filtered**, exit 101, at the eligibility,
next-deadline and persisted-clamp placeholders. This runtime red is recorded
before replacing those bodies with behavior. The export warning identifies the
necessary public wiring: `repository::remote` is private, so Task 1 also needs
two type re-exports in `src/repository.rs`, with no other change to that file.

### Implementation And Integration Boundary

- Fresh schemas and additive migration install both nullable integer UTC-second
  columns with checks for `0..=253402300799` (RFC 3339 through year 9999). Missing
  both upgrades to null; missing just one fails closed, transactionally. Snapshot
  and status readers validate stored times; startup audit continues to pass.
- Snapshot exposes both times as `Option<OffsetDateTime>`. The existing versioned
  status DTO's deadline field renders RFC 3339 UTC; no schema/other DTO change.
- Pure eligibility enforces Disabled → Paused → RecoverySuspended before due
  time. The service checks only the policy row inside `with_transaction`, before
  any endpoint/reservation work; its sole write is the bounded rollback clamp.
  Counted SQL writes prove one clamp, an unchanged second check, and eventual due.
- Pure `next_eligible` consumes **pre-finish** policy and returns a replacement
  deadline, or `None` to preserve it. Automatic network/protocol deadlines use
  60-second doubling capped at 900; attention failures use the interval; explicit
  failures and cancellation retain the stored deadline. Reservation finish still
  owns backoff writes. The transaction-only `record_poll_attempt` writer updates
  attempt time and optionally deadline alongside existing outcome/phase writes;
  atomicity/preservation is tested. Task 2 will wire it into finish and cancellation.
- No request/session debugging, endpoint, content, credential or path column is
  introduced. The existing schema/privacy scan now covers the polling columns.
- Two public type re-exports in `src/repository.rs` are necessary API wiring.
  No other behavior there was changed. All CLI/desktop/schema files, dependencies,
  lockfiles and synchronization/merge/comment suites are unchanged.

### Green Evidence And Lint Investigation

| Command | Result |
| --- | --- |
| `devenv shell -- cargo fmt` | PASS; formatted intended Rust files |
| `devenv shell -- cargo test --locked --lib remote::poll` | PASS; 8 passed, 0 failed, 509 filtered |
| `devenv shell -- cargo test --locked --lib remote::state` | PASS; 46 passed, 0 failed, 471 filtered |
| `devenv shell -- cargo test --locked --lib read::status` | PASS; 10 passed, 0 failed, 507 filtered |
| `devenv shell -- cargo check --all-features --locked` | PASS on final Rust tree |
| `devenv shell -- cargo fmt --check` | PASS on final Rust tree |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS on final Rust tree |
| `devenv shell -- cargo test --all-features --locked` | PASS on final Rust tree; 1,581 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS; exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 1m 18s, reached `Running target/debug/manyhands`, no startup diagnostic; stopped by the smoke-test timeout. No visual window interaction claimed. |

The first check/fmt passed, then Clippy failed on a collapsible clamp `if` and
`large_enum_variant` in existing `RefreshOutcome`: adding the inline timestamps
grew its snapshot payload to 280 bytes versus 48 for the next variant. Traced the
growth to `RemoteSnapshot`; boxed only its private polling-policy storage, keeping
the accessor and public result shape intact, and collapsed the condition. Re-ran
check/fmt/Clippy successfully, then ran the full suite on this final Rust tree.
No lint suppression or public result-enum change was used.

Full-test breakdown: **517 library** (13 new Task 1 tests), **775 ordinary
integration**, **280 real-SSH** (comment publication 41, merge recovery 25,
observation 15, synchronization 63, SSH fixture 31, SSH transport 105), **9
compile-fail doctests**. Both binary test targets have zero tests; the one ignored
test remains the thousand-item read characterization. The untouched sync/merge/
comment suites passed. This is verification of the changed Task 1 tree, not a
replay of Task 0's throwaway characterizations or its original-tree baseline.
Display inspection found `DISPLAY=:1` and `WAYLAND_DISPLAY=wayland-1`, so the
bounded desktop launch was attempted. Native matrix remains unexecuted.

### Task 1 Review Handoff — 2026-10-13T14:51:10Z

Reviewed the production and test diffs against Task 1 and the settled result
table; no Task 1 blocker found. No later task implementation was started. Skill
review handoff is to the controller's independent reviewers, with no subagent
dispatch by this implementer. The ticket remains open, Task 1 is ticked, both
Task 1 review gates are pending. Canonical comment `01M4TVXXYRWN0MSPHSM83A0F8E`
records the resume, red/green evidence and review readiness.

Task 1 base: `4c1590d73b9dbab17030670989bd8825328f1f55`; the local commit head
and exact base-to-head range are reported after commit in the implementer handoff,
avoiding a self-referential SHA in committed content. Final status/diff/recent-log
inspection, `git diff --check` and `scripts/show-ticket` validation precede staging
only Task 1 source and canonical bookkeeping. No push, PR, native dispatch, merge,
closure or worktree cleanup is performed.

## Task 2 Execution — 2026-10-13

Assigned Task 2 only, inline with no subagents; approved design/plan and D1–D4
remain settled. The controller already refreshed/rebased at resume (no-op main
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`); no repeated fetch/rebase.
Verified clean assigned worktree/branch and Task 2 base
`5ff9f4eee31cd88067a406ba8acbc317c0ee2045`. All Rust uses Devenv.

### Prior Task 1 Independent Review Evidence

Controller reports full Task 1 range
`4c1590d73b9dbab17030670989bd8825328f1f55..5ff9f4eee31cd88067a406ba8acbc317c0ee2045`:

- Spec `ses_eca3c077effeAOiqWMn8w5xeR5`: compliant.
- Quality `ses_eca397d08ffeZI2d2YhlbUJ99h`: approved; minor missing direct
  corrupt-timestamp read coverage carried into this task.

Focused hardening `devenv shell -- cargo test --locked --test remote_reservation
corrupt_poll_times`: 1 passed, 9 filtered. Both columns independently tested
with CHECK enforcement bypassed: negative, year-10000, text, real values;
snapshot and reopen return RecoveryRequired; deadline gate returns
RecoveryRequired and status returns the existing InternalError mapping.
This is already-green hardening, not new-behavior RED. Initial trial incorrectly
expected the deadline-only gate/status projection to read attempt time; it
returned Eligible. Source confirms the projection consumes only next_eligible_at;
corrected the test scope, with no production change.

### New-Behavior RED Before Implementation

- `devenv shell -- cargo test --locked --lib remote::reservation`: compile-red,
  10 missing new seam/variant errors. Refined ConflictPending into an internal
  full-poll reservation wrapper, keeping public observation/sync enums unchanged.
  Added behavior-free seam delegates to existing observe paths so assertions run.
  Rerun: exit 101, **30 passed, 7 failed, 487 filtered**. Missing poll_kind migration,
  D3 gate, kind-specific safe-point ordering/finish/restart and timestamps.
- `devenv shell -- cargo test --locked --test remote_reservation`: exit 101,
  **10 passed, 1 failed**: finish did not record latest_attempt_at.
- `devenv shell -- cargo test --locked --test local_authoring`: initial test typo
  used nonexistent comment_id helper; corrected to existing root_comment_id.
  Rerun exit 101, **131 passed, 1 failed**: automatic poll guard returned
  RecoveryRequired instead of RepositoryBusy. No production guard change yet.

New tests use real local Git repositories; no transport claim, fake transport,
sleep, global Git configuration, developer key, network or prompt is introduced.
Implementation begins only after this recorded RED checkpoint.

### Additional Policy-Boundary RED

After the initial local-stage implementation, strengthened the policy tests to
seed backoff/deadline **before** the advertisement batch rather than afterwards.
`devenv shell -- cargo test --locked --lib remote::reservation` then exited 101:
**35 passed, 2 failed, 487 filtered**. Existing observe `complete_batch` clears
backoff and writes Completed immediately; an update batch must retain policy
until the local stage finishes. This exposes a missing update-specific batch
fork, not a reason to change observation or synchronization behavior. The
following fix uses the existing policy-independent advertisement writer for
update batches, retaining existing observe complete_batch and sync handling.

### Implemented Boundary

- Probe-and-add nullable `poll_kind` with only observe/update allowed on poll
  rows and NULL (empty) on non-poll rows; migrate old poll rows to observe.
  Legacy NULL poll rows decode as observe. A separate immutable-kind trigger
  avoids rebuilding either CHECK list or changing the existing target trigger.
- Internal `reserve_remote_poll` and `RemotePollReservationOutcome` distinguish
  D3 ConflictPending from ordinary reservation outcomes. Existing public
  observation/synchronization reservation enums remain intact. Same-ID kind
  mismatch is OperationMismatch, including terminal replay and restart.
- Update-only safe-point fork permits repeatable BeforeLocalMutation after the
  committed batch, once-only BeforeDiscovery, both under local_prepared; the
  existing active index and journal guard already include that phase. Completed
  requires BeforeDiscovery. Observe remains AfterBatchCommit-complete and rejects
  BeforeDiscovery; its old BeforeLocalMutation/restart restrictions remain.
- Internal restart_remote_poll resets interrupted/abandoned update records at
  every stage to reserved, clears step/yield, increments owner epoch and fences
  the original service's old token. Terminal completed/cancelled records replay.
- Update batches persist the complete advertisement without writing a poll
  outcome/backoff. Finish consumes the pre-finish policy, then atomically writes
  outcome, backoff, attempt time, deadline and phase. Automatic network/protocol
  failures double the previous backoff; attention uses interval; explicit failure
  and cancellation preserve policy; Completed clears backoff and uses interval.
  Cancel acknowledgement writes attempt time with no deadline/backoff change.
- D3 rejects new update reservations before insertion, with no policy write;
  observe remains permitted. D4 uses the existing journal guard's sole active
  phase predicate: automatic poll requests yield, commits that request and
  returns RepositoryBusy before journal insertion; manual remains RecoveryRequired.
  Both kinds and every existing synchronization branch retain their contracts.

Additional already-green coverage checks the inclusive UTC endpoints and NULL,
the recovery journal guard directly at update local_prepared (save/context/comment
× both priorities), cancellation-writer rollback, actual AfterBatchCommit request
acknowledgement as well as the pre-batch transaction recheck. The guard tests
leave zero new local rows and retry successfully after acknowledgement. This
hardening does not claim a second new-behavior RED cycle.

### GREEN, Gate Failures And Final Verification — 2026-10-13T16:19:29Z

All commands below ran in the assigned worktree through Devenv, sequentially:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib remote::reservation` | Final focused PASS: 39 passed, 487 filtered; final full gate also passes all 39 |
| `devenv shell -- cargo test --locked --test remote_reservation` | PASS: 12 passed |
| `devenv shell -- cargo test --locked --test local_authoring` | PASS: 132 passed |
| `devenv shell -- cargo test --locked --lib remote::` | PASS: 299 passed, 225 filtered before the final batch-policy fork/hardening; final full gate covers the resulting 301 matching tests |
| `devenv shell -- cargo test --locked --test remote_observation` | PASS: 15 real-SSH cases, suite untouched |
| `devenv shell -- cargo test --locked --test read_status` | PASS: 23 passed after the deadline expectation correction |
| `devenv shell -- cargo check --all-features --locked` | PASS on final Rust tree |
| `devenv shell -- cargo fmt --check` | PASS on final Rust tree |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS on final Rust tree |
| `devenv shell -- cargo test --all-features --locked` | PASS: 1,594 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS: exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 14.66 s, reached Running; bounded stop after 90 s, no startup diagnostic; no visual interaction claimed |

Full gate breakdown: **526 library** (+9 Task 2 tests), **779 ordinary
integration** (+3 reservation and +1 authoring), **280 real-SSH** (comment
publication 41, merge recovery 25, observation 15, synchronization 63, fixture
31, transport 105), **9 compile-fail doctests**. Both binaries have zero tests;
the existing thousand-item read characterization remains ignored. The final
full log is `tool_135f2723a001mSAMD0YzfOXEpI` in the harness output directory.

Honest trial history after first implementation:

- Reservation tests initially had 35 pass/2 failures because observation-restart
  equality assertions assumed no timestamps. Updated those assertions to prove
  all observation/context/policy/publication evidence stays identical while
  finish adds exactly attempt+interval. A trial used a nonexistent history_unknown
  accessor; corrected to the existing absent-item publication evidence accessor.
- The first integration run had 9 pass/2 failures: the old lifecycle guard test
  expected RecoveryRequired instead of approved D4 RepositoryBusy; updated its
  expected result and asserted durable yield. The generic restart refactor had
  also inadvertently changed non-poll restart errors to OperationMismatch;
  restored the original RecoveryRequired rule before the passing rerun.
- Direct journal test initially used unwrap_err on opaque RecoveryRecord, which
  has no Debug; used err/expect instead, without adding Debug to production.
- The first required full gate passed library/preceding targets (including 41
  comment SSH cases), then failed one tests/read_status assertion expecting None
  after an automatic network failure (22 pass/1 fail). The explicitly required
  finish timestamp now gives a deadline. **Minimal extra test-only file** beyond
  the Task 2 list: tests/read_status.rs now asserts attempt+60 in the existing DTO
  and that inserting an observation retains it. No status production/schema
  change. Focused status passed; final check/fmt/Clippy and full gate passed.
  This concrete failure justified the second full gate; no third broad repeat.

### Local Review Handoff

Task 2 implementation is ready for the controller's independent spec then quality
review. Self-reviewed the base-to-working-tree production/test diff against Task 2
and the settled result table. The Task 1 minor coverage finding is resolved.
No material design contradiction, unresolved local failure or later-task work.
Native matrix evidence and Task 2 independent approval remain open gates.

Task 2 base is `5ff9f4eee31cd88067a406ba8acbc317c0ee2045`; its exact head and
base-to-head range are returned after the single local task commit (a commit
cannot contain its own SHA). This entry and comments bind that range to the
`feat: give poll reservations a local stage+bookkeeping` checkpoint. Canonical
RED comment `01M4TX8PRJBBYH50CQ3GT5VYTP` and review-ready comment
`01M4V0ZN2MHG18CD1BFC2CYDWX` retain prior review and current verification evidence.
Ticket remains open; Task 2 is ticked with its two independent review gates open.
Final status/full diff/recent log, diff whitespace check and scripts/show-ticket
validation precede staging only intended Task 2 source/tests/bookkeeping.

`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` successfully validated/rendered
the open ticket and all eight canonical comments, including both Node-generated
Task 2 ULIDs. `git diff --check` passed; status, production/test/bookkeeping
diffs and recent `git log --oneline -10` were inspected. Only the seven intended
Rust/test files, this ledger, the open ticket and its two comments are staged.

## Task 3 Execution — 2026-10-13

Fresh implementer assigned Task 3 only, inline, no subagents, on the assigned
ticket worktree/branch. Confirmed clean HEAD at Task 3 base
`6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`. Controller's resume refresh/rebase
remains authoritative; no new fetch/rebase. Approved design and D1–D4 retained.

### Prior Task 2 Reviews

Controller reports range
`5ff9f4eee31cd88067a406ba8acbc317c0ee2045..6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`:

- Spec `ses_ec9ebcd99ffekVz7ijwoL63NJx`: compliant.
- Quality `ses_ec9e9909cffeS4kt0Z0ENmyjss`: approved; minor suggested direct-SQL
  poll-kind immutability regression remains explicitly deferred to final review.
- Task 1 minor direct corrupt timestamp read coverage was resolved by Task 2.
  Existing green evidence is not a new-behavior RED claim.

### Test-First RED — 2026-10-13T16:37:08Z

Before production behavior, sequential Devenv commands:

- `devenv shell -- cargo test --locked --lib transport`: compile-red for missing
  callback API and private combined scope; behavior-free signatures/placeholders
  then gave runtime RED: **28 passed, 2 failed, 498 filtered** at mapping/stop
  placeholders.
- `devenv shell -- cargo test --locked --test ssh_transport poll_`: compile-red
  for missing fetch API/private scope (also corrected missing existing
  PassphraseResponse import). With behavior-free fetch placeholder, runtime RED
  exits 101 on the first real-SSH case, `poll_fetch_set`. Harness stops at first
  failing child; individual filters `poll_absent_primary_and_empty_set`,
  `poll_tracking_race`, `poll_noncommit_tip` each exit 101 at the same missing
  fetch behavior. `poll_progress_stop` also exited 101, but later investigation
  identified a pre-transport setup failure; corrected behavioral RED is below.
  No transport assertions passed yet.

Tests register seven real in-process SSH cases, including policy compatibility,
and two unit tests. No new cfg(test) checkpoint hook or fake transport. The
source-inclusion policy driver constructs a frozen expectation from real public
registry/configuration state; races use the caller callback and public lease
probe. New behavior begins only after this checkpoint.

### Stop Fixture Correction And Behavioral RED

First GREEN trial: library 30 passed; SSH filter passed fetch-set, absent/empty,
tracking race and noncommit, then stop case failed before transport. Investigation
found its fixture helper tried a parentless `commit(Some(main))` over existing
main; libgit2 refuses that inconsistent parent update. Corrected the test helper
to create the object without an update, then set the fixture ref. The earlier
stop RED was a setup failure and is **not** stop-behavior evidence. Removed the
progress registration before continuing; corrected
`devenv shell -- cargo test --locked --test ssh_transport poll_progress_stop`
exited 101 at poll.rs line 307 (`unwrap_err` after a successful full download).
This is behavioral RED for the missing registration; registration is implemented
only after this corrected checkpoint. Unit stop-intent RED remains valid.

Additional exact-set scope test preceded the extracted predicate:
`devenv shell -- cargo test --locked --lib transport` exited 101 with
**30 passed, 1 failed, 498 filtered**, at its behavior-free scope placeholder.
It requires the complete advertised parsed set and rejects duplicates, omitted
members, wildcards, malformed/unadvertised sources, local destinations and Push.
Two added SSH hardening cases (download-connection advertisement, partial guarded
write failure) and stronger excluded graph/key-ID assertions were already green;
they are not presented as new-behavior RED.

### Implemented Task 3 Boundary

- `fetch_poll_set` uses the shared `TransferAction::FetchTracking` path. Its
  renewed selected-key connection lists the advertisement, derives explicit
  forced mappings for the advertised primary and parsed document/ticket names,
  and captures the old destinations before download. Malformed names remain
  in the returned advertisement but are not downloaded; other families are
  ignored. The original download-connection advertisement is returned.
- Empty parsed sets return before `download`; no configured/default refspec
  fallback. FETCH_HEAD updates, tag following and pruning are disabled. D1
  retains previously fetched tracking refs even when sources disappear.
- Under the existing short repository lease, the complete parsed mapping set,
  configuration, endpoint, selected key and host policy are rechecked; every
  fetched tip must be a commit and every old destination must still match
  before the first write. Existing guarded `reference_matching`/nonforcing
  creation performs the writes. A later write failure returns failure while
  retaining the valid tracking observation already written, never completion.
- Progress registration belongs only to the poll action. Its explicit sticky
  closure-stop flag prevents writes after false independently of libgit2's
  GenericError/User code; result stays the existing redacted TransportUnavailable
  category so the later orchestrator must consume its caller-owned stop flag.
  Transport neither knows reservations nor throttles queries. Task 6 must supply
  the short read-only flags query, <=250-ms caller throttle and acknowledgement.
- The existing combined automatic+frozen-expectation scope is visible to the
  repository's remote module; its policy behavior is unchanged. Automatic blocked
  unlock and endpoint/key/source mismatch cases are compatibility evidence,
  not a new policy-behavior RED claim.
- Source-inclusion fixture driver adds no checkpoint hook. Callback races use
  real local refs or fixture ref locks; stop uses paced real SSH and the public
  repository lease probe. Privacy scans run after transport/leases have ended.
  No frontend, dependency, schema or later-task implementation changed.

### Acceptance Evidence

| Task 3 requirement | Passing real-SSH case |
| --- | --- |
| Primary + document + ticket exact tips; malformed/outside commit/tree/blob objects absent; tags, FETCH_HEAD, unrelated/local refs preserved; tracking-only ancestry rewind | `poll_fetch_set` |
| Absent primary leaves its tracking unchanged; empty parsed set makes no download callback, downloads no outside graph, retains absent context tracking (D1) | `poll_absent_primary_and_empty_set` |
| Existing or initially absent destination changed after capture: fails, preserves changed destination and primary still old; all checked before first write | `poll_tracking_race` |
| Downloaded blob tip is present but rejected before any tracking write | `poll_noncommit_tip` |
| Paced stop exits without refs/FETCH_HEAD/commit, precisely two helpers (scope+download), no further connection; callback can acquire public lease | `poll_progress_stop` |
| Frozen endpoint, missing/replaced key selection, changed key source: zero helpers/prompts before refusal | `poll_frozen_policy` |
| Blocked automatic unlock: no second prompt, zero Git helpers | `poll_automatic_blocked_unlock` |
| New context added after earlier scope advertisement: returned advertisement and tracking belong to download connection | `poll_download_advertisement` |
| Second tracking ref lock fails guarded write; first valid tracking tip retained, context/local ref preserved, no completed claim | `poll_partial_guarded_write` |

Every case (and every loop iteration) waits for real helper audits, asserts zero
receive-pack commands/advertisements and zero receiver updates, then scans the
registry for password/endpoint probes and registers them with the child-output
privacy harness. All nine cases use the real in-process SSH fixture. Three new
library tests prove parsed mapping selection, complete exact-set validation and
sticky stop intent; library tests make no SSH transfer claim.

### GREEN And Full Verification — 2026-10-13T17:21:49Z

All Rust commands ran sequentially through Devenv in the assigned worktree:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo fmt` | PASS |
| `devenv shell -- cargo test --locked --lib transport` | Final focused PASS: 31 passed, 498 filtered |
| `devenv shell -- cargo test --locked --test ssh_transport poll_` | PASS: 9 real-SSH cases after fixture correction/hardening; final entire target/full gate also cover all nine after exact-set predicate wiring |
| `devenv shell -- cargo test --locked --test ssh_transport` | PASS on final Rust tree: 114 real-SSH cases, including all 105 untouched existing cases |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `devenv shell -- cargo test --all-features --locked` | PASS: 1,606 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS: exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 8.03 s, reached Running, no startup diagnostic; bounded timeout stop, no visual interaction claimed |

Full gate breakdown: **529 library** (+3 Task 3), **779 ordinary integration**,
**289 real-SSH** (comment publication 41, merge recovery 25, observation 15,
synchronization 63, fixture 31, transport 114), **9 compile-fail doctests**.
Both binary test targets have zero tests; the existing thousand-item read
characterization remains ignored. Full log: `tool_1362dc32a001ZRHM1I40DZfKCY`.
Existing synchronization, observation, merge and comment cases passed unchanged.
No broad gate failure or duplicate broad rerun. Initial compile trials, corrected
fixture failure and corrected behavioral RED are recorded above, not concealed.

Unchanged lock hashes:

- Cargo.lock: `8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8`.
- devenv.lock: `d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e`.

### Review Handoff And Deferred Finding

Self-reviewed the complete Task 3 base-to-working-tree diff against the approved
Task 3 plan and transport constraints. Task 3 is ready for the controller's
independent spec then quality review, with no implementer subagent dispatch.
Base: `6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`; exact local commit head/range
will be returned after committing `feat: fetch the poll ref set with guarded
tracking refs+bookkeeping`, avoiding a self-referential SHA in committed content.

**Deferred minor still open:** Task 2 quality review
`ses_ec9e9909cffeS4kt0Z0ENmyjss` suggested a direct-SQL kind-immutability
regression. Carry explicitly into the final Cycle review ledger; Task 3 did not
add it or change poll-kind production behavior. Task 1 corrupt-time minor remains
resolved. Native matrix and independent Task 3 review are remaining gates.
Linux paced stop evidence is not a universal callback-latency guarantee; existing
10-s TCP/30-s blocking-SSH limits remain, without a total deadline. D2 Linux
partial-worktree characterization remains authoritative and untouched.

Canonical comments `01M4V1ZZM9SAQHCJFT8DFA7SHJ` (RED/prior reviews) and
`01M4V4HS2G64D9TR1P71N6CQB8` (review-ready/corrected RED/GREEN) record checkpoints.
Ticket remains open; Task 4 was not started. Final status/full diff/recent-log,
`git diff --check` and `scripts/show-ticket` validation precede staging only the
seven owned Rust/test files, this ledger, the ticket and its two new comments.

`scripts/show-ticket 01K7F6H9J2N4Q6S8V0X2Z4B6DG` successfully validated/rendered
the open ticket and all ten canonical comments, including both new Node ULIDs.
Final diff/recent log were inspected and `git diff --check` passed.

## Task 3 Independent Reviews And Atomic-Absence Correction — 2026-10-13

Controller reports review of Task 3 range
`6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a..dc96f0b71fad8bba911f349bbf53140864b81d1b`:

- Spec `ses_ec9b35ee6ffeBdb0HkkfCCp9R4`: passed.
- Quality `ses_ec9b0a1d9ffeWP162PBxjUK6l4`: **blocked, Important**. Initially
  absent tracking destinations use `reference(..., false, ...)`, which has a
  check-to-lock race against external Git. Fix before Task 4; new local commit,
  no amendment. No guard-contract or design amendment is needed.

Resumed inside the same assigned worktree/branch, clean fix base
`dc96f0b71fad8bba911f349bbf53140864b81d1b`; no fresh fetch/rebase. Loaded receiving
code review and applied the existing systematic-debugging/TDD instructions.

### Locked Backend Investigation

Verified the actual Cargo registry source, `git2 0.20.4` and
`libgit2-sys 0.18.8+1.9.7`, rather than assuming force=false is atomic:

- git2 `repo.rs:1509–1517` calls `git_reference_create`; libgit2 `refs.c:487–495`
  delegates with NULL `old_id`. `refdb_fs.c:1112–1125` checks existence before
  `refdb_fs_backend__write` takes `loose_lock` at 1613. `cmp_old_ref:1521–1523`
  skips comparison for NULL. A Git creator after that check can be replaced.
- `reference_matching` passes a non-NULL expected OID through
  `git_reference_create_matching` unchanged (`repo.rs:1564–1571`,
  `refs.c:480–481`). The filesystem backend takes the ref lock before
  `write_tail:1634` calls `cmp_old_ref`. That comparator accepts lookup NotFound
  only when the supplied expected OID is zero (`1525–1527`); existing direct
  values or symbolic types differ and return Modified without committing.
- The force bit must be true to bypass the non-atomic preliminary existence
  check, **not** to bypass the expected-value comparison under the ref lock.
  The existing Some(old) path already uses this bit with a nonzero expectation.
  A zero expectation therefore represents atomic absence for valid Git refs.

### Focused RED Before Guard Behavior — 2026-10-13T17:37:20Z

Extracted the existing match unchanged into `write_tracking_tip`; caller still
validates every commit and destination before any write. Two real local-ref
primitive tests use the public git2 Transaction lock and external Git with system
and global configuration excluded, no sleeps, test hooks or simulated transport.

`devenv shell -- cargo test --locked --lib transport_absent_tracking_guard`
exited 101: **0 passed, 2 failed, 529 filtered**. The lock test captures absence,
creates a competing ref using external `git update-ref`, holds its public ref
lock, then invokes the unchanged primitive. Expected Locked, got Exists. The
other test expected under-lock Modified after competing creation, got Exists.
These errors distinguish the best-effort pre-lock check from the required locked
CAS path. The test does not claim to pause inside C between instructions; the
locked source analysis plus deterministic lock/compare-path assertions prove the
chosen atomic primitive. Competing OID/reflog preservation, release/retry, real
absence success and direct/same-OID/packed/symbolic variants are also asserted.
The correction starts only after this recorded RED.

### Atomic-Absence Fix And GREEN — 2026-10-13T18:13:46Z

The shared local writer now always uses `reference_matching`, retaining the
captured OID for an existing destination and supplying `Oid::zero()` for captured
absence. `force=true` skips only the preliminary best-effort existence check;
the locked backend still compares the supplied expectation under its ref lock.
No unguarded write is introduced. The all-tips commit/old-value validation loop
stays before the first write; later guarded failures still retain valid earlier
tracking observations and return ProtocolFailure. Existing Some(old) behavior
and all mapping/observation/stop code are unchanged.

Sequential Devenv evidence:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo fmt` | PASS |
| `devenv shell -- cargo test --locked --lib transport_absent_tracking_guard` | GREEN: 2 passed, 529 filtered (after RED 0 passed/2 failed) |
| `devenv shell -- cargo test --locked --lib transport` | PASS: 33 passed, 498 filtered |
| `devenv shell -- cargo test --locked --test ssh_transport poll_` | PASS: 9 real-SSH cases, including initial-absence race, validate-all-before-first-write and permitted partial failure |
| `devenv shell -- cargo test --locked --test ssh_transport exact_` | PASS: 7 real-SSH cases, shared-writer compatibility |
| `devenv shell -- cargo check --all-features --locked` | PASS |
| `devenv shell -- cargo fmt --check` | PASS |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS after test-only range-pattern correction |
| `devenv shell -- cargo test --all-features --locked` | PASS: 1,608 passed, 0 failed, 1 existing ignored |
| `devenv shell -- cargo run --locked --bin manyhands-cli` | PASS: exit 0, no window |
| `timeout --signal=TERM 90s devenv shell -- cargo run --locked --features desktop --bin manyhands` | Built in 6.48 s, reached Running, no startup diagnostic, bounded timeout stop; no visual interaction claim |

The first Clippy gate flagged the new test's `0 | 1 | 2` pattern; changed only
that pattern to `0..=2`, then check/fmt/Clippy passed and the full gate ran once.
No production trial failure after the recorded RED. No duplicate broad test run.
Full log: `tool_1365d606d001URFdZV4VOOZeUU`. Counts: **531 library** (+2 fix
regressions), **779 ordinary integration**, **289 real-SSH** (41 comment, 25
merge, 15 observation, 63 synchronization, 31 fixture, 114 transport), **9
compile-fail doctests**. Both binary targets have zero tests; the existing
thousand-item characterization remains ignored. All existing SSH cases passed
without edits. Real transport assertions remain in real-SSH targets; new unit
tests claim only local reference primitive/lock behavior and backend semantics.

Lock hashes still match the previous checkpoint: Cargo.lock
`8a20e8ba089f63bddfa9b87a08d5d4308404b4c2440c050453654e422a9549f8`, devenv.lock
`d4a78b8259525e7dcf66814477b0f96282e6570c9a21ebc84eda7d7fb8950d6e`.

Self-reviewed the base-to-fix diff against the Important finding and source
semantics. The blocker is technically corrected with RED/GREEN evidence;
independent quality re-review remains pending and is not implied by this entry.
New local commit `fix: guard absent tracking refs atomically` from fix base
`dc96f0b71fad8bba911f349bbf53140864b81d1b`; its exact head/range is returned after
commit. Full Task 3 re-review starts at `6cb69f4b6dd4f9798e509f9cdddc1acc7cd4f39a`.
Only remote.rs and canonical ledger/ticket/two fix comments change.
Comments `01M4V5E6NFNSBJQM0FDDG8DVRK` and `01M4V7GXPAC24SSPC0WXW6BAEF` record
review, investigation, RED and GREEN. Task 2's direct-SQL kind-immutability minor
remains explicitly deferred; Task 1's corrupt-time minor remains resolved.
Native matrix remains pending. Ticket stays open and Task 4 remains unstarted.

Final `git diff --check` passed; status/full diff/recent log inspected before
staging only those five intended files. `scripts/show-ticket
01K7F6H9J2N4Q6S8V0X2Z4B6DG` successfully validated/rendered the open ticket and
all twelve canonical comments, including both Node-generated fix ULIDs.
