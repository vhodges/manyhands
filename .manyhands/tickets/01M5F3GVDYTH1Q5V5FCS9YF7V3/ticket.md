---
manyhands_managed: true
manyhands_kind: ticket
id: "01M5F3GVDYTH1Q5V5FCS9YF7V3"
title: "Desktop follow-up: settings UX from the D1 reviews"
type: "task"
status: "open"
project: "manyhands"
team: "core"
---

## Origin and delivery order

On 2026-10-21 the owner approved moving all non-Structure findings from
`01M5CS4MJ0G7B0MPJD0KN0QMYA` into this ticket. They originate in the D1
desktop reviews (Wave 03 Cycle D1, ticket `01M4CC0VMY5RBMP70QVBXWXM6A`),
which judged that none blocked D1's merge.

Implement this ticket after the Structure ticket is reviewed and merged to
main. The Structure ticket can close independently to unblock D2's desktop
work; these UX findings are not part of that prerequisite. Work remains
desktop-only under `src/desktop/`, with no expected shared-library change.
Remap source locations after merging the Structure work. Original references
describe D1 as reviewed at `ba89ea0`.

The sibling `01M5CS4MH25QQ769HV61ZRKHXG`, "Desktop follow-up: tests and polish
from the D1 reviews", retains its separate non-settings findings. Coordinate
overlapping work rather than duplicating it.

## Product decisions to ask before implementation

- Should the Repositories forms clear after a successful operation?
- Should this ticket carry the UI changes the owner plans after trying the
  desktop, or link to separate tickets for them?

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

- The Structure ticket is merged before this work begins.
- Every finding above is either fixed or dropped with a reason in a comment.
- Product decisions are recorded before the affected behavioral changes.
- Each behavioral fix comes with a test that fails without it.
- The desktop gate passes: `desktop::` tests, check and clippy with
  `--features desktop`, fmt, and clippy with `--all-features`; follow the
  current `AGENTS.md` for all required final gates through Devenv.
- Close only after review approval.
