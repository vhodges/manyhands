---
manyhands_managed: true
manyhands_kind: comment
id: "01M4VQFGNCG4TE2MMA6WD27C3Z"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-13T22:52:37Z"
---

Controller reports spec `ses_ec8bd9b3effeS3OD0g7achFQNy` passed through clean
fix base `6daaaec520aeb37e777185cc28b8576e51b43383`. Quality
`ses_ec892692effePXdmvv7I1Ys7ER` BLOCKED two Important findings: matching
workdir/branch/OIDs do not prove opened context ownership; docs/.md has no
extension and is skipped by discovery despite poll acceptance. Poll-only fixes
authorized, frozen shared helper bodies and approved agreement unchanged.

Meaningful RED for each before production, sequential Devenv:
- `cargo test --locked --lib remote::poll::tests::poll_context_identity_`:
  **0 passed/3 failed/549 filtered**, exit 101. Standalone/foreign-linked repos
  replace the deterministic directory while root registration survives and
  matching objects/branch are present. Poll wrongly fast-forwards them; full
  snapshots detect changes. Same-root/wrong-admin identity fails too. Auxiliary
  fixture backlink setup corrected before this behavioral run.
- `cargo test --locked --lib remote::poll::tests::poll_document_selection_matches_discovery_extensions_including_dotfiles`:
  **0 passed/1 failed/551 filtered**, exit 101: docs/.md accepted rather than
  rejected. Nested .md, .hidden.md and ignored-duplicate controls included.

Locked Worktree::validate only proves shape/presence, not exact ownership;
poll-local checks must bind commondir, registered admin gitdir and backlink.
Document source selection must match Path::extension == md. Fixtures use public
ODB object copying, synthetic local identity, no transport/fake/sleeps/global Git
mutation. Full details in ledger. No production correction precedes this RED.

Ticket open; spec re-review after corrections precedes quality re-review.
Cumulative review base `961b3c21db11f797e5bd1cdac0bf85d748cd1dec`; Task 2 SQL
kind-immutability minor deferred. No subagents, later tasks or contract amendment.
