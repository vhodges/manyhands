---
id: 01M4XWFCGW9N53Y99FMXFHPGER
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-14T18:58:25Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Owner authorized a normal branch push and one five-target Build run. Published
reviewed commit `3d8f8a5a054fd74dc83eb2a4a93c260769177ce8` by fast-forward from
`094e5734d1d4480ee29e6f675aaba4a8883b9060`; no force push. Dispatched
[38612548902](https://github.com/vhodges/manyhands/actions/runs/38612548902) on
that exact commit and watched it through completion. Overall conclusion:
**failure** (18:08:00Z–18:56:51Z on 2026-10-14).

| Target | Result |
| --- | --- |
| Linux x86_64 | PASS; 1,700 configured tests, including 84 polling SSH cases; artifact uploaded |
| Linux aarch64 | PASS; 1,700 configured tests, including 84 polling SSH cases; artifact uploaded |
| Windows x86_64 | FAIL library worktree-identity fixture; 578 passed, 1 failed; integrations not reached |
| Windows aarch64 | FAIL same library fixture; 578 passed, 1 failed; integrations not reached |
| macOS aarch64 | FAIL polling in-transfer yield test after 48 passing polling cases; later tests unproved |

All five release builds and binary verifications succeeded. Windows fails
`repository::remote::poll::tests::poll_context_identity_requires_exact_registered_admin_and_gitfile_backlink`
at `poll_tests.rs:5065`: the fixture's opened workdir points to the runner
checkout rather than its temporary deterministic context. macOS fails
`poll_automatic_in_transfer_yields_to_manual` with `isolated child or watchdog
failure`; filtered output does not establish the underlying assertion or timeout.
The same in-transfer test passes on both Linux targets.

Final job metadata, failed-step logs and both Linux success logs were inspected.
Full job links, count breakdowns and log references are in the execution ledger.
Native acceptance is blocked by the observed failures. Local 1,779-test evidence
and review approvals remain historical local evidence, not a five-target pass.

This is a result-recording checkpoint only. No source/test/workflow changes,
local Rust reruns, retry/second dispatch, PR, merge, closure or cleanup occurred.
Fresh authorization is required for another dispatch. Ticket remains open.
