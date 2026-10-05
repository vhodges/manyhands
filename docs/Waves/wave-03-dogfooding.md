---
title: "Wave 03: Dogfooding"
date: 2026-10-05
status: approved
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46YZ3VTG83C7WPP008AQSWT"
---

# Wave 03: Dogfooding

## Outcome

Wave 03 makes the local and remote domain workflows available through a usable
desktop application and a documented, headless CLI. At its exit, a collaborator
can onboard a repository, manage credentials, discover and edit items, discuss
work, recover conflicts, promote documents and close tickets without operating
Git manually for the supported canonical workflows. Agents can perform the
equivalent explicit operations with stable JSON, concurrency preconditions,
confirmation and recoverable partial-success results.

The desktop provides rich-text editing with Markdown source mode, recoverable
drafts, keyboard workflows and an application-owned polling/indexing worker.
CLI commands perform one requested operation and exit. There is no CLI daemon,
separate polling executable, shared singleton service or cross-process poller
election. A future server/CI change watcher that triggers automations belongs
to a separate PRD/enhancement.

The product owner approved this Wave on 2026-10-05 at reviewed revision
`4ea2aa9`, including its create/enable identity input and closure-filter
refinements. Approval establishes the Wave scope and sequence; it does not
establish entry-gate evidence. Its Cycles require their own tickets, documents,
designs, plans, approval and implementation authorization.

## Authority And Traceability

This Wave implements [PRD v0.5](../PRD/mvp.md), the
[MVP architecture RFC](../RFC/mvp-rfc.md), and these approved technical RFCs:

- [Desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md).
- [CLI contract](../RFC/cli-contract.md).
- [Application runtime and polling](../RFC/application-runtime-and-polling.md).
- [Canonical content and comment schema](../RFC/canonical-content-and-comment-schema.md).
- [Git workflow and conflict recovery](../RFC/git-workflow-and-conflict-recovery.md).
- [Repository index persistence and refresh](../RFC/repository-index-persistence-and-refresh.md).
- [Authentication and credential handling](../RFC/authentication-and-credential-handling.md).
- [Test and compatibility strategy](../RFC/test-and-compatibility-strategy.md).

The [RFC approval register](../RFC/wave-03-rfc-review.md) records the approved
choices and remaining feasibility obligations. Wave planning does not reopen
rich-text/source mode, desktop-only background polling, canonical-only in-app
conflict resolution, the shared SSH key, or full ULIDs as mutation selectors.

| PRD area | Owning Cycles and acceptance |
| --- | --- |
| `MH-PROD-001/002`, `MH-UX-001` | Cycles 01–12 compose complete front ends; Cycle 13 proves non-developer dogfooding. |
| `MH-REPO-001`–`004`, `MH-CRED-001` | CLI administration in 03; desktop onboarding/settings in 07; selected-key/session evidence in both. |
| `MH-CONTENT-001`–`004` | CLI discovery/authoring in 01/04; desktop discovery in 07 and rich-text/source authoring in 08/09. |
| `MH-COMMENT-001/002` | CLI discussion in 05; desktop discussion in 10; ordering/rebuild recovery in 13. |
| `MH-COLLAB-001/002` | Shared request/recovery boundary in 02; local authoring in 04/08/09. |
| `MH-COLLAB-003/004` | Deliberate CLI collaboration in 05/06; desktop collaboration in 10/11; automatic polling in 12. |
| `MH-COLLAB-005/007`, `MH-SCOPE-001` | CLI promotion/closure in 06, desktop in 11, whole-branch effects and interrupted-cleanup evidence in both. |
| `MH-COLLAB-006`, `MH-NFR-003/004/006/008` | Every mutating Cycle; explicit progress, consent, safe-point cancellation, redaction, stale-write and retry evidence. |
| `MH-INDEX-001`–`003`, `MH-NFR-001/007` | Discovery and refresh/rebuild in 01/04/07; local drafts in 08; polling in 12, preserving offline use and index-only nonmutation. |
| `MH-CLI-001` | Cycles 01–06 cover every approved command, input/result schema and exit class; no desktop dependencies. |
| `MH-NFR-002/005` | Native/headless checks throughout; keyboard and native desktop evidence in 07–13. |

## Planning Baseline

