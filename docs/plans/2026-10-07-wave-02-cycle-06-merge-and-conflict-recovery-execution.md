---
title: "Wave 02 Cycle 06 Merge And Conflict Recovery Execution Ledger"
date: 2026-10-07
status: active
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4B1FEW1HFNXG8FG10QAAFVN"
---

# Wave 02 Cycle 06 planning and execution ledger

Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DE`
Branch: `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DE`
Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DE`
Artifacts: [Cycle](../Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md),
[design](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md),
[implementation plan](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md).

## Authority and preflight — 2026-10-07

User authorized planning all three artifacts and self-review with the applicable
skill, then owner review/approval. Implementation, local checkpoint commits,
push/PR, CI dispatch, merge, ticket closure and worktree cleanup are not selected
or authorized. Recommended future execution is sequential direct tasks in the
existing ticket worktree; delegation requires separate authorization.

Read `starting-a-cycle`, its worktree preflight, `review-cycle-docs` and its
review matrix. Reused the existing ticket and its registered clean worktree;
no replacement ticket/worktree. Main's unrelated untracked `.superpowers/` and
`devenv.nix~`, and all other worktrees, were preserved.

Fresh `git fetch origin main` observed
`b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`; local main matches fetched main.
Ticket before rebase: `2dbd3b4777674e7cd0fab584239e62202f677f9c`.
Ticket after rebase: `8fb768e39835acb7b84cde81df7083a03995b645`.
Rebase completed without conflicts; fetched base ancestry and clean status
verified. Reread changed rebased `AGENTS.md`. This rebases the ticket ONTO main,
not the user's main checkout onto the ticket.

## Grounding and evidence limits

Read approved Wave 02, canonical schema, Git workflow/conflict recovery and
test strategy, relevant repository/index and authentication contracts, approved
Wave 03 owned-path/external-repair constraints, Cycle 05 artifacts/ledger/final
comments and actual source/CI seams named in the design.

Current main contains Cycle 05 synchronization. Its final comments record
local789/static/CLI and whole-Cycle review; its ticket remains open. These are
historical dependency facts, not fresh Cycle 06 passes. Its owner-approved native
CI deferral applied to that delivery, not automatically this one. The current
workflow is manual-only; no enabling/dispatch/publication was performed.

No Rust implementation, backend characterization, baseline test suite or native
Cycle 06 verification has run. Future evidence is explicitly planned in Tasks
0–7; source feasibility assumptions are not proof of acceptance.

## Self-review decision — 2026-10-07

Review found a material ambiguity not settled by the owned-Markdown restriction:
whether a mixed canonical/unsupported conflict set could receive partial
in-process canonical resolution, or required external recovery of the whole
merge. Asked the owner with both concrete options and their capability/recovery
cost. The owner selected **"Whole merge external (Recommended)"**.

Record: require external tools to resolve AND commit the entire mixed merge,
then deliberate re-observation/resume. Make no canonical resolution writes to a
mixed set. This preserves a single unambiguous checkpoint and avoids ambiguous
partial staging. Accepted cost: Manyhands cannot resolve the canonical portion
of mixed conflicts. All-canonical supported journeys remain required. This
answer is a scoped recovery decision, NOT approval of the three artifacts or
implementation authorization. Recorded in Cycle, design, plan and ticket.

## Engineering rulings

These preserve approved behavior; each is an internal/reversible implementation
choice. The design decision audit links source, promised contract and required
proof. They are proposed engineering commitments for artifact review.

- **Ruling: dedicated two-parent resolution candidate** — the existing scoped
  checkpoint builds from HEAD with one parent, losing a merge's nonconflicting
  incoming tree. Build from the recorded merged index and explicit owned blobs.
  **Cost if wrong:** refactor the writer; never stage arbitrary code or relax
  parent/path scope. Proof: A3/A5 tree, parent and replay cases.
- **Ruling: ordered child integration evidence under the existing envelope** —
  context/primary steps cannot rewind one old monotonic local checkpoint. Keep
  one reservation/owner with per-stage input/candidate/completion evidence.
  **Cost if wrong:** internal schema adaptation; retain prior rows/effects.
  Proof: A5/A8 migration, faults, repeated-stage and fencing tests.
