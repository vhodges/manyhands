---
manyhands_managed: true
manyhands_kind: comment
id: "01M5J01BZ1RHCC2J96PGGNP0X8"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T14:26:00Z"
---

## Task 4 Completion Checkpoint

Implemented Task 4 only from `28055bae53c26a4b814a6d263f8cabc28e6e0303`,
inline in the ticket branch/worktree without subagents. The required main
fetch/merge was already up to date and changed no history.

Recorded Task 3's independent spec APPROVE (`ses_e9c083b62ffeI5tLJunx3nZAV1`),
quality APPROVE (`ses_e9c073b81ffeUNboZn1PFsSisp`) and push at `28055ba` in the
execution ledger.

Added read-once JSON input, opaque recursive duplicate-refusing objects, all
specified field accessors and typed errors. A parse-local serde seed captures
duplicate keys without parsing or exposing serde error text. Escaped equivalent
keys and nested array/object duplicates are refused, recursion stays bounded,
one leading BOM is skipped, and trailing JSON is refused. Diagnostics contain
fixed text and escaped field names, never input values or backend errors.

Added the closed commit-identity input schema and three schema contract tests:
real JSON-help inventory and reference reachability, supported keywords/closed
objects, and example validation. commit_identity is the sole pending reference
until Task 6. No available input verbs or examples exist yet, so the example
directory intentionally contains only .gitkeep.

Targeted TDD, all Cargo commands through Devenv:

- `cargo test --locked --bin manyhands-cli input`: RED **25 new failures** at
  stubs plus two existing passes; GREEN **27 passed**, zero failures/ignored,
  116 filtered, no warnings.
- `cargo test --locked --test cli input_schemas`: RED **two failures** for the
  not-yet-published schema directory plus one empty-example pass; GREEN
  **3 passed**, zero failures/ignored, 64 filtered, no warnings.
- `cargo fmt` and `cargo fmt --check` passed.

Self-review covered all errors and accessor semantics, read-to-EOF behavior,
temporary file cases, UTF-16 details, duplicate capture, recursion and sentinel
privacy. No library gap or unresolved issue found. No existing tests changed
except module wiring; no dependency, lockfile, library or output-schema change.
`git diff --check` and the pre-commit merge-tree check passed.

The [execution ledger](../../../docs/plans/2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md)
records the base, decisions, files and verification counts. This comment's ID
came from the built CLI with isolated temporary HOME/XDG/app data, disabled Git
configuration and no controlling terminal.

Delivery: `feat: read and check JSON mutation input`.
**Task 4 is ready for controller review.** Task 5 has not started. The controller
owns review and subsequent push; the ticket stays open.