Planning uses the existing `planning/wave-03-rfcs` branch and
`.worktrees/wave-03-rfcs` checkout. Before this document was written, the
worktree and main checkout were clean. A fresh `git fetch origin main` observed
`89ce24d0c5b99c817ec81615fe610f65d7c81a99`. Local main remained
`a21a31aeea5aaf7c03cf7692848ddadb5dde1242`, including the already-retained local
tooling/research commits. Rebase onto local main was a no-op; ancestry checks
passed for both main and origin/main. The before/after planning HEAD was
`58130244786901f03b46a6a232258e570260cf2a`.

At that baseline, Wave 01 and Wave 02 Cycles 01/02 are available; the CLI is
still a scaffold and the desktop a hello-world view. The parallel Wave 02 Cycle
03 checkout was inspected read-only at `5519991`, with further work in progress.
Its transport startup/timeout design informs the risks below, but no unmerged
code or completion claim is imported into this branch. Wave 02 Cycles 04–10
remain dependencies, not APIs this Wave assumes already exist.

No Cycle tickets or implementation plans are created by this Wave document.
Before planning each implementation Cycle, create/reuse its one Manyhands
ticket and use its branch/worktree. Use the CLI for ticket/comment operations
as soon as the required verbs are implemented; until then use canonical paths.
Refresh each Cycle from current main and record verification/review checkpoints.
The present worktree continues to hold Wave/RFC planning only.

## Entry Gate And Readiness Work

Wave approval can precede Wave 02 completion. Items 1–3 and the baseline
checks in 7 gate the first implementation Cycle. Items 4–6 have the specific
deadlines below, so Linux work can proceed while native access is arranged:

1. The Wave and governing RFCs are approved and the source clarifications below
   are adopted. Approval does not itself prove feasibility or authorize code.
2. Wave 02's complete integration gate has review and verification evidence on
   the main branch used for implementation. Transport-only success is not
   synchronization, polling, merge, promotion or closure evidence.
3. A current-main API audit maps every approved CLI/desktop operation to the
   actual library entry point or a narrowly scoped Wave 03 addition. It covers
   preflight/confirmation, expected observations, request replay, progress,
   cancellation, selected-key sessions and partial outcomes. Missing behavior
   must be assigned to an earlier Cycle before a consumer is planned.
4. The editor investigation evaluates a pinned `zorite-editor` first and a
   pinned Velotype extraction if needed. Record dependency compatibility with
   the one GPUI Kit graph, host-controlled persistence/resources, rich/source
   fidelity, undo, tables, IME and focus, plus remaining native checks. Select
   the integration approach before editor Cycle implementation planning.
5. Before Cycle 12 implementation, runtime readiness reconciles the final
   Wave 02 transport startup and cancellation contract. Preserve initialization before GPUI/worker threads;
   measure blocked DNS/connect/SSH/teardown behavior and define responsive
   safe-point stopping without claiming an unsupported total deadline.
6. Plan implementation and early desktop evidence around Linux, as the product
   owner selected on 2026-10-05. The product owner's early beta testers have a
   mix of Windows and Mac machines. The Release Owner confirms specific tester
   assignments, OS/architecture coverage and scheduling at each desktop Cycle.
   Arrange test sessions in time to complete their journeys and additional architecture
   smoke checks before Cycle 13 exits. Linux-only evidence cannot satisfy the
   approved final native matrix.
7. Required Rust checks pass at the implementation baseline, or a documented
   existing failure is explicitly resolved before its dependent Cycle starts.
   Each Cycle has its ticket, approved artifacts and execution authorization.

Items 3–6 are readiness activities with the deadlines above, not a hidden Cycle 00
or an instruction to implement a prototype during this planning request.
Any executable feasibility probe needs its own bounded scope and authorization.
If a probe shows that the approved product cannot be delivered, amend the
owning RFC/PRD before planning the affected implementation.

## Clarifications And Gaps Found During Wave Planning

