---
title: "Wave 03 Provisional API and Ownership Audit"
date: 2026-10-06
status: in-progress
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48VZ76MZ72AQY9C7R4ZAWQV"
---

# Wave 03 Provisional API and Ownership Audit

> **Replan note (2026-10-07).** Wave 03 was replanned into a foundation, two
> parallel tracks and a joint gate after this record was written. The
> `W3-01`…`W3-13` owner codes here use the 2026-10-05 numbering. Translate them with the mapping
> in [Wave 03, Structure And Identifiers](../Waves/wave-03-dogfooding.md#structure-and-identifiers):
> 01 → F1 and C1; 02 → F2; 03 → F2 and C2; 04 → F1, F2 and C3; 05 → C4;
> 06 → C5; 07 → D1; 08 → D2; 09 → D3; 10 → D4; 11 → D5; 12 → D6; 13 → G1.
> The complete Wave 02 integration gate is now required before G1 exits, not
> before the first Wave 03 Cycle; each Cycle names its own Wave 02
> dependencies. The findings and evidence below are otherwise unchanged.

## Authority, revision and limits

Task 2 only, under the approved [exploration](wave-03-readiness-exploration.md),
[design](../plans/2026-10-06-wave-03-readiness-exploration-design.md#api-inventory-design),
[implementation plan](../plans/2026-10-06-wave-03-readiness-exploration-implementation.md#task-2--inventory-actual-apis-and-ownership)
and [execution ledger](../plans/2026-10-06-wave-03-readiness-exploration-execution.md).
[Ticket 01M48S808PF2D8ZWVYM918RK2M](../../.manyhands/tickets/01M48S808PF2D8ZWVYM918RK2M/ticket.md)
remains open. Governing contracts are the [CLI RFC](../RFC/cli-contract.md),
[desktop RFC](../RFC/desktop-information-architecture-and-editor.md),
[runtime RFC](../RFC/application-runtime-and-polling.md),
[canonical schema](../RFC/canonical-content-and-comment-schema.md),
[Wave 02](../Waves/wave-02-collaboration.md) and
[Wave 03 ownership map](../Waves/wave-03-dogfooding.md#capability-ownership-and-api-audit).

- Start/inspected HEAD: `a416d836a10702afeba8f25000b3aa58fbed13a2` on
  `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`; initial worktree/index clean.
- Main baseline: `29f3f5a25957836a8513cba8a318306e1b928063`. The range to
  inspected HEAD changes planning/ticket documents only, not audited source/tests.
  Wave 02 01–04 complete; 05–10 are **future contracts**, not available APIs.
- This is **source-inspected** evidence, including test definitions/assertions.
  No Rust command, service constructor, API call, key access or remote contact
  was executed by this audit. Tests cited below were **not run by this task**.
  The separate Task 1 artifact reports six passing baseline commands at this
  HEAD; that external report neither proves these future contracts nor replaces
  independent baseline review. Source-only acceptance does not depend on an
  executable baseline being green.
- Public reachability was traced from [lib.rs](../../src/lib.rs#L3),
  [repository module/export boundary](../../src/repository.rs#L28),
  [keys exports](../../src/repository/keys/mod.rs#L1), and
  [transport exports](../../src/repository/transport/mod.rs#L18).
  No unfinished other-worktree implementation is evidence here.
- This document is not final-main readiness, permission to start Wave 03,
  editor selection, publication, ticket closure or a service implementation.

## Checklist and reading key

The CLI RFC's 36 grouped taxonomy rows expand to **56 distinct two-word verbs**.
Add three invocation capabilities: no arguments, help, version: **59 CLI rows**.
The supplementary checklist below has **31 desktop-only capabilities** and
**16 runtime capabilities**: **106 rows total**. Shared operations are not
repeated as desktop verbs: desktop Settings/Save/Sync/Promote/Close/Conflict/
Poll/Refresh invoke the corresponding CLI inventory's domain boundary, with
extra desktop behavior recorded separately. Rich vocabulary is one capability
with all required constructs enumerated, not an omitted subset.

Each inventory has the design's eight columns. The request/outcome/test catalog
is part of every row referencing its evidence code. Cross-cutting contracts C0–C6
are likewise part of each row, including explicit missing guarantees. `None`
means no existing service or tests for the requested behavior, not success.

Evidence statuses:

- **partially-present**: an actual primitive exists but not the complete request,
  DTO or consumer guarantee; cited tests cover only that primitive.
- **present-but-unverified**: the named narrow capability exists; current-task
  execution and frontend integration remain unverified, even with test source.
- **absent**: no requested public service/adapter exists; adjacent helpers are
  explicitly non-equivalent.
- **pending-Wave-02**: a collaboration requirement awaits its approved owner;
  available state/transport scaffolding is not the operation.
- No row is `present-and-tested`: this task ran no executable tests.

Owners use `W1-03`, `W2-04`, `W3-01`, etc. Consumers are planned Cycles, not
claims that their implementation exists. Acceptance proof and re-audit codes:

- **R-read**: W3-01 must prove repository/common identity resolution, safe reads,
  complete deterministic DTOs (including malformed/stale entries), no implicit
  refresh/network, public schemas/redaction. Recheck every read against final
  completed-Wave-02 main, and whenever exports/cache schema/read paths change.
- **R-adapter**: W3-02 must prove exact intent digests, absence/base observations,
  accepted consent and expiry, lost-output/partial replay/cache loss, typed
  effects/progress/cancel. Recheck final-main requests/outcomes/preflights and
  any changed observation, operation, reservation or recovery implementation.
- **R-admin**: W3-03 must prove confirmed local identity/config/key/host effects,
  terminal-only secrets, deletion retry and imported non-deletion; depends on
  R-read/R-adapter. Recheck final main and credential/endpoint/storage changes.
- **R-author**: W3-04 must prove scoped authoring, moves, explicit folder/repair/
  adoption and closure filters; W3-09/10 consume later. Recheck final main,
  serialization/no-op paths and canonical schema/context/closure changes.
- **R-domain**: named W2-05…10 must merge with their Wave exit evidence before
  W3-05/06/10/11/12 plans consume them. Re-audit actual public types, completed
  effects, interruption/cache loss, conflict/consent and cleanup ordering on
  final-main; advertisement or private transfer tests cannot satisfy this.
- **R-draft**: W3-08 proves protected/versioned atomic files, flush failure,
  crash/migration, revision retirement, stale comparison and export/discard.
  Recheck before W3-09/10 and on save/context/draft format changes.
- **R-editor**: W3-09 requires accepted pinned feasibility plus actual editor
  byte/undo/resource/native evidence; W3-13 completes platform/journey matrix.
  Recheck candidate/GPUI graph and final-main save/read contracts before planning.
- **R-worker**: W3-12 must characterize final locked transport stalls first,
  then prove one process worker, policy/fairness/time changes, credential and
  safe shutdown behavior over W2-08. Recheck final-main transport/policy/
  reservation APIs, dependency lock changes and platform evidence.
- **R-shell**: W3-07 proves owned async models, stale-generation rejection,
  keyboard navigation/focus and Root-first startup; W3-13 owns remaining native
  acceptance. Recheck final-main DTOs/adapter and Kit APIs/native matrix.

An unresolved owner is a **consumer-planning blocker**, not permission to assign
work to a late consumer. The gaps here have the explicit approved owners above;
new gaps at final-main re-audit must be escalated if they do not fit that map.

## C0 — Invocation, DTOs, errors and safe reads

[CLI main](../../src/bin/manyhands-cli.rs#L1) has only transport initialization:
no parser, help, version, schemas, JSON envelope, stdin handling, prompts,
request IDs, exit-class adapter or dispatch. It initializes even for help-like
arguments; no-argument success is silent, not RFC help. Startup failure emits
plain stderr/exit 1. W3-01 must parse service-free help/version while preserving
pre-thread initialization for actual operations and recognizable JSON failures.
W3-01 owns complete read DTOs and command schemas; each mutating CLI Cycle owns
its schemas, with complete coverage gated at W3-06. No directly serialized Rust
enum is an RFC envelope. W3-02 owns stable completed-effect/error mapping.

`RepositoryService::open_default/open_at` [908–915](../../src/repository.rs#L908)
reach [2944–2982](../../src/repository.rs#L2944): create application data directory,
open/migrate SQLite, audit remote history, use a cache-recovery lock, and mark
structural corruption degraded. They are not service-free read constructors.
`inspect` itself reads local Git/config/identity/remotes without enabling or
refreshing. `repository_snapshot` uses read-only SQLite and rollback, but its
[cache guard](../../src/repository/coordination.rs#L118) opens/creates a writable
recovery lock file. `recovery_inspection` uses the same guard. `remote_snapshot`
delegates to snapshot; `active_remote_operation` uses an immediate read-write
transaction. `list_shared_keys` opens/migrates a writable registry.
`list_key_material_recovery` also calls
[material_registry](../../src/repository/keys/generation.rs#L205), taking an
exclusive `cache_write_guard` and opening writable SQLite through
[open_registry](../../src/repository/discovery.rs#L1062). It may create the
SQLite database and requests WAL setup even if the recovery query then fails;
this is not a read-only SQLite recovery inspector. A read DTO must distinguish
permitted application-local bookkeeping from canonical/Git
mutation; do not assign these wholesale to service-free help or claim zero
filesystem effects. Safe-read assertions exist for snapshot/inspect, not all
future DTOs.

The private [canonical_repository_root](../../src/repository.rs#L7225) requires
an exact working-directory root, allows linked worktree roots, and uses common
Git directory only for leases; it does **not** implement the RFC's complete
common-identity frontend resolver. Snapshot root normalization is path-based.
`DiscoveredItem` is metadata, path/context/activity/comment IDs only: no body,
source, unknown metadata, closed_by, observation or author. Malformed entries
appear as separate `DiscoveryProblem`s, not full nullable item/comment DTOs.
Private `discovery::observe_root` collects source for scanning, not public reads.
`canonical::parse_item/validate_context` accept supplied strings; they do not
read a selected repository safely or create opaque context observations.

[RepositoryError](../../src/repository.rs#L801) has typed kind/context, optional
external-change diagnostic and backend source. Display includes message and
source chains can expose backend details. Canonical YAML errors can include
parser text. [RemoteInfo](../../src/repository.rs#L498) contains **raw URLs**
([8099](../../src/repository.rs#L8099)); these may contain embedded credentials.
W3-01/02 must emit fixed redacted categories, sanitize URLs, never dump source
chains/Debug/SQL. Requested `show`/conflict sources are intentional read payloads;
mutation/errors/progress/operation records must not echo bodies or credentials.
Lists need RFC sorting, filters, `complete: true` and no silent truncation;
scanner depth/entry/history limits are not proof of complete frontend lists.

## C1 — Target observations, authoring and replay/effects

[ExpectedPathObservation](../../src/repository.rs#L167) is Missing or exact-byte
Blake3, not the opaque source/destination/context token required by frontends.
New authoring expects Missing; document save/move requires a destination
observation, but [SaveDocumentRequest.expected_source](../../src/repository.rs#L413)
is **optional**. Source comparison before reading ([1593–1607](../../src/repository.rs#L1593))
and before removal ([1757–1774](../../src/repository.rs#L1757)) only runs for
`Some` (including the internally supplied Missing observation on completed-move
replay). With `None`, an edit/move can read the current source and a move can
remove it without comparing the caller's observed source bytes. Mandatory source
observation and rejection of its omission are **absent guarantees**, not enforced
by the current public request. The [existing stale-source test](../../tests/local_authoring.rs#L5850)
supplies `Some`; it does not prove omission is rejected. W3-02/04 must require a
source observation for edit/move and add tests submitting `None` that reject
before write/removal/checkpoint, preserving both paths and repository/worktree
state, alongside supplied-stale rejection and eligible move-replay tests.
Tickets have one path precondition, comments destination absence. A supplied
stale observation returns typed `ExternalChange` and preserves canonical state. `AuthoringTarget`
includes root/kind/ULID/intent/operation ID; `prepare_context` creates/reuses a
branch/worktree, journals and hands off discovery. Viewing must not call it.

Local operations use `OperationId`; there is no external `request_id` mapping.
Private [begin_or_reconcile_operation](../../src/repository/recovery.rs#L230)
checks root/action/target and pending operations. Authoring target strings bind
kind/ID/intent/path, **not full semantic body/metadata**. Remote-add targets hash
name/URL. Replays also compare observed owned files/serialized results. This is
bounded domain reconciliation, not the RFC's altered-input digest guarantee.
W3-02 must extend the existing boundary, not invent another journal/lease or
have UI/CLI query SQL/Git directly. Generation replay checks label/store/action
but [not protection mode](../../src/repository/keys/generation.rs#L400);
W3-02 must match non-secret protection intent without passphrase-derived hashes.

`SaveOutcome`/`CommentSubmissionOutcome` distinguish IdentityRequired, Saved,
IndexPending; `LocalCheckpoint` distinguishes NoChange, committed and refresh
pending with OID. Errors after effects do not provide a universal effects DTO;
re-observation/recovery is required. `RecoveryInspection` lists pending local
records only, not all completed operations or original body. Cache rebuild can
lose intent/consent/history: no indefinite exactly-once claim. If Git/canonical
facts cannot prove replay, W3-02 must return recovery_required, ask for original
input where needed and never regenerate a comment ID or infer publication.

Document edit serializes then writes even an equivalent parsed item
([1640](../../src/repository.rs#L1640), [1730](../../src/repository.rs#L1730));
YAML formatting/CRLF header may change, possibly creating a commit. Ticket edit
compares semantic `CanonicalItem` before writing ([2009](../../src/repository.rs#L2009)).
Canonical serialization preserves unknown YAML **values**/body, not original
header bytes. Neither serializer tests nor `NoChange` name prove the universal
byte-preserving no-change UI save contract. W3-02/04/09 must prove no-write,
no-commit no-op behavior and preserve untouched source. Ticket draft cannot
supply closure fields; edit retains them. The current save layer has no complete
lifecycle-closed write/reopen guard; re-audit with W2-10 before exposing edits.
Comments lack an author field in the canonical model; read DTO author provenance
needs W3-01's explicit treatment, not invented persisted metadata.

## C2 — Consent, progress, cancellation and worker boundary

There is no shared prepare/confirm ID, ten-minute expiry, exact effect preview,
accepted consent replay or absent-root observation service. W3-02 owns these
adapter additions over W2 lifecycle preflights; W3-03/04/05/06 consume them.
Creation previews must not initialize Git; remote removal previews must show
poll/publication effects; lifecycle previews enumerate **whole branch** paths,
OIDs, remote and cleanup. Changed targets invalidate unused consent; accepted
unchanged remaining work can replay. Repairs cannot adopt implicitly.

Local authoring/enable/index methods are synchronous without public progress or
cancel callbacks. Refresh briefly holds repository lease, starts an internal
index-owner heartbeat thread, scans unleased, re-observes and persists stable
contexts; this is not a desktop poller. W3-02 adds typed bounded-worker progress,
request-generation tagging and cancellation at actual approved safe points,
not thread killing or rollback claims. CLI must produce one final JSON object,
stdout free of prompts/progress/logs, with cancellation/partial/input/recovery/
transient/internal/success precedence (130/4/2/3/5/1/0). Indexing failures after
checkpoint or configured publication failure are partial, not simple failure.

W2-04 reservations have durable cancel/yield requests and named safe points.
`cancel_remote_operation` requests cancellation; blocking DNS/connect/SSH must
return before safe-point acknowledgement. A completed batch stays completed
when cancellation follows it. `remote_safe_point`, `finish_remote_operation`
and restart operate on service-bound ownership tokens, not generic UI control.
W3-02 maps this progress; W3-12 characterizes stalls, stop feedback at ten seconds
and safe waiting, retaining ownership/secrets until worker use ends. No total
operation deadline follows from per-address/per-call timeouts.

## C3 — Credentials, trust and privacy

Local content/config/index operations need Git commit identity when checkpointing,
not an SSH unlock. Imported registration stores references and optional public
fingerprint, no private parse/copy/delete. Generated storage protects owned files;
store configuration is non-writing but store locking/review can create protected
SSH directories/lock files. Public-key text by ID is missing despite fingerprints.
Selection is shared registry state, unrelated to commit identity; session
invalidation on selection changes is reconciled at use, not pushed to every
process's cached value immediately.

`SessionCredentials<P>` owns zeroizing passphrases and redacted Debug. Successful
validation caches by selected key/source token; failure, cancellation, explicit
clear and changed source invalidate as implemented. CLI provider must be masked
controlling-terminal only; JSON/noninteractive returns unlock_required, never
stdin/argv/env/files/agent. Desktop supplies one process session shared with its
worker, deduplicates prompts and separates unlock/trust attention from durable
user pause. Providers run outside short leases; selection/source/endpoint is
rechecked after prompts/connection. No secret-bearing Git handle crosses UI state.
Existing single-session tests do not prove multi-window prompt deduplication.

`verify_ssh_transport` is a public selected-key **network** verification, not a
local inspector. `HostApproval` binds authority plus expected/presented identities;
pins are persisted only after authenticated re-observation. The actual
[callback](../../src/repository/transport/callbacks.rs#L72) accepts a matching app
pin or exact approval; with no app pin/approval/recovery fence it returns
CertificatePassthrough to the backend known-hosts verifier. That is not automatic
application pin creation or a public host inventory. Credentials submit only the
selected private-key path (one key attempt, no agent/password/default-key
fallback); prompts are outside callbacks. W3-03 can use verification without
fetching/pushing, but exact authority must match configured endpoint.
`read_host_pin`/`finalize_host_pin` are restricted test helpers; `read_host_trust`
is repository-internal, no public host list/inspect. Registry-loss trust fence
requires explicit reapproval; it must not become blind trust. Transport/key
errors use fixed guidance and tests probe privacy, but frontend URL/parser/error
redaction remains missing (C0). Confirmation never accepts a blanket yes for
host replacement or generated deletion.

## C4 — Advertisement is not collaboration

W2-04 `observe_publication_remote` authenticates Fetch-direction advertisement,
reads local tracking OIDs and writes durable batch/policy/outcome/host-trust state.
It **does not transfer objects/refs, fast-forward, merge, push, materialize a
worktree, checkpoint, refresh local discovery, promote, close or clean up**.
It trusts caller eligibility: it does not schedule/sleep/override pause. Exact
ref plans/classifications are not proof of canonical tree validity. Cache loss
sets history-unknown/recovery suspension, not remote deletion proof.

`with_authenticated_remote`/`AuthenticatedSshRemote::{download,push}` are
crate-private, presently consumed by fixtures; transfer tests are not public
sync APIs. `RemoteOperationAction` contains future action names, not operations.
W2-05 owns clean deliberate sync, W2-06 merge/conflict resolution, W2-07 compound
comment publication, W2-08 one-shot poll/clean updates/materialization, W2-09
promotion and W2-10 closure. Their source/outcome tests are absent here.

At the original audit base, `submit_comment` only local checkpoint/discovery: no remote gives PublishPending;
configured remote gives SyncDeferred. It is **not** Post and sync. Reuse comment
ID/timestamp/file/checkpoint on retry; W2-07 must add publication-only continuation
without resubmission. Ordinary save remains local. Automatic poll must never
push/checkpoint/merge/cleanup. Previously published remote-deleted branches
require explicit republish consent, not ordinary silent sync.

**Cycle 07 implementation update — 2026-10-10 (verification in progress):**
`RepositoryService::submit_comment<P>(PublishCommentRequest,
&mut SessionCredentials<P>)` replaces the local-only signature. The request
wraps the unchanged `SubmitCommentRequest` draft plus optional `HostApproval`
and `ConfirmedCommitIdentity`; identity confirmation goes to child sync only.
`CommentSubmissionOutcome::Saved` carries an original `CommentReceipt`
(action/comment/item/parent IDs, bound synchronization ID, original checkpoint
OID and derived context/path), `CommentPublicationState::{Published,
AlreadyCurrent, Pending}` and independent `CommentIndexingState` local/remote
pending flags. There is no normal `SyncDeferred` outcome. `IdentityRequired`
remains pre-write; other pre-checkpoint failures use redacted
`CommentSubmissionError`. The additional `context`/`checkpoint` Saved fields
describe local effects of this call, not publication authority.

`retry_comment_publication<P>(RetryCommentPublicationRequest, &mut
SessionCredentials<P>)` takes the original operation ID, root, optional host/
identity controls and explicit restart; it accepts no body. The bound child ID
keys existing Cycle 06 inspection/resolution. `cancel_comment_publication`
looks up that child and requests only existing active safe-point cancellation.
Comments in another writable context may checkpoint while a conflict exists,
then report pending/Busy; authoring inside the context owned by the pending
merge is refused before checkpointing. Whole already-checkpointed context
history can publish; unsaved item buffers are not saved. Focused local tests
are executed during this update; full Cycle acceptance and native evidence
remain pending Tasks 2–6 in the Cycle 07 execution ledger.

## C5 — Desktop-only state and editor obligations

[Desktop main](../../src/main.rs#L1) is HelloWorld/Button; it initializes Kit but
allocates child before Root ([38](../../src/main.rs#L38)), not Root-first proof.
No repository UI/editor/draft/scheduler models exist. Navigation/tab identity is
resolved common repository + kind + full ULID; viewing primary does not provision.
Async responses need request/generation checks; dirty drafts survive repository/
context disappearance, refresh, polling and stale completion. Domain work is
bounded/off-UI-thread, with owned DTOs only, no frontend SQL/Git shortcuts.

Draft store (W3-08) is protected application-local versioned atomic files, **not**
SQLite/journal/canonical state: one-second idle persistence, explicit flush,
visible failure, preserved originals on migration failure, no age eviction.
Restore never overwrites canonical or automatically reopens closed/incompatible
contexts. Save retires only proved checkpointed draft revision, preserving newer
text. Close choices are Save/Keep draft/Discard/Cancel, discard scoped explicitly.

W3-09 must integrate accepted pinned native editor: rich default, exact body
source, named metadata and full canonical repair source separately. Preserve
original header/body/unknown values, untouched/unsupported blocks and CRLF;
rendering cannot serialize whole document. Shared draft/selection/undo across
modes; no save on toggle; parser failure retains usable source with diagnostics.
Resources deny HTML execution, remote images, traversal/symlink escape,
executable schemes/private-key locations. Internal renderer access must be
contained too. Keyboard/IME/clipboard/accessibility/high-DPI/performance require
native evidence, not source inspection or toy String-model tests. The 1,000-item/
100 KiB p95 rendered-input and cancellation-feedback targets belong to integrated
W3-13 evidence, not this inventory.

## C6 — Runtime policy, sessions and shutdown obligations

`RemotePollingConfiguration` currently stores enabled/paused/60–3600-second
interval/backoff/recovery suspension, with explicit-vs-automatic delay logic.
`set_remote_polling` updates all policy fields without a request ID or consent;
`resume_remote_polling_after_recovery` clears **recovery suspension**, not pause.
Policy/configuration changes invalidate observations; no complete scheduler or
completion-derived next-time/status DTO exists. W2-08 must provide full attempt
results; W3-01/02 read/adapt them, W3-12 schedules them.

One desktop-owned worker across windows, sequential due repositories with manual
work checks, initial eligible launch poll, no catch-up after restart/suspend,
monotonic waits + UTC persisted deadlines, rollback clamp, 60-second doubling
backoff capped 900 seconds, busy/yield/cancel not network failure, explicit Poll
now bypassing pause/backoff without clearing pause: these are future requirements,
not current `delay_for` guarantees. No resident CLI poller/service/process registry,
IPC or multiple-resident-poller election is authorized. Removing registration
stops scheduling; queued remote/key observations invalidate; active work stops/
reconciles safely before changed config. Unlock/host attention is distinct from
persisted pause; explicit unlock clears only its block.

Shutdown stops admission, requests safe cancel, flushes progress/drafts, stays
responsive with still-stopping feedback at ten seconds, waits safely, then clears
credentials after use. Forced termination is interruption, not rollback; no
reservation reclamation or arbitrary worker killing while live. Existing unsafe
startup initializer must run before threads; 10-second TCP/address and 30-second
blocking SSH-call settings leave DNS and total transfer duration unbounded.

## Request, outcome and test evidence catalog

All catalog tests are source/assertion evidence only, **not executed here**.
Line anchors identify inspected definitions at the recorded HEAD.

| Code | Public request/outcome and implementation | Existing test evidence and limitation |
| --- | --- | --- |
| E-id | [ItemId::generate](../../src/canonical.rs#L19), FromStr L32; no request/outcome wrapper | [generated_item_ids_display_as_reparsable_canonical_spelling](../../tests/canonical_foundation.rs#L141); no CLI request/new envelope |
| E-canon | [parse_item](../../src/canonical.rs#L159), [validate_context](../../src/canonical.rs#L236), [ordered_comment_threads](../../src/canonical.rs#L386), [serialize_item](../../src/canonical.rs#L605); CanonicalItem/ValidatedContext/ValidationProblem | [valid_document_round_trip_preserves_unknown_yaml_and_exact_body](../../tests/canonical_foundation.rs#L362), ticket/comment equivalents L369–399; [comment_order_is_deterministic_after_serialization_and_reparse](../../tests/canonical_foundation.rs#L964). Pure supplied-string tests, not safe repo DTOs/no-op header preservation |
| E-inspect | [inspect](../../src/repository.rs#L2984), root → RepositoryInspection L475; IdentityInspection L492 has Available/Required only | [inspect_born_repository_reports_its_local_state](../../tests/repository_enablement.rs#L894), [inspect_valid_configuration_preserves_source_bytes](../../tests/repository_enablement.rs#L916), [identity_uses_common_local_config_when_inspecting_a_linked_worktree](../../tests/repository_enablement.rs#L1145); no name/email/source DTO |
| E-enable | [enable](../../src/repository.rs#L2989), EnableRepositoryRequest L291 → EnableRepositoryOutcome L506; [create_and_enable](../../src/repository.rs#L3427), CreateRepositoryRequest L283 → same outcome | [enable_born_main_writes_canonical_config_exclusion_and_single_file_commit](../../tests/repository_enablement.rs#L1169), [enable_identity_required_precedes_exclude_configuration_and_commit_writes](../../tests/repository_enablement.rs#L2786), [create_replay_after_repository_initialization_uses_the_observed_step](../../tests/recovery_foundation_gate.rs#L1388). No creation preview/request digest |
| E-remotes | [list_remotes](../../src/repository.rs#L2997) → Vec<RemoteInfo>; [add_remote](../../src/repository.rs#L3003), AddRemoteRequest L299; [remove_remote](../../src/repository.rs#L3097), RemoveRemoteRequest L314 → RemoteOutcome L515; [set_publication_remote](../../src/repository.rs#L3193), SetPublicationRemoteRequest L307 → PublicationRemoteOutcome L528 | [remote_listing_addition_and_removal_use_only_local_configuration](../../tests/repository_enablement.rs#L1287), [publication_selection_commits_only_canonical_configuration_changes](../../tests/repository_enablement.rs#L1655), [remote_replay_rejects_a_different_url_for_the_same_operation_id](../../tests/recovery_foundation_gate.rs#L1314); raw URL/consent gap |
| E-remove | [remove_registration](../../src/repository.rs#L4171), RemoveRegistrationRequest L321 → RemoveRegistrationOutcome L536 | [remove_registration_only_deletes_its_canonical_registry_row](../../tests/repository_enablement.rs#L2366), [registration_removal_replay_preserves_failed_state_then_removes_once_and_noops](../../tests/recovery_foundation_gate.rs#L1656). Deletes registry/recovery rows, not files/drafts; no confirmation |
| E-context | [prepare_context](../../src/repository.rs#L1211), AuthoringTarget L369 → ContextProvisionOutcome L385 / ItemContext L377 | [prepare_context_creates_a_document_context_at_its_deterministic_location](../../tests/local_authoring.rs#L77), [context_replay_worktree_interruption_uses_one_journaled_context](../../tests/local_authoring.rs#L5685); mutation, not viewing |
| E-document | [save_document](../../src/repository.rs#L1453), SaveDocumentRequest L408 / DocumentDraft L392 → SaveOutcome L439 / LocalCheckpoint L433 | [document_create_writes_canonical_markdown_and_a_scoped_checkpoint](../../tests/local_authoring.rs#L1575), [document_move_removes_source_and_checkpoints_only_the_owned_pair](../../tests/local_authoring.rs#L1656), [stale_document_edit_preserves_external_replacement](../../tests/local_authoring.rs#L5807), [recovery_document_registry_failure_returns_refresh_pending_and_retry_does_not_commit_again](../../tests/local_authoring.rs#L2706); destination observation mandatory, source comparison only for Some; no missing-source-observation rejection proof, assigned W3-02/04 with C1 tests; C1 no-op/digest gaps |
| E-ticket | [save_ticket](../../src/repository.rs#L1837), SaveTicketRequest L418 / TicketDraft L398 → SaveOutcome / LocalCheckpoint | [ticket_create_writes_exact_canonical_body_and_scoped_checkpoint](../../tests/local_authoring.rs#L3088), [ticket_edit_preserves_unknown_closure_and_exact_body](../../tests/local_authoring.rs#L3132), [stale_ticket_edit_preserves_pre_save_repository_and_worktree_state](../../tests/local_authoring.rs#L5966); no W2-10 lifecycle/closed-edit proof |
| E-comment | [submit_comment and retry_comment_publication](../../src/repository/comment_publication.rs), PublishCommentRequest / RetryCommentPublicationRequest + caller SessionCredentials → CommentSubmissionOutcome, original CommentReceipt, publication and indexing states | [local authoring regressions](../../tests/local_authoring.rs), [receipt/identity/rejection/reconciliation tests](../../src/repository/comment_publication_tests.rs); Cycle 07 implementation in progress, real-SSH and native acceptance pending |
| E-index | [repository_snapshot](../../src/repository.rs#L1160), root → RepositorySnapshot L542 / DiscoveredItem L588 / DiscoveryProblem L613; [refresh_repository](../../src/repository.rs#L970), RefreshRepositoryRequest L327 → RefreshOutcome L622; [rebuild_repository](../../src/repository.rs#L1106), RebuildRepositoryRequest L333 → RepositorySnapshot | [snapshot_reads_a_registered_cache_without_mutating_available_state](../../tests/discovery_rebuild.rs#L537), [discovery_public_types_hold_metadata_only](../../tests/discovery_rebuild.rs#L434), [refresh_requires_registration_but_rebuild_registers_the_explicit_root](../../tests/discovery_rebuild.rs#L567), [refresh_scan_race_retains_previous_rows_then_converges_on_retry](../../tests/discovery_rebuild.rs#L867), [rebuild_corrupt_cache_restores_only_the_explicit_root](../../tests/discovery_rebuild.rs#L1930); cache/lock effects, metadata-only |
| E-recovery | [recovery_inspection](../../src/repository.rs#L1198), root → Vec<RecoveryInspection> L339; private pending_for_root in recovery.rs L525 | [pending_lifecycle_records_block_differently_identified_mutations_without_side_effects](../../tests/recovery_foundation_gate.rs#L790), [fresh_service_replays_each_wave_one_failure_without_duplicate_artifacts](../../tests/recovery_foundation_gate.rs#L2007); exact typed original calls, not generic list/show/resume service |
| E-keyreg | [register_shared_key](../../src/repository/keys/registry.rs#L25), RegisterSharedKeyRequest in keys/mod.rs L94 → RegisterSharedKeyOutcome; [list_shared_keys](../../src/repository/keys/registry.rs#L107) → Vec<SharedKeyRegistration>; [select_shared_key](../../src/repository/keys/registry.rs#L151), [clear_shared_key_selection](../../src/repository/keys/registry.rs#L206) → SharedKeySelectionOutcome; [unregister_shared_key](../../src/repository/keys/registry.rs#L237) → UnregisterSharedKeyOutcome | [registration_survives_reopen_and_remains_unselected](../../tests/shared_key_registry.rs#L811), [selection_replaces_the_selected_key_and_survives_reopen](../../tests/shared_key_registry.rs#L1236), [unregistering_requires_clearing_the_selected_key_and_preserves_key_fixtures](../../tests/shared_key_registry.rs#L1413); no request IDs/public text read |
| E-keygen | [generate_shared_key](../../src/repository/keys/generation.rs#L45), GenerateSharedKeyRequest / KeyProtection / GenerateSharedKeyOutcome in [keys/mod.rs L294–320](../../src/repository/keys/mod.rs#L294); [list_key_material_recovery](../../src/repository/keys/generation.rs#L193) → Vec<KeyMaterialRecovery>; recovery listing takes an exclusive cache guard and writable open_registry, with possible SQLite creation/WAL setup (C0) | [generation_plain_and_encrypted_round_trip](../../tests/key_material.rs#L25), [generation_replay_never_creates_a_second_pair](../../tests/key_material.rs#L116), [generation_crash_gap_does_not_adopt_unproven_files](../../tests/key_material.rs#L313); protection-intent/cache-loss adapter gap |
| E-keydelete | [preflight_generated_key_deletion](../../src/repository/keys/registry.rs#L126) → GeneratedKeyDeletionPreflight; [review_generated_key_deletion](../../src/repository/keys/deletion.rs#L41), KeyStore/key ID → one-use GeneratedKeyDeletionReview; [delete_generated_key](../../src/repository/keys/deletion.rs#L59), store/op ID/review/confirmed → DeleteGeneratedKeyOutcome | [deletion_cancel_writes_nothing](../../tests/key_material.rs#L1026), [deletion_refuses_imported_selected_and_unproven_generated_rows](../../tests/key_material.rs#L1076), [deletion_retry_requires_fresh_confirmation](../../tests/key_material.rs#L1180), [deletion_retry_preserves_replacement_files](../../tests/key_material.rs#L1289); no serializable ten-minute confirmation service |
| E-session | [inspect_selected_key](../../src/repository/keys/inspection.rs#L9) → SelectedKeyInspection; [unlock_generated_key](../../src/repository/keys/inspection.rs#L36) → GeneratedKeyUnlockOutcome; [SessionCredentials](../../src/repository/keys/session.rs#L260), new/clear/invalidate/with_passphrase L277/285/296/330; [KeyStore::for_home](../../src/repository/keys/storage.rs#L34) | [successful_unlock_prompts_once_per_session](../../tests/session_credentials.rs#L54), [new_key_or_source_evicts_secret](../../tests/session_credentials.rs#L153), [credential_formatting_is_redacted](../../tests/session_credentials.rs#L236), [generated_unlock_cancel_preserves_state](../../tests/key_material.rs#L668), [store_configuration_does_not_create_ssh_directory](../../tests/key_storage.rs#L4); no frontend provider/prompt broker |
| E-ssh | [verify_ssh_transport](../../src/repository/transport/operation.rs#L14), VerifySshTransportRequest / HostApproval / SshTransportVerified in [transport/mod.rs L50–72](../../src/repository/transport/mod.rs#L50); SshTransportErrorKind/guidance in [error.rs](../../src/repository/transport/error.rs#L17) | [transport_callback_host_decision_table](../../src/repository/transport/tests.rs#L118), [state_rechecks](../../tests/ssh_transport/state.rs#L27), [host_and_key_failures](../../tests/ssh_transport/observation.rs#L156); custom SSH fixture source, no public host-read API |
| E-observe | [observe_publication_remote](../../src/repository/remote/observation.rs#L114), ObservePublicationRemoteRequest L71 → RemoteObservationOutcome [state.rs L1572](../../src/repository/remote/state.rs#L1572) or RemoteObservationError L84; RemoteRefPlan [refs.rs L34](../../src/repository/remote/refs.rs#L34) | [authenticated_exceptional_states_preserve_local_contexts](../../tests/remote_observation.rs#L493), [authenticated_absence_after_cache_loss_stays_history_unknown](../../tests/remote_observation.rs#L642), [authenticated_batch_failure_is_atomic_and_restartable](../../tests/remote_observation.rs#L836), [malformed_redaction](../../tests/ssh_transport/observation.rs#L429); advertisement, not poll/sync |
| E-policy | [remote_snapshot](../../src/repository/remote/state.rs#L1512) → RemoteSnapshot; [set_remote_polling](../../src/repository/remote/state.rs#L1517), root/enabled/paused/PollingInterval → Result<(), RepositoryError>; [resume_remote_polling_after_recovery](../../src/repository/remote/state.rs#L1499); RemotePollingConfiguration/delay_for L250/300 | [polling_policy_survives_reopen_and_local_snapshot_stays_intact](../../tests/remote_observation.rs#L167), [explicit_recovery_resume_preserves_pause_and_unknown_history](../../tests/remote_observation.rs#L245), [automatic_backoff](../../tests/ssh_transport/observation.rs#L498); no scheduler/complete next-attempt DTO |
| E-reserve | [reserve_remote_operation_with_priority](../../src/repository/remote/reservation.rs#L285) root/op/RemoteOperationTarget/priority → RemoteReservationOutcome; [active_remote_operation](../../src/repository/remote/reservation.rs#L339) → Option<RemoteOperationInspection>; [cancel_remote_operation](../../src/repository/remote/reservation.rs#L351); [remote_safe_point](../../src/repository/remote/reservation.rs#L368) → RemoteSafePointOutcome; [restart_remote_observation](../../src/repository/remote/reservation.rs#L392); [finish_remote_operation](../../src/repository/remote/reservation.rs#L457) | [stable_id_replays_without_a_second_owner_and_mismatch_is_fixed](../../tests/remote_reservation.rs#L106), [cancellation_waits_for_each_named_safe_point_without_git_lease](../../tests/remote_reservation.rs#L130), [restart_fences_old_poll_owner_and_refuses_manual_or_mutation_work](../../tests/remote_reservation.rs#L265); no full transfer-stage cancel/total deadline |
| E-start | [initialize_git_transport_before_threads](../../src/runtime.rs#L25) → Result<(), TransportInitializationError>; [git_transport_initialized](../../src/runtime.rs#L38) → bool | [local_git_does_not_silently_initialize_network_settings](../../src/runtime.rs#L45); source calls in both binaries, no stalled-shutdown or frontend failure-envelope proof |

## CLI inventory — exactly one row per verb or invocation

C0–C4 apply to every CLI row; references in cells specialize them. All mutation
rows require R-adapter even where only an admin/author/domain re-audit is listed.
Read rows have no mutation request identity/confirmation requirement, but reads
may have the application-local bookkeeping effects explicitly recorded in C0.

| Requirement / planned consumer | Actual public symbol, request/outcome/test evidence or missing API | Target and observations | Identity, replay/cache loss and effects | Consent, progress and cancellation | Credentials and redaction | Evidence status | Owner / acceptance and re-audit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `repo list` — W3-01/07 | Missing repository enumeration; E-index only reads one known root; no inventory DTO/tests | All registered roots, inaccessible/recovery state; no network | Read only; registry loss cannot recover unknown registrations by inference (C0) | No consent; safe complete list needed, no refresh | No unlock; paths/problems redacted C0 | absent | W3-01 read addition; R-read |
| `repo inspect` — W3-01/07 | `inspect` E-inspect → RepositoryInspection; no frontend observation DTO | Exact root currently; common identity/opaque observations missing C0 | No enable/scan; service-open bookkeeping C0; stale cache distinct | No consent; synchronous, no progress/cancel | No unlock; raw URLs/errors need C0 | partially-present | W1-02 primitive; W3-01; R-read |
| `repo create` — W3-03/07 | `create_and_enable` E-enable; CreateRepositoryRequest → EnableRepositoryOutcome | Empty/unused target + primary + optional identity; absent-parent observation missing | Op ID; init Git/config/local identity/checkpoint/register/index; partial outcomes C1 | No public prepare/confirm/expiry; C2; preview must not init | Local Git identity, not SSH; no global write; C0 | partially-present | W1-02/05; W3-02/03; R-admin |
| `repo enable` — W3-03/07 | `enable` E-enable; EnableRepositoryRequest → EnableRepositoryOutcome | Exact root, checked-out primary/clean state; observations not frontend token | Op ID; config/exclude/init checkpoint/registry; AlreadyEnabled/IdentityRequired/pending | C2 absent consent/progress/cancel | Optional explicit local name/email; no invented identity; C3/C0 | partially-present | W1-02/05; W3-02/03; R-admin |
| `repo remove` — W3-03/07 | `remove_registration` E-remove | Existing canonical root required; no expected-registration token | Op ID; only registry/derived/recovery rows removed; preserves files/drafts, replay becomes NotRegistered | Required confirmation missing C2; no worker invalidation yet C6 | No SSH; C0 error map | partially-present | W1-02/05; W3-02/03; R-admin |
| `repo identity` — W3-01/07 | `inspect` E-inspect returns availability only; missing name/email/source public read/tests | Effective local/same-config-level identity, common repo | Read; no persistence allowed, C0 | No consent; synchronous primitive | Not key identity; return only requested non-secret identity | partially-present | W3-01 read addition over W1-02; R-read |
| `repo identity-set` — W3-03/07 | Missing public setter; private identity resolver/config provider and hidden testing methods not APIs | Existing local config; exact identity/config observation needed | Request ID/digest absent; explicit repo-local write only, never global | Required exact confirmation missing; C2 | Non-secret name/email; C0 errors | absent | W3-03 public identity boundary, W3-02 safety; R-admin |
| `remote list` — W3-01/07 | `list_remotes` E-remotes → Vec<RemoteInfo>; publication selection via config | Exact root; sorted names; publication/push eligibility | Read local Git only; no fetch; C0 | No consent; no refresh/progress needed | Raw URL secrets **not redacted**; W3-01 sanitizes | partially-present | W1-02; W3-01; R-read |
| `remote add` — W3-03/07 | `add_remote` E-remotes; AddRemoteRequest → RemoteOutcome | Named Git config URL; conflict if name has other URL | Op ID binds name/URL digest; local config/cache changed, no implicit select; C1 | C2 request/preflight/progress missing | No SSH contact; input URL/error redaction C0 | partially-present | W1-02/05; W3-02/03; R-admin |
| `remote remove` — W3-03/07 | `remove_remote` E-remotes; RemoveRemoteRequest → RemoteOutcome | Named remote; selected removal blocked until clear; no observation token | Op ID; Git config/cache mutation with pending discovery; C6 invalidation | Required polling/publication preview/confirmation missing C2 | No SSH; never echo endpoint credentials C0 | partially-present | W1-02/05, W2-04 state; W3-02/03; R-admin |
| `remote select` — W3-03/07 | `set_publication_remote` E-remotes; optional name → PublicationRemoteOutcome | Valid config/primary/clean config path; SSH fetch/effective push URLs | Op ID; canonical config checkpoint plus registration/index; no network | C2 missing token/preview; configuration invalidates queued state C6 | Git identity for commit; not SSH unlock; sanitize URL C0 | partially-present | W1-02/05, W2-04 state; W3-02/03; R-admin |
| `key list` — W3-01/07 | `list_shared_keys` E-keyreg → registrations; no safe frontend DTO | Application-global keys/selection/ownership/fingerprint/source states | No request; writable open/migration/cache lock despite list C0 | No consent/progress; no private display | C3; fingerprint not public text; safe paths only | partially-present | W2-01; W3-01; R-read |
| `key show` — W3-01/07 | E-keyreg list + opaque SharedKeyId; missing public single-key safe-read DTO/tests | Exact key ID, not label; inspect_selected_key is selection-only | Read registration only; no cache-loss invented ownership | No consent; no private parse for registration read | C3; no private bytes | partially-present | W3-01 narrow read over W2-01; R-read |
| `key generate` — W3-03/07 | `generate_shared_key` E-keygen + KeyStore; Created/AlreadyCreated/RecoveryRequired | App-owned generated paths, explicit label/protection | Op ID/owned creation evidence; interruption retains files; non-secret protection replay gap C1 | No RFC confirmation ID/expiry; no progress/cancel mid-write; C2 | Passphrase is SecretPassphrase input, terminal provider missing; no passphrase digest C3 | partially-present | W2-02; W3-02/03; R-admin |
| `key import` — W3-03/07 | `register_shared_key` E-keyreg, Imported ownership by adapter | Invocation-relative paths need normalization to absolute source refs; no private copy | No op/request ID; dedup normalized private source; lost registry not durable retry proof | C2 adapter validation/replay missing | No unlock; optional bounded public fingerprint read; C3 | partially-present | W2-01; W3-02/03; R-admin |
| `key select` — W3-03/07 | `select_shared_key` E-keyreg → SharedKeySelectionOutcome | Opaque key ID; pending material blocks; single shared selection | No request ID; transaction changes shared state/remote observations; C6 | No token/progress; queued/active invalidation adapter missing | Session rechecks at use, no commit identity change C3 | partially-present | W2-01/04; W3-02/03; R-admin |
| `key clear` — W3-03/07 | `clear_shared_key_selection` E-keyreg | Shared selection, no repository target | No request; Cleared/AlreadyCleared, invalidates state; no cross-process secret IPC | C2; frontend safe invalidation C6 needed | Explicit session clear after safe worker use C3/C6 | partially-present | W2-01/02/04; W3-02/03; R-admin |
| `key remove` — W3-03/07 | `unregister_shared_key` E-keyreg | Exact key; selected/pending material blocks | No request; registration-only deletion; generated/imported files remain | C2; distinct from generated deletion consent | No private deletion/read; C3 | partially-present | W2-01; W3-02/03; R-admin |
| `key delete` — W3-03/07 | `review_generated_key_deletion`/`delete_generated_key` E-keydelete | Exact owned pair + file identity/creation evidence; imported or selected rejected | Op ID, fresh review on interruption; Deletes files/ownership/registration; RecoveryRequired partial | One-use review + confirmed bool, cancellation before access; **not** ten-minute token C2 | No passphrase needed; protected-store effects; C3 | partially-present | W2-02; W3-02/03; R-admin |
| `key public` — W3-01/07 | Missing public text-by-ID read; E-keyreg only fingerprint; KeyStore raw operations private | Registered key/public source observation; unavailable companion explicit | Read public material only; cache loss cannot infer owned provenance | No consent/progress; bounded safe public reader required | Never derive by exposing private contents; C3 | absent | W3-01 narrow read over W2-01/02; R-read |
| `host list` — W3-01/07 | Missing public pins enumeration; trust.rs readers internal/test-only E-ssh | Application-global non-secret pins; no remote contact | Read only; permanent cache-loss reapproval fence C3 | No consent; no re-observation network for list | Exact authority/fingerprint, no secrets C3 | absent | W3-01 read over W2-03; R-read |
| `host inspect` — W3-01/07 | Missing public authority snapshot; verify_ssh_transport E-ssh is **network**, not safe read | Authority + explicit repository for relevant observation | Read pin/history/state only; cache loss explicit C3 | No consent for local read; frontend observation missing | C3; do not call restricted read_host_pin as public API | absent | W3-01 read over W2-03; R-read |
| `host approve` — W3-03/07 | `verify_ssh_transport` E-ssh with HostApproval expected None | Exact configured SSH authority/presented fingerprint, transport rechecks | No op/request ID; authenticated pin write only; no fetch/push | Exact approval supported, RFC prepare/expiry/intent binding missing C2 | Selected-key session + masked provider; C3 | partially-present | W2-03; W3-02/03; R-admin |
| `host replace` — W3-03/07 | `verify_ssh_transport` E-ssh with HostApproval expected old pin | Exact authority/old/new fingerprint; re-observe before pin persistence | No op/request ID; pin replacement; changed trust rejects C3 | No generic yes; missing confirmation service C2 | Selected-key session, no raw backend output C3/C0 | partially-present | W2-03; W3-02/03; R-admin |
| `document list` — W3-01/07 | `repository_snapshot` E-index; no complete nullable document DTO | Root/context precedence/path then ID, malformed/inaccessible entries | Cached metadata read only; stale scan distinct; C0 effects | No consent/implicit refresh; completeness limits visible C0 | No unlock; list omits body/source | partially-present | W1-04; W3-01; R-read |
| `document show` — W3-01/07 | Missing repository-targeted full read; parse_item E-canon is supplied-source only | Full ULID or exact malformed path + context/source observation | Read exact source/unknown metadata, no context provisioning | No consent, no implicit repair or worktree | Requested source allowed; errors redacted C0 | absent | W3-01 narrow read over W1-01/04; R-read |
| `folder create` — W3-04/09 | Missing public folder-only operation; save_document's parent creation is not equivalent | Repository-relative docs path; context and absence observation | Explicit local empty directory, no placeholder/checkpoint/publication; replay absent | C2 request/absence; no implicit browsing creation | No SSH/secret; scoped symlink-safe path/errors | absent | W3-04 scoped filesystem addition, W3-02 safety; R-author |
| `document create` — W3-04/09 | `save_document` E-document Create target + caller ItemId | Exact new docs destination; Missing required; owned shared context | Op ID; provision/write/scoped checkpoint/discovery, no publish; C1 | C2 adapter digest/progress/cancel missing | Local commit identity only; bodies excluded from logs | partially-present | W1-03/05; W3-02/04; R-author |
| `document save` — W3-04/09 | `save_document` E-document Edit target | Destination observation mandatory; source observation optional, checked only for Some; missing-source-observation rejection absent C1; full draft not patch input | Op ID; values preserved, serializer header/no-op hazard C1; no publish | W3-02/04 must require source observation and prove None rejection before effects (C1); C2 missing patch/opaque token adapter | Local identity; body only explicit input, no diagnostics C0 | partially-present | W1-03/05; W3-01/02/04; R-author + R-adapter |
| `document move` — W3-04/09 | `save_document` E-document source/destination pair | Destination observation mandatory; optional source observation checked only for Some before read/removal, None may remove unobserved current source; same ID/owned context C1 | Op ID binds pair; write/remove/checkpoint both paths, partial recovery C1; source-omission rejection not guaranteed | W3-02/04 require source observation and None-rejection/state-preservation tests (C1); C2 request/preview/progress missing; no force | Local identity, no remote; C0 path/error redaction | partially-present | W1-03/05; W3-02/04; R-author + R-adapter |
| `document repair` — W3-04/09 | Missing public complete-source repair/adoption; E-canon validates only | Exact malformed path/base observation; uniqueness/context/ID invariant | Request/adoption ID replay absent; normal owned checkpoint, not primary patch | Explicit valid preview + confirmation missing C2 | Local identity; malformed source not logged C0 | absent | W3-04 repair/adoption + W3-02 safety; R-author |
| `ticket repair` — W3-04/09 | Missing public complete-source ticket repair; E-canon not authoring service | Exact path/base, uniqueness/closure/no-reopening/owned context | Replay absent; preserve existing ID/closure, adoption explicit | Required preview/confirmation C2 missing | Local identity; source redaction C0 | absent | W3-04 boundary, W2-10 closure invariants; R-author + R-domain |
| `ticket list` — W3-01/04/07 | `repository_snapshot` E-index metadata includes closed_at | Root/context, activity descending then ID; status/type/project/closure filters missing | Read cached metadata; lifecycle closed != status text; include malformed C0 | No consent; stale/empty/failed scan distinction | No unlock; no list bodies; C0 | partially-present | W1-04; W3-01 DTO, W3-04 filters; R-read + R-author |
| `ticket show` — W3-01/07 | Missing full repository item read; E-canon/E-index incomplete | Full ID including closed ticket; body/unknown/closed_by/context/token | Read only; do not provision/reopen/repair | No consent; safe read missing | Requested source allowed; C0 | absent | W3-01 over W1-01/04, W2-10 invariants; R-read |
| `ticket create` — W3-04/09 | `save_ticket` E-ticket Create; caller ID, full TicketDraft | Missing canonical ticket path/unique ID; status text closed allowed | Op ID; provision/write/checkpoint/discovery; closure fields omitted; no publish | C2 absence/digest/progress missing | Local identity only; no raw body logs | partially-present | W1-03/05; W3-02/04; R-author |
| `ticket save` — W3-04/09 | `save_ticket` E-ticket Edit | Exact byte expected_path; project/team optional full draft, patch/null adapter missing | Op ID; preserves unknown/closure and semantic no-op; lifecycle guard re-audit C1 | C2; no closure via save, no force | Local identity only; C0 | partially-present | W1-03/05; W3-01/02/04, W2-10 guard; R-author + R-domain |
| `comment list` — W3-01/10 | `repository_snapshot` E-index IDs/order only; ordered_comment_threads E-canon pure | Item ID in canonical context; roots/replies and visible malformed thread DTO needed | Read; no body/author/token public service; no checkpoint | No consent/context provisioning; complete safe reads missing | Requested bodies allowed, C0/C1 author provenance gap | partially-present | W1-01/04; W3-01; R-read |
| `comment add` — W3-05/10 | Compound `submit_comment`, original receipt and body-free `retry_comment_publication` E-comment; Cycle 07 acceptance in progress | Exact item/context/comment ID/parent + Missing destination; one bound sync child | Scoped checkpoint/discovery, then context sync; separate Saved publication/index state; original action reused on retry | Immediate configured context sync; explicit child restart/cancel; source separate from unsaved item drafts | Local commit identity; caller-owned selected-key session/host approval; confirmation only for child merge | implementation-in-progress | W2-07 over W1-03 and W2-05/06; W3-02/05; R-domain |
| `item sync` — W3-05/10 | Missing public sync; E-observe/E-reserve and private transfer helpers not service | Exact item/shared context/remote/ref observations; never unsaved caller input | No sync outcomes/tests; future fetch/FF/merge/push/discovery effects C4 | C2 safe cancel + deleted-branch republish consent required | Future selected-key/trust C3 | pending-Wave-02 | W2-05 clean + W2-06 merge; W3-02/05; R-domain |
| `repo sync` — W3-05/10 | Missing public primary sync; E-observe is advertisement | Exact primary/remote, clean-primary observations | Future fetch/FF/merge/push/discovery, not polling; replay unproved | C2 progress/cancel, no inferred publication intent | Future selected-key/trust C3 | pending-Wave-02 | W2-05/06; W3-02/05; R-domain |
| `document promote` — W3-06/11 | Missing public preflight/confirmed promotion; future enum action not API | Exact item + whole branch/primary/remote/OIDs/paths/cleanup | Future final checkpoint/sync/merge/primary publish/remote-first cleanup; partial/replay absent | W2-09 preflight then W3-02 expiry/accepted consent/progress C2 | Local identity + selected-key if remote; local-only pending explicit | pending-Wave-02 | W2-09; W3-02/06; R-domain |
| `ticket close` — W3-06/11 | Missing public close/preflight; ticket serializer fields are not lifecycle | Exact ticket + whole branch, confirmed identity + closed_at/by | Future final closure checkpoint/integration/publication/cleanup; cleanup-only replay absent | W2-10 confirmed preflight + C2; no reopening/status shortcut | Local confirmed identity + optional selected-key/trust | pending-Wave-02 | W2-10; W3-02/06; R-domain |
| `index status` — W3-01/07 | `repository_snapshot` E-index refresh_required/problems, no full frontend freshness DTO | Explicit root; cache age/stale/failed scan distinguish | Read-only SQLite + cache-lock/open effects C0; no fetch | No consent/implicit refresh | No unlock; metadata/problems redaction C0 | partially-present | W1-04; W3-01; R-read |
| `index refresh` — W3-04/07 | `refresh_repository` E-index → Refreshed/RetryRequired/IndexPending | Registered root, re-observed contexts; races retain prior rows | Op ID; scan/cache/journal/heartbeat effects only; no Git/canonical rewrite/network | No public progress/cancel; C2; one-shot thread joins | No SSH; source not persisted; C0 | partially-present | W1-04/05; W3-02/04; R-author |
| `index rebuild` — W3-04/07 | `rebuild_repository` E-index → RepositorySnapshot | Explicit root even after cache loss; observe/recheck, stale flag | Op ID; cache replacement/diagnostics/re-registration, trust/history fences; no draft/Git deletion | C2 progress/cancel missing; no implicit poll | No SSH; preserve trust reapproval fence C3/C4 | partially-present | W1-04/05, W2-03/04 fences; W3-02/04; R-author |
| `poll status` — W3-01/12 | `remote_snapshot` E-policy + active_remote_operation E-reserve partial | Durable policy/latest outcome/ref states/active reservation; full attempt/next-time DTO missing | No network; snapshot read + active transaction C0, no process discovery | No consent; distinguish worker state only desktop C6 | No unlock; redacted categories, no stale unlock claim | partially-present | W2-04/08 domain status; W3-01; R-read + R-domain |
| `poll configure` — W3-06/12 | `set_remote_polling` E-policy; interval 60–3600 | Exact registered root/policy; no observation token | No request ID; durable enabled/paused/interval transaction; future due logic C6 | C2 request/progress mapping absent, no scheduling now | No unlock; C0/C3 | partially-present | W2-04/08 policy; W3-02/06; R-adapter + R-domain |
| `poll pause` — W3-06/12 | `set_remote_polling` E-policy updates tuple, no isolated pause request | Policy observation, preserve interval/enabled/backoff/unlock distinction | No request ID; caller must avoid overwriting newer tuple C6 | C2; active operation safe stop adapter missing | No unlock/prompt; pause not authentication block | partially-present | W2-04/08; W3-02/06; R-adapter + R-domain |
| `poll resume` — W3-06/12 | `set_remote_polling` E-policy; recovery resume is **different** | Clear explicit pause, due eligibility; preserve unlock/recovery/history states | No request ID; full due semantics absent; not resume_remote_polling_after_recovery | C2; deliberate recovery authorization separate | No implicit unlock/host approval C3/C6 | partially-present | W2-04/08; W3-02/06; R-adapter + R-domain |
| `poll once` — W3-06/12 | Missing one-shot poll/materialization; observe_publication_remote E-observe is only precursor | Exact remote/primary/context clean observations, explicit invocation | Future fetch/clean FF/materialization/index once; no push/checkpoint/merge/cleanup C4 | E-reserve safe points partial; C2 full progress/cancel; no resident worker | Selected-key session/host re-observation C3 | pending-Wave-02 | W2-08; W3-02/06; R-domain |
| `operation list` — W3-01/07 | E-recovery pending locals + E-keygen recovery + E-reserve active only; missing unified public inventory | Explicit repo, start time/ID ordering; completed/pending scope | Recovery listing E-keygen takes exclusive cache guard/writable open_registry, possible SQLite creation/WAL setup C0; no automatic resume; incomplete records/cache loss explicit | No consent; query intent does not imply side-effect-free inspection C0 | No source/passphrase persistence/echo C0/C1 | partially-present | W3-01 safe read addition over W1-05/W2-02/04/future domains; R-read |
| `operation show` — W3-01/07 | Missing public op-ID lookup/re-observation/effects DTO; E-recovery not full show | Exact op ID, actual Git/filesystem wins stale record | Need typed completed/remaining effects, input-needed after cache loss; no publication | No confirmation/resume inferred; C2 mapping | No raw original body reconstruction/logging C0/C1 | partially-present | W3-01 read/reconcile + W3-02 effects, W2 owners; R-read + R-adapter |
| `operation resume` — W3-05/06/11 | Missing generic deliberate resume; exact local calls E-recovery / restart_remote_observation E-reserve only | Exact existing action/target; original input may be needed | C1 domain replay is not permission for other action; cache loss blocks if unprovable | C2 accepted consent eligible remainder only; progress/cancel needed | Future selected-key/trust if network; no secret/body record | partially-present | W3-02 adapter over W2-05…10; consumers W3-05/06; R-adapter + R-domain |
| `conflict show` — W3-01/11 | Missing public conflict operation/path/base/local/remote read; no domain conflict record yet | Op ID/path observations, owned/noncanonical conflict kind | Read preserved sources; actual index state not marker-text heuristic | No consent/implicit staging/resolution | Requested comparison source allowed; errors redacted C0 | pending-Wave-02 | W2-06 domain, W3-01 DTO; R-read + R-domain |
| `conflict resolve` — W3-05/11 | Missing public owned-canonical resolution/checkpoint/resume | All conflicted owned paths + observations + explicit sources; validate all before writes | Future checkpoint/eligible remainder; stale input preserved; no arbitrary code staging | C2 explicit per-path action, no blanket ours/theirs; safe cancel | Local identity + future selected-key; source not diagnostic C0/C3 | pending-Wave-02 | W2-06; W3-02/05; R-domain |
| `id new` — W3-01 | `ItemId::generate` E-id; reusable canonical ULID primitive | No repository/context | Fresh ID returned to caller; no mutation/cache/replay guarantee | No consent/progress/cancel required | No credentials/secret; output canonical string | partially-present | W1-01; W3-01 envelope; R-read |
| `no arguments` — W3-01 | CLI main L1/E-start only; missing concise help/tests | Service-free invocation | Current silent zero after initializer, not help; no registry should open | No consent/progress; startup envelope C0 | No provider/service/key use | partially-present | W3-01; R-read |
| `--help` — W3-01 | Missing parser/help implementation/tests in CLI main L1 | No services; list implemented vs unavailable commands | No request/effects; currently ignored args still initialize transport | No consent/progress; exit zero required | No credentials/registry/network | absent | W3-01; R-read |
| `--version` — W3-01 | Missing version handling/tests in CLI main L1 | Service-free version | No request/effects; currently ignored args, no version output | No consent/progress; exit zero required | No credentials/registry/network | absent | W3-01; R-read |

## Desktop supplementary inventory

These are actual desktop RFC capabilities beyond invoking the CLI-domain rows.
C0–C5 apply to every row. `Missing; main.rs L1` means the only production desktop
is the inspected HelloWorld scaffold, with no request/outcome types or tests for
that capability. Catalog evidence is adjacent domain evidence, never UI proof.

| Requirement / planned consumer | Actual public symbol, request/outcome/test evidence or missing API | Target and observations | Identity, replay/cache loss and effects | Consent, progress and cancellation | Credentials and redaction | Evidence status | Owner / acceptance and re-audit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| D01 repository/document-tree/ticket navigation — W3-07 | Missing; main.rs L1; E-index supplies partial metadata | Repository identity/tree, lifecycle-open default + Closed/All, malformed visible | Browse only; no provision/save/sync; C0/C5 | Async scan/empty/stale/failed states missing | No unlock to browse; C0 safe DTOs | absent | W3-01 reads then W3-07; R-read + R-shell |
| D02 stable multi-repository tabs and focus existing item — W3-07 | Missing tab identity/model/tests; main.rs L1 | Common repository/kind/full ID, not title/path | Opening same item focuses; tab/repo switch never saves/publishes | Focus/progress preserved; no mutation confirmation | Owned non-secret DTOs; C5 | absent | W3-01 resolver, W3-07 tabs; R-shell |
| D03 context provenance and duplicate/mismatch recovery view — W3-07 | Missing presentation; E-index contexts/problems | Primary/active branch/worktree, ambiguous contexts block edit | Read only, no context chooser/repair on browse | Persistent actionable recovery, no automatic retry | Path/problem redaction C0 | partially-present | W1-04 reads, W3-01 DTO/07 presentation; R-read + R-shell |
| D04 edit/comment context provisioning before authoring — W3-09/10 | `prepare_context` E-context; no async authoring-enabled model | Exact item intent/context, viewing primary never provisions | Op ID, branch/worktree/cache effects C1 | Report progress before enable editor; C2 adapter missing | Local identity separately; no implicit SSH | partially-present | W1-03/05, W3-02 adapter then 09/10; R-adapter + R-author |
| D05 settings/onboarding/key/host dialogs and focus return — W3-07 | Missing; CLI admin inventory/E-enable/E-keyreg/E-ssh partial | Explicit root/primary/identity/key/authority/effect preview | Same shared services, no UI SQL/Git; removal preserves drafts | Exact consent/provider cancel/focus return C2/C3 | Masked provider; no private copy UI | absent | W3-01/02/03 boundaries then 07; R-admin + R-shell |
| D06 persistent operation/recovery area — W3-07/10/11 | Missing; E-recovery/E-reserve partial records | Request/operation scope, completed and remaining steps | Restart exposes pending manual work, no auto publish/resume | Progress/safe cancel, failure not transient toast only C2 | Redacted owned results, no shell commands | partially-present | W3-01/02 then 07; R-read + R-adapter + R-shell |
| D07 rich/source shared body draft and selection mapping — W3-09 | Missing editor entity/mode adapter/tests; E-canon pure body only | Exact original Markdown body + base/generation | Toggle not save/reload; same draft/selection/history C5 | Explicit mode/focus controls, dirty input survives | No credential/resource access by editor | absent | W3-08 drafts then 09; R-draft + R-editor |
| D08 minimum rich vocabulary and source-span focused edits — W3-09 | Missing editor integration/tests; main.rs L1 | Paragraphs/headings/emphasis/strong/links/ordered/unordered/task lists/quotes/fenced code/tables incl cell/alignment/row/column edits | Changed-block formatting only, untouched bytes/unsupported source preserved | Keyboard format/table actions; no implicit save | Deny executable rendering C5 | absent | W3-09 pinned selection; R-editor |
| D09 shared undo/redo across mode switches — W3-09 | Missing editor history/readback evidence; no service equivalent | Same draft, UTF-8-safe selection/offset mapping | Toggle must not reset history; undo-to-original/redo exact | Keyboard activation/focus required | No persistence/credential side effect | absent | W3-09; R-editor |
| D10 named metadata/lifecycle fields separation — W3-09/11 | E-canon Document/Ticket, E-ticket draft; no UI model/tests | IDs and closed_at/by immutable in ordinary edit, status free-form | Values/body preserved, closure only confirmed close; C1 no-op risk | Validate before save; lifecycle consent via W2-10/C2 | Commit identity not SSH selection | partially-present | W3-01/02/04 then 09, W2-10 invariants; R-author + R-domain |
| D11 full canonical inspect/repair source view — W3-09 | Missing safe full read/repair API, E-canon validates supplied strings | Exact path/source observation; visible conformity diagnostics | Invalid input remains draft; explicit adoption, no primary shortcut | Preview/confirm repair, duplicate identity blocks C2 | Requested source allowed, diagnostics redacted | absent | W3-01 read/02 consent/04 repair then 09; R-read + R-author |
| D12 parse failure/unsupported-source fallback — W3-09 | Missing renderer/fallback tests; E-canon not rich parser | Retain original malformed/unsupported source, label region/read-only rich if unsafe | No simplify/drop/normalize whole document; source remains editable | Precise non-destructive diagnostics, source mode | No HTML execution/secret logging | absent | W3-09; R-editor |
| D13 keyboard rich/plain paste and formatting — W3-09/10 | Missing editor clipboard/native input tests | Rich paste supported structures only; plain text always | Explicit draft edit, not canonical write; undo shared | Focusable toolbar/paste actions C5 | Synthetic fixtures in evidence, no clipboard secret logging | absent | W3-09 editor then 10 composer; R-editor |
| D14 image/resource containment — W3-09 | Missing host providers/negative tests | Permitted local docs resources, traversal/symlink/private-key/scheme restrictions | No remote images/file save/HTML execution; audit internal access | Unsupported resource visible placeholder, not auto opener | No renderer key/credential paths; C5 | absent | W3-09; R-editor |
| D15 relative-link resolution and deliberate external open — W3-09 | Missing link policy/opener service/tests | Repo-permitted doc tree; external request explicit | No arbitrary scheme/automatic OS/network open | Deliberate action, visible blocked link reason | No private-key/symlink escape resource; C5 | absent | W3-09; R-editor |
| D16 draft/base/source identity and validation model — W3-08 | Missing draft ID/revision/base DTO/store; C1 only path hash | Item/comment local draft ID + base path/context/source/current metadata/body | Invalid edits retained separately, never canonical/SQLite authority | No automatic Git checkpoint; visible unsaved state | Owner-protected local source files, never credentials | absent | W3-01 observations/02 effects then 08; R-draft |
| D17 atomic protected recovery persistence and idle/close flush — W3-08 | Missing draft file schema/store/flush outcomes/tests | Versioned app-local per-draft source/base, max 1s idle | Atomic last-successful flush, migration preserves original, no age eviction | Report failure; no unflushed crash-safe claim | C5 source permitted in draft only; no secrets | absent | W3-08; R-draft |
| D18 restart inspect/restore/export/discard — W3-08 | Missing draft recovery API/UI/tests | Exact stored revision, incompatible/closed/vanished context visible | Restore not canonical overwrite; explicit scoped discard, preserve migration original | User choice, no automatic reopen/cleanup | Protected exports/source, no credentials | absent | W3-08 + W2-10 guard; R-draft + R-domain |
| D19 save completion retires only checkpointed draft revision — W3-08/09 | E-document/E-ticket partial save effects; missing revision retirement | Draft generation submitted vs newer typing, base observation | Retire only saved revision, newer dirty text survives; distinguish checkpoint/index pending | C2 progress/retry, no global save-failure claim | No publication or credential echo | partially-present | W3-02 effects then 08/09; R-adapter + R-draft |
| D20 unsaved tab/app close choices — W3-08/09 | Missing close protocol/tests | Selected draft/revision, Save/Keep draft/Discard/Cancel | Flush recovery, explicit discard only; cancel leaves work | Graceful wait/flush errors visible; C6 shutdown separately | Credentials retained until worker ends | absent | W3-08 UI integration 09, shutdown 12; R-draft + R-worker |
| D21 dirty external-change three-way review and clean reload — W3-08/09 | C1 expected paths exists; missing base/new/current comparison DTO/model | Base context/source + new canonical + dirty draft, generation | Dirty never auto replaced by poll/CLI/refresh; clean deliberate reload preserves selection | Stale save preserves input, explicit review action | Requested comparison sources only, no logs | partially-present | W3-01 reads/02 observations then 08/09; R-read + R-draft |
| D22 drafts survive removed/inaccessible repository or vanished context — W3-08 | E-remove/index do not implement draft store/tests | Persisted draft accessible independent of registration/context/cache | Preserve recovery/export on remove/rebuild; no auto worktree recreation | Explicit recover/discard, no hidden age cleanup | Owner-protected store separate C5 | absent | W3-08 over W3-01/02/03; R-draft |
| D23 rich/source recoverable comment/reply composer — W3-10 | E-comment local checkpoint only; missing composer/draft/editor | Item/parent/comment IDs, body/base observation | One submission ID, do not include unsaved item draft; same fidelity as D07–D19 | Post and sync vs Save locally labels; partial publication retry | Local identity + future selected-key/trust C3/C4 | absent | W3-08/09 editor, W2-07 domain, W3-10; R-draft + R-domain |
| D24 saved comment shown once; retry publication only — W3-10 | E-comment exact local replay; missing compound outcome/UI/tests | Retained comment/request/operation ID and original shared context | No new file/checkpoint/timestamp on publication retry C4 | Show completed/local saved and remaining publication; deliberate resume | Same session/trust boundary; redacted partial effects | pending-Wave-02 | W2-07 + W3-02 adapter then 10; R-domain |
| D25 explicit sync/completion dialogs distinguish branch-wide effects — W3-10/11 | Missing; CLI sync/promote/close rows apply, no desktop preflight/UI | Whole branch incl code, primary/context/remote, final saved draft | Unsaved text separate; integration != cleanup; local-only publication pending | Confirm changed target anew, reuse unchanged accepted remainder C2 | Confirmed Git identity + selected-key/trust | pending-Wave-02 | W2-05/06/09/10 + W3-02 then 10/11; R-domain |
| D26 canonical conflict base/local/remote/result editing — W3-11 | Missing W2-06 public resolution/read and UI/tests | Owned canonical paths + all expected observations/valid resolutions | Validate all before write/checkpoint; markers not Git-resolution authority | Per-path explicit Mark resolved, no automatic side choice | Requested source vs redacted progress C0 | pending-Wave-02 | W2-06 + W3-01/02/05 then 11; R-domain |
| D27 noncanonical external recovery/recheck guidance — W3-11 | Missing public conflict/re-observe DTO/UI; private Git not frontend API | Code/binary/rename/delete/structural conflicts preserved | Open location deliberate; recheck actual Git, never arbitrary staging/publish | Explicit operation resume after real repair; no marker-only inference | Owned redacted recovery arguments C0 | pending-Wave-02 | W2-06 observations + W3-01/02/05 then 11; R-domain |
| D28 keyboard focus/dialog Escape/action activation — W3-07…12 | Missing full control/focus contract/tests; Button scaffold only | Every core action labeled/focusable, tab order/visible focus/focus return | UI state only, color not sole state signal | Escape cancellation at dialog/request safe boundary | No secret text in UI diagnostics | absent | W3-07 shell, each feature, W3-13 gate; R-shell |
| D29 native IME/Unicode/accessibility/high-DPI evidence — W3-09/13 | Missing native editor/control evidence; scaffold not proof | Supported OS/architecture native inputs, composition/selection | Preserve exact draft/source including bidi/combining/non-BMP | Keyboard minimum != screen-reader conformance | Synthetic redacted fixtures/evidence | absent | W3-09 editor, W3-13 native matrix; R-editor + R-shell |
| D30 responsive owned async models and stale-completion guards — W3-07…12 | Missing worker/model/generation types/tests; RepositoryService is synchronous | Request ID/generation/repo/item identity before applying completion | Old result cannot replace newer draft/selected item; no git2 handle in UI | Bounded worker work off UI; typed progress/cancel C2 | No secret Debug/logs, session stays worker-owned C3 | absent | W3-01/02 headless boundary then 07/08/09; R-adapter + R-shell |
| D31 integrated responsiveness and native journeys — W3-13 | Missing final app/journey fixtures/results; no API substitutes | 1,000 items/100 KiB recorded hardware and supported native matrix | Actual six journeys/two clones, exact bytes/refs/effects, no simulation claim | p95 rendered input + cancel feedback, stalled-work usability | Isolated synthetic keys/repos for tests, redacted artifacts | absent | W3-13 after all earlier owners; R-editor + R-worker + R-domain |

## Runtime supplementary inventory

C0–C4 and C6 apply throughout; no scheduler/service/API was run. A status record
is not evidence a desktop worker is alive, and an action enum is not a method.

| Requirement / planned consumer | Actual public symbol, request/outcome/test evidence or missing API | Target and observations | Identity, replay/cache loss and effects | Consent, progress and cancellation | Credentials and redaction | Evidence status | Owner / acceptance and re-audit |
| --- | --- | --- | --- | --- | --- | --- | --- |
| R01 pre-thread transport startup — CLI/desktop | `initialize_git_transport_before_threads` E-start; both mains call it | Process globals, unsafe before any thread/native concurrent Git | OnceLock result, global timeout writes; not total deadline | No consent; handled CLI failure envelope/help boundary absent | Fixed startup error, no provider/key | present-but-unverified | W2-03 primitive, W3-01 CLI/07 desktop ordering; R-read + R-shell |
| R02 shared request/outcome/effects/recovery adapter — all W3 | Missing headless adapter; E-document/E-recovery/E-reserve partial | Explicit repo/item/context/request/op/stage and observations | Existing journal/lease authority, no alternate framework; C1 complete effects/replay missing | C2 exact consent/typed recovery/progress/cancel | C0 redacted DTOs, no raw error/source in mutation results | partially-present | W3-01 DTO + W3-02 adapter; R-adapter |
| R03 bounded workers and worker-local Git handles — W3-02/07/12 | Missing execution-capacity/progress API/tests; refresh heartbeat E-index not worker pool | Owned requests/DTOs across threads, handles created/used/dropped on worker | No direct UI filesystem/SQL/Git; actual leases retain authority | Async progress/cancel, prompt/network outside short lease | Worker-local session and redacted Debug C3 | absent | W3-02 headless execution then 07/12; R-adapter + R-worker |
| R04 manual priority/yield/reservation safe points — W3-02/12 | E-reserve public tokens/outcomes; target enum future actions only | Exact root/op/action/priority/service ownership; observed durable phase | Replay no new owner, explicit restart fences read-only abandoned observation | Durable cancel/yield ack at named boundaries, not total transport cancel C2 | No secrets in tokens/progress, C3 | partially-present | W2-04 plus W2-05…10 phases, W3-02; R-adapter + R-domain |
| R05 process-local selected-key credential session — CLI/desktop | SessionCredentials + inspect/unlock E-session | Key/source token, provider outside lease; selection/endpoint recheck | Cache success in process only, invalidate at use/clear/drop; no IPC/persistence | Cancellation/provider-unavailable distinct; no automatic prompt-loop proof | Zeroizing passphrase + redacted Debug, C3 | present-but-unverified | W2-02/03; W3-02/03/12 wiring; R-admin + R-worker |
| R06 terminal-only one-shot CLI secret provider — W3-03 | Missing masked controlling-terminal/noninteractive provider/tests | Invocation-local session, stdin reserved JSON body once | No daemon/unlock sharing/passphrase record/digest | JSON/no terminal returns unlock_required; no prompt fallback | No argv/env/files/pipe/agent secret ingress C3 | absent | W3-03 using W3-02; R-admin |
| R07 desktop prompt deduplication/unlock block — W3-07/12 | E-session blocked source state + automatic transport behavior; missing prompt coordinator/tests | One selected-key session across UI/windows/worker, current source | Explicit unlock clears auth block, not durable pause; recheck source/selection | Cancel/failure suspends affected polling, attention action no repeated modal | Never clear secret still used by worker C3/C6 | partially-present | W3-02 credential adapter, W3-07 interaction/12 worker; R-admin + R-worker |
| R08 host attention suspension/exact approval — W3-03/07/12 | E-ssh approval/typed host outcomes; missing public pin DTO/worker attention model | Exact authority/old/presented fingerprint and transport re-observation | Persist pin after auth, cache-loss fence; no automatic trust recovery | Approval separate from lifecycle; background suspend, not ordinary network backoff | Selected-key session; no generic yes; C3 | partially-present | W2-03/04 + W3-01/02/03 then 07/12; R-admin + R-worker |
| R09 durable policy/attempt/status and no process discovery — W3-01/06/12 | E-policy/E-reserve; latest category/ref evidence, incomplete attempt/deadline DTO | Explicit repo/policy/result/reservation; desktop memory state separately | No CLI worker/process registry inference, recovery suspension separate C6 | Policy controls use adapter IDs; not implicit poll | No unlock needed for status; no stale session claim | partially-present | W2-04/08 domain + W3-01/02; R-read + R-domain |
| R10 one desktop-owned scheduler across windows — W3-12 | Missing worker lifetime/start/stop/types/tests; E-observe not scheduler | Enabled/unpaused eligible SSH repos, one process worker | Initial poll respecting backoff, no separate executable/service/election | Sequential due repos/manual checks, busy bounded recheck C6 | Shared process session, prompt attention C3 | absent | W2-08 one-shot prerequisite, W3-12; R-worker |
| R11 completion-based deadlines/backoff/suspend/clock handling — W3-12 | E-policy delay_for/backoff primitives; missing scheduling/clock/result tests | Completion time, monotonic wait/UTC restart, future-deadline clamp | No catch-up; configured interval success; failures 60→900 cap; busy/yield/cancel not failure | Explicit Poll now bypasses pause/backoff once; resume due without clearing other blocks | Host/unlock failures attention not endless network backoff C6 | partially-present | W2-08 attempt outcomes + W3-12 scheduling; R-worker |
| R12 config/key/registration changes invalidate queued work — W3-02/12 | E-remotes/E-keyreg invalidate durable observations; no scheduler queue/tests | Recheck pause/remote/key before attempt, active endpoint observations | Remove stops scheduling; re-add remote not silently unpause | Active work stops/reconciles at safe point before new config C2/C6 | Selection/source session recheck, no premature secret clear | partially-present | W2-04/08 state + W3-02/03 + W3-12; R-adapter + R-worker |
| R13 one-shot CLI polling/index lifetime and manual overlap — W3-04/06/12 | E-index heartbeat scoped; missing poll W2-08/CLI adapter/lifetime tests | Explicit operation then exit; domain lease/reservation shared with desktop | No resident worker/child poller; no scheduled auto publish C4 | Safe cancel/result before exit, exact IDs; overlap needs real SSH evidence | One invocation session; no desktop unlock sharing | pending-Wave-02 | W2-08 + W3-02/04/06; R-domain + R-worker |
| R14 safe shutdown admission stop/flush/credential retirement — W3-12 | Missing shutdown protocol/UI/tests; E-reserve cancel primitive only | All live operations, draft revisions, progress/session lifetime | Preserve journals/drafts; no live reservation steal or rollback claim | Ten seconds feedback not exit deadline, responsive wait then stop | Clear credentials after worker use ends C6 | absent | W3-02 cancellation, W3-08 flush, W3-12 lifecycle; R-worker + R-draft |
| R15 DNS/connect/SSH/teardown stall characterization — before W3-12 | E-start per-call/address budgets, E-reserve safe points; missing total native stall evidence | Final locked libgit2/platform transport, each durable phase | Forced termination interruption; durable effects/retry preserved | DNS/total unbounded, callback presence not stop latency proof | Retain live ownership/secrets while waiting; redacted fixture | partially-present | W3-12 readiness with W2-03/08 APIs, W3-13 native gate; R-worker |
| R16 poll-triggered indexing and dirty draft preservation — W3-12 | Missing complete poll/index orchestration W2-08; E-index/E-observe separate | Clean canonical worktree vs unsaved in-memory draft, newer observations | Automatic fetch/clean FF/materialize/index only; never overwrite dirty draft/push/merge/checkpoint | Same worker lifetime; typed partial/index pending, safe cancel | Selected-key only transport; bodies stay draft/read payloads C0/C5 | pending-Wave-02 | W2-08, W3-01/02/08 then 12; R-domain + R-worker + R-draft |

## Coverage, gaps and acceptance checkpoint

Checklist totals above are the acceptance denominator, not numbers of implemented
commands. Every taxonomy verb has one row, including the separately grouped
repair and lifecycle verbs. Invocation, editor/draft/focus/resource and scheduler/
shutdown rows are explicit. Shared contract references deliberately retain
requirements absent from current APIs. No requested API was executed to infer
behavior. Status totals across the 106 rows: **53 partially-present**, **2
present-but-unverified**, **37 absent**, **14 pending-Wave-02**; zero
present-and-tested. Test source is evidence of existing assertions, not today's
passing coverage or native/frontend readiness.

Key blockers and earlier owners:

1. **Reads and identity resolution (W3-01):** no repository enumeration/full
   safe canonical DTO/public-key text/host inspection/unified operation/conflict
   reads. Existing metadata snapshots, pure parsers and writable inspectors
   cannot be substituted silently. Needs schemas, malformed/complete list and
   URL/error redaction proofs.
2. **Adapter safety (W3-02):** external request identity/digests, context/creation
   observations, consent IDs/expiry/effect previews, progress/cancel/recovery
   mapping and completed-effect/cache-loss proofs missing. Existing operation
   target matching is narrower; generation protection intent and document
   no-op header rewriting require explicit proof/fix in approved early owners.
3. **Domain collaboration (W2-05…10):** clean sync (05), merge/conflicts (06),
   comment publication (07), actual one-shot poll/materialization (08), promotion
   (09), closure (10) unavailable. Future types/private fixture transfers and
   advertisement do not retire these blockers. Audit final closed-ticket guard,
   remote-deleted republish consent, whole-branch previews and cleanup-only replay.
4. **Narrow admin/author boundaries (W3-03/04):** public identity setter,
   terminal provider, configured-host approval adapter, folder-only operation,
   repair/adoption and patch/null/closure filters need their approved owners,
   using W3-01/02 rather than shortcuts.
5. **Desktop/runtime (W3-07/08/09/12):** shell owned models/focus, real protected
   draft persistence, accepted lossless editor and worker/shutdown integration
   are missing. Startup scaffold/heartbeat/session tests are not product evidence.
   W3-13 still owns integrated native matrix/journeys/responsiveness.

This fulfills a **provisional inventory**, not the entry gate. Final-main audit
is mandatory after all Wave 02 Cycles merge and before W3-01 planning, with
actual public symbols/types/tests/preflights and their side effects rechecked.
Any changed baseline invalidates these line anchors until re-audited. Unallocated
material behavior discovered then blocks the consumer plan and returns to the
user/approved owner; it is not silently assigned downstream.

Documentation validation passed using an external Node assertion script (not
added to the project): exact 36-group/56-verb taxonomy comparison + three
invocations; 59/31/16 rows; every inventory row has eight nonempty columns,
status/owner/re-audit and public evidence or explicit gap; all evidence codes and C0–C6
sections resolve. Managed scalar frontmatter matches canonical document required
fields; the ULID is valid and unique among existing tracked Markdown. This is
not execution of the Rust canonical parser. Relative file/heading links, source
line ranges and 63 exact named test/function anchors validate. Source/tests are
unchanged in the inspected range from main. Whitespace is checked using
`git diff --no-index --check /dev/null docs/research/wave-03-api-audit.md` (covers
the new file) and `git diff --check`. The handoff records exact validation output,
sole-file local commit/range and post-commit empty-index evidence. No
Rust/API/native/remote execution is claimed.
