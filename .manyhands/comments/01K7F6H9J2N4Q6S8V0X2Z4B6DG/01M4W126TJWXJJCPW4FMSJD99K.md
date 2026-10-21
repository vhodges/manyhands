---
manyhands_managed: true
manyhands_kind: comment
id: "01M4W126TJWXJJCPW4FMSJD99K"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-14T01:40:07Z"
---

Task 5 spec ses_ec7f49995ffeVTGZBqRN4rzEoX BLOCKED P2: unfinished intent's
fully materialized owned context incorrectly demands clean/original OID before
Present. Fix-only assignment at verified clean base
4dc35de84fab433a299c4de8c640ad5647ceb15c; cumulative review base
ab07d0d3096a0e6129add1ba32a2f977a131b80a. Spec re-review before quality.

Meaningful RED before production: devenv shell -- cargo test --locked --lib
remote::poll::tests::poll_materialization_owned_unfinished_ exits 101:
1 passed, 3 failed, 567 filtered. Successful creation interrupted at public
PollAfterMaterializationWorktree leaves branch_created. Untracked file, valid
canonical body edit and actual local descendant commit then each return
MaterializationIncomplete, expected Present. Clean Current control passes;
malformed/foreign/missing-index controls already pass. Positive assertions cover
full root/target/config preservation, original intent owner/OID retained with
complete, normal classification and repeated no-op. Document/ticket loops finish
at GREEN. No setup/compile trial; production/helpers/documents unchanged at RED.

Authorized correction affects only fully valid owned materialization predicate;
branch-only resume stays exact recorded branch_created OID with no path/admin/
registration. Final unchanged canonical validator still gates completion under
the already-approved step-5-only same-lease exception; no broader scan/amendment.
D1–D4 unchanged, Task 2 SQL kind-immutability minor deferred final. Ticket open;
Task 6 unstarted, no subagents/outward actions. Production correction follows RED.
