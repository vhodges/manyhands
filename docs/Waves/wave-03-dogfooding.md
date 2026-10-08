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

On 2026-10-07 the product owner directed a replan of the Cycle structure: a
two-Cycle serial foundation, a CLI track and a desktop track that run in
parallel, and one joint gate. The outcome, scope, boundaries and integration
conditions above and below are unchanged. Cycle allocation, ordering,
identifiers and the Wave 02 entry dependency changed; see
[Tracks And Cycles](#tracks-and-cycles) and
[Approval And Remaining Readiness](#approval-and-remaining-readiness).

## Authority And Traceability

This Wave implements [PRD v0.6](../PRD/mvp.md), the
[MVP architecture RFC](../RFC/mvp-rfc.md), and these approved technical RFCs:

- [Desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md).
- [CLI contract](../RFC/cli-contract.md).
- [Application runtime and polling](../RFC/application-runtime-and-polling.md).
- [Canonical content and comment schema](../RFC/canonical-content-and-comment-schema.md).
- [Git workflow and conflict recovery](../RFC/git-workflow-and-conflict-recovery.md).
- [Repository index persistence and refresh](../RFC/repository-index-persistence-and-refresh.md).
- [Authentication and credential handling](../RFC/authentication-and-credential-handling.md).
- [Test and compatibility strategy](../RFC/test-and-compatibility-strategy.md).
- [Ticket relationships and short codes](../RFC/ticket-relationships-and-short-codes.md).

The [RFC approval register](../RFC/wave-03-rfc-review.md) records the approved
choices and remaining feasibility obligations. Wave planning does not reopen
rich-text/source mode, desktop-only background polling, canonical-only in-app
conflict resolution, the shared SSH key, or full ULIDs as mutation selectors.
The short code added on 2026-10-07 is a search key for people and does not
change that last rule.

| PRD area | Owning Cycles and acceptance |
| --- | --- |
| `MH-PROD-001/002`, `MH-UX-001` | F1–F2, C1–C5 and D1–D6 compose complete front ends; G1 proves non-developer dogfooding. |
| `MH-REPO-001`–`004`, `MH-CRED-001` | Shared bridges in F2; CLI administration in C2; desktop onboarding/settings in D1; selected-key/session evidence in both tracks. |
| `MH-CONTENT-001`–`004` | Shared reads and bridges in F1/F2; CLI discovery/authoring in C1/C3; desktop discovery in D1 and rich-text/source authoring in D2/D3. |
| `MH-CONTENT-005/006` | Fields, validation, short-code generation and graph reads in F1/F2; CLI queries in C1 and authoring in C3; desktop display in D1 and editing in D3; made-ready reporting on close in C5/D5. |
| `MH-COMMENT-001/002` | CLI discussion in C4; desktop discussion in D4; ordering/rebuild recovery in G1. |
| `MH-COLLAB-001/002` | Shared request/recovery boundary in F2; local authoring in C3/D2/D3. |
| `MH-COLLAB-003/004` | Deliberate CLI collaboration in C4/C5; desktop collaboration in D4/D5; automatic polling in D6. |
| `MH-COLLAB-005/007`, `MH-SCOPE-001` | CLI promotion/closure in C5, desktop in D5, whole-branch effects and interrupted-cleanup evidence in both. |
| `MH-COLLAB-006`, `MH-NFR-003/004/006/008` | Every mutating Cycle; explicit progress, consent, safe-point cancellation, redaction, stale-write and retry evidence. |
| `MH-INDEX-001`–`003`, `MH-NFR-001/007` | Discovery and refresh/rebuild in F1/C1/C3/D1; local drafts in D2; polling in D6, preserving offline use and index-only nonmutation. |
| `MH-CLI-001` | F1/F2 and C1–C5 cover every approved command, input/result schema and exit class; no desktop dependencies. |
| `MH-NFR-002/005` | Native/headless checks throughout; keyboard and native desktop evidence in D1–D6 and G1. |

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

The 2026-10-07 replan was written on ticket `01M4CAQGTMM3JYFMWQJCZXP1KZ`, in
its own branch and worktree created from freshly fetched main
`ae3d69035d901d352cb709eacaee5fc2b51084de`. At that baseline Wave 02 Cycles
01–05 are closed and Cycle 06 is in flight in its own worktree; Cycles 07–10
are not started. The CLI is still a scaffold and the desktop a hello-world
view. The replan changes documents only and ran no Rust checks.

## Entry Gate And Readiness Work

Wave approval can precede Wave 02 completion. Since the 2026-10-07 replan,
Wave 03 implementation can too: each Cycle is gated by the Wave 02 Cycles it
actually consumes, not by the complete Wave 02 integration gate. Items 1, 3
and the baseline checks in 7 gate the first implementation Cycle. Item 2 gates
each Cycle individually. Items 4–6 have the specific deadlines below, so Linux
work can proceed while native access is arranged:

1. The Wave and governing RFCs are approved and the source clarifications below
   are adopted. Approval does not itself prove feasibility or authorize code.
2. Every Wave 02 Cycle named for a Wave 03 Cycle in the table below has review
   and verification evidence on the main branch used for that Cycle's
   implementation, before that Cycle's implementation plan is approved. An
   unmerged or in-flight Wave 02 branch never satisfies a dependency, and
   transport-only success is not synchronization, polling, merge, promotion or
   closure evidence. Wave 02's complete integration gate is required before G1
   exits.
3. A current-main API audit maps every approved CLI/desktop operation to the
   actual library entry point or a narrowly scoped Wave 03 addition. It covers
   preflight/confirmation, expected observations, request replay, progress,
   cancellation, selected-key sessions and partial outcomes. Refresh it before
   F1 planning for everything F1, F2 and the Cycles without an unfinished
   Wave 02 dependency consume. Re-audit the rows owned by an unfinished Wave 02
   Cycle when that Cycle merges and before its first Wave 03 consumer is
   planned. Missing shared behavior must be assigned to a foundation Cycle, or
   handled through the shared-library change rule below, before a consumer is
   planned.
4. The editor investigation evaluates a pinned `zorite-editor` first and a
   pinned Velotype extraction if needed. Record dependency compatibility with
   the one GPUI Kit graph, host-controlled persistence/resources, rich/source
   fidelity, undo, tables, IME and focus, plus remaining native checks. The
   product owner selected `zorite-editor` on 2026-10-07, so the selection is
   made. The [feasibility record](../research/wave-03-editor-feasibility.md)
   found that its load normalization changes source bytes. The product owner
   accepted those byte changes on 2026-10-07, and the desktop/editor RFC is
   amended to bound the exception: normalization is not a user edit, a
   no-change save still writes nothing, and content is never dropped. D3
   planning records the pinned release's normalizations and the remaining
   native checks. This item is on the desktop track only; it does not gate
   the foundation, the CLI track, D1 or D2.
5. Before D6 implementation, runtime readiness reconciles the final
   Wave 02 transport startup and cancellation contract. Preserve initialization before GPUI/worker threads;
   measure blocked DNS/connect/SSH/teardown behavior and define responsive
   safe-point stopping without claiming an unsupported total deadline.
6. Plan implementation and early desktop evidence around Linux, as the product
   owner selected on 2026-10-05. The product owner's early beta testers have a
   mix of Windows and Mac machines. The Release Owner confirms specific tester
   assignments, OS/architecture coverage and scheduling at each desktop Cycle.
   Arrange test sessions in time to complete their journeys and additional architecture
   smoke checks before G1 exits. Linux-only evidence cannot satisfy the
   approved final native matrix.
7. Required Rust checks pass at the implementation baseline, or a documented
   existing failure is explicitly resolved before its dependent Cycle starts.
   Each Cycle has its ticket, approved artifacts and execution authorization.

| Wave 03 Cycle | Wave 02 Cycles required on main | Why |
| --- | --- | --- |
| F1, C1, C2, C3, D1, D2, D3 | 01–04 | Key registry, session unlock, transport and remote observation are read, administered or displayed. All were closed at the replan baseline. |
| F2 | 01–05 | Replay and confirmation are proven against real clean synchronization as well as local saves. Closed at the replan baseline. |
| C4 | 06, 07 | Divergent sync and conflict recovery; comment publication. |
| C5 | 08, 09, 10 | One-shot polling; promotion; closure. |
| D4 | 06, 07 | Comment publication; a divergent sync must surface as a preserved conflict state. |
| D5 | 06, 09, 10 | Conflict resolution; promotion; closure. |
| D6 | 08 | One-shot polling and materialization. |
| G1 | Complete Wave 02 integration gate | Integrated journeys on the final revision. |

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
| Prepared confirmations need useful observations even when the destination repository or item does not yet exist. | F2 binds creation intent to the target parent/path, absence observation and supplied non-secret input; execution rechecks them. Preview must not initialize a repository as a side effect. |
| The CLI lists closed tickets by default, while the charter suggested hiding them in the desktop. Free-form status cannot serve as a closure filter. | Preserve the CLI all-ticket default. The explicit closure filter and desktop default below are approved and adopted in their RFCs; do not infer closure from status text. |
| Some requested interface operations lack an obvious public API in the baseline. | F1 and F2 own narrow read, identity, public-key, folder and repair bridges; refresh the audit per the entry gate, rather than exposing internal SQLite/Git manipulation in either front end. |
| Full command-specific JSON schemas and malformed-content DTO details are not yet published. | Every CLI Cycle publishes schemas/fixtures with its verbs. C5 cannot exit until the complete command inventory has schemas and documented exit behavior. |
| Marker-only repair needs an identity and context before it can become conforming; duplicate/mismatched IDs are different problems. | F2 provides explicit adoption previews, stable ID allocation and retry evidence. Ambiguous identity stays non-editable; no implicit migration during refresh. |
| A pristine worktree may have an unsaved editor draft, including while a CLI action changes or cleans up its context. | D2/D3 preserve base/current drafts and reject stale writes. A removed context leaves an exportable/recoverable draft; never silently recreate a closed ticket context or lose the draft. |
| Existing transport bootstrap runs before argument handling and emits plain stderr on failure in the unmerged Cycle 03 design. | C1 must preserve the pre-thread initialization boundary while adapting handled startup failures to the CLI envelope when `--json` is recognizable. Verify actual merged startup rather than overwriting it. |

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

Use a short serial foundation, then two parallel tracks, then one joint gate.
The foundation puts every headless contract that both front ends consume in
the library before either front end is built on it: read models, result and
error types, request replay, confirmation and the narrow mutation bridges. The
CLI track and the desktop track then proceed independently over those services
and converge at the gate.

The Wave approved on 2026-10-05 ran thirteen Cycles in one sequence, CLI
first. The product owner directed this two-track replan on 2026-10-07. It has
fourteen Cycles, and its longest serial path is nine (F1, F2, D1–D6, G1). The
CLI track still makes the project's own ticket/comment operations usable
early. The desktop track still builds native and keyboard evidence during
each desktop Cycle, not only at the end.

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
short codes as identities or mutation selectors, a custom status vocabulary
and automated attachment imports. Ticket analytics beyond plan and critical
path, typed links, priority, assignment and full-text search are on the PRD
roadmap, not in this Wave.
In-app conflict editing covers owned canonical Markdown. Other conflicts get
external-tool guidance and safe re-observation before deliberate resume.

## Tracks And Cycles

### Structure And Identifiers

| Phase | Cycles | Runs |
| --- | --- | --- |
| Foundation | F1, F2 | Serially, before either track. |
| CLI track | C1–C5 | Serially within the track, in parallel with the desktop track. |
| Desktop track | D1–D6 | Serially within the track, in parallel with the CLI track. |
| Convergence | G1 | After C5 and D6. |

Cycle documents are named `wave-03-foundation-NN-…`, `wave-03-cli-NN-…`,
`wave-03-desktop-NN-…` and `wave-03-gate-NN-…`, the track form the
[MVP architecture RFC](../RFC/mvp-rfc.md) permits. Cycle tickets use `wave: "03"`
and the identifier as `cycle`, for example `cycle: "C1"`. The paths below are
planned documents, not files or tickets claimed to exist.

The identifiers replace the 2026-10-05 numbering as follows. Documents written
before the replan, including the provisional
[API audit](../research/wave-03-api-audit.md) and its `W3-01`…`W3-13` owner
codes, use the earlier numbers.

| 2026-10-05 Cycle | Replanned owner |
| --- | --- |
| 01 Shared Results And CLI Inspection | F1 (library reads, result model); C1 (CLI shell and read verbs) |
| 02 Request Replay And Confirmation | F2 |
| 03 CLI Repository And Credential Administration | F2 (identity and host-approval bridges); C2 (commands) |
| 04 CLI Authoring And Discovery Maintenance | F1 (closure filter); F2 (folder and repair/adoption bridges); C3 (commands) |
| 05 CLI Discussion, Synchronization And Conflict Recovery | C4 |
| 06 CLI Completion And One-Shot Polling | C5 |
| 07 Desktop Shell, Onboarding And Discovery | D1 |
| 08 Recoverable Drafts And External-Change Handling | D2 |
| 09 Rich-Text And Source Authoring | D3 |
| 10 Desktop Discussion And Deliberate Synchronization | D4 |
| 11 Desktop Conflict Recovery, Promotion And Closure | D5 |
| 12 Desktop Background Polling And Shutdown | D6 |
| 13 Native Journeys And Dogfooding Gate | G1 |

### Track Rules

1. **Dependencies.** Every Cycle depends on the Wave entry gate and its own
   Wave 02 dependencies. F2 depends on F1. Both tracks open when F2 exits.
   Within a track each Cycle depends on every preceding Cycle of that track.
   G1 depends on C5 and D6. No Cycle may depend on a later Cycle.
2. **Track independence.** No Cycle depends on a Cycle of the other track. A
   plan that needs the other front end to exist is moved, split or changed
   before approval; it does not create a hidden cross-track dependency.
3. **Early starts.** C1 and D2 consume only F1 and may start when F1 exits if
   capacity allows. This is an allowance, not a third track: at most one Cycle
   is in flight per track, plus at most one foundation Cycle.
4. **Cross-front-end evidence.** A scenario that needs both front ends acting
   on the same repository belongs to G1. Inside a track, external change and
   concurrency are proven with a second actor that calls the headless library
   directly, in a separate process where the scenario is cross-process.
5. **Shared-library changes.** After F2, a track Cycle that finds a missing or
   wrong headless capability does not work around it in its front end and does
   not carry the fix inside its own front-end branch. The change gets its own
   ticket, branch and worktree, stays narrowly scoped to the library, is
   reviewed and merged to main first, and both tracks rebase onto it at their
   next work boundary. Front-end Cycle branches otherwise leave
   `src/repository.rs` and `src/repository/` unchanged, which keeps the two
   tracks' branches from conflicting in the same files.
6. **Lifecycle bindings.** F2 defines confirmation and replay generically over
   effect previews and proves them with the operations available at its
   baseline. Promotion and closure do not exist until Wave 02 Cycles 09/10.
   Whichever of C5 or D5 is planned first lands the promotion/closure binding
   and its real-fixture replay evidence as a shared-library change under rule
   5; the other consumes it.
7. **Mutation evidence.** Every mutating Cycle must include a named failure
   or interruption case and replay evidence in its detailed plan.

Readiness deadlines apply to their named Cycles. Windows/macOS access does not
block Linux implementation.

## Foundation

### F1: Headless Read Boundary And Result Model

**Planned document:** `docs/Cycles/wave-03-foundation-01-read-boundary-and-results.md`

**Purpose:** Establish the headless front-end boundary both tracks read
through, so neither front end scrapes SQLite, Git or the filesystem itself.

**In scope:** Explicit target and repository resolution; the shared result,
error and recovery-action taxonomy; redaction; and versioned, serializable
JSON v1 DTOs for every read. Read services cover repository/identity/remote/
key/host inspection, public-key text, document/ticket/comment reads,
operation inspection, index/poll status and ID generation. Conflict
inspection is not in F1: no conflict record exists before Wave 02 Cycle 06,
so whichever of C4 or D5 is planned first lands that read as a shared-library
change under track rule 5 (product owner, 2026-10-07). Ticket
lists carry the approved lifecycle closure filter, independent of status text.
Ticket DTOs carry `slug`, `parent`, `deps`, readiness and relationship
problems. Read services cover the ticket relationship queries: ready, blocked,
dependencies in both directions, children, cycles, plan, critical path and
find by short code, over index edge records rebuilt from canonical files.
Fill narrow missing read APIs and report stale/malformed state honestly; reads
do not perform implicit refresh or network contact. Reads list what the index
holds and never list a directory or scan; the indexer picks up new files, and
a short delay before a new file appears is accepted (product owner,
2026-10-08). Comment reads return one flat, ordered list with each entry's
depth, and report a comment's `created_by` as its author without writing it.

**Out of scope:** Command parsing, human output, exit codes and help; desktop
views; canonical/config/key writes; network attempts; request replay; polling
scheduling. No CLI verb or desktop screen is delivered by this Cycle.

**Exit evidence:** Library integration tests against real repositories cover
every read service, typed malformed rows, deterministic ordering and complete
lists. Golden DTO schemas and redaction fixtures are published. An open ticket
whose status text is `closed` is listed as lifecycle-open.
Relationship queries are proven across primary and active worktrees with an
unresolved dependency, a merged-in cycle and a duplicate short code, each
reported and none repaired. No display or GPUI
dependency, no canonical mutation and no resident process.

### F2: Request Replay, Confirmation And Shared Mutation Bridges

**Planned document:** `docs/Cycles/wave-03-foundation-02-replay-confirmation-and-bridges.md`

**Purpose:** Provide the shared safety boundary every mutation adapter consumes,
and the narrow headless mutations both front ends need but the baseline lacks.

**In scope:** External request IDs mapped to domain operation IDs, exact target
and semantic-input matching, expected observations, non-secret request records,
completed-effect reconciliation, two-phase confirmation, progress/cancellation
events and typed recovery actions. Bind absent destinations, including a
not-yet-created repository root; preview must not initialize a repository as a
side effect. Use actual Wave 02 preflights and reservations; do not create an
alternate journal or lock authority. Add the shared bridges: confirmed local
identity configuration for create/enable, configured-host approval through
scoped transport verification, folder creation, and marker-only
repair/adoption with stable ID allocation.
Ticket create and save accept `deps` and `parent`, write them in canonical
form and reject a cycle before any write. Create generates the short code;
add the explicit short-code assign operation, repository-local initials and
the optional repository prefix.
Comment creation writes the comment's optional `created_by` field from the
confirmed Git identity, as the canonical schema RFC defines; F1 only reads it
(product owner, 2026-10-08).

**Out of scope:** New Git lifecycle algorithms, CLI verbs, terminal secret
interaction, desktop confirmations and background scheduling. Promotion and
closure bindings follow track rule 6.

**Exit evidence:** Real save and clean-sync fixtures prove stale preview
rejection, changed-input rejection, accepted-consent retry, lost output
reconciliation, cache-loss recovery and no duplicate effects. Cancellation is
observed at approved safe points. Each bridge has expected-observation, retry
and failure evidence; host approval publishes nothing and refreshes no remote
ref; ambiguous identity stays non-editable.
Golden vectors fix the short-code and initials derivations; a short code is
unchanged by rename, identity change and prefix change. No body/secret enters request
records.

## CLI Track

### C1: CLI Shell And Inspection

**Planned document:** `docs/Cycles/wave-03-cli-01-shell-and-inspection.md`

**Purpose:** Make existing state inspectable through the CLI without enabling
mutation commands prematurely.

**In scope:** Command parsing, no-argument/help/version, human output, JSON v1
envelopes over the F1 DTOs, stable error/exit mapping, and every read verb F1
serves, including `ticket list --closure open|closed|all` defaulting to `all`.
Include the relationship read verbs: `ticket ready`, `blocked`, `deps`,
`children`, `cycles`, `plan`, `critical-path` and `find`. A short code
passed as `--id` is rejected as an invalid ID.
Preserve early transport initialization while adapting handled startup
failures to the CLI envelope when `--json` is recognizable, and test that
failure output boundary against the actual merged startup.

**Out of scope:** Canonical/config/key writes, network attempts, request replay,
desktop views and polling scheduling. Help must distinguish the implemented
command set; absent verbs must not return fabricated success.

**Exit evidence:** Real-repository read tests through the binary, golden
schemas and human/JSON error tests; no display or desktop dependency; no
canonical mutation or resident process.

### C2: CLI Repository And Credential Administration

**Planned document:** `docs/Cycles/wave-03-cli-02-repository-and-credentials.md`

**Purpose:** Make repository onboarding and SSH setup usable without GUI or
external Git/key-generation commands.

**In scope:** Create/enable/remove, confirmed local identity, remote add/remove/
publication selection, key generation/import/select/clear/unregister/delete,
host approve/replace, terminal secret interaction and noninteractive recovery.
Apply the approved missing-identity input refinement. Use F2 for request IDs,
confirmation, replay and the identity/host bridges.

**Out of scope:** Content authoring, item synchronization, secret inputs through
argv/environment/files/pipes, forge APIs and background workers.

**Exit evidence:** Empty/existing repository onboarding, missing identity,
cancelled/wrong unlock, exact host replacement, imported-key non-deletion,
generated-key deletion retry and configuration failures. Publish schemas for
each new verb; prove no prompt consumes JSON/body stdin or persists a secret.

### C3: CLI Authoring And Discovery Maintenance

**Planned document:** `docs/Cycles/wave-03-cli-03-authoring-and-discovery.md`

**Purpose:** Make local document/ticket lifecycle and explicit discovery repair
available for agents and project dogfooding.

**In scope:** Document/ticket create/save, document move, folder creation,
explicit repair/adoption, index refresh/rebuild and identity recovery, over
the F2 bridges.
Ticket create and save take `deps` and `parent`; add `ticket slug-assign`,
initials on `repo identity-set` and the prefix on `repo create`/`enable`.
Record the Wave 03 Cycle dependencies in the project's own tickets when these
operations meet this gate. Preserve unknown metadata, ULIDs and body content; use actual
context provisioning and source/destination observations. Empty folders are
local until they contain tracked content; do not add hidden placeholder
commits. Index-only actions never fetch or rewrite canonical state.

**Out of scope:** Discussion submission, remote synchronization, promotion,
closure, draft storage or GUI editing. Status text `closed` is ordinary metadata.

**Exit evidence:** Offline checkpoints/no-ops, stale writes, invalid/malformed
repair, adoption retry, move collision, index loss/corruption and commit-success/
refresh-failure preserve actual Git/filesystem state. New JSON schemas and
documented command examples pass. Switch ticket create/update dogfooding to the
CLI when those operations meet this gate.

### C4: CLI Discussion, Synchronization And Conflict Recovery

**Planned document:** `docs/Cycles/wave-03-cli-04-discussion-and-sync.md`

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

### C5: CLI Completion And One-Shot Polling

**Planned document:** `docs/Cycles/wave-03-cli-05-completion-and-polling.md`

**Purpose:** Complete the approved headless command contract.

**In scope:** Document promotion, ticket close, their prepare/confirm/resume
flows, explicit poll once and polling policy controls. Verify whole-branch
effects, closure identity, local-only pending publication, remote-first cleanup
ordering and cleanup-only retries through Wave 02 services.
Ticket close reports the tickets it made ready. Complete every
command's input/output schema, help, recovery example and exit mapping.

**Out of scope:** CLI background scheduling/indexing, desktop behavior,
automatic cleanup and new merge/transport policies.

**Exit evidence:** All approved CLI commands are accounted for. Real fixtures
prove dirty-primary blocking, stale consent, push/cleanup failures, no duplicate
close/merge, later primary publication and safe explicit polling. Run the six
CLI journey counterparts, use explicit polling for background discovery, and
prove command exit leaves no resident worker. This is the CLI contract gate.

## Desktop Track

### D1: Desktop Shell, Onboarding And Discovery

**Planned document:** `docs/Cycles/wave-03-desktop-01-shell-and-onboarding.md`

**Purpose:** Provide a keyboard-usable desktop over the foundation services.

**In scope:** Repository navigation, document trees, ticket lists and closure
filter, multiple open-item tabs, active-context provenance, malformed/stale
views, repository/remotes/identity/key/host settings, manual refresh/rebuild,
operation area and asynchronous service calls. Lists default to lifecycle-open
tickets with a visible Closed/All filter.
Show each ticket's short code, offer ready and blocked filters with reasons,
find by short code with every match listed, and show a ticket's dependencies,
dependents, parent and children read-only. Settings mutations use F2
confirmation, replay and bridges directly; they do not wait for or call the
CLI. Viewing does not provision a context. Restore focus after prompts; do not
serialize credential-bearing state into UI diagnostics. Preserve pre-thread
transport bootstrap and Root/init rules.

**Out of scope:** Editable item bodies, comment submission, lifecycle completion
buttons and automatic polling. Unavailable actions remain visibly unavailable.

**Exit evidence:** Native startup and keyboard onboarding/credential/inspection
checks; responsive navigation during delayed scans, failed refresh, unavailable
repositories and unlock/host prompts. No display dependency leaks into the CLI.
At least one real repository is browsable end to end without fabricated rows.

### D2: Recoverable Drafts And External-Change Handling

**Planned document:** `docs/Cycles/wave-03-desktop-02-recoverable-drafts.md`

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

### D3: Rich-Text And Source Authoring

**Planned document:** `docs/Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md`

**Purpose:** Deliver the selected editor experience with trustworthy local saves.

**Additional prerequisite:** `zorite-editor` is the selected editor (product
owner, 2026-10-07), with its load normalization accepted under the amended
desktop/editor RFC. This Cycle's design records the pinned release and its
exact normalizations before the implementation plan is approved.

**In scope:** Integrate the chosen native editor through GPUI Kit, shared draft/
undo state across rich/source modes, metadata controls, item create/edit/save,
document moves/folders, explicit repair and stale-edit review.
Metadata controls edit `deps` and `parent` by choosing tickets and explain a
rejected cycle before any save. Use D2 recovery
and the F2 observation/progress results and bridges. Support source fallback
for unsupported constructs, safe local resource rendering and keyboard
formatting. Expose unsaved/local checkpoint/index-pending states accurately.

**Out of scope:** Replacing Markdown with an authoritative editor AST/database,
remote resource execution, comment publication, promotion/closure or auto-save
to Git. A mode switch or no-change save creates no checkpoint.

**Exit evidence:** Golden rich/source round-trips, untouched-byte preservation
outside the recorded load normalization, a normalized-on-load item that is
not dirty and writes nothing on a no-change save,
unknown YAML values, CRLF/Unicode, tables/unsupported blocks, undo/IME/focus,
draft persistence errors, stale edits made by a second library-level actor and
old async completions. Native offline edit/checkpoint journey succeeds; valid
recovery never loses newer text.

### D4: Desktop Discussion And Deliberate Synchronization

**Planned document:** `docs/Cycles/wave-03-desktop-04-discussion-and-sync.md`

**Purpose:** Make discussion and explicit publication understandable and usable.

**In scope:** Ordered threaded comments/replies with rich/source composers and
draft recovery, Post and sync (or Save locally without a remote), item/primary Sync, progress,
safe-point cancel, selected-key/host recovery and publication retry. Keep an
unsaved item draft separate from synchronized checkpoints. Show saved comments
once after partial success and resume publication without resubmission.

**Out of scope:** Conflict result editing, promotion/closure and scheduled
polling. Conflicts remain visible preserved recovery states until D5.

**Exit evidence:** Keyboard discussion/explicit-sync journeys against real SSH
repositories, offline/local pending, failed publication, repeated-submit guard,
rebuild/thread ordering, unlock cancellation and continued UI responsiveness.

### D5: Desktop Conflict Recovery, Promotion And Closure

**Planned document:** `docs/Cycles/wave-03-desktop-05-recovery-and-completion.md`

**Purpose:** Complete deliberate desktop collaboration lifecycles.

**In scope:** Base/local/remote/result comparison, explicit canonical-path
resolution and checkpoint/resume, external guidance for unsupported conflicts,
Approve and merge/Close ticket effect summaries, exact confirmation, final
draft-save boundary and partial publication/cleanup recovery. Show all affected
branch paths and lifecycle closure metadata separately from project status.
A completed close lists the tickets it made ready.
Primary publication success is not equivalent to cleanup success.

**Out of scope:** Arbitrary code/binary resolution writes, forced overwrite,
ticket reopening and automated confirmation or cleanup.

**Exit evidence:** Native keyboard conflict and completion journeys; stale
resolution/confirmation, dirty primary, local-only close/promotion, remote
cleanup rejection, interruption and cleanup-only replay. Preserved drafts and
contexts remain inspectable and no retry repeats a merge or closure checkpoint.

### D6: Desktop Background Polling And Shutdown

**Planned document:** `docs/Cycles/wave-03-desktop-06-polling-and-shutdown.md`

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
exceptional-context preservation, pause versus unlock state, safety against a
concurrent explicit operation from a second process calling the library,
stalled transport with responsive stopping status, configuration changes and
shutdown/restart. No background push/checkpoint/merge/cleanup; no remaining
worker after graceful process exit.

## Convergence

### G1: Native Journeys And Dogfooding Gate

**Planned document:** `docs/Cycles/wave-03-gate-01-native-journeys-and-dogfooding.md`

**Purpose:** Close the evidence matrix and demonstrate the complete product with
real collaborators on supported platforms.

**In scope:** Consolidate per-Cycle evidence, run all six PRD desktop journeys
and explicit CLI counterparts on the final integrated revision, complete native
architecture/keyboard/input checks, measure responsiveness, and fix defects
within approved scope. Run the cross-front-end scenarios the tracks could not:
a CLI edit, cleanup or poll against an item with an open desktop draft, and a
CLI operation concurrent with the desktop background worker. Include two
trusted people using separate clones.
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

The detailed Cycle plans must turn this mapping into actual files/APIs against
the main branch each Cycle is implemented on. It is an allocation of work, not
a claim those public APIs exist.

| Capability needing scrutiny | Owner | Required boundary |
| --- | --- | --- |
| Repository inventory, full canonical reads, malformed DTOs, public-key/host inspection, closure filter | F1 | Read-only library APIs; no front-end SQLite scraping or implicit fetch. |
| Request digest, creation observations, consent, progress/cancel and replay | F2 | Wrap actual domain operations; preserve Git authority and existing leases. |
| Ticket relationship fields, cycle rejection, short-code generation and graph reads | F1 (reads), F2 (writes) | Index edge records rebuilt from canonical files; ULID remains the only identity and mutation selector. |
| Comment author (`created_by`) | F1 (reads), F2 (writes) | Written once at comment creation from the confirmed Git identity; never derived from Git history. |
| Missing identity during create/enable and public identity configuration | F2 | Explicit confirmed local persistence, including a not-yet-created root. |
| Configured-host approval | F2 | Scoped transport verification; no publication or remote-ref refresh. |
| Folder creation and marker-only adoption/repair | F2 | Scoped filesystem boundary, stable IDs, expected observations, no hidden migration. |
| Command grammar, envelopes, exit classes, terminal secret provider | C1, C2 | CLI-only; no domain logic and no secret through argv/environment/files/pipes. |
| Discussion/sync/conflict and resume | C4, D4, D5 | Wave 02 operations and owned canonical paths only. |
| Conflict inspection read | First of C4 or D5, as a shared-library change | Read-only over the Wave 02 Cycle 06 conflict record; same DTO and redaction rules as F1. |
| One effective copy per item across worktrees | Defect ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G`, before F1 | An item worktree contributes only its own item; every other item is read from primary. |
| Promotion/close preflight and safe cleanup bindings | First of C5 or D5, as a shared-library change | Real effect preview and existing publication/cleanup ordering. |
| Explicit one-shot polling | C5 | Existing Wave 02 one-shot operation; no resident worker. |
| Protected draft store and base snapshots | D2 | Separate from canonical files/cache/journals; versioned recoverable local files. |
| Rich/source integration and stale response handling | D3 | One compatible GPUI graph; editor does not own Git, credential or network policy. |
| Background scheduling and process exit | D6 | Desktop lifetime, existing one-shot operation, truthful safe-point stopping. |

Broad refactoring of `src/repository.rs`, a second event framework or a generic
automation engine is not implicit scope. Split modules only where the actual
adapter/domain addition needs a clear boundary and retains existing behavior.

## Integration Gate

Wave 03 is complete only when all fourteen Cycles have approved exit evidence
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
| Editor feature claims do not establish compatible, lossless embedding | `zorite-editor` is selected and pinned; its load normalization is accepted and bounded by the amended desktop/editor RFC; D3 records the pinned normalizations as golden fixtures, proves a normalized-on-load item is not dirty, and verifies one GPUI graph and native input. A pin change re-reviews its normalizations. |
| API gaps become UI-specific filesystem/SQL/Git shortcuts | Assign additions to the headless library in the owning early Cycle and test both consumers through it. |
| A stale confirmation or ambiguous retry mutates a different target | Exact observations and request identity, no blind replay after cache loss, real interruption tests in F2 and every consumer. |
| Unsaved text disappears after poll, CLI cleanup or an old async result | Draft/base preservation, generation checks and explicit recovery; no automatic overwrite or context recreation. |
| Credentials or source leak through diagnostics/JSON | Typed redacted outcomes, terminal-only secret input and isolated privacy fixtures; requested read bodies are distinct from logs/errors. |
| Early CLI implementation drifts from the published contract | Publish schemas/help with each verb; inventory audit and all exit-class coverage at C5. |
| Transport shutdown outlasts the desktop's feedback threshold | Responsive still-stopping state; retain live ownership/secrets until safe completion; test forced interruption separately. |
| Beta tester machines do not cover every required native target, or sessions are delayed | Work on Linux first; Release Owner confirms Windows/macOS tester assignments, architectures and scheduling at each desktop Cycle and obtains missing evidence before G1 exit. Platform acceptance is not silently reduced. |
| An intermediate desktop is mistaken for the complete product | Mark unavailable workflows; accumulate real evidence per Cycle and require the integrated final gate. |
| Parallel tracks edit the same headless files and conflict, or one front end grows its own domain shortcut | Shared-library change rule: separate ticket, library-only scope, merged to main first; front-end branches leave `src/repository*` unchanged. |
| The foundation's DTOs and confirmation model are fixed before any front end consumes them | F1/F2 publish schemas and prove them through library integration tests over real repositories; a mismatch found by C1 or D1 returns as a shared-library change, not a front-end workaround. |
| A Wave 03 Cycle starts on a Wave 02 capability that is unmerged or still changing | Per-Cycle Wave 02 dependency table; in-flight branches never satisfy it; re-audit the owning rows when each Wave 02 Cycle merges. |
| The tracks each pass alone but fail together | Each track proves external change with a second library-level actor; G1 owns the real CLI-against-desktop scenarios and cannot exit without them. |
| Too much work is packed into one Cycle | Split before its plan is approved if independent deliverables cannot be reviewed/tested together; update this Wave and downstream prerequisites rather than add hidden subcycles. |

## Approval And Remaining Readiness

The Wave's scope, outcome, boundaries and integration conditions were approved
on 2026-10-05 and are unchanged. On 2026-10-07 the product owner directed the
replan recorded here and selected its three shaping decisions: a library-only
two-Cycle foundation, a per-Cycle Wave 02 dependency in place of the complete
Wave 02 entry gate, and track-based Cycle identifiers and document names. The
product owner approved the detailed text of this revision on 2026-10-07,
including the track form added to the MVP architecture RFC, on ticket
`01M4CAQGTMM3JYFMWQJCZXP1KZ`.

On the same date the product owner approved `zorite-editor` as the editor.
The product owner also accepted the byte changes its load normalization makes;
the desktop/editor and canonical RFCs are amended accordingly. Neither
decision completes the native evidence D3 and G1 owe.

On 2026-10-07 the product owner also directed that ticket dependencies, a
parent hierarchy and a human short code be added in this Wave, with the ULID
remaining the canonical identity. PRD v0.6 and the
[ticket relationships and short codes RFC](../RFC/ticket-relationships-and-short-codes.md) define them. The product
owner approved that RFC and this Wave's allocation of it on 2026-10-07, on
ticket `01M4CGDANTBDR4T8AGFZP8ZGP7`.

On 2026-10-08 the product owner authorized amendments on the F1 ticket,
`01M4CC0VMQ7R15A7M9SPN3KB67`, to bring this Wave and the CLI, relationships
and index RFCs into agreement with rulings made while implementing F1. In
this document they add writing the comment `created_by` field to F2, and
state that reads list what the index holds and that comments are read as a
flat list.

The refreshed API audit, native tester/machine
assignments and implementation-baseline evidence remain open; none is implied by design
approval. The product owner confirmed that early beta testers have Windows and
Mac machines. Continue Linux-first work, then arrange the required native
sessions with those testers and fill any architecture coverage gaps. Machine
availability is not completed evidence.

The create/enable identity input and closure-filter refinements are adopted in
their owning RFCs. Establish readiness evidence and prepare F1 in its ticket
worktree when its prerequisites are met. This record does not start
implementation, publish the branch or close any ticket.
