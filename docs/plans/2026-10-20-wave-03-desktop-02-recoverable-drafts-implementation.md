---
title: "Wave 03 D2 Recoverable Drafts And External-Change Handling Implementation Plan"
date: 2026-10-20
status: "proposed"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5E48FZAGA44CWYWZY9SY63X"
---

# Recoverable Drafts And External-Change Handling Implementation Plan

> **For agentic workers:** Do not begin until the product owner approves the
> Cycle, design and this plan and explicitly authorizes implementation. Then
> use `implementing-a-cycle` and the selected execution method task by task,
> re-read the design, repeat the ticket preflight, and record every
> checkpoint on ticket `01M4CC0VMZ21Z79B09YQRA0P7J`. Parts B and C
> additionally wait for the Structure part of `01M5CS4MJ0G7B0MPJD0KN0QMYA`
> to be merged to main (Cycle decision 4). Steps use checkbox (`- [ ]`)
> syntax for tracking.

**Goal:** Unsaved work that survives crashes, failed writes, removed
repositories and changed items, with a desktop surface to inspect,
restore, export and discard it, ready for D3's editor to write into.

**Architecture:** A headless `src/drafts/` library module owns the file
format, owner protection, atomic writes, per-draft locks, listing, the
three-way comparison and target classification. The desktop owns sessions
(revisions and flush timing on GPUI timers, writes on the background
executor), save completion through the F2 worker, tab integration, the quit
flush and the Drafts page.

**Tech stack:** Rust 2024; `serde`/`serde_json`, `fs4`, `ulid`, `time`,
`directories`, `libc` and `windows-sys` as already locked; `gpui-kit` 0.6.6
through `gpui_kit::*` only; Devenv. No new dependency.

**Spec:** [Cycle](../Cycles/wave-03-desktop-02-recoverable-drafts.md) and
[design](2026-10-20-wave-03-desktop-02-recoverable-drafts-design.md).
Executors read both. Section names below refer to the design.

**Status:** Proposed. Not approved.

## Global Constraints

- Work only in ticket `01M4CC0VMZ21Z79B09YQRA0P7J`'s worktree and branch.
  Preserve unrelated edits in main and every other worktree.
- Run every Rust command as `devenv shell -- cargo …`.
- Do not change `src/repository.rs`, anything under `src/repository/`,
  `src/canonical.rs`, `src/results.rs` or `src/runtime.rs` (track rule 5).
  `src/lib.rs` gains exactly `pub mod drafts;` (decision 2). A missing or
  wrong library capability stops the task: file a shared-library ticket with
  `file-ticket` and ask the product owner.
- `src/drafts/` imports nothing from `gpui_kit` and no desktop module.
- No new dependency. `windows-sys` features already enabled are enough;
  `Cargo.toml` changes only to register `[[test]]`/`[[example]]` entries if
  needed. Commit `Cargo.lock` if it changes.
- GPUI only through `gpui_kit::*`; no `use gpui_kit::*` glob anywhere.
- No type holding draft text (`Draft`, `DraftFields`, `DraftBase`,
  `DraftSession`, `Intent`) derives `Debug`. `DraftError` carries only a
  kind; its `Display` is a fixed sentence.
- Never put draft body text into an operation entry, notification, dialog
  message, log or error text.
- The store never calls a repository mutation. Only `Intent::DocumentSave`
  and `Intent::TicketSave` through the worker write canonical content.
- Every test uses its own temporary data directory and the isolated Git
  home. No test touches the real application data directory or reaches a
  network. No desktop test initializes the transport.
- `cargo test` takes one positional filter; each Verify line uses one.
- A view test that waits on the worker calls `cx.executor().allow_parking()`
  and finishes within the scheduler's 15 s limit. Read and draft-I/O delays
  are async gates.
- Desktop tasks verify with both `--features desktop-test-support` tests
  and `--features desktop` check and clippy (a test-support type in
  production code compiles only under the former).
- Write each behavior's test before its code. Commit per task with a
  conventional message and the `Co-Authored-By` trailer.
