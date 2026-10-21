---
title: "Wave 03 D2 Recoverable Drafts And External-Change Handling Design"
date: 2026-10-20
status: "proposed"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5E48FY5JG4KQS2KJ00XZXC4"
---

# Wave 03 D2 Recoverable Drafts And External-Change Handling Design

## Intent And Approval Boundary

This design implements the
[D2 Cycle](../Cycles/wave-03-desktop-02-recoverable-drafts.md). It says
where drafts are kept and how they are written, how the desktop flushes and
retires them, how a draft is compared with an item that changed, what the
recovery surface shows, and how each requirement is proved. The Cycle
document owns scope, exclusions and the six product-owner decisions of
2026-10-20 and 2026-10-21; this document cites them by number.

It is proposed for approval. Approval does not authorize implementation.

## Library Surface Audit At `3bda743`

Read from source at `origin/main` `3bda743`. No Rust command was run. Line
numbers are at that revision.

Re-checked on 2026-10-21 at `d7962c3`. Since `3bda743` the library changed
only `repository.rs`, `mutation/bind/repo.rs`, `mutation/outcome.rs` and
`recovery.rs`, for the enablement-index and registration fixes. No entry
point, type or code cited below changed. `open_default`'s directory is now
at `repository.rs:7219`; every other cited line is unchanged.

