---
title: "Wave 03 RFC Review and Decision Register"
date: 2026-10-05
status: draft
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46S07YFKEMG0WX39DQX7C1G"
---

# Wave 03 RFC Review and Decision Register

## Purpose and planning baseline

This is the review index for Wave 03's proposed RFCs, not a Wave document,
Cycle plan or implementation approval. The product owner requested early RFC
authoring while another session implements Wave 02 Cycle 03. This checkout
does not modify that session's ticket, branch, worktree or approved source
documents. No Wave 03 Cycle tickets or implementation plans are created yet.

Draft base: local `main` at `a21a31aeea5aaf7c03cf7692848ddadb5dde1242`,
which includes fetched `origin/main` at
`89ce24d0c5b99c817ec81615fe610f65d7c81a99`. The new branch
`planning/wave-03-rfcs` was created and rebased onto local `main` before edits,
in `.worktrees/wave-03-rfcs`. Wave 02 remains in progress; its future APIs must
be checked at the Wave 03 entry gate. These drafts describe contracts, not
completed implementation or verification.

## Proposed RFC set

| Document | Decision boundary |
| --- | --- |
| [Desktop information architecture and editor](desktop-information-architecture-and-editor.md) | Navigation, rich-text/source fidelity, drafts, keyboard interactions, consent and conflict presentation. |
| [CLI contract](cli-contract.md) | Command/input grammar, JSON v1, exits, consent, retry identity and headless workflows. |
| [Application runtime and polling](application-runtime-and-polling.md) | Shared operation adapters, competing schedulers, process-local credentials and daemon shutdown. |

The architecture explicitly requires the first two. The third factors out
decisions that would otherwise be duplicated and potentially contradictory.
The existing [test strategy](test-and-compatibility-strategy.md) remains the
test authority; the evidence proposal below extends it, rather than creating
a competing test RFC. All new files remain `draft` pending review.

## Decisions, gaps and blockers

| ID | Finding and proposed resolution | State / owner / gate |
| --- | --- | --- |
| W3-01 | Rich-text editing with Markdown source mode; preserve unsupported constructs and untouched source. Carry forward the charter's minor preference for `zorite-editor`, with extraction from Velotype as the alternative. | Product-owner mode direction recorded 2026-10-05; candidate preference comes from the charter. Technical Lead must evaluate pinned candidates for a single compatible GPUI graph, fidelity, host integration and native input before editor Cycle planning. No library selection is approved yet. |
| W3-02 | Daemon unlock occurs in its own terminal session; restart to unlock again. No desktop credential IPC or secret pipe input. | Product-owner direction recorded 2026-10-05; verify terminal/nonterminal behavior before runtime Cycle exit. |
| W3-03 | A clean worktree can have unsaved in-memory drafts. Propose protected crash-recovery files, base observations and explicit stale-edit review. | Proposed; Product Owner/Technical Lead approval required for persistence scope and crash guarantee before editor planning. |
| W3-04 | Ticket branches can contain code; resolve canonical Markdown in-app and provide explicit external-tool guidance for other conflicts. | Product-owner direction recorded 2026-10-05. Keep the owned-path boundary and prove safe re-observation after external repair. |
| W3-05 | Multiple schedulers need atomic due-state recheck plus Wave 02 reservation claim, separate from short local leases. | Proposed; Technical Lead approval and API audit before runtime planning. No replacement of Wave 02 transport protocol. |
| W3-06 | Session unlock blocking must not become a persisted user pause or prevent another unlocked process from polling. | Proposed; authentication/index refinements required before runtime planning. |
| W3-07 | Libgit2 cancellation callbacks do not themselves prove bounded shutdown in every transport phase. Propose ten-second graceful drain budget with visible incomplete recovery. | Feasibility gate; Technical Lead must demonstrate native behavior or approve a cancellation design before runtime implementation. |
| W3-08 | CLI retries need durable request identity and stable comment IDs; current domain requests do not provide the complete external protocol. | Proposed; CLI/Git/persistence contract review before CLI mutation planning. Cache loss can require explicit recovery. |
| W3-09 | Repair/adoption, folder creation, identity configuration, key public export and confirmation previews need an API audit. UI controls cannot manufacture safe missing domain behavior. | Technical Lead must map to existing operations or propose narrowly scoped additions in Wave 03 Cycles. |
| W3-10 | Build CI is not proof of keyboard, IME, rich-text fidelity or end-to-end desktop use. | Proposed native evidence matrix below; Release Owner must arrange machines/runners before final Wave planning. |
| W3-11 | A ticket close integrates the full context branch and can remove a worktree containing non-item files. | Effect preview must enumerate paths and preserve dirty/unexpected work; reconcile with Wave 02 closure preflight before close UI planning. |
| W3-12 | Short-ID research is not an approved replacement for canonical ULIDs. | Keep full IDs for mutation and copy controls. Aliases need a separate collision/ambiguity contract if requested. |

W3-01, W3-02 and W3-04 record product choices, not approval of the whole RFC set.
Unanswered proposals remain proposals; elapsed review time is not approval.

