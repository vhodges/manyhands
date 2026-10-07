---
title: "Wave 03 RFC Review and Decision Register"
date: 2026-10-05
status: approved
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46S07YFKEMG0WX39DQX7C1G"
---

# Wave 03 RFC Review and Decision Register

## Purpose and planning baseline

This records the product owner's approval of the three Wave 03 RFCs on
2026-10-05, including their decisions, source amendments and evidence gates.
The reviewed branch tip was `f560295`. It is not a Wave document, Cycle plan
or implementation approval. The product owner requested early RFC
authoring while another session implements Wave 02 Cycle 03. This checkout
does not modify that session's ticket, branch or worktree. This planning branch
includes the product-owner-directed PRD 0.5 scope amendment and matching source
updates removing CLI resident polling. No Wave 03 Cycle tickets or
implementation plans are created yet.

Draft base: local `main` at `a21a31aeea5aaf7c03cf7692848ddadb5dde1242`,
which includes fetched `origin/main` at
`89ce24d0c5b99c817ec81615fe610f65d7c81a99`. The new branch
`planning/wave-03-rfcs` was created and rebased onto local `main` before edits,
in `.worktrees/wave-03-rfcs`. Wave 02 remains in progress; its future APIs must
be checked at the Wave 03 entry gate. These approved RFCs describe contracts, not
completed implementation or verification.

## Approved RFC set

| Document | Decision boundary |
| --- | --- |
| [Desktop information architecture and editor](desktop-information-architecture-and-editor.md) | Navigation, rich-text/source fidelity, drafts, keyboard interactions, consent and conflict presentation. |
| [CLI contract](cli-contract.md) | Command/input grammar, JSON v1, exits, consent, retry identity and headless workflows. |
| [Application runtime and polling](application-runtime-and-polling.md) | Shared operation adapters, desktop-owned background worker, process-local credentials and safe shutdown. |

The architecture explicitly requires the first two. The third factors out
decisions that would otherwise be duplicated and potentially contradictory.
The existing [test strategy](test-and-compatibility-strategy.md) remains the
test authority; it adopts the evidence gate below. The three RFCs and this
approval register are `approved`. Approval establishes design authority; it
does not claim completed feasibility work, authorize implementation, or authorize
pushing or merging the planning branch.

## Decisions, gaps and blockers

| ID | Decision or remaining risk | State / owner / gate |
| --- | --- | --- |
| W3-01 | Rich-text editing with Markdown source mode; preserve unsupported constructs and untouched source. Carry forward the charter's minor preference for `zorite-editor`, with extraction from Velotype as the alternative. | Product-owner mode direction recorded 2026-10-05; candidate preference comes from the charter. Technical Lead must evaluate pinned candidates for a single compatible GPUI graph, fidelity, host integration and native input before editor Cycle planning. The product owner selected `zorite-editor` on 2026-10-07; on the same date the product owner accepted the byte changes of its recorded load normalization, and the desktop/editor and canonical RFCs are amended to bound that exception. |
| W3-02 | Background polling/indexing runs in a desktop-owned worker. CLI daemon mode is removed; explicit one-shot poll/refresh remains. No separate executable, shared singleton or IPC. | Product-owner clarification adopted in PRD 0.5 on 2026-10-05; supersedes the earlier daemon unlock/restart choice. |
| W3-03 | A clean worktree can have unsaved in-memory drafts. Use protected crash-recovery files, base observations and explicit stale-edit review. | Approved with the RFC set on 2026-10-05; Technical Lead must verify persistence failures and the stated crash guarantee during editor work. |
| W3-04 | Ticket branches can contain code; resolve canonical Markdown in-app and provide explicit external-tool guidance for other conflicts. | Product-owner direction recorded 2026-10-05. Keep the owned-path boundary and prove safe re-observation after external repair. |
| W3-05 | Concurrent resident-poller coordination is not required. Remove scheduler election, atomic due-slot claiming and process heartbeat records; retain existing Wave 02 leases/reservations for actual operations. | Resolved by the W3-02 scope clarification. No new scheduler-coordination protocol or test matrix. |
| W3-06 | Desktop unlock blocking must remain distinct from persisted user pause. Its worker shares the desktop credential session; CLI credentials last one invocation. | Approved with the RFC set; verify during runtime implementation. No cross-process readiness or unlock coordination. |
| W3-07 | Ten seconds is a shutdown feedback threshold, not a process-exit deadline. Show still stopping, preserve recovery state/live ownership and wait for the operation to stop safely. | Product-owner clarification during Wave planning, 2026-10-05. Technical Lead must characterize native cancellation and responsive stopping before worker implementation; callbacks alone are not evidence. |
| W3-08 | CLI retries need durable request identity and stable comment IDs; current domain requests do not provide the complete external protocol. | Approved contract; audit actual domain APIs before CLI mutation planning and verify replay/cache-loss recovery during implementation. |
| W3-09 | Repair/adoption, folder creation, identity configuration, key public export and confirmation previews need an API audit. UI controls cannot manufacture safe missing domain behavior. | Technical Lead must map to existing operations or propose narrowly scoped additions in Wave 03 Cycles. |
| W3-10 | Build CI is not proof of keyboard, IME, rich-text fidelity or end-to-end desktop use. | Product owner selected Linux-first work and confirmed early beta testers have Windows and Mac machines on 2026-10-05. Release Owner tracks specific assignments, architecture coverage and scheduling; the complete native evidence matrix remains a Wave exit requirement. |
| W3-11 | A ticket close integrates the full context branch and can remove a worktree containing non-item files. | Effect preview must enumerate paths and preserve dirty/unexpected work; reconcile with Wave 02 closure preflight before close UI planning. |
| W3-12 | Short-ID research is not an approved replacement for canonical ULIDs. | Keep full IDs for mutation and copy controls. Aliases need a separate collision/ambiguity contract if requested. |