| Need | Library entry point | Finding |
| --- | --- | --- |
| Read an item with its base material | `RepositoryService::show_item(&ResolvedRepository, &ItemId) -> Result<ItemDto, ReadError>` (`read/items.rs:1784`) | Returns `observation` (opaque; changes with `context.branch`, `path` or the file's bytes), `source` (`None` when not UTF-8), `body`, named fields, `closure`, `context` (`kind`, `branch`, `worktree`, `head_oid`) and `problems`. Enough for a draft's base. |
| Resolve a draft's repository | `resolve_repository(&Path)` (`read/resolve.rs:228`) | Fails with `repository_not_registered`, `repository_inaccessible`, `not_repository` and others. These map to the target states below. |
| A comment draft's parent | `list_comments(&ResolvedRepository, &ItemId)` (`read/comments.rs:258`) | A flat list with `id` and `parent_id`. Enough to tell whether a reply's parent is present. |
| Save a draft's revision | F2 `prepare`/`execute` with `Mutation::DocumentSave(DocumentSaveInput { root, id, draft: DocumentDraft { title, body }, observation: Option<String> })` (`bind/document.rs:59`) and `Mutation::TicketSave(TicketSaveInput { root, id, draft: TicketDraft { title, ticket_type, status, project, team, body }, deps, parent, observation: String })` (`bind/ticket.rs:58`). Neither needs confirmation (`command.rs:436`). | The observation is the caller's token from `show_item`. A changed file answers `external_change`; a closed ticket answers `ticket_closed`. Unknown metadata is kept by the library. `deps` and `parent` are `RelationshipWrite::{Unchanged, Clear, Set}` (`repository.rs:613`). |
| Know a save was checkpointed | `Envelope.effects.checkpoint: CheckpointEffect::{NotRequested, Unchanged, Committed, Pending}` (`results.rs:63`) | `committed` proves a commit. `unchanged` with `success` or `noop` proves canonical already equals the draft. `pending` proves nothing. |
| The token of what a save wrote | none | `DocumentMutationDto` and `TicketMutationDto` return `id` and `path`, not the new observation (`mutation/dto.rs:906`, `930`). The desktop re-reads the item and moves the base only if the read's fields equal the saved ones (ruling R5). |
| Where application data is | `RepositoryService::open_default` uses `ProjectDirs::from("com", "manyhands", "Manyhands").data_local_dir()` (`repository.rs:7193`, private) | The draft store computes the same directory itself (ruling R2). |
| Drafts are not touched by the library | registry open, corruption recovery, rebuild and registration removal | No library path removes data-directory entries other than its own registry and lock files (`coordination.rs:165`, `188`, `197`). `drafts/` collides with none of them. |
| Owner-protected files | `keys/storage/{unix,windows}.rs` | `pub(super)` and specific to `~/.ssh/manyhands`. Not reusable without changing `src/repository/keys` (decision 3 avoids that). |
| `windows-sys` features | `Cargo.toml` | `Win32_Foundation`, `Win32_Security`, `Win32_Security_Authorization`, `Win32_Storage_FileSystem` and `Win32_System_Threading` are already enabled. The owner check needs no manifest change. |

Provisional audit rows refreshed: D16 (draft identity and base), D17
(protected persistence and flush), D18 (inspect, restore, export, discard),
D19 (retirement), D20 (close choices), D21 (comparison and reload), D22
(survival) and D30 (stale completions). All were `absent` or
`partially-present`. This Cycle supplies them except the editor-side halves
of D19–D21, which are D3's.

## Module Layout

| Path | Role |
| --- | --- |
| `src/lib.rs` | Gains `pub mod drafts;` and nothing else (decision 2). |
| `src/drafts/mod.rs` | Public types and `DraftStore`. |
| `src/drafts/schema.rs` | Version 1 file format; decode with version dispatch; encode. |
| `src/drafts/store.rs` | Directory opening, atomic write, listing, claims, retirement, discard, failure points. |
| `src/drafts/protect.rs`, `protect/unix.rs`, `protect/windows.rs` | Owner protection of the folder and files. |
| `src/drafts/compare.rs` | Fields of an `ItemDto`, three-way comparison, target classification. |
| `src/drafts/export.rs` | Export rendering. |
| `src/drafts/*_tests.rs` | Unit tests, the repository's convention. |
| `tests/drafts.rs`, `tests/drafts/*.rs` | Integration tests over real files, child processes and real repositories. |
| `examples/seed_draft.rs` | Development-only seeding for the native checklist. |
| `src/desktop/model/drafts.rs` | GPUI-free session, flush schedule, save-completion transitions, page rows (`DraftRow`, `RowState`, `RowActions`). |
| `src/desktop/services/drafts.rs` | The registry entity, store calls on the background executor, save submission and completion routing. |
| `src/desktop/view/drafts.rs` | The Drafts page entity: its shown state, rows and composition. |
| `src/desktop/view/drafts/{inspect,actions}.rs` | The Inspect panel; Restore, Export and Discard, shared by a row and the tab's restored-draft section. |
| `src/desktop/view/draft_comparison.rs` | The comparison view the tab and the page share. |
| `src/desktop/view/item/draft.rs` | The draft banner and restored-draft section of a tab, composed by `view/item.rs`; `view/item/reads.rs` recomputes the target state on reload. |
| `src/desktop/view/workspace.rs` | `ContentArea` holds the cached Drafts page entity beside Settings. |
| `src/desktop/view/workspace/content.rs` | `ContentPage::Drafts` and its routing; close-tab choices; releasing sessions when a repository's tabs close. |
| `src/desktop/view/workspace/lifecycle.rs`, `view/quit.rs` | The quit flush phase; its status text and Quit now. |
| `src/desktop/view/workspace/requests.rs`, `view/operations.rs`, `model/operations.rs` | Operation-area entries and the actions they call. |
| `src/desktop/view/workspace/render.rs`, `src/desktop/keys.rs` | `OpenDrafts`, its binding and its handler. |

The desktop paths follow the Structure split merged at `d7962c3` (decision
4), as its plan's
[D2 handoff map](2026-10-21-desktop-structure.md#5-d2-handoff-map) places
them. Shared helpers come from it too: `Read<T>` (`model/read_state.rs`),
`TicketName` (`model/names.rs`) and `ItemLocator`
(`model/navigation.rs`). Every `Workspace` field is private, so sibling
views reach D2's state through named `Workspace` actions and queries, not
fields. Each new desktop source file, and each file of the Structure split
that D2 grows, stays below about 500 lines, the review target the
Structure plan set. One that would pass it is split by responsibility
behind its parent, as `view/item/` is. Test files are exempt.

## The Draft Store

### Location

`DraftStore::open_default()` uses
`ProjectDirs::from("com", "manyhands", "Manyhands").data_local_dir().join("drafts")`,
the registry's own directory plus `drafts/`. `DraftStore::open_in(data)`
uses `data.join("drafts")`; tests and the desktop fixture pass the same
temporary data directory they give `RepositoryService::open_at`.

The desktop keeps opening the service with `RepositoryService::open_default`,
as the CLI does, so both front ends always share one registry. The store
computes its own path with the same `ProjectDirs` arguments, and a doc
comment on each names the other. If the two ever diverged, drafts would be
kept in another folder; the registry would not be split (ruling R2).

### File format, version 1

One file per draft, `<draft-id>.json`, UTF-8 JSON. An encode is
deterministic in every build: `gpui-pre` turns on `serde_json`'s
`preserve_order` feature (`gpui-pre-0.3.6/Cargo.toml:448`), so in the
desktop and `--all-features` builds a `serde_json::Map` keeps insertion
order. The encoder therefore inserts every object's keys already sorted,
as `discovery.rs:275` does, and never relies on the map type (ruling R18):

```json
{
  "format": "manyhands-draft",
  "version": 1,
  "id": "01J...",
  "revision": 7,
  "created_at": "2026-10-20T14:02:11Z",
  "flushed_at": "2026-10-20T14:05:40Z",
  "target": {
    "repository": "/home/u/work/project",
    "item_kind": "ticket",
    "item_id": "01M...",
    "comment": null
  },
  "base": {
    "observation": "…",
    "path": ".manyhands/tickets/01M.../ticket.md",
    "context": { "kind": "primary", "branch": "main", "worktree": "/home/u/work/project", "head_oid": "…" },
    "source": "---\n…",
    "fields": { "kind": "ticket", "title": "…", "type": "…", "status": "…", "project": null, "team": null, "deps": [], "parent": null, "body": "…" }
  },
  "current": { "kind": "ticket", "title": "…", "type": "…", "status": "…", "project": null, "team": null, "deps": ["01M..."], "parent": null, "body": "…" }
}
```

- `target.repository` is the registration root as the repository list
  returned it, the desktop's `RepoKey`.
- `target.comment` is `{ "comment_id": "…", "parent_id": "…" | null }` for
  a comment draft, whose `item_kind`/`item_id` are the commented item's and
  whose `base` is `null`.
- `base` is `null` for a comment and for a future new item (D3).
  `base.observation` is `null` when the base is unverified (ruling R5).
  `base.source` is `null` when the file was not UTF-8.
- `fields.kind` is `document` (`title`, `body`), `ticket` (`title`, `type`,
  `status`, `project`, `team`, `deps`, `parent`, `body`) or `comment`
  (`body`). Values are stored as typed; an invalid value stays in the draft.
- Times are UTC RFC 3339.
- `revision` counts every change to what the file must hold, of its text
  or its base. It never decreases for one draft.

**Decoding** reads `format` and `version` first. `version` 1 is parsed in
full. A greater version is `Unreadable(NewerFormat { version })`. A missing
or wrong `format`, invalid JSON, invalid UTF-8, an unknown kind or an
invalid ID is `Unreadable(Damaged)`. A decode never rewrites or removes the
file. For later versions the rule is fixed now: an older version is
migrated in memory and written in the new version only on the next write
by its claim holder; a failed migration is `Unreadable` and the file is
untouched (ruling R7).

### Directory and file protection (decisions 3 and 5)

- **Unix (decision 5).** The library creates the data directory with the
  user's umask, so it is commonly 0775 under umask 002. A group-writable
  data directory is accepted. One that is writable by others (`o+w`), or
  owned by another user, is `ProtectionUnavailable`.
  - The store opens the data directory with
    `O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC`. It makes `drafts/` with
    `mkdirat(…, 0o700)`, or opens the existing one with `openat` and the
    same flags, and keeps that `drafts/` handle open for its lifetime.
  - On the handle, `fstat` must show a directory owned by the effective
    user. Group and other bits are cleared with `fchmod(mode & 0o700)`,
    which never adds a bit, so a 0500 folder stays 0500. A link fails the
    open; a link or another owner is `ProtectionUnavailable`.
  - Every later operation is relative to the handle: `openat` with
    `O_NOFOLLOW` (and `O_CREAT | O_EXCL`, mode 0600, for new files),
    `renameat`, `unlinkat` and `fsync` of the handle. A group member who
    renames or swaps `drafts/` therefore cannot redirect a write. At worst
    they move the folder away, which hides drafts but cannot expose them.
    This follows `keys/storage/unix.rs:21-30`.
  - If the handle's folder has been removed (`openat` answers `ENOENT` for
    a new file), the store reopens `drafts/` once, with the same checks,
    and retries the step.
  - A draft file that is not a regular file is `Unreadable(NotAFile)`; one
    owned by another user is `Unreadable(NotOwned)`.
- **Windows (decision 3).** `drafts/` inherits the ACL of `%LOCALAPPDATA%`;
  no DACL is written. Operations are path based. A reparse point
  (`FILE_ATTRIBUTE_REPARSE_POINT` from `MetadataExt::file_attributes`) is
  `ProtectionUnavailable` for the folder and `Unreadable(NotAFile)` for a
  draft file. The folder's owner (`GetSecurityInfo`,
  `OWNER_SECURITY_INFORMATION`) must equal the process token's user or its
  default owner (`GetTokenInformation`, `TokenUser` and `TokenOwner`), so an
  elevated administrator whose default owner is the Administrators group is
  accepted (ruling R4). Otherwise `ProtectionUnavailable`. Draft files
  inherit the folder's ACL and their owner is not checked one by one.

The folder is checked when the store opens and before every write. A
same-user adversary is outside the threat model, as it is for the key
store.

### Writing a revision

`DraftClaim::write(revision, current, base)` is atomic replacement:

1. Re-check the folder; encode.
2. Create `.<draft-id>.<random>.tmp` in `drafts/` (0600, exclusive).
3. Write all bytes; `sync_all`.
4. Rename over `<draft-id>.json` (`renameat` on Unix; on Windows
   `std::fs::rename`, which replaces an existing file).
5. Sync the folder (`fsync` of the handle on Unix). Windows has no folder
   sync, so step 5 does nothing there; its failure point is still reached,
   so the tests are the same on every platform.

Any failure removes the temporary file if it can and returns a typed
`DraftError`. The previous `<draft-id>.json` is untouched. Success returns
the written revision and `flushed_at`. Only the claim holder writes (next
section).

A failure in step 5, after the rename, is still reported as a failed
write: the new revision is in place but not proven durable, so the session
does not count it as flushed and writes again.

Failure points for tests, set with
`DraftStore::with_failure_point_for_testing(point, action)`: `point` is
`CreateTemp`, `Write`, `SyncFile`, `Rename` or `SyncFolder`. `Fail(kind)`
makes that step return the error instead of running. `Crash` ends the
process with `std::process::exit(CRASH_EXIT_CODE)` right **after** that step
has run (for `Write`, after half the bytes), without unwinding, so nothing
the store holds is cleaned up. It is deterministic, leaves no core dump,
and works on every platform (ruling R8).

