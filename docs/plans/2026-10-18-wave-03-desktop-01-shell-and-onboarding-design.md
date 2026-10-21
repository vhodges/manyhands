---
title: "Wave 03 D1 Desktop Shell, Onboarding And Discovery Design"
date: 2026-10-18
status: "implemented"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M58Y0GZAQ2P9Z23H31H23DSH"
---

# Wave 03 D1 Desktop Shell, Onboarding And Discovery Design

## Intent And Approval Boundary

This design implements the
[D1 Cycle](../Cycles/wave-03-desktop-01-shell-and-onboarding.md). It says how
the desktop is laid out, how it calls the synchronous headless library
without blocking, how prompts reach the user, and how each requirement is
proved. The Cycle document owns scope, exclusions and the six product-owner
decisions of 2026-10-18; this document cites them by number.

The design was approved in conversation on 2026-10-18 as a summary; this
written form is not yet approved. Approval does not authorize implementation.

## Library Surface Audit At `f6a4c06`

Read from source at `f6a4c06300f4be80eb18f843a9e4fcbde847e47e`. Nothing was
compiled or run. Paths are under `src/repository/` unless stated. This
replaces the provisional
[API audit](../research/wave-03-api-audit.md)'s D01–D06, D28, D30 and R07 rows,
which predate F1 and F2: every headless part those rows marked missing now
exists, and only presentation is absent.

Re-checked on 2026-10-19 after merging main at `021f684` (Wave 03 C1). The
library changed only by adding `ResultCode::UsageError` and
`ResultCode::CommandUnavailable`, which the CLI emits itself and
`RepositoryService` never returns, with their `recovery` arms. No signature
cited below moved. D1 shows codes through `ResultCode::message()`, never an
exhaustive match, so the new variants need no desktop change. C1 did not edit
the API audit rows replaced here.

| D1 need | Library entry point | Notes for D1 |
| --- | --- | --- |
| Open the service | `RepositoryService::open_default()` (`repository.rs`), `open_at(&Path)` for tests | `RepositoryService` is asserted `Sync` (`repository.rs`, test near the end of the file). Every method is synchronous. |
| Repository list | `list_repositories() -> Result<RepositoryListDto, ReadError>` (`read/admin.rs`) | `RepositorySummaryDto{root: String, enabled_at, accessibility, configuration, index, problem_count}`. `accessibility` is never updated (`01M4EHGE2K`); D1 does not rely on it. |
| Resolve a repository | `resolve_repository(&Path) -> Result<ResolvedRepository, ReadError>` (`read/resolve.rs`) | Starts with `fs::canonicalize`: a deleted root answers `invalid_path`, a root without `.git` `not_repository`. This is D1's unavailability signal (decision 2). |
| Inspect, identity, remotes | `inspect_repository`, `repository_identity`, `list_remotes_redacted` (`read/admin.rs`) | Remote locations already redacted. |
| Document tree | `list_document_folders(repo) -> DocumentFolderListDto`, `list_documents(repo) -> ItemListDto` (`read/items.rs`) | Folders carry `path` (`docs/…`) and `empty`. |
| Ticket list | `list_tickets(repo, &TicketFilter) -> ItemListDto` (`read/items.rs`) | `TicketFilter{status, ticket_type, project, closure: ClosureFilter (default All), slug, readiness: Option<ReadinessFilter>}`. A nonconforming entry is never removed by a filter. |
| Ready and blocked | `TicketFilter.readiness` on `list_tickets`; `ItemDto.readiness{state, reasons[{code, ids, complete}]}` | `ticket_readiness` (`read/graph.rs`) lists no nonconforming entry, so D1 does **not** use it for the list (ruling R5). |
| Find | `find_tickets_by_slug(repo, &str) -> ItemListDto` (`read/graph.rs`); `show_item(repo, &ItemId)` | Several matches are possible and all are listed. |
| Relationships | `ItemDto.{parent, deps}`, `ticket_dependencies(repo, id, DependencyDirection, depth)`, `ticket_children(repo, id)` (`read/graph.rs`) | Dependents come from `DependencyDirection::Up`. |
| Item read | `show_item(repo, &ItemId) -> ItemDto`, `show_path(repo, context, &Path)` (`read/items.rs`) | `ItemDto.context: ItemContextDto{kind: primary/unverified/active, branch, worktree, head_oid}` is the provenance. |
| Comments | `list_comments(repo, &ItemId) -> CommentListDto` (`read/comments.rs`) | Flat, ordered, each with `depth`, `parent_id`, `author`. |
| Index and operations | `index_status(repo)`, `list_operations(repo)`, `show_operation(repo, OperationId)` (`read/status.rs`) | `OperationDto.next_action` names resume, retry or review; `item_id` is null for local authoring rows (`01M4W5SC`). |
| Keys and hosts | `list_keys`, `show_key`, `public_key_text`, `list_host_pins`, `inspect_host` (`read/credentials.rs`) | Public key text only; no private material is readable. |
| Host observation | `observe_host<P>(root, Option<&SshAuthority>, &mut SessionCredentials<P>) -> Envelope<HostObservationDto>` (`mutation/bind/host.rs`) | The one read that connects and may unlock. Runs where the session lives. |
| Mutations | `prepare(&Mutation) -> Envelope<PrepareDataDto>` (`mutation/confirm.rs`); `execute<P>(MutationCall, &mut SessionCredentials<P>, &mut dyn ProgressSink, &CancellationToken) -> Envelope<MutationDataDto>` (`mutation/execute.rs`) | `prepare` takes no session, no lease and no network, and writes one confirmation record. `execute` runs on the caller's thread. |
| Commands D1 calls | `Mutation::{RepoCreate, RepoEnable, RepoRemove, RepoIdentitySet, RemoteAdd, RemoteRemove, RemoteSelect, IndexRefresh, IndexRebuild, KeyGenerate, KeyImport, KeySelect, KeyClear, KeyRemove, KeyDelete, HostApprove, HostReplace, OperationResume, OperationAbandon}` (`mutation/mod.rs`) | Confirmed: `RepoCreate`, `RepoEnable`, `RepoRemove`, `RepoIdentitySet`, `RemoteRemove`, `KeyDelete`, `HostApprove`, `HostReplace`, `OperationAbandon`. |
| Progress and cancellation | `ProgressSink::event(&mut self, &ProgressEvent)`, `CancellationToken` (`Clone`, `Arc<AtomicBool>`), `cancel_request(RequestId)` (`mutation/progress.rs`) | Events arrive synchronously on the executing thread, some under the lease: the sink must only forward. After acceptance only `ItemSync`/`RepoSync` have safe points, so a local D1 command cannot be stopped once accepted. |
| Credentials | `keys::SessionCredentialProvider::request_passphrase(&mut self, &UnlockRequest) -> PassphraseResponse{Supplied, Cancelled, Unavailable}`; `SessionCredentials<P>::new(provider)`; `SecretPassphrase::new(String)` (`keys/session.rs`) | `SessionCredentials` is passed `&mut`, so one owner per process. |
| Key store | `KeyStore::for_current_user()`, `KeyStore::for_home(&Path)` (`keys/storage.rs`) | Tests use `for_home` with a temporary home. |
| Results | `Envelope<T>`, `Outcome`, `ResultCode::{message, failure_class}`, `RecoveryAction{action: RecoveryActionKind, operation_id, arguments}` (`src/results.rs`) | Messages are fixed text per code and safe to show. |
| Test hooks | `RepositoryService::hold_lease_for_testing(root, data_dir, LeaseKind::{Repository, Bootstrap, CacheRead, CacheWrite})`, `open_at_with_failure_point_for_testing` (`repository.rs`) | Real-library delay and busy fixtures. |
| Transport init | `runtime::initialize_git_transport_before_threads()`, `git_transport_initialized()` (`src/runtime.rs`) | Already called first in `main.rs`. |

