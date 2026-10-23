---
id: 01M5A4ZYP8BF57QZCFQ5XXVJWF
item_id: 01M4CC0VMSZ7BRS740C8VSQ5ZK
created_at: 2026-10-19T13:18:10Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

Task 5 implementation checkpoint, base
`4fb690cb70f5f43566838dc26e1a7ea9ea3794a1`.

Task 4 handoff records **spec PASS and quality PASS**, including metadata-control
escaping fix `4fb690c`, with **66 bin + 33 CLI tests passing**. This resolves the
pending re-review in the prior comment. Task 5 reuses the shared field helper
for metadata and preserves requested bodies/sources exactly.

Implemented only `document list/show`, `folder list`, `ticket list/show` and
`comment list`, through existing F1 reads and unchanged DTOs. ID/path/context
selection, kind mismatch and error scopes follow the approved design. Ticket
filters forward every option and default closure to All; the helper safely
accepts the narrower future relationship argument sets. Eight relationship
placeholders remain for Task 6.

- **RED:** all 11 content integration tests failed with `command_unavailable`;
  six renderer tests, one filter-helper test and the placeholder-count test
  failed before their implementation.
- **GREEN:** **73 bin + 44 CLI + 43 mutation-contract = 160 passed**, zero failed
  or ignored. All Rust commands used Devenv and locked dependencies (fmt excepted).
- Generated and read **7 JSON + 2 text** goldens, then verified without update
  mode. All seven JSON fixtures are byte-identical to the corresponding F1
  goldens. Text pins cover ticket list and absent-ticket stderr.
- Scoped clippy with warnings denied, fmt check and diff whitespace check pass.
- Self-review **spec/quality PASS**, with no unresolved finding or library gap.
  Independent controller review is pending. No pre-C1 semantic test, library,
  repository, schema or dependency edits.

Tests cover every filter against F1, both selector paths and explicit linked
contexts, both directions of ID/path kind mismatch, invalid selectors,
repository-before-ID precedence, stale list/show reads, empty/nonconforming
outputs, comment indentation, field-control escaping and exact body/source
content. Unit tests cover every index/incomplete-note condition independently.

Owner-authorized checkpoint commit: `feat: CLI document, ticket and comment
reads`, with the required co-author trailer. Task 6 remains unstarted; ticket
stays open. Full-suite verification is reserved for Task 8. No subagents,
rebase, push, PR, merge into main, closure, cleanup or native workflow.

Details:
[execution ledger](../../../docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-5-checkpoint--2026-10-19).