| Finding | Resolution and gate |
| --- | --- |
| Wave 02 allows 10 seconds per TCP address and 30 seconds per blocking SSH call, with DNS/total transfer outside those budgets. A ten-second process-exit promise is unsupported. | Product owner selected a **ten-second feedback threshold**: report still stopping, retain recovery state and wait for a safe point. Runtime/test wording is amended accordingly; native cancellation behavior remains required evidence. |
| The desktop RFC mistakenly tied the literal ticket status `closed` to lifecycle closure. | Align desktop/CLI wording with the canonical schema: status is free-form; only the explicit close lifecycle writes `closed_at`/`closed_by`. Include an open ticket whose status text is `closed` in tests. |
| The CLI create/enable input table had no way to provide missing Git identity in noninteractive mode; `repo identity-set` cannot configure a repository that does not yet exist. | Approved with this Wave and adopted in the CLI RFC: optional input `identity: {name, email}` for create/enable, included in its explicit confirmation preview before local persistence. Existing library requests already accept identity. |
| Prepared confirmations need useful observations even when the destination repository or item does not yet exist. | Cycle 02 binds creation intent to the target parent/path, absence observation and supplied non-secret input; execution rechecks them. Preview must not initialize a repository as a side effect. |
| The CLI lists closed tickets by default, while the charter suggested hiding them in the desktop. Free-form status cannot serve as a closure filter. | Preserve the CLI all-ticket default. The explicit closure filter and desktop default below are approved and adopted in their RFCs; do not infer closure from status text. |
| Some requested interface operations lack an obvious public API in the baseline. | Cycles 01/03/04 own narrow read, identity, public-key, folder and repair bridges; refresh the audit after Wave 02, rather than exposing internal SQLite/Git manipulation in either front end. |
| Full command-specific JSON schemas and malformed-content DTO details are not yet published. | Every CLI Cycle publishes schemas/fixtures with its verbs. Cycle 06 cannot exit until the complete command inventory has schemas and documented exit behavior. |
| Marker-only repair needs an identity and context before it can become conforming; duplicate/mismatched IDs are different problems. | Cycle 04 provides explicit adoption previews, stable ID allocation and retry evidence. Ambiguous identity stays non-editable; no implicit migration during refresh. |
| A pristine worktree may have an unsaved editor draft, including while a CLI action changes or cleans up its context. | Cycles 08/09 preserve base/current drafts and reject stale writes. A removed context leaves an exportable/recoverable draft; never silently recreate a closed ticket context or lose the draft. |
| Existing transport bootstrap runs before argument handling and emits plain stderr on failure in the unmerged Cycle 03 design. | Cycle 01 must preserve the pre-thread initialization boundary while adapting handled startup failures to the CLI envelope when `--json` is recognizable. Verify actual merged startup rather than overwriting it. |

The two CLI/discovery refinements were approved with this Wave and adopted in
the CLI and desktop RFCs:

- Create/enable identity is explicit and confirmed, stored only in repository
  config; missing identity yields recovery without inventing a global identity.
- CLI `ticket list --closure open|closed|all` defaults to `all`, independent of
  `--status`. Desktop lists default to lifecycle-open tickets and offer a visible
  Closed/All filter; direct opening by ID still exposes a closed ticket. Existing
  approved list ordering and malformed-entry visibility remain unchanged.

These refinements do not add reopening, controlled status vocabularies, cloning,
or a schema migration. They preserve the canonical closure contract.

## Delivery Approach And Boundaries

Use shared contracts first, CLI capabilities next, then desktop workflows over
the same services. This makes the project's own ticket/comment operations usable
early and gives desktop work established recovery outcomes. A desktop-first
sequence would delay agent dogfooding; alternating every CLI/UI feature would
require both shells before either is useful. The chosen sequence still builds
native and keyboard evidence during each desktop Cycle, not only at the end.

All production domain/runtime behavior stays in the headless library. Desktop
modules depend on GPUI through `gpui-kit`; editor dependencies are desktop-only
and must use the same compatible GPUI package graph. Initialize the transport
before threads, initialize GPUI Kit inside `app.run`, and create `Root` first
for each window. Neither front end becomes a second Git orchestration engine.

Wave 02 retains ownership of SSH transfer, remote-ref semantics, one-shot
polling/materialization, merge, promotion, closure and cleanup algorithms.
Wave 03 adds necessary caller boundaries and presentation, with explicit
changes to those algorithms requiring the owning RFC/Cycle review. No separate
service, resident CLI worker, arbitrary conflict-file writes or automatic
publication is introduced.