**Lock behavior relevant to responsiveness.** A refresh observes and scans
without the cache lock and takes the cache write lock only to persist
(`refresh_repository_observed`, `repository.rs`). A read waits at most
250 ms for the shared lock (`coordination.rs`, `LEASE_TIMEOUT`) and then
answers `busy`. So reads stay available during a long scan except for
short persist windows, and a held lock produces `busy`, not a hang.

**Not verified by reading, checked in Task 2.** That `Mutation`,
`Envelope<MutationDataDto>`, `Envelope<PrepareDataDto>`, every read DTO and
`ReadError` are `Send + 'static`, and that `RepositoryService` is `Send`, so
they can cross to and from worker threads. A failure is a shared-library
change, not a desktop workaround.

## Module Layout

Desktop code is part of the `manyhands` binary only. `src/lib.rs` gains
nothing.

| Path | Role |
| --- | --- |
| `src/main.rs` | Transport init, then `desktop::run()`. |
| `src/desktop/mod.rs` | `run()`: application, Kit init, key bindings, first window. |
| `src/desktop/services/mod.rs` | `Services` handle given to views; the `Backend` seam. |
| `src/desktop/services/reads.rs` | Read jobs on the background executor, busy retry. |
| `src/desktop/services/worker.rs` | The mutation worker thread, its queue and session. |
| `src/desktop/services/prompts.rs` | Credential provider bridge and prompt deduplication. |
| `src/desktop/services/requests.rs` | Desktop request records, intents and retry. |
| `src/desktop/model/*.rs` | GPUI-free state: navigation, filters, list state, tabs, relationships, operations, generations. |
| `src/desktop/view/*.rs` | GPUI views: workspace, sidebar, documents, tickets, find, tabs, item, relationships, settings pages, dialogs, operation area. |
| `src/desktop/keys.rs` | Actions and key bindings. |
| `src/desktop/test_support/*.rs` | `#[cfg(test)]` fixtures: real repositories, isolated homes, the delaying backend. |

Unit tests sit beside each module as `*_tests.rs`, the repository's
convention. Model tests need no GPUI; view tests use the Kit test support
(ruling R2).

## Startup

```text
main()
  unsafe initialize_git_transport_before_threads()   // first statement, unchanged
  desktop::run()
    gpui_kit::application().with_assets(..).run(|cx| {
      gpui_kit::init(cx);
      desktop::keys::bind(cx);
      match RepositoryService::open_default() {
        Ok(service) => open_main_window(Services::start(service, cx), cx),   // Root first
        Err(error) => open_startup_failure_window(StartupFailure::from(&error), cx),
      }
    })
```

`StartupFailure` keeps only the error's kind and a fixed message, never its
text.

`open_main_window` follows the editor spike's verified pattern: the outer
`cx.new` creates `Root`, and the workspace is created inside its closure, so
`Root` is the window's first entity. `Root`'s render does not draw dialog,
sheet or notification layers; the workspace renders
`Root::render_dialog_layer`, `render_sheet_layer` and
`render_notification_layer`. Window size uses `Bounds::centered` with the
synchronous `App` before spawning.

If the service cannot open (`open_default` fails), a window still opens
(`Root` first) with a single view: a fixed message for the failure's kind,
**Retry**, which calls `open_default` again and on success replaces the view
with the workspace, and **Quit**. There is no service, so nothing else, and
no rebuild, is offered. The process does not exit by itself.

No desktop module uses a glob import of `gpui_kit`: `gpui-pre` re-exports
its `test` attribute unconditionally, and a glob would shadow `#[test]` in
every child module that does `use super::*`. Import names explicitly. This
also replaces today's `use gpui_kit::*` in `main.rs`.

`Services::start` spawns the mutation worker thread. GPUI's own threads
exist by then; the transport was initialized before either.

## Service Layer

### The seam

```rust
pub(crate) trait Backend: Send + Sync + 'static {
    fn service(&self) -> &RepositoryService;
    fn key_store(&self) -> KeyStore;                // for_current_user(); for_home in tests
    /// Awaited inside the read's future, before the blocking library call.
    fn read_gate(&self, _kind: ReadKind) -> Option<ReadGate> { None }
    /// Called on the worker thread before a job's library call.
    fn before_job(&self, _kind: JobKind) {}
}
```

Production uses `LibraryBackend`, whose hooks do nothing. The seam only
**delays**; it never fails or fabricates a call (contract 1). Failures come
from the library's own test entry points.

- **Read delays are asynchronous.** A `ReadGate` is a
  `futures::channel::oneshot::Receiver<()>` that the read's future awaits
  before it calls the library. Under `#[gpui_kit::test]` the background
  executor is GPUI's `TestScheduler`, which runs background work on the test
  thread itself (`gpui-pre-scheduler` `test_scheduler.rs`), so a read that
  blocked on a barrier would block the only thread and nothing could release
  it.
