---
id: 01M70MWMRKV4MG7GWFA6KADM97
item_id: 01M6V8RK8B0SBCH0YZXFVSR9E3
created_at: 2026-11-09T17:14:58.847891624Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Owner replies applied (2026-11-09). Ledger: section "Owner replies to the Task 12 report".

1. **Main's 93 lib failures: confirmed to fail on main.** On a plain git archive of origin/main f4442618 (outside any worktree, own target dir), `cargo test --all-features --locked --lib -- repository::context_retirement::` gave 49 passed, 93 failed, the IDENTICAL set as this branch, same NotFound panic reading a fixture's .manyhands/config.toml. So it is main's, not this Cycle's. Not fixed here; follow-up on main in a separate session.
2. **Time allowance raised** (42440a0d). Correction: the harness wait was already 60 s, not the 10 s I reported earlier; the flaky test failed after 60 s of real time under the full parallel run. It is now 180 s (a constant and comment only). No test relies on a wait expiring (checked every user). The slowdown of that test (about 5 s before this Cycle, about 9 s now) is noted for the performance work. The full parallel run was not repeated.
3. **K-19 accepted.** 4. **Document amendments approved** and applied to the design and the Cycle document (746e347d), with K-19 to K-21 and Cycle decision 20.
5. **Formal on-screen checklist not run now**; the owner verifies as the work lands; the checklist is kept for later. The "on screen (owner to run)" entries stay open and no on-screen result is recorded.
6. **ChromeColors.selection stays in** (light/dark palette).

Not closed, merged, cleaned up, or sent to CI; none of that is authorized.