---
id: 01M4W3AQQGAF0YJ28908ABW64Q
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-14T02:19:44Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Task 5 spec P2 ses_ec7f49995ffeVTGZBqRN4rzEoX technically corrected, awaiting
SPEC re-review BEFORE quality. Incremental fix base
4dc35de84fab433a299c4de8c640ad5647ceb15c; cumulative Task 5 review base
ab07d0d3096a0e6129add1ba32a2f977a131b80a; head returned after local fix commit.

Fully valid owned materialization no longer requires clean/original tip merely
to finish an unfinished intent. Exact Task 4 ownership/direct branch/index/path
checks remain. Unchanged canonical validator in existing final step 5 still
gates complete under SAME single lease, within the existing owner exception.
Original intent owner/creation OID retained; Git unchanged; Present delegates
Dirty/LocalAhead/Current classification normally. Branch-only resume retains
exact original branch_created OID and absent path/admin/registration. Partial,
malformed, foreign, symbolic and missing-index states retained incomplete.
No amendment, schema or shared-helper/Task 4 classifier change; D1–D4 unchanged.

Before production, exact Devenv regression command
cargo test --locked --lib remote::poll::tests::poll_materialization_owned_unfinished_
gave 1 passed/3 failed/567 filtered (wrong MaterializationIncomplete). GREEN same
command 4 passed, then whole poll 44 passed. Eight positive document/ticket cases
prove full root/target/config snapshot preservation, immutable original owner/OID
with complete, normal classification, one registration and repeated no-op. Three
negative controls prove unchanged malformed/foreign/missing-index state; malformed
now reaches final canonical validator. Existing partial/branch-only tests pass.

Sequential required Devenv check/fmt/strict Clippy/full-test gates PASS: 1,648
passed, 0 failed, 1 existing ignored (571 library, 779 ordinary integration,
289 unchanged real-SSH, 9 doctests), log tool_138173849001kHezuxNuakarWT. One
full gate for this fix, no timeout/trial failures after RED. CLI smoke exit 0.
No native claim; no Rust edit after final verification.

Only poll Rust files, ledger/open ticket and two Node-ULID comments belong to
this local fix. Status/diff/log, whitespace and scripts/show-ticket schema/
rendering checked before staging intended files. Task 2 SQL kind-immutability
minor remains deferred final. Ticket OPEN, Task 6 NOT STARTED; no subagents/
push/PR/native dispatch/merge/closure/removal. Return for SPEC re-review first.
