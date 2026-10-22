# Desktop Structure Implementation Plan

> **For Claude:** After owner approval and explicit implementation authorization,
> use `implementing-a-cycle` and `executing-plans` to implement this plan
> task-by-task. Work in the existing ticket worktree; merge main, never rebase.

**Goal:** Land the behavior-preserving desktop Structure refactor that unblocks
D2, with shared read/name/target helpers, private Workspace state and smaller
files organized by responsibility.

**Architecture:** Keep the existing GPUI entities, models, worker and subscriptions.
Extract cohesive implementation modules behind private state. Views use named
commands and read-only queries; one small content router selects tabs or Settings.
Shared presentation and item-address helpers live in GPUI-free desktop model code.

**Tech Stack:** Rust 2024, existing `gpui-kit` dependency through `gpui_kit::*`,
existing desktop test support and Devenv. No new dependency.

**Status:** Plan and design approved by the owner on 2026-10-21; the owner then
explicitly authorized inline implementation in this session. The delivery split
is also approved. Implementation and automated verification are complete;
independent review found no actionable findings. The owner approved closure,
local merge, publication and Structure branch/worktree cleanup on 2026-10-21.
The native interactive walkthrough remains unconfirmed.
See the execution ledger below for evidence and remaining lifecycle steps.

## 1. Scope, base and delivery

- Ticket: `01M5CS4MJ0G7B0MPJD0KN0QMYA`, **Desktop follow-up: structure from the D1 reviews**.
- Branch: `manyhands/ticket/01M5CS4MJ0G7B0MPJD0KN0QMYA`.
- Worktree: `.manyhands/worktrees/01M5CS4MJ0G7B0MPJD0KN0QMYA` under the primary checkout.
- Fetched main: `0b2366453cb0228aaeb341c51a2dad963df4990f`.
- Ticket checkpoint `857a735` was merged with main in `8fd42e7`. A fresh fetch
  during planning found the same main; `git merge origin/main` was a no-op.
- Scope split approved by the owner: all non-Structure findings are retained in
  `01M5F3GVDYTH1Q5V5FCS9YF7V3`, **Desktop follow-up: settings UX from the D1 reviews**.
  That ticket has its own branch/worktree based on `0b23664` and starts after this
  refactor merges. Its form-clearing and future UI-change questions remain open.
- The tests-and-polish sibling `01M5CS4MH25QQ769HV61ZRKHXG` was still at its
  initial checkpoint `c116a1a`, clean, when inspected. Recheck before coding.
- D2 (`01M4CC0VMZ21Z79B09YQRA0P7J`) Parts B/C wait for this Structure change on
  main. Its proposed design and implementation File Map were read in its own
  worktree. Do not edit them here; D2 Task 5 remaps them after this merge.

The scope is `src/desktop/`, this plan and ticket records. No shared-library,
CLI, manifest, lockfile, published schema or CI change is planned. Do not implement
drafts, change settings behavior, fix sibling findings or add a page framework.
In particular, preserve the current shared content scroll offset, existing
Settings open/read behavior, tab element IDs and focus behavior. Their follow-up
tickets own changes to those behaviors.

## 2. Approved design

### Approach

**Recommended: responsibility extraction with a method boundary.** Keep the
current entity ownership and callback timing. Move Workspace orchestration into
private child modules; keep view rendering beside its view. Turn its fields
private and have external view modules use read-only queries and explicit actions.

Alternatives considered:

- **File moves plus field getters only:** smaller diff, but mutable getters would
  preserve the coupling and allow views to change state without its transitions.
- **Make every region a new entity/controller:** stronger isolation, but changes
  subscriptions, notification and focus lifetimes. Too broad for this refactor.

The recommendation deliberately retains existing `impl Workspace` render helpers
where convenient. The boundary is enforced by private fields, not by changing all
render functions into a new framework. Private Workspace children may access its
state; sibling views may not. Do not relocate whole sibling views under Workspace
merely to gain access to its private fields.

### Shared read state

Create `src/desktop/model/read_state.rs` with one `Read<T>`:

```rust
pub(crate) enum Read<T> {
    None,
    Loading,
    Ready(T),
    Busy,
    Failed(ResultCode),
}

impl<T> Read<T> {
    pub(crate) fn from_result<U>(result: ReadResult<U>, f: impl FnOnce(U) -> T) -> Self;
}
```

Use the existing `ReadResult` conversion unchanged. `None` preserves repository
management's no-selection/not-started state; it is not Loading or an empty result.
Other pages never assign `None`; their rendering handles it without starting a
read or inventing a result. Keep their existing initial states. Do not replace
`ListState<T>`: it carries completeness, index state and row retention that this
small read state does not. No GPUI import is needed in the new module.

### Shared names and open targets

Create `src/desktop/model/names.rs`. Use one borrowed helper for the two optional
name components and their existing fallback/rendering rules:

```rust
pub(crate) struct TicketName<'a> {
    pub slug: Option<&'a str>,
    pub title: Option<&'a str>,
}

impl<'a> TicketName<'a> {
    pub(crate) fn title_or(&self, fallback: &'a str) -> &'a str;
    pub(crate) fn short_or(&self, fallback: &'a str) -> &'a str;
    pub(crate) fn with_short(&self, fallback_id: &str) -> String;
    pub(crate) fn parts(&self) -> impl Iterator<Item = &'a str>;
}
```

`title_or` and `short_or` use `Option::unwrap_or`, without trimming, normalizing or
treating an empty string as missing. `with_short` joins the short code (or ID)
and present title with one space; with no title it returns the short code or ID.
`parts` yields the present slug then title; punctuation remains with the caller.
Adapters on `TicketRow` and `RelatedRow` expose this helper without changing the
fields their existing tests inspect. Use it in each naming site, preserving:

| Site | Existing presentation to retain |
| --- | --- |
| Ticket list | Title or path as row label; separate short-code column with its existing missing-code label. |
| Find matches | Title, else ID, else path; short code remains a separate optional cell. |
| Relationship rows | Optional slug and title cells; accessible summary stays comma-separated, followed by lifecycle, resolution and ID. |
| Tab titles | Last successfully read title, else item ID or inspection path. A failed refresh retains the title; no short code is added to tabs. |
| Open-dependency reasons | Short code or ID, plus present title. |
| Cycle/unresolved reasons | Cycle uses short code or ID; unresolved uses the raw reason IDs. |

Keep `OpenTarget::for_item(repo, item)` in `model/navigation.rs` as the single
item-DTO entry point. Factor its address selection into a small GPUI-free
`ItemLocator` in that same module: `Identified { kind, id }` or
`Inspect { worktree, path }`. `ItemLocator::for_item` performs the current parse
and fallback, and `in_repository` is the one place constructing the two
`OpenTarget` variants. `OpenTarget::for_item` delegates to those two operations.

`TicketRow::from_item` stores this locator privately alongside its existing
display fields, so `TicketRow::target` delegates rather than reimplementing the
selection rule or retaining an entire `ItemDto`. Relationship rows, which have
an ID rather than an item DTO, validate their existing resolved/parse conditions
then use the same identified-locator conversion. Find keeps its current refusal
to open an inspection target from a short-code match. Documents continue using
`OpenTarget::for_item`. Retain these current entry-point signatures for callers
and tests. A slug is never an identity or tab key.

### Workspace boundary

Make **every Workspace field private**. Preserve current test-facing methods
and their behavior. Promote an existing query out of `#[cfg(test)]` only when a
production view actually needs it; unused test hooks remain test-only.

