---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5CS4MJ0G7B0MPJD0KN0QMYA"
title: "Desktop follow-up: settings UX and structure from the D1 reviews"
type: "task"
status: "open"
project: "manyhands"
team: "core"
---

The D1 desktop reviews (Wave 03 Cycle D1, ticket
`01M4CC0VMY5RBMP70QVBXWXM6A`) left minor findings on the four settings pages
(Repositories, Remotes, Keys, Hosts), plus structural cleanup the reviewers
asked for before later desktop Cycles add more pages. None blocks merging D1.

This is desktop-only work under `src/desktop/`, and no library change is
expected. Do it after D1 merges and before D2/D3 add pages. Line references
are to the D1 branch as reviewed at `ba89ea0`. The owner also plans UI changes
after trying the desktop; this ticket can carry them or link to them.

## Structure (do first)

- `Read<T>` is defined twice (`view/settings/mod.rs` and
  `view/settings/repositories/manage.rs`). Keep one shared definition.
- Tickets are named by short code and title in slightly different ways in
  `TicketRow`, Find matches, relationship rows, tab titles and blocked
  reasons. `TicketRow::target` duplicates `OpenTarget::for_item`. Use one
  helper.
- `Workspace` exposes about 20 `pub(super)` fields that `lists`, `tabs`,
  `operations`, `sidebar` and `find` reach into. Narrow them to methods or
  events.
- Split the large files:
  - `view/workspace.rs` (about 1150 lines);
  - `view/settings/keys.rs` (about 1080 lines);
  - `view/item.rs` (about 920 lines).

## Repositories page

- Browse-unavailable reason:
  - It is drawn once at the bottom of the page. Move it beside each Browse
    control.
  - While the folder chooser is pending, Browse is disabled with no text.
    Add text.
- A busy folder inspection offers no Retry. Add one.
- A root that is not UTF-8 is silently ignored by `inspect_root`. Show a fixed
  message instead.
- Settings and tabs share one scroll offset. Reset the offset on a mode switch,
  or keep one offset per mode.
- Forms are not cleared after success. Decide whether they should be.
- Tests:
  - an absent-path Create preview;
  - the effects rows of a Partial result;
  - busy, failed and double-Add inspection paths;
  - Escape on the sidebar Remove dialog;
  - removing a repository and registering it again;
  - where focus lands after a successful removal.
- A slow identity read can overwrite text the user is typing. Prefill only
  empty fields.
- The "From:" line is hidden when no email is configured.

## Remotes page

- Reopening Settings ignores a sidebar selection made since the last open.
- The first open reads the list twice.
- `RemoteAdd`'s raw URL lives in two long-lived records (ruling R-T14a).
  Optionally, have the UI-side `DesktopRequest.intent` hold a redacted URL.
  Only the worker's record needs the raw one for Retry.

## Keys page

- Add a test that the operation area's Retry of a request that is not a
  protected key generation still goes through `retry(id, None)`.
- The focus fallback for Delete is keyed by row index. Key it by key ID.
- No tests cover the key-source lines for a missing or unreadable file, or a
  busy or failed Copy.
- Add typed-failure tests for key delete and for a non-busy key generate
  (named as gaps in the D1 acceptance table).

## Hosts page

- An Approve preview refused with `host_replacement_required` offers no
  Replace. Show the envelope's `host.replace` recovery.
- `NOT_AVAILABLE_YET` now means "nothing this session can act on". Reword it,
  for example "Not available in this session".
- A recovery that arrives while a check is connecting can start a second
  `observe_host`.
- "No SSH publication remote" can appear next to "Connecting".
- Note for D6: the `defer_in` prompt fix relies on a single prompt source.

## Keyboard and accessibility

- A focus restore from the Kit's `defer_close_dialog` could take focus away
  from Quit now. This is rare and untested.
- The keyboard-test helper `on_handle` does not assert visibility, and
  `open_a_document` asserts only the region.
- The close buttons sit inside `Role::TabList`.

## Acceptance

- Do the structure items before adding any page.
- Every other item is either fixed or dropped with a reason in a comment.
- Each behavioural fix comes with a test that fails without it.
- The desktop gate passes:
  - `desktop::` tests;
  - check and clippy with `--features desktop`;
  - fmt;
  - clippy with `--all-features`.