- **Job delays may block,** because jobs run on the desktop's own worker
  thread: `before_job` waits on a `std::sync::Barrier`-style gate the test
  releases.
- **Library delays and failures** use the library's hooks:
  `set_refresh_persist_hook_for_testing` and
  `set_observation_hook_for_testing` (both `repository.rs`, run on the
  calling thread, so on the worker) to hold a refresh mid-way;
  `open_at_with_failure_point_for_testing` with
  `FailurePoint::BeforeIndexTransactionCommit` for a failed refresh or
  rebuild; `hold_lease_for_testing` for busy and lease-held cases.

### Reads

```rust
impl Services {
    pub fn read<T, F>(&self, stamp: Stamp, f: F, cx: &mut Context<V>,
                      apply: impl FnOnce(&mut V, Stamp, ReadResult<T>, &mut Context<V>) + 'static)
    where F: Fn(&RepositoryService) -> Result<T, ReadError> + Send + 'static, T: Send + 'static;
}
```

`read` spawns on `cx.background_executor()` and returns to the entity with
`cx.spawn`. `prepare` returns an `Envelope<PrepareDataDto>`, not a
`Result<_, ReadError>`, so it has its own `Services::prepare` with the same
stamping and threading. On `busy` it retries up to three times at 100, 200 and 400 ms,
then delivers `busy` for the view to show with Retry (ruling R6). The
returned `Task` is stored in the asking model's slot, so replacing a request
drops the previous one; the stamp check below still applies, because a
dropped task's blocking library call cannot be interrupted.

### Stamps and stale completions

```rust
pub struct Stamp { pub scope: ViewScope, pub generation: u64 }
```

`ViewScope` identifies what asked: the repository list, a repository's
document tree or ticket list with its filter, a tab, a settings page. Each
model keeps a generation per scope and increments it whenever its inputs
change (repository selected, filter changed, refresh completed). `apply`
first compares the stamp with the model's current generation for that scope
and drops a mismatch. Tests assert both outcomes (contract 3).

### The mutation worker

One `std::thread`, started by `Services::start`, owns:

- `Arc<dyn Backend>`;
- `SessionCredentials<DesktopProvider>`, the process's only session;
- a queue of jobs in two lanes: **user** and **background**. The worker
  always takes the next user job before any background job. A running job is
  never preempted.

```rust
enum Job {
    Execute { request: DesktopRequestId, call: MutationCall, cancel: CancellationToken },
    ObserveHost { request: DesktopRequestId, root: PathBuf, authority: Option<SshAuthority> },
}
```

`prepare` does not use the worker: it needs no session, takes no lease and
is quick, so it runs as a read (ruling R7). Launch and post-onboarding
refreshes are background jobs; everything the user starts is a user job.

The worker sends `WorkerEvent::{Started, Progress(ProgressEvent), Finished(Envelope<…>)}`
to the UI over a `futures::channel::mpsc` channel that a foreground task
(`cx.spawn`) awaits and drains into the request model; the worker receives
jobs over a `std::sync::mpsc` channel. GPUI does not re-export an async
channel, so this needs `futures` as a direct, desktop-only dependency
(Cycle decision 7, approved 2026-10-19; already locked at 0.3.34 through GPUI). Without it the
only alternatives are a UI timer that polls a `std` channel, which the scroll
performance lessons warn against, or parking an executor thread on a blocking
receive. Its `ProgressSink` only forwards a clone of the event (the F2 rule
that the sink must not call back into the library).

### Requests and retry

A desktop request is what the operation area lists:

```rust
pub struct DesktopRequest {
    pub id: DesktopRequestId,
    pub request_id: RequestId,          // the F2 replay identity, kept for every attempt
    pub intent: Intent,                 // non-secret form values that rebuild the Mutation
    pub confirmation: Option<ConfirmationId>,
    pub state: RequestState,            // Queued, Running(stage), Finished(summary), Dismissed
}
```

`Intent` is D1's own enum of form values, one variant per command. Each
attempt builds a fresh `Mutation` from it, so D1 does not depend on the
library's input types being `Clone`. **Key generation with a passphrase**
holds no passphrase in its intent; a retry prompts for it again. That is
safe because the F2 digest records only whether the key is protected
(`mutation/bind/key.rs`), so the retry is the same request.

Retry sends the same `request_id`. A finished request answered from its
record is shown as that record. A request whose answer was
`request_mismatch` is not retried with new input under the same ID: a new
user intent always gets a new request ID (contract 5).

### Cancellation

Cancel is offered:

- for a queued job: it is removed from the queue and never executes;
- for a running job before the `accepted` progress event: its token is
  cancelled, which every command honors before acceptance;
- never after acceptance for D1's local commands, where it would have no
  effect. The control then reads "Finishing" and is disabled, not hidden.

`cancel_request` is not needed in-process: the token reaches the running
call. Host observation has no token; while it runs, Cancel is unavailable. The
transport bounds each TCP connect (10 s) and each blocking SSH call (30 s),
but not DNS or the whole exchange, so it can run long; the request shows
"Connecting" until it returns.

### Credential prompts

`DesktopProvider` implements `SessionCredentialProvider` on the worker
thread:

1. `request_passphrase(&UnlockRequest)` sends `PromptRequest{key_id, label,
   reason, reply: futures::channel::oneshot::Sender<PromptReply>}` to the UI
   and blocks on the reply with `futures::executor::block_on`. The worker
   thread is not an executor thread, so blocking it stalls nothing else. If
   the send fails (the UI side is gone), it returns `Unavailable` at once and
   never blocks. A dropped reply sender also reads as `Unavailable`.
2. **Deduplication is structural.** One worker runs one job at a time and the
   provider is called synchronously, so two prompts can never be open at
   once. After a successful unlock the session caches the passphrase for the
   selected key, so a later job for that key does not prompt
   (`SessionCredentials`, `keys/session.rs`). No answer is copied to several
   waiters.
3. The dialog's input is `InputState::new(window, cx).masked(true)`; the
   mask toggle (`Input::mask_toggle`) is not offered. The masked input
   already refuses copy and hides its value from accessibility. Submit moves
   the text into `SecretPassphrase::new`, calls `set_value("")`, drops the
   `InputState` entity and replies `Supplied`. Escape or Cancel replies
   `Cancelled`. A window closing or the application quitting replies
   `Unavailable`.
