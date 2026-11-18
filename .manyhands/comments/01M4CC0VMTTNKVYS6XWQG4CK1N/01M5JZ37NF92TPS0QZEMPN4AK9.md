---
manyhands_managed: true
manyhands_kind: comment
id: "01M5JZ37NF92TPS0QZEMPN4AK9"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T23:28:16Z"
---

## Part B Spec Review P2 — Execution-Output Privacy

Review `ses_e9a146cbdffeOvI7kGAmzMRtSQ` found that privacy coverage counted
command names while several cases returned before their mutation bindings:
remote select and key select/clear/remove used prepare, generate returned
unlock_required, and delete refused an imported key. Verified and corrected
under the owner's targeted test-only authorization.

The strengthened test now requires successful non-prepare execution outputs for
all 15 available C2 verbs in each mode on Unix. Human and JSON use independent
isolated fixtures and requests, so JSON is not merely replaying human mutations.
Registry mutations remain cross-platform; real generated-key create/delete are
Unix-only (13 successful verbs per mode on other platforms). Confirmed verbs
execute after previews, including repository and remote removal. Generated files
are asserted present then absent; imported source bytes/mtimes are preserved.

Every execution, preview and companion read has both streams scanned for C1
URL/private-key/operation-material sentinels as UTF-8 and UTF-16LE. JSON asserts
exact codes; human mutations check their finished request's exact stored code
after scanning actual human output. Previous refusal cases remain additional
coverage, now with explicit expected codes. Protected-generation PTY scans are
unchanged.

Verification through Devenv:

- **RED:** `cargo test --locked --test cli privacy` — 1 passed / 1 failed;
  execution inventory exposed four successful command names instead of 15.
- **GREEN:** same command — **2 passed / 0 failed**, 162 filtered, 6.56s.
- `cargo clippy --locked --test cli -- -D warnings` — passed.
- `cargo fmt --check` and `git diff --check` — passed.

The prior 448-test checkpoint was not rerun. No production/domain, schema or
golden changes. Self-review found no unresolved correction issue; controller
re-review and quality review remain pending.

Entry head was `84a6c92`; required fetch/merge incorporated main's comment-ID-only
fix, giving correction base `50ece1ad9f89cddbb61a9d9c0abac684a2da6b2d`.
The correction is this comment's containing commit, with the required coauthor.
ID allocated using built CLI `id new` in a fresh isolated detached session.
No subagents, push, native CI, closure or Task 10 work.