- **Ruling: transient preparation ODB** — self-review of locked
  [libgit2 v1.9.7 merge.c](https://github.com/libgit2/libgit2/blob/v1.9.7/src/libgit2/merge.c)
  found `git_odb_write` in content resolution: an in-memory merge index does
  NOT make preparation on the destination ODB read-only. Locked
  [git2 0.20.4 odb.rs](https://github.com/rust-lang/git2-rs/blob/git2-0.20.4/src/odb.rs)
  exposes `add_new_mempack_backend(1000)`. Prepare on a separate worker-local
  memory-ODB handle, then import only verified result objects under the lease.
  **Cost if wrong:** reassess the preparation seam; no silent outside-lease
  disk writes or long-lease computation fallback. Proof still required:
  A1/A8 no-disk-write/handle-isolation/backend-lifetime and object-import faults.
  Source inspection is feasibility input, not executed backend proof.
- **Ruling: resolution-attempt plus per-path digests** — partial multi-file
  writes and uncertain commits must recognize own effects without storing bodies.
  **Cost if wrong:** additional recovery evidence and manual stop, not rollback.
  Proof: A3/A5/A8 write/record/ref faults, altered-input replay and privacy.
- **Ruling: deterministic primary subjects** — use `Merge remote primary` and
  `Resolve synchronization primary` where item-specific RFC subjects do not fit.
  **Cost if wrong:** fixed non-secret wording only. Proof: A1 subject assertions.
- **Ruling: exact recorded-parent proof for external repair** — marker edits,
  resolved-looking index flags or arbitrary commit subjects are insufficient.
  Require a clean two-parent merge of the recorded inputs before eligible resume.
  **Cost if wrong:** stricter manual recovery for unusual external history, no
  silent overwrite. Proof: A4/A5 positive and adversarial external repair cases.
- **Ruling: auxiliary identity-confirmation evidence** — missing Git identity
  needs caller approval without changing original synchronization target or
  regenerating a prepared candidate. Use a distinct bound confirmation ID and
  expected identity-config observation; reconcile partial config writes.
  **Cost if wrong:** stricter identity recovery/internal API adaptation, never
  overwrite a valid effective identity. Proof: A3/A5 identity/replay faults.
- **Ruling: bounded current-ref integration pass** — re-observe newer remote
  history without repeatedly chasing concurrent writers in one invocation.
  Pin one fetch observation, at most context then primary (primary: one stage)
  and one push attempt; later races return recovery for deliberate retry.
  **Cost if wrong:** another explicit invocation, not a lost merge or force push.
  Proof: A5/A6 receive-race and completed-stage preservation.

## Review corrections and completion

Self-review checked every applicable matrix row: interfaces/scope, identity and
host/key trust, secrets/prompts, persistence/corruption/recovery, time/failure,
locked backend/platform, real fixture/test fidelity, verification and lifecycle.

Corrections before plan-ready handoff:

- Classified mixed-set policy as an OWNER DECISION, not an implied settled RFC
  requirement; recorded its explicit usability cost in all artifacts.
- Clarified identity-required supplementation versus stable request/attempt
  input, no identity for non-committing fast paths and partial local-config writes.
- Corrected the assumption that an in-memory merge index is read-only: real
  libgit2 writes merged blobs. Specified a preparation-only memory ODB handle,
  explicit no-disk/other-handle characterization and coordinated object import,
  keeping actual object/index/ref mutation under the lease.
- Clarified typed conflict categories are recoverable errors, not publication
  authority; fixed the existing recovery module path.
- Bounded changed-remote reconciliation and scoped the pending-merge authoring
  guard to the affected target, not an indefinite freeze of unrelated contexts.
- Kept native evidence pending and separate from prior Cycle 05 deferral; no
  mock/backend assumption or planned command is reported as completed proof.

Re-read the three corrected artifacts and ticket/checkpoints for consistency.
Static document checks passed across seven planning/ticket/comment files:
29 local links/anchors, canonical frontmatter/ULID shape, 164 globally scanned
unique IDs, balanced fences, final newlines and whitespace, plus tracked
`git diff --check`. All three approval artifacts remain `proposed`. Only ticket
Markdown is tracked-modified; new docs/comments are uncommitted. No source,
Cargo/Devenv or CI edits, Rust/native test runs, commit or publication occurred.
Main's unrelated untracked files remain preserved.

No material question remains open after the owner's answer. Backend safe-
checkout/merge-metadata characterization and native-platform behavior are future
feasibility/evidence gates; stop for document reassessment if they fail.

## Implementation entry — Task 0 — 2026-10-07

The user explicitly authorized implementation of Tasks 0–7 in this ticket
worktree, requested sequential execution with local checkpoint commits, and
requested subagent-driven development. The ticket remains open; no push/PR, CI
dispatch, merge, closure or cleanup is authorized.

Fresh preflight preserved the unrelated main-checkout `.superpowers/` and
`devenv.nix~` entries and all other worktrees. The clean ticket branch started
at `a3c051f599172dec7c487dc6836460416289ac67`. `git fetch origin main` observed
`origin/main` at `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`; rebasing the ticket
onto it was a no-op. The resulting head remains
`a3c051f599172dec7c487dc6836460416289ac67`, fetched-main ancestry passed, the
planning commit remains an ancestor, and ticket status was clean before this
Task 0 record.

Fresh baseline on that revision passed through Devenv:

- `cargo check --all-features --locked` (exit 0);
- `cargo fmt --check` (exit 0);
- `cargo clippy --all-targets --all-features --locked -- -D warnings` (exit 0);
- `cargo test --all-features --locked` (exit 0); and
- `cargo run --locked --bin manyhands-cli` (exit 0; headless smoke test).

This is new Cycle 06 local evidence, not a substitution for the planned
backend/remote tests or native five-target evidence. Native evidence remains
pending and must be collected through separately authorized manual CI execution;
Cycle 05's deferral does not apply. Task 1 may proceed subject to the approved
mempack-isolation stop condition.

## Task 1 — locked merge backend characterization — 2026-10-07

Checkpoint commit `6f65a79c9a0be0f52b5891d5a9b098b6f2ed3682` adds private
`remote::merge` graph, eligibility, redaction, resolution-request and optional
caller-confirmed-identity seams. No synchronization/network orchestration,
state migration, UI/CLI grammar, dependency or Devenv change was made.

Focused local commands passed through Devenv:

- `cargo fmt --check` (exit 0);
- `cargo clippy --locked --lib -- -D warnings` (exit 0);
- `cargo test --locked --lib repository::remote::merge` (exit 0; 9 tests); and
- `cargo test --locked --lib repository::remote::refs` (exit 0; 9 tests).

The mempack hard-stop characterization passed on locked git2 0.20.4/libgit2
1.9.7: a separately opened worker handle with
`add_new_mempack_backend(1000)` produced a same-file clean-merge blob while
object-directory names, destination index, HEAD/ref, worktree bytes and a
separately opened destination handle remained unchanged. The generated blob
was unavailable through that other handle until an explicit destination ODB
import; that import preserved the exact OID. Resetting the mempack made the
unimported object unavailable through the worker, and recomputation restored
its same OID. This proves the required preparation boundary on this Linux
fixture; no disk-ODB fallback was used.

Real local fixtures also characterize index-only `merge_commits`, actual
`merge` conflict index stages/markers/MERGE_HEAD cleanup with unchanged HEAD,
two-parent clean trees, safe checkout preservation for tracked/untracked/
ignored/symlink/file-directory collisions, and linked-worktree index/merge
metadata isolation. Native five-target evidence, actual orchestration,
persistence/replay, canonical parsing and public resolution behavior remain
unimplemented and unverified future tasks; this local result is not a native
platform claim.

## Task 1 review remediation — 2026-10-07

Independent review correctly found that the original mempack fixture checked
only the resolved current HEAD target, so a write to another destination ref
could have escaped the assertion. Commits
`b9931fd21c5a724028c503d5f3c09bb4b696879f` and
`b80ba384bb0393b0ea9c7e4b0582937783050b50` resolve that P1 without product
behavior changes. Before worker preparation, the fixture now recursively
snapshots byte-for-byte the common Git directory and the worktree Git directory
for `HEAD`, loose `refs`, optional `packed-refs`, and reflogs. The snapshot
records directories, regular-file bytes, symlink targets, and path absence by
map membership; equality after both the initial and reset/recomputed worker
merges therefore detects changed, created, or removed loose/packed refs,
symbolic/direct HEAD contents, and reflogs in either repository layout location. The original destination object,
index, worktree, other-handle visibility, mempack-lifetime, and explicit-import
assertions remain intact.

Focused Devenv formatting and merge tests passed for this test-only correction.
It remains Linux local fixture evidence; it does not add native-platform,
orchestration, persistence/replay, canonical-resolution, or remote/SSH proof.

### Task 1 validation remediation — 2026-10-07

The required all-target/all-feature clippy validation found
`clippy::unnecessary_mut_passed` in the linked-worktree portion of the Task 1
test at `src/repository/remote/merge_tests.rs:428`. Commit
`e5a3ab2a8c5a4b6e39149c25bbe07844c4393422`
(`test: pass immutable worktree options`) corrects only that call site:
`WorktreeAddOptions` remains mutable while `reference(...)` configures it, then
is passed to `Repository::worktree` as `Some(&worktree_options)` rather than
`Some(&mut worktree_options)`. The ODB/mempack isolation, conflict-state,
index, and linked-worktree assertions are unchanged.

Required Devenv validation passed with exit 0: `cargo fmt --check`; `cargo test
--locked --lib repository::remote::merge` (9 tests); `cargo clippy --all-targets
--all-features --locked -- -D warnings`; and `git diff --check`. This remains
local Linux fixture evidence only; native five-target and later Cycle evidence
remain pending. No product behavior, dependencies, network/orchestration,
canonical writes, UI/CLI, CI, push/PR, merge, closure, or cleanup changed.

## Task 2 — ordered recovery and resolution evidence — 2026-10-07

Task 2 source checkpoint is
`2db1632bfc78fcc9a79698f36568613034c5b82f`
(`remote: persist ordered merge recovery evidence`). It adds the approved child
evidence to the existing remote operation journal, without a second lock or any
transport/canonical mutation:
`remote_integration_steps`, `remote_identity_confirmations`,
`remote_resolution_attempts`, and `remote_resolution_paths`. The migration is
one SQLite transaction and refuses partially present child schema. Required
columns and immutable triggers are checked on repeat migration; startup audits
foreign keys for the child tables and decodes linked evidence while preserving
all Cycle 04/05 operation rows. Existing terminal authority, index-pending,
cancellation and ambiguous-push evidence is retained and does not infer a
merge/resolution/identity effect.

Rows bind operation record/repository target through the immutable parent,
configuration generation, owner epoch, ordered stage ordinal, immutable OIDs,
and 32-byte digests. Bounded indexes and operation-scoped queries avoid walking
retained history in ordinary transitions. Integration intent becomes durable
before its effect; observed OID/tree completion is written only afterwards.
Identity confirmation has independently bound input/configuration digests and
observed configuration progress. Resolution attempts bind exact observation and
input digest, optional confirmation reference and digest-only per-owned-path
facts; no body, credential, endpoint, server text or arbitrary path is stored.

Conflict release changes only the active parent slot to interrupted after the
conflict stage row is durable. The conflict is retained, a different
synchronization is busy, old tokens are fenced, normal restart cannot adopt it,
and explicit reacquisition requires the matching operation target, generation,
stage and conflict digest before advancing the epoch. The new state-only
transitions cover prepare/apply/observe for integration, identity, resolution
paths, candidate and checkpoint; Task 3/4 remain responsible for real Git and
canonical effects.

Focused Devenv commands passed (all exit 0):

- `cargo test --locked --lib repository::remote::state` (21 tests);
- `cargo test --locked --lib repository::remote::reservation` (25 tests);
- `cargo test --locked --test remote_reservation --test recovery_foundation_gate`
  (9 and 50 tests);
- `cargo fmt --check`;
- `cargo clippy --locked --lib -- -D warnings`; and
- `git diff --check`.

Regression coverage exercises actual disposable SQLite migration/repeated
migration, legacy published/index-pending preservation, partial schema and
orphan rows, bounded retained-history migration, ID replay mismatch, stale
owner/two-service race/conflict reacquisition, ordered durable-before-effect
and observed-after-effect transitions. Privacy coverage scans schema, live WAL,
database and `VACUUM INTO` backup for body/credential/endpoint sentinels;
formatted state contains only fixed values. This is local Linux SQLite evidence.
Native five-target behavior remains pending; no CI was enabled or dispatched.
No remote/network, canonical file, UI/CLI, dependency, workflow, push/PR,
merge, closure or cleanup change was made. Ticket remains open.

### Task 2 P1 review remediation — 2026-10-07

The independent-review P1 findings are remediated in a focused source checkpoint.
Context synchronization now permits only `context` at ordinal 0 and `primary` at
ordinal 1; primary synchronization permits only `primary` at ordinal 0. Both
preparation and retained-evidence audit reject any other pairing, and new
reservation/startup-corruption tests cover it. Same-ID resolution-attempt replay
now resolves the supplied confirmation to a parent-bound row before comparing its
stored foreign key, so absent, changed, foreign, or otherwise mismatched
confirmation input is recovery-required; the regression test prepares two valid
parent-bound confirmations and rejects replay with the changed one.

Merge-evidence schema validation now compares normalized canonical SQLite
fingerprints for every Task 2 child table, explicit child index, and immutable
trigger during migration and startup audit. The fingerprints cover table
CHECK/UNIQUE/FK definitions and trigger bodies while tolerating case/whitespace
normalization. Existing complete-but-weakened schemas are rejected rather than
repaired. A disposable SQLite test recreates all four named child tables with a
weakened foreign key, CHECK, index, or no-op immutable-trigger body and proves
startup returns `RecoveryRequired` for each.

Focused Devenv evidence after remediation (all exit 0):

- `cargo test --locked --lib repository::remote::state` (23 tests);
- `cargo test --locked --lib repository::remote::reservation` (26 tests);
- `cargo test --locked --test recovery_foundation_gate` (50 tests);
- `cargo fmt --check`; `cargo clippy --locked --lib -- -D warnings`; and
  `git diff --check`.

The wider required `cargo clippy --all-targets --all-features --locked -- -D
warnings` remains blocked by the pre-existing unrelated
`clippy::unnecessary_mut_passed` at `src/repository/remote/merge_tests.rs:428`;
that file is outside this remediation and was not changed. No source scope was
widened to repair it. Native five-target behavior remains pending; no network,
canonical, UI/CLI, dependency, CI, push/PR, merge, closure, or cleanup action was
performed.

## Next lifecycle checkpoints

1. Completed 2026-10-07: owner approved Cycle, design and implementation plan.
   Their state is `approved`; ticket remains open.
2. Explicit implementation authorization/method; Task 0 fresh preflight/baseline.
3. Tasks 1–6 focused red/green and preservation/privacy evidence; task comments
   with decisions, failures and review results. Commits only when authorized.
4. Task 7 required local check/fmt/clippy/test/CLI gates and whole-change review;
   real native runs after separate authority or an explicit Cycle-specific
   deferral. Review-ready comment states every remaining gap.
5. Reviewed implementation code-review/PR approval and separate delivery scope;
   canonical closure in final pre-merge checkpoint, never premature planning
   closure. Push/PR, merge and cleanup remain separately authorized.

## Owner approval and planning commit — 2026-10-07T11:26:39Z

The user stated: "The three docs are approved, update their state and commit.
Give me a brief to hand to another session to implement".

Marked the Cycle, detailed design and implementation plan `approved`, removed
current awaiting-approval wording and checked the ticket's artifact-approval
checkpoint. This supporting ledger is now an active lifecycle record; its
historical planning entries remain intact. Preserved the owner-approved
whole-merge external mixed-conflict decision and all review rulings.

## Task 3 P1 remediation and acceptance checkpoint — 2026-10-07

Initial Task 3 checkpoints are `3a9c197` (ordered synchronization conflict
recovery) and `03fe4d6` (its first verification record). Independent review
found three P1s: an owned applying candidate could not continue through the
public restart envelope; conflict side reads selected one mode while
materializing all sides; and prepared divergence inputs were not completely
re-observed under the lease. A later focused review found a fourth P1: reading
`current` through the worktree could follow a post-inspection symlink.

The remediation keeps a prepared candidate private until verified import,
persists its intent before preparation, and accepts restart replay only at the
recorded old HEAD or candidate HEAD. Candidate reconciliation now observes that
local ref state but deliberately remains `reconciling` through a fresh Fetch;
it finalizes the outer local-fast-forward envelope only after that Fetch batch
is durable, preserving ordinary push/publication continuation. Any third head
remains recovery-required. Before every divergence fast-forward or clean merge
mutation, the service reopens the selected target under the common-Git lease
and rechecks its clean symbolic HEAD/index/worktree, endpoint/key
configuration, primary tracking ref, and selected tracking ref.

Conflict inspection tokens now retain the mode of every present ancestor/local/
incoming index entry. A side read rejects unsafe/non-UTF-8/traversal paths and
any non-regular or executable stage before opening blobs. It returns immutable
Git index sides only: `current` is deliberately `None`, so no worktree path is
read or symlink followed after inspection. This protects opaque-token privacy
and retains the external whole-merge recovery boundary for unsupported/mixed
conflicts.

New regression coverage includes exact old/candidate/third-head candidate
restart handling; configuration and tracking races at the pre-mutation recheck;
binary, symlink, and mixed regular/symlink conflict sides; and an actual
conflicted worktree file replaced by a symlink to an external canary. The
existing SSH divergence test was updated from Task 2's obsolete
`MergeRequired` expectation to verify Task 3 ordered two-parent integration
and publication.

Three independent reviews were completed. The final reviewer found no P0/P1.
It retained only a P2 coverage-depth note: the candidate restart test exercises
the controller-level fresh-fetch/finalization path rather than a separate
public SSH failure-point exactly at child `Applying`. This is explicit residual
test depth, not a behavior or safety exemption.

Final required local Devenv validation passed, all exit 0:

- `cargo check --all-features --locked`;
- `cargo fmt --check`;
- `cargo clippy --all-targets --all-features --locked -- -D warnings`;
- `cargo test --all-features --locked` (including 15 + 41 + 31 + 103 SSH
  harness cases and all named integration/unit suites); and
- `cargo run --locked --bin manyhands-cli`.

The first full validation exposed the stale repository-schema expectation in
`tests/repository_enablement.rs`; it was updated to list the Task 2 child
tables. It also exposed an obsolete SSH divergence expectation; it now asserts
Task 3's ordered parents and published ref. Revalidation after both corrections
passed. Native five-target evidence remains mandatory and pending; no CI,
push/PR, merge, ticket closure, or cleanup occurred. Task 3 is ready for one
local remediation checkpoint commit; Tasks 4–7 remain pending.

Current authority is Task 0–7 sequential implementation with local checkpoint
commits only; delivery actions remain explicitly unauthorized. The following
three paragraphs are a historical record of the earlier planning-only approval,
not a current implementation or delivery constraint.

Historical record — implementation was not then authorized. Future work
required a fresh ticket fetch/rebase/ancestry check and baseline checks at
implementation entry; no push/PR, CI dispatch, merge, ticket closure, or cleanup
was authorized.

Historical record — the planning checkpoint commit scope was four Cycle 06
documentation files, this ticket and its planning/review/approval comments only.
Main and other worktrees were preserved.

Historical record — approval-bookkeeping validation found all eight scoped files
had managed frontmatter, valid ULIDs, balanced fences/final newlines/whitespace;
33 local links resolved and IDs were unique. Three specification states were
`approved`; ticket remained open. Wider ULID-shape scan found one pre-existing
25-character ID in Cycle 01 comment
`.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6D9/01M44B2KM3C6H6SG8M2W5P7R9.md`.
Verified it is already in HEAD; left this unrelated canonical content untouched.
This is not a Cycle 06 metadata regression or a claim of globally valid content.

## Task 4 blocked protocol review — 2026-10-08

Task 3 checkpoint is `6bd1f18`; HEAD remains there. Task 4 is uncommitted
and is not accepted. Tasks 5–7 have not begun. Ticket remains open; no delivery
action is authorized or performed.

Latest worker run `94696a95-3fd3-4b3b-b147-879d7fb721f6` reports 139 remote
tests, check, strict clippy, formatting, and diff checks passing through Devenv.
Its full all-feature test attempt failed in the concurrent discovery test
`services_sharing_a_corrupt_cache_replace_it_once` with `RepositoryBusy`; an
isolated retry passed. This is not evidence of a complete full-suite pass.
Native five-target verification remains pending.

Independent static review `51579227-f2bd-4461-a115-d69ac8468b33` reports
BLOCK with seven P1s and one P2. These are source-traced findings, not executed
breakpoint reproductions: real acquisition/serialization/pre-ref crash windows;
path replacement before durable applied observation; retirement placeholder
crashes; content-only recovered lock ownership; foreign-lock release during
retirement; same-image inode substitution at persistence; ticket closure
invariants checked against only one side; and partial merge-metadata cleanup.
Review artifact:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/51579227-f2bd-4461-a115-d69ac8468b33/task4-lock-crash-review.md`.

These findings concern the promised effect-aware recovery and preservation
contracts, not optional test depth. Another patch-only loop is paused pending
a bounded protocol-design decision. Preserve the existing dirty diff and prior
accepted checkpoints. The current Linux-only implementation and added direct
SHA-1 dependency also need explicit reconciliation with the approved platform
and dependency assumptions; they are not silently accepted plan changes.

Owner selected **Protocol redesign review** on 2026-10-08: authorize a bounded
delegated design review of ownership, syscall/crash states, platform support,
and dependency changes. Preserve all work and present a revised protocol for
approval before further implementation. This authorizes analysis, not new code,
acceptance, commits, delivery, or a change to preservation guarantees.

Protocol workflow `a9313d9d-e75f-4cfc-9d9b-161a4ee421be` completed its
proposal and independent challenge, then failed before launching the response:
`protocol-design` and `protocol-response` inherited the same output path.
This was an orchestration artifact-collision failure, not source/test evidence.
Before same-protocol retry, HEAD/branch were verified unchanged and the tracked
binary diff/status/untracked inventory captured under
`/tmp/manyhands-task4-design-retry-h3GSuP`. The original dirty worktree remains
preserved. Retry `e16722d7-48f0-42fa-a222-2af305b7a1e3` resumes only the
completed advisor, with a distinct response output binding. No execution-mode
fallback, new implementation, or approved contract change occurred.

Independent design challenge additionally requires a live backend ref-lock
crash/provenance strategy and concrete native operation choices. The owner's
question about possible relaxations is not approval of a concurrency boundary
or manual-recovery limitation; these remained explicit decisions to present
at that checkpoint.

### Owner concurrency decision and bounded review completion — 2026-10-08

Owner subsequently approved relaxing concurrent-writer protection because
per-item worktrees reduce collision risk and autonomous writers are instructed
to access canonical contents through Manyhands API/CLI. Record the supported
boundary as cooperative writers participating in reservation/lease/authoring
guards. Direct mutation bypassing coordination during bounded resolution apply
and reconciliation is unsupported. Reads and unrelated safe work remain allowed;
external repair outside that interval invalidates observations. Worktrees do not
isolate shared Git refs. No guarantee of perfect expected-inode pathname CAS or
continuous foreign-lock exclusion is claimed against arbitrary namespace
substitution. Observed stale changes still reject, and ambiguous/foreign locks
remain preserved. This is recorded in Cycle, design, and implementation plan.

The response-only retry completed with advisor run
`90d44f5b-8d62-4f56-ae6d-d7ae9f7e3a35`. Final response artifact:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/e16722d7-48f0-42fa-a222-2af305b7a1e3/task4-protocol-response.md`.
The scoped concurrency decision arrived after that report was finalized; its
statement that concurrency is still unapproved is superseded by the owner
answer here. A follow-up steer was queued but did not change the report.

Recommended internal redesign: stable anchored index-lock sentinel, separate
libgit2 index serialization (remove custom SHA-1 serializer/direct dependency),
one actual-state recovery dispatcher, and independently recoverable metadata
retirement. These are proposals, not implementation acceptance. Existing public
API, validation, identity/fencing and applicable tests can be retained.

A separate owner decision remains: ambiguous live locks created inside libgit2
have a creation-before-record window without a public ownership hook. Proposed
narrow exception is operator-controlled recovery after quiescing Git writers,
then exact same-ID reconciliation; never automatic stale-lock deletion or
content/age/PID-only ownership inference. Full automatic backend-lock recovery
would require separately scoped feasibility/backend work. The exception was not
yet approved at that checkpoint.

Owner then selected **Allow narrow manual recovery**: preserve ambiguous live
locks created internally by libgit2 after a crash, require operator quiescence
and verified stale-lock handling, then exact same-ID reconciliation. No automatic
foreign/stale-lock deletion or content/age/PID-only ownership inference. Other
owned effects still recover automatically. This separate scoped policy amendment
is recorded in Cycle, design, implementation plan, and ticket checkpoint.

Prepared proposed technical amendment:
[Task 4 resolution protocol](2026-10-08-wave-02-cycle-06-task-04-resolution-protocol-amendment.md).
It incorporates both approved owner policy decisions, stable anchored sentinel,
separate libgit2 serialization without the new SHA-1 dependency, actual-state
recovery, per-member metadata retirement, all-side closure validation, proposed
native APIs, and real process-death validation. Technical implementation approval
remains pending. Native functionality/storage-ordering proofs and full-suite
validation remain unresolved; the policy answers waive neither.

### Technical amendment approved; implementation preflight — 2026-10-08

Owner approved the Task 4 protocol amendment and sequential local implementation.
This supersedes the preceding pending-technical-approval notes; Task 4 itself
is still unaccepted. Independent review and all validation/native gates remain
binding. No delivery, closure, or cleanup authority was added.

Fresh fetch observed main `0b9c7c2695fda0f178ce28fb993ea560929a9222`, which
is not an ancestor of ticket HEAD `6bd1f185b3959375977dcfa1ff6ca22b0aa1d8eb`.
Main adds Wave 3/editor documentation, interim ticket scripts, AGENTS changes,
and Devenv changes, without Rust implementation changes. Preserve the existing
19-path agent-owned Task 4/doc diff in an explicitly unaccepted local WIP
checkpoint before rebasing the ticket onto fetched main. Never stash/reset/clean
or modify the main checkout's unrelated `.superpowers/` directory. Record the
post-rebase accepted-task mapping before dispatching implementation.

Rebase completed without conflicts. The shared `origin/main` ref advanced during
preflight to `ceb1be49477cfec5e14093082d00cf01a5367fb3`; this is the actual
verified ancestor/base of rebased ticket HEAD
`45a139b0d799d4129e5af45688a33938be375e8b`. The intervening commits are
approved Wave 3 ticket relationships/short-code documentation, not Rust source.
All 19 ticket commit subjects/order were preserved. Source/Cargo trees are
identical to the pre-rebase WIP; the only `tests/` change is main's interim-ticket
shell test. Main's unrelated `.superpowers/` remains untouched.

| Checkpoint | Before | After |
| --- | --- | --- |
| Task 0 | `456b509` | `151b0cc` |
| Task 1 through lint remediation | `46fb3cc` | `e125f8c` |
| Task 2 accepted | `50fea1c` | `22d1406` |
| Task 3 accepted; Task 4 whole-change review base | `6bd1f18` | `51fe6f2` |
| Unaccepted Task 4 preservation WIP | `6e07c8e` | `45a139b` |

Full pre-rebase diff, untracked archive, logs and commit mapping are retained at
`/tmp/manyhands-task4-pre-rebase-qK7blK`. Do not treat the WIP as Task 4 acceptance.
Reread rebased AGENTS: rebase direction is now explicitly ticket onto main.
Reviewed changed canonical RFC: editor normalization is desktop-only, not a
headless exact-byte exception; new Wave 3 optional fields remain forward-compatible
unknown metadata for this Cycle. Devenv adds Claude Code tooling and its locked
input, so obtain fresh compile/focused verification before mutation.

Dispatch first bounded redesign milestone as one tightly coupled protocol seam:
owned artifact evidence, stable sentinel, and separate authoritative index
serialization/lifecycle integration. One writer owns the state/reservation/sync
seam and its tests; splitting those across concurrent writers would overlap the
same protocol/files. Later sequential milestones cover remaining path/closure/
metadata/native helpers and real process-death proof. Independent read-only
review follows each milestone; maximum three fix-review rounds before escalating
unresolved feasibility. No Task 5 or checkpoint acceptance until Task 4 gates.

### Milestone 1 timeout recovery — 2026-10-08

Workflow `97d778e9-9e54-405a-800a-9a006acc0c02` failed when writer
`d961bb54-0fc8-4a33-bcf2-020862914b80` reached its 1,800-second deadline.
No dependent reviewer launched and the requested handoff was missing. HEAD and
branch remain `45a139b` on the ticket branch; no staged files. Partial tracked
binary diff, staged diff, untracked archive and status are preserved at
`/tmp/manyhands-task4-m1-timeout-fzxDuy`. No Cargo/rustc process remained in the
ticket cwd; the remaining MCP server was unrelated and was left alone.

The transcript records fresh cargo check and 150 focused remote tests passing,
plus additional focused suites; final command/report attribution and remaining
fmt/clippy status must be confirmed by the retained writer. Do not claim a
full-suite or milestone acceptance from this transcript alone.

Worker was authorized to add only the new evidence table to
`tests/repository_enablement.rs`'s exact schema inventory expectation. It also
identified libgit2's reflog-append-before-ref-install split. This remains a
blocking next-milestone recovery item: uncertain ref/reflog effects must fail
closed without duplicate replay, truncation, or foreign-lock cleanup. The
approved manual exception concerns ambiguous backend locks only, not automatic
reflog repair. No Task 4 acceptance or Task 5 follows from milestone progress.

Same-protocol recovery will resume the exact retained writer only for missing
validation and handoff, with a distinct output binding, then independent review.
No foreground/CLI fallback, extra implementation scope, or replay of accepted
Tasks 0–3 is authorized by the timeout.

### Milestone 1 reviewed; next canonical seam — 2026-10-08

Recovery workflow `7342286e-49ff-44c8-8044-85afe0c97e8a` completed the
retained writer's handoff (`26b19c94-bb80-439b-bd12-85af3d9902d1`) and fresh
independent review (`de57b8fe-5928-49f8-a272-6b71f7caf3bf`). The recovered
writer made no new source changes or duplicate validation runs. Review verdict:
**OK with notes, milestone 1 only**; no delivered-seam issues. This does not
accept the committed unaccepted WIP range or all Task 4.

Artifact directory:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/7342286e-49ff-44c8-8044-85afe0c97e8a/`:
`task4-protocol-m1-recovered-handoff.md` and
`task4-protocol-m1-recovered-review.md`.
Reviewed source diff/status snapshot:
`/tmp/manyhands-task4-m1-reviewed-BJdTZH`. HEAD remains `45a139b`; nothing staged.

Final source validation, after last writer changes, is handoff-attributed:
Devenv all-feature locked check, fmt check, strict all-target/all-feature clippy,
150 remote tests, 9 remote-reservation, 50 recovery-foundation, 73 repository-
enablement tests, and diff checks passed. Reviewer corroborated retained test
logs; it did not rerun commands. Full all-feature suite, CLI smoke and native
runs were not performed for this milestone.

The additive artifact journal proves ownership before stable-sentinel publication
and retains exact private anchors through verified release. Separate libgit2
serialization replaced the custom memfd serializer/direct SHA-1 dependency.
The ref-intent fence safely stops old-HEAD uncertainty without another backend
invocation after operator lock handling. It is deliberately not convergence.

Next bounded milestone 2a owns canonical path-write observation recovery and
all-side closure/comment-immutable validation. Keep stable-lock/ref-intent behavior
unchanged except required wiring. Path results equal to bound caller output
must be observed without rewriting; third values and unsafe paths stop. Validate
invariants against every recorded side; an open base cannot authorize reopening
or override immutable disagreement. Ref/reflog convergence, native helpers,
real process death/storage ordering and final gates remain explicit later work.
Do not broaden operator recovery or claim Task 4 acceptance; review this seam
independently before moving to the next component.

### Milestone 2a reviewed — 2026-10-08

Workflow `3eddd263-1b22-43e7-9d1f-36eaa37c4250` completed writer
`83bd1f32-ae31-4ece-aeb0-548f232a93d5` and fresh reviewer
`d15ee6f9-e67d-49cb-a066-b122d9855d0b`. Verdict: **OK with notes,
milestone 2a only**; no delivered-seam issues. Whole Task 4 remains unaccepted.
Artifacts:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/3eddd263-1b22-43e7-9d1f-36eaa37c4250/task4-canonical-m2a-worker.md`
and adjacent `task4-canonical-m2a-review.md`.

Only sync.rs, sync_tests.rs and repository.rs changed from reviewed milestone 1.
Exact old/result/third-image classification observes already installed bound
results without rewriting. All members validate before any remaining writes.
All recorded sides constrain closure/comment immutables for fresh requests and
retained candidates. A descriptor-bound resolution read rejects unsafe modes;
redundant checkout was removed because it rewrote caller paths. Stable sentinel,
serialization, metadata checks and uncertain-ref replay fencing remain intact.

Handoff and inspected logs show final Devenv check/fmt/strict clippy plus 70 sync,
9 merge and 112 local-authoring tests passing. Six red/green regressions cover
observation faults, no second write/inode-mtime changes, unsafe/third/stale input,
all-side disagreements, exact bytes/unknown keys and legacy candidate rejection.
Reviewer ran no commands. Reviewed source copies/diff/status are preserved at
`/tmp/manyhands-task4-m2a-reviewed-5w6HFi`; HEAD remains `45a139b`, staging empty.

Next milestone 2b is a bounded source-characterized pass on separate ref/reflog
actual-state evidence and safe convergence. Use the approved public locked
backend only; preserve the existing stop fence unless a tested safe path can
prove which effects occurred. No duplicate append, automatic truncation/repair,
custom backend or broadened operator exception is approved. If stock APIs cannot
converge under those constraints, return the exact feasibility gap and owner
decision rather than inventing scope. Native helpers, real process death/storage
ordering, discovery contention and final whole-task/full-suite/CLI/native gates
remain pending. No Task 5, checkpoint acceptance or delivery action yet.

### Milestone 2b diagnosis verified; recovery decision required — 2026-10-08

Workflow `1fe78e41-782d-4673-a7ce-258e1cd9c25e` completed writer
`e8f91cb6-f196-4086-a30e-c9f810dabca7` and fresh reviewer
`e706134b-85d5-42a4-b310-255e0b2e1777`. Verdict: **OK with notes for the
bounded diagnosis/test-only delta; convergence incomplete, decision required**.
Only sync_tests.rs changed from reviewed milestone 2a: seven stock-API tests and
two fixture helpers. No production recovery, schema or dependency changes.
Artifacts:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/1fe78e41-782d-4673-a7ce-258e1cd9c25e/task4-ref-m2b-worker.md`
and adjacent `task4-ref-m2b-review.md`.
Preserved reviewed diff/status: `/tmp/manyhands-task4-m2b-reviewed-7t3NUH`.
HEAD remains unaccepted WIP `45a139b`, staging empty.

Locked libgit2 appends branch then HEAD logs before installing the ref. Public
transaction-supplied reflogs suppress both implicit appends but serialize/replace
whole files; parsing can skip malformed lines and writing normalizes messages.
Public reflog reads also create absent files. Real API tests demonstrate branch
append followed by HEAD error leaves old ref and duplicates branch logging on
stock retry. Source supports partial-write interruption; this is not real process-
death or storage proof. Candidate-HEAD log authentication remains unimplemented.

Both writer and independent reviewer conclude that old ref plus complete branch
append plus partial HEAD append cannot converge under current stock APIs without
duplicate append, prohibited full-log repair/replacement, or additional machinery.
The existing uncertainty fence remains unchanged and preserves actual effects.
The approved stale-backend-lock intervention alone does not resolve this state.
Conditional byte-preserving complete-log replacement is only a subset candidate,
not complete partial-tail recovery and not approved implementation.

Retained logs show seven characterization tests and 163 remote tests passing,
plus Devenv check/fmt/strict clippy and diff/empty-staging checks. Initial fixture/
API expectation failures were corrected; do not label them production red/green.
No full-suite/CLI/native/death gate was run or waived. Pause this seam for an
owner decision: broader operator recovery of interrupted/failed backend ref/log
effects, or separately scoped automatic mutation/backend feasibility work.
Other owned sentinel/path/index replay and all prior safety gates remain required.

### Owner backend-effects exception approved — 2026-10-08

Owner selected **Broaden operator recovery** after independent stock-API diagnosis.
This explicitly supersedes the lock-only exception: interrupted or failed backend
ref/reflog effects, including partial logs and ambiguous locks, may require
operator-controlled verification/repair after quiescing writers. Agents preserve
state and never automatically delete ambiguous locks, normalize/rewrite history,
repair partial tails, or add custom backend machinery. Other owned sentinel/path/
index/metadata recovery remains automatic. Same-operation retry must prove an
allowed recorded baseline or completed-candidate ref/log state and the rest of
its frozen input, not infer authority merely because an operator removed a lock.
Recorded in Cycle, design, implementation plan, protocol amendment and ticket.

Internal completion ruling under this approved boundary: bind fixed branch/HEAD
log roles, frozen normal-update signer/time/message authority and baseline/result
images before a ref invocation. No raw log/signature text or arbitrary paths in
SQLite/journals. Read live images without mutation/no-follow; prepare expensive
read evidence outside the short lease and revalidate. Old ref with original log
images may perform a missing native transition. Candidate plus exact intended
logs may observe completion without reappend. Every partial/mixed/foreign image
or ambiguous lock stops for operator recovery. Supplying whole reflogs for repair
is not authorized. Legacy missing evidence is not invented.

Resume only this bounded production proof/state-classification seam using stock
normal updates with explicit frozen signatures, then independent review. Preserve
reviewed milestones 1/2a and the seven 2b characterization tests. No new dependency,
platform/native waiver, full-suite waiver, Task 4 acceptance or delivery authority.

### OpenCode recovery authorization and evidence — 2026-10-08

Owner selected **Use OpenCode recovery** in the fresh session: authorize replacing
the unavailable Pi resume protocol with sequential OpenCode subagents and
independent reviews, plus an explicitly unaccepted preservation checkpoint before
the current-main rebase. Reconstruct the missing handoff first; approved scope,
policies and acceptance gates remain unchanged. No delivery authority was added.

Read-only recovery agent `ses_ee647b3d4ffeiOLG0Gb4ZOKZ49` inspected the exact
latest writer `dcefc734-637f-432e-8955-783b74ec8206`, its transcript/logs and
retained timeout source against the reviewed milestone 2b baseline. The runner
terminated, canonical session lease was released, required report was missing,
and no independent review launched. Current process-name-only inspection found
no Cargo/rustc/test validation process; it exposed no command lines/environment.

Recovered source implements fixed-role anchored baseline/transition manifests,
frozen signer authority, no-follow branch/HEAD observations, stock explicit-
signature expected-old updates and old-baseline/candidate-result classification.
This inventory is not acceptance; no production defect was conclusively diagnosed
by evidence recovery alone. Seven latest-writer files are identified in the
fresh-session handoff. Preserve earlier milestones and review only this delta
before extending implementation.

Attributed final retained-source evidence: the last `repository::remote` suite
passed 176 tests after final edits and formatting. The interrupted final integration
run passed 112 local-authoring and 50 recovery-foundation tests, but reservation
ended after seven individual successes and enablement was not reached. Earlier
complete integration coverage passed 9 reservation, 73 enablement, 112 authoring
and 50 foundation tests, but predates final sync.rs edits. All-feature check,
fmt check and strict all-target/all-feature clippy passed before those final edits;
they require fresh verification. `/tmp/task4-m2c-remote-final.log` and
`integration-final.log` hold final-run logs; earlier overwritten results survive in
the attributed writer transcript. A combined earlier batch ended with wrapper exit
2 after its Cargo checks, so it is not an entirely successful validation batch.

Fresh fetch observes main `60b0324f3993b32f783fcb98e0f6e24dd9f750dd`, matching
local main. Ticket HEAD before preservation is `45a139b`; it is dirty with
unaccepted Task 4 source/docs. Main adds effective-copy discovery/indexing fixes
in repository.rs/discovery.rs and discovery tests. Main's unrelated `.superpowers/`
is preserved. Inspect and preserve this identified agent-owned diff, then rebase
the ticket onto fetched main and record checkpoint mapping before new source edits.

Preservation checkpoint `f854d1e` was created with explicit owner authorization;
it is unaccepted WIP, not a verified implementation checkpoint. Ticket rebase onto
`60b0324` completed without conflicts. HEAD is now
`409a6f9da1554b9c1646907494bbdc05811d5d23`; main ancestry and clean status passed.
`git range-diff` maps all 20 commits as patch-equivalent. Only the eight upstream
effective-copy source/test/ticket paths differ between preserved and rebased tips;
the seven milestone-2c source files are unchanged. Reread AGENTS and inspected
upstream discovery changes: they alter effective-copy observation/indexing, not
the resolution effect protocol. Refresh/discovery regressions must cover this
upstream behavior when completing Task 4.

| Checkpoint | Before this rebase | Current |
| --- | --- | --- |
| Task 0 | `151b0cc` | `ae77aac` |
| Task 1 through lint remediation | `e125f8c` | `9c353b9` |
| Task 2 accepted | `22d1406` | `fccb2bc` |
| Task 3 accepted; whole-Task-4 review base | `51fe6f2` | `9c7253f` |
| Earlier unaccepted Task 4 WIP | `45a139b` | `b2b81b7` |
| Fresh-session unaccepted preservation | `f854d1e` | `409a6f9` |

Next: fresh milestone-2c specification review against the exact retained
milestone-2b source baseline, then code-quality review. Fresh targeted/static
verification covers the final edits and rebase; no completed tasks are redispatched.

### Milestone 2c recovered, remediated and reviewed — 2026-10-08

Fresh Devenv checks on rebased source passed all-feature locked check, fmt check,
strict all-target/all-feature clippy, 13 `ref_log_` tests, 9 remote-reservation
and 73 repository-enablement tests. This fills retained final-edit verification
gaps without claiming a whole-suite/native pass.

Independent specification reviewer `ses_ee63f4c99ffeL1VawuaRUiby9U` reviewed the
seven-file delta against exact retained milestone-2b source and found two P1s:
refreshed snapshots did not compare the frozen branch digest, and reconciliation
did not reread the live candidate OID after its out-of-lease log observation.
Controller verified both missing gates in source. Worker
`ses_ee6398e7affeYeh5WjZNa3cn3C` changed only sync.rs/sync_tests.rs: shared final
proof validation under the reacquired lease now binds the refreshed branch and
exact live candidate before observation/checkpoint/retirement. Three deterministic
post-backend/pre-refresh tests exercise initial/reconciliation branch substitution
and a third same-tree reconciliation commit. They verify preservation of actual
refs/images/metadata/sentinel and progress, including rejected retries.

Worker evidence: `cargo test --all-features --locked --lib ref_log_final_proof_`
failed all three intended assertions before the fix and passed all three after;
final `ref_log_` passed 16 and `repository::remote::sync::tests::` passed 92.
Final all-feature locked check, fmt check, strict all-target/all-feature clippy
and diff check passed. All Rust commands used Devenv. A transient parallel Devenv
startup missing-file error was followed by successful sequential static checks.

Specification re-review accepted this round-1 remediation with no new blockers.
Fresh code-quality reviewer `ses_ee6338511ffe4aG9Q19YPblNCZ` independently reviewed
the complete milestone delta and locked backend wiring, approving the scoped seam
with no concrete blockers. Neither reviewer reran tests or changed files.
This is milestone-2c approval only: immutable ref/log authority and safe exact-
state completion are reviewed; whole Task 4 remains unaccepted and no accepted
Task-4 checkpoint commit was taken. Native helpers, real child death/storage
ordering, discovery contention and final whole-task/full-suite/CLI/native gates
remain. Continue sequentially to the approved native-helper/proof seam; do not
begin Task 5 or infer delivery authority.

### Owner robustness ruling — 2026-10-08

Owner stated: **"Best efforts on robustness, but it does not need to be perfect."**
Apply this ruling to ongoing work and review: prefer proportionate fixes for
demonstrated failures, meaningful representative recovery tests and candid evidence
limits over pursuing perfect robustness or exhaustive every-syscall/power-loss
coverage. Do not reopen settled architecture or add machinery solely to eliminate
all theoretical failure windows. Carry remaining robustness limitations explicitly
in review rather than treating every imperfection as an automatic blocker.
Functional platform requirements and factual verification claims remain distinct
from a promise of perfect robustness. No delivery/lifecycle authority is added.

### Linux child-death proof reviewed — 2026-10-08

Worker `ses_ee629d5b5ffeHX0EX3ernRfWLm` changed sync.rs/sync_tests.rs only,
adding real filtered unit-test child processes, parent-issued kill/wait with
SIGKILL verification, fresh-process exact-request replay, and 21 kill boundaries:
sentinel publication before/after directory barrier, canonical installation,
scratch serialization/sync/output identity, index link/rename/install, actual
backend lock/ref intent/commit, each of three metadata members before/after its
barrier, and sentinel release before unlink/after unlink/after barrier. Three
foreign-state controls and refusal preservation/privacy/lease checks are included.
These are actual death without Drop, not panic/catch_unwind simulations.

Specification reviewer `ses_ee61c8e96ffeP60e52g1LxgKMf` found one P1: a child
timeout could panic/unwind before a later parent SIGKILL. Worker replaced it with
`libc::_exit(86)` and added a 20ms timeout/delayed-parent Drop-canary regression,
red before the fix and green afterward. Normal kills require SIGKILL and no Drop
canary; timeout status cannot qualify. Refusal snapshots now include bytes and
device/inode/presence for canonical/index/logs/all metadata/sentinel/anchor/backend
locks and HEAD, addressing a nonblocking coverage note. Spec re-review accepted
round 1. Fresh quality reviewer `ses_ee61324b3ffejlgT2JiazoQDfV` approved with no
important concrete bugs under the owner best-effort ruling.

Worker-attributed Devenv evidence: final `cargo test --all-features --locked --lib
resolution_process_death -- --test-threads=1` passed 6; preceding full sync passed
97 and ref-log 16. Final all-feature check, fmt and strict all-target/all-feature
clippy plus diff check passed. Native/internal backend-append kills and power-loss
durability are not inferred. Privacy scan skips absent/unreadable files, retained
as a nonblocking evidence limitation rather than perfect-coverage requirement.

### Discovery contention diagnosed, corrected and reviewed — 2026-10-08

Controller's fresh all-feature discovery suite failed 3 of 67 tests with
`RepositoryBusy`: concurrent refresh, same-service corrupt rebuild and two-service
corrupt rebuild. Worker `ses_ee629d5c3ffe2VQMwiyWCqz7IS` traced the bounded 250ms
Git/cache lease acquisition and unchanged main orchestration. Controlled 300ms
fsync delay reproduced all three failures even with serial test-runner execution;
holders continued and released leases, so leaked locks/deadlocks were not needed.
The pre-existing tests incorrectly required unconditional success despite bounded
contention. No production timing/retry change was justified.

Same worker changed only tests/discovery_rebuild.rs: join both callers, accept
only RepositoryBusy and replay after explicit completion with original IDs;
keep refresh owner paused until contender returns; retain no-scan/single-epoch,
single completed record and one corrupt diagnostic assertions. Two deterministic
held Git/cache lease regressions verify bounded Busy then exact-ID replay after
release. No extra sleeps or deadline widening. Controlled fsync run turned green
(3 passed, 57.52s), and full discovery suite passed 69 tests (7.68s), followed by
Devenv check/fmt/strict clippy and diff check. Independent spec reviewer
`ses_ee60d0b51ffehcj9dpgQuDGKTi` and quality reviewer
`ses_ee60c6cf5ffe130wALWrnnq3M2` approved the actual test-only diff, no blockers.

### Shared Unix native-helper implementation; owner pause — 2026-10-08

Read-only inventory `ses_ee62ec651ffedKlKVvs87md0yc` distinguished missing native
implementation from demonstrated feasibility failure. Worker
`ses_ee60ad283ffet61HIC7U2OlkIL` then completed a coherent shared Linux/macOS seam:
new private src/repository/native_resolution.rs pins Unix root/ancestor traversal;
repository.rs adds pinned/nonblocking reads and macOS exclusive/swap rename APIs;
sync.rs shares Unix lock/ref/log proof/recovery paths, replaces /proc serialization
assumptions with validated private paths, and adds ordinary loose-object barriers
before candidate journaling plus ref/log file/directory barriers before observed.
sync_tests.rs adds helper regressions and applicable Unix gates. No Git config or
global libgit2 option was changed. This new seam has NOT received independent review.

Worker reports these final Linux commands passed, all through Devenv:

```sh
devenv shell -- cargo test --locked --lib repository::remote::sync::tests
# 103 passed, including process-death coverage
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```

The full library/integration/SSH/doc-test suite and CLI smoke are worker-attributed
passes, not native macOS/Windows evidence. Do not rerun the unchanged full suite
merely because a new session starts. macOS target libraries are absent; no macOS
compile/runtime, Windows or Linux ARM result is claimed. Packed/alternate object
storage still depends on backend fsync policy; macOS full device-cache flushing
and exhaustive power-loss durability are unproved, carried under best-effort ruling.

Owner requested: **"Pause after the current subagent finishes. We'll finish up in
the morning."** The native writer returned; execution is paused. No follow-on
reviewer, writer or validation job was launched. Narrow process-name inspection
found no cargo/rustc/rustfmt/clippy-driver process; it did not inspect command lines
or environments or claim absence of every possible unrelated process.

HEAD remains `409a6f9da1554b9c1646907494bbdc05811d5d23`, accepted Task 3 review
base `9c7253f`, last rebased main `60b0324`. Empty staging and the exact ticket
branch were verified; tracked dirty paths are repository.rs, remote sync.rs/tests,
discovery_rebuild.rs, this ledger/handoff and canonical comment 01M4C900. New
native_resolution.rs is untracked and must be preserved. Diff check passed.
No accepted Task-4 commit, push/PR/CI dispatch, merge, closure or cleanup occurred.

Resume first with independent specification then code-quality review of the Unix
native seam, preserving earlier reviewed m2c/death/discovery work. Then implement
Windows retained-ancestor/reparse-safe reads, volume/file identity and anchors,
absent-only hard-link publication, native output install/metadata retirement/
verified release, and ref/log proof/serialization/barriers. Native evidence remains
pending separately. Obtain final whole-Task-4 review against `9c7253f` before its
accepted checkpoint and Task 5. Tasks 5–7 have not begun; ticket stays open.

### Milestone 2c timeout; fresh-session handoff saved — 2026-10-08

Workflow `c65ad8a8-a967-4447-89c9-38336033372e` failed because retained writer
`dcefc734-637f-432e-8955-783b74ec8206` exceeded 1,800,000 ms. Process terminal
observed; requested handoff missing; independent reviewer never launched. Seven
owned source/inventory files changed. No milestone2c validation or correctness
claim is attributed/accepted from this failure.

Partial diff, staging/status, exact seven-file copies and scoped untracked archive
are preserved at `/tmp/manyhands-task4-m2c-timeout-f20R2v`. Branch/HEAD match
expected ticket/45a139b; staging empty. Preliminary source inventory locates a new
ref-log-artifact table and snapshot/manifest/context helpers; this is not review.
No Cargo/Rust process found with ticket cwd in a narrow names-only check; this
is not proof about all detached jobs. Unrelated MCP processes left untouched.

Owner advised targeted tests and a longer timeout: full suites alone may take
15–20 minutes or more. Next combined implementation/validation run should budget
longer (e.g. 90 minutes), checkpoint before long checks, and separate broad final
validation where appropriate. Do not duplicate already attributed gates or waive
mandatory final checks.

Owner then requested a saved handoff for a fresh session. Saved
`docs/plans/2026-10-08-wave-02-cycle-06-task-04-handoff.md` with all approvals,
latest run/session IDs, exact baselines/snapshots, remaining gates and recovery
sequence. Work is paused. No resume, source fixes, validation, commit or delivery
was launched after that request. Task4 unaccepted; Task5 not begun.

### Owner resume and current-main preflight — 2026-10-08

Owner requested **"Please continue"**. Resume sequential OpenCode implementation
and independent review under the recorded best-effort robustness ruling. Reuse
the existing ticket/worktree and preserve the last writer's unreviewed Unix-native
seam; earlier m2c/death/discovery approvals remain valid.

Fresh fetch observes main `60b0324f3993b32f783fcb98e0f6e24dd9f750dd`, unchanged
from the pause and matching local main. Ticket starts at `409a6f9` with the recorded
eight dirty/untracked paths; no operation/conflict is in progress. Main's unrelated
`.superpowers/` remains untouched. Identified source/test/doc changes are preserved
in an explicitly unaccepted WIP snapshot for clean no-op rebase preflight, using
the approved preservation authority. This is not Task 4 acceptance. Pending first
step is independent specification then quality review of the native-helper seam.

Unaccepted preservation checkpoint is `fb6d234d31c958275e6c24ddae6686f2eb5776ff`.
Rebase onto freshly fetched `60b0324` was a clean no-op; ancestry and clean status
passed. Accepted Task 3 remains `9c7253f`; earlier checkpoints need no remapping.
The reviewed source tree from the last writer is unchanged by preflight. Do not
repeat the unchanged Linux full suite solely to resume; seek the pending review.

### Shared Unix seam reviewed; Windows entry — 2026-10-08

Specification reviewer `ses_ee4e19292ffefcpB6WZ5NFyeoz` found a fixture portability
defect: macOS /var aliases could mismatch canonical libgit2 workdirs and lexical
hook keys, or make ancestor-refusal tests reject the system alias rather than the
intended fixture mutation. Worker `ses_ee60ad283ffet61HIC7U2OlkIL` changed only
sync_tests.rs: canonicalize temporary parents before fixture creation, add a
deliberate alias-parent regression and hook-fired/positive-read attestations.
Production strict no-follow traversal stays unchanged.

Worker demonstrated the new alias regression red before the fix, then 6 native
and 104 sync tests green, plus Devenv all-feature check, fmt, strict all-target/
all-feature clippy and diff check. Spec re-review approved round 1, no remaining
causal blocker. Fresh code-quality reviewer `ses_ee4d77b4affeFtOWpMkasGZQJs` approved
the actual Unix-helper source/wiring and fixture fix under the best-effort ruling.
Native macOS evidence remains pending; the documented storage limits stand.

Continue one bounded Windows implementation writer, with the reviewed Unix source
at fb6d234 as its source baseline and the uncommitted alias test fix preserved.
No whole-Task-4 acceptance/checkpoint or delivery authority follows from this seam.

### Windows primitive checkpoint reviewed — 2026-10-08

Worker `ses_ee4d41ecaffe7LUs65TgfgGmAY` completed a lower-level Windows checkpoint,
not the full public resolution port: retained ancestor/no-reparse handles, path
policy, volume/file identity and stable stamps, absent-only hard-link publication,
matching-image retirement, same-volume guarded replacement, and repository owned
read/replacement dispatch. New private windows.rs/windows_path.rs/windows_tests.rs
live under native_resolution/. remote/sync.rs remains unchanged and Windows still
refuses at ResolutionIndexLock acquisition until the next wiring seam.

Specification reviewer `ses_ee4bdc9e1ffeW1dSGWyeUAowLZ` found a concrete hard-link
retirement defect: disposition through an ownership image opened by the anchor
name could delete that anchor instead of the requested sentinel name. Fixed to
open/verify and retire the requested target handle, close handles then prove
absence; a native regression preserves the anchor while retiring index.lock.
Spec re-review approved. Quality reviewer `ses_ee4b65b55ffeMR7JlUYeL58NGQ` found
ordinary readers unnecessarily requested DELETE access, incompatible with stock
libgit2 readers omitting delete sharing. Fixed ordinary images to request read/
write only, with DELETE limited to the fresh retirement target; three native
sharing regressions cover existing and retained images. Quality re-review approved.

Host-runnable Windows path policy demonstrated red/green (2 tests). Linux 104 sync
tests, required check/fmt/strict clippy, full all-feature suite (281 library tests
plus integration/SSH/doc runners) and CLI passed before Windows-only review fixes.
Final fixes passed host path tests, fmt/check/strict clippy and diff check without
repeating the unaffected broad suite. Native primitive/integration regressions
were added without ignore/skip conditions but are unrun: Windows cross-check
failed E0463 because target core/standard libraries are unavailable. Static API
review used the actual direct locked windows-sys 0.61.2; no new dependency.

Proceed to journal-backed Windows sentinel/index/metadata lifecycle and shared
ref/log proof wiring. Native compile/runtime, sharing execution and directory-
entry durability evidence remain factual gaps; best-effort ruling stands.

### Windows wiring and whole-Task-4 review — 2026-10-08

Worker `ses_ee4aecf58ffeZX1DI4HQaBFYh7` completed Windows journal-backed
baseline/sentinel/output acquisition, private stock-index serialization and native
installation, per-member metadata retirement, release reconciliation and shared
ref/log authority/final-proof wiring. Added private remote/windows_resolution.rs
and native installation/replay/privacy/foreign-state tests without ignore/skip.
Specification reviewer `ses_ee491823effeh8VmYuZ5vVf28m` and quality reviewer
`ses_ee48dc0ecffeUt63xELVCC6mrx` approved this source seam. Linux full/static/CLI
passes were attributed; Windows target-core E0463 still prevented native checking.

Whole-Task-4 spec reviewer `ses_ee47ce0a8ffe05X0GvViQT5x8r`, against accepted
Task 3 base 9c7253f, found four cross-seam gaps missed by bounded reviews: legitimate
clean merge entries rejected as outside-token dirt; active parent reservation not
released after local completion; refresh-required not marked; unrelated baseline
relationship diagnostics globally blocking resolution. A minor generic-subject
contract mismatch was also identified. Worker `ses_ee47466f3ffewtsPSphLrPYahf`
added regression-first fixes in remote sync/state/reservation and tests: separate
worktree dirt from recorded HEAD-to-index merge entries, fenced local finalization
to Interrupted/reconciliation_required after released sentinel, atomic refresh
invalidation with checkpoint, affected/baseline-scoped validation, and approved
subjects for new candidates only. Frozen candidates and publication authority
remain unchanged; SQL failures roll back observation and exact retry reuses effects.

Spec round 1 closed those findings but found one restart regression: Applied/released
resolution could enter the generic old-HEAD fast-forward path. Round 2 now branches
on phase before mutation: Applied requires exact candidate/result/tree, old/third
HEAD refuses without Git effects; Applying retains accepted Task 3 recovery.
Red/green regressions cover old/third restored HEAD, clean merged code, local
authoring after release, refresh/finalization faults and unrelated old diagnostics.
Spec round 2 approved with no remaining P1/P2. Fresh whole-task quality reviewer
`ses_ee440831effe0IgrVSKlX7bdb3` approved the complete actual tree with no important
causal findings under the best-effort ruling. Native gates remain pending.

Final worker-attributed Linux verification on the amended Rust tree passed Devenv
all-feature locked check, fmt check, strict all-target/all-feature clippy, full
all-feature locked tests (292 library tests plus all integration/SSH/doc suites),
and CLI smoke. Focused candidate 19 and sync 115 also pass. No unchanged-tree
broad rerun is needed for verification-checkpoint bookkeeping.

### Native CI publication authority and preparation — 2026-10-08

Owner authorized push for manual CI, noting previous macOS/Windows failures and
disabled automatic triggers. Workflow remains workflow_dispatch-only; build jobs
remain Linux x86_64/aarch64, Windows x86_64/aarch64 MSVC and macOS ARM64. Inspect
actual run results rather than inferring platform success. Added affected Task-4
reservation/foundation/enablement/authoring/discovery integration tests to the
existing native headless command, preserving all prior credential/SSH tests and
release artifacts. Native-runner direct Cargo uses the existing AGENTS exception.

Prior run 37475030123 built release binaries on all five targets but failed the
test step on both Windows jobs and macOS; Linux jobs passed. Another ticket's
manual run 37777664096 is unrelated evidence and must not be modified or counted.

Fresh ticket remote is 2dbd3b4, its sole local-missing commit being the original
ticket checkpoint. Its entire patch creates this ticket, identical to rebased
checkpoint 18aced7; current ticket retains its subsequent approved evolution.
Publish without force: preserve that equivalent checkpoint by an explicit
history-only merge and verify the reviewed file tree is unchanged. A moved remote
must be inspected again, not overwritten. Verification checkpoint remains
unaccepted Task-4 work until native results and remaining gates are handled.
No PR, merge, ticket closure or cleanup authority was added.

### Previous native fixture failure corrected before dispatch — 2026-10-08

Downloaded failed logs for run 37475030123. All 25 state/reservation unit failures
on macOS and Windows stem from RepositoryNotRegistered in fixtures: raw tempdir
paths were manually inserted while production canonicalizes before registry lookup.
Current fixture source retained that mismatch. Worker
`ses_ee4396fb5ffeVmkjClsqg4L9Vi` changed only state_tests.rs/reservation_tests.rs:
canonicalize explicit fixture parents before tempdir creation, preserving production
canonical guards. Linux deliberate alias-parent regressions first failed with the
same RepositoryNotRegistered, then passed; existing sync alias control also passed
(3 total), state 28 and reservation 28 plus required static checks/diff passed.
No global TMPDIR mutation, ignore/skip or production workaround was added.
Independent spec `ses_ee435ef12ffeU3OkR6KWbAibem` and quality
`ses_ee434d1aeffeA37gdLoS2eYoM3` approved this narrowly scoped test correction.
Native success remains unproved until the new matrix actually runs. Rust production
matches the whole-task reviewed/full-tested tree; this extra test-only correction
does not justify repeating unchanged broad validation before publication.

### Verification checkpoint published; native matrix running — 2026-10-08

Created authorized verification checkpoint `d0a49897e06b43d1ad725a9c7c1dce12639b4f8b`.
History-only reconciliation with the sole equivalent remote checkpoint produced
`a9593a0d1b1ab27513b7abc34f6b9a422a29b41c`. Reviewed tree identity remained
`f7f874aca83eda77693f1e9a5853963144f5d847`; full tree diff from d0a4989 is empty,
and original remote 2dbd3b4 is an ancestor. Normal non-force push advanced the
existing ticket remote to a9593a0, preserving all remote history.

Manually dispatched workflow build.yml on that ticket branch:
https://github.com/vhodges/manyhands/actions/runs/37788386532
This run must be inspected on exact SHA a9593a0; pending jobs are not passes.
Task 4 remains unaccepted and Tasks 5–7 have not begun. Ticket stays open; no PR,
main merge, closure or cleanup occurred. Source is unchanged by bookkeeping.

### Native run 1 failures and reviewed fixes — 2026-10-08

Run 37788386532 finished failed on all five jobs. Windows release compilation
stopped at E0308 (Index::open received PathBuf instead of &Path); macOS built
successfully and passed 286 library tests, failing one secondary raw-path fixture;
Linux built and passed library tests, then failed four recovery-foundation creation
cases. Early failures mean later integration/platform behavior was not verified.
Failed logs are retained at /tmp/opencode/cycle06-native-37788386532-failures.log.

Worker `ses_ee42ad3aaffejfL5vNnKwsZjFP` borrowed the Windows index pathname and
fixed the remaining secondary reservation fixture's canonical parent. The new
Unix alias regression reproduced RepositoryNotRegistered before the fix, then
reservation 29 and integration reservation 9 plus static checks passed. Its broad
suite attempts hit 120s/600s shell limits; partial 295-library/integration results
were not a full pass. No cargo/rustc process names remained in the subsequent check.

Read-only diagnosis, then writer `ses_ee42ad399ffeL9qT7imV0tlMXF`, proved the Linux
root cause with isolated HOME/default-branch runs: libgit2 is_empty is branch-default
dependent, so actual unborn HEAD=main is misclassified when the effective default
is master/unset. Developer global default main masked this pre-existing production
bug. CI was not configured to hide it. Enablement now classifies actual HEAD,
propagates non-UnbornBranch errors, and rejects a pre-existing requested primary
before mutation/rollback can delete its history. Deterministic local-default tests
first failed main/trunk behavior; classification alone exposed an existing-ref
rollback deletion; the early guard then passed all three regressions. Foundation
unborn assertions now check symbolic HEAD, UnbornBranch and absent refs directly.
Two existing rollback fixtures receive local identities instead of developer config.

Isolated master and main configurations each passed foundation 50 and enablement
76 tests. Final required Devenv check/fmt/strict clippy/full all-feature suite and
CLI completed successfully with an appropriate 40-minute suite budget; all SSH
15 observation/41 synchronization/31 fixture/103 transport and doc tests finished.
Independent spec `ses_ee406346cffe2JVWc1SQp12K7z` and quality
`ses_ee4048c90ffevjaEILObiSP4mK` approved this actual CI-fix set. Native re-run is
still required; no Task 4 acceptance follows from the local correction.

Published reviewed CI fixes as d108e73c5428a51e9c7be9036f241ceb27150871 through
normal push; manually dispatched second native run on that exact source:
https://github.com/vhodges/manyhands/actions/runs/37795672482
Inspect actual five-target results, including Windows tests previously hidden by
the compile error. No native pass or Task 4 acceptance is inferred while pending.

### Native run 2: builds pass, integration fixtures corrected — 2026-10-08

Run 37795672482 built and verified release binaries on all five targets. macOS
library tests passed, but discovery had five raw/canonical fixture mismatches;
Windows test compilation stopped on four unguarded Unix symlink calls in discovery;
Linux library/foundation passed, but enablement had one vendor-template literal
mismatch. These later failures were previously hidden by earlier build/test stops.
Logs: /tmp/opencode/cycle06-native-37795672482-failures.log. No successful whole
native job is inferred from successful release builds.

Worker `ses_ee3f2e600ffeTgeJNNL4hfIkn6` changed only discovery_rebuild.rs and
repository_enablement.rs. Portable file/directory symlink helpers now use proper
Unix/Windows APIs and fail honestly on fixture capability errors, without skips.
Canonical registry/trigger/context/race keys retain alias requests; five deliberate
alias regressions reproduced the macOS failure modes and now pass while preserving
cache-only/nonmutation/corruption/cascade/multi-root/race assertions. An independent
pre-application init_opts probe captures the installed Git template, so recovery
asserts full raw template preservation plus one owned rule rather than Nix-only
comment text. Both creation cases retain cleanup/commit/config/registration checks.
Shared support and production are unchanged.

Headless and all-feature discovery 74 and enablement 76 pass, with required static
checks and diff check. Independent spec `ses_ee3eccb60ffeHYOQh7Frnmcw6H` and
quality `ses_ee3eb2572ffe9uUM6fdmsYylWK` approved the bounded fixes. Next publish
and run the same complete native coverage; Windows runtime remains unexecuted
because compilation of the newly included discovery target previously failed.

Published the reviewed test-only portability fixes as
c3ad08c83059a53635f7411021e7f0705d87783a through normal push. Third complete
native matrix is manually dispatched on that source:
https://github.com/vhodges/manyhands/actions/runs/37799832943
Continue inspecting actual results; no source/coverage gate is waived.

### Native run 3: Linux green; Windows runtime and macOS authoring fixes — 2026-10-08

Run 37799832943 passed complete native build/test/artifact jobs on Linux x86_64
and ARM64. macOS reached local-authoring (103 passed/9 failed): eight alias-path
assertions and one APFS-invalid-byte fixture constructor EILSEQ. Both Windows
jobs built and executed library tests (217 passed/43 failed): two primitive
existing-destination replacements returned AccessDenied, most resolution cases
stopped before candidate creation; separate LF assumptions and native separators
in canonical collection caused checkout/context failures. Logs are retained at
/tmp/opencode/cycle06-native-37799832943-failures.log.

Read-only diagnosis `ses_ee3cf87dbffes8BMpj5oJswb0T` traced classic MoveFileEx
open-destination replacement semantics, fixture checkout filtering and component
path representation. It rejected an unproved verbatim-prefix hypothesis: relevant
production registration guards already canonicalize. Worker
`ses_ee39ea6f6ffeYoUXGSdUTZNlDd` adds one verified requested-source DELETE-handle
FileRenameInfoEx helper (POSIX replacement for existing targets, absent-only flags
otherwise), retaining parent/proof handles and exact pre/post identity/bytes.
Durable anchors survive; unsupported/ACL/readonly errors propagate with no classic
fallback or readonly override. Five native controls include alternate-anchor source
proof, non-BMP names, old-target readability, identical-byte substitution, absence
and readonly refusal. Existing windows-sys WindowsProgramming feature supplies named
flags; no new dependency or lockfile change.

The small canonical collector fix joins native OsStr components with '/', without
lossy conversion or replacing literal Unix backslashes. Fixture-local autocrlf=false
isolates LF assumptions; separate tests verify production CRLF checkout and exact
LF/CRLF caller bytes in both resolved worktree and blob. Root/path guards remain.

Worker `ses_ee3cf87caffeGduWuBXPVTLL38` changed only local_authoring.rs: eight
alias regressions first failed old expectations, then canonical expectations pass
while alias requests and preservation/redaction checks stay. Exact macOS EILSEQ
constructor refusal verifies all protected state unchanged and explicitly proves
filesystem rejection, not unreachable app-guard coverage; filesystems supporting
invalid names still execute the original application rejection. No ignored case.

Final Devenv sync 120/path 2/authoring 120, check/fmt/strict clippy, full all-feature
suite (300 library tests and complete integration/SSH/doc suites) and CLI pass.
Spec `ses_ee2e5b1b1ffeql3VtSxhSPsWJc` and quality
`ses_ee2e1e5f0ffe12GBg2EOGkzsRq` approved this actual correction diff, conditional
on native results. FileRenameInfoEx and APFS branch execution still need the next
matrix; local success is not native Windows/macOS success. Task 4 remains unaccepted.

Published those reviewed corrections as 4776157b779ba233f03f22eebfb4c9c7e0df1790
through normal push; fourth native matrix is manually running:
https://github.com/vhodges/manyhands/actions/runs/37837512832
Inspect the exact source results, including all five new Windows controls and
the macOS authoring cases. No pending result accepts Task 4.

### Native run 4: runtime rename verified; remaining representation fixes — 2026-10-08

Run 37837512832 again fully passed both Linux jobs. New Windows rename controls
and most resolution cases passed on both architectures (261 library pass/8 fail).
Remaining failures were six missed fault callbacks and two post-resolution own-
authoring checks. macOS moved past authoring and failed two foundation fixtures:
an absent-root pending-create key and one replay-path expectation. Logs reside at
/tmp/opencode/cycle06-native-37837512832-failures.log.

Worker `ses_ee2c191f0ffeduZNiUrvKoIfMp` verified all Windows callback sites exist;
registration used raw fixture spelling while dispatch used paths from canonical
repository opens. Test-only registration/dispatch now share best-effort canonical
identity with identical raw fallback. Alias regressions reproduced missed stale/
panic callbacks and verify execution, independence, absence fallback and actual
seven-stage order; old native fault cases explicitly attest callbacks.

The production own-worktree guard compared canonical intended paths with raw
registration metadata. It now compares existing canonical locations while requiring
the registered spelling's chain to contain only real directories, no symlinks or
Windows reparse points. Different/missing/both-missing controls refuse. An initial
canonical-only version failed the symlink control; strict final guard passes and
retains branch/item/kind/common-root validations and pre-effect placement.

Foundation test keys now use canonical existing parent plus absent leaf, preserving
alias API input and root/cache nonmutation. A new alias case first reproduced the
incorrect creation and now refuses; full replay expects canonical path only.
No Windows rename rewrite or production creation-guard workaround was added.

Devenv sync 127, authoring 120, foundation 51, SSH synchronization 41, static gates,
CLI and one complete long-budget all-feature run pass (766 unit/integration tests,
9 doctests and 190 SSH cases). Independent spec
`ses_ee2a01fd1ffeX1ZAtffu3DfPqH` and quality
`ses_ee29de16affeK4RCHkRyovADnL` approve this actual diff. New native execution
must confirm the remaining callbacks and authoring/foundation cases; Task 4 is
not yet accepted. The log did not print both Windows hook keys, so root spelling
cause is source/regression evidence, not a claim of a logged native key-pair dump.

Published that reviewed fix as 7b897cf7fc2e9784ca8addf8e3646b36c2f33970 through
normal push; fifth complete native matrix is manually dispatched:
https://github.com/vhodges/manyhands/actions/runs/37846555078
This is the current exact-source gate. Task 4 remains unaccepted while pending.

### Native run 5: core cases pass; remaining fixture observations reconciled — 2026-10-08

Run 37846555078 fully passed both Linux jobs. Original Windows callback and
post-resolution authoring failures are resolved; two newly added metadata fixtures
failed before exercising the production guard, and Windows x86 also hit a pin-
approval loser expectation. macOS progressed through authoring/foundation and
stopped at the SSH noncolliding control's receiver-ledger equality (observation
2011). Logs: /tmp/opencode/cycle06-native-37846555078-failures.log.

Worker `ses_ee281dd1bffe0lqErPRgT3X7QO` confirmed pinned libgit2 worktree metadata
parsing scans '/' while fixture display paths used backslashes. Test-only Git
path encoding preserves Windows drive/UNC roots and Unix literal backslashes;
positive physical/noncanonical attestations and four negative parser-location/
exact-error/Git-preservation checks remain. Production guard is unchanged.

Read-only diagnosis then writer `ses_ee281dd0affeiwyy7VDDSitMTH` proved receiver
publication occurs after successful fixture push return. A gated authentic fixture
reproduced the failing early ledger baseline with no actual synchronization write.
Successful changing setup/peer pushes now wait for exact accepted post-cursor
receipts (ref, old/new OIDs) before returning. Condvar/gate cases cover delayed and
duplicate publication, no-op and rejection, retaining unchanged-ledger/ref/index/
ignored-byte assertions and privacy-harness isolation. No arbitrary sleep was added.

The Windows approval failure was zero HostTrustChanged losers, not zero winners.
Forced 400ms fsync reproduced legitimate bounded cache-lease RegistryUnavailable
before CAS. Tests still require exactly one original success; after both joins,
unchanged original winner intent succeeds idempotently and loser returns exact
HostTrustChanged, with one surviving winning pin across reopen. Held-guard coverage
proves refusal/release reconciliation; production trust/deadlines are unchanged.

Quality reviewer `ses_ee2564b63ffeFBrNrsu54zLVO2` found a controller failure could
detach the worker owning the SSH fixture. Scoped threads now cancel the hold before
joining on error/unwind and release before success join; two authentic teardown
controls failed before and pass after, proving fixture roots removed before return.
Spec `ses_ee258ebf5ffeB1B5R336jKaToz` and quality re-review approve the full bounded
five-file test/helper diff. Before cleanup-only amendment, complete Devenv full
verification passed 970 tests/cases including 194 SSH; final amended synchronization
47 SSH cases and required static checks pass. No unaffected broad rerun or production
workaround. Exact next native matrix must confirm these corrections; Task 4 pending.

Published the reviewed test/helper corrections as
7123adee42c6714ccdf2995c89fcf2fa60d0f090 through normal push; sixth full native run:
https://github.com/vhodges/manyhands/actions/runs/37856496873
Continue exact-source result inspection before Task 4 acceptance.

### Native run 6: core and SSH controls pass; diagnostic persistence corrected

Run 37856496873 passed both Linux jobs. Windows library/core and metadata fixtures
now pass; discovery reaches 72 pass/2 fail because valid native diagnostic paths
were stored with backslashes then rejected by strict snapshot decoding. macOS
SSH synchronization passes all 47 cases and reaches five enablement fixture lookup/
error-root representation failures. Logs: /tmp/opencode/cycle06-native-37856496873-failures.log.

Worker `ses_ee22d9386ffeJTbaIjJMipr6D2` changes only repository.rs diagnostic path
persistence plus discovery/enablement tests. Component-wise '/' joining fixes
Windows paths without lossy text or rewriting literal Unix backslashes. Strict
cached-path decoding is unchanged. Spec reviewer `ses_ee215eb43ffeR1nyFrGVmRsxCV`
found an initial overreach rejecting unrepresentable diagnostic paths, blocking
readable-context discovery. Restored nullable diagnostic semantics: path NULL,
owning context/code/guidance/time retained. A regression first reproduced root
persistence failure, then proves refresh/rebuild/snapshot/replay keep the diagnostic
visible with readable primary and active items, without canonical/Git mutation.

Mac fixture lookups use canonical existing roots, or canonical parent plus absent
leaf for creation targets; requests remain aliased. Five symlink-parent scenarios
exercise pending fields, rollback and registration/cleanup invariants. Decoder
negatives for corrupt backslash/absolute/traversal rows still reject without repair.
Spec re-review and quality `ses_ee2005613ffe5YKiGh50fx2mNI` approve the corrected
three-file scope. Final focused headless/all-feature discovery 77/enablement 77,
static gates, full Devenv suite (780 Rust-harness tests including docs plus 196 SSH)
and CLI pass. Actual next native execution remains required; Task 4 unaccepted.

Published these approved corrections as bed07e47ee59d64c20b4d5c4da9d151ed9ed3fb9
through normal push; seventh full native matrix is running:
https://github.com/vhodges/manyhands/actions/runs/37863696730
Inspect exact-source results before Task 4 acceptance.

### Owner CI-first verification ruling — 2026-10-09

Owner requested running fixes in CI rather than duplicating local and CI work:
**"Only run them locally if you get a regression in the Linux CI run."** This is
the explicit verification exception for continued Cycle work: use source review
then authorized manual native CI, and launch local Rust verification/reproduction
only when Linux CI regresses. Preserve all native coverage and factual evidence;
do not repeat local suites just because Windows/macOS fixes changed a tree.
The latest worker had completed its local commands before this steering arrived.
No additional local verification will be launched for that unchanged correction.

### Native run 9: final transport fixture preservation controls — 2026-10-09

Run 37872797196 fully passed Linux x86_64/ARM64; Windows and macOS reached the
last transport controls. Worker `ses_ee179f036ffeKnrpaJumd77eLq` reproduced the
Windows context-identity check 540 before the second production call: raw native
gitdir text again differed from Git-format metadata. Parser-correct encoding and
physical decoy attestation retain mismatch, protected-state and no-auth predicates.
The observation fixture's alias-root manual registry insert independently reproduced
RepositoryNotRegistered and now uses canonical keys, retaining alias requests,
complete/replay/snapshot/helper/prompt/private-key invariants. Exact macOS native
failure was not exposed by its filtered child capture, so fixed numeric diagnostics
were added; no claim that the local alias reproduction proves that native cause.

The worker had completed selected regressions and local static/full verification
before the CI-first steering arrived. Read-only spec
`ses_ee16485e4ffeUkQNbfl1pbodWq` and quality
`ses_ee163148fffehVG2r9qm3MGa3b` approve the two-file test-only correction. No
further local Rust command is required without a Linux CI regression. Publish
and inspect native results, including numeric-only diagnostics if the Mac case
still fails. Native gate is not waived; Task 4 remains unaccepted.

### Native run 8: remaining legacy fixture spelling and phase watchdog — 2026-10-09

Run 37868930176 again fully passed both Linux jobs. Windows reaches legacy
foundation assertions: four expected descendant strings mixed canonical Windows
roots with '/' suffixes, and the failed-child check assumed POSIX ExitStatus text.
macOS had one registry-open AfterWal notification exceed the one-second test wait.
Logs: /tmp/opencode/cycle06-native-37868930176-failures.log.

Worker `ses_ee1af6bdeffegc0tqlCc4VKgCE` changes only support/mod.rs,
recovery_foundation_gate.rs and repository_enablement.rs. Native PathBuf joins retain
exact legacy rows/schema/reset/privacy checks. Failed-child assertions use fixed
early-exit category, exact code 101, captured streams and structured NotFound rather
than platform display wording. Registry test uses one ten-second test watchdog,
preserving phase ordering, locked-window assertions and connection release before
join/assertions; controlled 1.1s notification hold reproduces the former timeout.
Production SQLite five-second busy timeout and leases stay unchanged; the log
does not establish an OS/fsync cause, only a notification later than one second.

Focused foundation 51/enablement 78, static gates, full long-budget Devenv suite
and CLI pass. Independent spec `ses_ee19c5e78ffetygUsh6SL3c4N6` and quality
`ses_ee19acde1ffe6QpiRuHVkMNU9d` approve the narrow diff. Next exact-source native
verification remains required; Task 4 unaccepted.

Published approved fixtures as 7b8949ba914774ac4398b5be53c043a1b04e48eb;
ninth full native matrix:
https://github.com/vhodges/manyhands/actions/runs/37872797196
Inspect exact-source results before Task 4 acceptance.

### Native run 7: later authoring fixture policies corrected — 2026-10-09

Run 37863696730 passed both Linux jobs and prior Windows library/discovery gates,
then exposed 19 local-authoring fixture assumptions: plain Git worktree paths versus
verbatim canonical paths and inherited CRLF checkout versus expected LF bytes.
macOS failed only the new invalid-byte diagnostic constructor (APFS EILSEQ).
Logs: /tmp/opencode/cycle06-native-37863696730-failures.log.

Worker `ses_ee1e897b0ffeI4PwHqlqSeSf5T` changes only local_authoring.rs,
support/mod.rs and discovery_rebuild.rs. Existing actual/expected path comparisons
use physical canonical locations; missing registrations remain visible, names and
counts exact. Born/unborn fixture repositories set local autocrlf=false before
staging, with explicit CRLF checkout/stale-LF/exact-CRLF save controls preserving
unknown metadata and blobs. No production/global configuration change. Symlink
preservation/type/target assertions precede cleanup.

Exact macOS EILSEQ constructor rejection checks state nonmutation/readability,
qualified as filesystem rejection; supported filesystems retain real invalid-name
producer coverage. A separate portable SQL-NULL case proves decoding only, without
pretending synthetic insertion exercises the producer. No ignored/removed cases.
Spec `ses_ee1ca57c8ffeOIh4wk563qpY8i` and quality
`ses_ee1c87b59ffeuwTmk9mXdsoElk` approve the bounded test-only changes. Headless/
all-feature authoring 124 and discovery 78 pass, including isolated inherited
autocrlf=true; static/CLI and one complete full run pass (981 tests/cases: 308
library, 468 integration, 196 SSH, 9 docs). Native next-source confirmation remains
required, Task 4 unaccepted.

Published approved fixture corrections as 600e0ff659891d36e631b8967c3e8b56860c5709
through normal push; eighth full native matrix:
https://github.com/vhodges/manyhands/actions/runs/37868930176
Inspect exact-source results before Task 4 acceptance.