4. An empty passphrase or one containing NUL is refused in the dialog,
   matching `SecretPassphrase::new`'s `InvalidPassphrase`.

Only `observe_host` and the two host commands can reach the provider in D1;
local commands never ask. After `Cancelled`, the operation ends with
`unlock_required`, nothing changes and the page offers Retry.

The masked input's `SharedString` is not zeroizing (decision 6). Residual
risk: freed copies of typed text may remain in process memory until reused.
No other copy is made: the text is never formatted, logged, stored in a
model or placed in an `Intent`.

### Quitting

D1 has no polling worker; D6 owns the shutdown contract. GPUI gives
`on_app_quit` futures at most 200 ms (`SHUTDOWN_TIMEOUT`, `gpui-pre`
`app.rs`), so waiting must happen **before** quit is allowed to proceed:

- A `Quit` action (`secondary-q`) and the window's close request
  (`Window::on_window_should_close`) both go through one handler. With no
  job running it allows the close or quits.
- With a job running it refuses the close, stops admitting jobs, empties the
  queue, cancels the running job's token, answers a waiting prompt
  `Unavailable`, and shows "Finishing <command>" with a **Quit now** button
  (action `QuitNow`). When the job returns it quits.
- Quit now ends the process at once. That is an interruption, which the
  library's journals recover, as the runtime RFC treats forced termination
  (ruling R9).
- Host observation is bounded per attempt by the transport's 10 s connect
  and 30 s per-SSH-call timeouts, but DNS and total transfer are not bounded
  (`src/runtime.rs`). "Finishing" can therefore last indefinitely there;
  Quit now is always available.

## View Models

All GPUI-free and unit-tested without a window.

- **`Navigation`**: the repository list state, the selected repository root,
  and the active pane (Documents or Tickets).
- **`ListState<T>`**: `Loading | Ready{rows, complete, index} | Busy |
  Failed{code, recovery}`. `index` (`IndexStateDto`) separately distinguishes
  current, stale and never-refreshed. Empty is `Ready` with no rows and is
  labeled differently from never-refreshed. An unavailable index is
  `Failed{index_unavailable}`.
- **`RepositoryEntry`**: a summary plus `Resolution::{Unknown, Resolved,
  Unavailable{code}}`. Resolution is attempted on selection and on Retry,
  never in a loop.
- **`TicketQuery`**: `closure` (default `Open`, decision of the Wave), and
  `readiness: None | Ready | Blocked`, mapped to `TicketFilter`.
- **`DocumentTree`**: built from folders plus document paths. Each node is a
  folder (possibly empty) or a document row; nonconforming documents are
  rows with a problem badge. Ordering follows the read's order within a
  folder, folders first.
- **`Tabs`**: an ordered list of `TabKey`s and the active index.
  `TabKey::Item{root, kind, id}` or `TabKey::Inspect{root, worktree, path}`
  for a nonconforming entry (ruling R4). `open(key)` focuses an existing tab
  or appends one.
- **`ItemView`**: the item `ListState`-like state for one tab, its comments
  and, for a ticket, its relationships.
- **`Relationships`**: dependencies (`deps` with resolution), dependents
  (`ticket_dependencies(Up, depth 1)`), parent and children.
- **`Operations`**: desktop requests plus each repository's
  `list_operations` rows.

## Layout And Keyboard

```text
┌────────────┬───────────────────────┬──────────────────────────────────────┐
│ Repos      │ [Documents|Tickets]   │ tab │ tab │ tab                       │
│ ▸ repo-a   │ Find: [_____________] │ header: title · code · ULID · context │
│   repo-b ⚠ │ Open|Closed|All       │ metadata · relationships             │
│            │ All|Ready|Blocked     │ body (source, read-only)             │
│ Settings   │ list / tree           │ discussion (read-only)               │
├────────────┴───────────────────────┴──────────────────────────────────────┤
│ Operations: running · failed · pending                                    │
└────────────────────────────────────────────────────────────────────────────┘
```

Regions are focus groups. Bindings use GPUI's `secondary` modifier (Ctrl on
Linux and Windows, Cmd on macOS); they are defined once in
`desktop/keys.rs` with `gpui_kit::actions!` (ruling R8):

| Action | Binding |
| --- | --- |
| Next / previous region | `F6` / `shift-F6` |
| Focus repositories, navigation, content, operations | `secondary-1` … `secondary-4` |
| Find | `secondary-p` |
| Next / previous tab, close tab | `ctrl-tab` / `ctrl-shift-tab`, `secondary-w` |
| Refresh / Rebuild repository index | `secondary-r` / `secondary-shift-r` |
| Settings | `secondary-,` |
| Quit / Quit now (while finishing) | `secondary-q` / the focused Quit now button |
| Open selected row | `enter`; lists and the tree also use the Kit's arrow bindings |
| Cancel dialog | `escape` |

Tab and shift-Tab move through every control of the window in order, from
one region into the next (Kit `Root` focus cycling is window-wide, ruling
R-T17a); the region keys move between regions directly. Every
dialog traps focus and on close focuses the control recorded when it opened,
or the region's first control if that control is gone. State is never shown
by color alone: stale, unavailable, nonconforming, closed and failed each
carry a text label.

## Discovery

### Repository selector

Lists `list_repositories` rows by root, each labeled with its directory name
and full root as secondary text. Selecting a row resolves it. On success the
repository's configuration and index state show, and its document tree and
ticket list load. On `invalid_path`, `not_repository`,
`repository_inaccessible` or `repository_not_registered` the entry becomes
**Unavailable** with the
code's message and two actions: Retry (resolve again) and Remove (prepare
`RepoRemove` for that root). Remove depends on `01M4R19P94`; until that
merges the preview's own failure is shown as returned (decision 2).

An enabled repository with an invalid configuration shows its
`configuration.problems` and stays browsable where reads succeed.

### Documents

`list_document_folders` and `list_documents` run in parallel under the same
stamp. The tree shows folders, including empty ones, and documents by title
with their path as secondary text. A nonconforming document shows its path
and problem code; activating it opens an inspection tab. Folder creation is
visibly unavailable.

### Tickets

