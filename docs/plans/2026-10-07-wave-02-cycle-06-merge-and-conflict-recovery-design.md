---
title: "Wave 02 Cycle 06 Merge And Conflict Recovery Design"
date: 2026-10-07
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M4AZH9MXR8BJF6NXQ9VHEBAM"
---

# Merge And Conflict Recovery Design

## Status, Goal And References

Approved by the user on 2026-10-07 alongside the
[Cycle](../Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md) and
[implementation plan](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md).
The user authorized a local planning commit and implementation-session handoff,
not implementation in this session. Planning evidence, rulings and approval are
in the [execution ledger](2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-execution.md).

Deliver non-rebase integration and explicit recovery within the existing
headless synchronization service. The approved Wave 02 and Git, canonical,
index, authentication and test RFCs linked by the Cycle govern behavior.
The Git RFC's approved owned-Markdown/external-tool restriction governs the
resolution writer. Wave 03 interaction and identity prompts are not implemented
here; the domain must be usable through caller-supplied confirmed inputs.

## Observed Implementation And Required Changes

Grounded at fetched main `b666c1e1f0a708562ff4cc25b0dfb18dc99dd6a9`, rebased
Cycle 06 head `8fb768e39835acb7b84cde81df7083a03995b645`:

| Existing seam | What it actually provides / limitation |
| --- | --- |
| `remote/refs.rs::plan_clean_integration` | Pure all-clean virtual context plan; divergence returns `MergeRequired`. Keep its fast path and negative tests. |
| `remote/sync.rs::synchronize_remote` | Exact fetch, current advertisement/tracking comparison, target cleanliness, ordinary Push-endpoint verification, durable authority and refresh replay. Restart currently refetches and requires a clean target; it cannot resume its own conflict. |
| `sync.rs::fast_forward` | Locks/checks expected old ref, safe-checks out with `overwrite_ignored(false)`, then commits ref transaction. Checkout-before-ref failure is already preservation/recovery, not rollback. |
| `remote/state.rs` | One operation envelope with synchronization checkpoint, expected/local/tracking/primary/push OIDs, endpoint generation, authority and index-pending. There is no ordered multi-merge or resolution-attempt evidence. |
| `remote/reservation.rs` | Fenced owner/epoch/generation, exact root/action/target identity, safe points, explicit restart, poll yield and verified-push reconciliation. Its ordered single local checkpoint must be extended, not reused as if two merge stages were one. |
| `repository.rs` | Guarded owned-path reads/replacements, expected byte digests, canonical context validation, identity helpers and local checkpointing. `checkpoint_owned_paths` rebuilds from HEAD and commits ONE parent; it is not a merge-resolution writer. |
| `repository/recovery.rs` integration | Existing operation guards prevent competing application actions; external Git/filesystem changes must still be detected. A pending synchronization merge needs an explicit authoring guard. |
| `transport/*`, SSH fixture | Scoped selected key/host/session callbacks, exact ref transfers, distinct Push advertisement and receive-race/disconnect/privacy controls. No new network capability is needed. |
| `.github/workflows/build.yml` | Manual-only five native targets; existing headless contracts include `remote_synchronization`. Native Cycle 06 behavior remains unproved. |

Locked stack: `git2 0.20.4`, `libgit2-sys 0.18.8+1.9.7`, `libssh2-sys 0.3.3`.
No dependency/backend upgrade is proposed. Source/API characterization and real
fault tests must precede relying on merge/index/checkout behavior of this stack.

## Interfaces And Ownership

Names below are proposed new API names; the existing types above are observed.
Keep library exports in `src/repository.rs`, with implementation in private
`remote` modules. No GPUI, CLI grammar, global repository handle or cross-thread
libgit2 object is introduced.