The following are outside this Wave: boards/rollups, templates/scaffolding,
multi-repository planning, Git-forge key upload, application authorization,
HTTP(S) publication, cloning, ticket reopening, comment editing/deletion,
lossy short IDs, a custom status vocabulary and automated attachment imports.
In-app conflict editing covers owned canonical Markdown. Other conflicts get
external-tool guidance and safe re-observation before deliberate resume.

## Ordered Cycles

Each Cycle depends on the Wave entry gate and every preceding Cycle. Listed
readiness deadlines apply to their named Cycles; Windows/macOS access does not
block Linux implementation. Additional dependencies identify the important
consumed capability. No Cycle
may depend on a later Cycle. The paths below are planned documents, not files
or tickets claimed to exist. Every mutating Cycle must include a named failure
or interruption case and replay evidence in its detailed plan.

### Cycle 01: Shared Results And CLI Inspection

**Planned document:** `docs/Cycles/wave-03-cycle-01-results-and-cli-inspection.md`

**Purpose:** Establish the headless front-end boundary and make existing state
inspectable through the CLI without enabling mutation commands prematurely.

**In scope:** Command parsing, explicit target resolution, no-argument/help/version,
human output, JSON v1 envelopes/DTOs, stable error/exit mapping and redaction.
Expose repository/identity/remote/key/host inspection, public-key output,
document/ticket/comment reads, operation/conflict inspection, index/poll status
and ID generation. Fill narrow missing read APIs and report stale/malformed
state honestly; reads do not perform implicit refresh or network contact.
Preserve early transport initialization and test its failure output boundary.

**Out of scope:** Canonical/config/key writes, network attempts, request replay,
desktop views and polling scheduling. Help must distinguish the implemented
command set; absent verbs must not return fabricated success.

**Exit evidence:** Real-repository read tests, typed malformed rows, deterministic
ordering and complete-list output; golden schemas and human/JSON error tests;
no display or desktop dependency; no canonical mutation or resident process.

### Cycle 02: Request Replay And Confirmation

**Planned document:** `docs/Cycles/wave-03-cycle-02-request-replay-and-confirmation.md`

**Purpose:** Provide the shared safety boundary every mutation adapter consumes.

**In scope:** External request IDs mapped to domain operation IDs, exact target
and semantic-input matching, expected observations, non-secret request records,
completed-effect reconciliation, two-phase confirmation, progress/cancellation
events and typed recovery actions. Bind absent destinations and whole-branch
promotion/closure effects. Use actual Wave 02 preflights and reservations; do
not create an alternate journal or lock authority.

**Out of scope:** New Git lifecycle algorithms, mass exposure of CLI mutations,
desktop confirmations and background scheduling.

**Exit evidence:** Real save, sync and confirmed lifecycle fixtures prove stale
preview rejection, changed-input rejection, accepted-consent retry, lost output
reconciliation, cache-loss recovery and no duplicate effects. Cancellation is
observed at approved safe points. No body/secret enters request records.

### Cycle 03: CLI Repository And Credential Administration

**Planned document:** `docs/Cycles/wave-03-cycle-03-cli-repository-and-credentials.md`

**Purpose:** Make repository onboarding and SSH setup usable without GUI or
external Git/key-generation commands.

**In scope:** Create/enable/remove, confirmed local identity, remote add/remove/
publication selection, key generation/import/select/clear/unregister/delete,
host approve/replace, terminal secret interaction and noninteractive recovery.
Apply the approved missing-identity input refinement.
Use Cycle 02 for request IDs, confirmation and replay; use scoped transport
verification for host approval without publishing or refreshing remote refs.

**Out of scope:** Content authoring, item synchronization, secret inputs through
argv/environment/files/pipes, forge APIs and background workers.

**Exit evidence:** Empty/existing repository onboarding, missing identity,
cancelled/wrong unlock, exact host replacement, imported-key non-deletion,
generated-key deletion retry and configuration failures. Publish schemas for
each new verb; prove no prompt consumes JSON/body stdin or persists a secret.

### Cycle 04: CLI Authoring And Discovery Maintenance

**Planned document:** `docs/Cycles/wave-03-cycle-04-cli-authoring-and-discovery.md`

**Purpose:** Make local document/ticket lifecycle and explicit discovery repair
available for agents and project dogfooding.

