---
id: 01M4VPNGXTADT2ASHG97ZQH66Q
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-13T22:38:26Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Task 4 spec P1 from `ses_ec8bd9b3effeS3OD0g7achFQNy` technically corrected,
awaiting spec re-review; quality not started. Incremental fix base
`e08d42d8cfdbc1aa5798c30a94d554d4b7f175e9`; cumulative Task 4 base
`961b3c21db11f797e5bd1cdac0bf85d748cd1dec`. New local head returned after commit.

Before production, regression RED **0 passed/1 failed/547 filtered** proved
unrelated afile changed and dir/child deleted while index/HEAD/refs stayed old
and ignored sentinel survived. The poll now inspects retiring directory entries
against the old tracked tree before calling unchanged sync::fast_forward. Extra
or foreign entries are Dirty; unreadable state is Inaccessible, not guessed clean.
Only targeted retiring subtrees are inspected, without file-body reads or
symlink traversal. Shared helper bodies and approved contract/amendment unchanged.

Final real-local-repository matrix: **20 obstruction cases**, including root/
nested directory-to-file/symlink, ignored files/deeper tracked descendants/empty
directories/symlinks, all Dirty with complete snapshot and outside-sentinel
preservation. **4 clean controls** advance branch/index/files while preserving
noncolliding ignored state. No fake transport, sleeps, global Git mutation or
production testing hook. Initial fixture compile borrow error corrected before
the recorded behavioral RED.

Sequential Devenv GREEN: poll **25**, remote umbrella **322**, required check/
fmt/Clippy/full-test gate PASS. Full count **1,625 passed, 0 failed, 1 existing
ignored** (548 library, 779 ordinary integration, 289 real SSH, 9 doctests).
Unchanged shared/SSH suites include synchronization 63 and merge recovery 25.
Only two poll Rust files plus ledger/ticket/two comments changed in this fix.

Ticket remains open, no Task 5, no subagents. Deferred Task 2 minor direct-SQL
kind-immutability regression remains for final Cycle review; native evidence
pending. Return for spec re-review before any quality review.
