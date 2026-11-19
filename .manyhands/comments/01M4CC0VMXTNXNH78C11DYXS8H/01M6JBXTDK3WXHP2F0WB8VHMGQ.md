---
manyhands_managed: true
manyhands_kind: comment
id: "01M6JBXTDK3WXHP2F0WB8VHMGQ"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M6J8TRPMRC4978C729QY9N3W"
created_at: "2026-11-04T04:08:57Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## The scoped review's rulings applied

- `87bffef7`: F2 reverted, since a promotion's integration conflict can be a modify/modify. A blocked
  prepare on `integration_conflict` prints the four `context` lines for both verbs, plus a note that
  `external_resolution_required` is repaired with Git in the context. Its golden, recovery-table row,
  tests, walk comment and design sentence are reverted to match.
- Decision 35's wording (a fetched deletion, once its tracking ref is pruned) and its place after 34;
  `RemoteContextHistory`'s three-way doc; the ledger's evidence sentence; the outcome tests' doc.
- Verified: fmt, clippy, `--bin manyhands-cli` 305, `--test cli` 474, the fast tier (79 results,
  5163 passed), `cli_remote` 161, all exit 0; merge-tree clean. Not review-ready.
