# 06 · Suggested implementation plan

Same conventions as the workspace plan: one Manyhands ticket per Cycle, worked in its worktree, Devenv gates, checkpoints recorded as ticket comments. This plan assumes the workspace Cycles 1 (theme) and 5 (item area tabs and header) have landed, or are done at the same time.

## Cycle A: Item frame

- The scroll region with a sticky header block, the white page, the 760 px column and minimum-height fill.
- Header contents: breadcrumb, title, chips (as buttons, opening a placeholder sheet), state pill, actions.
- View → edit transition with the "Getting a private copy ready…" state (wire it to existing context provisioning); Done back to view.
- **Accept:** viewing provisions nothing; Edit provisions once and reuses after; the keyboard reaches every header control; the light and dark header tokens apply.

## Cycle B: zorite-editor on the page

- Host zorite in the page in read-only (view) and editable modes. Map the block rendering table in `02` onto zorite's styling hooks.
- Labeled source blocks for unsupported syntax, with Edit source.
- Rich text / Markdown switch sharing one draft and undo history; front matter stays out of the editor.
- Load normalization treated as the clean baseline.
- **Accept:** the RFC fidelity fixtures pass; opening, switching modes and a no-op save write nothing; IME and Unicode checks are recorded.

## Cycle C: Toolbar

- A sticky toolbar in edit mode with every action in `02`, all with keyboard shortcuts and focus order; dimmed but operable in Markdown mode.
- The "Draft kept" indicator wired to the draft store's flush result, including the failure state.
- **Accept:** every action works by keyboard on all platforms; the indicator never claims a flush that didn't happen.

## Cycle D: Front matter sheet

- Ticket and document details sheets per `03`, editing the shared draft. Read-only in view mode.
- Waiting on and Parent pickers using the ticket-relationships RFC (rejecting cycles inline).
- Unknown keys kept in file order (note the fixed defect 01M5MHQ16VD0P2RTGY8PMRYWGY about key reordering).
- **Accept:** a no-op open and close writes nothing; status text "closed" doesn't close; required-field errors focus the field; inspect raw YAML works.

## Cycle E: Inline discussion

- The Discussion section, threads in RFC order, one reply level, the Markdown renderer shared with the page.
- Navigation: the header button, the floating jump button (hidden when the discussion is in view), Back to top, Ctrl Shift C, scroll-to-comment highlight.
- The new-comment composer and the reply composer per `04`, with per-thread drafts.
- Posting outcomes table, including the single-ID retry and "not shared yet".
- **Accept:** the PRD's "Durable threaded discussion" journey passes through the UI; a crash during typing restores the reply; posting never includes unsaved body edits; the label is "Post and sync" or "Save comment locally" depending on the remote.

## Open questions for the owner

- Should typing into the page in view mode start editing (with a prompt), or only the Edit button and E?
- Rich editing inside comment composers (a small zorite instance) vs plain Markdown text?
- Sort toggle for the discussion (oldest or newest first)? The RFC requires ascending storage order; display order could still differ.
- Avatar color derivation.
