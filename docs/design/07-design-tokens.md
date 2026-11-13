# 07 · Design tokens

Implement these as a custom GPUI Kit theme (dark and light), mapped onto the theme's semantic slots (`background`, `border`, `muted_foreground`, `accent`, …). Token names below are this handoff's own; map them to the nearest GPUI Kit theme key.

## Color: dark (default) and light

| Token | Dark | Light | Used for |
| --- | --- | --- | --- |
| `editor` | `#1a1c20` | `#ffffff` | Item area, inputs, result boxes |
| `chrome` | `#1f2226` | `#f3f2ef` | Title bar, panel headers, tab strip, footer |
| `panel` | `#212429` | `#f8f7f5` | Dock panel bodies, active panel tab, chips |
| `surface` | `#23272d` | `#ffffff` | Sheets, dialogs, popovers |
| `hover` | `#2a2e34` | `#ebe9e5` | Hover, secondary buttons, toggles on |
| `hover-strong` | `#353a41` | `#e2dfda` | Hover on hover (tab close) |
| `border` | `#2e3238` | `#dedbd5` | All 1 px dividers |
| `border-strong` | `#3a3f46` | `#cfcbc4` | Button and input borders, sheet edge |
| `faint` | `#4a5059` | `#b5b0a8` | Hidden-dock icons, unfocused tab line, radio off |
| `muted` | `#646b75` | `#767a82` | Counts, ages, placeholder text |
| `secondary` | `#8d949e` | `#575c64` | Secondary text and icons |
| `text-soft` | `#a9afb8` | `#4a4f56` | Window control icons |
| `text` | `#d5d8dd` | `#1f2328` | Primary text |
| `selection` | `#29363a` | `#fbefd6` | Selected list row |
| `accent` (brand amber) | `#E9AD36` | `#a86f00` | Selection edge, focused tab line, primary button, editing, "to share" |
| `accent-hover` | `#f2bd4f` | `#8f5e00` | Primary button hover |
| `on-accent` | `#1a1c20` | `#ffffff` | Text on primary buttons |
| `teal` (brand teal, lightened for dark) | `#5fb4c3` | `#07596a` | Links, focus ring, in-progress, info icons, checking |
| `teal-hover` | `#8fd0db` | `#0a7a90` | Link hover |
| `teal-strong` | `#3f8f9c` | `#07596a` | Switch on, chosen conflict option |
| `green` | `#7cc28f` | `#2c7a43` | Success, ready, up to date |
| `red` | `#e3787a` | `#b3393c` | Failure, blocked, conflict, attention |
| `close-hover` | `#c95b5d` | `#c95b5d` | Window close button hover |
| `avatar` | `#07596A` on `#e9f3f5` text | same | Identity avatar |

Tints: status pills and banners use the hue at **13%** alpha (pill backgrounds), **8%** (banner and outcome backgrounds), **35%** (banner borders), **14%** (diff line backgrounds) and **18%** (focus ring). Example: `rgba(95,180,195,.13)`.

Striped placeholder (editor body): `repeating-linear-gradient(135deg, #1c1f23 0 10px, #202328 10px 20px)`; light `#f7f6f3` / `#efede9`. Placeholder only, not for production.

Brand reference colors: amber `#E9AD36`, teal `#07596A`, ink `#2B3038` (wordmark).

## Typography

| Role | Font | Size / weight | Notes |
| --- | --- | --- | --- |
| Base UI | IBM Plex Sans (or toolkit UI font) | 13 / 400 | rows, body |
| Small UI | Sans | 12 / 400–500 | panel tabs, buttons, breadcrumbs |
| Meta | Sans | 11–11.5 / 400 | counts, tags, pills, footnotes |
| Column header | Sans | 11 / 400, uppercase, +0.04em | |
| Sheet title | Sans | 15 / 600 | |
| Item title | Sans | 22 / 600, lh 1.25 | |
| Codes, paths, fingerprints, times | IBM Plex Mono | 11 / 400 (11.5 in diff and paths) | never wraps |

## Spacing, sizing, radii

- Spacing values in use: 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 32 px.
- Heights: title bar 38 · panel header 32 · tab strip 34 · column header 24 · row 28/24 · footer 28 · inputs 32 · passphrase 34.
- Icon sizes: 11–12 (inline markers), 13 (rows, tabs), 14 (toolbar, panel icons), 15–16 (dialog and banner icons), 18 (step state).
- Radii: 3 (tags), 4 (icon buttons, chips), 5 (segmented, state pill, small buttons), 6 (buttons, inputs, banners), 8 (cards, key lists), 9 (status pills), 10 (dialogs, popovers, window).
- Borders: always 1 px. Selection and focus lines are 2 px inset.

## Shadows and overlays

- Sheet: `-24px 0 48px rgba(0,0,0,.45)`. Dialog: `0 24px 64px rgba(0,0,0,.5)`. Popover: `0 16px 48px rgba(0,0,0,.45)`.
- Scrims: sheets `rgba(10,11,13,.55)`, dialogs `rgba(10,11,13,.6)`.

## Motion

- Spinner: 16 px ring, 2 px stroke, `border-strong` track with a `teal` head, 0.8 s linear rotation.
- Sheets and dialogs: use the GPUI Kit defaults. Nothing else animates.
