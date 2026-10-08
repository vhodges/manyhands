---
manyhands_managed: true
manyhands_kind: comment
id: "01M4DYC8RPXEXKPVYB54BMTCTZ"
item_id: "01M4CC0VMQ7R15A7M9SPN3KB67"
created_at: "2026-10-08T14:23:49Z"
---

Task 8 checkpoint (relationship view and index edges): complete at
`de6f876`, range `fcbe353..de6f876`. A ticket's `slug`, `parent` and `deps`
are read from its front matter, stored in the index (a slug column and two
new tables, written with the items in one transaction) and shown on item
reads. No save path changed; the fields stay in the file as written.

Independent review: no blocker; the write path, migration and save path were
judged sound. Fixes in `6374474` and `de6f876`, not re-reviewed. Controller
decisions on points marked for the product owner, reported to them: a null
value is absent; an uppercase slug is accepted and lowercased; problem
objects gain `target_id`; `parent` is `{id, state}`.

Product-owner decision: no content stamp for older builds in F1; no builds
are in use yet. Index versioning (schema and content) goes to a ticket at
handoff.

Evidence through Devenv on Linux at `de6f876` (controller, full gate):
`cargo check --all-features --locked`, `cargo fmt --check`, clippy with
warnings denied and `cargo run --locked --bin manyhands-cli` exit 0.
`cargo test --all-features --locked --no-fail-fast` exited 101 with 915
passed and 1 failed, plus 15, 35, 31 and 103 SSH cases passed. The failure is
`concurrent_corrupt_rebuilds_replace_the_cache_once` in `discovery_rebuild`:
the second rebuild was refused as busy. The test file is unchanged by F1. It
then passed in 8 single runs, 6 runs of the whole target and 40 runs eight at
a time. Whether F1's extra index work makes this timing-sensitive test more
likely to fail was not measured. Task 10's gate must pass cleanly.

Next: Task 9, relationship queries.
