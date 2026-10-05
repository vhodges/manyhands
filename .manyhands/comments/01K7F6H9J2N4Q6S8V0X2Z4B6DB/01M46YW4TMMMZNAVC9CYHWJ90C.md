---
manyhands_managed: true
manyhands_kind: comment
id: "01M46YW4TMMMZNAVC9CYHWJ90C"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T21:17:48Z"
---

User authorized push and PR. Published [PR #9](https://github.com/vhodges/manyhands/pull/9).
The original remote checkpoint is preserved in merge 5519991; its tree was
verified identical to reviewed d5deb20. No history overwrite occurred.

[Native run 37373092577](https://github.com/vhodges/manyhands/actions/runs/37373092577)
tested head 5519991: macOS ARM64 and Linux ARM64 passed. Linux x86-64 was
cancelled before starting. Both Windows jobs failed fixture helper provisioning
before Rust execution, so neither established Windows compilation/runtime results.

Diagnosis: Git for Windows may omit standalone dashed builtin executables, but
provides upload-pack/receive-pack through git.exe. The fixture and workflow
incorrectly required standalone files. A focused subagent is fixing test-only
helper discovery/invocation and adding a real SSH fallback regression. Native
acceptance remains pending the reviewed fix and another complete run.

Implementation ruling: permit a resolved Git executable with a fixed builtin
subcommand when standalone helpers are absent, preserving standalone preference,
exact SSH allowlist, separate owned repository arguments, and no shell. Potential
cost if wrong: revise the test helper discovery/invocation boundary.

The ticket remains open. Merge and cleanup have not been requested.

Follow-up: fix c9c3dc3 is committed and independently approved with no findings.
The new fallback regression and all four required local gates pass: 582 tests
including 28 fixture and 45 transport cases, no failures or ignored tests.
Publishing the amended revision for a fresh five-target native run.