Columns: short code, title, type, status, lifecycle (Open or Closed, as text,
from `closure`), and change time. The list calls
`list_tickets(repo, &TicketFilter{closure, readiness, ..})`. Status text is
displayed as written, so an open ticket with status `closed` shows status
`closed` and lifecycle Open. Under Blocked each row shows its reasons:
an open dependency names the blocking tickets by short code and title, an
unresolved dependency names the missing ULID, and a cycle says so.
Incomplete reasons (`complete: false`) are labeled incomplete.

### Find

One input. If the text parses as a ULID, `show_item` opens it in a tab;
a closed ticket opens normally. Otherwise the text is taken as a short code:
`find_tickets_by_slug` lists every match with title, lifecycle and context,
and choosing one opens it by ULID. No match says so. A short code is never
used as a tab key or mutation input.

## Item Inspection

Opening a row creates or focuses its tab and calls `show_item` (or
`show_path` for an inspection tab), then `list_comments`, then for a ticket
`ticket_dependencies(Up, Some(1))` and `ticket_children`.

- **Header**: title, kind, short code, ULID, each copy action separate
  (`write_to_clipboard`, which is a write only), lifecycle closure with
  `closed_at`/`closed_by` when closed, project status as a separate field.
- **Provenance**: context kind as text (Primary, Active item worktree,
  Unverified), branch, worktree and head commit. Nothing claims the copy
  equals or differs from primary (`01M4EPRY` limit). When the item carries a
  context problem (a duplicate or mismatched context, reported in
  `problems`), the header becomes a recovery view: the problem codes with
  their fixed guidance and the paths involved, and no way to choose a
  context (desktop RFC).
- **Metadata**: named fields, then unknown metadata as read-only key/value
  rows, with `null` values labeled not representable.
- **Body**: the source Markdown in a read-only, selectable, monospaced view.
  If `source` is null (not UTF-8), say so. No rendering, no resource
  loading, no link activation.
- **Discussion**: comments in list order, indented by `depth`, with author
  (or "unknown author") and time. Nonconforming comments show path and
  problem.
- **Relationships**: four groups, each row with short code, title, lifecycle
  and resolution; an unresolved target shows its ULID and cannot be opened.
  Relationship problems on the ticket are listed under the groups.
- **Unavailable actions**: Edit, Comment, Sync, Approve and merge, Close
  ticket and Repair show as disabled with "Available in a later release".

Refreshing a repository increments its generations; open tabs reload their
item and keep their position in the tab strip. An item that no longer
exists shows `item_not_found` with Refresh, and its tab stays until closed.

## Index Maintenance

Refresh and Rebuild are `IndexRefresh`/`IndexRebuild` user jobs. At launch,
the worker enqueues one `IndexRefresh` background job per registered
repository, in list order; after a successful create or enable it enqueues
one for that root (decision 4). A failed refresh shows in the operation area
with its recovery, and the lists show the stale state they then read. A
refresh never runs on window focus, timer or navigation.

## Settings

A settings page per area, built with the Kit `Settings`/`SettingPage`
components, reached from the sidebar or `secondary-,`. Every action builds an
`Intent`, then:

- for an unconfirmed command, enqueue a user job;
- for a confirmed command, `prepare` (as a read), show the confirmation
  dialog, and on Confirm enqueue `execute` with the `ConfirmationId`.

### Repositories

- **Add existing**: path field (keyboard text, with an optional Browse that
  uses `prompt_for_paths`), primary branch, optional prefix, identity fields
  shown when `inspect_repository` reports none. `RepoEnable` (confirmed).
- **Create**: path to an empty or absent directory (Browse uses
  `prompt_for_new_path`), primary branch, identity, optional prefix.
  `RepoCreate` (confirmed). The preview creates nothing.
- **Identity and initials**: `RepoIdentitySet` (confirmed), with initials
  Unchanged, Clear or Set.
- **Remove**: `RepoRemove` (confirmed). The dialog says local files remain.
  On success every tab of that root closes, so a later registration at the
  same root never inherits old tabs.

Browse uses `prompt_for_paths` with directories only, for both Add and
Create (Create accepts a chosen empty directory or a typed absent path). If
the platform dialog fails or is dismissed, the text field is unchanged and
stays usable; a failure also disables Browse with its reason (ruling R10).
The test platform can simulate only a selection or a dismissal, so the
failure branch is covered by the native checklist, not a test.

### Remotes

List from `list_remotes_redacted` with publication eligibility and
selection. Add (`RemoteAdd`), Remove (`RemoteRemove`, confirmed; a refused
removal shows its code and message, `01M4PB7W`), Select for publication or
none (`RemoteSelect`). Non-SSH remotes are listed and labeled not eligible.

### Shared SSH keys

List with label, fingerprint, kind and selection. Generate (label, optional
passphrase with confirmation field, `KeyGenerate` with
`KeyStore::for_current_user()`), Import by private and public key path
(`KeyImport`), Copy public key (`public_key_text`), Select, Clear selection,
Unregister (`KeyRemove`), and Delete for a generated key only (`KeyDelete`,
confirmed). An imported key offers no Delete. There is no action that shows
or copies private material.

### Hosts

List pins (`list_host_pins`). For a repository with an SSH publication
remote, Check host runs `observe_host` on the worker; it may prompt for the
selected key's passphrase. The result shows host, port, algorithm and
fingerprint. If unpinned, Approve prepares `HostApprove`; if pinned to a
different key, Replace prepares `HostReplace` and the dialog shows old and
new fingerprints side by side. If it matches, it says so. Approval writes
only after the library re-observes on an authenticated connection.

## Confirmation Dialog

One component renders a `PreviewDto`: command title, scope (repository,
target), each `PreviewEffectDto` as a line, the command-specific details,
and the expiry as local time. A `HostReplacement` preview renders both pins.
Confirm executes with the `ConfirmationId` and the request's own
`request_id`. Escape or Cancel closes it with no effect; the record simply
expires. If execute answers `confirmation_expired`, `confirmation_used`,
`confirmation_mismatch`, `confirmation_not_found` or `external_change`, the
dialog is replaced by the message and a Prepare again action, which takes a
fresh preview under the same request.

## Operation Area

Two sections:

- **This session**: every desktop request, newest first, with command,
  scope, state and, while running, the latest stage. Finished requests show
  their outcome: success and noop collapse to one line; `partial`,
  `blocked`, `error` and `cancelled` stay expanded with message, completed
  effects and recovery actions until dismissed (contract: failures are not
  transient). Notifications may echo an outcome but are never its only
  record.
