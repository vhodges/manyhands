---
manyhands_managed: true
manyhands_kind: comment
id: "01M65FG0DZVS76X6NT05MCJNX5"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M65DZRAM0XV1TX0N7XDN5B36"
created_at: "2026-10-30T04:04:24Z"
created_by: "Claude Sonnet 5.5 <noreply@anthropic.com>"
---

## Task 9 complete; the task review is pending

Part B's third task. It adds `conflict show` and `conflict resolve`. Only the
CLI, the input schema and fixture, and tests changed; there is no library
change, so no `--lib` target ran.

- **The verbs.**
  - `conflict show` is a read of `show_conflict` for `--repo` and
    `--operation-id`. It takes no request ID, and `--prepare` is a usage
    error. Its text prints the state, the next step and each path with its
    kind, structure and token; a side is printed only with `--json`.
  - `conflict resolve` takes `observation` and a non-empty list of
    `{token, source}` objects (new `object_list` reader). The library judges
    the tokens. It runs through the shared driver and the Ctrl-C window
    already wired for it; `--prepare` is `not_confirmable`.
  - Both refuse a local-context sync's operation as `invalid_input`
    (decision 19), tested with a sync made by the gate's library mutation.
  - The help goldens were regenerated (E-1).
- **Human output.**
  - `external_change` of a resolve whose conflict reads `historical` adds the
    note to restore a hand-edited file; JSON is unchanged.
  - `original_request_required` of a resolve names the request and says the
    written paths were edited. Task 7's supersede note stays `operation
    resume`'s.
  - `remote_branch_deleted` of a resolve prints Task 7's three `next:` lines
    with the holder's request, and the item from the conflict read. The
    reservation note names the holder when it is not the refused request.
  - An `incompatible_closures` path prints the design's note. A path's display
    text is printed once, never doubled.
- **Verification.** The binary tests were written first and failed. Then: the
  `manyhands-cli` bin 243 passed, `--test cli` 330 passed, and the
  `input_schemas` filter 7 passed. Clippy (all targets and features),
  `fmt --check`, `check --bin manyhands-cli` and the desktop `check` pass.
- **Review.** The task review is pending. The live rows (a real conflict, its
  text golden, a stale observation, the hand-edit escape and the deleted
  branch end to end) are Task 12b's.