**In scope:** Document/ticket create/save, document move, folder creation,
explicit repair/adoption, index refresh/rebuild, identity recovery and the
approved closure-filter refinement. Preserve unknown metadata, ULIDs and body
content; use actual context provisioning and source/destination observations.
Empty folders are local until they contain tracked content; do not add hidden
placeholder commits. Index-only actions never fetch or rewrite canonical state.

**Out of scope:** Discussion submission, remote synchronization, promotion,
closure, draft storage or GUI editing. Status text `closed` is ordinary metadata.

**Exit evidence:** Offline checkpoints/no-ops, stale writes, invalid/malformed
repair, adoption retry, move collision, index loss/corruption and commit-success/
refresh-failure preserve actual Git/filesystem state. New JSON schemas and
documented command examples pass. Switch ticket create/update dogfooding to the
CLI when those operations meet this gate.

### Cycle 05: CLI Discussion, Synchronization And Conflict Recovery

**Planned document:** `docs/Cycles/wave-03-cycle-05-cli-discussion-and-sync.md`

**Purpose:** Let CLI users collaborate deliberately and recover incomplete work.

**In scope:** Comment/reply submission with stable IDs, item and primary sync,
canonical conflict resolution and operation resume for these actions. Explain
that comment submit can publish other checkpointed context work but not caller
drafts. Preserve publication-pending versus local checkpoint outcomes and exact
republish consent for a remotely deleted branch.

**Out of scope:** Promotion/closure/cleanup adapters, automatic retries of
manual lifecycle intent, arbitrary code conflict writes or a resident poller.

**Exit evidence:** Real two-clone SSH tests cover comment publish success/failure,
ordered replies, one saved comment after retry, clean/divergent sync, stale
resolutions, external noncanonical repair and no secret/raw-server output.
Ctrl-C and lost stdout preserve recoverable effects and meaningful exit status.
Switch ticket comment dogfooding to the CLI when supported.

### Cycle 06: CLI Completion And One-Shot Polling

**Planned document:** `docs/Cycles/wave-03-cycle-06-cli-completion-and-polling.md`

**Purpose:** Complete the approved headless command contract.

**In scope:** Document promotion, ticket close, their prepare/confirm/resume
flows, explicit poll once and polling policy controls. Verify whole-branch
effects, closure identity, local-only pending publication, remote-first cleanup
ordering and cleanup-only retries through Wave 02 services. Complete every
command's input/output schema, help, recovery example and exit mapping.

**Out of scope:** CLI background scheduling/indexing, desktop behavior,
automatic cleanup and new merge/transport policies.

**Exit evidence:** All approved CLI commands are accounted for. Real fixtures
prove dirty-primary blocking, stale consent, push/cleanup failures, no duplicate
close/merge, later primary publication and safe explicit polling. Run the six
CLI journey counterparts, use explicit polling for background discovery, and
prove command exit leaves no resident worker. This is the CLI contract gate.

### Cycle 07: Desktop Shell, Onboarding And Discovery

**Planned document:** `docs/Cycles/wave-03-cycle-07-desktop-shell-and-onboarding.md`

**Purpose:** Provide a keyboard-usable desktop over the proven headless services.

**In scope:** Repository navigation, document trees, ticket lists and closure
filter, multiple open-item tabs, active-context provenance, malformed/stale
views, repository/remotes/identity/key/host settings, manual refresh/rebuild,
operation area and asynchronous service calls. Viewing does not provision a
context. Restore focus after prompts; do not serialize credential-bearing state
into UI diagnostics. Preserve pre-thread transport bootstrap and Root/init rules.

**Out of scope:** Editable item bodies, comment submission, lifecycle completion
buttons and automatic polling. Unavailable actions remain visibly unavailable.

**Exit evidence:** Native startup and keyboard onboarding/credential/inspection
checks; responsive navigation during delayed scans, failed refresh, unavailable
repositories and unlock/host prompts. No display dependency leaks into the CLI.
At least one real repository is browsable end to end without fabricated rows.

### Cycle 08: Recoverable Drafts And External-Change Handling

**Planned document:** `docs/Cycles/wave-03-cycle-08-recoverable-drafts.md`

**Purpose:** Establish preservation of unsaved work before editor integration.