| Interface | Input / result contract |
| --- | --- |
| Existing `synchronize_remote` | Retain `SynchronizeRemoteRequest` root, operation ID, validated target, host approval, explicit restart. Extend with optional caller-confirmed Git identity for missing effective identity; existing callers use `None`. No caller URL/refspec/path/force or arbitrary merge OID. |
| Existing synchronization result/error | Preserve `Complete`/`IndexPending` around `Published`, `AlreadyCurrent`, `PublishPending`. Extend `SynchronizationError` with typed `ConflictPending`/`ExternalResolutionRequired` categories carrying operation/target/stage identifiers, not source text; neither establishes terminal publication authority. |
| `inspect_synchronization_recovery` | Root + original operation ID. Read actual target/HEAD/index/merge state, correlate durable stage; return safe paths, eligibility, parent/blob OIDs and an opaque conflict observation. Does not mutate, fetch or assume cached flags prove conflicts. |
| `read_synchronization_conflict` | Inspection token + supported path token. Recheck observations; explicitly return ephemeral base/local/remote and current bytes in a type with redacted formatting. A stale read returns external-change. Unsupported entries give external-tool guidance rather than decoded arbitrary binary content. |
| `ResolveSynchronizationRequest` | Root, parent synchronization ID, distinct resolution-attempt ID, exact conflict observation, complete list of owned path tokens/result bytes and optional confirmed identity. Bodies and path observations are not public diagnostics; no arbitrary path string is accepted as authority. |
| `resolve_synchronization` | Local-only validation/writes/two-parent checkpoint; returns stage completion, authoritative commit OID, or partial/stale/validation/identity/recovery category. Does not fetch or publish. |
| Original explicit restart | After local resolution or external repair, resume original target/operation. First reconcile actual effects, then finish the next integration/publication/discovery step. Changed input, target/root or generation is never silently a new action. |

Inspection tokens bind operation/stage, repository identity, configuration
generation, target branch/HEAD, ordered parents, actual index stage entries and
modes, merge metadata, and current owned-file digests/absence. They are
optimistic preconditions, not authorization to bypass path validation. Incoming
OIDs come from service observations, not caller requests. Unknown or malformed
path text is bounded/redacted; only safely validated repository-relative paths
may be rendered. Never follow symlinks or accept traversal/absolute/control-
character paths merely because Git has a corresponding index entry.

Commit identity uses the existing effective Git identity resolver: a complete
name AND email at an approved configuration level, without inventing or mixing
partial identities. Clean fast-forwards/current/pending outcomes still do not
require a commit identity. If a merge/resolution needs one and it is incomplete,
return typed identity-required before candidate effects. Optional confirmed
identity input carries BOTH values, its own stable confirmation ID and expected
identity-configuration observation. Reuse existing identity validation/local
configuration writing under the lease; no prompt holds that lease.

Like host approval, this is auxiliary approval for the original action, not a
changed synchronization target. It may be added on explicit restart at the
identity-required boundary before any candidate. Its own ID/input/observation
binding rejects changed confirmation replay; existing valid effective identity
wins and is never overwritten by an unsolicited supplied value. Persist only
non-secret intent/configuration digests and completion evidence, and reconcile
a partial local identity write before commit. Legacy Cycle 05 rows imply no
identity confirmation. Once a candidate exists, reuse its OID/signature; later
configuration changes cannot regenerate it. Resolution-attempt semantic input
includes its identity-confirmation reference. This confirms identity only, not
promotion/closure consent. Task 2 adds this child confirmation evidence to the
same operation journal, not a separate reservation.

## Integration Planning And Data Flow

```text
same-ID authority? --> terminal result / index-only replay
        |
preflight + manual reservation --> exact authenticated fetch/observation
        |
all-clean virtual plan --> existing Cycle 05 local update/push path
        |
otherwise: context remote --> primary into context   [primary: primary only]
        |
per-stage ancestry --> no-op / guarded FF / prepared merge
        |
clean candidate --> guarded checkout/ref --> observed stage completion
conflict candidate --> real merge index/markers --> conflict pending (no push)
        |
local resolution or external repair --> reconcile stage --> deliberate restart
        |
remaining stage(s) --> ordinary exact push/verify --> discovery/IndexPending
```

