---
manyhands_managed: true
manyhands_kind: comment
id: "01M6KZ2PM15ZE3ZSMMVS76YDSB"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
created_at: "2026-11-04T19:02:54Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Brief for the implementing agent

**Why it matters:** anyone with more than one clone hits this. After a ticket is closed (or a document promoted) on one clone, its remote branch is deleted. Every other clone's `poll once` keeps the old local branch and worktree, reports `remotely_deleted`, and reads on that clone can show the closed ticket as open and its dependents as blocked. Nothing offers a way out except manual Git.

### Where to work
- Ticket `01M6KHVD6D0DB8KYT6VGQP10Y0`; branch `manyhands/ticket/01M6KHVD6D0DB8KYT6VGQP10Y0`; worktree `.manyhands/worktrees/01M6KHVD6D0DB8KYT6VGQP10Y0` in the registered main checkout `/home/vhodges/work/src/manyhands`.
- Follow current `AGENTS.md`: Devenv for all Cargo; merge `origin/main` into the branch, never rebase; the tiered test gate (static gates, fast tier, SSH tier by changed path; `src/repository/remote/` means `ssh_transport`, `remote_observation`, `remote_synchronization`, `remote_merge_recovery`, `remote_polling`, `cli_remote`; `mutation/` means `mutation_remote`).
- This changes recovery and cleanup semantics in the shared library, so plan first with the `planning-a-cycle` skill: design and plan, independent review, and the owner's approval before code. Close at the end with `ticket close` after review approval, setting the resolution status first (`file-ticket`).

### What exists (read these first)
- **Poll classification:** `src/repository/remote/poll.rs`, `classify_local_target` (~:201). `PollTrackingState::RemotelyDeleted` returns `PollDisposition::RemotelyDeleted` at ~:213, before any local inspection. Dispositions and guidance are at ~:21-90. Decision 28 (C5) added `primary_holds` for materialization only.
- **Context release:** `src/repository/context_release.rs`, `release_context` (~:241), `release_under_lease` (~:351). Its module doc is the contract. It removes a context that "holds nothing of anyone's", under one lease, with recoverable steps (`ContextWorktreeReleased`, `ContextBranchReleased`) and an index hand-off. Today it **refuses a closed ticket** (`TicketClosed`, step 3) and **refuses any context with publication evidence** (step 4c, R-43). Both block this case, by design.
- **Bindings:** `Command::ContextRelease` ("context release") exists at the mutation boundary (`src/repository/mutation/command.rs`) and is used by the desktop (`src/desktop/services/created_contexts.rs`, D3's created-contexts flow). It is deliberately **not** a CLI verb: an exempt entry in `tests/cli/inventory.rs`.
- **Reads:** `src/repository/read/items.rs`, `effective_rows` (~:1567). An item's own worktree row wins, so the stale open copy is what reads see.
- **Closure on primary:** lifecycle closure is `closed_at` and `closed_by` in primary's file, not status text, and not whether the branch tip is contained in primary.

### The cases (real data, 2026-11-04)
On the owner's second machine: C5 `01M4CC0VMXTNXNH78C11DYXS8H` and D4 `01M4CC0VN1KTQFSVR99378TDHV` are closed on main, with local tips contained in main. `01M5MK57JKCSQFXBNZ8DBE548K` and `01M669FJR27FRHCNPWAMD6VH4V` are closed on main, with local tips **not** contained in main (C5 copied their ticket files rather than merging their branches). All four local copies are open copies.

### What to build
1. **Recognition in the poll.** For a remotely deleted target whose item is lifecycle-closed on primary (ticket) or promoted (document), report a new disposition, e.g. `closed`, with guidance that it is safe to retire. Keep `remotely_deleted` for an item still open on primary: that is a real deletion with possible work to keep. This changes the poll's report, schema enum, goldens and `PollTargetDisposition`.
2. **A deliberate retire verb**, CLI and library binding (the desktop consumes the binding later). Probably extend `release_context` with a closed-item mode rather than a new mechanism. It removes the local worktree and branch, then refreshes the index. Explicit only: Wave 03 excludes automatic cleanup.
3. **Reads**, optionally: until retire runs, stop the stale copy masking primary's closure. That overlaps `01M6K04YQM2JF6SBGPB1ZT6R8T` (reads by the latest copy); decide in planning whether to touch reads here.

### Questions for the owner (ask during planning; don't decide alone)
- **What counts as safe to discard:** a clean worktree, plus every commit on the local branch either contained in primary or superseded by primary's closure? The folded tickets' branches hold commits not in primary (the old open ticket file). Is "closed on primary" enough, or must each commit be shown already published?
- **A CLI verb:** add `context release` itself, with a closed-item mode, or a new verb such as `context retire`?
- **Is `closed` the right disposition name,** and do promoted documents get the same treatment?
- **Should the desktop offer retire in D6,** or later?

### Acceptance
Two-clone SSH rows, through the binary:
- clone A closes a ticket and its branch is deleted; clone B polls, sees the new disposition, retires, and then reads report closed;
- a dirty, or unpublished-ahead, local copy is refused with nothing removed;
- a folded-style branch whose tip isn't in main;
- a promoted document;
- an item still open on primary keeps `remotely_deleted` and isn't retirable this way.
Library rows for each release step's re-entry. Don't fit an expectation to an answer: a row that answers otherwise stops for the owner.

### Gated
Pushing branches other than this one, merging, closing, native CI and deleting anything outside this ticket all need the owner's separate go-ahead.
