# Handoff: Manyhands desktop workspace (Zed-style docking shell)

## Overview

These documents specify the Manyhands desktop workspace: the window chrome, the dockable panels (Repositories, Projects, Teams, Tickets, Documents, Activity), the central item area (open ticket and document tabs, with splits), a global footer that toggles panel visibility, and the sheets and dialogs for adding repositories, settings, closing a ticket, resolving conflicts, unlocking the SSH key and reviewing things that need attention.

The direction was chosen from HTML mockups (rounds 1 and 2) in a design session with the product owner. The intended audience is mostly non-technical, with developers as a second audience. So the UI uses plain language everywhere and keeps Git vocabulary in tooltips and detail views.

The ticket viewer/editor and document viewer/editor are **out of scope**. They need their own design round. This handoff covers only their header (breadcrumb, title, metadata chips, state pill, lifecycle buttons) and the banner slot above the editor body.

## About the design files

`mockups/manyhands-workspace-mockups.html` is a **design reference built in HTML**. It's a clickable prototype that shows the intended look and behavior. It is not code to port. Recreate the design in the existing Rust desktop app (`src/desktop/`) using **GPUI through `gpui_kit::*`** and GPUI Kit components, following `AGENTS.md`. Open the HTML file in a browser. It needs no network access and takes a few seconds to render all 14 frames.

`mockups/source/Workspace.dc.html` is the readable source of the prototype. Its inline styles and data tables are the authority for exact values when this README and the specs disagree. Read it as reference only; it does not run outside the design tool.

## Fidelity

**High fidelity for layout, hierarchy, copy and states. Medium fidelity for color.** Exact sizes, spacing and copy are specified and should be matched. Colors are given as a complete palette (`07-design-tokens.md`). Implement them as a **custom GPUI Kit theme**, a dark and a light variant, rather than hard-coding hex values in views, so that later theme work stays in one place.

## Reading order

| File | What it covers |
| --- | --- |
| `01-information-architecture.md` | Panels, the single-scope model, what drives what, item identity |
| `02-layout-and-docking.md` | Dock regions, the three layouts that were explored and the chosen default, footer toggles, persistence |
| `03-panels.md` | Each dockable panel: toolbar, rows, columns, sorting, empty, loading and error states |
| `04-item-area.md` | Tabs, splits, item header, state pill, lifecycle buttons, banners |
| `05-sheets-and-dialogs.md` | Add repository, Settings (keys, polling, appearance), Close ticket, Approve and merge, Resolve conflict, Unlock key, Needs attention popover |
| `06-language-and-states.md` | Plain-language glossary mapped to domain and Git states. **Use this copy.** |
| `07-design-tokens.md` | Colors (dark and light), type, spacing, radii, shadows, icons |
| `08-gpui-kit-mapping.md` | GPUI Kit component per element; how it maps onto the current `src/desktop/view` modules |
| `09-implementation-plan.md` | Suggested Cycles, order, acceptance checks, open decisions |

## Constraints and conflicts to resolve first

1. **The Desktop IA RFC must be amended.** `docs/RFC/desktop-information-architecture-and-editor.md` currently says "No boards, project rollups or configurable docking system are required" and describes a fixed layout: repository selector, navigation pane, tab strip, content, and an operations area. This design adds a configurable dock. Get product-owner approval and amend the RFC before implementation (see `09-implementation-plan.md`, Cycle 0).
2. **Projects and Teams are filters, not rollups.** The PRD lists "team rollups, or project rollups" as non-goals. The Projects and Teams panels only **select a scope**: the union of items whose `project` or `team` metadata matches, across all enabled repositories. They show counts, not progress or status rollups. Keep it that way.
3. **GPUI Kit version.** The repo locks `gpui-kit` 0.6.6. The Dock docs consulted for this design are for v0.8.0 (`DockArea`, `DockSkin`, `DockLayout`, `DockPlacement`, `DockAreaState`). Check which Dock API is available at the locked version, or plan the upgrade as its own Cycle.
4. **Every existing requirement still holds.** Failures are never transient-only (Activity and Needs attention keep them). Keyboard support is required for every action. Color alone never signals unsaved or failed state. Tabs are keyed by repository + kind + ULID. Viewing a primary item never provisions a worktree.

## Assets

- `assets/manyhands-icon.svg`: app icon (amber `#E9AD36` + teal `#07596A`), from `brand/manyhands-icon.svg` in the repo.
- `assets/manyhands-logo.svg`: wordmark, from `brand/manyhands-logo.svg`.
- Icons: **Lucide** names are used throughout the specs (`folder-git-2`, `ticket`, …). GPUI Kit's `Icon` component ships Lucide; use the same names where they exist, or the nearest equivalent.
- Fonts: the mockups use IBM Plex Sans and IBM Plex Mono. The font choice wasn't decided with the owner. Using GPUI Kit's default UI font plus one monospace font is acceptable; keep the size scale.

## Files

```
design_handoff_desktop_workspace/
  README.md
  01-information-architecture.md … 09-implementation-plan.md
  mockups/manyhands-workspace-mockups.html   ← open this
  screenshots/{frame-id}-{name}.png         ← 1440×900 still of each frame
  mockups/source/Workspace.dc.html           ← exact values
  mockups/source/Manyhands Workspace Mockups.dc.html
  assets/manyhands-icon.svg, assets/manyhands-logo.svg
```

`screenshots/` holds a 1440×900 PNG of every frame, named by id (for example `2c-close-ticket-confirm.png`). They're quicker to scan than the HTML, but they can't show hover tooltips or interactions. Frame ids used in these docs (`1a` … `2h`) match the badges in the mockup file.