Retain exact `+P:T(P)` and optional `+C:T(C)` fetch, unchanged `FETCH_HEAD`, no
pruning, and independent Push-direction target observations. Absent primary,
remote-deleted context and unknown publication history are checked before local
merge mutation. A new action also refuses pre-existing merge/rebase/cherry-pick
state even if its index currently looks clean; only reconciliation of this
operation's exact recorded merge may enter the recovery path. No ordinary retry
repopulates a previously published deleted context; that later explicit
republish boundary is outside this Cycle.

Each stage is determined from actual ancestry:

- Equality/incoming ancestor: no local effect; persist the skipped step.
- Local ancestor: existing guarded FF, with per-stage completion evidence.
- Divergence: require a merge base and available objects; no unrelated-history
  merge, synthetic root or silent rebase. Prepare with locked git2 merge APIs.
- Context always considers fetched context before fetched primary. Primary
  integrates only fetched primary. If the all-clean virtual path succeeds,
  retain Cycle 05's single final update and its no-partial-update guarantee.
- On the divergence path, commit one clean merge per required divergent stage.
  A later conflict retains the earlier completed merge. This is intentional
  inspectable progress, not a reason to reset the branch.

Fetch observations remain endpoint-specific. A Push-only divergent target is
still rejection/recovery; Manyhands must not fetch arbitrary Push refs to invent
a third merge stage. After all local integration, reuse Cycle 05's candidate,
ordinary push, ambiguity reconciliation and post-push authority.

### Merge Preparation And Application