- **Pending in repositories**: `list_operations` rows that are not ended,
  for each resolved repository, reloaded after each finished request and on
  refresh. Each row shows command, scope, operation ID and `next_action`.
  Actions (decision 5):
  - **Resume** when `next_action` is resume and the operation is local:
    `OperationResume` user job. A synchronization row shows Resume disabled
    with "Synchronization is not available yet".
  - **Abandon** for a pending local operation: `OperationAbandon`
    (confirmed).

Recovery actions in an envelope map to controls: `index.refresh`,
`index.rebuild`, `request.retry`, `request.prepare`, `operation.resume`,
`operation.abandon`, `repo.identity_set`, `host.approve`, `host.replace` and
`repo.inspect` each open or run the matching D1 action. `request.cancel`
maps to Cancel. An action D1 does not offer is shown as text, disabled.

## Result Presentation And Redaction

Views show `ResultCode::message()` text and the envelope's `message`, both
fixed per code, plus the code itself in small type for support. They never
format a `ReadError`, `RepositoryError`, `git2` error, URL or input with
`Debug`. Nothing is written to a log by D1. Model types that could hold a
secret do not derive `Debug`; `Intent::KeyGenerate` holds only
`protected: bool`.

## Testing

### Harness

- Feature `desktop-test-support = ["desktop", "gpui-kit?/test-support"]`
  (ruling R2). View tests are `#[cfg(all(test, feature =
  "desktop-test-support"))]`; model tests are plain `#[cfg(test)]`. All are
  unit tests of the `manyhands` binary, which
  `cargo test --all-features --locked` runs.
- No desktop module, test or not, uses `use gpui_kit::*` (see Startup).
- **Threading under test.** `#[gpui_kit::test]` runs foreground and
  background work on the test thread. A test that waits for the real worker
  thread must call `cx.executor().allow_parking()` (parking otherwise
  panics), and then runs under the scheduler's 15 s hard timeout. Read delays
  are async gates; job delays block the worker thread only.
- **Feasibility gap, checked first in Task 1:** that a `#[gpui_kit::test]`
  opens a window on Linux with no display **and** completes one worker
  round trip (a real `IndexRefresh` whose `Finished` event reaches the
  model). If either fails, stop and return to the product owner before
  Task 2.
- **No transport in desktop tests.** Bin unit tests run under libtest's
  multi-threaded harness, where `initialize_git_transport_before_threads`
  cannot be called safely, and transport refuses to run uninitialized. No
  desktop test reaches a network; host success and unlock during observation
  are native checklist evidence plus F2's SSH tests (ruling R11).
- `src/desktop/test_support/` includes `tests/support/git_home.rs` with
  `#[path]` so every desktop test isolates libgit2's global, XDG and system
  configuration the way library tests do, and builds real repositories
  through the library (`RepoCreate`, `TicketCreate`, `DocumentCreate`,
  `TicketSave` with `deps`/`parent`) in temporary directories, with
  `RepositoryService::open_at` and `KeyStore::for_home`. States that no
  local library operation produces (a lifecycle-closed ticket, a duplicate
  short code, a merged-in cycle, a nonconforming file) are made as the
  library's own read tests make them (`tests/support/items.rs`,
  `tests/read_items.rs`): write the canonical file, commit it with `git2`,
  then refresh through the library. Desktop code never does this; only
  `test_support::fixtures` does.

### Proof map

| Claim | Test |
| --- | --- |
| `Root` first, layers rendered | View test inspects the window's root entity type and opens a dialog. |
| No fabricated rows | View test compares visible rows with the same library read. |
| Closure and readiness filters | Fixture with an open ticket whose status is `closed`, a closed ticket, a blocked chain and a nonconforming ticket file; each filter's visible rows. |
| Find | Duplicate short code lists both; a ULID opens a closed ticket. |
| Tabs | Model tests for focus-existing and cross-repository keys; view test that a refresh keeps tabs. |
| Stale completion | An async `ReadGate` holds the first repository's list; the user selects a second, whose read completes; the gate is released; the first result is dropped. |
| Busy | `hold_lease_for_testing(CacheWrite)` during a list read shows Busy and Retry succeeds after release. |
| Delayed refresh | `set_refresh_persist_hook_for_testing` holds a real refresh on the worker thread; navigation and tab switching still work; release updates the list. |
| Failed refresh | `open_at_with_failure_point_for_testing(FailurePoint::BeforeIndexTransactionCommit)` makes a real refresh and a real rebuild fail; the operation area keeps the failure until dismissed. (`FailurePoint::RefreshFailure` is checked only by promotion and does not apply.) |
| Unavailable repository | Rename the fixture's folder away; selection shows Unavailable; rename it back; Retry resolves. Remove test enabled after `01M4R19P94` merges. |
| Settings commands | One view test per command for success and its typed failure; confirmed commands also for Escape and a stale preview (change the observed state between prepare and confirm). |
| Partial | `RepositoryService::open_at_with_failure_point_for_testing` with `FailurePoint::AfterRepositoryInitialization` (repository create) and `FailurePoint::AfterRemoteMutation` (remote add); the operation area shows `partial`, its completed effects and its recovery, and Retry under the same request completes it. |
| Prompts | Provider tests from a plain thread: Supplied, Cancelled, a dropped reply and a failed send each answer as designed, the last two `Unavailable` at once; a second job for an unlocked key does not prompt; quit and window close answer `Unavailable`. Host success and unlock during observation are native checklist step 9 plus F2's SSH tests (ruling R11). |
| Secrets | Sentinel passphrase absent from every model's `Debug`-free snapshot, request records, operation area text and notifications; input empty after submit. |
| Keyboard | View tests send keystrokes for each core action, assert the focused element and visible focus style, Escape closes dialogs and focus returns. |
| Second actor (track rule 4) | A second `RepositoryService` over the same data directory, and for cross-process cases a child process of the test binary (the pattern `tests/support/git_home.rs` uses for `own`), refreshes, abandons or holds the repository lease (`hold_lease_for_testing(LeaseKind::Repository)`) while the desktop shows or submits; the desktop shows the new state after its next read and the typed answer to its job. |
| Duplicate context | A fixture with a context problem on an item shows the recovery view and no context choice. |
| CLI isolation | `scripts/check-cli-no-gpui.sh` runs `cargo tree --locked -e normal --no-default-features -p manyhands` into a file, fails if that command fails, then fails if the file mentions `gpui`. |

