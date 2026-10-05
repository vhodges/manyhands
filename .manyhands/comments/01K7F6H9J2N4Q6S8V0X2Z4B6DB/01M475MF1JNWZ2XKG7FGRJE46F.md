---
manyhands_managed: true
manyhands_kind: comment
id: "01M475MF1JNWZ2XKG7FGRJE46F"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T23:15:57Z"
---

Run 37385680755 on b0f9594 passed both Linux and both Windows targets. macOS passed the full suite, then failed renewed rejection at probe 13. Numeric evidence shows GenericError/Os before host/key callbacks, with the server having accepted all three TCP connections. The user's settling/socket-reuse hypothesis prompted a readiness and lifecycle audit: port zero is bound before readiness; the failure is a reconnect to the same listener. Old helper cleanup overlaps the reconnect.

A controlled real libgit2/Manyhands reproduction established that SIGCHLD can interrupt the backend TCP wait despite SA_RESTART, yielding TransportUnavailable. This is a candidate mechanism, not yet the confirmed native cause. Reproducer artifacts are preserved in the ignored execution evidence.

Reviewed amendment 6c3b2bd adds fixed numeric OS-reason classification, without raw error text or behavior changes. The unchanged bounded 30-triple macOS probe now precedes the full suite for prompt evidence; both remain required for green. Independent review approved without findings. All four required local gates passed: 582 tests, including 28 fixture and 45 transport cases, no failures or ignored tests. Publish for native reason confirmation. No production retry or fixture signal-routing changes yet. Ticket and PR remain open.
