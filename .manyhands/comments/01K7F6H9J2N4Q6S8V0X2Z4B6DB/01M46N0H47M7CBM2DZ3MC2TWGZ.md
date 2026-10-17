---
manyhands_managed: true
manyhands_kind: comment
id: "01M46N0H47M7CBM2DZ3MC2TWGZ"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T18:25:26Z"
---

The user explicitly accepted the documented backend limits: 10 seconds per TCP
address connection attempt and 30 seconds per blocking SSH call. DNS is outside
the connect budget; partial progress inside a control call does not reset its
timer; teardown may add budgets; there is no whole-transfer deadline.

The read-only timeout investigation identified a safe early initializer and
verified a custom-main Cargo test-host source inclusion pattern in a disposable
probe. Task 2 now owns runtime/startup wiring and harness=false hosts; Task 4
requires successful bootstrap before networking. Both binaries initialize before
threads/GPUI construction. Native runtime timing proof remains required.

Implementation preflight freshly fetched origin/main (89ce24d), rebased onto
local main (a21a31a), and verified both ancestors. Rebase was a no-op at 5e111d9.
Baseline check, fmt, and Clippy passed; full tests are still building.
