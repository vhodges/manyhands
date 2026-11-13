---
id: 01M4VSWK7DJCKF0FWEA82VZC6Q
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-13T23:34:43Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Both Important findings from quality `ses_ec892692effePXdmvv7I1Ys7ER` are
technically corrected, awaiting spec re-review then quality re-review. Prior
spec `ses_ec8bd9b3effeS3OD0g7achFQNy` approval through 6daaaec5 is historical
controller-reported evidence, not approval of this correction. Fix base
`6daaaec520aeb37e777185cc28b8576e51b43383`; cumulative Task 4 base
`961b3c21db11f797e5bd1cdac0bf85d748cd1dec`; new head returned after commit.

Poll-local ownership wrapper now proves linked-worktree identity, canonical
owner commondir, exact registered admin gitdir/name/path, and exact regular
.git backlink. Classification and update preparation use it; immediately before
safe checkout the poll uses a freshly opened, revalidated owned handle. Invalid
or unreadable identity is Inaccessible before effects. Full root/target/foreign-
owner snapshots remain unchanged for standalone, foreign-linked, same-common
wrong-admin and malformed-backlink cases. Shared helper bodies are untouched.

Poll document selection now matches discovery's extension == md boundary:
docs/.md and nested .md rejected; .hidden.md selected; ignored .md duplicate
does not invalidate the selected visible item. All accepted controls are active
in existing discovery. Approved agreement, other conformity/comment rules and
tree-reader/discovery bodies unchanged.

RED before production: identity filter **0 passed/3 failed/549 filtered**, with
wrong FastForwarded and mutated snapshots; selection **0 passed/1 failed/551
filtered**, with docs/.md wrongly accepted. Final sequential Devenv GREEN:
poll **29**, remote **326**, required check/fmt/Clippy/full-test PASS. **1,629
passed, 0 failed, 1 existing ignored** (552 library, 779 ordinary integration,
289 real SSH, 9 doctests). All shared/SSH suites remain unchanged and green.
No fake transport, sleeps, developer keys or global Git mutation.

Only the two poll Rust files plus ledger/ticket/two comments changed; no contract,
schema or dependency amendment. Ticket open, no Task 5/subagents/push. Deferred
Task 2 SQL kind-immutability minor remains for final review; native evidence
pending. Return the exact complete range for spec first, then quality.
