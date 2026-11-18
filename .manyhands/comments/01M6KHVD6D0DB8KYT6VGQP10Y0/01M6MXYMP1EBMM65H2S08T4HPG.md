---
id: "01M6MXYMP1EBMM65H2S08T4HPG"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6MX09FC0R79VR76JGFA269D"
created_at: "2026-11-05T04:02:27Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Remaining S1 gitfile backlink correction — re-review pending

Controller's same SPEC re-review at
`d11655ea1ab05ccb03d14bd26c6ce5bfcf1fac15`: **S2/S3 resolved**, **one Important
S1 current-absence gap remains**, quality held. This additive correction preserves
that parent/history and the full Task 2 base `7b1912d1`; it is not a policy or
authority extension. Fetch/merge from this ticket found main already up to date.

The old shared inventory validates canonical admin links but does not read the
worktree `.git` contents; libgit2 derives its worktree path from the admin link.
New behavioral tests reproduce false `AlreadyAbsent`/D14 consumption with a
foreign gitfile target, malformed contents, a symlinked gitfile and a redirected
ancestor. **RED: 1 passed / 4 failed**, including a passing relative-link control;
**GREEN: 5 passed**, with all foreign/malformed table subcases executing.

The new **retirement-only** inventory complement no-follow checks the gitfile,
its ancestors and target admin, requires a regular gitfile with exactly one
`gitdir: <target>` record, resolves relative targets at the worktree and requires
the exact owning admin already bound to the leased common repository. A different
admin of the same common repository is refused too. **Cleanup/release behavior and
`cleanup.rs` are unchanged.** No foreign repository opening or new lease.

Full admin/external resource byte/type/link images, old materialization, remote
rows and repository metadata are preserved. Refusal flags assert no effects or
intent settlement. Same-request replay preserves complete local/companion/attempt
rows; fixture repair of regular gitfile contents does not change the old refusal.
The existing complete-unrelated positive control and relative/CRLF gitfile control
still permit genuine absence. Windows junction/reparse fixtures are cfg-gated and
not run/cross-compiled on Linux; no native validation is claimed.

Frozen serial Devenv evidence in `/tmp/opencode/`:

- `task2-specfix-s1-backlink-{red,green}.log`: **1/4 RED → 5/0 GREEN**.
- `task2-specfix-s1-backlink-controls-final.log`: **14 passed**, 0 failed, including
  all prior S1/S2/S3 controls and the five new Linux tests.
- `task2-specfix-s1-backlink-context-final.log`: **50 passed**, 0 failed (prior 45 +
  5 tests, not matrix groups).
- `task2-specfix-s1-backlink-{check,fmt,clippy}-final.log`: each exit **0**, full
  check/fmt/strict all-target/all-feature Clippy; exact commands/source fingerprints
  and scope/self-review recorded in the execution ledger.

No unchanged cleanup/proof/broad baseline, recovery/foundation or SSH gate rerun;
the requested context filter includes its unchanged proof tests. No zero-case
pass, push, native CI, agent, other-worktree edit or real-context action.
Index-only continuation/completed replay paths remain unchanged.

**Next: same limited SPEC reviewer re-reviews S1, then quality.** Neither pass is
claimed. Task 2 remains the limited initial-absence core; full Task 2, remaining
44-matrix cases and registered nonzero `mutation_remote retirement_` coverage
remain owed. No source/batch/destructive authority or Task 3/4 extension.
