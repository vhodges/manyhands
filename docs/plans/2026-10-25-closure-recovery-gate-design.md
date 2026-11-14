---
title: "Closure Recovery Gate Design"
date: 2026-10-25
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5TP0N68C00XN3C5B84EMVY6"
---

# Closure Recovery Gate Design

## Status And Approval Boundary

Approved design under the [contract](../Cycles/wave-03-closure-recovery-gate.md).
The owner approved this design, the contract and plan, detailed rulings, E1–E9,
implementation, subagent execution and ticket-branch commits/pushes: “Every is
approved. Proceed using subagent driven development”. Start comment
`01M5TZ0E62RMQPKBGRPX3CX92E` records the approval and separate lifecycle gates.
Source was inspected at `189bb36` (main `189284e` merged), then
reconciled with documentation-only main `0464c42`, merged as `0cbeab0`. No
Cargo command or test was run for planning.

**Final implementation amendment, 2026-10-27:** Tasks 0–9 are complete and the open
ticket is `review_ready`. Controller-reported SPEC and QUALITY approve the full
`0464c42..f92bc00` range with zero findings, including the final shared live-executor
prerequisite correction. A13's local verification/review requirements are met; the
[final checkpoint](2026-10-25-closure-recovery-gate-execution.md#task-9-final-review-ready-checkpoint)
separates pre-fix and post-fix evidence. No owner final acceptance or merge is
claimed. The shared interfaces below exist in production library code. The
[execution ledger](2026-10-25-closure-recovery-gate-execution.md#task-9-documentation-and-source-audit)
and [API audit](../research/wave-03-api-audit.md#closure-recovery-library-amendment-2026-10-27)
record the actual exports, wire fields, recovery meanings and scope audit. This is
ticket-branch availability, not merged-main or front-end delivery. C4/C5/D5 remain
gated. The numbered baseline facts and planning reviews below are historical.

Approval covers the public inputs, effect/cancellation and ownership tables,
schema additions, R-1–R-10, and named edits E1–E9. Review may uncover further
owner decisions. The [plan](2026-10-25-closure-recovery-gate-implementation.md)
must not invent a different recovery protocol during implementation.

## Historical Planning Baseline Facts

References use `src/repository/` as the prefix for `remote/`, `mutation/`,
`promotion/`, `closure/`, `read/`, `recovery.rs` and `coordination.rs`;
`repository.rs`, `results.rs` and `canonical.rs` are under `src/`. Other paths
are repository-relative. Task 0 rechecked behavior and symbols rather than
trusting line numbers; its results are retained in the execution ledger. These
facts describe the pre-implementation tree, not missing work at the current head.

1. `remote/merge.rs:47–69` has pure `classify_integration(local, incoming,
   is_ancestor, merge_base_exists)`, returning equal, contained, fast-forward,
   merge-required or unrelated histories.
2. `remote/sync.rs:715–805` has memory-ODB `prepare_clean_merge_in`; tree import,
   checkout and ref movement are separate mutations. Pure preparation can be
   reused without a remote operation.
3. `mutation/bind/sync.rs:1–22,100–103` binds publication synchronization and sets
   `refuse_divergence: true`. It is not the proposed local operation.
4. No-remote domain synchronization is a refresh with publish-pending
   (`remote/sync.rs:7577–7683`); `recovery.rs:648–656,758–763`'s similarly named
   local-synchronization helper only authorizes that tagged refresh.
5. `operation_records` is the local operation envelope (`recovery.rs:497–537`).
   It lacks merge parent/candidate/conflict/artifact evidence. Companion-table
   precedent exists in `comment_publication_bindings` (`recovery.rs:47–198`).
6. Remote integration/resolution tables reference remote operation records;
   `resolve_synchronization` requires publication-remote identity and reservation
   (`remote/state.rs:721–932`; `remote/sync.rs:5593–5860`). Reusing their SQL rows
   for a local operation without a remote is invalid.
7. The repository lease is an exclusive common-directory file lock with a 250ms
   acquisition limit (`coordination.rs:14,79–95,216–258`). A SQL row alone is not
   ownership of Git files.
8. Generic local begin has a legacy unjournaled fallback when the registry path
   is a directory (`repository.rs:3322–3328`). A new durable integration must
   reject this condition before effects.
9. Pending local rows fence new incompatible work (`recovery.rs:1334–1393`);
   remote reservations also inspect them (`remote/reservation.rs:238–258`).
   A matching pending closure is a separate lifecycle journal, not the same row.
10. Written closures block remote context publication (`closure/record.rs:741–760`,
    `remote/reservation.rs:310–323`). Those guards must remain intact.
11. `closure/checkpoint.rs:389–408,482–498,698–799` accepts a validated checkpoint
    descendant and retains closure authority. New adoption is allowed at `Closing`;
    `Integrating` binds the recorded adoption/base (`closure/record.rs:610–645`).
12. `closure/run_tests.rs::conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization`
    currently calls `synchronize_context_by_hand` before resume. It is the direct
    acceptance fixture to replace, not evidence of an existing local API.
13. `classify_conflict_entry` checks canonical shape/kind/ID but not closure
    equality (`remote/sync.rs:4430–4545`). Whole-set eligibility is shared
    (`remote/merge.rs:135–185`).
14. `preserves_item_invariants` compares every nonempty closure's parsed timestamp
    and exact author against the result (`remote/sync.rs:4556–4617`); external
    integration applies it to all three trees (`4742–4832`). Canonical ticket
    `closed_at` is `Option<OffsetDateTime>` (`canonical.rs:114`), not YAML text.
15. `ChildExecution` stores the parent kind but `fence_mismatch()` hard-codes
    promotion (`promotion/lifecycle_record.rs:510–592`). Outer synchronization
    and closure error displays mask details; inner wording is the narrow fix.
16. Request operation families are Local/Remote/KeyMaterial with a closed SQL
    constraint (`mutation/records.rs:102–109`). A companion local journal avoids
    adding another family, but still requires new action/step read mappings.
17. `OperationResumeBinding::found_local` recognizes ended rows and index handoff,
    not this new integration (`mutation/bind/resume.rs:339–366`). Abandon currently
    closes a local row without cleaning Git (`bridges/abandon.rs:428–518`).
18. Ordinary refresh may take over an operation's local row for index completion
    (`recovery.rs:765–768,1377–1380`). This must be gated by proven local-sync
    integration and artifact release, never allowed on an unresolved conflict.
19. Existing conflict observations/tokens have private fields and redacted Debug;
    `current` content is deliberately absent to avoid substituted worktree paths
    (`remote/merge.rs:71–133,188–233`; `remote/sync.rs:5051–5129`). There is no wire
    encoding to reuse as-is.
20. Request intent hashing supports raw bytes without storing them
    (`mutation/identity.rs:117–247`). Stored mutation DTOs must remain content-free.
21. `Effects::is_durable` treats writes, commits and complete integration as durable;
    `Outcome::Partial` maps to Incomplete regardless of the underlying code
    (`results.rs:282–288,302–366`).
22. Existing result codes include `worktree_not_clean`, `worktree_conflicted`,
    `original_request_required`, `recovery_required`, `external_change` and
    `identity_required`. `merge_required`'s message specifically describes remote
    divergence and must not label this local operation.
23. Command, golden/schema, conformance-home, action and read-enum inventories
    enumerate closed sets; adding a binding without extending them is incomplete
    (`mutation/command.rs`; `tests/mutation_contract.rs:1494–1705`;
    `tests/support/conformance.rs:354–377`; `tests/read_contract.rs:703,1018–1035`).
24. The native workflow already runs library tests and the existing read/mutation,
    merge/recovery and ticket-closure targets. No new test target is necessary.
    This says nothing about whether new code works on native platforms.

## Architecture And File Ownership

Add `src/repository/local_sync/` with modules `mod.rs`, `record.rs`, `observe.rs`,
`run.rs`, `conflict.rs`, `resolve.rs`, `recover.rs`, and adjacent tests. Its public
boundary is through new service reads and `Mutation` variants; low-level execution
remains library-owned. It uses the existing local operation envelope and new
typed companion tables. Do not create fake remote configuration or remote rows.

Extract transport-neutral Git integration primitives to
`src/repository/integration/` (`mod.rs`, `prepare.rs`, `artifacts.rs`,
`validate.rs`, `resolution.rs`). Existing remote orchestration remains responsible
for remote reservation and journal sequencing. Shared primitives receive explicit
ordered parents, expected HEAD/index/metadata and an already-held lease; they do
not inspect a publication remote or silently acquire a different operation owner.

Use separate adapters for remote and local evidence. Reuse the resolution
lock/manifest/partial-write protocol for submitted canonical resolutions. The
existing fast-forward and initial conflict installation are coarser libgit2
operations; they do not supply a general per-path checkout journal. Decision 6
preserves/refuses ambiguous interrupted effects instead of inventing such a
journal. The extraction task preserves remote behavior; a separately named
reciprocal admission guard prevents retained remote work and new local work from
acquiring contradictory ownership. Transport/merge policy remains unchanged.

Allowed library areas include repository registration/exports, local recovery,
coordination, integration primitives, relevant closure/promotion fences, reads,
mutation binding/command/progress/results/schema plumbing, and their tests.
No front-end implementation, dependency changes, or incidental refactoring.

### Scoped owner addendum: Task 4 namespace review

On 2026-10-26 the owner answered **“yes”** to canonical question
`01M5V7SPS9W51SSF5XYTJR5J08`; approval comment `01M5VWDPG57KWRAJ55Y2EMXRQR`
records the answer and context. The additional allowed files are
`src/repository/keys/{generation,deletion,registry}.rs`, solely for transactional
refusal of operation IDs already owned by local synchronization before journal
insertion/effects. The shared guard/audit belongs in `local_sync/record.rs` and
new tests in `local_sync`. No old assertion changes or other key behavior changes
are authorized by this addendum. Existing valid cross-journal handoffs remain
valid. This closes the scope question, not the five Task 4 SPEC findings; fixes
still require SPEC then QUALITY re-review before Task 5. No additional files,
front-end work, PR/merge/closure or native CI permission is granted here.

## Shared Interfaces (Implemented On The Ticket Branch)

New input structs, exported through `manyhands::repository`, contain no Debug,
Display or Serialize implementation that could disclose submitted source:

```rust
pub struct LocalContextSyncInput {
    pub root: PathBuf,
    pub kind: AuthoringKind,
    pub id: ItemId,
    pub observation: String,
}
pub struct LocalConflictResolutionInput {
    pub token: String,
    pub source: String,
}
pub struct LocalContextResolveInput {
    pub root: PathBuf,
    pub operation_id: OperationId,
    pub observation: String,
    pub resolutions: Vec<LocalConflictResolutionInput>,
}
```

- `Mutation::LocalContextSync`: command `context sync-local`, local operation,
  unconfirmed. Request digest includes all input fields; operation identity
  includes root/kind/item, context/primary refs and the accepted observation.
- `Mutation::LocalContextResolve`: command `context resolve-local`, unconfirmed,
  operating on the existing parent local operation, like resume's target model.
  It has its own request ID and durable resolution-attempt identity. Digest hashes
  command, resolved repository scope, parent operation ID, top-level observation,
  and every token/source pair in canonical token order; duplicate tokens refuse.
- Existing `Mutation::OperationResume` gains explicit local-sync routing. It never
  substitutes a new primary snapshot or automatically resumes ticket closure.

Public read methods (source-checked at `03a27cb`):

```rust
pub fn observe_local_context_sync(&self, repo: &ResolvedRepository,
    kind: AuthoringKind, id: &ItemId)
    -> Result<LocalContextSyncObservationDto, ReadError>;
pub fn inspect_local_context_sync(&self, repo: &ResolvedRepository,
    operation_id: OperationId)
    -> Result<LocalContextSyncInspectionDto, ReadError>;
pub fn read_local_context_conflict(&self, repo: &ResolvedRepository,
    operation_id: OperationId, token: &str)
    -> Result<LocalContextConflictContentDto, ReadError>;
```

The exact wire table below is authoritative for names/types and presentation.
Content is available only through the explicit content-read DTO, never a stored
mutation result; there is no mutable current-file read. Mutation output is
content-free and tagged `local_context_sync`; effect fields retain existing
definitions. Add closed schemas
`local_context_sync_observation`, `local_context_sync_inspection`,
`local_context_conflict_content`, `local_context_sync_mutation`, and referenced
entry objects under `schemas/v1/`; all properties required, nullable where needed.
Add goldens and root/inventory registrations. No existing C3 schema is rewritten.

Keep the global recovery-action enum unchanged: existing desktop code matches it
exhaustively (`desktop/model/operations.rs:529–563`). The new typed local
inspection/next-action DTO is the front-end contract for conflict handling;
`repo.inspect` may direct callers to existing pending-operation inspection. Use
existing `operation.resume`, `request.retry`, `repo.identity_set`, and guarded
`operation.abandon` where actually applicable.
Do not offer a misleading resume or abandon as a cure for an incompatible closure.
No new global result code or recovery-action kind is introduced: typed local DTO
state/reason/next-action distinguishes recoveries. This avoids unrelated UI
changes while still delivering all local read/resolve/resume entry points.

## Observation And Identity

### Exact wire fields and recovery choices

Every listed property is required; `?` means JSON null is allowed. Names below
are the serialized snake_case names. IDs/OIDs/tokens are strings. Display paths
are not mutation inputs. `display_path` is already escaped for control/non-UTF-8
bytes; consumers never unescape it or recover a filesystem path from it. They
present it as plain text with their ordinary context-appropriate rendering escape.
Enum values are closed and registered in contract tests.

| DTO | Fields |
| --- | --- |
| `LocalContextSyncObservationDto` | `kind` (`document`/`ticket`), `item_id`, `context_worktree`, `context_ref`, `primary_ref`, `context_head`, `primary_head`, `observation`: strings. |
| `LocalContextSyncInspectionDto` | `operation_id`, `kind`, `item_id`, `context_ref`, `primary_ref`, `context_parent`, `primary_parent`: strings; `context_head`: string?; `state`, `reason`, `next_action`: enums below; `observation`: string?; `eligibility`: `eligible_canonical`/`external_resolution_required`/null; `paths`: array of entry DTOs; `can_start_resolution`: bool; `starting_request_id`, `cancellation_request_id`: strings; `active_resolution_request_id`, `required_request_id`, `result_oid`: string?; `cancel_requested`, `cancel_acknowledged`: bool. |
| Entry DTO | `display_path`, `token`: strings; `content_available`: bool. Ordered by raw repository path bytes; display escapes control/non-UTF-8 bytes and is never decoded as a writable path. |
| `LocalContextConflictContentDto` | `operation_id`, `token`: strings; `base`, `local`, `incoming`: string?; `historical`: bool, true once a resolution attempt is accepted, including before its first path write; sides remain the immutable recorded blobs even after live stages change. No `current`. Source fields have no revealing Debug implementation. |
| `LocalContextSyncMutationDto` | `type`: constant `local_context_sync`; `item_id`, `kind`: strings; `operation_id`, `resolution_request_id`, `required_request_id`, `result_oid`: string?; `state`, `reason`, `next_action`: enums; `eligibility`: enum?; `cancel_acknowledged`: bool. It contains no side/source content. |

State enum: `prepared`, `applying`, `conflict_pending`, `resolution_prepared`,
`commit_prepared`, `integrated`, `index_pending`, `complete`, `recovery_required`,
`cancelled`, `abandoned`, `refused`. `refused` is mutation output before acceptance,
never a retained pending operation. `cancelled` is the terminal pre-effect state;
recoverable cancellation keeps the underlying phase plus acknowledgement fields.

Reason enum: `none`, `canonical_conflict`, `unsupported_conflict`,
`immutable_closure_disagreement`, `original_resolution_required`,
`foreign_change`, `ambiguous_backend`, `unrelated_histories`, `index_pending`, `identity_required`,
`cancelled`, `abandoned`, `authority_missing`. Missing journal authority returns
an existing ReadError where no trustworthy inspection can be built; never invent
parents or a starting request ID to populate the DTO.

Next-action enum: `none`, `resolve`, `retry_resolution`, `resume`,
`external_repair`, `set_identity`. Inspection chooses it from evidence:

| Evidence | Next action / request ID / fresh resolution |
| --- | --- |
| Pending canonical conflict, no accepted R | `resolve` / null / true; fresh observation and complete tokens required. |
| Partial R needing original bytes | `retry_resolution` / R / false; exact accepted R input is required. If bytes are unavailable, preserve state and diagnose; no promised reset or abandonment. |
| Prepared work or fully prepared candidate; no ambiguous artifact | `resume` / S / false; U may take ownership through the existing resume input. |
| Index-only work | `resume` / S / false. |
| Unsupported but otherwise admissible conflict | `external_repair` / null / false; exact external-result proof required. |
| Foreign or ambiguous Git state | `external_repair` / null / false; preserve evidence, require operator proof before retry. |
| Missing commit identity | `set_identity` / the accepted S or R if one exists / false; then repeat that original input. |
| Proven incompatible closures before effects | `none` / null / false; diagnostic-only immutable-disagreement reason, no pending operation. |
| Unrelated histories before effects | `none` / null / false; recovery_required with unrelated_histories reason, no accepted operation. |
| Complete, abandoned or terminal cancelled | `none` / null / false; immutable request replay still available. |

Recoverable cancellation does not override the needed recovery action: after
acknowledgement, a canonical conflict still needs resolve, and a partial R still
needs R's source. Progress reuses existing `accepted`, `observing`, `writing`,
`updating_local`, `indexing` and `stopping` values from `mutation/progress.rs:82–93`.
No network/connecting progress is emitted by local execution.

Reads use read-only sessions and no persistent token-issuance table. Use existing
**BLAKE3**, not a new SHA-256 dependency, with versioned domain separators and
length-delimited fields. Observations bind canonical root, registered context
identity, ordered refs/OIDs, target index/worktree state and routing configuration.

Snapshot protocol: (1) read registration/journal facts in read session A and close
its transaction; (2) read relevant Git state and immutable blobs; (3) use a **new**
read session B to verify registration, semantic revision, phase, attempt association
and cancellation counters; (4) recheck Git refs/index/metadata after B. Return
`external_change` on any difference, without an unbounded read retry. A second
query in session A is insufficient because SQLite retains its snapshot. No write
lease or worktree creation occurs. The consistency point is the matching B/Git
recheck; subsequent mutations still perform their own checks.

Separate monotonic `semantic_revision` (phase/attempt/content/registration changes)
from executor `owner_epoch`. Tokens bind semantic revision and conflict snapshot
generation; ownership takeover alone does not invalidate a conflict input. Each
executor must still prove its current owner epoch before effects. Cancellation
counters participate in inspection stability, not the immutable conflict token.

Tokens are opaque prefixed digests, matched against recorded, scope-checked
conflict entries—not decoded into caller-chosen paths or refs. Hash raw path bytes,
modes, stage OIDs, parent operation and conflict snapshot generation. A malformed
token is `invalid_input`; a well-formed foreign token is `external_change`.
Regular-file/UTF-8 guards apply to explicit side reads.

Before resolution acceptance, a fresh resolution observation requires a current
conflict snapshot and live Git fingerprint. After acceptance, the immutable stage
OID snapshot remains readable by its original tokens even if owned writes removed
live index stages: these are historical sides. Inspection exposes
`can_start_resolution=false` and the required request ID. Only that exact request
may reconcile its stored attempt before token revalidation; fresh requests cannot
use historical tokens to authorize new writes. A new conflict generation, completed
or abandoned operation, missing blobs or lost journal invalidates side reads.

Only a **new** operation validates the initial observation against live parents.
An accepted retry matches stored intent first, then reconciles frozen evidence.
This avoids refusing a request because its own work advanced HEAD. The original
observation never gets silently replaced by a fresh read.

Commit identity is the usable effective Git identity at the committing boundary.
Local sync never writes identity configuration, prompts, or accepts a separate
confirmed identity. Missing identity returns `identity_required` with
`repo.identity_set`. Identity fields do not make up merge-input observations;
changing identity alone can unblock explicit retry/resolution without changing
frozen parents. Once a candidate OID is journaled, its commit identity is fixed.
Do not create a fresh candidate on retry merely because current identity changed.

An existing active context is required. Missing/wrong-kind item or absent active
context returns item-scoped `item_not_found`. Target HEAD must name its registered
context branch. Uncommitted tracked or untracked target changes refuse
`worktree_not_clean`; pre-existing unowned conflict state refuses
`worktree_conflicted`. Local primary's committed HEAD is the input; uncommitted
primary content is neither read as merge input nor written by this operation.

## Journal, Ownership And Admission (R-2, R-3)

Use `OperationFamily::Local` and a new action `context_sync_local` in the existing
local envelope. Typed companion tables live in the same database:

- `local_context_sync_records`: envelope FK, format version, kind/item, exact
  refs/worktree identity, original observation digest, frozen parent OIDs,
  state/generation, admitted closure parent/checkpoint if any, candidate/result
  OIDs, cancellation acknowledgement, and integration/index completion evidence.
- `local_context_sync_steps`: ordered durable Git-effect intents and observed
  completion, baseline/expected index and merge-metadata digests, ref CAS inputs.
- `local_context_sync_resolutions`: attempt/request identity, observation and
  complete input digest, candidate OID, state and owner epoch. No source text.
- `local_context_sync_request_control`: request FK, nullable parent FK while
  acceptance is being linked, immutable intended parent operation ID and repository
  scope, role (`start`, `resolve`, `resume`), current execution request attempt,
  issued/acknowledged cancellation counters, and per-request effect evidence.
  Control rows are created atomically with F2 acceptance for new commands;
  local-target resume links are established before takeover/effects. Existing
  requests keep their current boolean cancellation semantics. Intended-parent
  identity exists before the domain FK/attempt link and survives across retries.
- Owned path/index/ref-log/merge-metadata evidence follows the extracted existing
  resolution protocol, including expected pre-image OIDs/digests and per-path progress.
  Paths/IDs/hashes needed for recovery are metadata, never diagnostic dumps.

Creation of envelope plus companion identity is one immediate transaction under
the repository lease. No unjournaled fallback. Schema upgrade is additive, versioned
and shape-audited; inconsistent/missing companion evidence is a refusal, not a
reason to infer authority from Git. A unique operation ID remains global across
all existing journals and child identities. The closure ID is never reused.

Every new executor, resolver, resume and owner takeover verifies the common-Git
lease, database owner epoch and recorded generation. Only the exact operation can
resume its pending row. Local sync remains a repository-wide local pending owner,
except for narrowly identified actions needed to read, resolve/resume it, or set
identity. Generic refresh cannot prematurely mark it complete. Index handoff
starts only after Git integration and owned-artifact release are proven.

| Existing state | New local-sync admission |
| --- | --- |
| Ordinary document/ticket context, no conflicting owner | Allow with clean target and fresh observation. |
| Matching ticket closure `Closing`, written checkpoint, valid descendant/closure fields | Allow; bind closure ID/checkpoint, recheck under lease. |
| Closure before a written checkpoint | Refuse while that closure owns work; caller resumes/reconfirms it first. |
| Closure `Integrating`, publishing/cleanup, completed or abandoned | No special exception; never alter a context held by integrating/cleanup work. An ordinary remaining context must independently satisfy normal admission. |
| Promotion owner or unrelated lifecycle owner | Refuse; no new promotion-recovery promise. |
| Other pending local operation or incompatible remote reservation | Refuse with existing busy/recovery semantics; never steal ownership. |
| Remote operation with retained integration, resolution, release or index-handoff obligations, even phase `interrupted` and even another context | Refuse local start until that owner settles; phase alone is not proof of release. |
| Same accepted local-sync operation | Reconcile its recorded state, not fresh admission. |

Closure preview/checkpoint/integration must recognize an unfinished local sync for
its exact context and refuse advancement. Remote publication of a written closure
remains prohibited. Local sync completing does not change the closure record or
its consent: the subsequent explicit closure resume performs adoption itself.

Admission is reciprocal and transactional. Remote conflict reacquisition and
restart must check pending local-sync ownership **before** changing remote phase
or owner epoch, including `reacquire_synchronization_conflict`'s transaction
(`remote/reservation.rs:1916–1963`). Local admission checks retained remote rows
and artifacts as well as active reservations. Two acquisition-order tests must
prove that one owner refuses without altering its phase, rather than both owners
becoming active and fencing each other. This is the explicitly scoped reservation
guard change permitted alongside the otherwise behavior-preserving extraction.

## Execution And Recovery State Machine

States below belong to the typed companion, not fabricated remote stages:

`prepared → applying → conflict_pending | commit_prepared | integrated →
index_pending → complete`, with `resolution_prepared` before resolution writes,
and `recovery_required` retaining the last proven stage/evidence on disagreement.

1. Prepare validates scope/observation/cleanliness/admission, classifies the graph
   in memory and checks prerequisites. Inspect prepared conflicts for incompatible
   closure pairs before accepting a persistent pending operation (decision 5).
   Such a refusal returns `recovery_required`, eligibility external-only, reason
   `immutable_closure_disagreement`, and no executable recovery action; no pending
   owner or Git mutation remains. Recheck under the lease before binding accepted
   intent/frozen OIDs atomically.
2. Equal/contained goes to index handoff without Git mutation. Fast-forward records
   exact expected checkout/ref intents. Clean merge builds its candidate in memory,
   imports objects, then records its OID before any ref change.
3. Before each checkout, conflict installation, path/index write, ref transition
   or owned metadata removal, persist ownership intent. Reconcile actual pre-state
   and exact proven post-state on retry. Submitted canonical resolution has the
   inherited per-path protocol. Initial libgit2 checkout/conflict installation
   does not: an incomplete or ambiguous image is preserved and refused for operator
   repair, not automatically resumed as if per-path ownership were known.
4. Conflict installation records its full index-stage fingerprint and owned merge
   metadata, then returns recoverable partial state. Do not write final resolution
   candidates or accept blanket ours/theirs.
5. Resolution validates the complete canonical set, IDs, all-side immutable fields
   and prospective context before writes. Persist attempt identity/digest and
   per-path intents; write only validated canonical owned paths; prepare a single
   ordered-parent merge candidate; CAS context ref; release only owned artifacts.
6. Mark integration proven before index handoff. Index failure yields
   `discovery_pending`; retry only completes discovery/settlement. Successful
   completion verifies candidate reachability and empty owned-artifact obligations.

An explicit external repair is accepted only as a clean commit with **exact
ordered recorded parents** and all canonical invariants preserved, as in the
existing remote contract. Deleting markers, staging files, wrong-parent commits,
resetting refs or changing closure provenance does not prove completion. This
allows supported external code repair but cannot reconcile incompatible immutable
closure pairs. Preserve and report that condition without offering a false cure.

### Backend limits and operator proof (decision 6)

The supported concurrency model is cooperative Manyhands writers sharing the
repository lease. Direct external edits observed between phases are preserved and
refused. Concurrent direct filesystem namespace replacement during bounded apply
is outside the inherited guarantee; no claim of linearizable protection against
arbitrary Git/filesystem writers is made.

Before a coarse checkout or conflict-install call, persist frozen parents, expected
target/ref/index baseline, prospective candidate or conflict stages, and applicable
owned metadata intent. After a crash, continue only from the exact baseline or a
fully proven intended result. Unknown partial files, backend lock/reflog ambiguity,
or **foreign/unproven** metadata produce reason `ambiguous_backend`, retaining the
operation and every unproven artifact. This does not include interrupted retirement
of individually proven owned metadata or sentinels: an already-absent recorded
member counts as retired, and a remaining exact-owned member is retired through
the existing protocol. That path resumes automatically without another commit.
No best-effort deletion/reset of unproven artifacts is allowed.

An operator may restore the recorded clean baseline after preserving their edits,
or establish a fully validated expected result; for an externally made resolution
that means the exact ordered-parent clean commit with all invariants preserved.
They must settle/remove ambiguous backend locks/logs externally after confirming
no live owner. Retry rechecks refs, index, worktree and metadata; neither operator
assertions nor merely deleting a lock are proof. This exceptional operator step
is not the normal supported canonical-conflict workflow or a new automatic abort.
Use child-process termination without Drop at checkout/metadata/ref boundaries,
in addition to injected returned failures, to prove preservation and refusal.

## Request Resolution, Resume, Cancellation And Abandon

Call the starting request **S**, a resolution request **R**, and an
`operation resume` request **U**. S owns the local operation; R owns a separately
recorded resolution attempt; U can take execution ownership without supplying
source. Only one executor/attempt may mutate at a time.

R has no new `request_operations` parent row, but it is **not absent evidence**.
Add a local-control association lookup before `Accepted::read_journal_row`'s
family-None shortcut and before `recorded_effects`'s absent-row shortcut
(`mutation/bind/mod.rs:513–524,899–921`). Durable evidence lookup is keyed by stable
request ID; current execution-attempt authorization is checked separately. It
returns the parent/resolution-attempt's pending/final evidence and effects. A
linked pending R returns `Standing::Owed` or a real pending journal standing;
it is never deleted as “nothing ran”. Unknown/unreadable association evidence
retains the request. Request mismatch consults R's recorded attempt effects.

The acceptance transaction creates each new command's control row. Linking it to
the local operation/attempt occurs in the same transaction as the domain identity,
under the repository lease, conditional on the F2 request still being Accepted
at the expected request attempt. No effects precede this association. A crash in
the acceptance-to-link window therefore has no Git effect; exact retry links it,
and any competing stale executor fails the request-attempt check. The narrow
generic no-row/delete race documented by F2 is not inherited as write authority
for these new operations. Every domain mutation and final settlement verifies
both request attempt and current domain owner epoch. Linked U follows the same
evidence lookup while executing this local operation; other resume targets keep
their existing semantics.

On exact F2 re-entry, `enter_request` already increments `request_records.attempt`
before journal lookup. Extend that **same transaction** for all local-control
requests, linked or still unlinked,
to advance only the control row's execution-attempt fence to the new value.
Preserve intended/linked parent identity, resolution-attempt identity, historical
effects, and issued/acknowledged cancellation counters. Do not recreate the row
or key durable evidence by the previous execution attempt. Missing/corrupt control
is a refused handover, never a replacement empty record. This advances no domain
owner epoch: domain takeover still requires the repository lease and its own
fenced transaction. An old executor fails its next attempt/epoch check and cannot
settle against the new attempt. A crash after F2 handover but before domain takeover
is harmless to evidence: another retry advances the fence again and reconciles
the same recorded parent/attempt, including outstanding cancellation.

Intended-parent identity is recorded at R/U acceptance, independently of nullable
domain linkage. Linking rechecks that exact scoped parent and its live state.
If another resolution completed the parent before an **unlinked** R/U could run,
refuse `external_change` with no own effects and release that unlinked request
under the ordinary pre-effect refusal rule. Never create a new attempt against a
completed parent. For a terminal cancelled/abandoned parent, finish the unlinked
request cancelled with no own effects and the parent's terminal DTO state; retain
its ID. A missing parent or corrupt association is not either terminal case: retain
the request and report missing authority rather than guessing a disposition.

A successful resolution completes the parent integration and index handoff and
stores its own content-free result. The original starting request can subsequently
finish from authoritative parent evidence; it never restarts the merge. Failure
between parent completion and either request settlement is independently replayable.

`operation resume` can continue prepared deterministic integration, accept a valid
external merge, or finish indexing. It cannot invent missing submitted source.
If a partial resolution needs bytes that are not reconstructible from a fully
journaled candidate, return `original_request_required` naming the resolution
request, not the starting sync request. Repeating that exact resolution request
reuses the attempt even after the live index no longer holds the old conflict
stages. Do not re-enter through fresh-token validation before attempt reconciliation.

### Cancellation generations and settlement

Only the local-control rows use monotonic `cancel_issued` and `cancel_acknowledged`
counters; both start at zero. `request cancel S` or `request cancel R` atomically
increments that request's issued counter while Accepted. Request acceptance and
counter existence are atomic, including before the parent is linked. Existing
commands retain their boolean cancellation implementation. For U, `request cancel`
remains unsupported as it is today: inspection identifies S as the stable
`cancellation_request_id`, which the local U executor checks. A synchronous
cancellation token from U's caller records a cancellation against S.

Every executor checks the vector of issued counters for S and the active R at
safe points, including an executor entered through U. At a recoverable stop it
durably records the last proven Git/attempt phase and acknowledges exactly the
observed counter values in the same transaction. Never assign acknowledgement
from an unconditionally reread latest value. A later issued value remains pending.
No safe point interrupts an unjournaled primitive; after ambiguous backend effects,
retain/refuse rather than pretending cancellation made the state resumable.

| Event | S / R / U request state | Parent/attempt and reply |
| --- | --- | --- |
| Accepted S cancelled before any Git effect, no R exists | S Finished with immutable `cancelled`; no ID reuse | Parent final-cancelled, no pending owner. A new intent uses a new request ID/observation. Cancellation before acceptance retains the existing no-record behavior. |
| S or R cancelled while conflicts/owned work remain | S and linked R stay Accepted; executing linked U stays Accepted | Keep nonfinal parent/attempt and return `cancelled` with true effects using `Standing::Owed`, never FinalKind::Cancelled. |
| Crash after durable acknowledgement, before reply | Requests remain Accepted | Original IDs resume from recorded phase; acknowledged counters do not cancel again. A higher issued counter is still honored. |
| Explicit resend R after its cancelled reply | R Accepted until its work finishes | Verify original complete digest before reconciling attempt; acknowledged counters permit continuation. |
| U attempts a partial resolution lacking complete candidate | S/R remain Accepted; U is refused before takeover/effects and its record is released under the existing before-effect refusal rule | Identify R in data and `request.retry`; no source is invented, and U's ID may be reused because no finished result was stored. |
| U resumes deterministic work or completed candidate | S/R remain pending until authoritative completion; executing U follows its actual result | U checks S/active-R counters and current owner epoch; it never supplies missing source. |
| Integration/index and owned release complete | Finish each executing request from authoritative evidence; still-Accepted S/R may settle on exact retry | Finished request results never change. No operation is restarted. |

`show_request` truthfully shows Accepted with no stored result after recoverable
cancellation. `CancelDto.acknowledged` for these local commands consults the
generation counters rather than requiring a finished request. Inspection exposes
the cancellation target and required resolution request. A replay of a genuinely
Finished cancelled request stays cancelled forever; no special reopening exists.
Explicit continuation supplies a fresh caller cancellation token; reusing an
already-cancelled token still requests cancellation of that invocation.

### Atomic abandonment disposition

After the repository lease and fresh artifact-free proof, one transaction marks
the parent/attempt abandoned, revokes executor epochs, and **finishes every still
Accepted associated S/R/U** with `cancelled`, DTO state `abandoned`, and that
request's proven historical effects. It never deletes/reuses those IDs. Already
Finished records retain their immutable results. The abandon request itself
returns the existing successful abandonment result. This is a local-sync-specific
extension of `bridges/abandon.rs`, not a change to old local-operation semantics.

Association for abandonment includes **intended scoped parent identity**, not only
the linked FK or `request_operations`: an accepted-but-unlinked R/U is included
and finished cancelled with no own effects. Acceptance/link transactions check
terminal parent state under the same database serialization, so a request racing
after abandonment either joins that disposition or observes the terminal parent
and finishes without effects. This prevents an R0 accepted before a different R1
completes from remaining Accepted forever after the artifact-free parent is
abandoned. Terminalization returns the stored result for an executing request
already finished by that transaction; it does not attempt a second finish or
overwrite any existing Finished result.

Prepared-before-effects or integrated-and-released/index-pending can be abandoned.
Conflict, partial resolution, ambiguous checkout or lock/metadata obligations
refuse before any journal/request change. No Git history is rewound. A crash before
the transaction leaves every owner intact; a crash after it leaves all associated
requests terminal and replayable. Any stale executor's attempt/epoch checks prevent
later writes or settlement. Exact retries replay cancelled historical results;
changed input under any of those IDs gives request_mismatch. New unrelated work
may proceed only after the artifact-free parent was durably released.

## Effects And Required Outcomes (R-8)

Publication and cleanup are always `not_requested`; primary is never the mutation
scope. Reads have all effects `not_requested`. Effects follow actual evidence,
including partial writes before a ref moves.

| State/result | Code/outcome and effects |
| --- | --- |
| Equal/contained, discovery current | `already_applied` / noop; write/checkpoint unchanged, integration not_requested, no commit OID. |
| Fast-forward completed | `ok`; write written, checkpoint unchanged, integration complete, commit OID = adopted incoming tip. |
| Clean/resolved/external merge completed | `ok`; integration complete; checkpoint committed only for a commit this operation made, otherwise unchanged; result OID present. |
| New refusal before effects | Existing input/blocked code; no requested durable effect falsely reported. |
| Conflict installed | `recovery_required` / partial; write written, checkpoint pending, integration pending; inspection data/action identify eligible or external-only conflict. |
| Integration committed, refresh failed | `discovery_pending` / partial; integration complete, commit OID retained, discovery pending. |
| Accepted-input mismatch or later external change | Existing mismatch/change code with historical effects; Partial/Incomplete precedence can make this exit 4, not the code's ordinary class. |
| Cancelled | `cancelled` with exact completed effects; outstanding work remains inspectable where present. |

Mandatory recovery table, one fixture per sequence (Task 5 is test-first):

| Sequence | Required outcome |
| --- | --- |
| Crash after intent, before Git effects | Same-ID retry completes recorded pair once; no new observation needed. |
| Crash during initial checkout/conflict installation | Exact baseline or fully proven result resumes; ambiguous partial filesystem/index/metadata state is preserved and requires operator repair (decision 6). |
| Primary advances after acceptance | Finish frozen pair; report it honestly. A fresh operation is needed for new primary. |
| Context changes outside owned progress | No destructive reconciliation; external_change/recovery_required with original evidence retained. |
| Conflict submitted with stale/foreign/duplicate/missing token | Refuse before resolution writes; index/files/refs unchanged. |
| Crash after some resolution paths, before complete candidate | `operation resume` requires the original resolution request; exact resend completes once. |
| Crash after candidate/ref transition, before settlement | Reconcile same candidate, release only owned artifacts, no second commit/ref effect. |
| Cancel while conflict or resolution is pending | Retain files/evidence; explicit continuation can progress; abandon refuses unsafe release. |
| Resolution finishes; starting request output lost | Both requests replay their own recorded/provable answers, no new Git effect. |
| Pending closure at Closing + local sync partial | Closure and incompatible work refuse; reads and exact recovery remain available. |
| Local sync completes + explicit closure resume | Adopt validated descendant; original closure_oid, closed_at, closed_by retained. |
| Healthy rebuild during recoverable state | Evidence retained; no authority manufactured or prematurely finished operation. |
| Cache/journal lost with conflict/lock metadata present | Recovery required; no adoption, cleanup, fresh merge or silent abort. |
| Cancellation acknowledged after an R path write, then process dies before response | S/R remain Accepted, not finished-cancelled; exact R resend continues; U identifies R; no dropped source or cancellation loop. |
| Another cancellation races with acknowledgement or U takeover | Newer issued counter survives; U checks S/active-R controls and stops at the next safe point. |
| R integrated/released but indexing failed; abandon runs | One transaction abandons parent/attempt and finishes all Accepted S/R/U cancelled; old finished results remain immutable, IDs never reused. |
| R accepted but dies before parent link | No Git effect; exact retry links under fresh request-attempt/epoch checks; stale executor cannot write. |
| Changed only R's top-level observation after partial writes | request_mismatch with R's historical effects; accepted attempt and files unchanged. |
| Retained remote work versus local start, either acquisition order | Only one owner admits work; refused reacquisition changes no remote phase/epoch; no reciprocal fence stalemate. |
| Known incompatible closures found during preparation | Refuse without pending owner/effects; unrelated repository mutation succeeds afterward. |
| Child process killed inside backend ref/log/metadata window without Drop | Preserve ambiguous artifacts; operator settles them and establishes exact baseline or validated result before retry can proceed. |
| Child exits during proven per-member owned metadata/sentinel retirement | Automatically observe already-absent members and retire exact-owned remainder, without operator repair or another commit; foreign members remain a refusal. |
| Partial R retries, F2 advances attempt, then crashes before domain takeover | Next retry retains the same durable association/effects/counters; only the current attempt/epoch can execute or settle. |
| Unlinked accepted R0 races with R1 completing and artifact-free abandonment | Intended-parent identity includes R0 in atomic cancellation, or R0 observes the terminal parent and finishes without effects; no dangling Accepted request. |
| Parent completes before an unlinked R/U can acquire its attempt | Refuse external_change without own effects and release that unlinked request; no new attempt or rewrite of the completed parent. |

## Schema Loss, Platform And Failure Boundaries

Healthy rebuild must leave typed journals intact. Validate companion/table shape
alongside local journal validation and registration-removal checks. Corrupt
replacement/lost registry does not reconstruct operation identity from Git.
Orphaned conflict/owned-artifact evidence blocks mutation and remains for external
diagnosis. A fully committed, clean result can be observed afresh after rebuild;
old request identity is not claimed to survive database destruction.

No new crate. Use existing platform-safe lock and no-follow/path checks; never
derive mutable paths from untrusted display strings. Test symlink substitution,
non-UTF-8/unsupported modes, CRLF input preservation, dropped handles before
database replacement, and two-process lease contention. Finite local work has
no network timeout/retry budget; the existing lease timeout and explicit safe
points bound waiting/cancellation. Never spin waiting for a lock holder or
silently retry after foreign changes. Linux evidence is local; native validation
is separately authorized after merge through existing targets.

Local persistence/access mapping: journal unavailable/permission-denied before
effects returns `index_unavailable`; repository/blob/path access denied returns
`repository_inaccessible`. Busy lock/SQL contention stays `busy`. A later failure
retains proven effects and a pending owner; durable effects make the answer partial.
These are mappings of the new local API, not changes to unrelated existing error
conversions. Use injected typed PermissionDenied at exact owned I/O boundaries
for portable tests, with Unix real-permission controls where the fixture can
enforce them; do not claim Windows ACL evidence from those injections.

After full journal loss, conflict/owned lock evidence refuses `recovery_required`;
a partial checkout without recognizable owner metadata still refuses via dirty/
conflicted-worktree checks rather than inventing a missing operation. A fully
integrated clean result may be observed by a new request; the erased old request
remains `request_not_found`, and a stale original observation never authorizes a
second integration. These are distinct loss cases, not one blanket success claim.

## Approved Rulings

- **R-1 Classification:** compare distinct parsed nonempty closure pairs across
  all three stages; preserve whole-set eligibility. Cost: more conflicts honestly
  become external-only; immutable-race recovery remains outside this ticket.
- **R-2 Local envelope plus companion:** reuse Local family; add typed integration
  evidence rather than fake remote rows. Cost: migrations, audits and fence plumbing.
- **R-3 Closing-only special admission:** allow only exact matching written closure
  at Closing; reject new local sync during integrating/cleanup/promotion ownership.
  Cost: not every lifecycle state can be repaired by starting a new local sync.
- **R-4 Wire capabilities:** deterministic opaque observation/path hashes recomputed
  from scoped evidence, no persistent read-token writes. Cost: stale reads require
  re-observation; tokens are not authorization outside the registered repository.
- **R-5 Local identity:** effective configured identity at commit, no config writes
  or inline confirmation. Cost: missing identity requires identity-set and retry.
- **R-6 Two request identities:** resolution owns an attempt of the parent operation;
  original-request-required identifies resolution input when necessary. Cost: callers
  retain both IDs and may need to resend sources explicitly.
- **R-7 Cancel/abandon:** acknowledged recoverable cancellation is resumable; abandon
  requires artifact-free proof. Cost: no reset/rollback escape hatch.
- **R-8 Effects:** distinguish fast-forward, made commit, external commit, conflict
  writes and discovery; never infer exit class from code alone. Cost: larger tests.
- **R-9 Boundary names:** `context sync-local` / `context resolve-local` and local
  read DTOs; no available CLI verbs now. Cost: front-end plans must add deliberate
  adapters and explain local-primary versus publication synchronization.
- **R-10 Scope-preserving extraction:** share neutral mechanics, keep remote
  transport/merge policy unchanged, add only the explicit reciprocal admission
  guard, and retain decision 6's backend ambiguity stops. Cost: extraction needs
  its own regression gate; no general transactional checkout guarantee is added.

R-3 through R-9 affect caller semantics or evidence and were included in the
owner's detailed approval. Decisions 1–6 remain settled.

## Named Edits To Pre-Existing Tests

Only these scoped expectation/inventory changes are approved. New tests and new
module/golden registrations are additions. Any other semantic test edit stops for
owner review. Task 0 proved the inventory before implementation; Task 9 records
the full-branch named-edit audit, including the two owner addenda below.

| Edit | Existing location | Authorized change |
| --- | --- | --- |
| E1 | `tests/ticket_closure/races.rs::true_race` | Fresh inspection/resolution becomes external-only instead of EligibleCanonical/ValidationFailed; retain no-write assertions. |
| E2 | `remote/sync_tests.rs::all_recorded_ticket_sides_constrain_retained_closure_before_writes` | Only incompatible-closure rows change to external-only; retain other immutable validation rows and legacy-attempt behavior. |
| E3 | `closure/run_tests.rs::conflicting_advance_after_the_checkpoint_needs_a_deliberate_synchronization` and `adopted_context_is_merged_reconciled_after_f4_and_retired` | Replace manual sync helper use in these cases with the real new local API/explicit canonical resolution; retain closure authority assertions. |
| E4 | `read/status_tests.rs::{every_action_the_recovery_store_writes_is_one_the_read_knows,a_local_action_is_one_the_recovery_store_writes}`; `recovery.rs::every_step_is_stored_under_its_own_name_and_read_back` | Add the new local action/step names and update exact inventory boundaries, including the companion local-action partition. Owner explicitly approved the omitted companion edit in comment `01M5V0RYTAKJE3F155FH4QTP53`; Task 4 owns implementation. |
| E5 | `mutation/command.rs::only_a_synchronization_stops_at_safe_points_and_none_needs_confirmation` and `only_a_command_whose_work_no_journal_records_has_no_operation` | Account for the two local commands, local progress, and resolution's durable association without a new parent operation; preserve old cases. |
| E6 | `tests/support/conformance.rs` `HOMES`; `tests/mutation_contract.rs` command/golden/data-root inventories | Register the two new bindings and their owned test homes/data roots; do not exempt existing commands. |
| E7 | `tests/read_contract.rs::contract_enumerations` and its schema-root/enum coverage; `schemas/v1/operation.schema.json`, `operation_abandon_preview.schema.json`, `operation_mutation.schema.json` | Add local action and new local DTO enum values/roots; preserve every existing value. No global recovery-action addition. |
| E8 | Library module visibility/declarations and test module registrations | Mechanical moves/import rewiring for shared integration extraction, preserving function bodies/assertions except E1/E2. |
| E9 | `tests/support/{golden.rs,mutation_golden.rs,mutation.rs}`, `tests/support/repair.rs::repair_data` and `tests/mutation_replay/conformance.rs` | Add new golden registrations and conformance rows; add explicit new-variant refusal arms to the seven existing DTO helper matches (`ticket_data`, `document_data`, `remote_data`, `operation_data`, `key_data`, `identity_data`, `repair_data`), without weakening their old assertions. New local helpers handle the new DTO. Owner explicitly approved the omitted repair helper in comment `01M5V0RYTAKJE3F155FH4QTP53`; Task 7 owns implementation. |

### Scoped owner addendum: Task 7 migration fixture

On 2026-10-26 the owner answered **“yes”** to question
`01M5X2Y6RPMMT9BMS75Y0EXTYQ`, recorded in approval comment
`01M5X3KKAFXVGD84SYBH3TBGFB`. The additional named edit permits changes only to
`tests/mutation_replay.rs::the_migration_adds_the_request_tables_to_an_older_database_and_changes_nothing_else`
and its private `REQUEST_TABLES` fixture/removal/migration-marker inventory for
the new local request-control table. Preserve ALL existing old-column,
unrelated-data and privacy assertions. This resolves the Task 7 scope stop;
it grants no wider old-test rewrite or Task 7 review approval.

E1's exact SSH harness selector is
`true_race_distinct_identities_needs_external_recovery`; `true_race` is its Rust
function. Task 0 checked this exact inventory for drift; it did not grant an
open-ended test-edit exemption. No old output fixture changed solely from adding
a local command; any additional old golden diff needs owner review. Tests marked
NEW in the approved plan were added during execution; Task 8 maps all 25 required
outcomes and 18 mandatory names to their actual assertion owners.

## Document Amendments And Known Limits

- Append a dated amendment to Cycle 06 conflict protocol and Cycle 10's design:
  new fresh-conflict classification and actual local recovery entry points. Keep
  execution ledgers historical; do not rewrite their observed old results.
- Update Wave 03 gate/API audit with the local boundary, unavailable front-end
  adapters, and remaining C4/D5 remote-conflict and C5/D5 lifecycle obligations.
- Amend the CLI conflict/recovery RFC only to distinguish the local library capability
  and future adapters; never mark a CLI verb available before its Cycle delivers it.
- Amend `docs/RFC/git-workflow-and-conflict-recovery.md` synchronization/recovery
  sections for dedicated local-primary input, fixed-parent retry, cooperative
  writers and the inherited ambiguous-backend operator boundary.
- Document schema/action additions in their contract evidence inventory; global
  result-code and recovery-action-kind sets stay unchanged.
- Preserve the ticket's three items and scope; record progress/comments through
  its lifecycle. Do not close or remove the gate from downstream tickets early.

Limits: no immutable-closure winner policy, no automatic rollback, no fabricated
recovery after journal destruction, no new promotion-specific recovery promise,
no network or primary refresh, and no native execution evidence during planning.

## Planning Review Record

Round 1: source/feasibility 3 Blocking, 3 Important, 1 Minor; contract review
1 Blocking, 6 Important, 1 Minor. Revisions add explicit request/abandon settlement,
generation counters, dual-snapshot reads, exact wire fields, retained-owner guards,
named edits and missing test cases. Owner decisions 5–6 settle admission and
backend ambiguity policy. Round 2 found 0 Blocking/2 Important/0 Minor source
findings and 0 Blocking/1 Important/1 Minor contract findings; the request-attempt,
unlinked-contender, owned-retirement and DTO consistency revisions resolved them.
Round 3: both independent reviewers **approved**, each with 0 Blocking,
0 Important and 0 Minor findings. They reviewed the uncommitted drafts against
code base `0cbeab0`. Subsequent changes only record review and approval status.
The owner subsequently approved the detailed documents and implementation; no
new behavior has executed proof at approval. See the contract and start comment.