**In scope:** Versioned owner-protected draft files outside SQLite, atomic
debounced writes, explicit flush/recovery failure outcomes, base/current source
observations, restore/export/discard, saved-revision retirement and stale-change
comparison models for items and comment composers. Protect drafts on repository
removal, index rebuild and context disappearance; do not restore into a closed
ticket or newly incompatible context automatically.

**Out of scope:** Rich-text engine integration, automatic source merging,
canonical writes from the draft store and unlimited crash-loss guarantees.

**Exit evidence:** Real-file crash/partial-write/access/migration cases preserve
the last successfully flushed draft. Save completing while newer text exists
retires only the saved revision. Repository/cache removal does not delete drafts;
stale or vanished targets remain recoverable and never silently overwritten.

### Cycle 09: Rich-Text And Source Authoring

**Planned document:** `docs/Cycles/wave-03-cycle-09-rich-text-and-source-authoring.md`

**Purpose:** Deliver the selected editor experience with trustworthy local saves.

**Additional prerequisite:** The pinned editor feasibility/selection record is
accepted before this Cycle's detailed design/implementation plan is approved.

**In scope:** Integrate the chosen native editor through GPUI Kit, shared draft/
undo state across rich/source modes, metadata controls, item create/edit/save,
document moves/folders, explicit repair and stale-edit review. Use Cycle 08
recovery and Cycle 02 observation/progress results. Support source fallback for
unsupported constructs, safe local resource rendering and keyboard formatting.
Expose unsaved/local checkpoint/index-pending states accurately.

**Out of scope:** Replacing Markdown with an authoritative editor AST/database,
remote resource execution, comment publication, promotion/closure or auto-save
to Git. A mode switch or no-change save creates no checkpoint.

**Exit evidence:** Golden rich/source round-trips, untouched-byte preservation,
unknown YAML values, CRLF/Unicode, tables/unsupported blocks, undo/IME/focus,
draft persistence errors, stale CLI/poll edits and old async completions. Native
offline edit/checkpoint journey succeeds; valid recovery never loses newer text.

### Cycle 10: Desktop Discussion And Deliberate Synchronization

**Planned document:** `docs/Cycles/wave-03-cycle-10-desktop-discussion-and-sync.md`

**Purpose:** Make discussion and explicit publication understandable and usable.

**In scope:** Ordered threaded comments/replies with rich/source composers and
draft recovery, Post and sync (or Save locally without a remote), item/primary Sync, progress,
safe-point cancel, selected-key/host recovery and publication retry. Keep an
unsaved item draft separate from synchronized checkpoints. Show saved comments
once after partial success and resume publication without resubmission.

**Out of scope:** Conflict result editing, promotion/closure and scheduled
polling. Conflicts remain visible preserved recovery states until Cycle 11.

**Exit evidence:** Keyboard discussion/explicit-sync journeys against real SSH
repositories, offline/local pending, failed publication, repeated-submit guard,
rebuild/thread ordering, unlock cancellation and continued UI responsiveness.

### Cycle 11: Desktop Conflict Recovery, Promotion And Closure

**Planned document:** `docs/Cycles/wave-03-cycle-11-desktop-recovery-and-completion.md`

**Purpose:** Complete deliberate desktop collaboration lifecycles.

**In scope:** Base/local/remote/result comparison, explicit canonical-path
resolution and checkpoint/resume, external guidance for unsupported conflicts,
Approve and merge/Close ticket effect summaries, exact confirmation, final
draft-save boundary and partial publication/cleanup recovery. Show all affected
branch paths and lifecycle closure metadata separately from project status.
Primary publication success is not equivalent to cleanup success.

**Out of scope:** Arbitrary code/binary resolution writes, forced overwrite,
ticket reopening and automated confirmation or cleanup.

**Exit evidence:** Native keyboard conflict and completion journeys; stale
resolution/confirmation, dirty primary, local-only close/promotion, remote
cleanup rejection, interruption and cleanup-only replay. Preserved drafts and
contexts remain inspectable and no retry repeats a merge or closure checkpoint.

### Cycle 12: Desktop Background Polling And Shutdown

**Planned document:** `docs/Cycles/wave-03-cycle-12-desktop-polling-and-shutdown.md`

**Purpose:** Keep discovery current inside the desktop process while preserving
manual priority and unsaved work.

