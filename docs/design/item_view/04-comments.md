# 04 · Comments (inline)

## Decision

Comments are shown **inline, below the body, in the same scroll region**. The other options explored were a popout sheet (3e) and a dockable panel (3f). The owner chose inline. Reasons:
- It's familiar from issue trackers.
- It reads as part of the item's record, which matches how comments are stored.
- There's one scroll region, so no extra layout state.
- Search, print and export see everything.

Its main drawback is that you can't see a comment and the text it discusses at the same time. Jump navigation reduces that, and an optional panel is listed under "Later".

## Discussion section

- It sits after the page, full width, background `discussion` `#f8f7f5`, top border 1 px `#dedbd5`, padding 28 px 48px 48 px. The content column is 664 px max, aligned with the page column.
- Header row: `message-square` + "Discussion" (16 px / 600) + "{n} comments · oldest first" (12 px `#767a82`) + right-aligned "↑ Back to top" (teal link).
- **Threads:** one white card per root comment (1 px `#dedbd5`, radius 8). Order follows the RFC: roots ascending by creation; each root's direct replies ascending beneath it.
- **Comment row:** padding 12 px 14 px.
  - 26 px avatar (initials, color per person).
  - Name (13 px / 600) and time (12 px `#767a82`: "Oct 22, 10:14", "Today, 15:36").
  - Body: Markdown, 14 px / 1.55, using the same renderer as the page.
  - Actions row (12 px `#575c64`): **Reply** on root comments.
  - Replies are indented to 50 px and separated by 1 px `#efede9`.
- One level of replies. A reply's Reply action replies to the root.
- Malformed threads stay visible with a reason (RFC); they're never hidden.

## Navigation

- The header button **Comments {n}** scrolls to the Discussion header, leaving 120 px of offset below the sticky block.
- A **floating jump button** ("{n} comments ↓"): pill shape, white, 1 px `#cfcbc4`, shadow `0 4px 14px rgba(0,0,0,.14)`. It sits at the bottom right of the pane, 16 px from the edge, and hides once the discussion is in view.
- "Back to top" returns to the start of the page.
- Shortcut Ctrl Shift C. The jump moves focus to the Discussion heading, for screen readers.
- When opened from Activity or a notification about a comment, the item scrolls to that comment and highlights it briefly (`#fbefd6` background, fading out over 1.5 s).

## New comment composer

The composer sits at the end of the discussion. It's a white card with a 1 px `#cfcbc4` border and radius 8.
- Placeholder: "Add a comment. Markdown works here." (14 px). Minimum height 70 px; grows with content.
- Footer: an explanation, 12 px `#767a82`: "Posting also shares other saved work in this ticket. Unsaved edits above are not included." (RFC requires this explanation.)
- Button:
  - With a publication remote: **Post and sync**.
  - Without one: **Save comment locally**.
- On focus the composer expands, shows the mini toolbar (below) and gets the focus ring.

## Reply composer (4a, 4b)

- Opens **inside the thread**, under its last reply, indented like a reply, with the user's avatar. Focus moves into it immediately.
- Focus ring: 1 px `#07596a` border + a 3 px `rgba(7,89,106,.14)` ring. Radius 8.
- **Mini toolbar** (1 px `#efede9` bottom border): "Replying to {name}" (11.5 px), separator, then Bold, Italic, Inline code, Link and Bulleted list (26×24 buttons). On the right, "Draft kept" with a green `shield-check`.
- Text area: 14 px / 1.55, minimum 64 px, Markdown (rich editing via a small zorite instance or plain text with Markdown; implementer's choice, keep source fidelity).
- Footer: "Ctrl Enter to post · Esc to cancel · Also shares other saved work in this ticket" · **Cancel** (secondary) · **Reply and sync** (amber; "Save reply locally" without a remote).
- Only one reply composer is open at a time. Clicking Reply in another thread moves it there, and its draft is kept per thread.
- **Esc / Cancel** with text present asks "Discard this reply?" (Discard · Keep editing). An empty composer closes silently.

## Drafts

Comment and reply drafts use the same draft store and rules as body drafts: flushed within 1 s, restored after a crash, scoped per item and thread. They are **separate from the body draft**, so posting never includes unsaved body edits.

## Posting outcomes (RFC MH-COLLAB-003/004)

Posting runs: provision or reuse the item's context → write the comment → checkpoint → sync (when a remote exists). The composer allocates one comment ID per submission and keeps it through retries.

| Outcome | UI |
| --- | --- |
| In progress | The button shows a spinner and "Posting…", and the composer is read-only. The comment appears in the thread at 60% opacity |
| Posted and shared | The comment appears normally and the composer clears. Activity: "Shared a comment on …" |
| Saved, sharing failed | The comment shows **once**, with an amber chip: `cloud-off` + "Saved on this computer · not shared yet", plus a **Retry sharing** action. Retry never creates a second comment. A Needs attention entry is added |
| No publication remote | The comment shows with "Saved on this computer" (no failure styling); the footer count "N to share" increases |
| Checkpoint failed | The composer keeps the text. A red inline message names the step, with Retry. Nothing appears in the thread |

## Later (not in this build)

An optional **Comments panel** that docks on the right or bottom and shows the same threads and composer for the active item (frame 3f), off by default and turned on from the footer. It's for side-by-side discussion while editing long documents.
