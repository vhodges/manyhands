---
title: "Wave 03 D1: Desktop Shell, Onboarding And Discovery"
date: 2026-10-18
status: proposed
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M58Y0GYPEDXT99ZC797CSKPG"
---

# Wave 03 D1: Desktop Shell, Onboarding And Discovery

## Purpose And Authority

Replace the hello-world window with a keyboard-usable desktop over the
foundation services. A user can choose a registered repository, browse its
documents and tickets, open several items in tabs, see where each item's
effective copy comes from, follow ticket relationships, and administer
repositories, remotes, identity, shared SSH keys and host trust. Every read
goes through the F1 read services and every change through the F2 mutation
boundary. Nothing in this Cycle edits an item body, posts a comment,
synchronizes, promotes, closes or polls.

This is Cycle D1 of
[Wave 03](../Waves/wave-03-dogfooding.md#d1-desktop-shell-onboarding-and-discovery),
tracked by ticket `01M4CC0VMY5RBMP70QVBXWXM6A`. The
[desktop information architecture and editor](../RFC/desktop-information-architecture-and-editor.md),
[application runtime and polling](../RFC/application-runtime-and-polling.md),
[authentication](../RFC/authentication-and-credential-handling.md),
[ticket relationships and short codes](../RFC/ticket-relationships-and-short-codes.md),
[repository/index](../RFC/repository-index-persistence-and-refresh.md) and
[test strategy](../RFC/test-and-compatibility-strategy.md) RFCs are
authoritative. The
[design](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md)
and
[implementation plan](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-implementation.md)
accompany this document. None of the three is approved yet, and approving them
does not authorize implementation.

## Entry Evidence And Dependency Boundary

- Reused the existing ticket worktree
  `.manyhands/worktrees/01M4CC0VMY5RBMP70QVBXWXM6A` and branch
  `manyhands/ticket/01M4CC0VMY5RBMP70QVBXWXM6A`; no replacement was created.
  The branch had never been pushed.
- On 2026-10-18 a fresh `git fetch origin` observed `origin/main` at
  `f6a4c06300f4be80eb18f843a9e4fcbde847e47e`. The ticket head, two
  checkpoint commits, rebased from
  `697ba001325795c408150a686b3c9edc4d70a262` to
  `94feb28a24f8e0fbe871181c3bb238a5ef2b1a9a` without conflicts. The fetched
  base is an ancestor of the result and the worktree was clean. The main
  checkout's untracked `.superpowers/` and every other worktree were left
  untouched. That rebase followed the `AGENTS.md` read before it; the
  `AGENTS.md` it brought in says to **merge** `origin/main` into ticket
  branches and never rebase them. Because the branch had never been pushed,
  no published history was rewritten. From here on this Cycle merges.
- **Wave 02 dependency.** The Wave requires Cycles 01–04 on main for D1. All
  ten Wave 02 Cycles are closed and on main.
- **Wave 03 dependency.** F1 (`01M4CC0VMQ7R15A7M9SPN3KB67`) and F2
  (`01M4CC0VMR8HSPZXQ1WX41GWVK`) are closed and merged.
- **Entry-gate item 3, the API audit.** The design's
  [library surface audit](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md#library-surface-audit-at-f6a4c06)
  refreshes the provisional audit's D01–D06, D28, D30 and R07 rows at
  `f6a4c06`, by reading source. No Rust command was run for planning.
- **Entry-gate item 7.** The required checks at the implementation baseline
  are the plan's first step and have not been run.
- **Entry-gate item 6, native testers.** D1 is planned and evidenced on Linux
  (decision 3). Windows and macOS tester assignments are not made by this
  Cycle and remain the Release Owner's to confirm before G1.
- **Prerequisite shared-library change (decision 2).** Defect ticket
  `01M4R19P943QZKHHKRXV2D2TP2`, "A registration whose directory is gone
  cannot be removed", must be fixed, reviewed and merged to main as a
  shared-library change under track rule 5 before Part C begins. It is open
  and unstarted at this baseline. Parts A and B do not depend on it.
- **Parallel work.** C1 (`01M4CC0VMSZ7BRS740C8VSQ5ZK`) is the only other
  Wave 03 Cycle based on current main. The two tracks may both touch
  `Cargo.toml`, `Cargo.lock` and `.github/workflows/build.yml`. Whichever
  lands second merges main and resolves those files; neither changes the
  other's sources. Track rule 5's "rebase onto it" is read as "merge it",
  which `AGENTS.md` requires.

Before implementation, fetch and merge then-current main, repeat the
ancestry check, and re-check the audit against any library change landed in
between. Check `git merge-tree --write-tree HEAD origin/main` before calling
the work review-ready.

## Scope

- **Shell.** `main.rs` initializes the transport before any thread, then
  starts the desktop. `gpui_kit::init` runs inside `app.run` and `Root` is the
  first view of every window (today's scaffold creates its child first; this
  Cycle fixes it). The window shows a repository selector, a navigation pane
  with Documents and Tickets, an open-item tab strip, the selected item, a
  settings surface and a persistent operation area.
- **Service layer.** Reads run off the UI thread on shared, synchronous
  library calls. Mutations run on one desktop-owned worker that holds the
  process's single credential session. Every request keeps its request ID
  through retries; progress and cancellation follow the F2 contract; every
  completion is checked against the current view before it is applied.
- **Repository navigation.** Registered repositories with their configuration
  and index state. A repository whose root cannot be resolved is shown as
  unavailable with its reason and Retry and Remove actions (decision 2).
- **Discovery.** A document folder tree, including empty folders, and a
  ticket list showing short code, title, type, status, lifecycle closure and
  change time. Tickets default to lifecycle-open with visible Closed and All
  filters, and Ready and Blocked filters with each blocked ticket's reasons.
  Nonconforming entries stay visible under every filter. Empty, loading,
  never-refreshed, stale, incomplete, busy and unavailable are distinct
  states.
- **Find.** One entry that accepts a short code, listing every match with its
  context, or a full ULID, which opens that ticket directly even when it is
  closed.
- **Item inspection.** Tabs keyed by repository, kind and ULID; opening an
  open item focuses its tab. A read-only item view with a provenance header
  (primary, active or unverified context, branch, worktree), named metadata,
  unknown metadata, the source body and the discussion as a read-only thread.
  A nonconforming entry opens by path in an inspection view. The ticket view
  shows dependencies, dependents, parent and children, each opening the
  related ticket. Copy actions offer the short code and the ULID separately.
- **Index maintenance.** Manual Refresh and Rebuild per repository, and one
  background index-only refresh per registered repository at launch and after
  add, create or enable (decision 4). None contacts a network.
- **Settings**, each through F2 `prepare`/`execute`:
  - repositories: add and enable an existing one, create a new one, with
    primary branch, identity when missing and the optional short-code prefix;
    set identity and initials; remove a registration;
  - remotes: list with redacted locations, add, remove (confirmed) and select
    for publication;
  - shared SSH keys: list with label and fingerprint, generate with an
    optional masked passphrase, import by reference, copy the public key,
    select, clear, unregister, and delete a generated key (confirmed);
  - host trust: list pins, observe the key a host presents, approve an
    unknown host and replace a pin showing the old one (both confirmed).
- **Prompts.** One confirmation dialog for every confirmed command, showing
  the previewed effects and expiry. A masked passphrase prompt answering the
  library's credential provider, one per key at a time. Escape cancels and
  focus returns to the control that opened the prompt.
- **Operation area.** In-flight desktop requests with stage and Cancel where
  it can still take effect, failures that persist until dismissed, and each
  repository's pending operations. Resume is offered where it needs no
  network; Abandon (confirmed) for a pending local operation (decision 5).

## Explicit Exclusions And Downstream Obligations

- No body or metadata editing, item creation, document move, folder creation
  or repair. Repair is shown visibly unavailable on a nonconforming entry.
  D3 owns them.
- No draft store; D2 owns it. D1 holds no unsaved text, so there is nothing
  to preserve on removal or rebuild.
- No comment submission (D4), synchronization or resume of one (D4),
  conflict view, promotion or closure (D5), background polling, poll
  controls or worker shutdown policy (D6). Each is visibly unavailable where
  a user would look for it.
- No explicit Unlock action. Unlock happens when an operation needs the key;
  in D1 that is observing or approving a host. D4 and D6 own the Unlock and
  Sync recovery for polling.
- No change to the headless library: `src/lib.rs`, `src/repository.rs`,
  `src/repository/`, `src/canonical.rs`, `src/results.rs` and
  `src/runtime.rs` (track rule 5). A missing or wrong library capability found
  during implementation gets its own shared-library ticket.
- **Known library limits shown honestly, not worked around:**
  - `01M4EHGE2KDCBRZ9DMXBPP30R0`: the stored accessibility is never updated.
    D1 derives unavailability from resolution failing and shows the stored
    value only as reported.
  - `01M4W5SCBV6WFJP11Y7PJF5VS8`: operation reads name no item for local
    authoring rows. The operation area shows command and scope.
  - `01M4EPRY203WRKEDQH8403T35B`: whether an item worktree's copy differs from
    primary is not reported. Provenance shows which copy is read, nothing more.
  - `01M4PB7WZN9WNAQ3KXZ19J28KM`: removing some remotes is refused by the F2
    binding. D1 shows the refusal and its reason.
- **Native evidence on Windows and macOS** is an obligation carried to G1
  (decision 3).

## Required Desktop Contract

1. **The library is the only source.** No view reads SQLite, Git or the file
   system to find items, and no row is fabricated. What is shown is what a
   read returned, with the state it reported.
2. **The UI thread never blocks** on a read, a mutation, a prompt reply or
   the network. A slow or busy call leaves navigation usable.
3. **A completion applies only to the view that asked.** A result for a
   repository, tab or filter that has since changed is dropped.
4. **Viewing changes nothing.** Browsing, opening, switching tabs and
   selecting a repository provision no context, write no file and contact no
   network. Only refreshes, which are index-only, the user's explicit
   settings actions and the operation area's Resume and Abandon call the
   mutation boundary.
5. **Every change is the F2 boundary's.** One request ID per user intent,
   kept through retries; confirmed commands only with a fresh preview; no
   effect is reported that the envelope did not report; `partial` is shown as
   partial with its recovery action.
6. **Secrets stay in memory and out of output.** A passphrase is moved into
   the library's zeroizing holder on submit and the input is cleared. No
   passphrase, private key or credential reaches a log, notification,
   diagnostic, `Debug` rendering or persisted UI state. No private-key display
   or copy exists.
7. **Keyboard first.** Every action has a labeled focusable control, focus is
   visible, Tab order follows the layout, dialogs trap focus, Escape cancels
   and focus returns. Color is never the only signal of state.
8. **No display dependency in the CLI.** With no features enabled, the
   package's normal dependency graph contains no GPUI package. (`cargo tree`
   cannot select one binary; the CLI builds with no features, so this is
   its graph.)

## Acceptance And Exit Evidence

| Contract | Required evidence |
| --- | --- |
| Startup | Headless test: the window's first view is `Root`. Manual Linux Wayland run: the release build starts after transport initialization, with a registered repository listed. |
| Browsing end to end | Headless test against a real repository: tree, list and item view show exactly the rows the library returns, including a nonconforming entry. Manual run: this project's own repository is browsed end to end by keyboard. |
| Closure filter | An open ticket whose status text is `closed` is shown under Open; a lifecycle-closed ticket under Closed and All but not Open; a closed ticket opens directly by ULID. |
| Relationships | Ready and Blocked filters, blocked reasons, find by short code with a duplicate code listing both, and dependencies, dependents, parent and children each opening the related ticket by ULID. |
| Tabs | Opening an open item focuses its tab; items from two repositories stay open; a tab survives a refresh. |
| List states | Never-refreshed, stale, incomplete, empty, busy (cache lock held), failed refresh and unavailable index each render their own state. |
| Responsiveness | With a refresh delayed and with the cache lock held, the user still moves between repositories and tabs; the stale result of the earlier selection is not applied. |
| Second actor (track rule 4) | A second actor calling the library directly, as a child process of the test binary where the case is cross-process: it refreshes or abandons while the desktop shows the rows, and holds the repository lease while the desktop submits a job. The desktop shows the new state after its next read and the typed `busy`/blocked answer, never a stale success. |
| Unavailable repository | A registered repository whose folder is renamed away shows as unavailable with its reason; renaming it back and Retry resolves it; Remove, after `01M4R19P94` merges, removes the registration and leaves files untouched. |
| Settings | For each settings command: success, its typed failure, and, for confirmed commands, a stale preview and Escape cancelling with no effect. A `partial` result is shown with its recovery. |
| Prompts | Automated, at the provider and dialog: Supplied, Cancelled and Unavailable reach the worker; a closed window or quit answers `Unavailable` at once; a second job for the same key reuses the session's cached passphrase and does not prompt; focus returns. **Native checklist and F2's SSH tests only:** unlock during host observation, cancel then retry, host approve showing authority and fingerprint, and replace showing the old pin. Bin unit tests run under the multi-threaded test harness, where the transport cannot be initialized, so no automated desktop test reaches a real host. |
| Operation area | A pending local operation is listed after restart, resumed or abandoned with confirmation; a failure stays until dismissed. A pending synchronization's Resume is shown unavailable: proven by a model test over an operation row, since no local recipe makes a pending synchronization without SSH. |
| Secrets | Tests plant a sentinel passphrase and assert it is in no notification, operation entry, `Debug` output or persisted state, and that the input is cleared after submit. |
| Keyboard | Headless tests drive each core action by keystroke, assert visible focus and Escape/focus return. A manual keyboard-only checklist on Linux Wayland, recorded with build, display server and outcome per step. |
| CLI isolation | `cargo tree --locked -e normal --no-default-features -p manyhands` succeeds and its output contains no `gpui`; a failing `cargo tree` fails the check. |
| Regression | The four required Devenv checks, the CLI smoke test and the desktop smoke test pass. |

Windows and macOS execution is not claimed (decision 3).

## Product-Owner Decisions (2026-10-18)

Decided during planning, each as recommended.

1. **Shape.** D1 stays one Cycle in three Parts with a checkpoint review
   after each: Part A, shell, service layer, browsing, refresh and operation
   area; Part B, tabs and relationships; Part C, onboarding, settings and
   prompts. No Wave amendment.
2. **Unavailable repositories.** D1 derives unavailability from resolution
   failing rather than from the stored accessibility (`01M4EHGE2K` stays
   open). `01M4R19P94` is fixed and merged as a shared-library change before
   Part C, so Remove works for a repository whose folder is gone.
3. **Evidence.** Headless GPUI tests through GPUI Kit's test support, against
   real temporary repositories, plus a manual keyboard checklist on Linux
   Wayland run by the product owner. Windows and macOS are G1 obligations.
4. **Local refresh.** One background index-only refresh per registered
   repository at launch and after add, create or enable, plus manual Refresh
   and Rebuild. No refresh on focus; no network.
5. **Operation area actions.** Inspect everything; Resume where it needs no
   network; confirmed Abandon for a pending local operation. Synchronization
   resume stays unavailable until D4.
6. **Passphrase entry.** GPUI Kit's masked input is the transient entry
   buffer. On submit its text moves into `SecretPassphrase` and the input is
   cleared and dropped. That freed heap copies are not zeroed is a recorded
   residual risk.

## Product-Owner Decision (2026-10-19)

7. **`futures` as a direct, desktop-only dependency.** Approved. The
   mutation worker must wake the UI when a job progresses or finishes, and
   GPUI exposes no async channel. `futures = { version = "0.3", optional = true }`,
   enabled only by `desktop`; it is already locked at 0.3.34 through GPUI, so
   `Cargo.lock` gains only the direct edge. No other dependency is added.

## Review Record

The design's [Review Record](../plans/2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md#review-record)
holds the concern list and engineering rulings. Rows that change this
document's contract are the decisions above.

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing,
merging, closing and worktree cleanup each need their own authorization.
