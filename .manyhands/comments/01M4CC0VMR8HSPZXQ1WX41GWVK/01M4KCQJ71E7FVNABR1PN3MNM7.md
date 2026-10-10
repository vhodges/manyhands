---
manyhands_managed: true
manyhands_kind: comment
id: "01M4KCQJ71E7FVNABR1PN3MNM7"
item_id: "01M4CC0VMR8HSPZXQ1WX41GWVK"
created_at: "2026-10-10T17:10:51Z"
---

Part A checkpoint (Tasks 1–4: short codes and initials, relationship writers,
the relationship check, the comment author). Commits `fa883bd..d0dbc40`.
Parts B and C are not authorized and not started.

**Verification at `d0dbc40`**, through Devenv on Linux x86_64, each exit 0:
`cargo check --all-features --locked`, `cargo fmt --check`,
`cargo clippy --all-targets --all-features --locked -- -D warnings`,
`cargo test --all-features --locked --no-fail-fast` (1307 passed, 0 failed),
`cargo run --locked --bin manyhands-cli`. Baseline before Task 1 at `fa883bd`:
1251 passed. No workflow was dispatched; Windows and macOS are unproven.

**Review.** Each task had an independent spec and quality review; Tasks 2 and
3 each needed one fix round. An independent whole-range review found no
Critical issue and two Important ones, both fixed and re-reviewed: the
relationship check now reports the index state and completeness it answered
from, and `canonical_foundation` and `mutation_relationships` are in the
native workflow's test list.

**Rulings made during execution.**

1. Task 3 registers the result codes `relationship_cycle` and
   `invalid_relationship`, ahead of Task 5.
2. The existing parser reorders the unknown keys of a hand-laid-out file once,
   on its first save that writes; left as it is. Product owner: reordering
   keys is okay.
3. An untouched `deps: []`, `deps: ~` or `parent: ~` is removed by any save
   that writes.
4. A second F1 test that pinned unsorted `deps` through a save
   (`tests/read_items.rs`) was updated to the sorted order; the design
   counted one such test.
5. The relationship check is a successful read whose data carries the
   verdict (`rejection` with a code and IDs). Product owner: okay.
6. Six details the design left open in the check: typed input holding full
   proposed values; `unresolved` sorted and unique over both fields; a
   self-reference is a cycle of one; `invalid_relationship` names the
   offending IDs; both codes are input errors.
7. Order of answers when several apply: invalid target, self-reference,
   dependency cycle, parent cycle.
8. Cycle 06's conflict-resolution guard on a comment's author compares the
   new `created_by` field as well as unknown metadata, and one assertion in
   `sync_tests.rs` reads the field.
9. The relationship check returns `index` and `complete` like the sibling
   ticket reads.
10. The two test targets were added to the native workflow's list now, not in
    Task 19.
11. The goldens name the check `ticket check`. Product owner: okay.

**Known and accepted.**

- A retry of a comment submit whose first attempt never wrote adopts a
  foreign file with the same ID, parent and body and any valid author; the
  commit is under the retrying identity. Author-less files were already
  adopted this way.
- An external Git commit that nothing has refreshed or flagged still reads as
  a current, complete index in every index-backed read, the check included.

**For Part B.** `ProposedRelationships` is typed, so the ticket bindings map a
non-ULID entry to a code themselves. A `Set(deps)` equal to the file's list as
a set but in another order writes and commits. A create retry must pass a
short code if and only if the first attempt did. Fifteen minor findings are
deferred in the execution ledger.
