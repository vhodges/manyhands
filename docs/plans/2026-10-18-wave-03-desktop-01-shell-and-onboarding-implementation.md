---
title: "Wave 03 D1 Desktop Shell, Onboarding And Discovery Implementation Plan"
date: 2026-10-18
status: "implemented"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M58Y0H04KJKZWHHJSGZCM7E6"
---

# Desktop Shell, Onboarding And Discovery Implementation Plan

> **For agentic workers:** Do not begin until the product owner approves the
> Cycle, design and this plan and explicitly authorizes implementation. Then
> use `implementing-a-cycle` and the selected execution method task by task,
> re-read the design, repeat the ticket preflight, and record every
> checkpoint on ticket `01M4CC0VMY5RBMP70QVBXWXM6A`. Part C additionally
> waits for `01M4R19P943QZKHHKRXV2D2TP2` to be merged to main.

**Goal:** A keyboard-usable desktop that browses real repositories through
the F1 reads and administers repositories, remotes, identity, keys and hosts
through the F2 mutation boundary.

**Architecture:** Desktop code lives in `src/desktop/`, declared only from
`src/main.rs`. Reads run on GPUI's background executor over a shared
`RepositoryService`; mutations and host observation run on one desktop-owned
worker thread that owns the process's `SessionCredentials`. GPUI-free view
models hold all state; GPUI Kit views render it. Every completion carries a
stamp and is dropped if its view has moved on.

**Tech stack:** Rust 2024; `gpui-kit` 0.6.6 (GPUI `gpui-pre` 0.3.6) through
`gpui_kit::*` only; the headless library; `futures` 0.3 (desktop-only,
decision 7); Devenv.

**Spec:** [Cycle](../Cycles/wave-03-desktop-01-shell-and-onboarding.md) and
[design](2026-10-18-wave-03-desktop-01-shell-and-onboarding-design.md).
Executors read both. Section names below refer to the design.

**Status:** Approved by the product owner on 2026-10-19, together with the
Cycle and design, for subagent-driven execution. Approval authorizes the
Rust changes in this plan. It does not authorize publication, merge, ticket
closure or worktree cleanup. Part C still waits for `01M4R19P94`.

## Global Constraints

- Work only in ticket `01M4CC0VMY5RBMP70QVBXWXM6A`'s worktree and branch.
  Preserve unrelated edits in main and every other worktree.
- Run every Rust command as `devenv shell -- cargo …`.
- Do not change `src/repository.rs`, anything under `src/repository/`,
  `src/canonical.rs`, `src/results.rs`, `src/runtime.rs` or `src/lib.rs`
  (track rule 5). A missing or wrong library capability stops the task: file
  a shared-library ticket with `file-ticket` and ask the product owner.
- GPUI only through `gpui_kit::*`; no direct `gpui` dependency. Initialize
  the transport first in `main`, `gpui_kit::init(cx)` inside `app.run`,
  `Root` as each window's first entity.
- Dependencies: `gpui-kit`'s `test-support` through the new
  `desktop-test-support` feature; `futures = "0.3"` optional under `desktop`
  (decision 7, approved). Nothing else. Commit `Cargo.lock` with
  manifest changes.
- No desktop module, test or production, uses `use gpui_kit::*`; import
  names explicitly (`gpui-pre` re-exports a `test` attribute that a glob
  would let shadow `#[test]`).
- `cargo test` takes one positional filter. Every Verify command below uses
  one filter, the module path `desktop::…` given.
- A view test that waits on the real worker thread calls
  `cx.executor().allow_parking()` and finishes within the scheduler's 15 s
  limit. Read delays are async gates, never blocking ones.
- No desktop test initializes the transport or reaches a network.
- Merge `origin/main` at work boundaries; never rebase this branch.
- Every desktop test that touches Git calls the isolated Git home first
  (`test_support::git_home::isolated()`), builds repositories through the
  library, and uses temporary data directories and `KeyStore::for_home`.
- No view reads SQLite, Git or files to find items. No row is fabricated in
  production code. The `DelayingBackend` only delays or fails a real call.
- Never format a `ReadError`, `RepositoryError`, I/O or `git2` error, URL,
  `Intent` or passphrase with `Debug` or `Display` into UI text. Show
  `ResultCode::message()` and the envelope `message`.
- Models that may hold user input for a secret do not derive `Debug`.
- Write each behavior's test before its code. Commit per task with a
  conventional message and the `Co-Authored-By` trailer.
- Release and smoke runs use `--features desktop`, never `--all-features`.
- Test runs: within a task, run only the tests its Verify line names. The
  full suite runs at the Part checkpoints and in Task 17. This follows the
  owner's standing rule, applied to C1 on 2026-10-19.
- Implementers never dispatch subagents.

## Review Focus

1. **A completion lands on the wrong view.** The user switches repository,
   filter or tab while a read or job is in flight. The old result must never
   replace the new view. Pinned in Tasks 2, 4, 5 and 8.
2. **A repository folder disappears while it is selected or has open tabs.**
   The selection and tabs show unavailable or `item_not_found` with Retry;
   nothing panics or loops. Pinned in Tasks 4 and 8.
3. **A filter hides a nonconforming entry.** Open, Closed, Ready and Blocked
   must all keep it. Pinned in Tasks 5 and 9.
4. **A confirmed action is submitted twice** (Enter pressed twice, or Confirm
   after a retry). One request ID, one effect. Pinned in Tasks 7 and 12.
5. **A passphrase leaks** into an operation entry, notification, request
   record or retained input. Pinned in Tasks 11 and 15.

## File Map And Dependency Order

