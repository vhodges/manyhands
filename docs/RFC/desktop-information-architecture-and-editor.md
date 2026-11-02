---
title: "Desktop Information Architecture and Editor RFC"
date: 2026-10-05
status: approved
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46S07YFKRH50D9NTHZ302AK"
---

# Desktop Information Architecture and Editor RFC

## Status and intent

This RFC was approved by the product owner on 2026-10-05. It defines the
desktop portion of Wave 03 under [PRD v0.5](../PRD/mvp.md) and the
[architecture RFC](mvp-rfc.md). The product owner selected **rich-text
editing with a Markdown source mode** during drafting on 2026-10-05.
The product owner also selected in-application canonical Markdown conflict
resolution with guided external recovery for other conflicts. The approved
design retains the feasibility and implementation gates recorded in the
[review register](wave-03-rfc-review.md).

The intended outcome is a complete collaboration workflow for a person who
does not operate Git. Desktop and CLI invoke the same headless domain services.
The [runtime RFC](application-runtime-and-polling.md) governs scheduling,
credentials, operation outcomes, and cancellation; the [CLI RFC](cli-contract.md)
defines the equivalent automation surface.

| PRD requirements | Desktop responsibility |
| --- | --- |
| `MH-PROD-001/002`, `MH-UX-001` | Repository navigation, multiple open items, understandable lifecycle actions. |
| `MH-REPO-001`–`004`, `MH-CRED-001` | Onboarding, local identity, remotes, shared keys and host approval. |
| `MH-CONTENT-001`–`004`, `MH-COMMENT-001/002` | Editing, organization, nonconforming content, ordered discussion. |
| `MH-COLLAB-001`–`007`, `MH-SCOPE-001` | Context setup, checkpoints, publication, recovery, confirmation and cleanup. |
| `MH-INDEX-001`–`003`, `MH-NFR-001`–`008` | Local-first discovery, fidelity, responsive keyboard workflows and native-platform evidence. |

## Navigation and item identity

One application window contains a repository selector, a navigation pane with
Documents and Tickets, an open-item tab strip, and the selected item's content
and discussion. Settings exposes repositories, remotes, Git identity, shared
SSH keys and host trust. A persistent operation area shows background work and
recovery; failures MUST NOT exist only in transient notifications.