Approval resolves the design decisions above. Editor dependency selection,
transport shutdown feasibility, the Wave 02 API audit and native evidence
remain outstanding work; approval does not mark those gates satisfied.

The [approved Wave 03](../Waves/wave-03-dogfooding.md) assigns these obligations
to ordered Cycles. Wave planning also corrected desktop/CLI closure wording to
match the canonical schema: free-form status, including `closed`, never replaces
the explicit lifecycle action and its `closed_at`/`closed_by` fields. The product
owner approved the Wave at revision `4ea2aa9` on 2026-10-05, including optional
create/enable identity input and closure-list filters. Those refinements are now
adopted in the CLI and desktop RFCs. Wave approval does not establish completed
feasibility work, native evidence or Cycle implementation authorization.

## Adopted source amendments

The PRD 0.5 scope amendment and the following source amendments are adopted on
this branch under the product owner's approval. They define Wave 03 obligations
and do not imply changes to Wave 02 implementation. Reconcile intervening
Wave 02 changes before Wave 03 planning/implementation.

| Approved source | Adopted amendment |
| --- | --- |
| [Architecture](mvp-rfc.md) | Add shared runtime as the operation-adapter and desktop-worker authority; add it to the Wave 03 entry dependencies. Preserve every existing consent and single-context rule. |
| [Repository/index](repository-index-persistence-and-refresh.md) | Separate user pause from desktop unlock suspension and define ownership of draft files outside the rebuildable cache. Define CLI request/confirmation observation records and cache-loss recovery without storing bodies/secrets. No resident-poller coordination schema is needed. |
| [Authentication](authentication-and-credential-handling.md) | Define desktop/worker session sharing and separate unlock suspension from user pause. Specify terminal-only one-shot CLI secret input and absence of cross-process secret transfer. |
| [Git workflow](git-workflow-and-conflict-recovery.md) | Bind external confirmation/request identity to observed effects and resume only unchanged remaining work. Record W3-04's explicit external-repair/re-observation path without expanding the owned canonical write boundary. |
| [Canonical schema](canonical-content-and-comment-schema.md) | Clarify that display/repair never changes canonical identity implicitly, unknown metadata values survive edits, and unsupported rich-text constructs preserve source. No schema or ID migration is proposed. |
| [Test strategy](test-and-compatibility-strategy.md) | Adopt the Wave 03 matrix and acceptance obligations in this register and the three approved RFCs. Preserve all Wave 01/02 evidence. |

PRD 0.5 records the selected desktop-only background execution model. No PRD
amendment is needed for the selected editor. If further decisions weaken a PRD
journey, change supported platforms, introduce ticket
reopening or require non-developers to use Git for canonical conflict recovery,
explicit PRD revision is required rather than a hidden limitation.

## Approved Wave 03 evidence gate