**In scope:** One application-owned worker across windows, launch poll, existing
interval/pause/backoff state, explicit Poll now and policy controls, fair
repository scheduling, suspend/clock recovery, worker/UI credential handoff,
prompt deduplication and truthful status. Reuse Wave 02 reservations and safe
points; preserve unsaved drafts across clean-worktree fast-forwards. On exit,
stop admitting work, request cancellation, flush drafts, show still-stopping
feedback at ten seconds and wait safely. Do not clear worker-used credentials
or reclaim a live operation merely because the threshold elapsed.

**Out of scope:** CLI daemon, process discovery/heartbeat tables, shared
services, multiple-resident-poller coordination or a guaranteed total transport
deadline. External forced termination is tested as interruption, not rollback.

**Exit evidence:** Real SSH background update/materialization journey,
exceptional-context preservation, pause versus unlock state, concurrent explicit
CLI/manual operation safety, stalled transport with responsive stopping status,
configuration changes and shutdown/restart. No background push/checkpoint/merge/
cleanup; no remaining worker after graceful process exit.

### Cycle 13: Native Journeys And Dogfooding Gate

**Planned document:** `docs/Cycles/wave-03-cycle-13-native-journeys-and-dogfooding.md`

**Purpose:** Close the evidence matrix and demonstrate the complete product with
real collaborators on supported platforms.

**In scope:** Consolidate per-Cycle evidence, run all six PRD desktop journeys
and explicit CLI counterparts on the final integrated revision, complete native
architecture/keyboard/input checks, measure responsiveness, and fix defects
within approved scope. Include two trusted people using separate clones.
Windows/macOS tester assignments and coverage remain tracked; Linux work can proceed first,
but this Cycle cannot exit with their required native evidence missing.
Provide runnable CLI examples and desktop recovery instructions with the tested
build artifacts. Record host OS/display, revision, lockfile, steps and outcomes.

**Out of scope:** New product capabilities, silently relaxing a failing gate,
new installers/auto-update/signing commitments or simulated journey evidence.

**Exit evidence:** Every integration condition below has a linked automated or
reviewed manual artifact. Open data-loss, consent, secret exposure, canonical
fidelity or required-platform failures block completion. A skipped required
platform check is not a passing gate.

## Capability Ownership And API Audit

The detailed Cycle plans must turn this mapping into actual files/APIs after
Wave 02 merges. It is an allocation of work, not a claim those public APIs exist.

| Capability needing scrutiny | Owner | Required boundary |
| --- | --- | --- |
| Repository inventory, full canonical reads, malformed DTOs, public-key/host inspection | 01 | Read-only library APIs; no front-end SQLite scraping or implicit fetch. |
| Request digest, creation observations, consent, progress/cancel and replay | 02 | Wrap actual domain operations; preserve Git authority and existing leases. |
| Missing identity during create/enable and public identity configuration | 03 | Explicit confirmed local persistence, including a not-yet-created root. |
| Folder creation and marker-only adoption/repair | 04 | Scoped filesystem boundary, stable IDs, expected observations, no hidden migration. |
| Discussion/sync/conflict and resume | 05 | Wave 02 operations and owned canonical paths only. |
| Promotion/close preflight, safe cleanup and explicit polling | 06 | Real effect preview and existing publication/cleanup ordering. |
| Protected draft store and base snapshots | 08 | Separate from canonical files/cache/journals; versioned recoverable local files. |
| Rich/source integration and stale response handling | 09 | One compatible GPUI graph; editor does not own Git, credential or network policy. |
| Background scheduling and process exit | 12 | Desktop lifetime, existing one-shot operation, truthful safe-point stopping. |

Broad refactoring of `src/repository.rs`, a second event framework or a generic
automation engine is not implicit scope. Split modules only where the actual
adapter/domain addition needs a clear boundary and retains existing behavior.

## Integration Gate

Wave 03 is complete only when all thirteen Cycles have approved exit evidence
and all of the following hold:

- Both front ends use the same authoritative domain operations. CLI commands
  implement the complete approved inventory with schemas, documented human/JSON
  results and meaningful exit classes; no GUI dependency or resident worker is
  required for CLI use.
- Desktop users complete onboarding, multi-item navigation, rich/source editing,
  metadata, discussion, save, sync, canonical conflict recovery, promotion,
  closure, polling control and recovery with visible keyboard focus.
