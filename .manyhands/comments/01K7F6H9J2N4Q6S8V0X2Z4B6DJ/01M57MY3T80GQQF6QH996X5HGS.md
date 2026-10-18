---
manyhands_managed: true
manyhands_kind: comment
id: "01M57MY3T80GQQF6QH996X5HGS"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DJ"
created_at: "2026-10-18T13:59:03Z"
---

Full verification after the final-review fix rounds, on `35d4d39`:

- `cargo fmt --check` and Clippy (`-D warnings`) are clean.
- `cargo test --all-features --locked`: 3,325 passed (2,704 harness tests
  and 621 SSH cases), 0 failed, 1 ignored.
- `origin/main` is still `3765524`, and
  `git merge-tree --write-tree HEAD origin/main` is clean.

The owner confirmations listed in the review-ready comment are still
pending. Push, pull request, merge, closure and cleanup each need their own
authorization.