### Claims and locks

A claim is the right to write, retire and discard one draft. It holds an
exclusive advisory lock (`fs4`, `try_lock_exclusive`) on
`<draft-id>.lock` (0600) for as long as it lives.

- `DraftStore::create(target, base, current)` makes a new ULID, takes its
  lock and writes revision 1.
- `DraftStore::claim(id)` takes the lock without waiting; held elsewhere is
  `DraftErrorKind::OpenElsewhere`, and held by this store already is
  `DraftErrorKind::HeldHere`. It then reads the file and removes that
  draft's stray temporary files, which only a dead writer can have left.
- Dropping a claim releases the lock. The lock file stays and is harmless.
- The store, and every clone of it, shares one set of the IDs it holds
  claims on. `flock` and `LockFileEx` conflict between two handles of one
  process, so `list` never probes a held ID. It reports `Held::Here` for
  those, and probes the rest by taking and releasing the lock at once
  (ruling R19).
- Only a claim holder ever deletes a lock file, and only after it has
  deleted the draft file. `list` never deletes a lock file, even one with
  no draft. Otherwise another process's `list` could unlink a lock between
  `create`'s lock and its first write, and a later `claim` would lock a new
  file beside the first: two writers (ruling R20). Orphan lock files are
  tiny and stay.

Two processes can each create a draft for the same item. Both are listed
and both kept; uniqueness is per process (ruling R9).

### Listing

`DraftStore::list() -> Result<Vec<Listing>, DraftError>` reads every
`<ULID>.json` in `drafts/` and returns one `Listing` each:

```rust
pub struct Listing { pub id: DraftId, pub entry: Entry, pub held: Held }
pub enum Held { No, Here, Elsewhere }
pub enum Entry { Draft(Draft), Unreadable(UnreadableReason) }
pub enum UnreadableReason { Damaged, NewerFormat { version: u64 }, Access, NotOwned, NotAFile }
```

Entries are ordered by `flushed_at`, latest first, then ID; unreadable ones
follow, by ID. Names that are not `<ULID>.json`, `<ULID>.lock` or a
temporary file are not the store's and are ignored.

### Retirement, discard and raw export

- `DraftClaim::retire(saved_revision) -> Result<Retired, DraftError>`
  deletes the draft file only if the stored revision equals
  `saved_revision`, then its lock file, then releases the lock. Otherwise
  it returns `Retired::Kept(claim)`. The desktop flushes a revision before
  saving it, so the stored revision is the saved one whenever nothing newer
  was typed (ruling R21).
- `DraftClaim::discard()` deletes the draft file, then the lock file, then
  releases the lock. Rust opens files with `FILE_SHARE_DELETE` on Windows,
  so the held lock file can be deleted there too.
- `DraftStore::discard_unreadable(id)` takes the lock and deletes the file;
  it is how an unreadable entry is discarded.
- `DraftStore::raw_bytes(id)` returns the file's bytes for a raw export.

### Errors

```rust
pub struct DraftError { pub kind: DraftErrorKind }
pub enum DraftErrorKind {
    Unavailable,           // no application data directory
    ProtectionUnavailable, // link, reparse point, another owner, other-writable parent
    Access,                // permission denied
    NoSpace,               // disk full or quota
    OpenElsewhere,         // the draft's lock is held by another process
    HeldHere,              // this store already holds the draft's claim
    NotFound,              // the draft is gone
    Io,                    // anything else
}
```

A `DraftError` carries no path, no text and no underlying error message.
`Display` gives a fixed sentence per kind. It derives `Debug`; the types
that hold draft text (`Draft`, `DraftFields`, `DraftBase`) do not.

## Comparison And Target State

Pure functions in `src/drafts/compare.rs`, tested without a window.

### Fields of an item

`DraftFields::of(&ItemDto) -> Option<DraftFields>` takes the editable
fields of a conforming item (`None` for a nonconforming entry). A ticket's
`deps` are the IDs of `ItemDto.deps` in file order; its `parent` is
`ItemDto.parent`'s ID. A dependency the library ignored is not in
`ItemDto.deps`, which the save's `RelationshipWrite::Unchanged` preserves
(the save section below).

### Three-way comparison

```rust
pub enum Change { None, Mine, Theirs, BothSame, Both }
pub struct Comparison { pub fields: Vec<(Field, Change)> }  // in a fixed field order, body last
pub fn compare(base: &DraftFields, theirs: &DraftFields, mine: &DraftFields) -> Comparison
```

Per field: `None` when nothing differs from the base; `Mine` when only the
draft differs; `Theirs` when only the current item does; `BothSame` when
both changed to the same value; `Both` when they changed differently.
There is no text merge. With an unverified base, the comparison uses the
base fields it holds.

### Target state

```rust
pub enum TargetState {
    Current,                                // base observation equals the read's
    Changed { theirs: DraftFields, comparison: Comparison, context_changed: bool },
    AlreadySaved,                           // the item's fields equal the draft's
    Unverified { theirs: DraftFields, comparison: Comparison },
    TicketClosed,
    ItemNotFound,
    Nonconforming,
    RepositoryUnavailable(ResultCode),      // not registered, inaccessible, not a repository
    ReadFailed(ResultCode),                 // busy, index unavailable: try again
    // comment drafts only
    ParentNotFound,
}
pub fn classify(draft: &Draft, item: Result<&ItemDto, ResultCode>) -> TargetState
pub fn classify_comment(draft: &Draft, item: Result<&ItemDto, ResultCode>,
                        comments: Result<&CommentListDto, ResultCode>) -> TargetState
```

Order of decision for an item draft: a read error maps to
`RepositoryUnavailable`, `ItemNotFound` or `ReadFailed` by code; then a
lifecycle-closed ticket is `TicketClosed` whatever else holds; then a
nonconforming read is `Nonconforming`; then fields equal to the draft's are
`AlreadySaved`; then an unverified base is `Unverified`; then an equal
observation is `Current`; otherwise `Changed`, with `context_changed` when
the read's context kind or branch differs from the base's. A comment draft
is `ItemNotFound`, `TicketClosed`, `RepositoryUnavailable`, `ReadFailed` or
`ParentNotFound` as its item and thread say, otherwise `Current`. Comments
are never edited in this Wave, so a comment draft cannot conflict with
canonical text; its only staleness is whether its target can still take it
(ruling R10). A comment list with `complete: false` is `ReadFailed`, never
`ParentNotFound`. The list holds only what the index holds, so a parent
written since the last refresh also reads as gone until a refresh. The
label says so.

### Restore policy

| Target state | Restore | Notes |
| --- | --- | --- |
| `Current` | Allowed | |
| `Changed`, `Unverified` | Allowed, flagged | The tab shows the comparison; D3's Save needs a review first. |
| `AlreadySaved` | Allowed | Discard is offered first: nothing would be lost. |
| `TicketClosed` | Refused | Wave: never restore into a closed ticket. The library refuses to save one too (`ticket_closed`); export keeps the text. |
| `ItemNotFound`, `Nonconforming`, `ParentNotFound` | Refused | |
| `RepositoryUnavailable` | Refused | Restore becomes possible if the repository is registered again at the same root. |
| `ReadFailed` | Refused for now | Offers Retry. |
| any comment draft | Refused in D2 | "The comment composer arrives in a later release." |
| open elsewhere, unreadable | Refused | Export is offered. |
| held here (restored in this window) | Not needed | "Show" focuses its tab. |

