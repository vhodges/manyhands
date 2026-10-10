---
title: "Wave 02 Cycle 07 Comment Publication Execution Ledger"
date: 2026-10-07
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4B2JBSQQTM8011VSM1XJCT1"
---

# Comment Publication Execution Ledger

## Planning Authorization And Preflight — 2026-10-07

User requested the Wave 02 Cycle 07 Cycle, design, detailed implementation plan
and skill-based self-review for existing ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DF`.
At the initial request, authorization was planning only, pending user review.
No implementation, delegation, publication, merge, closure or cleanup was
authorized/performed. The later document approval and local planning-commit
authorization are recorded below.

- Repository: `/home/vhodges/work/src/manyhands`.
- Verified registered ticket worktree:
  `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DF`.
- Branch: `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DF`.
- Ticket worktree was clean before rebase. Main had unrelated untracked
  `.superpowers/` and `devenv.nix~`; left untouched. Other worktrees untouched.
  At final inspection main also showed modified `scripts/ticket-front-matter`
  and `tests/interim-ticket-scripts.sh`; this planning session did not edit
  either file and left those unrelated changes untouched.
- Fresh `git fetch origin main` succeeded; local main and fetched origin/main
  agreed at `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`.
- Before ticket HEAD: `92a3646ed1b5915713e68fdf2158e4a5ca49f7c2`.
- Ticket rebase onto fetched base succeeded without conflicts.
- After ticket HEAD: `c98127ed1203ff77c9a4a3658741fba256bdce1f`.
- `merge-base --is-ancestor` passed; post-rebase ticket status was clean.
- Re-read rebased `AGENTS.md`. Starting-cycle preflight makes the rebase
  direction unambiguous: ticket onto main; main was never rebased/switched.
- CLI inspection found startup initialization only, no supported ticket/comment
  operations. Canonical filesystem updates are therefore the approved fallback.

## Grounding

Read `starting-a-cycle` and its preflight reference, `review-cycle-docs` and its
review matrix, repository instructions and ticket; inspected Wave 02, PRD/MVP
and relevant schema/Git/index/authentication/test RFC sections, Wave 01 local
comment/recovery contracts, Cycle 05 artifacts/evidence, Wave 03 API audit,
local comment/checkpoint/recovery source, remote synchronization/reservation
source, SSH fixture/test architecture, Cargo test declarations and native CI.

Main contains Cycles 01–05; normal comment submit still reports `SyncDeferred`.
Cycle 06 documents/merge-conflict code are absent from this base. Its approved
merged contract and actual evidence are required at implementation Task 0.
No unmerged worktree's code/documents were copied or presumed approved.

## Artifacts

- [Cycle](../Cycles/wave-02-cycle-07-comment-publication.md)
- [Design](2026-10-07-wave-02-cycle-07-comment-publication-design.md)
- [Detailed implementation plan](2026-10-07-wave-02-cycle-07-comment-publication-implementation.md)

The user approved all three on 2026-10-07. Ticket remains `open`; implementation
is not authorized. Proposed execution: direct sequential tasks with checkpoint
comments and code review; do not infer delegation approval.

## Skill-Based Self-Review

| Source | Concern / promised behavior | Classification | Resolution / ruling | Proof or follow-up |
| --- | --- | --- | --- | --- |
| Wave 02 Cycle 07; Git RFC | Scope could absorb missing merge/conflict implementation | Settled | Delegate Cycles 05/06; Cycle 06 absent is an implementation gate, not reduced acceptance or new merge scope | Task 0 approved-source/evidence reconciliation; Task 5 real merge/conflict integration |
| Existing submit API; Wave 03 API audit | A second normal submit could leave a silent `SyncDeferred` bypass | Ruling | Ruling: change public normal submit to compound and extract a private local helper — required headless policy — cost if wrong: in-repository API/test migration; revisit newly merged consumers at Task 0 | Task 1 consumer scan, local regressions and audit update |
| Reservation `validate_index_identity`; local recovery | Reusing submit ID as sync ID violates action identity | Ruling | Ruling: persist one distinct child before any canonical write — preserve existing cross-action guards — cost if wrong: additive binding migration and receipt tests | Task 2 transaction/concurrency/collision tests; never loosen reservation identity |
| Local authoring/recovery RFC | Body-free retry and commit-before-receipt crash could duplicate a checkpoint | Settled | Actual Git tree/diff/ancestry and immutable ID/time prove original receipt; no body/hash persistence; ambiguity stops safely | Task 2 fault/reopen and negative proof tests; Task 4 receiptless retry |
| Publication receipt vs branch sync authority | External branch move/removal or conflict resolution could sync a tree excluding the original comment | Ruling | Ruling: prove checkpoint ancestry and original blob at delegated authority OID — branch success alone is insufficient — cost if wrong: conservative recovery instead of false publication | Self-review correction propagated to all three artifacts; Tasks 3 and 5 exclusion/replacement negative cases |
| Index RFC; `require_no_pending_local` | Immediate sync could bypass pending local recovery or cache failure could be mistaken for unsaved work | Ruling | Ruling: immediately attempt local discovery, repair pending handoff before child, return saved/index-pending if blocked — preserve existing recovery ordering — cost if wrong: delayed publication until repair | Tasks 3–4 ordering, independent local/remote index-failure tests |
| MVP local-only amendment; Cycle 05 local-only terminal replay | Freezing a no-remote child would prevent later configured-remote publication | Ruling | Ruling: return local pending without starting child — preserve same-receipt later publication — cost if wrong: one binding lookup rather than frozen local-sync replay | Task 4 remote-added-after-submit case; no prompt/network/reservation assertions |
| Scoped checkpoint RFC; desktop discussion contract | Whole context sync could accidentally checkpoint buffers/unrelated index or hide other committed publication | Settled | Only comment path checkpoints; whole already-committed context can publish; unrelated dirty state blocks unsafe sync after receipt | Tasks 1/5 live-index/status snapshots, prior-commit vs unsaved tests |
| Authentication RFC; approved transport limits | New composition might bypass selected key/trust, leak body/URL/secret or imply a total timeout | Settled | Existing caller-owned session/HostApproval and fixed typed errors; per-address/per-call budgets and safe points, no automatic retry/total deadline | Tasks 2/5 persistence/output privacy scans and existing transport timeout tests |
| Cycle 05/06 push and conflict recovery | Restart/cancel/ambiguous push might invent consent or duplicate merge/push/checkpoint | Settled | Delegate explicit fencing/re-observation/resolution; parent binding never grants ownership or republish consent | Tasks 4–5 cancelled, conflict, accepted-push-disconnect and terminal index-only replay |
| Migration/rebuild authority | Old rows or lost correlation might be auto-posted on reopen | Settled | Additive migration, no auto-publication/guessed adoption; missing binding is recovery; ordinary deliberate sync available for legacy comments | Task 2 old-database and corrupt/missing binding tests |
| Test RFC; Cargo/harness/CI | A bare remote, fixture wiring or Linux result could be called native SSH proof | Settled | Custom pre-thread real SSH/private-output target; exact-ref/effect snapshots; manual workflow remains disabled automatically | Task 5 real two-clone target; Task 6 native results pending authorized runs or explicit owner deferral |
| Dogfooding lifecycle | Planning approval might imply implementation, push or premature closure | Settled | Planning only; open ticket; code-review/PR approval before closure, normally final authorized pre-merge checkpoint; separate publication/merge/cleanup permission | Task checkpoints, final gate, review-ready and later delivery comments |

No unresolved product/security/recovery/scope question was identified after the
matrix audit. Approved authorities settle local-first behavior, trust, shared
branch/whole-context publication, conflict preservation and privacy. The rulings
are implementation mechanisms within that policy. Dependency availability and
native evidence remain explicitly open **gates**, not hidden product decisions.

### Corrections Made During Self-Review

The initial draft required verified context synchronization but did not explicitly
prove the original comment was still in that published tree. Added original
checkpoint ancestry and canonical blob proof at the delegated authority OID,
plus negative tests for external branch movement/removal/replacement, to Cycle,
design and Tasks 3/5. Rechecked the three artifacts for consistent receipt,
index-pending, no-remote-child, explicit restart and approval semantics.

## Verification And Approval Checkpoints

Planning verification at `2026-10-07T11:49:26Z`: six then-written
artifacts/ticket/comments and 25 relative links checked; zero missing links,
invalid/duplicate ULIDs or unresolved drafting placeholders. `git diff --check`
passed for tracked changes; a separate scan of all new/modified artifact lines
found no trailing whitespace. Cross-artifact decision/acceptance/task consistency
was rechecked after the self-review correction. Final review comment is added
at the review-ready checkpoint and included in the final documentation recheck.

These are documentation checks only. No Cargo command or platform test has been
run for this planning change; baseline/final Rust verification remains in
Tasks 0/6. No source, Cargo dependency, lockfile or workflow was changed during
planning. Drafts were uncommitted at the review handoff; the user subsequently
authorized approval-state updates and a local planning commit.

Implementation checkpoints: Task 0 base/dependency/baseline; Task 1 API/local
regression; Task 2 durable identity/receipt; Task 3 compound ordering/outcomes;
Task 4 body-free retry/index/cancel; Task 5 real SSH/privacy/recovery/native
wiring; Task 6 full Devenv gate/code review/review-ready. Each gets a ticket
comment with results and blockers. Ticket closure is not a Task 6 automatic
side effect and precedes merge only after review and explicit authorization.

## Document Approval And Commit Authorization — 2026-10-07T14:38:23Z

User decision: "The documents are approved, update the state and commit".

- Marked the Cycle, design and detailed implementation plan approved; updated
  this ledger's planning state and the ticket's approval checklist.
- Recorded approval in a canonical ticket comment using the filesystem fallback;
  the CLI still does not support the required operation on this ticket's base.
- Authorized scope is approval-state updates and one local planning commit on
  `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DF`.
- Ticket stays open. Implementation, push/PR, native CI dispatch, merge, closure
  and cleanup remain unauthorized. Cycle 06 and baseline gates are unchanged.
- Documentation-only checkpoint: recheck links, canonical IDs, state consistency
  and staged `git diff --check` before committing. No Rust source, dependency,
  lockfile or workflow changes; no Rust/native tests claimed.

## Rebase Onto Merged Cycle 06 And Plan Amendment — 2026-10-09T22:29:58Z

Owner request: rebase this ticket onto current main and review the approved plan
against the merged Cycle 06; then, "lets make the ammendments and push the branch
to origin".

- Fresh fetch observed origin/main at `5e4fad636fd7aad332b9221cf157a7558c32e375`
  (merge of pull request 14, Wave 02 Cycle 06).
- Before ticket HEAD `56ed280`; rebased without conflicts to `1de99c6`. The
  diff against main was only this Cycle's planning files. A local backup ref
  `backup/cycle07-pre-rebase-56ed280` was kept.
- The origin ticket branch still held only the original checkpoint `92a3646`,
  whose ticket content is identical to the rebased checkpoint commit. Publishing
  therefore replaced that single commit with a lease-checked push.

Review method: read the Cycle, design and plan in full and checked each
dependency claim against the merged source (synchronization request and error
types, conflict scope, cancellation, callers, schemas, registry rules, fixtures).
No Rust command was run; this is a documentation change.

| Finding against merged Cycle 06 | Classification | Resolution |
| --- | --- | --- |
| A comment checkpoint leaves the Git index stale and the immediate context synchronization then refuses the worktree as not clean; every Cycle 06 SSH fixture refreshes the index by hand | Blocker | New entry-gate prerequisite: ticket `01M4GD0KKXW684QBA49F6EX3WE` merged first; no workaround in production or fixtures |
| A pending conflict in any context returns `Busy` to every other synchronization; no abandon path (ticket `01M4H33R34Z7C7EEKTY1ZCT950`) | Accepted limit | Stated in the Cycle; mapped to saved-local; one acceptance case added |
| Cancellation with a pending conflict is a recoverable stop; cancel is a no-op on a child parked after a released conflict | Contract change | Design cancellation paragraph and Task 4 tests amended |
| `SynchronizeRemoteRequest.confirmed_identity` and opaque `ExpectedConfiguration` on `IdentityRequired` | Ruling | Ruling: both compound requests forward an optional confirmation to the child only — keeps a later merge completable without resubmission — cost if wrong: one unused optional field |
| New typed errors `ConflictPending`, `ExternalResolutionRequired`, continuing `PushRejected` | Settled | Mapped by name in design and Task 3, one test per category |
| A terminally cancelled bound child cannot be reused | Ruling | Ruling: the comment stays saved-local and publishes through a later ordinary context synchronization — avoids allocating a second child for one receipt — cost if wrong: retry reports recovery where a fresh child could have published |
| Resolution APIs are keyed by the synchronization operation ID | Settled | The receipt's bound child ID is that key; Task 5 resolves through it |
| `submit_comment` has more callers; the read boundary and `operation.schema.json` name the action; `SyncDeferred` is in no schema | Settled | Design caller list updated; action name kept |
| New table must follow Cycle 06 registry rules | Settled | Task 2 amended: all-or-nothing migration, startup validation, table inventory, uniqueness against remote records |
| Windows path headroom of about 40 characters; comment files are the longest paths | Settled | Task 5 fixture guidance amended |
| Planning-time statements that Cycle 06 is absent | Stale | Dated update notes added; originals kept as the planning record |

The Cycle 06 dependency gate is met. Implementation remains unauthorized and is
now gated on the index ticket. The owner approved both rulings above on 2026-10-09 ("I approve those two
choices"): forwarding an optional identity confirmation, and not reusing a
terminally cancelled child. Ticket stays
open; no implementation, pull request, native CI dispatch, merge, closure or
cleanup is authorized by this checkpoint.

## Task 0: Implementation Entry Checkpoint — 2026-10-10

The owner's hand-off authorizes Task 0 rebase, implementation of Tasks 1–6,
local commits, local gates and canonical ticket comments. Execution remains
direct and sequential, with independent read-only review per task. Publishing
the rebased branch (force-with-lease pinned to the original published HEAD),
later pushes, CI dispatch, PR, closure, merge and cleanup require separate
authorization. The owner subsequently approved the validated creation timestamp
field already required by the design when asked to reconcile it with the
hand-off's narrower persistence list. This is a narrowly scoped timestamp
exception; bodies, body hashes, secrets, endpoints and arbitrary paths remain
excluded from new recovery metadata and diagnostics.

### Base and dependencies

- Ticket worktree clean before rebase; main had unrelated untracked
  `.superpowers/`, left untouched. No other ticket worktree or branch changed.
- Fresh `git fetch origin main`: `f87ce81001f75ff6fc21192b71019b0bcaef6ea6`.
- Original published ticket HEAD:
  `59f27a94b1358bdc7172d10610ae08969b09ff90`.
- Rebase completed without conflicts; new planning HEAD:
  `424c60f03037761db9bbbeec4c9c53a53b607daa`. Ancestry check passed;
  post-rebase worktree clean. No push performed.
- Re-read rebased AGENTS, Cycle, design, plan, ledger, ticket/comments and
  merged Cycle 06 Task 7 evidence. The remote production modules are unchanged
  since the approved amendment's base `5e4fad6`; confirmed identity, child-keyed
  resolution, repository-wide publication conflict block, recoverable conflict
  cancellation and terminal cancellation rulings remain applicable.
- PR #16 (`842fd16`, implementation `2215834`) is an ancestor of this branch.
  `devenv shell -- cargo test --locked --test remote_synchronization
  context_first_current_fast_forward_local_ahead` passed: one real-SSH case.
  It invokes public save then public synchronization without fixture index repair.

### Compatibility review

Independent read-only code review at `424c60f` found no blocking behavioral
incompatibility from PRs #15/#16. The reviewer inspected code, approved documents,
operation schemas/read DTOs, rejection regressions and remote identity/replay
guards; it did not run Rust or change files.

| Finding | Classification | Implementation consequence |
| --- | --- | --- |
| PR #15 keeps an effect-free rejected local row as completed/rejected; same-ID same-target reuse resets that row to a new call, while mismatch checks run first | Compatible lifecycle refinement | Rejected-row reset does not reset compound identity: lookup existing binding independently of `RecoveryRecord.is_new`; retain the child across rejection, refresh, reopen and exact retry. Pin this in Task 2 tests. |
| Written/uncertain effects remain pending; completed/rejected is not checkpoint proof | Existing contract | Do not start a child or invent a receipt from journal status. Prove immutable Git evidence; preserve Saved on every post-checkpoint failure. |
| A rejected parent ID still occupies local identity and cannot become a remote ID | Existing guard | Generated child uniqueness includes every local row, rejected rows included, plus remote and binding identities. |
| PR #16 refreshes only committed owned live-index entries, including no-change repair, preserving unrelated entries | Prerequisite satisfied | No index-refresh workaround; foreign lock/collision still maps to saved-local publication recovery. |
| Additional normal submit caller: `tests/journal_rejection.rs:250` | Caller-list correction | Migrate with `tests/local_authoring.rs`, `tests/recovery_foundation_gate.rs`, `tests/remote_merge_recovery.rs` and `src/repository/remote/sync_tests.rs`. No desktop or CLI submit caller exists. |
| Wave 03 DTO/schema explains completed/rejected; action name unchanged | Documentation correction | Preserve `submit_comment` operation identity and rejection read semantics. Retiring SyncDeferred changes no published schema. |

The conflict-block amendment describes publication of every context. Existing
local authoring still refuses a write inside the context owned by a pending
merge; composition preserves that guard rather than weakening Cycle 06.

### Baseline evidence

All commands from the ticket worktree via Devenv at unchanged `424c60f`:

| Gate | Result |
| --- | --- |
| `cargo check --all-features --locked` | PASS |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS |
| `cargo test --all-features --locked` | PASS on rerun with 3,600,000 ms tool budget |
| `cargo run --locked --bin manyhands-cli` | PASS; no window |

The first full test attempt hit the 1,200,000 ms tool limit during the SSH
synchronization suite after 38 passing cases. No test failure was reported;
the next case (`remote_change_during_fetch`) passed in isolation. The rerun
completed all suites, including lib 468, local authoring 131, journal rejection
18, foundation recovery 51, real-SSH merge recovery 25, remote observation 15,
synchronization 63, SSH fixture 31, transport 105 and doctests 9. One existing
read-characterization test is intentionally ignored. No existing baseline
failure found. Full rerun output is in the session tool log
`tool_125cf44d8001yWWQ9isW7Y6AJm`; interrupted attempt in
`tool_125c05f8b001DT1z27ko2T9hSl`. Native evidence for this Cycle remains pending
authorized publication/dispatch. No Rust, Cargo, lockfile or CI edit at Task 0.

### Task 0 follow-up: affected contract wording awaiting owner approval

The PR #15/#16 compatibility review passes, but its final conflict-scope note
identifies a semantic discrepancy in the approved Cycle's amendment point 2.
The earlier entry above describes the existing guard; it is not approval to
correct the stronger promise in the approved Cycle. Task 0 remains blocked
on owner approval of this affected paragraph before Rust edits.

The approved Cycle (`wave-02-cycle-07-comment-publication.md:159–164`) says:
"A comment submitted meanwhile is saved locally with publication pending for
every item, not only the conflicted one." Actual public local authoring rejects
submission into the context owned by the pending synchronization merge before
its canonical write (`src/repository.rs:2219–2224`, with a second guard at
`:2259`). Other contexts can still author; their synchronization gets Busy.
This is not a change introduced by PR #15/#16, and no relaxation of Cycle 06's
guard is proposed. It is a correction to a behavioral promise in an approved
document, rather than a name-only difference.

Runtime evidence: `devenv shell -- cargo test --locked --lib
review_resolution_releases_parent_after_verified_cleanup_and_replays_original_attempt`
passes one test. It asserts public `submit_comment` in the matching pending
context returns `RepositoryErrorKind::RecoveryRequired` before resolution and
works after the parent's local release (`src/repository/remote/sync_tests.rs:
2447–2449,2511–2516`). An earlier attempt with `-- --exact` selected zero tests;
that invocation is not evidence and was corrected by the command above.

**Proposed replacement for Cycle amendment point 2, pending approval:**

> **Repository-wide conflict block.** A pending synchronization conflict in any
> context makes every other synchronization in the repository return `Busy`,
> and there is no abandon path yet (ticket `01M4H33R34Z7C7EEKTY1ZCT950`). A
> comment that can be checkpointed in another writable context is saved locally
> with publication pending/`Busy`. Existing local authoring refuses a new comment
> in the context owned by the pending merge before writing/checkpointing it;
> that pre-checkpoint refusal returns local recovery, not a saved receipt. This
> Cycle preserves both guards; contract point 6 maps delegated `Busy` after a
> proven checkpoint to saved-local.

The design replay table already specifies "Another context's conflict is
pending" and Task 5 already asks for the Busy case in another context. No
redesign of synchronization, cancellation, binding or conflict resolution is
requested. The approved Cycle paragraph has not been edited. No Rust edit,
branch push, CI dispatch, PR or closure performed. Next action: owner approves
the affected Cycle wording, then apply it and finish Task 0 before Task 1.

### Task 0 approval — 2026-10-10T13:10:42Z

The owner answered "Yes" to approval of the exact proposed replacement for
Cycle amendment point 2. Applied the qualification to that approved paragraph;
the existing Cycle 06 local guard and saved-local/Busy mapping stand. Task 0
is complete. No additional implementation, publication or lifecycle authority
is inferred from this document approval.

## Task 1: Compound API and Local Checkpoint Extraction — 2026-10-10

Task review base: `4d69314` (approved Task 0 qualification). Direct controller
implementation; independent read-only review and re-review of the actual diff.

- Added the compound requests, original receipt, independent publication/index
  states and redacted submission error in `repository/comment_publication.rs`.
  Requests carry host/confirmed-identity controls but no endpoint/refspec/key/
  passphrase/force controls. Large receipt/recovery/error values are boxed;
  Debug never renders drafts, roots, paths or backend text.
- Extracted existing validation/identity/owned-path/scoped-checkpoint behavior
  into the crate-private `checkpoint_comment_locally`. No public local-only
  submission bypass; the identity-config test seam also takes a compound request
  and caller-owned session. Retired normal `SyncDeferred`.
- Migrated local-authoring/foundation/journal callers through an explicit test
  session adapter calling the public API. Source-included Cycle 06 setup uses
  the crate-private helper to arrange independently synchronized history. This
  is fixture setup, not a public authoring bypass or index repair.
- Updated the Wave 03 API audit with a dated current-signature/receipt/retry
  update and acceptance limits while retaining its historical source evidence.
- A coherent public Saved receipt requires stable child correlation. The
  minimum binding/receipt/delegation foundation from Tasks 2–4 is therefore
  included in this API checkpoint rather than returning an ephemeral child or
  exposing a temporary bypass. Their migration/collision/fault/transport and
  retry acceptance remains separate work; this does not complete Tasks 2–4.

### Review and fixes

Initial independent Task 1 review requested changes. Confirmed each finding
against code and failing tests, then re-reviewed the fixes:

| Finding | Resolution/evidence |
| --- | --- |
| Missing-identity retry after an intervening context save could commit, then lose Saved because its frozen pre-OID no longer matched | Added failing regression using an intervening public document save. Now check frozen pre-OID before canonical writes; refusal leaves HEAD and missing comment unchanged. Known postcommit effects always retain Saved/local recovery. |
| Receiptless body-bearing replay could recreate an externally deleted comment | Added failing receipt-persistence/deletion/replay test. Reconcile existing binding against immutable Git before entering the writer; return the original OID and recovery without recreating the file. |
| Original receipt persisted after discovery rather than before it | Persist validated OID/time before discovery. On journal/receipt failure retain in-memory binding and known commit so the public result remains Saved. |
| Journal observation failure left body-free retry unable to claim local indexing | Added failing handoff-repair regression. A short-lease conditional update advances only pre-index states; existing active indexing owner state/epoch is preserved. Then reuse normal refresh after releasing the lease. |
| API audit stale and outcome assertions too broad | Dated audit update; exact NoPublicationRemote assertions; configured-remote regression requires delegated Transport(RuntimeUninitialized). |
| New binding FK blocked deliberate registration removal | Binding cascades with its local operation record, matching existing explicit registration-removal recovery deletion. Missing correlation subsequently requires recovery, never inferred adoption. Journal rejection regression passes. |

Re-review found no remaining Task 1 blocker by source inspection. It carried
these explicit Task 2 acceptance debts: reciprocal child-ID collision checks at
local and remote consuming transactions; strict substituted-schema/root/target
startup validation; table-inventory update; receiptless original checkpoint made
unreachable by a reset back to its pre-OID (must not create a replacement).

### Verification

Red type/API compilation established the missing compound signature and types;
fixture ULID spelling was corrected before the second red run. The two review
regressions and the journal-repair assertion were observed failing before fixes.
The new fixture constructs its born canonical initial commit before service
calls and performs no index repair after any save or comment.

Focused green checks via Devenv: library `comment_publication` 7; local-authoring
`comment_` 17; foundation recovery 51; journal rejection 18. All-target,
all-feature locked Cargo check and clippy (`-D warnings`) pass. Cargo fmt and
`git diff --check` pass. Clippy initially flagged large values; boxed them and
reran clippy plus focused library/authoring tests green. Full final and native
Cycle evidence is still owed by Tasks 5–6. The exact table inventory is a known
Task 2 regression migration, not a claimed passing full suite.

## Task 2: Durable Binding and Receipt Recovery — 2026-10-10

Review base: `82623fa`. Added strict binding-table/marker migration in the
**existing operation-record transaction**, including startup root-digest,
parent matcher and child root/action/item checks and the exact table inventory.
The allocation transaction rechecks its parent row identity and all local/
remote/binding ID collisions. Reciprocal consuming guards prevent another
local action, plain refresh or remote target from taking an unused child; only
its authoritative ordinary refresh may coexist.

Receipt proof checks immutable commit/tree/path, one owned added path, the
recorded direct pre-parent, original canonical IDs/time and context ancestry.
Receiptless reconciliation finds unique matching objects including unreachable
ones; an unreadable plausible candidate or competing checkpoint requires
recovery. Fresh new writes do not perform that replay ODB scan under the lease.

### Review and crash evidence

Independent review found one remaining replacement path after receipt failure,
reset and pruning/missing Git authority. Reproduced it in a red test, then added
fixed categories to the existing local journal:

- `comment_destination_prepared`: validated creation time is persisted before
  the first canonical write, never a body or fingerprint. A prepared/observed
  missing file requires recovery rather than another creation timestamp.
- `comment_checkpoint_intent`: durable before actual Git checkpoint execution.
  Without proved Git authority an exact retry cannot create a replacement,
  even when original objects have disappeared. The existing known injected
  precommit stop fires before this intent and still resumes its retained file.
- Two hidden failure points following the existing test-seam pattern pin the
  prepared-before-write and intent-before-commit windows. Both reopen into
  typed recovery with unchanged file observation, timestamp, HEAD and binding.

The existing binding timestamp field is now independent of checkpoint nullness:
it is validated from canonical serialization before write; checkpoint authority
is stored only after proof. This implements the owner's timestamp exception and
preserves one observed creation time. An ephemeral `receipt_recorded` flag
distinguishes stored authority from in-memory postcommit evidence; no extra
remote phase or reservation is persisted in the binding.

Review re-runs approved the production fixes and, after the two intent-window
tests, found no remaining Task 2 blocker or acceptance gap. Other covered cases:
pending local/interrupted remote rows preserved through legacy migration,
failure after binding DDL rolling back both table and marker, rejected-row
refresh/reopen identity, valid authoritative child/refresh/reopen coexistence,
missing commit/tree/blob with and without retained file, multiple plausible
checkpoints, cross-root/target/body/ID mismatch, concurrent services, and
body/endpoint/passphrase/server/body-hash canaries absent from database sidecars.
The authoritative-refresh unit case arranges durable metadata; it is not real
SSH publication proof.

Interim Task 1 registry definitions are intentionally not upgraded under the
owner's explicit hand-off ruling: no running instances and no earlier-branch
registry upgrade requirement. Main's legacy databases remain additive migrations
with no inferred correlation/publication. An early schema placement broke the
existing rollback regression; moved the new table/marker into the operation
transaction and proved rollback after DDL as well as during legacy import.

### Verification and next debts

Devenv: full library **493 passed**, including 25 comment-publication tests;
foundation recovery 51, enablement 78, remote reservation 9; full local authoring
131 and journal rejection 18 passed during the task, with comment-authoring 17
rerun after intent/timestamp changes. All-target/all-feature locked clippy with
warnings denied, fmt and `git diff --check` pass. The first concurrency fixture
used a resolution-only temp hook and timed out; switched to the actual normal
writer Replace boundary and verified it green. No sleeps or index workaround.

Carry to Task 3: live canonical-worktree validation can mask the child's typed
pending conflict recovery; missing-registration discovery must remain independent
index-pending evidence. Real SSH, remaining mapping/retry/cancel acceptance and
final native gates remain Tasks 3–6. No push, dispatch or closure performed.

## Task 3: Immediate Context Synchronization and Independent Indexing — 2026-10-10

Review base `41422f1`. Delegation uses the original receipt's one persisted child
and only `SynchronizationTarget::Context`, caller-owned credentials, optional
host approval and opaque identity confirmation. Publication relays verified
Published/AlreadyCurrent only with the original checkpoint ancestry and original
blob/mode at the authority OID. Fixed-category mapping preserves child recovery
payloads, including ExpectedConfiguration; repository backend diagnostics are
replaced with fixed text.

Resolved carried source debts:
- Context containment checks deterministic worktree/branch/common-directory
  identity without requiring malformed live conflict source to parse as an item.
  Committed ancestry/blob proof remains mandatory. The child supplies its own
  materialization/conflict recovery rather than losing it behind a generic error.
- Missing registration is independent local index-pending evidence. The original
  local record parks as completed/comment_registration_pending so explicit enable
  remains admissible; recovery inspection, compound pending detection and the
  remote pending-local guard still recognize the unfinished handoff. Only a
  proved original receipt can restore its pre-index state after registration.
  No implicit registration or replacement comment/checkpoint is performed.
- Initial review found a deadlock if receipt or parking metadata failed before
  successful parking. Reproduced red; proved-receipt retry now retries the eligible
  parking transition while registration is absent. It excludes live indexing
  owner states/epochs. Both interruption windows now enable and repair using the
  original identity without requiring a body.

Independent re-review approves Task 3 source with no remaining blocker. Targeted
assertions now cover both leases released at discovery, competing retry preserving
an active index owner, recorded-receipt reset/removal/replacement negatives,
named recovery mapping/redaction, opaque identity and missing-registration repair.

### Real SSH evidence and early Task 5 infrastructure

Created the planned custom-host `comment_publication` target and wired it into
the existing five-target manual workflow now so Task 3 ordering/delegation has
actual transport evidence. Fixtures seed a born canonical repository before
service calls, use two short-path clones and a one-round protected fixture key,
and perform no index refresh after authoring. No dependency/lockfile changes.

Eight real-SSH cases pass through the public API:
- document and ticket roots plus nested replies, three original checkpoints,
  discovery thread membership, exact remote ancestry/blob proof and historical
  no-transport terminal replay;
- local-only zero prompts/helpers/reservations, then configure and publish with
  the same body-free receipt/child;
- dirty context preserves saved receipt and unrelated bytes;
- credential prompt observes checkpoint and completed discovery with Git/cache
  leases available;
- another context's active reservation yields Busy after the scoped checkpoint;
- verified publication plus remote index failure stays Published/index-pending;
  reopen/retry repairs discovery only, with unchanged transport/push counters;
- dirty local-only context also proves zero remote work and byte preservation.

The Busy fixture must use a properly initialized endpoint scope; initial synthetic
reservation before endpoint initialization legitimately gave HistoryUnknown.
Prime deliberate primary observation/sync first for that case, preserving Cycle
06's policy rather than changing its production preflight. The unit no-key/runtime
fixture is not used as Busy transport proof.

Devenv green: focused publication library 32, comment authoring 17, full local
authoring 131, foundation recovery 51, remote reservation 9; real SSH target 8.
All-target/all-feature locked clippy and fmt pass; `git diff --check` passes.
No CI dispatch/native-pass claim. Full merge/conflict, cancellation/restart,
identity-removal/confirmation, collaborator AlreadyCurrent and comprehensive
transport/crash acceptance remain Tasks 4–5; final gates/review remain Task 6.
