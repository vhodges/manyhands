# 01 · Item frame

The center pane is one vertical **scroll region**. From top to bottom it holds:

1. **Sticky header block**. It stays at the top of the scroll region (`position: sticky; top: 0` equivalent).
2. **Sticky formatting toolbar**, in edit mode only, directly below the header block and stuck with it.
3. **Page**: the white body. It fills the rest of the pane and has a minimum height so that the white always reaches the bottom.
4. **Discussion**: inline comments, in the same scroll region (`04-comments.md`).

The tab strip above the scroll region is the one from the workspace handoff (`04-item-area.md`) and is unchanged.

## Header block

- Background `header-block` (`#262a30` dark, `#ebe8e3` light). Bottom border 1 px `header-border` (`#353a41` / `#e2dfda`). Padding 14 px 28 px. Vertical gap 10 px.
- **Breadcrumb** (12 px, `secondary`, no wrap): `folder-git-2` repository › section › file.
  - Tickets: `manyhands › Tickets › MH-VH-7Q3`.
  - Documents: `manyhands › docs/Waves › wave-03-dogfooding.md`.
- **Title**: 20 px / 600, line-height 1.25, color `header-title` (`#eef0f2` / `#15181c`). Wraps; never truncated.
- **Metadata chips** (wrap, gap 6). Each chip is a **button that opens the details sheet** (`03`). 11.5 px, padding 2 px 8 px, radius 4, 1 px `border-strong`, `key` in `secondary` then the value. Hover: border `teal`.
  - Ticket: `Type cycle` · `Status in progress` (tinted like the status pill) · `Project manyhands` · `Team core` · `Parent MH-VH-W03`.
  - Document: `Status draft` (amber tint) · `Author Vince Hodges` · `Date 2026-10-05` · `Updated 25 min ago`.
  - Last chip: `All details` (dashed border, `sliders-horizontal` icon).
- **Actions** (right; they wrap below the title on narrow panes; no wrapping inside a button):
  - **State pill.** Tooltip gives the branch and worktree.
    - View: `circle-check` green, "Viewing the shared copy".
    - Getting a copy ready: spinner teal, "Getting a private copy ready…".
    - Edit: `pencil` amber, "Editing · unsaved changes", which becomes "Editing · saved on this computer" after a save.
  - `Comments {n}`: secondary button with a count badge. It **jumps to the discussion** (`04`).
  - View mode: `Edit` (`pencil`). Edit mode: `Save` (`save`, Ctrl S) and `Done` (`eye`, which leaves edit mode; unsaved text stays as a draft).
  - `Share` (`cloud-upload`).
  - Primary (amber): `Close ticket…` / `Approve and merge…`. Opens the confirmation dialog from the workspace handoff.

## Page

- Background **white** (`page` `#ffffff`) in both themes.
- Content column: **max width 760 px**, centered, padding 36 px 48px 40 px. Body text 15 px / 1.65, `page-text` `#1f2328`.
- The page's minimum height makes sure the white fills the pane even for a short ticket.
- Keep the scrollbar light against the page (thumb `#cfcbc4`).

## View → edit

RFC rules: viewing never creates a worktree; starting an edit does.

1. The user clicks **Edit** (or presses E, or starts typing into the page; confirm with the owner).
2. If there's no editing context yet: the state pill shows a spinner and "Getting a private copy ready…". The Edit button is disabled. The page stays readable.
3. On success, the pill shows "Editing", the toolbar appears, and the caret is placed where the user clicked (or at the start).
4. On failure, an Activity row is added plus a banner in the header slot (workspace handoff `04-item-area.md`, banner rules) with Retry. The page stays in view mode.

When an editing context already exists, steps 2 and 3 are immediate.

**Done** returns to view mode **without saving**. The draft is kept, and the tab keeps its unsaved dot. Navigating away never saves, shares or merges.

## Keyboard

- Ctrl S save · E edit (when the page has focus) · Esc in edit mode moves focus to the header actions; it doesn't discard anything.
- Ctrl Shift C jump to the discussion · Ctrl Shift M switch Rich text / Markdown.
- Tab order: tab strip → header actions → chips → toolbar (edit mode) → page → discussion → composer.