| Path | Task | Change |
| --- | --- | --- |
| `Cargo.toml`, `Cargo.lock` | 1 | `desktop-test-support` feature; `futures` (decision 7). |
| `src/main.rs` | 1 | Transport init then `desktop::run()`. |
| `src/desktop/mod.rs`, `keys.rs` | 1, 4 | Startup, actions, bindings. |
| `src/desktop/test_support/{mod,git_home,fixtures}.rs` | 1 | Test isolation and real-repository builders. |
| `scripts/check-cli-no-gpui.sh` | 1 | CLI isolation check. |
| `src/desktop/services/{mod,reads}.rs`, `model/stamp.rs` | 2 | Backend seam, reads, busy retry, stamps, `Send` assertions. |
| `src/desktop/services/{worker,requests}.rs`, `model/operations.rs` | 3 | Worker, intents, requests, progress, cancellation, quit. |
| `src/desktop/model/{navigation,list_state}.rs`, `view/{workspace,sidebar}.rs` | 4 | Shell, repository selector, list states, regions. |
| `src/desktop/model/{documents,tickets}.rs`, `view/{documents,tickets}.rs` | 5 | Tree, list, closure filter. |
| `src/desktop/model/item.rs`, `view/item.rs` | 6 | Read-only item view and inspection. |
| `src/desktop/view/{confirm,operations}.rs` | 7 | Confirmation dialog, refresh/rebuild, operation area. |
| `src/desktop/model/tabs.rs`, `view/tabs.rs` | 8 | Tabs. |
| `src/desktop/model/find.rs`, `view/find.rs`, ticket filter additions | 9 | Short codes, find, ready/blocked. |
| `src/desktop/model/relationships.rs`, `view/relationships.rs` | 10 | Relationships panel. |
| `src/desktop/services/prompts.rs`, `view/passphrase.rs` | 11 | Provider bridge and prompt. |
| `src/desktop/view/settings/{mod,repositories}.rs` | 12, 13 | Onboarding and repository settings. |
| `src/desktop/view/settings/remotes.rs` | 14 | Remotes. |
| `src/desktop/view/settings/keys.rs` | 15 | Keys. |
| `src/desktop/view/settings/hosts.rs` | 16 | Hosts. |
| `.github/workflows/build.yml` | 17 | Desktop bin tests and CLI check. |

Tasks run in order. Each task's tests sit beside its module as
`*_tests.rs`.

## Baseline And Dependency Checkpoint

Before Task 1:

```sh
git fetch origin main
git merge origin/main          # AGENTS.md: merge, never rebase a ticket branch
git merge-base --is-ancestor origin/main HEAD
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo run --locked --bin manyhands-cli
```

Record base, both heads and each result on the ticket. A failure here is
resolved or explicitly accepted by the product owner before Task 1.

Re-check the design's Library Surface Audit against every change to
`src/repository*` since `f6a4c06`
(`git diff --stat f6a4c06 origin/main -- src/repository.rs src/repository src/results.rs`).
Update the design before coding if a cited signature moved.

Decision 7 (`futures` as a direct, desktop-only dependency) was approved on
2026-10-19; Task 1 adds it.

---

# Part A: Shell, Services, Browsing, Refresh And Operation Area

## Task 1: Feature, Skeleton, Startup And Test Harness

**Files:** `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/desktop/mod.rs`,
`src/desktop/mod_tests.rs`, `src/desktop/test_support/{mod,git_home,fixtures}.rs`,
`scripts/check-cli-no-gpui.sh`.