| Consumer | Allowed queries and actions |
| --- | --- |
| `view/lists.rs` | Read-only `navigation()`, `documents()`, `tickets()`, `documents_pane()`, `tickets_pane()`, `choice_focus(id)`; actions `show_pane`, `load_lists`, `set_closure`, `set_readiness`, `open_selected_document`. Read scheduling and result application move inside Workspace. |
| `view/find.rs` | `find_input()`, `find_state()`, `find_match_focus(index)`; actions `focus_find`, `submit_find`, `open_match`. Generations, read tasks and clearing stay inside Workspace. |
| `view/tabs.rs` | Own its tab state and expose operations on `TabContent`; callbacks call Workspace's `show_tab`, `close_tab`, `open_item`. No Workspace field access or mutable content getter. |
| `view/sidebar.rs` | Keep explicit read-only rendering inputs and current selection/settings/remove commands. It already largely follows this pattern. |
| `view/operations.rs` | Read through `services()` and retain `control_focus_for`/`abandon_focus_for`; call `cancel_request`, `dismiss_request`, `run_recovery`, `resume`, `abandon`, `load_pending`. Request routing and focus-map lifecycle stay inside Workspace. |
| `view/quit.rs`, `view/passphrase.rs` | Keep their view/types and dialog behavior. Move their Workspace orchestration into `workspace/lifecycle.rs`; quit rendering uses phase/text/focus queries. |

Queries return `&T`, scalar values or cloned focus handles, not `&mut` models,
task slots or state maps. Existing entity handles are exposed only where needed
for rendering or existing tests. Do not add a general state-update closure.
Use the narrowest useful visibility (`pub(super)` for sibling view calls,
`pub(in crate::desktop::view)` from nested implementation modules when needed).
Keep the existing externally used `pub(crate)` entry points.

Rename the existing `tabs::Content` to `TabContent`. It continues owning the
tab model, item entities, relationship subscriptions and focus handles. Add
methods `open`, `activate`, `close`, `close_repository`, `step`, `reload`,
`shown`, `shown_focus`, `scroll` and `render`, moving the corresponding code
from its Workspace impl. Methods that create listeners retain
`Context<Workspace>` so entity/subscription ownership does not change.
Workspace wrappers coordinate page visibility, focus and notifications in the
same order as today. Avoid double `notify` or moving task/subscription drops.

### One content-page switch

Replace Workspace's separate `content` and `settings` fields with a private
`ContentArea` containing the existing tabs and Settings entity. Route display,
content focus, Back to tabs, open-item, tab stepping, close-shown-tab and all
settings recovery entry points through `workspace/content.rs`.

Use a concrete `ContentPage { Tabs, Settings }` selector there. For this refactor
it is derived from `SettingsHost::is_shown()`; do not cache a second visibility
flag that can disagree with Settings' own `Shown` state. The one selector is
matched for rendering and focus, with named `show_tabs`/settings-opening methods
for transitions. Keep Settings' existing `Closed` event and cached page entities.

The selector and private content holder are the extension point for D2: it adds
its Drafts module and page variant/routing inside this boundary rather than more
top-level Workspace fields. This ticket adds no draft state, trait-based page
registry, generic page lifecycle or new page entity. D2 owns the state required
to select and retain its additional page.

Preserve the same `ContentScroll` handle and all current scroll offsets. Preserve
the current timing of `forget_passphrase`, including when leaving Keys for tabs
or another Settings page. Centralizing routing must not change the secret lifetime.

## 3. Target file map

Paths below are relative to `src/desktop/`. Keep the existing parent `.rs` files
as module facades, so existing import paths and test module paths stay stable.
New implementation children are private; expose only what their parent or a
named sibling needs. Use explicit imports, not new GPUI glob imports.