### Native checklist (Linux Wayland, product owner)

Run on a release build (`cargo run --release --locked --features desktop
--bin manyhands`) with an isolated application data directory and a
disposable SSH remote, keyboard only. Record build revision, `Cargo.lock`
digest, compositor, scale factor and each step's outcome in a ticket
comment.

1. Launch; the repository list shows; launch refreshes complete.
2. Add this project's repository; browse Documents and Tickets end to end.
3. Switch closure and readiness filters; read a blocked ticket's reasons.
4. Find by short code and by ULID; open three items; switch and close tabs.
5. Follow a dependency, a dependent, the parent and a child.
6. Create a repository in an empty directory, with identity and prefix.
7. Set identity and initials; add, select and remove a remote.
8. Generate a protected key; copy its public key; select it.
9. Check host: cancel the passphrase prompt; retry and approve the host.
10. Delete a repository's folder; see it unavailable; remove the registration.
11. Make an operation pending; resume or abandon it from the operation area.
12. Confirm visible focus at every step and focus return after each dialog.
13. Browse with the platform folder chooser unavailable or failing (for
    example, a session with no file-chooser portal). Browse shows "The
    folder chooser could not be opened. Type the folder's path instead.",
    and the path field stays usable: type the path, and Add or Create
    goes on. Step 6 covers Create with a working chooser only.
14. Run the steps above at the default window size, 1280×840, as well as
    maximized: every region, the operation area and each settings page
    fit and scroll by keyboard. The automated view tests use 1440×1400,
    so they do not prove this layout.

Windows and macOS runs of the same list are G1 obligations (decision 3).

## Alternatives Rejected

- **A desktop library target or desktop modules in `src/lib.rs`.** Would put
  GPUI in the library, against `AGENTS.md`.
- **`gpui-kit` with `test-support` as a dev-dependency.** Dev-dependencies
  cannot be optional, so every headless test build would compile GPUI.
- **Running mutations on the background executor.** `SessionCredentials` is
  `&mut` and per process; executor threads would need a lock held across a
  blocking prompt, and ordering of user and launch jobs would be lost.
- **`ticket_readiness` for the Ready/Blocked list.** Drops nonconforming
  entries, which the desktop RFC requires to stay visible under filters.
- **A custom zeroizing input.** Rejected by decision 6 for its IME, paste and
  native verification cost.
- **Showing stored accessibility as the unavailable signal.** Never updated
  (`01M4EHGE2K`); decision 2.

## Risks

| Risk | Control |
| --- | --- |
| GPUI headless test platform does not run on Linux CI or locally | Task 1 proves it before any view work; stop and ask if not. |
| A library type is not `Send` | Task 2 compile-time assertions; a failure becomes a shared-library ticket. |
| The read executor is saturated by slow reads | Reads are short and bounded by the 250 ms lock wait; only the worker does long work. A read that still runs long is a library finding, recorded. |
| `01M4R19P94` is not merged in time | Part C starts only after it merges; Parts A and B proceed. |
| Kit `Tree`/`List` keyboard behavior differs from the bindings planned | Tests in Tasks 4 and 5 pin it; adjust bindings, not the library. |
| `--all-features` builds the shipped binary with test support | Release and smoke commands use `--features desktop`; noted in the plan's gate. |
| C1 edits the same manifest and workflow files | Merge main at work boundaries; neither branch touches the other's sources. |
| D1 is large | Three Parts with checkpoint reviews; a Part may be split by the product owner at its checkpoint. |
| The native Wayland portal is absent | Browse is disabled with its reason; the text path always works. |

## Review Record

Reviewed against the Wave, the governing RFCs, F1/F2 records, the open
defect tickets and the source at `f6a4c06`. Classifications follow
`review-cycle-docs`.