**Interfaces:**
- Produces `pub(crate) fn run()`; `fn open_main_window(services: Services, cx: &mut App)`;
  `#[cfg(test)] fn open_test_window(cx: &mut App) -> WindowHandle<Root>`
  (opens the main window over a fixture's services)
  (Task 2 supplies `Services`; until then the window holds an empty
  `Workspace` placeholder view);
- `test_support::git_home::isolated()` (the file is
  `#[path = "../../../tests/support/git_home.rs"] mod git_home;`);
- `test_support::fixtures::{Fixture, Fixture::new() -> Fixture,
  fixture.service() -> &RepositoryService, fixture.data_dir(), fixture.home(),
  fixture.create_repository(name) -> PathBuf,
  fixture.commit_file(root, rel_path, bytes), fixture.refresh(root)}`;
  `create_repository` runs `prepare`+`execute` of `RepoCreate` with a fixed
  identity; `commit_file` writes and commits with `git2` for states no local
  operation produces (design, Testing, Harness).

**Steps:**

- [ ] Add to `Cargo.toml`:

  ```toml
  [features]
  default = []
  desktop = ["dep:gpui-kit", "dep:futures"]
  desktop-test-support = ["desktop", "gpui-kit?/test-support"]
  editor-probe = ["desktop", "dep:zorite-editor"]

  [dependencies]
  futures = { version = "0.3", optional = true }
  ```

  Run `devenv shell -- cargo check --all-features --locked`. If `--locked`
  refuses, run `cargo update -p futures --precise 0.3.34` (already locked)
  and confirm `git diff Cargo.lock` adds only direct-dependency edges.
- [ ] **Feasibility test first** (`src/desktop/mod_tests.rs`):

  ```rust
  #[gpui_kit::test]
  fn root_is_the_first_entity_of_the_main_window(cx: &mut gpui_kit::TestAppContext) {
      cx.update(|cx| gpui_kit::init(cx));
      let window = cx.update(|cx| super::open_test_window(cx));
      // Root's entity must be created before the workspace's.
      let (root_id, workspace_id) = window
          .update(cx, |root, _, cx| (cx.entity_id(), root.view().entity_id()))
          .unwrap();
      assert!(root_id < workspace_id);
  }
  ```

  Use the accessors `gpui-component` 0.6.6's `Root` actually has for its
  child view; adjust the snippet, not the claim.
- [ ] **Worker round trip** (needs Task 3's worker; in this task, a minimal
  thread that runs one `IndexRefresh` through `execute` and sends `Finished`
  over the channel): with `cx.executor().allow_parking()`, the event
  reaches a model within the test.

  Run `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::`.
  **If the window cannot open on this host with no display, or the round
  trip cannot complete under the test scheduler, stop and report to the
  product owner before Task 2** (design, Testing, feasibility gap).
- [ ] Implement `run()` and `open_main_window` per design Startup: outer
  `cx.new` creates `Root`, workspace inside it; workspace render includes the
  three Root layers. `main.rs` keeps the transport init as its first
  statement, drops its `use gpui_kit::*`, and calls `desktop::run()`.
- [ ] Startup failure: `open_startup_failure_window` shows the fixed message,
  Retry (calls `open_default` again) and Quit. Test it with a data directory
  that cannot be opened (a regular file in its place).
- [ ] Test: opening a dialog through `window.open_dialog` renders a dialog
  element (proves the dialog layer is rendered).
- [ ] Write `scripts/check-cli-no-gpui.sh`:

  ```sh
  #!/bin/sh
  # With no features, the package's normal dependency graph is the CLI's.
  set -eu
  out=$(mktemp)
  trap 'rm -f "$out"' EXIT
  cargo tree --locked -e normal --no-default-features -p manyhands --prefix none > "$out"
  if grep -E '^gpui' "$out"; then
    echo "the no-feature build depends on a GPUI package" >&2
    exit 1
  fi
  ```

  Run it through `devenv shell -- sh scripts/check-cli-no-gpui.sh`; it
  passes. Then prove it can fail: run it once with `--features desktop`
  added to the `cargo tree` line (locally, not committed) and see it exit 1,
  and once with a bad flag and see `set -e` stop it.
- [ ] Fixture test: `Fixture::new()` then `create_repository("a")` registers
  one repository and `list_repositories()` returns it.

**Verify:**
`devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands`,
`devenv shell -- cargo run --locked --bin manyhands-cli`.

**Commit:** `feat: start the desktop with Root first and a headless test harness`.

**Checkpoint:** ticket comment with the feasibility result and the
`Cargo.lock` diff summary.

## Task 2: Backend Seam, Reads And Stamps

**Files:** `src/desktop/services/{mod,reads}.rs`, `services/reads_tests.rs`,
`src/desktop/model/{mod,stamp}.rs`, `model/stamp_tests.rs`,
`src/desktop/test_support/delaying.rs`.

**Interfaces:**
- `pub(crate) trait Backend: Send + Sync + 'static { fn service(&self) -> &RepositoryService; fn key_store(&self) -> KeyStore; fn read_gate(&self, kind: ReadKind) -> Option<ReadGate> { None } fn before_job(&self, kind: JobKind) {} }`
- `pub(crate) type ReadGate = futures::channel::oneshot::Receiver<()>;`
  awaited inside the read's future before the blocking call.
- `pub(crate) struct LibraryBackend { service: RepositoryService }` with
  `key_store()` returning `KeyStore::for_current_user()` (its error becomes
  a fixed-message failure shown on the Keys page).
- `#[derive(Clone)] pub(crate) struct Services { backend: Arc<dyn Backend>, worker: WorkerHandle /* Task 3 */ }`
- `pub(crate) enum ReadKind { Repositories, Resolve, Documents, Folders, Tickets, Item, Comments, Dependencies, Children, FindSlug, Operations, IndexStatus, Remotes, Identity, Keys, Hosts, Prepare }`
- `pub(crate) enum ReadResult<T> { Ok(T), Busy, Failed(ReadFailure) }` where
  `ReadFailure { code: ResultCode, recovery: Vec<RecoveryAction> }` is built
  from `ReadError::code()` and its recovery, never from its text.
- `impl Services { pub fn read<V: 'static, T: Send + 'static>(&self, kind: ReadKind, stamp: Stamp, f: impl Fn(&RepositoryService) -> Result<T, ReadError> + Send + Sync + 'static, cx: &mut Context<V>, apply: impl FnOnce(&mut V, Stamp, ReadResult<T>, &mut Context<V>) + 'static) -> Task<()> }`
- `#[derive(Clone, Copy, PartialEq, Eq, Hash)] pub(crate) enum ViewScope { Repositories, Documents(RepoKey), Tickets(RepoKey), Tab(TabId), Settings(SettingsPage), Operations(RepoKey) }`
- `pub(crate) struct Stamp { pub scope: ViewScope, pub generation: u64 }`
- `pub(crate) struct Generations` with `next(scope) -> Stamp` and `is_current(&Stamp) -> bool`.
- `RepoKey` is the registration root as the list returned it (`String`).
- `impl Services { pub fn prepare<V: 'static>(&self, stamp: Stamp, mutation: Mutation, cx: &mut Context<V>, apply: impl FnOnce(&mut V, Stamp, Envelope<PrepareDataDto>, &mut Context<V>) + 'static) -> Task<()> }`
  (`prepare` returns an envelope, so it is not a `read`).
- `test_support::DelayingBackend` over a real service and a fixture home:
  `hold_read(kind) -> oneshot::Sender<()>` (send to release) and
  `hold_job(kind) -> JobGate` (a blocking gate on the worker thread,
  `release()`). It only delays. Failures in tests come from the library's
  failure points and hooks (design, The seam).

**Tests first:**

- Compile-time assertions in `reads_tests.rs`:
  `fn send<T: Send + 'static>() {}` for `RepositoryService`, `Mutation`,
  `MutationCall`, `Envelope<MutationDataDto>`, `Envelope<PrepareDataDto>`,
  `Envelope<HostObservationDto>`, `RepositoryListDto`, `ItemListDto`,
  `ItemDto`, `CommentListDto`, `DependencyTreeDto`, `OperationListDto`,
  `ReadError`, `ProgressEvent`. **A failing assertion stops the task**
  (Global Constraints).
- `Generations`: a stamp is current until `next` is called for its scope;
  other scopes are unaffected.
- `read` delivers `Ok` from a real fixture `list_repositories`.
- `read` with the cache write lock held
  (`RepositoryService::hold_lease_for_testing(root, data_dir, LeaseKind::CacheWrite)`)
  delivers `Busy` after its three retries; released before the second retry,
  it delivers `Ok`.
- A stale stamp: the test's `apply` checks `is_current` and records a drop;
  `hold_read` holds read 1, read 2 is issued under a new generation and
  completes (`cx.run_until_parked()`), read 1's sender fires; only read 2 is
  applied. No thread blocks in this test.

**Verify:** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::`.

**Commit:** `feat: run desktop reads off the UI thread and drop stale results`.

## Task 3: Mutation Worker, Requests And Cancellation

**Files:** `src/desktop/services/{worker,requests}.rs`, their `_tests.rs`,
`src/desktop/model/operations.rs`, `model/operations_tests.rs`.

**Interfaces:**
- `pub(crate) enum Lane { User, Background }`
- `pub(crate) enum Intent { RepoCreate{root, primary_branch, identity: Option<(String, String)>, slug_prefix: Option<String>}, RepoEnable{..same..}, RepoRemove{root}, RepoIdentitySet{root, name, email, initials: InitialsChoice}, RemoteAdd{root, name, url}, RemoteRemove{root, name}, RemoteSelect{root, name: Option<String>}, IndexRefresh{root}, IndexRebuild{root}, KeyGenerate{label, protected: bool}, KeyImport{label, private_key_path, public_key_path}, KeySelect{id}, KeyClear, KeyRemove{id}, KeyDelete{id}, HostApprove{root, authority, algorithm, fingerprint}, HostReplace{root, authority, algorithm, fingerprint, old_fingerprint}, OperationResume{root, operation_id}, OperationAbandon{root, operation_id} }`
  — no `Debug` derive.
- `impl Intent { fn build(&self, secret: Option<SecretPassphrase>, store: &KeyStore) -> Result<Mutation, MissingSecret>; fn confirmed(&self) -> bool; fn label(&self) -> &'static str }`.
  `build` maps each variant to the design's `Mutation` variant;
  `KeyGenerate{protected: true}` without a secret is `Err(MissingSecret)`,
  which the worker reports as a prompt to enter the passphrase, never as a
  library call.
- `confirmed()` must equal the library's own list; a test compares it with
  `prepare`'s `not_confirmable` answer for every variant.
- `pub(crate) struct DesktopRequestId(u64)`;
  `pub(crate) struct DesktopRequest { id, request_id: RequestId, intent: Intent, confirmation: Option<ConfirmationId>, lane: Lane, state: RequestState }`;
  `pub(crate) enum RequestState { Queued, Running { stage: Option<ProgressStage>, accepted: bool }, Finished(Outcome, ResultCode, Effects, Vec<RecoveryAction>, String /* envelope message */), Dismissed }`.
- `pub(crate) struct WorkerHandle` with
  `submit(intent: Intent, lane: Lane, confirmation: Option<ConfirmationId>, secret: Option<SecretPassphrase>) -> DesktopRequestId`,
  `retry(id, secret: Option<SecretPassphrase>)` (same `request_id`),
  `cancel(id) -> CancelResult { Removed, Signalled, TooLate }`,
  `observe_host(root, authority: Option<SshAuthority>) -> DesktopRequestId`,
  `quit() -> QuitState { Idle, Finishing(DesktopRequestId) }`.
- Events: `pub(crate) enum WorkerEvent { Started(DesktopRequestId), Progress(DesktopRequestId, ProgressEvent), Finished(DesktopRequestId, FinishedEnvelope), HostObserved(DesktopRequestId, Envelope<HostObservationDto>), Prompt(PromptRequest) }`,
  delivered on `futures::channel::mpsc::UnboundedReceiver`.
- `model::Operations` holds `Vec<DesktopRequest>` (newest first) and
  `BTreeMap<RepoKey, ListState<OperationListDto>>`, applies `WorkerEvent`s.
- The worker's provider is a placeholder that returns
  `PassphraseResponse::Unavailable` until Task 11.

**Tests first (real fixtures, `DelayingBackend` for ordering):**

- A user job submitted while a background job is queued runs before it; a
  running job is not preempted.
- `IndexRefresh` through the worker returns `success` and the request ends
  `Finished(Success, …)`; its `Progress` events arrive in order and contain
  no path outside the fixture root.
- Cancel of a queued job is `Removed` and the library never sees it
  (`before_job` never called for it).
- Cancel before acceptance: a held `before_job` gate, cancel, release: the
  envelope is `cancelled` and `list_operations` shows no pending row.
- Cancel after `accepted` returns `TooLate` and the state shows
  `accepted: true`.
- Retry of a finished request uses the same `RequestId` and the library
  answers from its record (no second effect: refresh count unchanged
  via `index_status`).
- `Intent::KeyGenerate{protected: true}.build(None, …)` is
  `Err(MissingSecret)`; the library sees no call.
- For every `Intent` variant, `confirmed()` agrees with `prepare` on a
  fixture (a confirmed command previews; any other answers
  `not_confirmable`).
- `quit()` while a job is held returns `Finishing`; after release, `Idle`.
- The workspace's `Quit` action and the window's close request
  (`Window::on_window_should_close`) go through one handler: with no job it
  allows; with a held job it refuses, shows "Finishing" and a Quit now
  control (`QuitNow` action), and allows once the job returns. Add `Quit`
  and `QuitNow` to `keys.rs`'s `actions!` list (`secondary-q` for Quit).
- Tests that wait on worker events call `cx.executor().allow_parking()`.

**Verify:** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::`.

**Commit:** `feat: run desktop mutations on one worker with retry and cancellation`.

## Task 4: Shell, Repository Selector And List States

**Files:** `src/desktop/model/{navigation,list_state}.rs` and tests,
`src/desktop/view/{mod,workspace,sidebar}.rs` and `view/workspace_tests.rs`,
`src/desktop/keys.rs`.

**Interfaces:**
- `pub(crate) enum ListState<T> { Loading, Ready { rows: T, complete: bool, index: IndexStateDto }, Busy, Failed(ReadFailure) }`
  with `fn label(&self) -> &'static str` returning distinct text for loading,
  empty, never-refreshed, stale, incomplete, busy, failed and unavailable
  index.
- `pub(crate) enum Resolution { Unknown, Resolved(ResolvedRepository), Unavailable(ResultCode) }`
- `pub(crate) struct RepositoryEntry { summary: RepositorySummaryDto, resolution: Resolution }`
- `pub(crate) struct Navigation { repositories: ListState<Vec<RepositoryEntry>>, selected: Option<RepoKey>, pane: Pane }`
  with `select(key) -> Stamp`, `retry_resolution(key) -> Stamp`.
- `keys.rs`: `gpui_kit::actions!(manyhands, [NextRegion, PreviousRegion, FocusRepositories, FocusNavigation, FocusContent, FocusOperations, Find, NextTab, PreviousTab, CloseTab, Refresh, Rebuild, OpenSettings, Quit, QuitNow])`
  and `bind(cx)` with the design's table.
- `view::Workspace` (entity) renders sidebar, navigation, content, operation
  area regions, each `track_focus` with a `key_context`.

**Tests first:**

- Model: each `ListState` variant and `IndexStateDto` state yields its own
  label; empty `Ready` differs from never-refreshed `Ready`.
- View: the fixture's two repositories are listed with names; selecting one
  resolves it and shows its configuration and index state.
- View: after renaming the selected repository's directory away, Retry
  shows Unavailable with `invalid_path`'s message and the Remove control;
  renaming it back and Retry resolves (**Review Focus 2**). Unavailable is
  also shown for `not_repository`, `repository_inaccessible` and
  `repository_not_registered` (model test over each code).
- Second actor (track rule 4): a second `RepositoryService` on the same data
  directory removes the selected repository's registration; the desktop's
  next read shows `repository_not_registered` as Unavailable, not the old
  rows.
- View: selecting repository A, then B while A's resolution is held, then
  releasing A, leaves B selected and shown (**Review Focus 1**).
- View: `F6`/`shift-F6` cycle regions; `secondary-1..4` focus each; the
  focused region has a visible focus style.
- No-side-effect: snapshot the fixture repository (refs, worktrees, status,
  `.git/config` bytes) before and after selecting and browsing; equal.

**Verify:** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::`.

**Commit:** `feat: add the desktop shell and repository selector`.

## Task 5: Document Tree And Ticket List

**Files:** `src/desktop/model/{documents,tickets}.rs` and tests,
`src/desktop/view/{documents,tickets}.rs` and tests.

**Interfaces:**
- `pub(crate) enum TreeNode { Folder { path: String, empty: bool, children: Vec<TreeNode> }, Document(ItemDto) }`;
  `fn build_tree(folders: &DocumentFolderListDto, documents: &ItemListDto) -> Vec<TreeNode>`.
- `pub(crate) struct TicketQuery { closure: ClosureFilter /* default Open */, readiness: Option<ReadinessFilter> }`
  with `fn to_filter(&self) -> TicketFilter`.
- `pub(crate) struct TicketRow` derived from `ItemDto`: `slug`, `title`,
  `ticket_type`, `status`, `lifecycle: Lifecycle { Open, Closed }`,
  `changed_at`, `nonconforming: Option<ProblemCode>`.
- Activating a row calls `Workspace::open_item(OpenTarget)` where
  `OpenTarget::{Item{repo, kind, id}, Inspect{repo, worktree, path}}`; in
  Part A this shows the item in the content region (Task 6); Task 8 turns it
  into tabs.

**Tests first:**

- `build_tree`: empty folder shown; nested folders; nonconforming document
  placed in its folder; folders before documents.
- `TicketQuery::default().to_filter().closure == ClosureFilter::Open`.
- Fixture tickets: open with status `closed`, lifecycle-closed (a committed
  canonical file carrying `closed_at` and `closed_by`, as
  `tests/support/items.rs` writes it), plain open, and a nonconforming file
  under `.manyhands/tickets/`; then a library refresh. Under Open: the status-`closed` ticket and the
  plain ticket and the nonconforming entry; under Closed: the closed ticket
  and the nonconforming entry; under All: all four (**Review Focus 3**).
- Visible rows equal the rows of the same library call (no fabrication).
- Changing the filter while the previous list is held drops the old result.
- Keyboard: arrows move in the tree and list; Enter opens.

**Verify:** `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::`.

**Commit:** `feat: browse documents and tickets with the closure filter`.

## Task 6: Read-Only Item View

**Files:** `src/desktop/model/item.rs` and tests, `src/desktop/view/item.rs`
and tests.

**Interfaces:**
- `pub(crate) struct ItemView { target: OpenTarget, item: ListState<ItemDto>, comments: ListState<CommentListDto> }`
  with `fn load(&mut self, services, cx) -> Stamp`.
- `fn provenance_label(ctx: &ItemContextDto) -> String` ("Primary",
  "Active item worktree", "Unverified" plus branch and worktree).
- Unavailable actions: a `const LATER: [&str; 6] = ["Edit", "Comment", "Sync", "Approve and merge", "Close ticket", "Repair"]`
  rendered as disabled buttons with "Available in a later release".

**Tests first:**

- Ticket in primary shows Primary; a ticket with its own item worktree
  (created by a library `TicketSave` that provisions the context) shows
  Active with branch and worktree.
- Lifecycle closure and project status shown as separate fields.
- Unknown metadata listed; a `null` unknown value labeled not representable.
- Source shown verbatim; non-UTF-8 file says so.
- Comments indented by depth in list order; a comment without author shows
  "unknown author".
- Nonconforming entry opens by path and shows its problem code and the
  disabled Repair action.
- An item carrying a context problem (fixture: the duplicate-context recipe
  of the library's discovery tests, e.g. two item worktrees for one ID, via
  `commit_file` and a refresh) shows the recovery view with the problem
  codes and offers no choice of context.
- `item_not_found` after the file is deleted and the index refreshed shows
  Refresh.
- Opening an item writes nothing (snapshot equality, as Task 4).

**Verify:** as Task 5.

**Commit:** `feat: show a read-only item with its provenance and discussion`.

## Task 7: Confirmation Dialog, Index Maintenance And Operation Area

**Files:** `src/desktop/view/{confirm,operations}.rs` and tests,
`src/desktop/model/operations.rs` (extend), workspace wiring.

**Interfaces:**
- `pub(crate) fn confirm(intent: Intent, origin: FocusHandle, services: &Services, window: &mut Window, cx: &mut App)`:
  runs `prepare(&intent.build(..))` as a `ReadKind::Prepare` read, opens a
  dialog rendering `PreviewDto` (or `HostReplacement`), on Confirm calls
  `WorkerHandle::submit(intent, Lane::User, Some(confirmation_id), secret)`
  once and disables Confirm, on Escape/Cancel closes and focuses `origin`.
- On a finished envelope with `confirmation_expired`, `confirmation_used`,
  `confirmation_mismatch`, `confirmation_not_found` or `external_change`:
  show the message and "Prepare again", which prepares under the same
  `DesktopRequest`.
- Launch: after the repository list loads, submit one
  `Intent::IndexRefresh` on `Lane::Background` per entry.
- `Refresh`/`Rebuild` actions submit on `Lane::User` for the selected
  repository.
- Operation area: "This session" and "Pending in repositories" per design;
  recovery-action-to-control mapping per design Operation Area.

**Tests first:**

- Launch with two repositories submits two background refreshes; a manual
  Refresh submitted meanwhile runs first.
- A failed refresh and a failed rebuild
  (`RepositoryService::open_at_with_failure_point_for_testing` with
  `FailurePoint::BeforeIndexTransactionCommit`; reopen the service with
  `open_at` before the retry, as library tests do) stay in the operation
  area until dismissed; the list shows its stale state; Retry under the same
  request succeeds.
- A delayed refresh (`set_refresh_persist_hook_for_testing` blocking on a
  gate the test releases; it runs on the worker thread): while held, the
  user switches repository and opens an item; after release the list
  updates.
- Second actor: while the desktop's refresh job is queued, another process
  (a child of the test binary) holds the repository lease
  (`hold_lease_for_testing(LeaseKind::Repository)`); the job's typed answer
  is shown in the operation area and nothing reports success.
- A pending local operation (fixture: reuse the recipe of
  `tests/mutation_confirmation/abandon.rs` and
  `tests/journal_rejection.rs::an_abandoned_operation_no_longer_blocks_the_repository_and_nothing_is_touched`,
  which leave a pending local row through a failure point) is listed after
  reopening the service; Abandon opens the confirmation dialog; Confirm abandons it
  and the row disappears after reload.
- Resume offered only when `next_action` is resume and the command is local
  (view test over a real pending local row). A synchronization row shows
  Resume disabled: a **model** test over an `OperationDto` built in the test,
  because no local recipe makes a pending synchronization without SSH (the
  Cycle's acceptance row says so). Production code never builds one.
- Second actor: another `RepositoryService` abandons the pending row the
  desktop shows; the desktop's own Abandon then answers its typed result
  and the row disappears on reload.
- Pressing Enter twice on Confirm submits one request; the second press does
  nothing (**Review Focus 4**).
- Escape closes the dialog with no request; focus returns to the Abandon
  control.
- A stale preview: prepare Abandon, change the state (resume the operation
  through the library directly), Confirm: `external_change` or
  `confirmation_*` shown with "Prepare again".

**Verify:** as Task 5.

**Commit:** `feat: add confirmation, index refresh and the operation area`.

## Part A Checkpoint

Run the four required checks, the CLI smoke test, the CLI isolation script
and a desktop smoke run on a display. Request an independent review of Tasks
1–7 against the design. Record the results, review findings and fixes on the
ticket. The product owner decides whether Part B proceeds as planned or is
split.

---

# Part B: Tabs And Relationships

## Task 8: Tabs

**Files:** `src/desktop/model/tabs.rs` and tests, `src/desktop/view/tabs.rs`
and tests, workspace wiring.

**Interfaces:**
- `pub(crate) enum TabKey { Item { repo: RepoKey, kind: ItemDtoKind, id: String }, Inspect { repo: RepoKey, worktree: String, path: String } }`
- `pub(crate) struct Tabs { open: Vec<(TabId, TabKey)>, active: Option<TabId> }`
  with `open(key) -> (TabId, bool /* newly opened */)`, `close(id)`,
  `next()`, `previous()`.
- Each tab owns an `ItemView` (Task 6) under `ViewScope::Tab(id)`.

**Tests first:**

- Opening an open key focuses its tab and returns `false`.
- Items from two repositories stay open together.
- A refresh of the repository reloads every open tab and keeps their order.
- Closing the active tab activates its neighbor; closing the last empties
  the content region.
- A tab whose repository becomes unavailable shows the item's failure with
  Retry, and other tabs are unaffected (**Review Focus 2**).
- `Tabs::close_repository(root)` closes every tab of that root; Task 13
  calls it after a successful removal.
- `ctrl-tab`, `ctrl-shift-tab`, `secondary-w` work and focus follows.
- Switching tabs while the previous tab's load is held does not show the
  previous load in the new tab (**Review Focus 1**).

**Verify:** as Task 5.

**Commit:** `feat: open items in tabs keyed by repository and ULID`.

## Task 9: Short Codes, Find And Readiness Filters

**Files:** `src/desktop/model/{find,tickets}.rs` and tests,
`src/desktop/view/{find,tickets}.rs` and tests.

**Interfaces:**
- `pub(crate) enum FindQuery { Id(ItemId), Slug(String) }`;
  `fn parse(text: &str) -> Option<FindQuery>` (ULID parse first, else
  non-empty trimmed slug).
- `pub(crate) enum FindResult { Opened, Matches(Vec<ItemDto>), None }`.
- `TicketQuery.readiness` gains the All/Ready/Blocked control;
  `fn blocked_reasons(item: &ItemDto, by_id: &HashMap<String, ItemDto>) -> Vec<String>`
  turning `ReadinessReasonDto` into text (open dependency names short code
  and title; unresolved names the ULID; cycle; incomplete label).
- Copy actions in the item header: short code and ULID separately, through
  `cx.write_to_clipboard(ClipboardItem::new_string(..))`.

**Tests first:**

- A ULID opens that ticket, including a lifecycle-closed one.
- A duplicate short code (fixture: two committed ticket files carrying the
  same `slug`, then a refresh; `TicketSlugAssign` refuses a taken code) lists both matches
  with context; choosing one opens it by ULID.
- No match says so; a short code is never stored as a `TabKey`.
- Ready: a ticket whose dependencies are closed. Blocked: a ticket with an
  open dependency, one with an unresolved dependency and one in a cycle,
  each with its reason. A nonconforming ticket remains under Ready and
  Blocked (**Review Focus 3**).
- The clipboard holds exactly the short code or the ULID after each copy.

**Verify:** as Task 5.

**Commit:** `feat: find tickets by short code and filter by readiness`.

## Task 10: Relationships Panel

**Files:** `src/desktop/model/relationships.rs` and tests,
`src/desktop/view/relationships.rs` and tests.

**Interfaces:**
- `pub(crate) struct Relationships { dependencies: Vec<DependencyDto>, dependents: ListState<DependencyTreeDto>, parent: Option<DependencyDto>, children: ListState<ItemListDto>, problems: Vec<ProblemDto> }`
- Loads `ticket_dependencies(repo, id, DependencyDirection::Up, Some(1))`
  and `ticket_children(repo, id)` under the tab's stamp.

**Tests first:**

- A ticket with two dependencies, one dependent, a parent and two children
  shows each group; activating each row opens the related ticket in a tab.
- An unresolved dependency shows its ULID and cannot be opened.
- A relationship problem on the ticket is listed.
- A document tab shows no relationships panel.

**Verify:** as Task 5.

**Commit:** `feat: show a ticket's dependencies, dependents, parent and children`.

## Part B Checkpoint

As Part A's: checks, review of Tasks 8–10, ticket comment. Before Part C,
confirm `01M4R19P94` is merged to main, then fetch and merge `origin/main`
into this branch; record the base.

---

# Part C: Prompts, Onboarding And Settings

## Task 11: Credential Prompts

**Files:** `src/desktop/services/prompts.rs` and tests,
`src/desktop/view/passphrase.rs` and tests; replace Task 3's placeholder
provider.

**Interfaces:**
- `pub(crate) struct DesktopProvider { tx: UnboundedSender<WorkerEvent> }`
  implementing `SessionCredentialProvider`:
  sends `WorkerEvent::Prompt(PromptRequest { key_id, label, reason, reply: oneshot::Sender<PromptReply> })`
  and `futures::executor::block_on`s the reply. A failed send returns
  `Unavailable` at once; a dropped reply sender reads as `Unavailable`.
- `pub(crate) enum PromptReply { Supplied(SecretPassphrase), Cancelled, Unavailable }`.
- At most one prompt exists at a time by construction (one serial worker,
  synchronous provider). No answer is shared or copied between requests.
- `PassphraseDialog` uses `InputState::new(window, cx).masked(true)` without
  `mask_toggle()`, label "Passphrase for <label>"; on submit it builds the
  one `SecretPassphrase`, calls `set_value("", ..)` on the input, and drops
  the `InputState` entity.

**Tests first:**

- Supplied, Cancelled, a dropped reply sender and a failed send each reach a
  provider call made from a plain thread with the designed answer; the last
  two return `Unavailable` without blocking.
- After one supplied unlock, a second job needing the same key does not
  prompt (the session's cache), and selecting another key does.
- Empty and NUL-containing submissions are refused in the dialog.
- Closing the window while a prompt is open answers `Unavailable`.
- After submit the input's value is empty and the dialog entity is released.
- Sentinel `"sentinel-passphrase-7f3a"` is absent from every
  `DesktopRequest` field, every operation-area text, every notification and
  the workspace's rendered text after a supplied prompt (**Review Focus 5**).
- Quit while a prompt is open answers `Unavailable`.
- Focus returns to the control that started the operation.

**Verify:** as Task 5.

**Commit:** `feat: ask for a key passphrase from the desktop`.

## Task 12: Repository Onboarding

**Files:** `src/desktop/view/settings/{mod,repositories}.rs` and tests.

**Interfaces:**
- Settings page with Add existing and Create forms: path text field,
  optional Browse (`cx.prompt_for_paths` with directories only, for both
  forms; a dismissal leaves the field unchanged; a platform failure disables
  Browse with a fixed reason), primary branch,
  optional prefix, identity name and email (shown when
  `inspect_repository` reports none).
- Submit builds `Intent::RepoEnable` / `Intent::RepoCreate` and calls
  `confirm` (Task 7). On success the repository list reloads and one
  background `IndexRefresh` for the root is submitted (decision 4).

**Tests first:**

- Create in an empty temporary directory with identity and prefix: preview
  lists its effects; the preview creates no directory content and no
  registration; Confirm creates and registers; list shows it; a refresh is
  submitted.
- Add an existing repository without identity: identity fields appear;
  success persists identity locally.
- A non-empty directory for Create, and a non-repository for Add, show their
  typed failures with no effect.
- Partial: `FailurePoint::AfterRepositoryInitialization` gives `partial`
  with its recovery; Retry under the same request completes it.
- Pressing Confirm twice makes one repository (**Review Focus 4**).
- Browse: a simulated selection fills the field; a simulated dismissal
  leaves it unchanged and usable. (The test platform cannot simulate a
  dialog failure; that branch is native checklist step 6.)

**Verify:** as Task 5.

**Commit:** `feat: add and create repositories from the desktop`.

## Task 13: Repository Identity And Removal

**Files:** `src/desktop/view/settings/repositories.rs` (extend) and tests;
sidebar Remove wiring.

**Interfaces:**
- Identity form: name, email, initials choice
  (Unchanged/Clear/Set) → `Intent::RepoIdentitySet` (confirmed).
- Remove → `Intent::RepoRemove` (confirmed); dialog text says local files
  remain. Reachable from settings and from an Unavailable sidebar entry.

**Tests first:**

- Identity set with initials; inspect shows the new identity.
- Remove an accessible repository: files remain, registration gone.
- Remove a registration whose folder was renamed away succeeds (requires
  `01M4R19P94`; this test is the Part C gate's proof), and its open tabs
  close (`Tabs::close_repository`).
- Escape on either dialog changes nothing.

**Verify:** as Task 5.

**Commit:** `feat: set repository identity and remove registrations`.

## Task 14: Remotes

**Files:** `src/desktop/view/settings/remotes.rs` and tests.

**Interfaces:** list from `list_remotes_redacted`; Add (`RemoteAdd`), Remove
(`RemoteRemove`, confirmed), Select / None (`RemoteSelect`).

**Tests first:**

- A remote URL with an embedded password is shown redacted.
- Add then select an SSH remote; it shows as selected for publication.
- A non-SSH remote is listed and labeled not eligible; selecting it shows
  the typed failure.
- Removal of a remote the F2 binding refuses (fixture: a sibling
  `origin.bak` beside `origin`, per `01M4PB7W`) shows the refusal and leaves
  both.
- Partial: `FailurePoint::AfterRemoteMutation` on add gives `partial`;
  Retry completes it.

**Verify:** as Task 5.

**Commit:** `feat: manage remotes from the desktop`.

## Task 15: Shared SSH Keys

**Files:** `src/desktop/view/settings/keys.rs` and tests.

**Interfaces:** list with label, fingerprint, kind and selection; Generate
(label, optional passphrase and confirmation in masked inputs →
`Intent::KeyGenerate{protected}` plus the secret), Import (two path fields),
Copy public key (`public_key_text`), Select, Clear, Unregister
(`KeyRemove`), Delete (generated only, `KeyDelete`, confirmed). The
`KeyStore` comes from `Backend::key_store()`: `for_current_user()` in
production, `KeyStore::for_home(fixture.home())` in tests.

**Tests first:**

- Generate unprotected and protected keys; both listed; the protected one's
  passphrase never appears anywhere (**Review Focus 5**) and both inputs are
  cleared after submit.
- Mismatched passphrase confirmation is refused before any job.
- Retry of a protected generation prompts for the passphrase again and uses
  the same request ID.
- Copy public key puts exactly the public key text on the clipboard; no
  control shows or copies private material.
- An imported key offers no Delete; Unregister leaves its files.
- Delete of a generated key removes its files after confirmation; Escape
  leaves them.

**Verify:** as Task 5.

**Commit:** `feat: manage shared SSH keys from the desktop`.

## Task 16: Host Trust

**Files:** `src/desktop/view/settings/hosts.rs` and tests.

**Interfaces:** list pins (`list_host_pins`); Check host for the selected
repository (`WorkerHandle::observe_host(root, None)`); then Approve
(`Intent::HostApprove`, confirmed) or Replace (`Intent::HostReplace`,
confirmed, dialog shows old and new fingerprints).

**Tests first:**

No desktop test reaches a host: bin tests cannot initialize the transport
(design, Testing; ruling R11). Automated tests cover what needs no network:

- The pin list shows the fixture's pins (none, then one written by the
  library's own test entry points if one exists for pins; otherwise only
  the empty state, recorded).
- Check host without the transport initialized shows the typed failure the
  library returns, and nothing is pinned.
- The approve and replace dialogs: `prepare` of `HostApprove` and
  `HostReplace` with known authority, algorithm and fingerprints renders the
  preview (preview makes no connection, `bind/host.rs`), the replace dialog
  shows old and new fingerprints, and Escape pins nothing.

Native checklist step 9 and F2's SSH tests carry: observation, unlock
during observation with cancel then retry, approval writing the pin, and
replacement.

**Verify:** as Task 5.

**Commit:** `feat: approve and replace host keys from the desktop`.

## Task 17: Keyboard Audit, CI, Native Checklist And Gate

**Files:** `.github/workflows/build.yml`, any view needing focus fixes,
ticket comment.

**Steps:**

- [ ] A test walks every core action of the Cycle's Scope by keystrokes
  only (region keys, Tab, arrows, Enter, Escape) and asserts the focused
  element after each and visible focus style.
- [ ] Add two new workflow steps after "Test headless credential and
  platform contracts":
  `cargo test --locked --no-fail-fast --target ${{ matrix.target }} --features desktop-test-support --bin manyhands`,
  and `sh scripts/check-cli-no-gpui.sh`. Leave the headless step alone,
  including C1's `--bin manyhands-cli --test cli`. Appending the desktop
  feature to it would build the headless tests with GPUI. Do not dispatch the
  workflow without the product owner's approval (`AGENTS.md`, Native CI).
- [ ] Fetch and merge `origin/main`; check
  `git merge-tree --write-tree HEAD origin/main` reports no conflict.
- [ ] Full local gate:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  devenv shell -- cargo run --locked --features desktop --bin manyhands
  ```

- [ ] Hand the design's Native Checklist to the product owner; record their
  results (build revision, `Cargo.lock` digest, compositor, scale, per-step
  outcome) as a ticket comment. A failed step is fixed in this Cycle or
  explicitly accepted.
- [ ] Update the Cycle document's acceptance table with a status column,
  each row naming its test, as F2 did. Record the open obligations: Windows
  and macOS native runs (G1), `01M4EHGE2K`, `01M4W5SC`, `01M4EPRY`.
- [ ] Independent whole-Cycle review; fix and re-review; record.
- [ ] Review-ready ticket comment. Pushing, PR, merge, closure and cleanup
  each need their own authorization; closure is in the final pre-merge
  push.

**Commit:** `test: drive the desktop by keyboard and run its tests in CI`;
`docs: record D1 evidence and review`.

## Plan Self-Review

- **Coverage.** Every Cycle Scope bullet maps to a task: shell 1/4, service
  layer 2/3, navigation 4, discovery 5, find 9, item inspection 6/8/10,
  index maintenance 7, settings 12–16, prompts 7/11, operation area 7.
  Every acceptance row has a named test or the native checklist; CLI
  isolation in 1 and 17.
- **Placeholders.** None. Two conditional paths are explicit with their
  fallback and record: the Task 1 feasibility stop and Task 16's SSH fixture.
- **Consistency.** `Intent`, `DesktopRequest`, `WorkerHandle`, `Stamp`,
  `ViewScope`, `ListState`, `OpenTarget` and `TabKey` are defined once
  (Tasks 2–8) and used with the same names later.
- **Review Focus.** Each of the five has a test in the tasks named.
- **Size.** Seventeen tasks in three Parts with checkpoint reviews (decision
  1). A Part may be split at its checkpoint by the product owner.
