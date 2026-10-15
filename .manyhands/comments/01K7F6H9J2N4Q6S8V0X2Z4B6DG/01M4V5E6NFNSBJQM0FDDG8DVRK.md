---
manyhands_managed: true
manyhands_kind: comment
id: "01M4V5E6NFNSBJQM0FDDG8DVRK"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
created_at: "2026-10-13T17:37:20Z"
---

Task 3 spec ses_ec9b35ee6ffeBdb0HkkfCCp9R4 passed; quality
ses_ec9b0a1d9ffeWP162PBxjUK6l4 blocked Important: initially absent tracking
creation is not atomic against external Git. Fix base
dc96f0b71fad8bba911f349bbf53140864b81d1b is clean in the assigned worktree.
New local fix commit authorized; Task 4 remains unstarted.

Verified locked libgit2 1.9.7 source: force=false existence check precedes the
ref lock and NULL expected skips comparison. reference_matching with zero
expected and force=true performs absence comparison under the ref lock;
NotFound matches zero, existing valid direct/symbolic refs do not match.

Extracted the old writer unchanged, then Devenv focused RED:
cargo test --locked --lib transport_absent_tracking_guard exited 101,
0 passed/2 failed/529 filtered. Public Transaction lock test expected Locked,
got Exists; competing creation test expected Modified, got Exists. No timing,
sleep, cfg(test) checkpoint hook or fake transport. This is deterministic
lock/compare-path evidence plus verified backend semantics, not a claim of
pausing C inside its check-to-lock window. Correction proceeds after this RED.
