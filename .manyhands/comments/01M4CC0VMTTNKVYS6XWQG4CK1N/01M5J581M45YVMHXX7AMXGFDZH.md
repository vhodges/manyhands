---
manyhands_managed: true
manyhands_kind: comment
id: "01M5J581M45YVMHXX7AMXGFDZH"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T15:56:21Z"
---

## Task 6 — Repository Verbs

Controller reports Task 5 spec PASS (`ses_e9bd75504ffemMwjV080mC11cY`) and
quality PASS (`ses_e9bbe16d3ffeiRbUv1m8h4c0Oz`), pushed through `c9b99aa`.
Task 6 began at `c9b99aabe370663c702b8256858256da92620083` on the assigned
ticket branch/worktree; fetching and merging main was a no-op.

Implemented confirmed `repo create`, `repo enable` and `repo remove` through
the existing mutation driver. Added closed create/enable input schemas and
examples, identity/prefix shape checks, catalog/dispatch wiring, repository
integration cases, human/no-op and confirmed-prepare coverage, four identity
PTY cases, and nine JSON plus one human preview golden. E3 help now advertises
the three delivered verbs and their options.

The tests prove index-current onboarding followed by CLI remote add and the
temporary library RemoteSelect call, registration through a second data
directory, prefix preview/write, local-only supplied identity, inherited global
identity without local writes, unchanged files on accessible removal, and exact
deleted-root removal. A trailing separator on a deleted root returns
`invalid_path` and preserves registration; the exact stored root then works.

Every missing-identity case starts with the prescribed read-only system Git
identity probe. PTY cases expect each prompt before answering. Create and enable
both prove later retries require the prompted identity in input: omission gives
`request_mismatch` without prompting, supplying it replays without prompting.
Global configuration remains byte-identical. Cancellation creates no target;
three empty names return `invalid_input`.

Verification through Devenv, Cargo locked except fmt:

- RED: **11 repository tests failed** with `command_unavailable` before wiring.
- GREEN: **14 repository tests passed** after wiring and new boundary cases.
- Final CLI target: **114 passed**; final bin target: **167 passed**.
- **281 distinct final passes**, including **21 new tests**.
- Formatting, diff check and merge-tree check passed.

The ledger records intermediate new-test corrections (schema/TOML names,
registration preview effects, timestamp normalization, and answering Email in
the empty-name script), exact commands/counts and self-review. All generated
goldens and the help diffs were read; schema and fixture inventory tests pass.
No library gap or unresolved self-review issue was found. No library, output
schema, result-code, dependency or lockfile changes.

Local delivery: `feat: add repo create, enable and remove`, with the required
coauthor. Ready for controller spec/quality review and subsequent push. Task 7
is not part of this delivery. Ticket remains open; no push, PR, closure,
cleanup, native CI or subagents. This ID came from the built CLI in a fresh,
detached, empty-environment HOME/XDG/application-data sandbox.