| File | Responsibility after the refactor |
| --- | --- |
| `model/read_state.rs`, `model/names.rs` | Shared read state and name components/fallbacks. Register in `model/mod.rs`. |
| `model/navigation.rs` | Existing navigation plus the shared locator/target construction. |
| `view/workspace.rs` | Private state declarations, construction/subscriptions, stable public/test facade and child module declarations. |
| `view/workspace/repositories.rs` | Repository list, selection/resolution, list-completion handling. |
| `view/workspace/lists.rs` | Document/ticket read scheduling, filters, result application, clear/reload, Kit list synchronization. |
| `view/workspace/find.rs` | Find input actions, stamped reads, result application and activation. |
| `view/workspace/content.rs` | Tabs/Settings router, settings recovery entry points, tab command wrappers and content focus/scroll queries. |
| `view/workspace/requests.rs` | Pending reads, operation-change handling, index maintenance, recovery/confirm/resume/abandon, request focus-map lifecycle. |
| `view/workspace/focus.rs` | Region definitions, focus/cycling/fallback and region decoration. Re-export current Region/REGIONS paths. |
| `view/workspace/lifecycle.rs` | Existing quit and passphrase-prompt orchestration, with existing ordering and deferred prompt opening. |
| `view/workspace/render.rs` | Shell assembly, action bindings, content scrolling and dialog/sheet/notification layers. |
| `view/{lists,find,tabs,operations,sidebar,quit,passphrase}.rs` | Existing view rendering and view-owned state, using the boundary above. |
| `view/settings/keys.rs` | KeysPage fields, constants, construction/subscriptions, stable entry-point/test facade. |
| `view/settings/keys/reads.rs` | Key listing and public-key copy reads, stamps and copy outcome. |
| `view/settings/keys/commands.rs` | Generate/import/select/clear/remove/delete, validation, secret extraction/clearing, protected retry and operation completion handling. |
| `view/settings/keys/render.rs` | Rows, forms, results, buttons and Render implementation. |
| `view/item.rs` | ItemPane state, construction, constants, facade and top-level Render composition. |
| `view/item/reads.rs` | `read_tab`, stamped loading/reloading and refresh completion handling. |
| `view/item/status.rs` | Loading/busy/failure/not-refreshed rendering and current recovery controls. |
| `view/item/details.rs` | Identity/copy controls, provenance, fields, problems, metadata and shared text/field rendering helpers. |
| `view/item/source.rs` | Source textarea synchronization and rendering; retain the same entity and selection. |
| `view/item/discussion.rs` | Comment rendering and later-release unavailable actions. |

The three formerly large parent files and their new children should each stay
roughly below 500 lines. This is a review target, not permission to fragment a
cohesive routine arbitrarily. Explain any exception at the checkpoint.

Keep `view/relationships.rs` and existing `*_tests.rs` in place. Parent facades
re-export helpers such as `item::text` where existing siblings use them. Adjust
visibility deliberately: a child-private method is not callable from its parent
or sibling without the appropriate visibility.

## 4. Implementation tasks and checkpoints