## Required source amendments on approval

This branch leaves approved RFCs unchanged while Wave 02 is active. Before
Wave 03 implementation, approval must adopt the following exact decisions in
their owning documents and resolve any intervening Wave 02 changes:

| Approved source | Proposed amendment |
| --- | --- |
| [Architecture](mvp-rfc.md) | Add shared runtime as the desktop/CLI scheduling authority; add it to the Wave 03 entry dependencies. Preserve every existing consent and single-context rule. |
| [Repository/index](repository-index-persistence-and-refresh.md) | Add atomic due-and-reserve, separate user pause from session readiness, non-secret runtime session observations, and explicit ownership of draft files outside the rebuildable cache. Define CLI request/confirmation observation records and cache-loss recovery without storing bodies/secrets. |
| [Authentication](authentication-and-credential-handling.md) | Define process session boundaries and separate unlock suspension from user pause. Specify terminal-only CLI secret input, daemon restart unlock, and absence of cross-process secret transfer. |
| [Git workflow](git-workflow-and-conflict-recovery.md) | Bind external confirmation/request identity to observed effects and resume only unchanged remaining work. Record W3-04's explicit external-repair/re-observation path without expanding the owned canonical write boundary. |
| [Canonical schema](canonical-content-and-comment-schema.md) | Clarify that display/repair never changes canonical identity implicitly, unknown metadata values survive edits, and unsupported rich-text constructs preserve source. No schema or ID migration is proposed. |
| [Test strategy](test-and-compatibility-strategy.md) | Adopt the Wave 03 matrix and acceptance obligations in this register and the three draft RFCs. Preserve all Wave 01/02 evidence. |

No PRD amendment is proposed for the selected editor or terminal-session model.
If decisions weaken a PRD journey, change supported platforms, introduce ticket
reopening or require non-developers to use Git for canonical conflict recovery,
explicit PRD revision is required rather than a hidden limitation.

## Proposed Wave 03 evidence gate

All six PRD journeys must run through **both** front ends with real local
repositories and the authenticated SSH fixture: offline checkpoint/later sync,
concurrent conflict recovery, background discovery, durable threaded discussion,
confirmed promotion, and confirmed closure with retryable failure. Assert
canonical bytes, commit/ref ancestry, local/remote context presence and operation
records in addition to visible results. Local-only promotion/closure and later
primary sync are separate required variants.

| Coverage | Minimum evidence |
| --- | --- |
| Existing build target matrix | Preserve Linux x86_64/aarch64, Windows x86_64/aarch64 and macOS aarch64 builds. This is the current repository matrix, not a new architecture support promise. |
| Native automated headless tests | Domain, CLI JSON/exits, real SSH, daemon and cross-process coordination on all five existing native targets. Missing fixture support is a tracked blocker, not a skipped passing test. |
| Native desktop journeys | Full six journeys on at least one Windows, macOS and Linux Wayland machine, identifying OS version, architecture, compositor where relevant and tested commit. |
| Additional built architectures | At minimum launch, edit/save, rich-text/source round-trip, keyboard navigation, credential prompt and shutdown smoke evidence for each remaining built target. |
| Keyboard and input | Entire `MH-NFR-005` workflow on all three OS families; visible focus, dialogs, IME, Unicode, high-DPI and unsupported-source handling. Record screen-reader results separately without claiming untested compliance. |
| Fault and restart | Kill/restart around draft flush, canonical write, checkpoint, refresh, fetch, merge, push and cleanup; reconcile actual state and demonstrate no duplicated content/effects. |
| Scheduler and credentials | Two processes, aliases, pause versus unlock, manual priority, transport stalls, key/remote change, suspend/clock jump and graceful/forced stop. |
| Trusted collaborators | At least two people using separate clones complete a shared-item edit/conflict/discussion journey; record observed usability problems and retest fixes. |

Proposed responsiveness evidence uses delayed file/network operations and a
repeatable fixture of 1,000 items plus a 100 KiB mixed-syntax document. The
measurement target is p95 local input response below 100 ms and cancellation
feedback within one second on a recorded reference machine; network completion
is not subject to those timings. These are acceptance targets requiring approval
and measurement, not current performance claims or hard document-size limits.
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

## Suggested dependency order after review

First resolve product scope and editor/runtime feasibility, then approve the
three RFCs and corresponding source amendments. Refresh this branch against
main after Wave 02 settles and audit actual APIs, transport and fixture evidence.
Then author the Wave 03 document with its ordered Cycles, entry/exit gates and
one Manyhands ticket per Cycle before each implementation plan. Do not infer
that early RFC authoring authorizes implementation or makes Wave 02 complete.

Runtime/adapter contracts should precede front-end integration; independent
desktop layout and read-only CLI work can be planned once their contracts are
approved. Editor fidelity and recovery must be proved before polishing rich
interactions. End-to-end/native evidence should accumulate throughout the Wave
rather than wait for its final Cycle. Exact Cycle allocation is intentionally
left to subsequent Wave planning after these risks are resolved.