- Within a task run only the tests its Verify line names; the full suite
  runs at Part checkpoints and in Task 12 (owner's standing rule).
- Merge `origin/main` at work boundaries; never rebase this branch.
- The Windows protection module is compiled and tested only by the native
  Build workflow after merge (`AGENTS.md`, Native CI). Keep it small and
  follow the call patterns of `src/repository/keys/storage/windows.rs`,
  which compiles today.
- A second actor is a child process of the test binary, re-run with
  `--exact <test path>` and an environment variable naming its role. Desktop
  tests do it as `view/operations_tests.rs::lease_holder_child` does;
  library tests as `tests/mutation_replay/execute.rs` does. The parent
  asserts the child ran exactly one test.
- Desktop entry points that only D3 will call in production (`begin`,
  `edit`, `save` and what only they reach) carry
  `#[cfg_attr(not(test), allow(dead_code))] // Called by D3's editor; tests until then.`
  (design ruling R17). No blanket allowance.
- After each task, add a ticket comment under
  `.manyhands/comments/01M4CC0VMZ21Z79B09YQRA0P7J/` (or the CLI once it
  supports comments). It records the commit, the tests run with their
  results, and any ruling made. Part checkpoints add the review and its
  fixes.
- Run scripts through Devenv too: `devenv shell -- sh scripts/check-cli-no-gpui.sh`.
- `gpui-pre` turns on `serde_json/preserve_order`, so in desktop and
  `--all-features` builds a `serde_json::Map` keeps insertion order. Never
  rely on the map type for an order; insert keys pre-sorted (design ruling
  R18).
- Implementers never dispatch subagents.

## Review Focus

1. **A late completion destroys newer text.** A save completes for
   revision N after the user, or a test, made N+1, or after the draft was
   discarded. Only N may retire; nothing else changes. Pinned in Task 7.
2. **A crash between steps of a write.** Every crash point leaves a whole,
   readable revision and no half file. Pinned in Task 3.
3. **A file the store does not understand is lost.** Newer, damaged and
   unreadable files must survive every listing, launch and claim, byte for
   byte. Pinned in Tasks 1 and 3, and again in Task 10 over the desktop.
4. **The base moves to someone else's change.** A second actor's save
   between commit and re-read must leave the base unverified. Pinned in
   Task 7.
5. **Two processes write one draft.** A held claim in another process makes
   every write, restore and discard here unavailable. Pinned in Tasks 3 and
   11.

## File Map And Dependency Order

| Path | Task | Change |
| --- | --- | --- |
| `src/lib.rs` | 1 | `pub mod drafts;` |
| `src/drafts/{mod,schema}.rs`, `schema_tests.rs` | 1 | Types, version 1 format. |
| `tests/drafts.rs`, `tests/drafts/format.rs`, `tests/fixtures/drafts/*` | 1 | Integration target and golden files. |
| `.github/workflows/build.yml` | 1, 12 | `--test drafts` in the headless test list; checked again in 12. |
| `src/drafts/protect.rs`, `protect/{unix,windows}.rs`, `protect_tests.rs` | 2 | Owner protection. |
| `src/drafts/store.rs`, `store_tests.rs`, `tests/drafts/{write,crash,access,claims}.rs` | 3 | Writes, claims, listing, retire, discard, failure points. |
| `src/drafts/{compare,export}.rs` and tests, `tests/drafts/targets.rs` | 4 | Comparison, classification, export. |
| `examples/seed_draft.rs`, `Cargo.toml` | 4 | Development seeding. |
| `src/desktop/services/{mod,drafts}.rs`, `src/desktop/mod.rs`, `test_support/{fixtures,delaying}.rs` | 5 | Store in the backend, registry, launch listing, store-unavailable entry. |
| `src/desktop/model/drafts.rs` and tests, `view/operations.rs` | 6 | Sessions, flush schedule, recovery state, flush-failure entry. |
| `src/desktop/services/{requests,worker,drafts}.rs` | 7 | Save intents and completion. |
| `src/desktop/view/item*.rs`, `view/tabs.rs`, `view/settings/repositories/manage.rs` | 8 | Banner, restore, restored-draft section, close choices, removal. |
| `src/desktop/view/quit.rs` | 9 | Quit flush. |
| `src/desktop/view/drafts.rs` and tests, `keys.rs`, `view/workspace*.rs`, `view/operations.rs` | 10 | Drafts page, `OpenDrafts`, Recovered drafts entry. |
| `src/desktop/view/drafts.rs` (Inspect, Export, Discard) | 11 | Page actions, open elsewhere. |
| `view/keyboard_tests.rs`, design checklist, Cycle acceptance table | 12 | Keyboard audit, privacy sweep, gate. |

Desktop paths are as at `3bda743`. After `01M5CS4MJ0`'s Structure part
merges, Task 5 begins by updating this table to the split files; the
split changes only where code goes, not what it does.

Tasks run in order. Each task's unit tests sit beside its module as
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

Re-check the design's Library Surface Audit against every change since
`3bda743`:
`git diff --stat 3bda743 origin/main -- src/repository.rs src/repository src/results.rs src/lib.rs`.
Update the design before coding if a cited signature moved.

Before Task 5, repeat the merge and ancestry check, confirm that the
Structure part of `01M5CS4MJ0` is on main, and update the File Map.

---

# Part A: The Draft Store

## Task 1: Module, Types And The Version 1 Format

**Files:**
- Modify: `src/lib.rs` (one line)
- Create: `src/drafts/mod.rs`, `src/drafts/schema.rs`, `src/drafts/schema_tests.rs`
- Create: `tests/drafts.rs`, `tests/drafts/format.rs`
- Create: `tests/fixtures/drafts/v1-ticket.json`, `v1-document.json`, `v1-comment.json`, `v2-ticket.json`
- Modify: `.github/workflows/build.yml` (append `--test drafts` to the headless `cargo test` list)

**Interfaces:**
- Produces, in `manyhands::drafts`:

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct DraftId(ulid::Ulid);              // Display/FromStr as the ULID text

pub enum ItemKind { Document, Ticket }        // "document" | "ticket"

pub struct CommentTarget { pub comment_id: ItemId, pub parent_id: Option<ItemId> }
pub struct DraftTarget {
    pub repository: String,                   // registration root, the desktop's RepoKey
    pub item_kind: ItemKind,
    pub item_id: ItemId,                      // crate::canonical::ItemId
    pub comment: Option<CommentTarget>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum DraftFields {
    Document { title: String, body: String },
    Ticket { title: String, ticket_type: String, status: String,
             project: Option<String>, team: Option<String>,
             deps: Vec<String>, parent: Option<String>, body: String },
    Comment { body: String },
}

pub struct BaseContext { pub kind: ContextKind, pub branch: Option<String>,
                         pub worktree: String, pub head_oid: Option<String> }
pub enum ContextKind { Primary, Unverified, Active }

pub struct DraftBase {
    pub observation: Option<String>,          // None: unverified
    pub path: String,
    pub context: BaseContext,
    pub source: Option<String>,
    pub fields: DraftFields,
}

pub struct Draft {
    pub id: DraftId, pub revision: u64,
    pub created_at: time::OffsetDateTime, pub flushed_at: time::OffsetDateTime,
    pub target: DraftTarget, pub base: Option<DraftBase>, pub current: DraftFields,
}

pub enum UnreadableReason { Damaged, NewerFormat { version: u64 }, Access, NotOwned, NotAFile }

pub(crate) fn encode(draft: &Draft) -> Vec<u8>;
pub(crate) fn decode(bytes: &[u8]) -> Result<Draft, UnreadableReason>;
```

- `Draft`, `DraftFields`, `DraftBase` derive no `Debug`.

- [ ] **Step 1: Write the failing tests.** In `schema_tests.rs`: encoding a
  ticket, a document and a comment draft and decoding it gives equal
  fields; `decode` of each fixture below. In `tests/drafts/format.rs`: each
  golden `v1-*.json` decodes, and `encode(decode(bytes)) == bytes` exactly.
  `v2-ticket.json` (`"version": 2`) is `NewerFormat { version: 2 }`; a
  missing `format`, `"format": "other"`, invalid JSON, a truncated copy of
  `v1-ticket.json` (first 40 bytes), invalid UTF-8 and an `item_id` that is
  not a ULID are each `Damaged`.
- [ ] **Step 2: Run them to see them fail.**
  `devenv shell -- cargo test --locked --lib drafts::` and
  `devenv shell -- cargo test --locked --test drafts` fail to compile.
- [ ] **Step 3: Implement.** Private serde structs mirror the design's
  JSON exactly, with field names as shown; `kind` tags for `DraftFields`
  and `ContextKind` as lowercase strings; times as RFC 3339 UTC through
  `time::format_description::well_known::Rfc3339`. `encode` builds each
  object from a key-sorted list of entries, as `discovery.rs:275` does, so
  the order holds whether or not `preserve_order` is on. It then calls
  `serde_json::to_vec_pretty` and appends a trailing newline. `decode` first reads `format` and `version` from a
  `serde_json::Value`; `version > 1` is `NewerFormat`; then full parse.
  Make the golden fixtures once with `encode` from drafts built in a
  throwaway test. Check each by eye against the design's example (field
  names, nesting, RFC 3339 times, sorted keys) and commit it. From then on
  the round-trip test pins the bytes. `v2-ticket.json` is `v1-ticket.json`
  with `"version": 2` and one extra top-level key.
- [ ] **Step 4: Run them to see them pass.** Same two commands, and
  `devenv shell -- cargo test --locked --all-features --test drafts`, where
  `preserve_order` is on.
- [ ] **Step 5: Lint.** `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`.
- [ ] **Step 6: Commit** `feat: add the draft file format`.

## Task 2: Owner Protection

**Files:**
- Create: `src/drafts/protect.rs`, `src/drafts/protect/unix.rs`, `src/drafts/protect/windows.rs`, `src/drafts/protect_tests.rs`
- Modify: `src/drafts/mod.rs`
- Create: `tests/drafts/access.rs` (folder cases)

**Interfaces:**
- Consumes: nothing from earlier tasks but the module.
- Produces:

```rust
pub struct DraftError { pub kind: DraftErrorKind }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DraftErrorKind { Unavailable, ProtectionUnavailable, Access, NoSpace, OpenElsewhere, HeldHere, NotFound, Io }
impl std::fmt::Display for DraftError { /* one fixed sentence per kind */ }
impl From<std::io::Error> for DraftError { /* PermissionDenied→Access, StorageFull/QuotaExceeded→NoSpace, NotFound→NotFound, else Io */ }

/// The drafts folder, held open. On Unix every file operation is relative
/// to `handle` (decision 5); on Windows to `path`.
pub(crate) struct Folder { path: PathBuf, #[cfg(unix)] handle: std::fs::File }
impl Folder {
    pub(crate) fn open(data: &Path) -> Result<Folder, DraftError>;          // creates or checks data/drafts
    pub(crate) fn reopen(&mut self) -> Result<(), DraftError>;              // after ENOENT: same checks, once
    pub(crate) fn create_new(&self, name: &str) -> Result<File, DraftError>; // 0600, exclusive, no-follow
    pub(crate) fn open_file(&self, name: &str) -> Result<File, UnreadableReason>; // regular, not a link, ours
    pub(crate) fn rename(&self, from: &str, to: &str) -> Result<(), DraftError>;
    pub(crate) fn remove(&self, name: &str) -> Result<(), DraftError>;
    pub(crate) fn sync(&self) -> Result<(), DraftError>;                    // no-op on Windows
    pub(crate) fn names(&self) -> Result<Vec<String>, DraftError>;
}
#[cfg(unix)] pub(crate) fn data_verdict(mode: u32, uid: u32, euid: u32) -> Verdict;   // o+w or foreign: Refuse
#[cfg(unix)] pub(crate) fn folder_verdict(mode: u32, uid: u32, euid: u32, is_dir: bool) -> Verdict;
pub(crate) enum Verdict { Ok, Tighten, Refuse }
pub(crate) fn default_data_directory() -> Result<PathBuf, DraftError>;      // ProjectDirs("com","manyhands","Manyhands").data_local_dir(); doc comment names repository.rs's twin
```

- [ ] **Step 1: Write the failing tests.** Unit: `data_verdict` accepts
  0700, 0755 and 0775 of ours and refuses 0777 (`o+w`) and another UID.
  `folder_verdict` gives Ok for 0700 ours, Tighten for 0755 ours, Refuse
  for another UID and for a non-directory. Integration (`#[cfg(unix)]`):
  - a fresh data directory gets `drafts/` at exactly 0700;
  - a data directory at 0775 is accepted, and one at 0777 is
    `ProtectionUnavailable`;
  - an existing `drafts/` at 0755 becomes 0700, and one at 0500 stays
    0500 (tightening only clears bits);
  - `drafts/` as a symlink to another folder is `ProtectionUnavailable`;
  - after `Folder::open`, renaming `drafts/` away and putting a new empty
    `drafts/` in its place, a file made with `create_new` lands in the
    renamed folder, not the new one;
  - with `drafts/` removed after opening, `create_new` reopens once and
    succeeds.

  Each permission test first checks `libc::geteuid() != 0` and panics with
  "permission tests cannot run as root" otherwise.
- [ ] **Step 2: Run them to see them fail.**
  `devenv shell -- cargo test --locked --test drafts` and
  `devenv shell -- cargo test --locked --lib drafts::`.
- [ ] **Step 3: Implement Unix,** following `keys/storage/unix.rs:21-30`
  and its `open_at` and `directory` helpers. Open the data directory with
  `O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC` and `fstat` it for
  `data_verdict`. Then `mkdirat(…, "drafts", 0o700)`, treating `EEXIST` as
  existing, and `openat` it with the same flags. `fstat` the handle for
  `folder_verdict`; tighten with `fchmod(handle, mode & 0o700)`. File
  operations use `openat` (with `O_NOFOLLOW`, and `O_CREAT | O_EXCL` and
  mode 0600 for new files), `renameat`, `unlinkat` and `fsync` of the
  handle. `names` reads entries with `fdopendir` on a `dup` of the handle
  (closed by `closedir`). `open_file` reports a non-regular file as
  `NotAFile` and another owner as `NotOwned`.
- [ ] **Step 4: Implement Windows.** Path-based. `MetadataExt::file_attributes() &
  FILE_ATTRIBUTE_REPARSE_POINT` refuses the folder (`ProtectionUnavailable`)
  and makes a draft file `NotAFile`. The owner check uses
  `GetSecurityInfo` with `OWNER_SECURITY_INFORMATION` on a handle opened
  with `FILE_FLAG_BACKUP_SEMANTICS`, `OpenProcessToken` and
  `GetTokenInformation` for `TokenUser` and `TokenOwner`, and `EqualSid`;
  either SID equal is accepted (design ruling R4). Free with `LocalFree`.
  No DACL is written. Mirror the handle and `LocalMemory` patterns of
  `keys/storage/windows.rs`.
- [ ] **Step 5: Run the tests to see them pass.** Same commands.
- [ ] **Step 6: Lint** as Task 1.
- [ ] **Step 7: Commit** `feat: keep the drafts folder owner-protected`.

## Task 3: Atomic Writes, Claims, Listing, Retirement And Discard

**Files:**
- Create: `src/drafts/store.rs`, `src/drafts/store_tests.rs`
- Modify: `src/drafts/mod.rs`
- Create: `tests/drafts/{write,crash,claims}.rs`; extend `tests/drafts/access.rs`

**Interfaces:**
- Consumes: Task 1 types and `encode`/`decode`; Task 2 `Folder`, `DraftError`.
- Produces:

```rust
pub struct DraftStore { /* Arc of: Mutex<Folder>, held IDs (Mutex<BTreeSet<DraftId>>), failure point */ }
impl DraftStore {
    pub fn open_default() -> Result<Self, DraftError>;
    pub fn open_in(data_directory: &Path) -> Result<Self, DraftError>;
    pub fn list(&self) -> Result<Vec<Listing>, DraftError>;
    pub fn create(&self, target: DraftTarget, base: Option<DraftBase>, current: DraftFields) -> Result<DraftClaim, DraftError>;
    pub fn claim(&self, id: DraftId) -> Result<DraftClaim, DraftError>;
    pub fn discard_unreadable(&self, id: DraftId) -> Result<(), DraftError>;
    pub fn raw_bytes(&self, id: DraftId) -> Result<Vec<u8>, DraftError>;
    #[doc(hidden)]
    pub fn with_failure_point_for_testing(self, point: FailurePoint, action: FailureAction) -> Self;
}
pub struct DraftClaim { /* id, lock: File, draft: Draft, store: DraftStore clone */ }
impl DraftClaim {
    pub fn draft(&self) -> &Draft;
    pub fn write(&mut self, revision: u64, current: DraftFields, base: Option<DraftBase>) -> Result<Written, DraftError>;
    pub fn retire(self, saved_revision: u64) -> Result<Retired, DraftError>;
    pub fn discard(self) -> Result<(), DraftError>;
}
pub struct Written { pub revision: u64, pub flushed_at: time::OffsetDateTime }
pub enum Retired { Deleted, Kept(DraftClaim) }
pub struct Listing { pub id: DraftId, pub entry: Entry, pub held: Held }
pub enum Held { No, Here, Elsewhere }
pub enum Entry { Draft(Draft), Unreadable(UnreadableReason) }
#[doc(hidden)] pub enum FailurePoint { CreateTemp, Write, SyncFile, Rename, SyncFolder }
#[doc(hidden)] pub enum FailureAction { Fail(DraftErrorKind), Crash }   // Crash: process::exit(CRASH_EXIT_CODE) right after the step
#[doc(hidden)] pub const CRASH_EXIT_CODE: i32 = 86;
```

- `DraftStore: Clone + Send + Sync`; clones share the folder and the set
  of held IDs. `DraftClaim: Send`; it adds its ID to the set on creation
  and removes it on drop.
- `claim` of an ID in the set is `HeldHere`, without touching the lock.
- `write` refuses a `revision` lower than the stored one with `Io`.
- Only `retire` and `discard` delete a lock file, after the draft file and
  before releasing the lock. `list` never deletes anything (design ruling
  R20).

- [ ] **Step 1: Write the failing tests.**
  - `write.rs`: create, write revisions 2 and 3, `claim` again in a new
    store reads 3. For `CreateTemp`, `Write`, `SyncFile` and `Rename` with
    `Fail(Io)` (and `Write` with `Fail(NoSpace)`): the write returns that
    kind, `list` shows the previous revision, and no `.tmp` file remains.
    For `SyncFolder` with `Fail(Io)`: the write returns `Io` and `list`
    shows the new revision.
  - `crash.rs`: the parent creates a draft at revision 1 and drops the
    claim. For each point, it re-runs the test binary
    (`std::env::current_exe()`, `--exact <child test name>`, env
    `MANYHANDS_DRAFTS_CRASH_POINT=<point>` and the data directory), as
    `tests/mutation_replay/execute.rs` does. The child claims, sets `Crash`
    at that point and writes revision 2. The parent asserts the child's
    exit code is `CRASH_EXIT_CODE`. `list` then gives revision 1 for
    `CreateTemp`, `Write` and `SyncFile`, and revision 2 for `Rename` and
    `SyncFolder`, decoded whole. Where a temporary file was left, `list`
    never shows it and the next `claim` removes it.
  - `claims.rs`: a child process claims and waits on stdin; the parent's
    `claim` is `OpenElsewhere` and `list` has `Held::Elsewhere`; after the
    child exits the parent can claim. While the parent holds a claim, its
    own `list` (and a clone's) reports `Held::Here` and a second `claim` is
    `HeldHere`. `retire(2)` on stored revision 3 is `Kept`, on 3 is
    `Deleted` and the file and lock file are gone. `discard` removes one
    draft of two. A stray `<ULID>.lock` with no draft survives `list`.
    Files not named like the store's are ignored and left alone.
  - `access.rs` (Unix, not root): a draft file at 0o000 is
    `Unreadable(Access)` and still there afterwards; a `<ULID>.json` that is
    a directory or a symlink is `Unreadable(NotAFile)`; `drafts/` at 0o500
    makes `write` fail `Access` with the previous revision readable once
    the mode is restored. Damaged and newer files from Task 1 placed in the
    folder survive `list`, `claim` of another draft and `create`, byte for
    byte; `discard_unreadable` removes one; `raw_bytes` returns its bytes.
- [ ] **Step 2: Run them to see them fail.** `devenv shell -- cargo test --locked --test drafts`.
- [ ] **Step 3: Implement** the write protocol of the design's Writing a
  Revision section exactly, through Task 2's `Folder`: re-check the
  folder; `encode`; `create_new(".<id>.<16 random hex>.tmp")`;
  `write_all`; `sync_all`; `Folder::rename`; `Folder::sync`. On failure
  remove the temporary file and return the kind; a failure of the folder
  sync is returned too. `Fail(kind)` replaces the named step with that
  error. `Crash` runs the named step (for `Write`, half the bytes) and then
  calls `std::process::exit(CRASH_EXIT_CODE)`. `SyncFolder` is reached on
  Windows too, where the step itself does nothing. Locks: `<id>.lock`
  opened through `Folder` (0o600, created if absent),
  `fs4::fs_std::FileExt::try_lock_exclusive`; `Ok(false)` or `WouldBlock`
  is `OpenElsewhere`. `list` takes `Folder::names`, matches
  `<26 Crockford chars>.json`, `open_file`, `decode`, reports held IDs as
  `Held::Here` without probing and probes the rest, then sorts by
  `flushed_at` descending then ID, with unreadable entries last by ID.
- [ ] **Step 4: Run them to see them pass.** Same command, and
  `devenv shell -- cargo test --locked --lib drafts::`.
- [ ] **Step 5: Lint** as Task 1.
- [ ] **Step 6: Commit** `feat: write, claim, list and retire drafts atomically`.

## Task 4: Comparison, Target State, Export And Seeding

**Files:**
- Create: `src/drafts/compare.rs`, `compare_tests.rs`, `src/drafts/export.rs`, `export_tests.rs`
- Create: `tests/drafts/targets.rs`
- Create: `examples/seed_draft.rs`; modify `Cargo.toml` only if the example needs an entry

**Interfaces:**
- Consumes: Tasks 1–3; `RepositoryService::{resolve_repository, show_item, list_comments}`, `ItemDto`, `CommentListDto`, `ResultCode`.
- Produces:

```rust
impl DraftFields { pub fn of(item: &ItemDto) -> Option<DraftFields>; }
impl DraftBase   { pub fn of(item: &ItemDto) -> Option<DraftBase>; }   // observation, path, context, source, fields
pub enum Field { Title, Type, Status, Project, Team, Deps, Parent, Body }
pub enum Change { None, Mine, Theirs, BothSame, Both }
pub struct Comparison { pub fields: Vec<(Field, Change)> }
pub fn compare(base: &DraftFields, theirs: &DraftFields, mine: &DraftFields) -> Comparison;
pub enum TargetState { Current, Changed { theirs: DraftFields, comparison: Comparison, context_changed: bool },
    AlreadySaved, Unverified { theirs: DraftFields, comparison: Comparison }, TicketClosed, ItemNotFound,
    Nonconforming, RepositoryUnavailable(ResultCode), ReadFailed(ResultCode), ParentNotFound }
pub fn classify(draft: &Draft, item: Result<&ItemDto, ResultCode>) -> TargetState;
pub fn classify_comment(draft: &Draft, item: Result<&ItemDto, ResultCode>, comments: Result<&CommentListDto, ResultCode>) -> TargetState;
pub enum Restore { Allowed, AllowedFlagged, Refused(RefusalReason) }
pub fn restore_policy(draft: &Draft, state: &TargetState, held: Held) -> Restore;
pub fn markdown(draft: &Draft) -> String;
```

- Code mapping in `classify`: `repository_not_registered`,
  `repository_inaccessible`, `not_repository`, `not_repository_root` and
  `repository_not_enabled` are `RepositoryUnavailable`; `item_not_found` is
  `ItemNotFound`; every other code is `ReadFailed`.
- In `classify_comment`, a `CommentListDto` with `complete: false` is
  `ReadFailed`, never `ParentNotFound`.

- [ ] **Step 1: Write the failing unit tests.** `compare` for each `Change`
  per field, the body last. `classify` decision order from the design's
  Target State section, each over an `ItemDto` built in the test:
  a closed ticket whose observation equals the base is `TicketClosed`; equal
  fields with a different observation is `AlreadySaved`; a base with no
  observation is `Unverified`; a different branch sets `context_changed`.
  `classify_comment` with an incomplete list is `ReadFailed`.
  `restore_policy` follows the design's table row by row; a comment draft is
  always `Refused`. `markdown` of a ticket has front matter with the named
  fields and no `id` or `manyhands_managed`, then the body; of a comment,
  the body only.
- [ ] **Step 2: Write the failing integration tests** in
  `tests/drafts/targets.rs`, over real repositories made through the
  library as `tests/local_authoring.rs` does, each with a draft whose base is
  `DraftBase::of(show_item(..))`:
  - `Current` straight after.
  - `Changed` after a child process (another `RepositoryService` over the
    same data directory) saves the ticket's title through F2 `TicketSave`.
  - `AlreadySaved` after a save of exactly the draft's fields.
  - `TicketClosed` after a local-only close through
    `preview_ticket_closure` and `close_ticket` with no publication remote,
    with `SessionCredentials::new(provider)` where the test's own provider
    answers `Unavailable`. No transport is used, as
    `src/repository/closure/run_tests.rs:119-125` shows with its
    `NoCredentials`. The closure also removes the item worktree.
  - `Changed` with `context_changed` when the draft's base is the item
    worktree copy and that worktree has been removed so the item reads from
    primary.
  - `ItemNotFound` after the file is removed in a commit and the index
    refreshed.
  - `RepositoryUnavailable` after `repo remove` through F2, and after the
    repository folder is renamed away.
  - `ParentNotFound` for a comment draft whose `parent_id` is not in the
    thread (after a refresh, so the index holds the thread).
  - Survival: with three drafts stored, after each of registration removal,
    `rebuild_repository`, and deleting `manyhands.sqlite3` and reopening the
    service, `list` returns the same three with identical file bytes.
- [ ] **Step 3: Run them to see them fail.**
  `devenv shell -- cargo test --locked --test drafts` and
  `devenv shell -- cargo test --locked --lib drafts::`.
- [ ] **Step 4: Implement** `compare.rs` and `export.rs` per the design.
  `export::markdown` writes YAML with `serde_yaml` from a `BTreeMap`.
- [ ] **Step 5: Run them to see them pass.** Same commands.
- [ ] **Step 6: Seeding example.** `examples/seed_draft.rs` takes a
  registration root and a ULID, opens `RepositoryService::open_default` and
  `DraftStore::open_default`, reads the item, and creates a draft whose
  current fields are the item's with " (recovered draft)" appended to the
  title. It prints the draft ID. Build it with
  `devenv shell -- cargo build --locked --example seed_draft`.
- [ ] **Step 7: Lint** as Task 1.
- [ ] **Step 8: Commit** `feat: compare drafts with their items and classify their targets`.

## Part A Checkpoint

Run the four required checks and the CLI smoke test, and
`devenv shell -- sh scripts/check-cli-no-gpui.sh`. Request an independent review of Tasks
1–4 against the design. Record results, findings and fixes on the ticket.
The product owner decides whether Part B proceeds as planned.

---

# Part B: Desktop Sessions

Starts only when the Structure part of `01M5CS4MJ0` is on main and merged
into this branch (decision 4).

## Task 5: The Store In The Desktop

**Files:**
- Modify: `src/desktop/services/mod.rs`, `src/desktop/mod.rs`
- Create: `src/desktop/services/drafts.rs`, `src/desktop/services/drafts_tests.rs`
- Modify: `src/desktop/test_support/{fixtures,delaying}.rs`, `src/desktop/view/operations.rs`, `src/desktop/model/operations.rs`

**Interfaces:**
- Consumes: Task 3 `DraftStore`, `Listing`; D1 `Backend`, `Services`, `ReadKind`, `JobKind`, `Operations`.
- Produces:

```rust
// Backend
fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable>;   // DraftStoreUnavailable(DraftErrorKind)
impl LibraryBackend { pub(crate) fn new(service: RepositoryService, drafts: Result<DraftStore, DraftError>) -> Self; }
// ReadKind gains Drafts, DraftWrite, DraftItem; JobKind gains DocumentSave, TicketSave
pub(crate) struct DraftRegistry { /* sessions, attachments, listing, saves */ }
impl Services { pub(crate) fn drafts(&self) -> &Entity<DraftRegistry>; }
impl DraftRegistry {
    pub(crate) fn reload_listing(&mut self, services: &Services, cx: &mut Context<Self>) -> Stamp;
    pub(crate) fn listing(&self) -> &ListState<Vec<Listing>>;
    pub(crate) fn stored_for(&self, repo: &RepoKey, kind: ItemDtoKind, id: &str) -> Option<DraftId>; // unclaimed by this process
}
```

- `run()` passes `DraftStore::open_default()` to `LibraryBackend::new`; the
  fixture passes `DraftStore::open_in(data.path())`.

- [ ] **Step 0:** Update the File Map to the post-Structure layout; commit
  `docs: update the D2 file map after the structure split`.
- [ ] **Step 1: Write the failing tests.** Launch lists the store once and
  the registry holds the listing. A `Drafts` read gate held, then a second
  listing requested and released first: the earlier result is dropped. A
  data directory whose `drafts/` is a symlink (Unix): the window opens, the
  operation area shows "Recovery copies are unavailable" with the fixed
  reason and no Dismiss; `stored_for` answers `None`. A stored draft for a
  ticket is found by `stored_for` with the ticket's `RepoKey`, kind and ID.
- [ ] **Step 2: Run them to see them fail.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::services::drafts`.
- [ ] **Step 3: Implement.** The listing runs through the same read path
  as D1's reads, under `ReadKind::Drafts`, on the background executor.
- [ ] **Step 4: Run them to see them pass**, and
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::operations`.
- [ ] **Step 5: Check and lint.**
  `devenv shell -- cargo check --locked --features desktop --bin manyhands`,
  `devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings`,
  `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`.
- [ ] **Step 6: Commit** `feat: open the draft store in the desktop`.

## Task 6: Sessions, Flush Schedule And Recovery State

**Files:**
- Create: `src/desktop/model/drafts.rs`, `src/desktop/model/drafts_tests.rs`
- Modify: `src/desktop/services/drafts.rs` and tests, `src/desktop/view/operations.rs`

**Interfaces:**
- Consumes: Task 5 registry; Task 3 `DraftClaim::write`.
- Produces:

```rust
pub(crate) const IDLE: Duration = Duration::from_secs(1);
pub(crate) const MAX_UNFLUSHED: Duration = Duration::from_secs(5);
pub(crate) struct DraftSession { /* design's Sessions fields */ }
impl DraftSession {
    pub(crate) fn new(claim: DraftClaim, now: Instant) -> Self;
    pub(crate) fn edit(&mut self, current: DraftFields, now: Instant) -> u64;
    pub(crate) fn rebase(&mut self, base: DraftBase, now: Instant) -> u64;   // raises revision too (ruling R16)
    pub(crate) fn flush_due(&self) -> Option<Instant>;
    pub(crate) fn next_write(&mut self) -> Option<PendingWrite>;     // None if nothing new or a write is in flight
    pub(crate) fn write_landed(&mut self, result: Result<Written, DraftError>);
    pub(crate) fn recovery_state(&self) -> RecoveryState;           // Saved(at) | Pending | Failed(kind)
}
pub(crate) struct PendingWrite { revision: u64, current: DraftFields, base: Option<DraftBase>, claim: Arc<Mutex<Option<DraftClaim>>> }
impl DraftRegistry {
    pub(crate) fn begin(&mut self, target: DraftTarget, base: Option<DraftBase>, current: DraftFields, cx: &mut Context<Self>) -> Task<Result<DraftId, DraftErrorKind>>; // D3's first edit
    pub(crate) fn edit(&mut self, id: DraftId, current: DraftFields, cx: &mut Context<Self>); // schedules the timer
    pub(crate) fn flush_now(&mut self, id: DraftId, cx: &mut Context<Self>) -> Task<Result<(), DraftErrorKind>>;
    pub(crate) fn flush_all(&mut self, cx: &mut Context<Self>) -> Task<Vec<(DraftId, DraftErrorKind)>>;
}
```

- The registry takes every `now` from `cx.background_executor().now()`,
  never `Instant::now()`, so `advance_clock` drives it in view tests. A
  `rebase` is flushed at once, not on the timer.

- [ ] **Step 1: Write the failing model tests.** With an injected clock:
  one edit is due at +1 s; edits every 300 ms for 12 s are due at +5 s and
  +10 s; after a write lands, nothing is due until the next edit; three
  edits during an in-flight write produce one `next_write`, of the newest
  revision; a failed write gives `Failed(kind)`, keeps `flushed`, and the
  next edit schedules again; `recovery_state` table: `Saved` only when
  `flushed == revision` with no failure.
- [ ] **Step 2: Write the failing registry and view tests.** A session
  made with `begin` and edited, with a `DraftWrite` gate held: after
  `advance_clock(1 s)` its `recovery_state()` is `Pending` and the store
  still holds revision 1; after release it is `Saved` and the store holds
  the newest revision. (Task 8 renders these states in the tab.) With
  `drafts/` made read-only (Unix) the operation area shows "Recovery copy
  of <title> not saved: …" with Retry; restoring the mode and pressing Retry
  clears it. A sentinel string in the body appears in no operation entry.
- [ ] **Step 3: Run them to see them fail.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::model::drafts`
  and `… desktop::services::drafts`.
- [ ] **Step 4: Implement.** The registry schedules
  `cx.background_executor().timer(due - now)` and, when it fires, takes
  `next_write` and spawns the blocking `write` on the background executor
  behind the `DraftWrite` gate; `write_landed` runs back on the entity.
- [ ] **Step 5: Run them to see them pass.**
- [ ] **Step 6: Check and lint** as Task 5.
- [ ] **Step 7: Commit** `feat: flush draft sessions on a schedule and report their recovery state`.

## Task 7: Save Completion

**Files:**
- Modify: `src/desktop/services/requests.rs`, `src/desktop/services/worker.rs`, `src/desktop/services/mod.rs`, `src/desktop/services/drafts.rs` and tests, `src/desktop/model/drafts.rs` and tests

**Interfaces:**
- Consumes: Task 6 sessions; Task 4 `DraftFields::of`, `DraftBase::of`; D1 `WorkerHandle::submit`, `WorkerEvent::Finished`.
- Produces:

```rust
// Intent gains:
DocumentSave { root: PathBuf, id: ItemId, title: String, body: String, observation: Option<String> },
TicketSave { root: PathBuf, id: ItemId, title: String, ticket_type: String, status: String,
             project: Option<String>, team: Option<String>, body: String,
             deps: RelationshipWrite<Vec<String>>, parent: RelationshipWrite<String>, observation: String },
impl DraftRegistry {
    pub(crate) fn save(&mut self, id: DraftId, cx: &mut Context<Self>) -> Task<Option<DesktopRequestId>>; // flushes, then submits
    pub(crate) fn finished(&mut self, request: DesktopRequestId, envelope: &Envelope<MutationDataDto>, cx: &mut Context<Self>);
}
// model
pub(crate) enum SaveVerdict { Retire, Pending, Keep }
pub(crate) fn save_verdict(envelope_outcome: Outcome, checkpoint: CheckpointEffect) -> SaveVerdict;
pub(crate) fn base_after_save(submitted: &DraftFields, read: Result<&ItemDto, ResultCode>, previous: &DraftBase) -> DraftBase;
```

- `save_verdict` is `Retire` exactly for `checkpoint == Committed`, or
  `checkpoint == Unchanged` with `outcome` `Success` or `Noop`; `Pending`
  for `checkpoint == Pending`; `Keep` otherwise.
- `save` returns `None`, doing nothing, while the session has a save in
  flight (ruling R15), when its base is missing, and when its base has no
  observation (unverified). Otherwise it first flushes: it waits for any
  write in flight, writes the newest revision N if it is not stored, and
  submits only once N is stored (ruling R21). A failed flush submits
  nothing; the returned task resolves to `None`.
- On a retiring completion the registry waits for any write in flight,
  takes the claim out of the session's slot and calls `retire(N)`.
- A `pending` completion keeps the save record and `saving` (ruling R22).
  Dismissing that request in the operation area, or abandoning its pending
  operation, clears them.
- `TicketSave`'s `deps`/`parent` are built per design ruling R13.
- Tests assert on the draft's text and base, not on revision numbers: a
  rebase raises the revision (ruling R16).

- [ ] **Step 1: Write the failing model tests.** `save_verdict` over every
  `Outcome` × `CheckpointEffect`. `base_after_save`: equal fields adopt the
  read's observation, path and context; different fields or a read error
  give the submitted fields with `observation: None`. The deps rule:
  equal → `Unchanged`; empty with base non-empty → `Clear`; else `Set`.
- [ ] **Step 2: Write the failing service tests**, over real repositories:
  - Edit and save at once, with no clock advanced: the save flushes N
    first; on completion the draft file and lock file are gone and the
    session ended.
  - Document: claim a draft, `edit`, `save`, hold
    `before_job(DocumentSave)`, `edit` to newer text, release. The draft
    file then holds the newer text, its base observation equals
    `show_item`'s, and a second `save` succeeds with
    `checkpoint: committed`.
  - Write in flight at completion: as above, but with a `DraftWrite` gate
    holding the newer text's write when the save completes. Retirement
    waits; on release the store holds the newer text with the moved base.
  - Ticket: the same with `TicketSave`, on a ticket whose file has an
    ignored dependency (an entry that is not a ULID). After both saves the
    file still has it.
  - Base after save, second actor: hold the re-read with a
    `ReadKind::DraftItem` gate; a child process saves the ticket's status;
    release. The base has no observation, the state is `Unverified`, and
    `save` returns `None`.
  - Stale save: a child process saves first; the desktop's `save` answers
    `external_change`; the draft file is byte-identical to before.
  - Discarded meanwhile: `discard` while the save is held; the completion
    changes nothing and does not panic.
  - One in flight: a second `save` while the first is held returns `None`.
  - Replay: after a save completes and retires its revision, Retry of the
    same request from the operation area is answered from F2's record; the
    branch has exactly one new commit and the registry changes nothing.
  - Pending, then Retry: swap the backend's service with
    `DelayingBackend::replace_service` for one opened with
    `RepositoryService::open_at_with_failure_point_for_testing(…,
    FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence)`, the recipe
    of `tests/mutation_replay/documents.rs:521-534`. The save answers
    `partial` with checkpoint `pending`; the draft is kept and a second
    `save` returns `None`. Reopen the service with `open_at` and Retry the
    request from the operation area: it commits once (one new commit on
    the branch) and the revision retires.
- [ ] **Step 3: Run them to see them fail.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::services::drafts`
  and `… desktop::model::drafts`.
- [ ] **Step 4: Implement.** `Services::start`'s event task forwards each
  `Finished` to `DraftRegistry::finished` after applying it to
  `Operations`. The re-read uses the D1 read path with busy retries.
- [ ] **Step 5: Run them to see them pass**, and the worker tests
  `… desktop::services::worker`.
- [ ] **Step 6: Check and lint** as Task 5.
- [ ] **Step 7: Commit** `feat: retire only the saved revision of a draft`.

## Task 8: Drafts In Tabs

**Files:**
- Modify: the item view files after the split (`src/desktop/view/item*.rs`), `src/desktop/model/item.rs`, `src/desktop/view/tabs.rs`, the repository removal flow in `src/desktop/view/settings/repositories/manage.rs` and the removal confirmation text in `src/desktop/view/confirm.rs` (shared by the settings, sidebar and recovery removal paths); tests beside each
- Create: `src/desktop/view/draft_comparison.rs` and tests

**Interfaces:**
- Consumes: Tasks 4–7.
- Produces: `DraftRegistry::restore(id, tab, cx)`, `DraftRegistry::release(id, cx) -> Task<Result<(), DraftErrorKind>>`, `DraftRegistry::discard(id, cx)`; the close-tab dialog `CloseWithDraft { KeepDraft, Discard, Cancel }`; `fn target_label(&TargetState) -> &'static str` with the design's labels; `DraftComparisonView`, which Task 11's Inspect reuses.
- The banner's "Review in Drafts" and the section's Export… and Discard… are added in Tasks 10 and 11.

- [ ] **Step 1: Write the failing tests.**
  - Banner: a stored draft for a ticket shows "A recovered draft of this
    item is kept" with Restore; a claimed one does not; opening the tab
    restores nothing (the store's listing still shows the draft
    unclaimed).
  - Restore: attaches the draft; the section shows its fields, "Saved for
    recovery at …", "The item is unchanged since this draft began", and
    disabled Edit and Save reading "Available with the editor in a later
    release". A snapshot of the repository's files and Git refs before and
    after is equal.
  - Recovery state rendered: with a `DraftWrite` gate held after a test
    `edit`, the section reads "Not yet saved for recovery" until release.
  - Restore refused for a second draft of the same target in this process;
    Restore of a draft discarded by a child process since the listing reads
    "This draft is no longer kept" and the listing reloads.
  - Refresh: after a child process saves the item and the user presses
    Refresh, the section's text is unchanged and the state reads "The item
    changed since this draft began"; Compare expands the comparison, which
    names the changed field as "The item now" only.
  - `target_label` gives a distinct text for every `TargetState`.
  - Close: Keep draft releases the claim and the draft is listed again;
    Discard deletes only that draft; Cancel keeps the tab; Save is disabled.
    With `drafts/` read-only, Keep leaves the tab open with the failure.
  - Removal: removing the repository through Settings closes its tabs,
    keeps the draft, and the dialog shows "Drafts of this repository's
    items stay in Drafts."
- [ ] **Step 2: Run them to see them fail.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::item`
  and `… desktop::view::tabs`, `… desktop::view::settings`.
- [ ] **Step 3: Implement** per the design's Tabs section.
- [ ] **Step 4: Run them to see them pass.**
- [ ] **Step 5: Check and lint** as Task 5.
- [ ] **Step 6: Commit** `feat: show, restore and keep drafts in their tabs`.

## Task 9: Flush On Quit

**Files:**
- Modify: `src/desktop/view/quit.rs`, `src/desktop/view/workspace*.rs`; tests in `src/desktop/view/quit_tests.rs` or the workspace tests D1 uses for quitting

**Interfaces:**
- Consumes: Task 6 `flush_all`.
- Produces: `QuitPhase::SavingDrafts` before D1's phases; dialog
  `DraftsNotSaved { count }` with Quit anyway and Cancel.

- [ ] **Step 1: Write the failing tests.** With a session dirty and a
  `DraftWrite` gate held, `secondary-q` shows "Saving drafts for recovery"
  with Quit now and does not quit; on release it quits. With the gate never
  released, Quit now quits. With `drafts/` read-only the
  dialog reads "1 draft could not be saved for recovery"; Cancel keeps the
  window and focuses the draft's Retry; Quit anyway quits. The window close
  request takes the same path. With no dirty session, D1's quit tests pass
  unchanged.
- [ ] **Step 2: Run them to see them fail.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::quit`
  (or the workspace module holding D1's quit tests, as found after the split).
- [ ] **Step 3: Implement** per the design's Quit section.
- [ ] **Step 4: Run them to see them pass**, and D1's quit tests.
- [ ] **Step 5: Check and lint** as Task 5.
- [ ] **Step 6: Commit** `feat: save drafts for recovery before quitting`.

## Part B Checkpoint

Run the four required checks, `--features desktop` check and clippy, the
CLI smoke test, the CLI isolation script and a desktop smoke run on a
display. Request an independent review of Tasks 5–9 against the design.
Record results, findings and fixes on the ticket. The product owner decides
whether Part C proceeds as planned.

---

# Part C: Recovery Surface

## Task 10: The Drafts Page

**Files:**
- Create: `src/desktop/view/drafts.rs`, `src/desktop/view/drafts_tests.rs`
- Modify: `src/desktop/keys.rs`, `src/desktop/view/workspace*.rs`, `src/desktop/view/operations.rs`, `src/desktop/model/drafts.rs`

**Interfaces:**
- Consumes: Tasks 4–8.
- Produces: action `OpenDrafts` bound to `secondary-shift-d`;
  `DraftsPage` entity shown in the content region in place of the tabs;
  `DraftRow { id, kind_label, title, repository, flushed_at, state: RowState, actions: RowActions }`;
  the operation area's "Recovered drafts: N" entry with Review and Dismiss.

- [ ] **Step 1: Write the failing tests**, each over a real fixture made as
  in Task 4's targets tests: one row per state (`Current`, `Changed`,
  `AlreadySaved`, `Unverified`, `TicketClosed`, `ItemNotFound`,
  `Nonconforming`, `RepositoryUnavailable`, `ReadFailed` through a held
  cache lease, `ParentNotFound`, unreadable `NewerFormat`, `Damaged`,
  `Access`, `NotAFile`), each with its own text label and exactly the
  actions the design's restore policy allows. A draft restored in this
  window reads "Restored in this window" and offers Show, which focuses
  its tab. Empty store: "No drafts are kept." Launch
  with two stored drafts: "Recovered drafts: 2" with Review; Review opens
  the page; Dismiss hides it for the session. The tab banner gains
  "Review in Drafts", which opens the page with that draft's row focused. A row's held `Item` read never
  lands after the listing reloads. A newer-format file is byte-identical
  after the page has listed it and the app has restarted. Launch restores
  nothing.
- [ ] **Step 2: Run them to see them fail.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::drafts`.
- [ ] **Step 3: Implement** per the design's Drafts Page and Operation
  Area sections.
- [ ] **Step 4: Run them to see them pass**, and
  `… desktop::view::operations`, `… desktop::view::workspace`.
- [ ] **Step 5: Check and lint** as Task 5.
- [ ] **Step 6: Commit** `feat: list kept drafts with their state`.

## Task 11: Inspect, Restore, Export And Discard From The Page

**Files:**
- Modify: `src/desktop/view/drafts.rs` and tests, the restored-draft section of the item view (Task 8) and its tests

**Interfaces:**
- Consumes: Tasks 4, 8, 10; `cx.prompt_for_new_path`; Task 8's `DraftComparisonView`.
- Produces: the Inspect panel; `export_to(path, bytes) -> Result<(), DraftErrorKind>`
  (temporary file in the target folder, then rename); the Discard dialog.
  Export… and Discard… on page rows and in the tab's restored-draft
  section use the same code.

- [ ] **Step 1: Write the failing tests.**
  - Inspect on a `Changed` row shows the comparison view, with each change
    as text; Escape closes it and focus returns to Inspect.
  - The tab's Export… and Discard… behave as the page's; Discard… from the
    tab ends the session and the section disappears.
  - Restore from the page opens the item's tab with the draft attached.
  - Export: `simulate_new_path_selection` to a temporary path writes
    `markdown(draft)`; an unreadable row exports its raw bytes; exporting
    over an existing file into a read-only folder (Unix) shows the failure
    and leaves that file's bytes unchanged.
  - Discard: the dialog names the draft; Confirm removes only it; Escape
    removes nothing and focus returns to the row's Discard; an unreadable
    row's Discard removes its file.
  - Open elsewhere: a child process holds the claim (as Task 3's claims
    test); the row reads "Open in another Manyhands window" and offers
    Inspect and Export only.
- [ ] **Step 2: Run them to see them fail**, as Task 10.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run them to see them pass.**
- [ ] **Step 5: Check and lint** as Task 5.
- [ ] **Step 6: Commit** `feat: inspect, export and discard drafts`.

## Task 12: Keyboard, Privacy, Native Checklist And Gate

**Files:**
- Modify: `src/desktop/view/keyboard_tests.rs` (and its `actions` and `settings` submodules as fits)
- Modify: `.github/workflows/build.yml` (confirm `--test drafts` is listed)
- Modify: `docs/Cycles/wave-03-desktop-02-recoverable-drafts.md` (Held by and Status columns), the design's native checklist if steps change

- [ ] **Step 1: Keyboard tests.** By keystroke only: `secondary-shift-d`
  opens Drafts; Tab reaches each row action; Inspect, Escape; Export
  through the simulated dialog; Discard, Escape, Discard, Enter; Restore,
  then in the tab Compare and the close-tab choices; quit with a held write.
  Each asserts visible focus as D1's keyboard tests do.
- [ ] **Step 2: Privacy sweep.** One test plants a sentinel in a ticket
  draft's body. It drives restore, refresh, a failing flush, a stale save,
  a save the library refuses as `invalid_input` (an empty type), and
  export. It then asserts the sentinel is in no operation entry, envelope
  message shown, notification, dialog message, or `format!("{:?}")` of any
  `DraftError` produced.
- [ ] **Step 3: Run them.**
  `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::keyboard`.
- [ ] **Step 4: Full gate.**

```sh
git fetch origin main && git merge origin/main
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
devenv shell -- cargo check --locked --features desktop --bin manyhands
devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
devenv shell -- sh scripts/check-cli-no-gpui.sh
devenv shell -- cargo run --locked --bin manyhands-cli
devenv shell -- cargo run --locked --features desktop --bin manyhands   # needs a display
git merge-tree --write-tree HEAD origin/main
```

- [ ] **Step 5: Evidence.** Fill the Cycle's acceptance table with the
  tests that hold each row, as D1 did, and mark what the native checklist
  and the post-merge native run still owe. Do not claim Windows or macOS.
- [ ] **Step 6: Commit** `test: drive drafts by keyboard and record D2 evidence`.

Then hand the native checklist to the product owner and record
review-ready on the ticket. Dispatching the Build workflow, pushing,
merging, closing and cleanup each need their own authorization.

## Plan Self-Review

- **Spec coverage.** Store (Tasks 1–3), comparison and classification
  (4), survival (4, 8), sessions and flush (6), retirement (7), tabs and
  close (8), quit (9), recovery surface (10, 11), keyboard and privacy
  (12), seeding (4), native list (1, 12). Every Cycle acceptance row maps to
  a task's tests.
- **Placeholders.** None. The local-only closure and the pending-checkpoint
  recipe were confirmed against source by the independent review, so
  their earlier fallbacks are gone.
- **Order.** The comparison view is built in Task 8 and reused in Task 11;
  the banner's Review in Drafts waits for the page in Task 10; Export… and
  Discard… arrive in Task 11 for the page and the tab together.
- **Type consistency.** `DraftClaim`, `DraftFields`, `DraftBase`,
  `TargetState`, `restore_policy`, `save_verdict` and `base_after_save` are
  named the same in every task that uses them.
- **Review Focus.** Each of the five lines has its test in the owning task.
