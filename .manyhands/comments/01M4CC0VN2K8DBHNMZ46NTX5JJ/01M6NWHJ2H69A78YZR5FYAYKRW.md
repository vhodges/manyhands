---
id: 01M6NWHJ2H69A78YZR5FYAYKRW
item_id: 01M4CC0VN2K8DBHNMZ46NTX5JJ
created_at: 2026-11-05T12:57:14.017548779Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
## Part A checkpoint (Task 9)

**Part A is done.** Tasks 5 to 9 gave the desktop its lifecycle: the Promote document… and Close ticket… actions, the dialog (save-first, prepare, blocked, confirm, stale, running, failed, stopped, done), made-ready, and the item lock while a lifecycle runs. Each task had an independent review, and every finding was fixed or recorded. See the execution ledger.

**Task 9 (lock).** Commits `9dc51dc1` and `31c632ff` (review fixes). The item is read-only while its lifecycle locks it (R-21 to R-23): Edit, Save, Share, comment Submit, Move, Repair, Continue editing and Drafts Restore wait with "Closing…" or "Merging…". "Merged, but not tidied up" offers Open folder and Check again (L-4, R-22). No library change.

**Known limits added:** K-19 (two windows with the same item's comment draft spin; the app opens one window) and K-20 (after a restart, a ticket past `closing` stays "Closing…" until Task 14's Resume).

**Main.** Unchanged at `c69789d9`, already merged.

**Verification at `31c632ff`:** the whole desktop bin suite, 1412 passed, 0 failed; fmt, check and both clippy lines clean.

**Next.** Part B, Task 10 (conflict read).