An independent reviewer then read all three documents against `AGENTS.md`,
the Wave, the RFCs, the library source and the GPUI Kit, GPUI Component and
`gpui-pre` 0.3.6 sources on 2026-10-18, by reading only. Its findings and
their corrections are the rows beginning "Independent review"; every one
was checked against source before it was applied.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| Wave D1, "too much in one Cycle" risk | D1 spans shell, discovery, relationships, onboarding, credentials and recovery. | Decision | Cycle decision 1: three Parts. | Checkpoint reviews in the plan. |
| `01M4EHGE2K`, `01M4R19P94`; Wave D1 exit "unavailable repositories" | Accessibility is never stored; a gone registration cannot be removed. | Decision | Cycle decision 2. | Unavailable-repository tests; Part C gate on the merge. |
| Test strategy; no GPUI harness; CI has no display | How keyboard and native evidence is produced. | Decision | Cycle decision 3. | Task 1 feasibility; native checklist. |
| F1 decision 7, no D6 worker | Lists only change when something refreshes. | Decision | Cycle decision 4. | Launch-refresh tests. |
| F2 resume binding; D4 owns sync | Which operation-area actions D1 offers. | Decision | Cycle decision 5. | Operation area tests. |
| Auth RFC zeroizing holder; Kit `SharedString` | Transient passphrase entry is not zeroizing. | Decision | Cycle decision 6, residual risk recorded. | Secrets tests. |
| `src/main.rs` | Child view created before `Root`, against `AGENTS.md`. | Settled | Fixed in Task 1 with the spike's pattern. | Startup test. |
| R1, `AGENTS.md` | Where desktop code lives. | Ruling | `src/desktop/`, declared only from `main.rs` — keeps GPUI out of the library — cost if wrong: move modules. | Task 1; CLI isolation script. |
| R2, Cargo features | Kit test support without compiling GPUI in headless tests. | Ruling | `desktop-test-support` feature enabling `gpui-kit?/test-support` — dev-deps cannot be optional — cost if wrong: one manifest line. | Task 1. |
| R3, runtime RFC "bounded worker capacity" | Thread model. | Ruling | Reads on the background executor, mutations and host observation on one worker owning the session — matches `&mut SessionCredentials` — cost if wrong: add a second local-only worker. | Task 2 tests. |
| R4, desktop RFC "tabs key on ULID" | A nonconforming entry has no ULID. | Ruling | `TabKey::Inspect{root, worktree, path}`, labeled nonconforming — the RFC's rule is about items with identity — cost if wrong: open nonconforming entries in a modal instead. | Tabs tests. |
| R5, F1 graph read | `ticket_readiness` omits nonconforming entries. | Ruling | Use `list_tickets` with `readiness` — keeps the RFC's visibility rule — cost if wrong: none. | Filter test with a nonconforming ticket. |
| R6, `busy` after 250 ms | How a held lock is presented. | Ruling | Three retries at 100/200/400 ms, then Busy with Retry — bounded, never a loop — cost if wrong: tune numbers. | Busy test. |
| R7, F2 `prepare` | Where `prepare` runs. | Ruling | As a read on the executor — no session, lease or network — cost if wrong: route through the worker. | Confirmation tests. |
| R8, MH-NFR-005 | Key map. | Ruling | The table in Layout And Keyboard — platform `secondary` modifier — cost if wrong: rebind in one file. | Keyboard tests; native checklist. |
| R9, runtime RFC shutdown; D6; GPUI 200 ms quit budget | Quit while a job runs. | Ruling | Intercept the Quit action and the window close request before GPUI quits; wait with "Finishing", or Quit now as an interruption — D6 refines — cost if wrong: D6 replaces it. | Quit and close tests with a held job. |
| R10, Linux portals | Native file dialog may be unavailable. | Ruling | Text path always; Browse optional and disabled with reason — cost if wrong: none. | Manual checklist step 6. |
| R11, transport cannot initialize in bin tests | Host prompt UI tests. | Ruling | No desktop test reaches a network. Automated: provider, dialog and preview rendering; a `HostApprove`/`HostReplace` preview needs no connection (`bind/host.rs`). Host success and unlock-on-observe: native checklist step 9 and F2's SSH tests — cost if wrong: a desktop-only host regression is caught by the checklist, not CI. | Task 16. |
| R12, `01M4W5SC` | Operation rows have no item. | Ruling | Show command and scope — honest — cost if wrong: none once the defect merges. | Operation area test. |
| R13, read-only discussion | Wave D1 does not list comments; the desktop RFC's item view includes discussion. | Ruling | Show comments read-only; submission stays D4 — inspection only — cost if wrong: remove one panel. | Item view test. |
| R14, key generation retry | The passphrase is not part of the replay digest. | Settled | Retry re-prompts; same request ID — `mutation/bind/key.rs` says two requests differing only in passphrase are the same request. | Key generation retry test. |
| R15, CI | Bin unit tests are not in the native workflow's `--test` list. | Ruling | Add `--bin manyhands` with `--features desktop-test-support` to the workflow's test step — native CI runs after merge — cost if wrong: desktop tests unrun natively until fixed. | Task 17. |
| Project convention (F1 decision 3) | The worker must wake the UI; GPUI exposes no async channel. | Decision | Cycle decision 7, approved 2026-10-19: `futures = { version = "0.3", optional = true }` enabled by `desktop`; `Cargo.lock` gains only the direct edge. | Task 1 manifest change. |
| Independent review 1, blocking | `cargo tree` has no `--bin`; under `sh` without `pipefail` the check passed even when `cargo tree` failed. | Settled | Package-level, no-feature graph written to a file; a failing `cargo tree` fails the script. Contract 8 reworded. | Task 1 script; Task 17 gate. |
| Independent review 2, blocking | The test scheduler runs background work on the test thread, so a blocking read gate deadlocks; parking panics unless allowed. | Settled | Async read gates; job delays on the worker thread only; `allow_parking` and the 15 s cap stated; feasibility test includes a worker round trip. | Task 1, Task 2. |
| Independent review 3 | `AGENTS.md` requires merging main into ticket branches, never rebasing. | Settled | Baseline and checkpoints merge; `merge-tree` check before review-ready. The planning rebase rewrote no published history. | Plan baseline, Part B checkpoint, Task 17. |
| Independent review 4 | `RefreshFailure` is promotion-only; a seam cannot fail a call without fabricating. | Settled | `BeforeIndexTransactionCommit`; the seam only delays; library refresh and observation hooks for delayed scans. | Proof map; Task 7. |
| Independent review 5 | Transport cannot initialize in bin tests, so no automated real host or unlock-on-observe. | Settled | Ruling R11 revised; Cycle acceptance row says which evidence is checklist-only. | Task 16; native checklist step 9. |
| Independent review 6 | Prompt deduplication across concurrent requests cannot occur with one serial worker. | Settled | Deduplication is structural; test cached-passphrase reuse instead; no per-waiter secret copies. | Task 11. |
| Independent review 7 | GPUI gives quit handlers 200 ms; no Quit action; close not handled; DNS unbounded. | Settled | Quit action and window close intercepted; Quit now always available. | Task 3, Task 11. |
| Independent review 8 | Pending-sync row could be skipped; track rule 4 had no second-actor test. | Settled | Pending-sync presentation proven at model level and the Cycle row says so; second-actor tests added. | Tasks 4, 7. |
| Independent review 9 | Startup failure offered Rebuild with no service. | Settled | Message, Retry open, Quit. | Task 1. |
| Independent review 10–13 | Glob import shadows `#[test]`; masked-input builder and no mask toggle; `prepare` returns an envelope; NUL in passphrase; one test filter per `cargo test`; key store through the backend; `repository_inaccessible`; tabs closed on removal; duplicate-context recovery view; rename rather than delete; `futures` still conditional. | Settled | Applied in the design and plan. | As cited in each section. |
| R16, Wave D1 "viewing does not provision a context" | Copy to clipboard and opening are the only effects of browsing. | Settled | No browse path calls the mutation boundary. | No-side-effect test: before/after snapshot of the fixture repository while browsing. |
| R-T17a, Kit `Root` Tab cycling (Task 17, 2026-10-20) | This section said Tab and shift-Tab move within a region; the Kit cycles focus over the whole window. | Ruling | The design is amended to what is built: Tab moves window-wide in layout order, and `F6`, `shift-F6` and `secondary-1` to `secondary-4` move between regions. A region-bounded Tab would need a focus trap per region, which the Kit keeps for dialogs — cost if wrong: a keyboard user Tabs out of a region at its last control. | `keyboard_tests.rs`; `tabs_tests.rs::the_tab_keys_work_from_inside_the_source`. |
