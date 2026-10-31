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
   Amended 2026-10-30 by C4 (decision 16): for C4 the owner accepted a
   departure from this item's timing. The design's Baseline Facts, read from
   current source, served as C4's planning audit, and C4's Task 0 refreshed
   the audit's C4 rows after the design was approved.
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

C4, C5 and D5 also need the Cycle 10 follow-up ticket "2.10 follow up:
closure recovery gaps before C4, C5 and D5" (`01M57SKVF6TQ3QM6AXJ35JGGSY`)
merged to main first (added 2026-10-18). Under track rule 5 those Cycles cannot
work around its gaps in their front ends:

- C4 and D5 would otherwise offer an in-product resolution for a race in which
  two clones close the same ticket differently, and that resolution always
  fails.
- C5 and D5 would otherwise leave a manual Git merge as the only way out of a
  refused closure checkpoint merge.
- C5 and D5 users would see promotion wording in closure error messages.

The other Cycle 10 cleanup ticket ("2.10 follow up",
`01M57SBA7MKR9BZR7PYECSJ05G`) does not gate any Wave 03 Cycle.

**Final gate checkpoint, 2026-10-27:** the prerequisite's Tasks 0–9 are complete
and its open ticket is `review_ready`. Whole-branch SPEC and QUALITY approve
`0464c42..f92bc00` with zero findings, and scoped local verification is complete;
the [final ledger checkpoint](../plans/2026-10-25-closure-recovery-gate-execution.md#task-9-final-review-ready-checkpoint)
distinguishes pre/post-fix evidence and Linux-only limitations. Production local
start/read/resolve/resume and parent-specific wording are delivered on the ticket
branch, with no owner final acceptance or merge claimed. **The merge prerequisite
above remains in force.** This branch does not deliver CLI verbs, human-mode conflict artifacts or
desktop conflict editing. C5/D5 must adapt the local boundary (amended
2026-10-30 by C4, decision 19, replacing "C4/D5 must adapt"), and C4/D5 still
provide the separate remote-conflict wire boundary: C4 delivers it for the
CLI, and D5 consumes it through D4. C5/D5 still owe lifecycle
prepare/execute, expiry/schema/resume and made-ready reporting. See the
[gate contract](../Cycles/wave-03-closure-recovery-gate.md) and
[current API audit](../research/wave-03-api-audit.md#closure-recovery-library-amendment-2026-10-27).

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
| Wave 02 Cycle 03 is merged; its original bootstrap ran before argument handling and emitted plain stderr on failure. | Amended 2026-10-18 by C1 (ruling R-1): C1 parses arguments first, without starting threads or native Git. Commands opening services initialize transport before any thread or native Git activity; help, version, usage errors and unavailable commands do not initialize it. Handled startup failures use the CLI envelope when `--json` is recognizable, preserving the pre-thread boundary. |

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
   Amended 2026-10-30 by C4 (decision 5): one ordering is recorded here.
   D4 waits until all of C4 is merged, because C4 carries the library
   bindings that D4's Post and Sync consume (comment submission, merging
   synchronization and the conflict data), under rule 5's exception below.
   D4 needs no CLI verb; Part A of C4 is not merged early.
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
   Amended 2026-10-30 by C4 (decision 1): the owner approved C4 as an
   exception. C4's branch carries the library additions its verbs need,
   limited to the files its design lists: the `comment add`,
   `conflict resolve` and `item republish` bindings, the conflict read,
   merging synchronization settings, resume's routes and the durable state
   of an interrupted resolution. They merge with C4 as one Cycle.
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
scoped transport verification, folder creation and the listing of folders
under `docs/` (product owner, 2026-10-10), and marker-only
repair/adoption with stable ID allocation.
Add a confirmed abandon of a pending local operation, which closes its
journal record and changes no file (product owner, 2026-10-10).
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

Amended 2026-10-18 by C1: `path observe` and `ticket check` move to C3,
which owns the repair and authoring flows they feed. C1 includes `folder list`
and by-path `document show`/`ticket show` with optional `--context`, and lists
the moved verbs as planned rather than executing them.

**Out of scope:** Canonical/config/key writes, network attempts, request replay,
desktop views and polling scheduling. Help must distinguish the implemented
command set; absent verbs must not return fabricated success.

**Exit evidence:** Real-repository read tests through the binary, golden
schemas and human/JSON error tests; no display or desktop dependency; no
canonical mutation or resident process.

### C2: CLI Repository And Credential Administration

**Cycle document:** `docs/Cycles/wave-03-cli-02-repository-and-credentials.md`

**Amended 2026-10-20 by C2.** The Cycle, design and plan were approved on
2026-10-22. Prerequisite `01M5B0Z6PNP7X4WH4JV3N0VQJ6` (create/enable leave
the committed configuration in the Git index) merged before approval, so remote
selection follows onboarding without an external Git workaround (decision 1).
Delivery is one Cycle in three independently checked/reviewed Parts (decision 4):
A, mutation layer/terminal/repository/remote/request verbs; B, keys/passphrase
entry; C, hosts/SSH unlock/exit journeys. Task 12 still owns final Cycle verification.

**Purpose:** Make repository onboarding and SSH setup usable without GUI or
external Git/key-generation commands.

**In scope:** Create/enable/remove, confirmed local identity, remote add/remove/
publication selection, key generation/import/select/clear/unregister/delete,
host observe/approve/replace, request show/cancel, terminal secret interaction
and noninteractive recovery. Include optional `ticket_slug_prefix` on create/
enable and optional nullable `initials` on identity-set (decision 5).
Apply the approved missing-identity input refinement. Use F2 for request IDs,
confirmation, replay and the identity/host bridges.

**Out of scope:** Content authoring, item synchronization, secret inputs through
argv/environment/files/pipes, forge APIs and background workers.

**Exit evidence:** Empty/existing repository onboarding, missing identity,
cancelled/wrong unlock, exact host replacement, imported-key non-deletion,
generated-key deletion retry and configuration failures. Publish schemas for
each new verb; prove no prompt consumes JSON/body stdin or persists a secret.

**Amended 2026-10-20 by C2 (decision 9):** wrong unlock proves the current
`transport_unavailable`/exit 5, no pin, no automatic retry and a human hint after
supplying a passphrase. `unlock_failed`/exit 3 is proven with the right passphrase
and a server that rejects the key. Library defect `01M5E6DYJ6GN7DMW4424TH3V78`
owns the future wrong-passphrase code change; C2 does not wait for it. Cancelled
unlock remains `unlock_required`/exit 3, distinct from pre-execute prompt
cancellation (`cancelled`/130).

### C3: CLI Authoring And Discovery Maintenance

**Cycle document:** [C3: CLI Authoring And Discovery Maintenance](../Cycles/wave-03-cli-03-authoring-and-discovery.md)

**Amended 2026-10-24 by C3.** The Cycle, design and plan were approved on
2026-10-24 at `e7b2b71`. All thirteen verbs are implemented on the C3 ticket
branch: document create/save/move/repair, folder create, ticket
create/save/repair/slug-assign, index refresh/rebuild, path observe and ticket
check. Parts A and B have spec and quality approval. The Part B checkpoint
at `9a2a5170c4704ef03420ef2276911c6b1bfa3959` passed six local gates and
**593 tests** (188 CLI unit, 290 CLI integration, 71 read-contract and
44 mutation-contract), as recorded in the
[execution ledger](../plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md).
This is the historical Task 8 checkpoint evidence. **Updated 2026-10-25:** C3
is review-ready through Task 9 Step 5. Whole-branch SPEC/QUALITY approved
`ad4d3f0..806057d3`, dogfooding completed in `8da3890`, and switch SPEC/QUALITY
approved `8da3890..297bd781`. The owner accepted the original **4528 passed /
1 failed / 1 ignored**, exit **101**, plus the authorized correction's **46
distinct passing tests** and check/fmt/clippy as an explicit verification
exception (`5cba600`). No green full-suite run or second full run is claimed.

The product-owner decisions govern delivery:

1. **Known defects, not prerequisites:** `01M4PB82G07RZSQMNJZN672S3W`
   (partial database damage), `01M4PB80T7SVB23TQ6466RDHCB` (repeat committed
   document move, whose existing F2 workaround is outside the lease), and
   `01M4PB7YZMBFAFFM94AZY08HWS` (an interrupted save's step-less row is
   rejected although its file was written). C3 adds no library workaround.
   Index evidence covers deletion and whole-file corruption, removing WAL
   sidecars first, not the partial-damage defect.
2. **CLI-owned save patch:** one fill read when a draft field is omitted,
   preserving the caller's observation. Later retries recompute omissions;
   a changed omitted field can cause `request_mismatch` after a successful
   save, while `request show` still exposes its stored result.
3. **Dogfooding in Task 9, with the owner's go-ahead then:** save dependencies
   C4 → C3 and gate `01M57SKVF6TQ3QM6AXJ35JGGSY`; C5 → C4 and gate;
   D4 → D3; D5 → D4 and gate; D6 → D5; G1 → C5 and D6. The approved
   procedure requires fresh preflight, clean checks, per-request records,
   value checks and separately authorized pushes of those six branches.
4. **One Cycle in two Parts:** A delivers shared CLI changes, both reads,
   index/folder and document create/save/move; B delivers ticket authoring,
   both repairs and the remaining evidence. Each Part has an all-feature
   checkpoint and independent review; the final dogfooding switch is completed
   and separately approved, as recorded below.
5. **Fill-read failures pass through:** under the save command, retain the
   read's code, scope and recovery plus the request ID, with no effects.
   Wrong-kind/unfillable items give item-scoped `item_not_found`. The fill
   read precedes replay, so a failed read can mask a finished result on
   retry; `request show` still returns that result.
6. **Exit evidence:** exit 4 has a reason backed by a library actor leaving
   a save partial and the CLI completing it. Exit 5 is reached through a
   child-held lease, and repair consent cancellation/130 through a Unix PTY.
   The table accounts for exactly 40 library / 42 CLI-inclusive codes.
7. **Accepted fill/full-field difference:** an unregistered-root fill read
   gives `repository_not_registered` with no recovery. A full-field save
   gives `repository_not_enabled` for a never-enabled root, or
   `repository_not_registered` with `index.rebuild` after cache loss. Both
   forms have evidence; the optional read-recovery library ticket was not filed.

**Purpose:** Make local document/ticket lifecycle and explicit discovery repair
available for agents and project dogfooding.

**In scope:** Document/ticket create/save, document move, folder creation,
explicit repair/adoption, index refresh/rebuild and identity recovery, over
the F2 bridges.
Ticket create and save take `deps` and `parent`; add `ticket slug-assign`.
Both relationships accept null: on create it means none; on save it clears,
while omission leaves the relationship unchanged. `ticket check` checks a
complete proposal, with omitted flags proposing none and rejection inside
`ok`/exit 0. `path observe` reports what is at the path in the selected active
context, including another item's file. Mutations have no `--context` in C3;
the multi-item-context ticket remains later work. Index execution requires a
request ID in JSON/noninteractive mode and takes no input. C3 adds no identity
prompt; after configuring identity, a save retry uses a fresh observation
because a refused save may have allocated a context, and can reuse the freed
request ID. Only the two repairs use confirmation.
**Amended 2026-10-20 by C2 (decision 5):** repository-local initials and the
create/enable prefix ship in C2 with their administration verbs, not in C3.
Record the Wave 03 Cycle dependencies in the project's own tickets when these
operations meet this gate. Preserve unknown metadata values, ULIDs and body content;
use actual context provisioning and source/destination observations. Empty folders are
local until they contain tracked content; do not add hidden placeholder
commits. Index-only actions never fetch or rewrite canonical state.

**Out of scope:** Discussion submission, remote synchronization, promotion,
closure, draft storage or GUI editing. Status text `closed` is ordinary metadata.

**Exit evidence:** Offline checkpoints/no-ops, stale writes, invalid/malformed
repair, adoption retry, move collision, index loss/corruption and commit-success/
refresh-failure preserve actual Git/filesystem state. New JSON schemas and
documented command examples pass. Switch ticket create/update dogfooding to the
CLI when those operations meet this gate.

**Runtime qualifications and known limits:** these preserve the approved
Task 0/3 rulings and the library as built.

- Front matter is re-emitted in canonical style when rewritten; a hand-written
  document save can commit normalization without changing values. Both save
  no-ops require intended content already committed (HEAD-equal); equal
  uncommitted ticket values may skip rewriting yet still checkpoint. A
  current-token no-change save can provision a context without adding a commit.
- Design fact 15's owner-approved correction: a primary-only document
  committed at its current path can move to itself as `already_applied`/`noop`,
  exit 0, without creating a context or commit. `Position::of` falls back to
  primary HEAD when the document branch is absent.
- Partial outcomes classify as Incomplete/exit 4 regardless of the code,
  including the repair actor's `internal_error` and the interrupted-save
  retry's `request_mismatch`/`external_change`. An actor has no process exit.
- A pending save fences only requests reaching a new domain operation;
  early binding no-ops/refusals and finished-request replay retain their
  answers. A same-request retry completes while its intended bytes remain.
  After an intervening edit, an omitted-field retry mismatches and a full-field
  retry is stale. Restore the bytes it committed to finish in C3; otherwise
  recovery waits for C4's `operation abandon`.
- `index rebuild` retains the owner-approved root-selection exception: it
  propagates only `repository_not_enabled` from ordinary root resolution,
  then validates the selected Git repository's own workdir. There is no
  universal `not_repository_root` promise for linked roots, and no claim
  that every such rebuild succeeds; this qualification is source-inspected.
- `folder create` is primary-only, with no placeholder commit. A repaired
  document can be listed twice until its context merges. Item reads of
  duplicated/mismatched contexts can give `item_not_found`, which fill inherits.
- Native CI is post-merge and separately authorized. Windows repair consent
  remains a native manual check; Linux evidence does not satisfy it.

**Dogfooding and switch complete, 2026-10-25.** The owner separately authorized
the saves/pushes and the fresh resume after the recorded checker error. All six
saves, value checks and pushes succeeded (`8da3890`); the subsequent AGENTS.md /
file-ticket switch (`297bd781`) has separate SPEC/QUALITY approval. At the
historical Task 8 checkpoint these steps were owed and those files unchanged.
Comments remain canonical until C4; closure remains canonical until C5, with
the explicit working-on-a-ticket exception. The normal main-based CLI workflow
becomes available after C3 merges. C3 is review-ready with its ticket **open**;
closure, PR, merge, cleanup and native CI require their own authorization. The
approved rule still forbids closure/merge while dogfooding evidence is owed
unless amended by the owner; that evidence is now met.

### C4: CLI Discussion, Synchronization And Conflict Recovery

**Cycle document:** [C4: CLI Discussion, Synchronization And Conflict Recovery](../Cycles/wave-03-cli-04-discussion-and-sync.md)

**Amended 2026-10-30 by C4.** The Cycle, design and plan were approved on
2026-10-28 at `8a94e9b`, with named edits E-1 to E-12, after the gate
`01M57SKVF6TQ3QM6AXJ35JGGSY` merged at `a372edb`. The product-owner
decisions, numbered in the Cycle document, change this section as follows:

- **The library part is in C4** (decisions 1 and 2): an exception to track
  rule 5, started after the gate merged. It adds the `comment add`,
  `conflict resolve` and `item republish` bindings, the conflict read,
  merging settings for both synchronization bindings, resume's routes and
  the durable state of an interrupted resolution, with no new result code
  or recovery action.
- **Divergent synchronization merges** (decision 3). A canonical conflict
  is resolved with `conflict show` and `conflict resolve`; any other gets
  outside-repair guidance, then `operation resume`.
- **Republish is its own confirmed verb, `item republish`** (decisions 4,
  7, 11 and 17). It pushes exactly the commit its preview showed and
  integrates nothing, and the next `item sync` integrates primary. A
  lifecycle-closed ticket is `ticket_closed`.
- **D4 waits for all of C4** (decision 5); see track rule 2.
- **A pending remote conflict cannot be abandoned** (decision 6); that
  stays on `01M4H33R34Z7C7EEKTY1ZCT950`.
- **The next step is typed data** (decision 8): a conflict keeps
  `recovery_required` and names `conflict: {operation_id, next_action}`.
- **`comment add` takes an optional `created_by`** (decision 9), validated
  as `Name <email>` and written once; without it, the Git identity.
- **One desktop arm** shows the republish preview (decision 10), merged
  beside D3's edit of the same match.
- **A per-path kind and structure, with set-wide resolution** (decision
  12): a mixed conflict is repaired outside as a whole.
- **The same request finishes an interrupted resolution** with the same
  commit (decision 13), and a lost one has a way out (decisions 18 and 20):
  `operation resume` finishes a prepared commit, and a new request
  supersedes an attempt whose written paths are unchanged.
- **A Ctrl-C handler** stops the six stoppable verbs at their next safe
  point, exit 130 (decision 14).
- **The comment exception is extended** (decision 15):
  `working-on-a-ticket` and `scripts/ticket-loop` keep canonical comment
  files, with the noreply `created_by`, until that skill is revised.
  Everything else switches to `comment add` in C4's final task.
- **The audit timing** (decision 16): see entry-gate item 3.
- **The gate's local verbs stay with C5** (decision 19). C4's
  `conflict show` and `conflict resolve` take remote synchronizations only.
- **C4 fixes main's red registry test** (decision 21) through named edit
  E-12.

**Delivery.** All eight verbs are implemented on the C4 ticket branch:
`comment add`, `item sync`, `repo sync`, `item republish`, `conflict show`,
`conflict resolve`, `operation resume` and `operation abandon`. Part A
(the library) and Part B (the CLI) each passed a checkpoint gate and an
independent review. Two-clone SSH journeys, interruption, lost output and
the exit table follow. The branch merged main after D3, never rebasing.
The [execution ledger](../plans/2026-10-26-wave-03-cli-04-discussion-and-sync-execution.md)
records each task and ruling. Amended 2026-10-30 by C4: C4 is
review-ready.
- The full suite passed on the final code, and the whole-branch review is
  done.
- The dogfooding comment published through `comment add`, after a fix that
  dogfooding found.
- The instruction switch is deferred by the owner (see C4's exit evidence
  below).

Closure, PR, merge and native CI need their own authorization.

**Known limits.** The [design](../plans/2026-10-26-wave-03-cli-04-discussion-and-sync-design.md#known-limits)
lists each:

- A pending remote conflict blocks other synchronizations and authoring in
  its context until it is resolved or repaired, and cannot be abandoned. A
  ticket two clones closed differently makes its conflict external-only,
  with no promise that an outside repair choosing one closure is accepted.
- An unfinished local-context sync anywhere in the repository makes C4's
  verbs that reserve, begin a comment or re-acquire a conflict answer
  `recovery_required`. `conflict show` does not inspect a local sync.
- A cancelled operation is final, and a new request completes the work. A
  second Ctrl-C leaves its request accepted with its reservation, so the
  same request is sent again first. A Ctrl-C after the last safe point
  leaves the run's own answer.
- A refused or blocked synchronization holds its reservation until its
  request is cancelled and sent once more (`01M4R19PC0T6ZPKM39X9DF26HX`).
- A resolution whose written paths were edited by hand is continued by no
  request until the edit is undone.
- The republish push cannot be guarded as a create.
- `created_by` is not authenticated, and a string token is valid only while
  its conflict is unchanged.
- Main's defects: a key selected after a publication remote is configured
  blocks a never-published context's first publication
  (`01M66HFV44YXNQ7ANJVA16TQ1H`), and `remote select` leaves the Git index
  stale (`01M669FJR27FRHCNPWAMD6VH4V`).
- The library defects `01M5E6DYJ6GN7DMW4424TH3V78` (a wrong passphrase),
  `01M5MK57JKCSQFXBNZ8DBE548K` (an unusable identity refused late),
  `01M4W5SCBV6WFJP11Y7PJF5VS8` (operation reads name no item for comment
  rows) and `01M4PB7YZMBFAFFM94AZY08HWS` (an interrupted step-less save)
  touch C4's verbs; C4 adds no workaround.
- Amended 2026-10-30 by C4 (the final review's rulings). A comment's
  publication stopped for an untrusted host offers `request.retry` only,
  not `host.approve` or `host.replace`. Its way out is
  `host observe --repo <root>`, which names the host and the key it
  presents, then `host approve` or `host replace`, then the same
  `comment add` request again.
- Amended 2026-10-30 by C4 (the final review's rulings). Two requests
  resolving one conflict at once can leave an attempt with no owner: a
  later re-entry of the first after a stop is treated as a new request,
  and `conflict show` names no request for it. No data is lost.
- Amended 2026-10-30 by C4 (the final review's rulings). A file system
  refusal after `conflict resolve` has written a path answers
  `internal_error`, exit 4, with no `next:` line; `conflict show` names
  `retry_resolution`, the way on.

**Owed after merge.** The Windows Ctrl-C console handler is first compiled
and checked natively, as a manual check. Native CI runs on main with its
own approval.

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

Amended 2026-10-30 by C4 (the owner's decision): C4's dogfooding comment
published through `comment add`, but the instruction switch is deferred.
With a passphrase-protected key, an agent's non-interactive publication
stops at `unlock_required` and holds the remote reservation, so agents keep
canonical comment files for now.

### C5: CLI Completion And One-Shot Polling

**Planned document:** `docs/Cycles/wave-03-cli-05-completion-and-polling.md`

**Amended 2026-10-30 by C4 (decision 19).** C5 exposes the gate's
local-sync verbs, `context sync-local` and `context resolve-local`, with
ticket closure, and any CLI inspection of a local-sync conflict they need.
C4's `conflict show` and `conflict resolve` take remote synchronizations
only, and answer a local sync's operation ID with `invalid_input`.

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

**Cycle document:** [D3: Rich-Text And Source Authoring](../Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md)

**Amended 2026-10-24 and 2026-10-26 by D3.** The Cycle, design and plan were
approved on 2026-10-26 at `c3904c7`. The product-owner decisions, numbered
in the Cycle document, change this section as follows:

- **Shape** (decisions 5 and 13): one Cycle in six Parts, 0 and A–E, each
  with a checkpoint review. This departs from the risk row's "split before
  its plan is approved"; the owner may split at a checkpoint.
- **Part 0 inside D3** (decisions 2 and 11): the library gains two F2
  commands, `context prepare` and `context release`, reviewed on their own
  before any desktop Part. This departs from track rule 5. `src/drafts/`
  also gains one target state for a never-created item (ruling R-20).
- **The item-view design** (decision 12): the tab strip, a fixed header, a
  white page, the inline display of comments, front matter in a details
  sheet and an editor toolbar, from `docs/design/item_view/`.
- **The read view** is the editor, read-only, so text can be copied
  (decision 17).
- **Unsupported constructs** are shown literally and edited in place, not as
  labeled source fallback (decision 8; desktop RFC amendment).
- **Safe local resource rendering** means local raster images inside the
  repository and link resolution between documents (decisions 9 and 10).
- **Untouched-byte evidence** keeps front matter by value: the library
  re-emits it on every save.
- **Context release** applies only to contexts the desktop process created
  (decision 18).

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

**Amended 2026-10-26 by D3 (decision 12).** D3 builds the read-only,
ordered display of comment threads inline below the item, with read-only
comment editors, and disabled Share, Close ticket… and Approve and merge…
actions in the item header. D4 keeps the composers, posting,
synchronization and its exit evidence, thread ordering after posting and
rebuild included. D4 may reuse D3's `context prepare` and `context release`
for a comment's context, and the item-view handoff's composer design.

**Amended 2026-10-30 by C4 (decision 5).** D4 waits until all of C4 is
merged. Its Post and Sync consume C4's library bindings: `comment add`, with
its stops at safe points and its retry that publishes without writing again,
the merging `item sync` and `repo sync`, and the conflict data their results
carry. See track rule 2.

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

**Amended 2026-10-30 by C4 (decisions 5 and 19).** D5 consumes C4's
conflict read (`show_conflict`) and `conflict resolve` binding for remote
synchronizations, through D4. With C5, it owns the gate's local-sync
boundary.

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
| Comment author (`created_by`) | F1 (reads), F2 (writes); C4 (the caller's value) | Written once at comment creation from the confirmed Git identity; never derived from Git history. Amended 2026-10-30 by C4 (decision 9): `comment add` may supply it instead, validated as `Name <email>`, and it is then written verbatim, once. |
| Missing identity during create/enable and public identity configuration | F2 | Explicit confirmed local persistence, including a not-yet-created root. |
| Configured-host approval | F2 | Scoped transport verification; no publication or remote-ref refresh. |
| Folder creation and marker-only adoption/repair | F2 | Scoped filesystem boundary, stable IDs, expected observations, no hidden migration. |
| Command grammar, envelopes, exit classes, terminal secret provider | C1, C2 | CLI-only; no domain logic and no secret through argv/environment/files/pipes. |
| Discussion/sync/conflict and resume | C4, D4, D5 | Wave 02 operations and owned canonical paths only. Amended 2026-10-30 by C4 (decision 1): C4 adds the bindings, `comment add`, `conflict resolve` and `item republish`, the merging settings of `item sync` and `repo sync`, and resume's routes; D4 and D5 consume them. |
| Conflict inspection read | Local boundary: closure-recovery gate; remote boundary: C4 (amended 2026-10-30 by C4, decision 19) | Local observation/inspection/immutable-side DTOs are implemented and code-reviewed on the review-ready gate branch, awaiting authorized lifecycle delivery and merge. Remote: C4's `show_conflict` read and `conflict resolve` binding, through the CLI's `conflict show` and `conflict resolve`; D5 consumes them through D4. Front ends consume the shared boundaries and F1 redaction rules. |
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