All commands run from the Structure ticket worktree. Steps are sequential;
the proposed execution method is inline, with owner review at the checkpoints.
Do not spawn implementers without authorization. Commit only when authorized;
each authorized commit ends with
`Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

### Task 1 — Record the baseline

1. Read current `AGENTS.md`, ticket and this plan. Fetch and **merge** current
   `origin/main`; verify ancestry. Check sibling progress and re-audit source
   changes since `0b23664`. Preserve the ticket split and pending planning edits.
2. Record the desktop test inventory before any Rust change:
   `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop:: -- --list`.
   Record names and count in the ticket evidence. The previous fix reported 362
   desktop tests; the fresh inventory, not that remembered count, is authoritative.
3. Read the affected existing model/view/keyboard tests as the behavioral baseline.
   No new behavior is requested, so use those tests rather than manufacture a
   red test. Do not add, delete, ignore, weaken or rewrite test bodies here,
   except the five owner-approved mechanical accessor changes recorded below.
4. Record plan approval and the base/inventory in a ticket comment. If an uncovered
   behavior needs new test bodies, ask for a scoped exception before proceeding.

### Task 2 — Consolidate read state, names and targets

**Create:** `model/{read_state,names}.rs`.
**Modify:** `model/{mod,navigation,tickets,relationships}.rs`,
`view/{tickets,find,relationships,item}.rs`, `view/settings/{mod,keys,remotes,hosts}.rs`,
`view/settings/repositories/manage.rs`. Audit `model/{find,documents}.rs` callers.

1. Move the two Read definitions into the shared union and update imports/matches.
2. Implement the name helper and replace each mapped naming site, preserving
   fallbacks, punctuation, separate cells, labels and empty-string handling.
3. Factor locator construction; retain current signatures, parse/refusal behavior
   and public row fields. Remove the duplicate target-construction matches.
4. Run the existing suites, each as a separate command:
   ```sh
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::model::
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::tickets
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::find
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::relationships
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::item
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::settings
   ```
5. Expect the same existing assertions to pass. Review the diff for label changes
   tests might not cover. Record the shared-helper checkpoint on the ticket.

### Task 3 — Extract Workspace responsibilities and enforce its boundary

**Create:** the `view/workspace/` children in the file map.
**Modify:** `view/workspace.rs`, `view/{lists,find,tabs,operations,sidebar,quit,passphrase}.rs`.

1. Move repository/list/find/request/focus/lifecycle methods into their assigned
   private children. Keep subscriptions and callback owners in the same entities.
2. Add the narrow queries/actions listed above. Sibling views use them; internal
   orchestration retains direct access. Make every Workspace field private.
3. Move tab-owned operations into `TabContent` and preserve Workspace wrappers.
   Group tabs and Settings in `ContentArea`; implement the single page selector
   and routing. Keep all existing test-facing methods and Settings events.
4. Move shell rendering/action wiring. Preserve element IDs, region order, dialog
   trapping, focus restoration, request ordering, `defer_in` prompt timing,
   secret clearing, list/task cancellation and stale-result checks.
5. Run only these affected suites:
   ```sh
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::workspace
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::documents
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::tickets
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::find
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::tabs
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::operations
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::passphrase
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::settings
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::keyboard
   ```
6. Record an encapsulation checkpoint: private fields, no mutable state getters,
   external view call sites, routing locations and focused test outcomes. Review
   this boundary before continuing; a failure is fixed without weakening tests.

### Task 4 — Split the Keys page

**Create:** `view/settings/keys/{reads,commands,render}.rs`.
**Modify:** `view/settings/keys.rs`.

1. Move the existing code by the file map, retaining KeysPage's state/constructor
   and module identity. Retain constants and test accessors at their current paths.
2. Preserve passphrase extraction/zeroization, masked-input undo clearing,
   request/attempt bookkeeping, protected-generation retry and ordinary retry.
   Preserve focus-map keys, list reads and copy behavior exactly.
3. Run:
   ```sh
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::settings::keys
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::operations
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::keyboard
   ```
4. Record the split and privacy-sensitive move review on the ticket. No new
   secret-bearing Debug output or broader state visibility is acceptable.

### Task 5 — Split the item view

**Create:** `view/item/{reads,status,details,source,discussion}.rs`.
**Modify:** `view/item.rs`; adjust imports in `view/relationships.rs` if needed.

1. Move the read pipeline and each rendering responsibility; keep the root
   composition small and in the same order. Preserve `ItemPane`, its field/entity
   ownership, relationships event, `tab_title`, test hooks and helper paths.
2. Preserve the sequence of item/comment/dependency/children/parent reads and the
   same early returns, stamps and retained-state rules. Move source synchronization
   without recreating its textarea on unrelated renders or refreshes.
3. Run:
   ```sh
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::item
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::relationships
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::tabs
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::scroll
   devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands desktop::view::keyboard
   ```
4. Record the item split checkpoint. D2's future banner/section goes into the
   root composition and its own new module; add neither in this task.

### Task 6 — Verify, review and hand off

1. Fetch/merge main and inspect any new changes before final gates. Compare the
   test inventory with Task 1 using the exact same `-- --list` command. Require
   identical names/count, or only documented module-path moves; no lost tests.
   Review every test-file diff: only necessary import/path edits and the five
   owner-approved mechanical accessor changes recorded below are permitted.
2. Audit the source against this acceptance list:
   - one Read definition, outside Settings, retaining the not-started state;
   - every listed naming site uses the shared helper without changed output;
   - item DTOs and row targets share address construction;
   - private Workspace fields and named transitions, with no mutable back door;
   - one content router; no scattered direct Settings hide/is_shown calls in
     sibling views, and no second visibility flag;
   - responsibility-based splits and a D2 file map;
   - no source changes outside `src/desktop/`, no new dependency or test target.
3. Format through Devenv as needed, then run the full gates **once at the end**:
   ```sh
   devenv shell -- cargo check --all-features --locked
   devenv shell -- cargo fmt --check
   devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
   devenv shell -- cargo test --all-features --locked
   devenv shell -- cargo check --locked --features desktop --bin manyhands
   devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
   devenv shell -- cargo run --locked --bin manyhands-cli
   ```
   Confirm the full test output includes the unchanged desktop test inventory.
   If a tool timeout interrupts the full run, record the last completed target
   and obtain authorization for an exact continuation; never report a timeout as
   a pass. Repeat broad gates only for changed code or a concrete unresolved issue.
4. With an active display, run
   `devenv shell -- cargo run --locked --features desktop --bin manyhands`.
   Exercise Settings/Back to tabs, tab stepping/close, region focus and scrolling,
   Find and read-only item inspection. Do not mutate real repositories or keys
   for this smoke check. Record unavailable display access as not run.
5. Run `git diff --check` and `git merge-tree --write-tree HEAD origin/main`.
   Obtain independent code review via the owner's chosen reviewer; do not claim
   the planning audit below is implementation review. Record findings, fixes,
   exact commands/results and the review-ready checkpoint on the ticket.
6. Request owner approval and publication/merge authorization. Close this ticket
   only after review approval, in the final authorized pre-merge update. The UX
   ticket remains open. Native Build is a separate, explicitly authorized run
   **after merge on main**; local gates suffice for review readiness.
7. Once merged, tell the owner D2's Structure prerequisite is on main and provide
   the mapping below. Cleanup also requires authorization.

## 5. D2 handoff map

This is a handoff, not an edit to D2's proposed documents or additional D2 scope.

| D2 task/need | Post-refactor location |
| --- | --- |
| Task 5 services/backend and fixtures | `services/{mod,drafts}.rs`, `test_support/{fixtures,delaying}.rs` as D2 already proposes; unchanged by this ticket. |
| Task 6/10 operation entries | `view/operations.rs`; Workspace orchestration in `view/workspace/requests.rs`. |
| Task 8 banner/restored section | `view/item.rs` composition, beside the new `view/item/` responsibility modules. |
| Task 8 close choices | Workspace tab command wrappers in `view/workspace/content.rs`, tab-owned state/rendering in `view/tabs.rs`. |
| Task 8 repository removal | `view/settings/repositories/manage.rs` and `view/confirm.rs`; completion routing in `view/workspace/requests.rs`. |
| Task 9 quit flush | `view/workspace/lifecycle.rs`; quit status UI stays in `view/quit.rs`. |
| Task 10 Drafts page/action | New `view/drafts.rs`, `keys.rs`, and `view/workspace/content.rs` routing. |
| Draft read state and item names | `model/read_state.rs`, `model/names.rs`, `model/navigation.rs`. |

## 6. Decision and review record

Planning audit performed against the brief, ticket/split checkpoint, current
`AGENTS.md`, Wave 03 boundaries, D1 shell contract, D2's proposed design/File Map,
affected source/tests and `.github/workflows/build.yml`. This is a source/design
audit, not fresh test evidence. The workflow already runs the whole desktop binary
with `desktop-test-support`; moving modules needs no new CI test entry.

| Concern and source | Classification | Resolution and planned proof |
| --- | --- | --- |
| Structure must land before UX; D2 decision 4 | Settled | Owner selected split; both tickets cross-link and this one closes independently after review approval. |
| Name consolidation versus behavior preservation; current list/Find/tab/relationship code | Ruling | Share components/fallbacks, retain per-surface layout and accessible strings. Cost if wrong: revise the helper; no product copy change. Existing model/view assertions plus call-site diff audit. |
| Extra `Read::None` in repository management | Ruling | Preserve the union; do not collapse idle into Loading. Existing settings tests and state-transition diff audit. |
| Private fields without disrupting GPUI entity lifetimes | Ruling | Private child implementation modules and commands/read-only queries; no new controller entities. Cost if insufficient: another scoped extraction, not a changed event contract. Compile privacy checks plus current async/focus tests. |
| Tabs/Settings and future Drafts selection | Ruling | Concrete content router, current visibility derived from SettingsHost, no duplicated flag or speculative framework. D2 extends the private content boundary. Existing settings/tab/keyboard tests. |
| Keys/passphrase and quit changes could alter secret retention or prompt ordering | Settled constraint | Move existing code with the same clearing/defer/answer order; existing privacy, operations, passphrase, keyboard and quit tests remain intact. No trust/retry/security policy change. |
| Test-count preservation versus a refactor's new helper modules | Settled constraint | Reuse existing behavior tests and stable facade/test module paths. No new test functions, deletion, ignored tests or weakened assertions. Five mechanical field-to-accessor updates were explicitly approved during Task 3. Inventory before/after plus test diff review. |
| Desktop-only compile versus test-support build | Settled constraint | Both desktop-only check/clippy and all-feature gates; no test-only type leak. |
| Native platform/runtime evidence | Settled constraint | This planning session proves no native behavior. Existing four-target workflow is post-merge and separately authorized; record local display evidence honestly. |
| Persistence, timeouts and recovery behavior | Settled constraint | No changes to persistence, read retry bounds, worker outcomes, cancellation or restoration semantics; preserve request/task lifetimes and generation checks. |

**Owner approval (2026-10-21):** "The plan is approved." This covers the module
layout, helper semantics, Workspace boundary, behavior-preservation/test policy,
ordered tasks and D2 handoff above.
There is no additional product-policy question for the Structure scope; the UX
questions remain in its follow-up ticket. Implementation, commits, publication,
merge, cleanup and native CI remain subject to their applicable authorizations.

## 7. Execution ledger

- Implementation authorized: 2026-10-21, owner answered Yes to beginning inline
  implementation in this session. Commits/publication were authorized later,
  at the delivery checkpoint below.
- Task 1 complete: fetched main remained `0b23664`; ticket HEAD `8fd42e7` already
  includes it. Sibling `01M5CS4MH2` remains clean at `c116a1a`.
- Baseline compiled and listed through the exact Task 1 command: **362 desktop
  tests, 0 benchmarks**. Sorted `desktop::…: test` lines, joined with newlines
  and a final newline, have SHA-256
  `a20c47450f3a8613a7c0b90ff1fca93bddf53ab8e3bdbb39e65cc0bcdd291343`.
  Compare that inventory after refactoring; the listing is not a test pass.
- Task 2 complete: shared Read, TicketName and ItemLocator extracted. The six
  planned filters passed: model 89, tickets 9, find 8, relationships 8, item 15,
  settings 90 (219 tests total). No test files changed. `cargo fmt` and
  `git diff --check` passed. Evidence: comment `01M5F7NMG8SQ6E3F3GWC44W56X`.
- Task 3 complete: all Workspace fields are private; sibling views use queries
  and actions, TabContent owns its state, and one content router owns the
  tabs/Settings switch. `workspace.rs` is now 261 lines, with eight private
  responsibility modules of 93–305 lines. No new GPUI entity or visibility flag.
- Task 3's nine filters passed: workspace 17, documents 4, tickets 9, find 8,
  tabs 17, operations 12, passphrase 11, settings 90, keyboard 12 (180 total).
  Desktop-only check and clippy with `-D warnings` also passed, checking that
  production queries did not depend on test-support compilation.
- The desktop inventory after Task 3 is still 362, with the identical hash
  recorded above. Test diffs are exactly the five authorized accessor edits.
  `cargo fmt` and `git diff --check` passed. See comment
  `01M5F8RMBGVWVYKCKDY5EC8434` for the checkpoint.
- Owner approved the Tasks 1–3 checkpoint ("Looks good, continue."). Continuing
  with Tasks 4–6, including full all-feature gates and independent final review.
- Task 4 complete: KeysPage is 311 lines, with reads (74), commands (311) and
  rendering (353) children. All three planned filters passed: keys 18, operations
  12, keyboard 12. Formatting/diff checks passed; no further test edits.
  Evidence: comment `01M5F97BM8WBSCWVDHT13SC9BZ`.
- Task 5 complete: ItemPane is 150 lines, with details (310), reads (157),
  status (121), source (85) and discussion (97) children. The five planned
  filters passed: item 15, relationships 8, tabs 17, scroll 2, keyboard 12
  (54 total). Missing trait imports found on the first compile were restored;
  no runtime-test failure or additional test edit occurred.
  Evidence: comment `01M5F9J0ABY8D78M0HHPJ45JT7`.
- Task 6 base check: main remains `0b23664`; ancestry and the required HEAD
  merge-tree check passed. That check covers committed HEAD `8fd42e7`, not the
  still-uncommitted implementation; repeat after an authorized checkpoint commit.
  The completed refactor still lists the identical 362-test inventory/hash and
  exactly five single-line test accessor edits.
- Final all-feature check, fmt check and all-target/all-feature clippy passed;
  desktop-only check and clippy with `-D warnings` passed. CLI smoke passed.
- The full `cargo test --all-features --locked` invocation timed out after 30
  minutes during `document_promotion`, without reported failures. It completed
  1217 library, 362 desktop and 79 CLI unit tests plus four integration targets.
  The owner authorized bounded continuation of the interrupted and remaining
  33 integration targets and doc-tests. All continuation targets passed, as did
  11 doc-tests. The pre-existing opt-in characterization remains ignored. This
  is completed all-feature coverage across split runs, not one uninterrupted pass.
- Independent read-only whole-change review, including untracked modules,
  reported **no actionable findings** (session `ses_ea165ca8effewYGQl7mX0oM7sK`).
- Desktop launch smoke reached `Running target/debug/manyhands` on the active
  Wayland display, with no startup error reported, then stopped by the bounded
  SIGINT timeout. Verified no matching child remained. Native interactive
  Settings/tab/Find/focus/scroll walkthrough is not claimed; owner confirmation
  remains pending. Automated keyboard/tab/settings/scroll coverage passed.
- Final verification evidence: comment `01M5FEZ8SR37B1AG8A0PME8PXJ`. At that
  checkpoint the ticket was open and the changes uncommitted. Delivery approval
  and the implementation checkpoint are recorded below.

### Approved delivery (2026-10-21)

The owner approved completion, ticket closure, commit, local merge to main,
push to origin and removal of the Structure ticket's local/remote branches and
worktree. The ticket is closed in the final pre-merge update.

- Implementation and prior evidence committed as
  `6c8bd121a4a00e7f5ebe2333fdd3afae46f8b517`.
- Repeated `git merge-tree --write-tree HEAD origin/main` on that committed
  implementation: conflict-free tree `14081599eb3e2a926afddec754678f91c521d878`.
  Origin/main remains `0b23664`, already included. No Rust changes since testing.
- UX follow-up `01M5F3GVDYTH1Q5V5FCS9YF7V3` committed and pushed on its own
  branch as `64c231aeb310a162dfbaf437a626fdb5df59714c`; it remains open.
- Close/approval comment: `01M5FF701TRWQCYNTYAE0Y1T2T`. Verify origin/main
  contains the closure and exact tested source before removing the Structure
  branch/worktree. D2 consumes the file map above after the merge.
- Native CI remains separately authorized after merge. No native interactive
  walkthrough is claimed by the bounded launch smoke.

### Task 3 test-interface exception (owner-approved, 2026-10-21)

The first test compilation after making Workspace state private identified five
test-helper field accesses. The owner selected **Allow accessor updates** in
response to the scoped exception request. Change exactly these accesses:

- `view/settings/{hosts,keys,remotes,repositories}_tests.rs`: one
  `workspace.settings.clone()` each becomes `workspace.settings().clone()`.
- `view/tickets_tests.rs`: one `workspace.tickets_pane.rows(cx)` becomes
  `workspace.tickets_pane().rows(cx)`.

Add a test-only read-only Settings entity accessor; retain the same entity,
assertions and scenarios. The 362-test inventory remains a requirement. This
exception does not authorize any other test-body or assertion change.
