# 05 · Token additions

These are added to the theme defined in `../design_handoff_desktop_workspace/07-design-tokens.md`. The **page** tokens are the same in the dark and light themes, because the page is white in both.

## Header block (themed)

| Token | Dark | Light |
| --- | --- | --- |
| `header-block` | `#262a30` | `#ebe8e3` |
| `header-border` | `#353a41` | `#e2dfda` |
| `header-title` | `#eef0f2` | `#15181c` |

Everything else in the header (chips, buttons, pill) uses the existing chrome tokens.

## Page (not themed)

| Token | Value | Used for |
| --- | --- | --- |
| `page` | `#ffffff` | Page background, comment cards, composers |
| `page-text` | `#1f2328` | Body text, caret |
| `page-text-secondary` | `#575c64` | Quotes, comment actions |
| `page-muted` | `#767a82` | Times, hints, done checklist items |
| `page-border` | `#dedbd5` | Tables, cards, separators |
| `page-border-soft` | `#efede9` | Separators inside cards |
| `page-border-strong` | `#cfcbc4` | Composer, jump button, scrollbar thumb |
| `page-fill` | `#f6f5f2` | Toolbar, table header, code block, label bars |
| `code-inline` | `#f3f2ef` | Inline code background |
| `discussion` | `#f8f7f5` | Discussion section background |
| `toolbar-hover` | `#e2dfda` | Toolbar hover and active mark |
| `segmented` | `#ebe9e5` | Mode switch track |
| `selection` | `#fbefd6` | Text selection, comment highlight |
| `page-teal` | `#07596a` | Links, headings in Markdown mode, checkbox fill, composer focus |
| `page-teal-hover` | `#0a7a90` | Link hover |
| `page-amber` | `#a86f00` | Primary button on the page (Post, Reply) |
| `page-amber-text` | `#8f5e00` on `#fbefd6` | "not shared yet" chip |
| `page-green` | `#2c7a43` | "Draft kept" icon |

Avatar colors used in the mockup: VH `#07596A`, JS `#8a5a00`, AM `#6b4fa0`. Derive them deterministically from the Git identity and keep white text at 4.5:1 or better.

## Type on the page

| Role | Size / weight / line-height |
| --- | --- |
| Body | 15 / 400 / 1.65 |
| Lead paragraph (documents) | 16 / 400, `#3b4047` |
| Heading 2 | 19 / 600 / 1.3 |
| Table | 14 |
| Comment body | 14 / 1.55 |
| Comment name / time | 13 / 600 · 12 / 400 |
| Markdown mode | Mono 13.5 / 1.75 |
| Inline code / code block | Mono 13 |

Layout: page column 760 px max, padding 36 px 48px 40 px. Discussion column 664 px. Header block padding 14 px 28 px. Toolbar padding 5 px 20 px.
