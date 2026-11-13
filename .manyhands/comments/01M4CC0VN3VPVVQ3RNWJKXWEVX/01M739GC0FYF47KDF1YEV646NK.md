---
id: 01M739GC0FYF47KDF1YEV646NK
item_id: 01M4CC0VN3VPVVQ3RNWJKXWEVX
created_at: 2026-11-10T17:53:46.172405372Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Part E checkpoint (Tasks 11a, 11b, 12), 2026-11-10. Branch HEAD 9d8ab962 (code 6a034e39).

Done: the retirement loop after a completed poll (RT-1..RT-11), ContextRetire answers and rows, the "Retiring…" read-only state, Retire… with its two asks, Resume/Abandon and Unlock… for a pending retirement, entry reconciliation, and end-state/replay/quit-during-pending tests. Each task had a fresh reviewer and one review-fix commit; the ledger has the details (E-16..E-34, rulings, K-40..K-61).

Owner decisions: Task 11b design checkpoint (decision 28 covers any running retirement; an unreadable draft entry is unknown; a no-data answer stops the loop; a successful Check now is the latest attempt and may start or replace a loop: Document Amendment 11). 12b: accept the gap (see below).

Needs the owner's acknowledgement: Document Amendment 12 (the in-flight entry's exits: a Removed/NotSent answer keeps an entry the library holds; a successful Abandon/Resume ends it; an entry absent from a complete fresh pending list is reconciled away). These are design-silent gap fixes found in implementation and review.

NOT verified in code, on the native checklist: U-9's observer row and K-29 (Resume on it may fail with invalid_input), and K-60 (the real library lists a live pending retirement under the entry's operation id). Reason: a real ContextRetire observes over SSH first, which a desktop test cannot reach (the library's synthetic advertisement is cfg(test)). retire_open_ticket_refused_real was not written; the open-ticket refusal is covered by the canned E3 table. Tests 1 and 3 show already_absent, not a retirement with a removal outcome (K-58).

U-11 settled: Export works for a closed ticket's item and comment drafts.

Main: origin/main is eb0798a2, already merged; merge-tree against it is clean (full output read).

Tests: only exact-path runs; static gates (fmt, check --all-features, clippy -D warnings) pass. The fast tier and the remote_polling SSH tier run once at Task 14.

Known limits carried (K-33 is library-shaped; no ticket created). Follow-up ticket candidates for the owner to decide: K-33, exposing the hand-off-only flag (owes_only_its_hand_off) so the Resume rule cannot drift, a library test hook for a real retirement, and K-59 (an entry with no operation has no exit).

Next: Task 13 (privacy, consent, keyboard journey), then Task 14.