---
manyhands_managed: true
manyhands_kind: comment
id: "01M4779ANC9VQ5H30DHZAH5CZJ"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T23:44:49Z"
---

Run 37387522542 passed both Linux and both Windows targets. macOS probe6 confirmed an interrupted TCP connect (512 1 1), before SSH callbacks. Controlled real libgit2 SIGCHLD reproduction and in-process helper cleanup establish the fixture signal-coupling mechanism addressed here; native logs do not themselves record the signal number.

Fixture-only correction 8e5ef0e blocks SIGCHLD on the isolated client case thread and explicitly permits server runtime async/blocking threads to receive it and reap children. One thread-bound guard restores the complete previous mask after fixture cleanup, including error/unwind paths. No production masking, retries, error-classification changes, assertion relaxation, or settling sleeps. Windows remains unchanged. Ruling14 potential cost: Unix test signal-routing complexity; separate-process server isolation is a larger fallback.

Three meaningful regressions fail on the old/partial implementation and pass with the complete correction: actual poll interruption protection, exact mask restoration including unwind, and two-fixture worker routing/helper reaping. All45 transport cases and privacy/cleanup checks pass. Independent spec/quality review approved without findings. All four required parent Devenv gates pass on 8e5ef0e:585tests including31Unix fixture+45transport; no failures/ignored. Windows retains28fixturecases.

Publish for native correction confirmation. The bounded30-triple macOS probe remains fail-first and precedes the unchanged required full suite. PR/ticket stay open; no merge, closure or cleanup.
