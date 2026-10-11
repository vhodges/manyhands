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