Export and Discard are offered in every state except that Discard is not
offered for a draft open elsewhere. A draft held here is discarded through
its session.

### Export

`export::markdown(&Draft) -> String` renders a Markdown file: a YAML front
matter of the draft's named fields, with no `id` and no `manyhands_managed`,
then the body. Without an ID it cannot be taken for an item; placed under
`docs/` it is a nonconforming entry. A comment draft exports its body only.
An unreadable entry exports its raw bytes. The desktop writes the chosen
path through a temporary file in the same folder and a rename, so a failed
export never truncates an existing file. The platform save dialog asks
about replacing a file (ruling R11). On Unix the exported file is created
0600; the user chose where it goes and can widen it.

## Desktop

### Services

`Backend` gains
`fn drafts(&self) -> Result<&DraftStore, DraftStoreUnavailable>`, where
`DraftStoreUnavailable(DraftErrorKind)` is the desktop's copy of why the
store did not open. `LibraryBackend::new` takes the store's open result
beside the service: production passes `DraftStore::open_default()`, and
the test fixture passes `DraftStore::open_in` of its temporary data
directory. The service is still opened by `RepositoryService::open_default`
(ruling R2). If the store cannot open, the desktop still starts. The
Drafts page shows the store's fixed reason, the operation area keeps a
persistent entry, and no session can be created.

If the service itself cannot open, D1 shows only its startup-failure
window. Drafts are then kept but unreachable until the service opens. That
is recorded as a limitation; the startup window gains no Drafts page.

Store calls are blocking file I/O and run on the background executor,
like reads. Three `ReadKind`s are added, so the `DelayingBackend` can hold
each with an async gate: `Drafts` (list and claim), `DraftWrite`, and
`DraftItem`, the registry's re-read after a save. `DraftItem` is separate
from `Item` because a save also makes D1 reload the repository's tabs,
whose `Item` reads would contend for one gate. Two `JobKind`s are added,
`DocumentSave` and `TicketSave`.

`DraftRegistry` is one entity in `Services`, shared by the workspace,
tabs and the Drafts page. It holds:

- the sessions, by draft ID, and which tab each is attached to;
- the last listing, with a generation for stale results;
- save requests in flight, by `DesktopRequestId`, each with its draft ID,
  revision and the fields submitted.

The event task in `Services::start` that drains worker events into
`Operations` also passes each `Finished` event to the registry. The
registry outlives tabs, so a save that completes after its tab closed
still retires its revision.

### Sessions

```rust
pub(crate) struct DraftSession {
    id: DraftId,
    target: DraftTarget,
    base: Option<DraftBase>,
    current: DraftFields,
    revision: u64,                // newest in memory
    flushed: u64,                 // newest written
    writing: Option<u64>,         // in flight
    failure: Option<DraftErrorKind>,
    first_unflushed_at: Option<Instant>,
    last_change_at: Option<Instant>,
    saving: Option<(DesktopRequestId, u64)>,
    claim: Arc<Mutex<Option<DraftClaim>>>,   // taken out to retire or discard
}
```

- Every `now` is `cx.background_executor().now()`, never `Instant::now()`,
  so `advance_clock` drives the schedule in tests.
- `edit(fields, now)` raises `revision` and records the times. D3's editor
  calls it. In D2 only tests do. A base change after a save
  (`rebase(base, now)`) raises `revision` too and is flushed at once, not on
  the timer.
- `DraftRegistry::begin(target, base, current)` creates a draft and its
  session. D3 calls it on the first real edit. In D2 only tests and the
  seeding example create drafts.
- `flush_due(now) -> Option<Instant>` is
  `min(last_change_at + 1 s, first_unflushed_at + 5 s)` when
  `revision > flushed`, else `None`. The view schedules a GPUI timer for it
  (ruling R6).
- At most one write is in flight. When it lands, `flushed` becomes the
  written revision and `failure` clears. If newer text arrived meanwhile,
  the next write is scheduled for the newest revision only. Intermediate
  revisions are never written.
- A failed write sets `failure`, keeps `flushed` and is retried on the next
  edit, on the user's Retry, and on a close or quit flush. There is no timed
  retry loop.
- `recovery_state()` is `Saved(at)` when `flushed == revision` and no
  failure; `Pending` while within the schedule; `Failed(kind)` otherwise.
  The UI claims safety only for `Saved`.
- `flush_now()` writes the newest revision now, used by close and quit.

A session is created by `DraftRegistry::begin` (D3, when the first edit
happens) or by Restore, which claims an existing draft. It ends when its
draft is retired or discarded, or when it is released by a kept close.

### Save completion

