# Handoff: Ticket and document view, editor and comments

## Overview

These documents specify the center-pane **item view**: how a ticket or a managed document is read and edited, how front matter is edited, and how comments work. They extend the workspace handoff in `../design_handoff_desktop_workspace/` and don't repeat it. The panels, docking, footer, plain-language glossary, base color tokens, dialogs (Close ticket, Approve and merge, conflicts, unlock) and the GPUI Kit component mapping all live there and still apply.

## About the design files

`mockups/manyhands-workspace-mockups.html` is a **design reference built in HTML**, not code to port. Rounds 3 and 4 (frames `3a`–`4f`) are the ones this package covers. Rebuild them in `src/desktop/` with GPUI through `gpui_kit::*`, and use **zorite-editor** for body editing, as the Desktop IA RFC requires. `mockups/source/ItemView.dc.html` is the readable source of the prototype. Its inline styles are the authority for exact values. Open the HTML file in a browser; it works offline and takes a few seconds to render.

## Fidelity

High fidelity for structure, sizes, copy and behavior. Implement colors through the custom GPUI Kit theme described in the workspace handoff (`07-design-tokens.md`), plus the additions in `05-tokens-additions.md` here.

## Decisions made with the product owner

1. The item header is a **sticky block with its own background**, above the content.
2. The ticket or document body **fills the remaining space on a white page**, in both the dark and the light theme.
3. **Edit mode** adds a **sticky formatting toolbar** directly under the header block.
4. **Front matter is shown and edited in a GPUI Kit `Sheet`**, opened from the header's metadata chips.
5. **Comments are inline**, below the body in the same scroll region, with navigation to jump to them. The popout sheet and the dockable panel were explored (frames `3e` and `3f`) and are not adopted. A dockable Comments panel can be added later (`04-comments.md`, "Later").

## Reading order

| File | Covers |
| --- | --- |
| `01-item-frame.md` | Tab, sticky header block, page, view and edit states, getting a private copy ready |
| `02-editor-and-toolbar.md` | Rich text / Markdown modes, toolbar, rendering of each block type, drafts |
| `03-front-matter-sheet.md` | Ticket details and document details sheets: fields, validation, read-only parts |
| `04-comments.md` | Inline discussion, threads, composer, reply, posting outcomes, navigation |
| `05-tokens-additions.md` | Page and header colors and type that are new in this round |
| `06-implementation-plan.md` | Suggested Cycles, zorite-editor integration points, acceptance checks |

## Frames

| Id | Shows |
| --- | --- |
| 3a | Ticket, view mode, inline comments, floating jump button |
| 3b | Ticket, edit mode with the sticky toolbar (try the Rich text / Markdown switch) |
| 3c | Ticket details sheet (front matter) |
| 3d | Inline discussion scrolled into view, with the new-comment composer |
| 3e, 3f | Rejected options: comments sheet, comments panel (for reference only) |
| 4a, 4b | Reply in progress inside a thread (dark and light chrome) |
| 4c | Document, view mode |
| 4d | Document, editing, details sheet open |
| 4e, 4f | Document editing and its details sheet, light chrome |

`screenshots/` has a 1440×900 PNG of each frame you'll build from, named by id. Hover states and clicking through still need the HTML.

## Files

```
design_handoff_item_view/
  README.md, 01 … 06 *.md
  mockups/manyhands-workspace-mockups.html   ← all rounds; 3a–4f are this package
  mockups/source/ItemView.dc.html            ← exact values
  screenshots/*.png
```