- No-op/mode-switch fidelity, unknown metadata, unsupported Markdown and stale
  draft preservation are proven; partial write/checkpoint/index/publication/
  cleanup outcomes remain distinguishable and retryable without duplication.
- Background work obeys its permitted fetch/clean-fast-forward/materialization
  boundary, manual priority, user pause and session unlock behavior. Shutdown
  reports ongoing work at ten seconds and reaches a safe stop without a false
  hard-deadline or rollback claim.
- All six real-repository/SSH desktop journeys and explicit CLI counterparts
  have evidence, including local-only and injected-failure variants. Two trusted
  collaborators complete shared-item edit/conflict/discussion from separate clones.
- The approved native matrix is complete: all five existing targets build and
  pass headless/SSH/CLI tests; full desktop journeys run on Windows, macOS and
  Linux Wayland; remaining built architectures have the approved native smoke
  evidence. Record IME, focus and high-DPI results rather than infer them from builds.
- The 1,000-item/100 KiB reference fixture meets approved p95 input-response
  and cancellation-feedback targets on a recorded reference machine. Larger
  unsupported content is preserved with usable source fallback, never truncated.
- Required project checks and applicable CLI/desktop smoke tests pass on the
  integrated tree; retain current Wave 01/02 regression and secret-redaction evidence.

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
devenv shell -- cargo run --locked --features desktop --bin manyhands
```

The desktop smoke test requires an active display. Native CI may use Cargo
directly under the existing approved exception. Tests use isolated home,
Git/SSH configuration, application data and disposable repositories, never
developer keys or worktrees. Documentation-only Wave planning does not run or
claim these implementation checks.

## Risks And Controls

| Risk | Control and owner |
| --- | --- |
| Wave 02 changes while this Wave is planned | Rebase/audit at the implementation boundary; retain its authoritative transport/operation behavior. Technical Lead owns readiness. |
| Editor feature claims do not establish compatible, lossless embedding | Pin and evaluate charter candidates before editor planning; verify one GPUI graph, native input and golden source fixtures. |
| API gaps become UI-specific filesystem/SQL/Git shortcuts | Assign additions to the headless library in the owning early Cycle and test both consumers through it. |
| A stale confirmation or ambiguous retry mutates a different target | Exact observations and request identity, no blind replay after cache loss, real interruption tests in Cycle 02 and every consumer. |
| Unsaved text disappears after poll, CLI cleanup or an old async result | Draft/base preservation, generation checks and explicit recovery; no automatic overwrite or context recreation. |
| Credentials or source leak through diagnostics/JSON | Typed redacted outcomes, terminal-only secret input and isolated privacy fixtures; requested read bodies are distinct from logs/errors. |
| Early CLI implementation drifts from the published contract | Publish schemas/help with each verb; inventory audit and all exit-class coverage at Cycle 06. |
| Transport shutdown outlasts the desktop's feedback threshold | Responsive still-stopping state; retain live ownership/secrets until safe completion; test forced interruption separately. |
| Beta tester machines do not cover every required native target, or sessions are delayed | Work on Linux first; Release Owner confirms Windows/macOS tester assignments, architectures and scheduling at each desktop Cycle and obtains missing evidence before Cycle 13 exit. Platform acceptance is not silently reduced. |
| An intermediate desktop is mistaken for the complete product | Mark unavailable workflows; accumulate real evidence per Cycle and require the integrated final gate. |
| Too much work is packed into one Cycle | Split before its plan is approved if independent deliverables cannot be reviewed/tested together; update this Wave and downstream prerequisites rather than add hidden subcycles. |

## Approval And Remaining Readiness

Wave approval is recorded above. Editor selection, final Wave 02 API/transport
audit, native tester/machine assignments and implementation-baseline evidence
remain open; none is implied by design approval. The product owner confirmed
that early beta testers have Windows and Mac machines. Continue Linux-first
work, then arrange the required native sessions with those testers and fill
any architecture coverage gaps. Machine availability is not completed evidence.

The create/enable identity input and closure-filter refinements are adopted in
their owning RFCs. Establish readiness evidence and prepare Cycle 01 in its
ticket worktree when its prerequisites are met. This approval record
does not start implementation, publish the branch or close any ticket.