`DraftRegistry::save(session)` first flushes: it waits for any write in
flight, then writes the newest revision N if it is not yet stored. Only
when N is stored does it submit (ruling R21). If that flush fails, nothing
is submitted and the flush failure is shown. `save` returns `None` without
doing anything when a save of this session is in flight, when the base is
missing (a new item is D3's) or when the base is unverified: an
unverified base has no observation token to send, and D3 must offer the
review first. It submits revision N's fields and the base observation as:

- for a document, `Intent::DocumentSave { root, id, title, body, observation }`;
- for a ticket, `Intent::TicketSave { root, id, title, ticket_type, status,
  project, team, body, deps, parent, observation }`, where `deps` and
  `parent` are `Unchanged` when they equal the base's, `Clear` when empty
  and the base had some, and `Set` otherwise.

It submits on the user lane and records `(request, draft, N, fields)`.
Neither save needs confirmation. A session has at most one save in flight
(ruling R15). Otherwise a second save would carry the base from before the
first one's commit and answer `external_change`. D3's Save control is
disabled meanwhile. The `Intent` holds the body and, like every intent,
derives no `Debug`. F2's request records hold no body.

On `Finished`:

| Envelope | Draft |
| --- | --- |
| `checkpoint` is `committed`, or `unchanged` with `success` or `noop` | Re-read the item (`ReadKind::DraftItem`, with the busy retries reads use). If the read gives exactly the submitted fields, the new base is that read. Otherwise, or if the read fails, the new base is the submitted fields with no observation (unverified). Wait for any write in flight to land. Then if the session's revision is still N, the claim is taken out and retired, and the session ends. If it is newer, the session keeps its text and is rebased, which raises the revision and flushes at once. The save record and `saving` clear. |
| `external_change` | The draft is untouched. The tab re-reads and shows `Changed`. The record and `saving` clear. |
| `ticket_closed` | Untouched; the tab shows `TicketClosed`. The record and `saving` clear. |
| `checkpoint` `pending` | Untouched. The file was written but not committed, and F2 holds a pending operation. The record and `saving` stay. The operation area offers Retry under the same request ID, and that completion applies this table again. `saving` also clears if the user dismisses the request or abandons the pending operation (ruling R22). |
| anything else | Untouched. The record and `saving` clear. The operation area shows the request as D1 does. |

A completion for a draft that was discarded meanwhile is ignored. A
Retry of a finished save under the same request ID is answered from F2's
record. Its record is gone by then, so the completion changes nothing.

**Known limit (D6).** If quitting is already waiting in D1's Finishing
phase when a save finishes, the window quits before the re-read and the
rebase flush land. The draft file then keeps revision N+1 with the old
base, and the next launch classifies it `Changed` or `AlreadySaved`. No
text is lost. D6's shutdown contract replaces this path.

If the desktop stops after the commit but before retirement, the next
launch classifies the draft `AlreadySaved`, and Discard loses nothing.

### Tabs

- **Banner.** When an item tab loads, the registry is asked whether the
  last listing holds a draft for that target (repository root, kind, ID)
  that this process has not claimed. If so the tab shows "A recovered draft
  of this item is kept", with **Restore** and **Review in Drafts**. Nothing
  is restored automatically.
- **Restore** claims the draft in the background, creates the session,
  attaches it to the tab (opening it if needed) and focuses the tab. The
  tab is opened through `Workspace::open_item` with
  `ItemLocator::Identified { kind, id }.in_repository(&repo)`, the one
  place that builds an `OpenTarget` (ruling R25). If
  another session of this process is attached to the same target, Restore
  is refused with "Another draft of this item is open". A draft removed
  since the listing reads "This draft is no longer kept", and the listing
  reloads. Restore writes nothing to the repository.
- **Restored draft section.** Above the item's own view: the draft's
  fields read-only (title, named metadata, body); its recovery state as
  text ("Saved for recovery at 14:05", "Not yet saved for recovery",
  "Recovery copy not saved: <reason>" with Retry); its target state as text
  (labels below); **Compare** when the state has a comparison, which expands
  the comparison inline; **Export…**; **Discard…**. "Edit" and "Save" are
  disabled controls reading "Available with the editor in a later release".
- **Target-state labels**, shared by the tab and the Drafts page:
  `Current` "The item is unchanged since this draft began"; `Changed` "The
  item changed since this draft began" (plus "and is now read from another
  copy" when `context_changed`); `AlreadySaved` "The item already holds this
  draft's text"; `Unverified` "The item may have changed since this draft
  was saved"; `TicketClosed` "The ticket is closed"; `ItemNotFound` "The item
  is gone"; `Nonconforming` "The item's file is not a valid item";
  `RepositoryUnavailable` "The repository is not available"; `ReadFailed`
  "The item could not be read" with Retry; `ParentNotFound` "The comment it
  replies to is gone".
- **Comparison view.** One component renders a `Comparison` as a row per
  field with "Your draft", "The item now" and "When the draft began" and
  the change as text. The tab's Compare and the page's Inspect both use it.
- **Refresh.** D1 reloads every open tab of a repository after a refresh.
  With a session attached, the reload re-reads the item and recomputes the
  target state; it never touches the session's text or base.
- **Close.** Closing a tab whose session is attached opens a dialog with
  **Keep draft** (the default: flush, then release), **Discard** (scoped to
  this draft), **Cancel**, and **Save** disabled with the D3 note. A flush
  failure on Keep leaves the tab open with the failure shown. The tab's
  Close button and `secondary-w` both reach `Workspace::close_tab` in
  `view/workspace/content.rs`, which asks the registry before closing.
- **Repository removal.** D1's removal closes the repository's tabs. With a
  session attached, it flushes and releases it; it never discards. Both
  paths that close a repository's tabs, a removal's completion
  (`workspace/requests.rs`) and a repository leaving the list
  (`workspace/repositories.rs`), reach `Workspace::close_repository` in
  `content.rs`, which does this before the tabs close. The
  removal confirmation (`view/confirm.rs`, shared by the settings, sidebar
  and recovery removal paths) adds a line: "Drafts of this repository's
  items stay in Drafts."

### Quit

`request_quit` (`view/workspace/lifecycle.rs`) gains a phase before D1's,
`QuitPhase::SavingDrafts` (`view/quit.rs`, which renders its status and
Quit now): if any session has
`revision > flushed` or a write in flight, quitting first flushes them
all and shows "Saving drafts for recovery" with **Quit now**, so a hung
write never traps the user. When all succeed, D1's quit proceeds. If any
fails, a dialog reads "N drafts could not be saved for recovery" with
**Quit anyway** and **Cancel** (the default). The window's close request
takes the same path. Flushing happens before GPUI is allowed to quit,
because `on_app_quit` gets at most 200 ms (`gpui-pre` `app.rs:75`). D6 later
replaces this path with its shutdown contract.

### Drafts page

`OpenDrafts` (`secondary-shift-d`) shows the page in the content region,
in place of the tabs, as Settings is shown. The operation area's
"Recovered drafts" entry and the tab banner open it too.

**Routing (ruling R23).** The page joins the Structure split's content
router in `view/workspace/content.rs`, not a new `Workspace` field:

- `ContentPage` gains `Drafts`. `ContentArea` holds the cached
  `Entity<DraftsPage>`, made with the workspace, as it holds Settings'.
  The page owns its shown state, as `SettingsHost` does; no other flag
  records it.
- The selector is Settings when Settings is shown, else Drafts when the
  page is shown, else Tabs. Every transition leaves at most one shown:
  `open_drafts` hides Settings; each Settings entry point in `content.rs`
  hides the page first; `show_tabs` hides both. So Back to tabs from
  Settings opened over Drafts shows the tabs.
- Leaving Keys for Drafts clears an unsent passphrase, as leaving it for
  the tabs does: the Drafts arm of `content_body` calls
  `forget_passphrase` too.
- `secondary-w` does nothing while the page is shown, as with Settings.
  Tab stepping and `open_item` already show the tabs.
- `focus_content` focuses the page's first control.
- The page scrolls with the content region's one `ContentScroll`, as
  Settings does. Whether each page keeps its own offset is a question for
  `01M5F3GVDYTH1Q5V5FCS9YF7V3`; D2 follows what main has when it lands.

- **Rows** come from a `Drafts` read of the store. Each row's target state
  then comes from its own reads (`resolve_repository`, `show_item`, and for
  a comment `list_comments`) under that row's stamp, so a late answer never
  lands on a newer listing. A row shows its kind ("Document", "Ticket",
  "Comment on <item title>"), the draft's title, the repository root, the
  time of its last recovery save, and its state as text, never by color
  alone.
- **Row types (ruling R24)**, in `model/drafts.rs`:

  ```rust
  pub(crate) struct DraftRow {
      id: DraftId,
      kind_label: String,       // "Document", "Ticket", "Comment on <item title>"
      title: String,            // the draft's own title, shown only in its row
      repository: String,
      flushed_at: String,       // local time of the last recovery save
      state: RowState,
      actions: RowActions,
  }
  pub(crate) enum RowState {
      Readable { held: Held, target: Read<TargetState> },
      Unreadable(UnreadableReason),
  }
  pub(crate) struct RowActions {
      restore: bool,            // restore_policy allows it
      show: bool,               // held here
      inspect: bool,            // readable
      discard: bool,            // not held elsewhere
      retry: bool,              // target ReadFailed
  }
  pub(crate) fn row_actions(state: &RowState, restore: Option<Restore>) -> RowActions;
  ```

  `target` is `Read::Loading` until the row's reads land, reading
  "Checking the item", and then `Ready`. Read errors are classified into
  the target state (`ReadFailed`, `RepositoryUnavailable`,
  `ItemNotFound`), so the page never assigns `None`, `Busy` or `Failed`,
  and renders them as `Loading`. While `Loading`, a row offers Inspect,
  Export… and Discard… only. Export is offered on every row: Export… for
  a readable draft, Export raw… for an unreadable one. `Read<T>` derives
  nothing, so a `DraftRow` holding draft text has no `Debug`.
- **Unreadable rows** read "This draft cannot be read" with the reason
  ("saved by a newer Manyhands", "damaged", "not accessible", "owned by
  another user", "not a file") and offer Export raw… and Discard….
- **Open elsewhere** rows read "Open in another Manyhands window" and offer
  Inspect and Export only.
- **Held here** rows read "Restored in this window" and offer Show, which
  focuses its tab, Inspect, Export… and Discard… (through the session).
- **Inspect** opens a panel with the draft's fields and, when the state has
  one, the comparison view. The body is shown as its three blocks, with no
  merge.
- **Export…** opens the platform save dialog (`prompt_for_new_path`) with a
  suggested name from the title, then writes as the Export section says.
- **Discard…** opens a confirmation dialog naming the draft. Confirm
  discards that draft only; Escape or Cancel discards nothing and returns
  focus to the row's Discard.
- **Empty** reads "No drafts are kept." A store that cannot open reads its
  fixed reason.

### Operation area

- **Recovered drafts.** After the launch listing, if any draft is stored
  that this process has not claimed, an entry reads "Recovered drafts: N"
  with **Review**. It can be dismissed for the session and is shown again
  at the next launch.
- **Store unavailable.** A persistent entry with the store's fixed reason.
- **Flush failure.** A persistent entry per failing session, "Recovery copy
  of <title> not saved: <reason>", with **Retry**. It goes away when a
  later flush succeeds.
- Save requests appear as D1's requests do.

The entries' Review and Retry call named `Workspace` actions:
`open_drafts` in `workspace/content.rs` and `retry_draft_flush` in
`workspace/requests.rs`, beside D1's request actions.

No entry, notification or message shows draft body text. A title shown
outside the draft's own row and section is the item's title as read,
never the draft's. It is rendered as tab titles are, with
`TicketName { slug: None, title }.title_or(id)` from `model/names.rs`,
falling back to the item ID (ruling R25). A save's envelope `message`, which the operation area
shows, is checked by the privacy sweep with a save the library refuses as
`invalid_input`.

### Keyboard

| Key | Action |
| --- | --- |
| `secondary-shift-d` | Open Drafts. |
| Tab / shift-Tab | Move through rows and their actions, window-wide (D1 ruling R-T17a). |
| Enter | Activate the focused control. |
| Escape | Close the Inspect panel or a dialog; focus returns to the control that opened it. |

Every control is a labeled, focusable Kit button. Dialogs trap focus.

## Testing

### Library

Unit tests beside each module, and `tests/drafts.rs` with submodules
`tests/drafts/{format,write,crash,access,claims,targets}.rs`. Every test
uses its own temporary data directory and the isolated Git home.

- **Format.** A golden version 1 file under `tests/fixtures/drafts/`
  decodes and re-encodes to identical bytes, both without features and
  with `--all-features`, where `preserve_order` is on. A version 2 file, invalid
  JSON, a truncated file, a wrong `format` and an invalid ID are each listed
  unreadable with their reason, and their bytes are unchanged afterwards.
- **Write.** `Fail(kind)` at `CreateTemp`, `Write`, `SyncFile` and `Rename`
  leaves the previous revision readable and no temporary file. At
  `SyncFolder` the new revision is in place and the write still reports
  the failure. `NoSpace` is injected; a real ENOSPC is not produced.
- **Crash.** The parent writes revision 1 through a claim. A child process,
  the test binary re-run with an environment variable, claims the draft,
  sets a `Crash` failure point and writes revision 2. The parent then lists
  revision 1 for `CreateTemp`, `Write` and `SyncFile`, and revision 2 for
  `Rename` and `SyncFolder`, each readable and whole. The next claim removes
  any stray temporary file. The child's exit releases its lock.
- **Access (Unix).** An unreadable file (`0o000`) is `Unreadable(Access)`.
  A read-only `drafts/` (`0o500`) makes a write fail `Access`, keeps the
  previous revision and stays 0500 after the store reopens it. A symlinked
  `drafts/` and an other-writable (0777) data directory are
  `ProtectionUnavailable`. A group-writable (0775) data directory is
  accepted. A `drafts/` at 0755 is tightened to 0700. Swapping `drafts/`
  for another folder after the store opened does not redirect a write: the
  draft lands in the folder the store holds. The ownership decision is a
  function of the metadata values and is unit-tested with a foreign UID,
  since tests do not run as root. On a host running the tests as root, the
  permission tests detect it and fail with a message, never skip silently.
- **Claims.** A child process holding a claim makes `claim` answer
  `OpenElsewhere` and `list` report `Held::Elsewhere`. After the child exits
  the draft can be claimed. A claim held by this store is `Held::Here` in
  `list` and `HeldHere` from a second `claim`. `retire` deletes only the
  matching revision. `discard` removes only its own draft. A lock file with
  no draft survives `list`.
- **Targets.** Over real repositories made through the library: `Current`;
  `Changed` after a save by a second `RepositoryService` in a child process;
  `AlreadySaved`; `TicketClosed` after a local-only closure through
  `preview_ticket_closure` and `close_ticket` with no publication remote
  and a credential provider that answers `Unavailable` (no transport is
  used, as `closure/run_tests.rs:119-125` shows), which also removes the
  item worktree; `Changed` with `context_changed`
  after the context is removed (ruling R12); `ItemNotFound` after the file
  is removed and the index refreshed; `RepositoryUnavailable` after
  registration removal and after the folder is renamed away;
  `ParentNotFound` for a reply whose parent is not in the thread; and
  `ReadFailed` for a comment list with `complete: false`. After each
  of registration removal, `rebuild_repository`, and deleting the index file
  then reopening the service, `list` returns the same drafts with the same
  bytes.

### Desktop

Model tests need no window. View tests use the Kit test support over real
temporary repositories and a real store in the fixture's data directory,
as D1's do (`desktop-test-support`).

- **Schedule.** `flush_due` under one edit, a burst inside one second, and
  continuous edits for twelve seconds (writes due at five and ten).
  Coalescing: three edits during a held write produce one more write, of
  the newest revision.
- **Recovery state.** The truth table of `Saved`, `Pending` and `Failed`. A
  view test holds a `DraftWrite` and shows "Not yet saved for recovery"
  until it is released.
- **Save completion.** Edit and save at once, with no clock advanced: the
  save first flushes N, and on completion the draft file and lock file are
  gone. With the worker's `before_job(DocumentSave)` held, the session is
  edited to newer text; on release the draft holds that text with its base
  moved, and a second save from that base succeeds. The same with a write
  of the newer text still in flight at completion. The same for
  `TicketSave`, with `deps` unchanged and the ignored dependency kept in
  the file. A second actor in a child process saves between the commit and
  the re-read (held with a `ReadKind::DraftItem` gate): the base is
  unverified and the state `Unverified`. A stale save answers
  `external_change` and leaves the file byte-identical. A completion after
  Discard is ignored. A save with a `pending` checkpoint, made by the
  library's `FailurePoint::AfterOwnedWriteBeforeLifecyclePersistence` through
  `DelayingBackend::replace_service`, keeps the draft and blocks a second
  save. Retry under the same request ID then commits once and retires the
  revision. `save` returns `None` for an unverified base.
- **Tabs.** The banner appears for a stored draft and not for a claimed
  one. Restore attaches the draft; the repository snapshot is unchanged. A
  refresh and a second-actor save leave the text unchanged and show
  `Changed`. Close offers the four choices; Keep releases; Discard deletes
  only that draft; Cancel keeps the tab. Repository removal keeps the
  draft and the page lists it `RepositoryUnavailable`.
- **Quit.** With a held write, quit waits, offering Quit now, then quits.
  With a failing write, the dialog offers Quit anyway and Cancel.
- **Routing.** `secondary-shift-d` shows the page; Settings opened over it
  and then Back to tabs shows the tabs; leaving Keys for Drafts clears an
  unsent passphrase; `secondary-w` while the page is shown closes no tab;
  Restore and Show hide the page and show the tab.
- **Drafts page.** One test per state over real fixtures, rendering its
  label and its allowed actions. Inspect shows the comparison. Export
  through `simulate_new_path_selection` writes the Markdown; an export to a
  folder that cannot be written shows its failure and leaves an existing
  file unchanged. Discard confirms; Escape discards nothing; focus returns.
  A held listing never lands after a newer one.
- **Open elsewhere and held here.** A child process holds a claim; the row
  offers only Inspect and Export. A draft restored in this window reads
  "Restored in this window", never "open elsewhere".
- **Privacy.** A sentinel string in a draft's body is in no operation entry,
  notification, dialog message or `Debug` output of a store error,
  including after a save the library refuses as `invalid_input`.
- **Keyboard.** Each action above driven by keystroke, with Escape and
  focus return, added to `keyboard_tests.rs`.

### Proof map

| Cycle contract | Proof |
| --- | --- |
| 1 Last flushed revision survives | Library format, write, crash and access tests. |
| 2 Only the saved revision retires | `retire` unit test; desktop save-completion tests with real saves. |
| 3 Drafts outlive their targets | Library targets tests; desktop removal test. |
| 4 Never silently overwritten | Desktop tab tests; restore snapshot equality; launch restores nothing. |
| 5 Truthful recovery state | Recovery-state table; held-write view test; flush-failure entry test. |
| 6 One writer per draft | Claims tests (held elsewhere, held here, orphan lock kept); open-elsewhere and held-here view tests. |
| 7 Protected and private | Access tests; privacy sentinel; Windows by native CI. |
| 8 Keyboard first | Keyboard tests; native checklist. |
| 9 No display dependency | `scripts/check-cli-no-gpui.sh`; `src/drafts/` imports no `gpui_kit`. |

### Native checklist (Linux Wayland, product owner)

Added to D1's checklist, using `devenv shell -- cargo run --locked --example seed_draft -- <root> <ULID>`
to write a draft for an item of this project's repository:

1. Launch: the operation area shows "Recovered drafts: 1"; Review opens
   Drafts.
2. Drafts by keyboard: Inspect, Escape, Export to a file, open the file in
   an editor.
3. Open the item: the banner shows; Restore; the section shows "Saved for
   recovery".
4. Change the item with the CLI (`ticket save` or `document save`), then
   Refresh: the draft is unchanged and reads "The item changed"; Compare
   shows the field.
5. Close the tab: Keep draft; the draft is listed again.
6. Quit and relaunch: the draft is listed; Discard, confirm; it is gone.
7. `kill -9` the desktop with a draft restored; relaunch: the draft is
   listed.

## Alternatives Rejected

- **Drafts in SQLite.** The RFCs require files outside SQLite and journals,
  and the index is rebuildable cache.
- **One file per revision, or a journal of edits.** More recovery points
  than the RFC asks for, and pruning would be an age-based eviction in all
  but name. One file per draft, replaced atomically, is the RFC's model.
- **A backup copy of the previous revision.** Atomic rename with file and
  folder sync already keeps the last flushed revision; a second copy adds a
  second retirement rule.
- **Adopting the read's observation after a save without comparing.** A
  change by another program between the commit and the read would become
  the base, and the next save would overwrite it silently.
- **A shared-library change returning the written observation from saves.**
  Correct, but a separate ticket. Comparing the re-read's fields is safe
  without it (ruling R5). Revisit if D3 finds the re-read slow.
- **An interim plain-text editor.** Rejected by decision 1.
- **A strict owner-only DACL on Windows.** Rejected by decision 3.
- **Retargeting a draft to a moved repository.** Not required; export
  covers it.
- **Making the desktop single-instance.** Out of scope; claims handle two
  processes.

## Risks

| Risk | Control |
| --- | --- |
| Windows rename fails while another program (an antivirus) has the file open | The write fails with a typed kind and is retried on the next edit or Retry; the previous revision stays. Native CI after merge. |
| Windows owner check rejects a legitimate profile, for example under folder redirection | Accept the token user or its default owner (ruling R4). A refusal is visible: the store is unavailable and editing in D3 must say crash recovery is off. Native CI and G1. |
| A permission test run as root passes vacuously | The test detects root and fails with a message. |
| Structure ticket `01M5CS4MJ0` reshapes `workspace.rs` and `item.rs` | Merged before Part B (decision 4, `d7962c3`); paths follow its handoff map (ruling R23–R25). |
| `01M5CS4MH2` edits the item view, lists and tabs at the same time | Merge main at work boundaries. |
| `01M5F3GV` changes the settings pages and perhaps the shared content scroll | Merge main at work boundaries; the Drafts page follows main's scroll policy. |
| Flush on quit outlasts the user's patience, or hangs, on a slow disk | Quit now is offered while drafts are being saved; Quit anyway after a failure. |
| The comparison is not enough to review a long body | D3 owns the editor's review; D2's side-by-side is for recovery. |
| D2 grows too large | Three Parts with checkpoint reviews; the product owner may split a Part at its checkpoint. |
| The Windows protection module first compiles in the post-merge native run (no cross target in Devenv) | Keep it small and modeled on `keys/storage/windows.rs`; a native failure is fixed as follow-on work under `AGENTS.md`. |
| Deleting a held lock file behaves differently on Windows | Rust opens files with `FILE_SHARE_DELETE`; the claims tests run natively after merge. |
| The service cannot open, so drafts are unreachable | Recorded limitation; drafts are kept and reachable once the service opens. |
| Encoding order differs between feature sets | Keys inserted pre-sorted (ruling R18); the format test runs with `--all-features` too. |

## Review Record

Reviewed against the Wave, the governing RFCs and register item W3-03, the
provisional API audit, the D1 Cycle, design and follow-up tickets, and the
source at `3bda743`. Classifications follow `review-cycle-docs`. On
2026-10-21 the desktop sections were remapped onto the Structure split at
`d7962c3`, read from source and from the Structure plan's handoff map
(rulings R23–R25).

An independent reviewer then read all three documents against `AGENTS.md`,
the Wave, the RFCs, the library and desktop source and the GPUI Kit, GPUI
Component and `gpui-pre` 0.3.6 sources, by reading only, on 2026-10-20. It
reported one blocking finding and eighteen others. Each was checked against
source before it was applied. Rows naming a finding record the result.

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |
| Wave D2 scope; no editor before D3 | Nothing in production creates a draft in D2. What UI ships? | Decision | Cycle decision 1: recovery surface. | Drafts page tests; native checklist with the seeding example. |
| Wave "all domain behavior in the library"; track rule 5; `AGENTS.md` | Where the store lives. | Decision | Cycle decision 2: `src/drafts/`, one line in `src/lib.rs`. | Module layout; CLI isolation. |
| Desktop and index RFCs "owner-protected"; key store not reusable | Windows protection. | Decision | Cycle decision 3: inherit the profile ACL; refuse reparse points and other owners. Residual risk recorded. | Access tests; native CI. |
| `01M5CS4MJ0` "before D2/D3 add pages" | Sequencing against the structure follow-up. | Decision | Cycle decision 4; satisfied on 2026-10-21 (`d7962c3`). | Cycle entry evidence. |
| R23, Structure plan "D2 extends the private content boundary"; `workspace/content.rs` derives the page from `SettingsHost::is_shown()` | How a second page joins the router without a second flag that can disagree. | Ruling | `ContentPage::Drafts`; the page entity owns its shown state; every transition leaves at most one page shown; the Drafts arm clears the passphrase as the tabs do. If wrong, the router's one file changes. | Routing view tests. |
| R24, plan Task 10's `RowState` and `RowActions` were named but not defined | The page's row types. | Ruling | Defined in the Drafts page section, with the target state as the Structure split's `Read<TargetState>`. If wrong, the model's row types change; the store is untouched. | Drafts page tests, one per state. |
| R25, Structure split's `TicketName` and `ItemLocator` | How D2 names items outside a draft's own row and opens their tabs. | Ruling | Item titles as tab titles show them (`title_or(id)`, no short code); tabs opened through `ItemLocator::in_repository`. | Privacy sweep; Restore and Show tests. |
| Ticket and track rule 3 say D2 consumes only F1 | D2 consumes D1 and F2 too. | Settled | All three are closed and merged; track rule 1 holds. | Cycle entry evidence. |
| Local main ahead of `origin/main` | Planning base. | Settled | Planned on `origin/main` per `AGENTS.md`. Main was published, and `d7962c3` was merged into the branch on 2026-10-21. | Cycle entry evidence. |
| R1, desktop RFC "at most one second of inactivity" | Continuous typing never goes idle. | Ruling | Also flush at most five seconds after the first unflushed change. This strengthens the RFC's guarantee. If wrong, it costs one constant. | Schedule tests. |
| R2, private `default_data_directory` | The store needs the data directory. | Ruling | Compute the same `ProjectDirs` path in `src/drafts/`; the service keeps `open_default`, so the desktop and CLI registries can never split. Divergence would only move drafts. If wrong, a shared-library change makes the function public. | Startup and fixture tests. |
| R3, Unix permissions on an existing folder | Refuse or tighten looser modes. | Ruling | Clear group and other bits on our own `drafts/` (never add bits); refuse a link or another owner. The parent rule is decision 5. If wrong, refusing instead is one branch. | Access tests. |
| Independent review, finding 6; `repository.rs:3901` `create_dir_all` | Refusing a group-writable data directory turns recovery off under umask 002. | Decision | Decision 5 (2026-10-20): accept group-write; refuse other-writable or foreign-owned; fd-relative operations on a held `drafts/` handle. | Access tests, including a swap of `drafts/`. |
| R4, Windows default owner of elevated processes | An admin's objects may be owned by Administrators. | Ruling | Accept the token user or its default owner. If wrong, a wrongly refused store is visible and native CI shows it. | Native CI. |
| R5, saves return no new observation | How the base moves after a save with newer text. | Ruling | Re-read; adopt only if the fields equal the submitted ones; else unverified. This is a correct use of the documented read, not a workaround. If wrong, a shared-library change returns the token. | Base-after-save test with a second actor. |
| R6, GPUI timers versus a store thread | Who schedules flushes. | Ruling | The session computes the due time; the view schedules a GPUI timer; writes run on the background executor. Testable with `advance_clock`. If wrong, move scheduling to a store thread. | Schedule and held-write tests. |
| R7, RFC "migration failure preserves the original" with only version 1 | How to prove migration. | Ruling | Version dispatch now; newer and damaged files kept byte for byte. The first real migration arrives with version 2 and must add its own test. If wrong, the fixture rule is added then. | Format tests. |
| R8, crash simulation | Signal timing versus a deterministic stop. | Ruling | `Crash` failure points in a child process, ending it with `process::exit` right after the step and without unwinding. That avoids core dumps and Windows error reporting, and changes nothing about the files compared with an abort. Power loss is not simulated. | Crash tests. |
| R15, two saves in flight | A second save made before the first completes carries a stale base. | Ruling | One save in flight per session. | Save-completion tests. |
| R16, base change after a save | A rebase with no new text would never be written. | Ruling | `revision` counts base changes too. | Save-completion tests. |
| R17, desktop entry points only D3 calls | `begin`, `edit` and `save` are dead code in the D2 release build. | Ruling | `#[cfg_attr(not(test), allow(dead_code))]` with a comment naming D3, the repository's convention (`desktop/mod.rs:155`). D3 removes it. | `--features desktop` clippy. |
| R18, independent review finding 2; `gpui-pre` enables `serde_json/preserve_order` | A `Value`-based encode is ordered differently with and without features. | Ruling | Insert keys pre-sorted, as `discovery.rs:275` does; test the golden file under `--all-features` too. | Format test, both feature sets. |
| R19, finding 4; `flock`/`LockFileEx` conflict within one process | `list` would report this process's own claims as open elsewhere. | Ruling | The store shares a set of held IDs across clones and reports `Held::Here` without probing. | Claims and held-here view tests. |
| R20, finding 5 | `list` deleting orphan lock files can give two writers. | Ruling | Only a claim holder deletes a lock file, after the draft file; orphans stay. If wrong, add an inode check after locking. | Claims test: orphan survives `list`. |
| R21, finding 1 (blocking) | Saving an unflushed revision could never retire it. | Ruling | `save` flushes N before submitting; retirement waits for any write in flight. Contract 2 stands as written. | Edit-and-save-at-once test; write-in-flight-at-completion test. |
| R22, finding 7; track rule 7 | What a `pending` checkpoint leaves behind, and how it ends. | Ruling | Keep the save record and `saving`; Retry under the same request ID applies the rule again; dismissing or abandoning clears it. | Pending-then-Retry test with the library's failure point. |
| Independent review, findings 3 and 8–19 | `Unverified` versus `Changed` after a disturbed save; contended re-read gate; clock source; text over revision numbers; save from an unverified base; closure fallback; `complete: false`; privacy wording; quit paths; verify commands; Cycle misstatements; Windows file checks; error-type naming; per-task ticket comments. | Settled | Each corrected in the three documents on 2026-10-20 after checking it against source. | This revision. |
| R9, two processes, one item | Duplicate drafts for one target. | Ruling | Allowed and both listed; one session per target per process. If wrong, a store-wide lock on create. | Claims tests. |
| R10, Wave "comparison models for items and comment composers" | What staleness means for an append-only comment. | Ruling | Target compatibility only (item gone or closed, parent gone, repository unavailable). If wrong, D4 adds a thread-changed signal. | Targets tests. |
| R11, export overwrite | Truncating an existing file on failure. | Ruling | Write through a temporary file and rename; the platform dialog confirms replacement. | Export failure test. |
| R12, "newly incompatible context" | Which contexts block restore. | Ruling | A closed ticket blocks. A changed context (a removed item worktree, a promoted document) is `Changed` with `context_changed`; restore is allowed and flagged. If wrong, refuse restore there too. | Targets tests. |
| R13, `deps`/`parent` on save | `Set` would drop dependencies the library ignored. | Ruling | `Unchanged` when equal to the base. | Ticket save-completion test. |
| R14, native checklist without an editor | How the owner sees a draft by hand. | Decision | Development example `seed_draft`; accepted by the product owner on 2026-10-21 (Cycle decision 6, formerly Q2). | Native checklist. |
