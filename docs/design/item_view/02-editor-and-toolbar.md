# 02 · Editor and toolbar

Body editing uses **zorite-editor** (pinned 0.10.0 per the RFC amendment). The page in `01` is the editor's surface: there is no separate editor chrome, and view mode is the same renderer, read-only.

## Toolbar (edit mode only)

Sticky directly under the header block. Background `toolbar` `#f6f5f2`, bottom border 1 px `#dedbd5`, padding 5 px 20 px, items in a wrapping row. Icon buttons are 28×28, radius 4, icon 15 px, color `#3b4047`. Hover background `#e2dfda`; active-mark background `#e2dfda`. Separators are 1×18 px `#dedbd5`.

Left to right:

1. **Mode switch** (segmented, background `#ebe9e5`, radius 5): `Rich text` | `Markdown`. The active segment is white with a 1 px shadow.
2. Undo (`undo-2`) · Redo (`redo-2`).
3. Block style dropdown: `pilcrow` + "Paragraph". Options: Paragraph, Heading 1–3.
4. Bold (`bold`, Ctrl B) · Italic (`italic`, Ctrl I) · Strikethrough · Inline code (`code`, Ctrl E) · Link (`link`, Ctrl K).
5. Bulleted list (`list`) · Numbered list (`list-ordered`) · Checklist (`list-todo`) · Quote (`text-quote`) · Code block (`square-code`) · Table (`table`, tooltip "Table (edited in Markdown for now)").
6. Right side: `shield-check` green + "Draft kept · 2s ago" (11.5 px, `#767a82`). Tooltip: "Unsaved text is kept as a recoverable draft on this computer". If draft persistence fails, it shows a red `circle-alert` with "Draft not kept. Save to protect your changes" (RFC: never claim unflushed text is crash-safe).

In **Markdown** mode, formatting buttons dim to 35% and stay operable. They insert syntax at the selection. Undo and Redo stay at full strength. The two modes share one draft, one undo history and one selection mapping where possible (RFC).

Every toolbar action is focusable and keyboard-operable; arrow keys move along the toolbar.

## Rendering on the page (view and rich-text edit)

| Block | Spec |
| --- | --- |
| Paragraph | 15 px / 1.65, margin between blocks 14 px |
| Heading 2 | 19 px / 600 / 1.3, 10 px extra space above |
| Inline code | IBM Plex Mono 13 px, background `#f3f2ef`, padding 1 px 5 px, radius 4 |
| Checklist | 15 px box, radius 3. Checked: filled `#07596a` with a white `check`; label struck through in `#767a82`. Unchecked: 1.5 px `#b5b0a8` border |
| Table | 1 px `#dedbd5` borders, radius 6, header row `#f6f5f2` / 600, cells padding 7 px 12 px, 14 px |
| Quote | 3 px left border `#dedbd5`, padding-left 14 px, text `#575c64` |
| Code block | background `#f6f5f2`, 1 px `#e6e3de`, radius 6, padding 12 px 14 px, mono 13 px / 1.6, no wrap, scrolls horizontally |
| Link | `#07596a`, hover `#0a7a90`. External links need deliberate opening (RFC) |
| **Unsupported syntax** (HTML, unknown extensions) | A **labeled source block**: dashed 1 px `#cfcbc4`, radius 6. Its label bar (`#f6f5f2`, 12 px `#575c64`) shows a `file-code` icon, "HTML block · shown as source, not rendered", and an **Edit source** action that switches to Markdown at that block. The body is mono 12.5 px. It's never dropped and never executed |

In edit mode, the selection highlight is `#fbefd6` and the caret is 2 px `#1f2328`.

## Markdown mode

The same page and column, set in mono 13.5 px / 1.75, color `#2b3038`. Headings are tinted `#07596a`. It shows exactly the body source; front matter is **not** shown here (it lives in the sheet).

## Saves and drafts (RFC behavior, surfaced here)

- **Save** (Ctrl S) validates, makes a checkpoint and refreshes the index. The pill then reads "Editing · saved on this computer" and the tab's unsaved dot clears. A save with no changes reports "No changes to save" and creates no commit.
- The draft is flushed after at most 1 s of inactivity and before a tab or app closes. Closing a tab with unsaved work offers **Save · Keep draft · Discard · Cancel**.
- **Load normalization:** opening a body in zorite may normalize it (math fences). It's not shown as a change, the tab isn't marked dirty, and nothing is written (RFC amendment 2026-10-07).
- An external change while the page is dirty puts the "Someone changed this file outside Manyhands" banner in the header and offers a three-way compare. The editor's text is never replaced automatically.