Documents retain their folder hierarchy. Tickets use a list with title, type,
status and latest content/metadata change time. Both lists include visible
nonconforming and inaccessible entries with a reason and recovery action.
An empty result, a scan still running, and a failed/stale scan are distinct.
No boards, project rollups or configurable docking system are required.
Short codes, ready and blocked filters and a ticket's dependencies, parent and
children are shown as the
[ticket relationships and short codes RFC](ticket-relationships-and-short-codes.md#desktop)
defines; tabs and every mutation still key on the canonical ULID.

Ticket lists default to lifecycle-open tickets, with visible Closed/All filters.
Closure is determined by lifecycle metadata, independently of free-form status.
Opening a closed ticket directly by ID remains supported. Filtering does not
hide malformed entries needing recovery. This default was adopted with Wave 03
approval on 2026-10-05.

Tabs are keyed by resolved repository identity, item kind and canonical ULID,
not title or path. Opening the same item focuses its tab. Different items can
remain open across repositories. A tab shows unsaved state and its context;
the detail header exposes branch, worktree and primary/context provenance.
Active-context precedence follows the index RFC. Duplicate or mismatched
contexts show a recovery view, never a context chooser.

Viewing a primary item MUST NOT provision a worktree. Starting an edit or
comment provisions/reuses the one shared context and reports progress before
authoring is enabled. Navigation does not implicitly save, synchronize or
promote. A removed or inaccessible repository leaves its open drafts available
for recovery/export without modifying the repository.

## Rich-text and source contract

Rich text is the default body editor. Source mode exposes the exact Markdown
body; metadata has named controls in both modes. A separate inspect/repair
source view exposes complete canonical YAML and Markdown when conformity
requires repair. IDs and lifecycle-owned closure fields cannot be changed by
an ordinary metadata edit. Ticket status remains a free-form project value,
including the literal `closed`; changing it does not perform closure. Only the
explicit confirmed Close ticket action writes `closed_at`/`closed_by` and runs
integration/cleanup. The UI distinguishes project status from lifecycle closure.

The minimum rich-text vocabulary is paragraphs, headings, emphasis, strong
text, links, ordered/unordered/task lists, block quotes, fenced code and tables.
Source mode supports all valid UTF-8 body text accepted by the canonical layer.
The editor MUST retain the original source and map rendered edits back to it.
A source-first editor with display/source offsets or a block editor with source
spans can satisfy this contract; a custom parsed-block engine is not required.
Rich-text edits change only the affected source; rendering MUST NOT serialize
the entire document. Changed blocks may use documented canonical formatting,
but untouched blocks, unsupported syntax and unknown metadata values are preserved.

Examples of syntax outside that vocabulary include embedded HTML, custom
directives and extensions the renderer does not understand. They appear as
labeled source blocks with a source-edit action (amended for the pinned
editor by D3, below). If a construct cannot be
isolated safely, the affected region remains source-editable and read-only in
rich text. No operation silently drops or simplifies it. Switching modes,
opening a document and a no-change save write nothing and create no commit.

### Editor load normalization (amended 2026-10-07)

The product owner selected `zorite-editor` on 2026-10-07 and accepted the
byte changes its load normalization makes. This amends the preservation rules
above for the desktop editor only:

- When the editor loads a body, it may rewrite that body with the pinned
  editor release's own load normalization. For the evaluated pin, 0.10.0,
  that is math-fence canonicalization: a `$$…$$` pair mixed into a line of
  words is moved onto its own lines. See the
  [feasibility record](../research/wave-03-editor-feasibility.md#pinned-source-preservation-blocker-r3).
- Normalization is not a user edit. The host treats the normalized body as the
  clean baseline, so opening, switching modes and a no-change save still write
  nothing, create no commit and do not mark the item dirty. The canonical file
  keeps its original bytes until the user saves an actual edit.
- A save after an actual edit may write the normalized body. Bytes outside the
  edited region can therefore differ from the original where normalization
  applied. Undo returns to the normalized baseline, not the original bytes.
- The acceptance covers layout-only rewrites that keep the Markdown's meaning.
  A normalization that drops, truncates or simplifies content is still
  prohibited, and unsupported syntax still appears as labeled source (both
  points amended by D3, below).
- Metadata does not pass through the editor. Unknown front-matter values keep
  the protection of the canonical RFC.
- The set of normalizations is recorded for the pinned release. Changing the
  pin requires recording its normalizations again and reviewing any new one
  before adoption.

### D3 amendments (approved 2026-10-26)

The product owner approved these with the Wave 03 D3 Cycle on 2026-10-26.
The [D3 design](../plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md)
records the detail and the evidence, and its decision numbers are the
[Cycle's](../Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md#product-owner-decisions-2026-10-24).

- **The recorded load normalization** (decision 1). For `zorite-editor`
  0.10.0 the recorded set is wider than the 2026-10-07 amendment named:
  - the host first converts every CRLF, then every lone CR, to LF (N-1);
  - the editor then splits `$$…$$` pairs mixed into a line of words onto
    their own lines, pairs fences attached to words, and trims the
    whitespace around them (N-2, N-2b, N-3);
  - these rewrites apply inside inline code, `~~~` fences, indented code
    and HTML blocks too. Only backtick fences are exempt (N-4).

  The owner accepted the whole set ("I am okay if the document is mutated
  by the editor"), so its acceptance no longer depends on a rewrite being
  layout-only. Everything else in the 2026-10-07 amendment stands: it is
  not an edit, it is written only with a real edit, and a new pin records
  its set again.
- **Unsupported syntax** (decision 8). In the pinned editor, syntax outside
  the minimum vocabulary is shown literally and edited in place. There is no
  labeled source block and no source-edit action. A source-first editor has
  no parse step that can fail, so the failed-parse rule below does not
  arise. Nothing is dropped or simplified.
- **Local resources and links** (decisions 9 and 10).
  - Images: PNG, JPEG, GIF (first frame), WebP and BMP files inside the
    repository, never inside `.git` or `.manyhands`, never reached through
    `..`, a symbolic link leaving the repository, a backslash, a drive letter
    or a network path; size-capped and decoded within fixed limits. Remote
    images and SVG are never loaded; they show as their source.
  - Links: a relative link resolves in the linking item's own work context,
    then on the primary branch, then to the nearest managed document with
    the same file name; `[[Title]]` matches a document title, nearest first.
    Only `http`, `https` and `mailto` links leave the app, on a click.
- **Reload** (D3 ruling R-19). A clean editor that reloads keeps its cursor,
  clamped. The editor exposes no selection to restore.
- **Keyboard** (D3 ruling R-59). In the editor, Escape first closes the
  editor's own menu after a pointer press, and otherwise moves focus out of
  the editor to the item header. In the read-only view, Tab and Shift-Tab
  move focus on.

The CLI and the headless library do not load bodies through the editor; the
canonical RFC's preservation contract applies to them unchanged.

The existing canonical serializer's metadata formatting is not promised to be
byte-preserving after an actual metadata change; unknown values and body content
remain protected by the canonical RFC.

Both modes share one draft, selection mapping where possible, validation state
and undo history. Switching modes is not a save. A failed parse keeps the draft
in source mode with precise diagnostics (this does not arise in the pinned
editor; see the D3 amendments above). Toolbar actions and paste must remain
usable by keyboard. Rich paste converts only supported structure; plain-text
paste is always available. Spellchecking, collaborative cursors, embedded
webviews and arbitrary HTML execution are outside this RFC's scope.

Rendering does not execute HTML/scripts or automatically load remote images.
Relative document links resolve within the repository's permitted document
tree; external links require deliberate opening. Traversal, symlink escape,
executable URL schemes and private-key locations are not preview resources.
Images outside supported local resources remain visible source placeholders.
The D3 amendments above define the supported local resources and link
resolution.

## Drafts, saves and external changes

Each draft records its base path/content observation, base context and source,
current body/metadata and a local draft ID. Invalid edits remain in the draft;
they are never written to canonical files just to make Git report a dirty tree.
Save validates, checkpoints through the domain service and refreshes discovery.
Save does not publish. The UI distinguishes unsaved, saved locally, checkpoint
failed, discovery refresh pending and publication pending.

Crash recovery uses an owner-protected, application-local draft store
separate from SQLite, operation journals and canonical content. It writes
atomically after at most one second of inactivity, and flushes before a graceful
tab/application close. The UI reports when recovery persistence fails and never
claims unflushed text is crash-safe. An abrupt crash may lose edits since the
last successful flush; acknowledged saves remain protected by domain recovery.
Draft files contain necessary source/base text but never credentials. Their
schema is versioned; migration failure preserves the original file.

On restart, recovery offers inspect/restore, export and explicit discard.
Restoring does not overwrite canonical content. Successful save removes only
the draft revision proved checkpointed; newer typing survives. A close with
unsaved work offers Save, Keep draft, Discard and Cancel. Discard is explicit
and scoped to the selected draft. Drafts are not an authoritative item database,
and index rebuild/removal MUST NOT delete them. Files persist until successful
save or explicit discard; there is no silent age-based eviction.

Before every save, compare the observed base with canonical state through the
existing expected-observation boundary. An external edit or poll can change a
clean worktree while a draft is unsaved in memory. Therefore disabling only the
desktop's poller is insufficient. Keep the draft and its base, show the new
canonical version, and offer a three-way review. A clean open editor may reload
after preserving selection (the cursor, as amended by D3 above); a dirty
editor MUST NOT be replaced automatically.
Switching repository/tab or refreshing discovery does not bypass this rule.

Folder creation is an explicit local filesystem operation. Empty Git folders
are not portable content; the UI explains that a folder becomes shareable when
it contains a managed document. Moves preserve identity, use the existing
source/destination preconditions and checkpoint both paths. Nonconforming
documents get a previewable explicit repair; adoption/ID generation never occurs
as a side effect of browsing. Ambiguous duplicate identities block repair until
the user identifies the intended canonical resource.

## Discussion, lifecycle and recovery

Comments and replies use the same rich-text/source fidelity rules and durable
draft recovery. Root comments and each parent's direct replies follow canonical
creation-time ordering and its tie-break rule. Malformed threads remain visible.
Submission labels are **Post and sync** with a publication remote and **Save
comment locally** without one. The composer explains that synchronization can
publish other checkpointed work in that shared context. Unsaved item edits are
not included implicitly.

The composer allocates one comment ID per submission and retains it through
retry. After a checkpoint succeeds and publication fails, it shows the saved
comment once with Retry publication; it does not resubmit a new comment.

| Action | Required presentation |
| --- | --- |
| Save | Local checkpoint and discovery outcome; no implicit publication. |
| Sync item / Sync primary | Explicit scope, progress, published/current/pending/recovery result. Unsaved text remains separate. |
| Approve and merge document | Item, primary, remote or local-only state, final save, merge, publication and cleanup confirmation. |
| Close ticket | Same effect summary plus closed status and closure identity. |
| Poll now / Refresh discovery | Distinct remote poll and local index-only actions. |
| Retry | Show completed and remaining steps; reuse the original operation. |

Promotion and closure confirmation MUST enumerate the entire branch's effects,
including files beyond the item's Markdown. A ticket worktree may contain code.
Unexpected/dirty unrelated paths block unsafe integration or cleanup under the
Git policy. A new path, target or remote observation invalidates confirmation
and requires a new effect summary; retry of unchanged unfinished work can reuse
the durable consent boundary. The UI never reports cleanup success solely
because primary integration succeeded.

Conflicts have base/local/remote views and an editable result, with a path list
and explicit Mark resolved action. Accepting one side is deliberate per file;
there is no automatic winner. Canonical resolutions must validate and retain
expected observations before the domain service checkpoints them. Markdown
markers alone are not the definition of an unresolved Git conflict. A stale
resolution stays recoverable and requires a refreshed comparison.

The approved Wave 02 resolution boundary currently names conflicted owned
canonical paths. Noncanonical code, binary, rename/delete and structural Git
conflicts MUST remain preserved and visible. The UI may offer inspect/open
location and recheck after external repair, but MUST NOT claim this satisfies
in-application recovery for the required canonical journeys. External recovery
is the selected noncanonical scope recorded as W3-04 in the review register.
Recheck observes externally repaired Git state before offering a deliberate
resume; it never stages arbitrary code or silently publishes external changes.

## Onboarding and credentials

Add repository validates a local path; Create repository requires an empty,
writable location. Both show and require the selected primary branch. Missing
Git identity prompts for name/email and explicit local-repository persistence.
Repository removal explains that local files remain. Remotes can be listed,
added, removed and selected for SSH publication; non-SSH remotes remain visible.

Key settings support generation with optional masked passphrase, import by
reference, label/fingerprint display, public-key copy, selection, clearing and
unregistration. Generated-file deletion is a separate confirmed action; imported
key sources are never deleted. No private-key display/copy action is provided.
Passphrase prompts and host trust follow the runtime/authentication RFCs.
Unknown hosts show authority/fingerprint; replacement also shows the old pin.
Cancellation leaves local work usable and exposes Unlock/Retry without repeated
background modal prompts.

## Architecture, accessibility and feasibility

Background polling and its index refresh run in one application-owned worker
thread/task inside the desktop process, shared across its windows and stopped
on exit. No separate polling executable or shared singleton service is used.
The CLI has only explicit one-shot polling/indexing.

Domain and runtime services remain in the headless library. Desktop modules
depend directly only on `gpui-kit`, initialize it inside `app.run`, and create
`Root` first for each window as required by `AGENTS.md`. Git/file/network work
runs off the UI thread; models carry owned data and request IDs, never `git2`
handles or secret-bearing debug values. Stale async responses cannot replace a
newer draft or the currently selected item's state.

Every core action has a labeled focusable control. Tab order, visible focus,
focus return after dialogs, Escape cancellation and keyboard activation are
required; Escape inside the editor follows the D3 amendments above. Editor
mode switching, formatting, conflict resolution, polling and
key management must all work without a pointer. Color alone cannot signal
unsaved work or failure. Native accessibility semantics and IME behavior require
evidence on supported platforms; keyboard support is the PRD minimum, not proof
of screen-reader conformance.

### Editor candidates from the charter

The [charter's initial technical directions](../charter.md#initial-technical-decisions-and-directions)
name `zorite-editor` with a minor preference and extracting an editor from
Velotype as an alternative. This RFC carries that preference forward:
**evaluate Zorite first, then Velotype if Zorite cannot meet the contract at
reasonable integration cost**. Building a new editor is not the default.

The following is a source/documentation assessment, not a compiled integration
result. Upstream branches and published packages can differ; a feasibility
record must pin the exact artifact/revision evaluated.

| Candidate | Evidence and fit | Evaluation risk |
| --- | --- | --- |
| `zorite-editor` — preferred first evaluation | The crate documents a host-agnostic GPUI editor with live Markdown styling, raw mode when styling is absent, undo/redo, IME and display/source offset mapping. Its crate manifest declares MIT. [API documentation](https://docs.rs/zorite-editor/0.10.0/zorite_editor/), [crate manifest](https://github.com/packetThrower/zorite/blob/main/crates/zorite-editor/Cargo.toml). | Prove mode switching, table edits, source fidelity and host-controlled save behavior. Determine which block providers/adapters are required; do not infer full integration from the feature list. |
| Editor extracted from Velotype — alternative | Its README describes native rich-text/source modes, an editable block model, fallback source and canonical Markdown serialization. Its manifest declares Apache-2.0 and `gpui` 0.2. [README](https://github.com/manyougz/velotype#readme), [manifest](https://github.com/manyougz/velotype/blob/main/Cargo.toml). | Extraction must separate window/file/save/network behavior from editing. Canonical reserialization needs particular scrutiny against unchanged-source preservation; its GPUI dependency requires compatibility work. |

At this planning base, Manyhands locks `gpui-kit` 0.6.6 and `gpui-pre` 0.3.6.
Zorite's inspected workspace uses `gpui-pre` with a `0.3` version requirement
under the dependency name `gpui`; its documentation requires the host and editor
to resolve one GPUI version. That suggests a closer dependency fit, but does
not prove compatibility with 0.3.6 or with the published package.
[Workspace manifest](https://github.com/packetThrower/zorite/blob/main/Cargo.toml),
[integration documentation](https://packetthrower.github.io/zorite/reference/crates/zorite-editor/).

Manyhands code must continue to use GPUI through `gpui_kit::*`, with no separate
direct `gpui` dependency. An editor's transitive GPUI dependency must resolve to
the same package/version/source used by GPUI Kit so its entity types interoperate.
Neither copying an upstream example's imports nor adding a second GPUI graph
is an acceptable integration shortcut. Keep editor dependencies desktop-only.
Record licenses/notices for the actual reused files and dependency closure;
do not infer the editor crate's license from its parent application's license.

Before selecting a dependency or planning editor implementation, compare a
pinned candidate against the fidelity fixtures and test mode switching with
shared undo, metadata separation, tables, IME/Unicode, keyboard focus, large
documents, draft recovery and external-change replacement. Host-controlled
image/link loading and save hooks must enforce this RFC's boundaries. The
result must identify required adapters, unresolved native-platform checks and
maintenance cost, then recommend adoption, adaptation or rejection.

Failure blocks the editor selection; changing to a source-only product requires
renewed product agreement. Only after evaluating the charter's candidates
should a custom or webview editor be proposed with its additional scope and
dependency/accessibility implications. Source-plus-preview was considered and
was not selected by the product owner.

## Acceptance evidence

The approved test strategy adopts the following required evidence:

- Golden source fixtures cover unknown YAML values, CRLF, Unicode, embedded
  HTML, unsupported extensions, tables and mixed formatting. Opening, mode
  switches and no-op saves write nothing to the canonical file. Focused edits
  preserve untouched content except for the recorded load normalization of
  the pinned editor, whose exact output is itself a golden fixture.
- Real-repository tests cover draft persistence failure/crash, stale saves,
  polling during unsaved edits, two processes editing an item, source repair,
  identity recovery, move collisions and partial checkpoint/index failure.
- Each PRD journey runs through the actual desktop with real repositories and
  SSH transport, including canonical conflict resolution and retry.
- Keyboard-only runs cover all `MH-NFR-005` actions on Windows, macOS and Linux
  Wayland, with focus/IME results recorded. Screenshots alone are insufficient.
- Delayed file/Git/network fixtures prove navigation and editing remain usable,
  cancellation shows safe-point progress and stale completion cannot lose text.

These are planned checks, not completed evidence. The review register defines
the native-platform matrix, adopted source amendments and remaining
implementation gates.