All six PRD journeys require desktop evidence, with corresponding explicit
operations exercised through the CLI using real local
repositories and the authenticated SSH fixture: offline checkpoint/later sync,
concurrent conflict recovery, background discovery, durable threaded discussion,
confirmed promotion, and confirmed closure with retryable failure. Assert
canonical bytes, commit/ref ancestry, local/remote context presence and operation
records in addition to visible results. Local-only promotion/closure and later
primary sync are separate required variants. For background discovery, the
desktop proves automatic scheduling; CLI coverage uses explicit `poll once`
and verifies the same domain effects without claiming resident scheduling.

| Coverage | Minimum evidence |
| --- | --- |
| Existing build target matrix | Preserve Linux x86_64/aarch64, Windows x86_64/aarch64 and macOS aarch64 builds. This is the current repository matrix, not a new architecture support promise. |
| Native automated headless tests | Domain, CLI JSON/exits, real SSH, one-shot polling/indexing and existing repository-operation coordination on all five native targets. Missing fixture support is a tracked blocker, not a skipped passing test. |
| Native desktop journeys | Full six journeys on at least one Windows, macOS and Linux Wayland machine, identifying OS version, architecture, compositor where relevant and tested commit. |
| Additional built architectures | At minimum launch, edit/save, rich-text/source round-trip, keyboard navigation, credential prompt and shutdown smoke evidence for each remaining built target. |
| Keyboard and input | Entire `MH-NFR-005` workflow on all three OS families; visible focus, dialogs, IME, Unicode, high-DPI and unsupported-source handling. Record screen-reader results separately without claiming untested compliance. |
| Fault and restart | Kill/restart around draft flush, canonical write, checkpoint, refresh, fetch, merge, push and cleanup; reconcile actual state and demonstrate no duplicated content/effects. |
| Desktop worker and credentials | One worker shared across windows, pause versus unlock, manual priority, transport stalls, key/remote change, suspend/clock jump and graceful/forced stop. Retain domain tests for explicit CLI overlap; omit concurrent resident-poller orchestration. |
| Trusted collaborators | At least two people using separate clones complete a shared-item edit/conflict/discussion journey; record observed usability problems and retest fixes. |

Required responsiveness evidence uses delayed file/network operations and a
repeatable fixture of 1,000 items plus a 100 KiB mixed-syntax document. The
measurement target is p95 local input response below 100 ms and cancellation
feedback within one second on a recorded reference machine; network completion
is not subject to those timings. These are approved acceptance targets requiring
measurement, not current performance claims or hard document-size limits.
Larger/unsupported content remains preserved with an explicit usable source
fallback; no silent truncation is allowed.

Evidence records must include tested SHA/lockfile, OS/architecture/display,
fixture configuration, commands or manual steps, expected/observed result,
redacted artifact links, reviewer and unresolved deviations. Compile success,
mocked screenshots and unit tests do not substitute for a native journey.

Local Rust changes use the four `AGENTS.md` Devenv checks plus relevant CLI and
desktop smoke tests; desktop requires an active display. Native CI may run
Cargo directly under the existing approved exception. Documentation-only RFC
authoring validates links, front matter, examples, consistency and Git diff;
it does not claim runtime tests or native journey evidence.

## Next planning steps

Follow the [approved Wave 03](../Waves/wave-03-dogfooding.md) readiness deadlines.
Refresh against main
after Wave 02 settles and audit actual APIs, transport and fixture evidence.
Resolve editor/runtime feasibility before the affected Cycles and create one
Manyhands ticket per Cycle before each implementation plan. Early RFC/Wave
authoring does not authorize implementation or make Wave 02 complete.

Runtime/adapter contracts should precede front-end integration; independent
desktop layout and read-only CLI work now have approved RFC contracts.
Individual Cycles still require planning and authorization. Editor fidelity
and recovery must be proved before polishing rich interactions.
End-to-end/native evidence should accumulate throughout the Wave
rather than wait for its final Cycle. The Wave's original thirteen-Cycle
allocation was approved on 2026-10-05 and replanned on 2026-10-07 into a
two-Cycle foundation, parallel CLI and desktop tracks and one joint gate; see
[Tracks And Cycles](../Waves/wave-03-dogfooding.md#tracks-and-cycles). Each
Cycle still requires its own detailed planning and review. The `W3-NN`
identifiers in this register name decisions, not Cycles, and are unaffected.