Preparation runs outside the lease on a separately opened worker-local handle
whose ODB has a high-priority transient in-memory backend. This is important:
[libgit2 v1.9.7 `merge.c`](https://github.com/libgit2/libgit2/blob/v1.9.7/src/libgit2/merge.c)
resolves content through `git_odb_write`, so `merge_commits` against the ordinary
disk ODB is NOT read-only just because its index is in memory. Locked
[git2 0.20.4 `odb.rs`](https://github.com/rust-lang/git2-rs/blob/git2-0.20.4/src/odb.rs)
provides `add_new_mempack_backend(1000)`. Use that preparation-only ODB to read
existing objects and retain generated merge blobs/index/tree in memory; do not
change refs, merge metadata or worktree through this handle. Characterize that
it makes no destination-disk writes and does not affect other handles; if it
cannot enforce this boundary, stop for a design amendment, not a disk-write
fallback. The handle/backend and prepared index stay local to the same worker.

Record input OIDs and prepared index/tree fingerprints, including fixed merge
options/backend version. Re-deriving a conflicted baseline from those same
parents/options must match its fingerprint before reuse; no body or full index
copy belongs in SQLite. Full canonical scans/validation also run outside the
lease and their source digests are checked before effects. Reopen/re-observe
under the common-Git lease, fence owner/epoch/generation and lock/check the
expected target ref before local mutation.

Under the lease, after durable input intent and re-observation, import only
missing prepared result blobs/tree objects through Git ODB APIs and verify
their OIDs; create/reuse a detached candidate with ordered parents
`[old local, incoming]` and deterministic subject. Only the coordinated
application handle writes the destination ODB. No system Git/pack helper or
persistent scratch copy is introduced. Object creation without moving refs
is not publication. Record
that exact candidate before checkout; failure before durable candidate storage
may leave an unreferenced object, never an extra branch checkpoint. Do not
regenerate a recorded candidate on retry. Use the safe expected-old checkout/ref primitive,
not `force`, `reset` or a checkout that overwrites ignored entries. Retain a
checkout/ref mismatch as recovery evidence if a transition partially succeeds.

A conflicted merge MUST create actual worktree/index conflict state and merge
metadata with the same observed parents, not just conflict text in an outcome.
Characterize `merge_commits`, `merge`, `index.conflicts`, `checkout_index`,
merge-head inspection and cleanup on the locked stack. Install only the
prepared/verified merge, using safe checkout, explicit marker style and no
favor-ours/theirs policy. Text conflicts retain markers; binary/structural
conflicts retain their index entries and existing inspectable files. Safe
checkout collisions stop with preservation/recovery. A successful conflict
installation keeps HEAD at old local, records the observed conflict fingerprint,
and releases active ownership without erasing pending stage/effect evidence.

Check a resulting context still contains the recognized target. Do not silently
repair malformed or marker-only content; discovery continues to report it.
If integration changes publication/primary configuration, stop for deliberate
re-observation/replanning rather than continuing against a newly inferred
endpoint. An operation's own configuration-changing tree is evidence, not
permission to expand its remote scope.

Subjects: `Merge remote context <ULID>`, `Merge primary into <kind> <ULID>`,
`Resolve synchronization <kind> <ULID>`, `Merge remote primary`, and
`Resolve synchronization primary`. The last two are deterministic extensions
of the RFC's item subjects, not user-authored names.

## Canonical Resolution Algorithm

### Eligibility And Validation

Inspect the COMPLETE actual Git conflict set. In-process resolution is eligible
only when every entry is a regular non-executable same-path UTF-8 canonical
managed document, ticket or comment and identity is provable from available
base/local/remote entries. Both sides changing an identity/path/kind, add/add
without a provable shared identity, rename/delete, binary, executable, symlink,
configuration or noncanonical entries require external repair. Path tokens must
be from this inspection, not any user path under `docs/`.

**Owner decision, 2026-10-07:** a mixed canonical/code, binary or unsupported
structural conflict set is externally resolved as a whole in this Cycle.
The owner selected "Whole merge external" during the self-review questionnaire.
Return the complete visible set and supported/unsupported categories; make no
in-process resolution writes to the mixed set. This keeps one unambiguous
checkpoint/retry boundary; its explicit cost is that Manyhands cannot resolve
even the canonical portion of mixed conflicts. Deferring that broader partial-
resolution capability does not waive supported all-canonical journeys or permit
Manyhands to stage arbitrary code.

Before the first write, validate all caller bytes as a prospective resolved
tree/context. Preserve identities and schema immutables (comment item/parent/
creation metadata where fixed, ticket closure/no-reopening constraints) and
check affected uniqueness/thread/target relationships. Unrelated pre-existing
nonconforming content remains visible; it is not silently upgraded nor a license
to introduce new problems in touched items/relationships. Return fixed
validation categories/paths, not source snippets. Caller bytes are used exactly;
unknown fields/body formatting are not automatically reserialized or chosen.
An explicit caller resolution, not absence of marker-shaped text, removes the
corresponding Git conflict entries.

A clean index against the recorded merge baseline is required outside owned
resolution paths. New unrelated staged/worktree/untracked changes block local
checkpointing; never stage or discard them. Previously ignored noncolliding
files remain untouched; an ignored checkout collision blocks safe application.
Candidate-tree validation must use guarded path reads and recheck sources at
write/commit boundaries, rather than holding the lease during a full scan.

### Identity, Partial Writes And Checkpoint

1. Canonicalize root, validate parent operation/stage and acquire fenced manual
   ownership. Validate resolution-attempt ID against every existing local/remote
   ID namespace and target. Replaying changed semantic input is an error.
2. Compute a domain-separated digest of sorted path tokens, expected observations
   and exact result bytes, plus applicable confirmed identity intent. Persist
   only the digest and per-path result/expected digests; no body or passphrase
   digest is stored. A failed preflight with no effect does not consume a valid
   synchronization's ability to receive a new resolution attempt.
3. Outside the lease, validate all bytes/prospective relationships. Under the
   lease, re-observe HEAD, parents, conflict entries/modes, merge state, paths
   and unrelated changes. Stale input fails before writes.
4. Persist the full immutable write intent before effects. Use existing guarded
   regular-file replacements for ONLY eligible paths. Observe/persist each
   resulting file digest; a partial failure preserves those files and the
   original merge state. Identical retry recognizes already-applied bytes and
   verifies remaining paths; any third value stops without overwriting.
5. Build the candidate merge index from the RECORDED merged non-conflict entries,
   removing only the explicitly resolved conflicts and adding only verified
   caller blobs. Never rebuild from HEAD using the one-parent authoring helper,
   and never use blanket `index.add_all`. Non-conflicting tracked code selected
   by Git is retained as parent-history content, not staged from new user edits.
6. Persist the resulting tree and detached TWO-parent resolution candidate OID
   before the branch ref transition. Recheck every read/source digest and exact
   index/merge ownership; write the resolved index and guarded expected-old ref.
   Do not commit while any index conflict or unexpected entry remains.
7. Observe exact HEAD/tree/index/worktree and parent relation, mark the stage
   complete, then retire only matching merge metadata. Failure to retire state
   is a recovery substep; no duplicate commit. Release ownership and return local
   checkpoint completion. Restart of the parent synchronization is a separate
   deliberate caller action.

## Durable Model And Replay

Extend `remote_operation_records` without replacing its reservation/authority.
Proposed child evidence tables belong to this same journal, not a second lock:

| Evidence | Minimal non-secret content |
| --- | --- |
| `remote_integration_steps` | Parent operation/repository/generation, ordered ordinal/kind, local/incoming OIDs, baseline tree/index fingerprint, intended/observed tree and candidate/result OIDs, phase and conflict fingerprint. |
| `remote_resolution_attempts` | Attempt ID, parent operation/stage, observation/input digest, candidate/checkpoint OIDs, progress/category and identity-intent digest. |
| `remote_resolution_paths` | Attempt and validated owned-path identity (or digest), expected/result byte digest, conflict-stage blob OIDs/modes, applied progress. No body, server text or unsupported raw path. |

Constraints enforce unique stage ordinals, parent linkage/target, valid OIDs,
complete field groups, immutable identity/digests and owner/generation fencing.
Migrate transactionally from Cycle 04/05 schemas, preserve terminal authority,
index-pending and historical rows, audit startup/rebuild, and fail closed on
partial/invalid schema. Extend active-index CHECKs and decoders together. Routine
reads/updates must remain bounded by this operation and conflict set, not scan
all historical records. Legacy rows do not infer merges or resolution authority.

Per-stage phases are prepared, applying, conflict-pending, resolution-prepared,
commit-prepared, applied, or recovery-required. The original operation retains
fetch/push/discovery phases and checkpoints; ordinals represent repeated local
steps rather than trying to rewind the old monotonic safe-point counter.
Persist intent BEFORE effects and observed completion AFTER them. Candidate
creation and record writes are not an atomic transaction with Git; interruption
handling explicitly reconciles that split.

| Actual state on retry | Eligible behavior |
| --- | --- |
| Prepared candidate, old HEAD and unchanged clean baseline | Reuse candidate; finish guarded application only. |
| Exact candidate HEAD/tree/index/worktree, completion record missing | Record observed completion; do not commit again. |
| Checkout/index changed but old HEAD, matching recorded transition | Reconcile the bounded intended transition; never reset/overwrite ambiguous state. |
| Matching unresolved index/merge state | Return inspection, or accept an explicit matching local resolution attempt; no fetch/push. |
| Partial resolution files equal old or recorded result digests | Complete only untouched intended paths; third-party edits stop retry. |
| External tool committed a clean merge with exact ordered recorded parents | Validate target and affected canonical state, reconcile stage, then explicit resume of remaining work. Manyhands stages no code. |
| Marker edits only; unresolved index; staged code without merge commit; ambiguous parentage | Remain recovery/external-resolution-required. No new automatic checkpoint. |
| First stage applied; second stage conflict | Retain first result; resolve only second and continue. |
| Merge complete, push uncertain/rejected | Re-observe publication before any push; retain merge. Unfinished newer remote integration uses new recorded stage evidence, never replay an applied stage. |
| Verified publication, index pending | Existing authority replay; refresh only, no network/local integration. |
| Changed target/generation or lost insufficient journal evidence | Recovery-required/external-change; no invented continuation or rollback. |

Completed stages are reconciled BEFORE refreshing unfinished network work.
Valid fetched objects/observations may be reused for unchanged pending local
steps, including offline canonical resolution. A crash before durable fetch
completion may require another read-only fetch: the promise is no duplicate
PROVEN effects, not exactly-once network calls after unknowable interruption.
Before an unfinished publication following caller interaction, obtain fresh
Fetch/Push evidence as needed. New remote commits append eligible integration
progress, with preserved earlier stage OIDs. Pin one current Fetch observation
per invocation: at most one context-then-primary integration pass (primary:
one stage) and one ordinary push attempt. Do not recursively refetch/merge
until a racing remote stops changing. Changed state returns retryable recovery
for a later deliberate invocation; applied stages are never reset.

Cache rebuild remains non-mutating. Unknown MERGE_HEAD/index state after cache
loss is reported, not claimed by a new operation. Git's two-parent history may
prove an existing effect only when remaining target/publication intent can also
be established; commit subject alone is insufficient.

## Concurrency, Cancellation And Discovery

**Owner decision — cooperative writers, 2026-10-08:** autonomous canonical
writers use Manyhands API/CLI and participate in the existing reservation,
lease, and authoring guards. From final apply re-observation through checkpoint,
metadata retirement, and sentinel release, direct external edits of affected
canonical paths/ancestors or relevant target-worktree/Git/staging state that
bypass coordination are unsupported. Reads and unrelated safe work remain
allowed. External repair outside that bounded interval invalidates observations.
Worktree isolation reduces canonical-file collisions, not shared-ref contention;
common-Git coordination remains required.

This is an explicit concurrency convention, not OS-enforced exclusion or proof
that editors honor index locks. No perfect expected-inode rename/unlink or
continuous foreign-lock exclusion is promised against arbitrary namespace
substitution. Observed stale changes still reject before effects; ambiguous
recovery preserves evidence and refuses foreign-lock adoption/removal. Scoped
writes, no-follow access, closure invariants, privacy, and effect-aware replay
remain binding. The concurrency answer alone approves no other change.

**Separate owner decision — narrow backend-lock recovery, 2026-10-08:** allow
operator intervention only for ambiguous live locks created internally by
libgit2 after a crash. Preserve the lock and actual effects, return fixed redacted
recovery-required, and never delete/adopt using contents, age, or PID alone.
The operator quiesces writers and verifies/handles the stale lock before identical
retry. Retry revalidates refs/index/worktree/metadata and intent, reuses the
candidate, and refuses third states. Owned sentinel/path/index recovery remains
automatic. Native/dependency scope remains unchanged. Owner separately approved
the technical amendment and sequential local implementation on 2026-10-08:
[Task 4 resolution protocol](2026-10-08-wave-02-cycle-06-task-04-resolution-protocol-amendment.md).

Extend the existing repository-local authoring guard to block canonical
checkpoints in the affected target worktree while synchronization merge state
is outstanding. Other safe contexts remain subject to the existing coordination
policy, not a new indefinite repository-wide edit freeze. A different operation
cannot adopt the pending merge simply by using the same target.
Reservation owner/epoch/generation fences all writes; stale owners cannot
advance a stage or overwrite a newer resolution. Keep the pending record when
releasing active conflict ownership so other safe work is not blocked forever.
Polling remains prohibited from merging or clearing conflicts.

Safe points include existing network boundaries plus before/after merge
preparation, conflict installation, each path-write atomic step, commit/ref
transition, merge-state retirement and between integration stages. Cancellation
preserves the current recoverable transition and does not roll it back. Backend
calls are synchronous; no total DNS/connect/merge/cancellation or process-exit
deadline is added. Transport policy remains Cycle 03's; prompts, full scans,
network and read-only merge preparation are outside the short lease. Native
worst-case merge duration is a measurement/evidence concern, not a fabricated
time bound.

Stable authoritative results retain Cycle 05 discovery and index-pending replay.
Conflict/recovery inspection comes from Git plus the journal, so an unavailable
or conflict-blocked cache cannot conceal it. Mark discovery refresh-required
and attempt the existing non-mutating refresh outside the lease when safe;
index failure never converts an unresolved merge into success or repeats it.
After resolution, return local completion and mark refresh-required; the
explicit parent restart performs remaining work and final discovery. Pure
inspection/refresh never publishes or checkpoints.

## Failure, Privacy And Platform Evidence

Errors expose fixed categories and validated target/operation/path identifiers.
Resolution/inspection bodies have custom redacted `Debug`/`Display`; opaque
observations are not source snippets. Recovery persistence contains identifiers,
OIDs, modes, digests and steps only. Scan rows, WAL/journal/backups, formatted
errors and raw fixture stdout/stderr for private source, key, passphrase,
credential URL, remote response and hostile-path canaries. Canonical worktree
files/Git blobs are intended content storage, not journal leakage.

Use genuine temporary repos and two disposable authenticated SSH clones. Tests
must inspect real conflict stages, parents, trees, worktrees and receive-pack
effects. Test-only failure seams must bracket actual Git/filesystem/SQLite
transitions. Pure graph/state tests cannot establish checkout/marker fidelity.
Extend the existing shell-free fixture and privacy harness rather than a new
server or mocks. Git server helpers remain test-only dependencies.

Native Linux x86_64/aarch64, Windows x86_64/aarch64 and macOS aarch64 must prove
locked libgit2 worktree/index, path/case, ignored-file and file-replacement
behavior. Keep CI manual-only. Add the focused test target to its existing
command, but execution needs separate authority. Mark missing native evidence
pending; obtain actual runs or explicit Cycle-specific deferral before treating
that gate as delivered. No Cycle 06 Rust verification has yet run.

## Alternatives And Risks

| Alternative / risk | Choice / consequence |
| --- | --- |
| Rebase or choose a side | Rejected by RFC; two parents and explicit caller content preserve inspectability. |
| One-parent save then merge | Rejected; loses the real conflict merge baseline or duplicates a checkpoint. |
| Atomic two-stage virtual merge only | Keep only the all-clean fast path; divergence uses explicit stages so second-stage conflicts retain meaningful first-stage progress. |
| Manyhands resolves arbitrary code/binary | Rejected by approved owned-Markdown boundary; external tools complete the merge and domain verifies it. |
| Abort/reset on a partial write | Rejected; preserve evidence and stop when re-observation cannot safely finish. No destructive rollback API. |
| Keep reservation active while user edits | Rejected; release active ownership but retain pending effect record and guard affected authoring. |
| Trust cached conflict/commit flags | Rejected; inspect actual Git/index/worktree and verify exact candidates/parents. |
| Fresh merge candidate on every retry | Rejected; record OID before branch mutation, reuse after uncertain record/push outcome. |
| Large merge/scan holds lease too long; merge preparation writes ODB blobs | Prepare with separate high-priority mempack ODB outside lease; import verified result objects under lease, revalidate before safe application. Characterize memory/disk/handle isolation and synchronous mutation. |
| Overstrict external-parent proof | Explicit recovery may require manual review of nonstandard external history; relaxing it would require proof of equivalent safe intent, not a subject match. |

## Decision Audit

Self-review uses `review-cycle-docs` and every applicable matrix row. Sources
below are approved requirements or inspected implementation, not proposed test
results. Rulings are duplicated in the ledger with cost-if-wrong.

| Source | Concern / promised behavior | Classification | Resolution | Proof / follow-up |
| --- | --- | --- | --- | --- |
| Wave 02; Git RFC ordered integration | Sequential divergence can complete context before primary conflict. | Settled | Journal both stages; retain earlier merge, resolve only pending stage. | A1/A5 ordered real graph and interruption cases. |
| Git RFC; `checkpoint_owned_paths` | Existing local save creates one parent and misses merged nonconflict entries. | Ruling | Dedicated two-parent resolution candidate from recorded merged index. | A3/A5 tree/parent/replay tests; cost if wrong: refactor writer, never weaken owned paths. |
| Git/desktop RFCs owned-path boundary | Whole context may contain code or structural conflicts. | Settled | Noncanonical/config/binary/structural entries require external tools; only eligible canonical paths can be written. | A3/A4 code/binary/rename/delete/symlink tests. |
| Self-review mixed-set ambiguity | Partial canonical writes versus external repair of the whole mixed merge changes capability/user effort. | Decision | Owner selected whole-merge external recovery on 2026-10-07; one checkpoint, no canonical writes to mixed sets. | A4 mixed-set non-mutation and external resume; acknowledged cost: no in-process canonical portion. |
| Git RFC; canonical schema | Resolved identity, stale files or unrelated staging could overwrite/commit unintended content. | Settled | Complete observations, validation, guarded writes, scoped staging; external change before effects. | A3 identity/thread/closure/traversal/race cases. |
| Git RFC conflict checkpoint | Partial multi-file writes and uncertain commit must not duplicate a resolution. | Ruling | Attempt/input/per-path digests plus prepared two-parent OID and actual-state reconciliation. | A5 fault matrix; cost if wrong: add evidence, preserve files and stop. |
| Index RFC; reservation single local phase | Two local steps cannot rewind an old monotonic checkpoint. | Ruling | Ordered child integration evidence under existing envelope/owner. | A5/A8 migration/fencing tests; cost if wrong: internal schema adaptation. |
| Locked libgit2 merge.c; git2 ODB API | In-memory merge index preparation can still write blobs into the real disk ODB. | Ruling | Separate preparation handle with high-priority mempack; verify no destination writes/other-handle effects, import only prepared result objects under lease. | A1/A8 backend characterization and object/ref fault tests; cost if wrong: reassess preparation seam, no silent long lease/disk fallback. |
| Git RFC primary synchronization | Primary commit subjects not item-specific. | Ruling | Fixed `Merge remote primary`/`Resolve synchronization primary`. | A1 subject tests; cost if wrong: non-secret wording only. |
| Git identity RFC | New commit paths require effective or caller-confirmed identity. | Settled | Existing identity helpers; fast-forward needs no identity; caller-confirmed values only when effective identity is missing. | A3 missing/partial/config-change tests. |
| Same-ID replay; identity-required retry | Adding missing identity must not silently change bound semantic input or invent a timestamped merge again. | Ruling | Auxiliary confirmation ID/input/config-observation evidence, like host approval; only before candidate effects, reuse candidate thereafter. | A3/A5 confirmation/partial-config-write/replay tests; cost if wrong: stricter identity recovery, not overwrite. |
| Cycle 05 push/absence contract; auth RFC | Retry might contact wrong endpoint, recreate deletion or persist secrets. | Settled | Reuse direction-qualified transport/evidence; no new remote stages or fallback. | A6/A8 SSH and privacy regressions. |
| Time/failure matrix; runtime RFC | Cancellation must not promise interruption inside synchronous calls. | Settled | Named safe points, no total timeout, no automatic retry loop; retained ownership/evidence until safe stop. | A8 cancellation/lease tests and timing qualification. |
| Git RFC external repair | Accepting marker edits or an arbitrary external commit is unsafe. | Ruling | Require clean exact recorded-parent merge and eligible canonical state before resume. | A4/A5 external repair rejection/acceptance; cost if wrong: stricter manual recovery, no silent loss. |
| Test RFC; manual-only CI; prior evidence | Local fixture cannot prove every native ABI/path case; prior deferral is not current approval. | Settled | Real fixture locally; future authorized native runs or explicit Cycle-specific deferral. | A9 honest evidence register. |
| Lifecycle skill/AGENTS | Planning approval could imply code, publication or premature closure. | Settled | Approval then explicit implementation; review-ready comments, review/PR approval, separately authorized pre-merge closure/merge/cleanup. | A9 ticket checkpoints. |

No material product/security/recovery/scope decision remains open in this
proposal. Implementation characterization may reveal a feasibility blocker;
stop and amend the documents rather than silently relaxing these contracts.
