---
id: 01M5RSBAYZPZYR4G10ZAEF13YE
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-25T05:43:16Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Part B approved; Task 8 documentation checkpoint

The fresh Task 8 brief explicitly conveys **Part B spec and quality APPROVED**
at `9a2a5170c4704ef03420ef2276911c6b1bfa3959`, over the Part B range from
`452f031fd06d3a39e2c5de6cfe711b74256ab8a1`. Retained evidence: six passed
checkpoint gates and **188 CLI unit + 290 CLI integration + 71 read-contract +
44 mutation-contract = 593 passing tests**. These are Task 7 results,
not tests rerun for Task 8. All prior decisions and owner rulings remain.

Task 8 amends the CLI RFC, relationships RFC, Wave 03 C3 section and API audit.
It documents the thirteen delivered verbs, nine closed inputs/examples,
one-read save fill and replay limits, scoped fill refusals and decision-7
recovery differences, complete ticket-check proposals, path/context semantics,
index request IDs, identity retry and primary-only folders. F2 cache-loss
codes replace the generic paragraph, with its Departures citation. The RFC
copies Task 7's exact nine-case fixture/outcome table: every example ok/0,
only observation fields substituted, corrected source and other fields intact.
The audit updates its existing thirteen rows, without changing any result set
or the **40 library / 42 CLI-inclusive** union.

The Wave retains all approved runtime qualifications: primary-only committed
move-to-self is noop/0, save no-ops need HEAD-equal content, partial envelopes
classify as exit 4, the pending fence applies to new domain operations, and
index rebuild retains its source-inspected root-selection exception.
Dogfooding is explicitly **OWED** and needs Task 9's own authorization.

Validation: full amendment diff/self-review and whitespace check passed.
A read-only Node check validated **38 changed file/heading/line links**,
**11 scoped CLI amendment markers**, **9 schema field/required inventories
and example shapes**, **9 exact Task 7 outcome rows**, **13 available C3
audit/catalog rows**, stale-text removal and the unchanged **40/42** union.
The library matches Task 0; src/tests/schemas/Cargo/AGENTS/skills match the
Task 8 base. No Rust tests or gates ran for these documentation-only edits.

Preflight/checkpoint fetches retained main at `e49f978`; merge into the ticket
was already current and merge-tree passed. Comment ID came from the built
CLI's id new through Devenv, timestamp from the UTC clock. The ledger records
full source checks and validation details, including the unavailable Python
validator attempt and successful Node execution.

Only the four named documents, execution ledger and this comment change.
Task-only commit and ticket-only push; independent Task 8 review is next.
No agents or Task 9 work. Full verification, whole-branch review and dogfooding
remain owed; this is not Cycle completion, native CI, PR, main integration,
closure or cleanup.

Evidence: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`.
