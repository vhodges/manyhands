---
id: 01M6SSJJ827QBXA1Z2BFCPAHTJ
item_id: 01M4CC0VN2K8DBHNMZ46NTX5JJ
created_at: 2026-11-07T01:22:10.480034458Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
D5 is review-ready again at 5df4ed03 after the owner's max-effort review (15 confirmed bugs + Tier A).

Fixed (each with a test that failed first; independent reviews after each round): tests sweeping the real data dir; the lifecycle lock gate (save-first, prepare banner, comment retry/post-as-new, checkpoint retry, armed save-first, lock-ended draft); resolve sheet (undo, choices while saving, editor rebuilds, immutable fields, dismissed sets, quit guard); lifecycle dialog (raw recovery, busy after merge, resumed child conflict, duplicate resumes, Continue beside a resume); kept sets, lock exemptions, stale Review changes, shared chip; deferred Republish, Unknown record, stale Stopping.

Library (owner-approved): way_out.rs child_conflict; promotion/run.rs reports recording on a resend that writes the checkpoint; RepositoryService::data_directory() made public. Named edit: tests/fixtures/cli_v1/ticket_close_checkpoint_merge_refused_text.txt gains `progress: recording` (owner-approved). Test conventions: the 30 s wait and the icon test removed; d5_progress.rs cut from six SSH cases to one.

Gates, run once at review-ready: check, fmt, clippy --all-targets clean. Fast tier: lib 1380, bins 1579 + 305, canonical_foundation 65, the remaining 26 targets 2132, 0 failed (the cli golden failed once at 39782b90, fixed in 5df4ed03 and rerun). SSH tier at 39782b90: document_promotion, ticket_closure, cli_remote and mutation_remote (81 min) all pass. check-cli-no-gpui and CLI smoke pass. merge-tree with main 79c3931b is clean.

Ledger row 17; limits K-28 to K-33 (design); deferred items are in follow-ups ticket 01M6RAS63EWVHWWVBSZ836RXG2 (committed on its local branch; ticket save not run).

Waiting on the owner: the on-screen checklist (Linux Wayland), code-review approval, then ticket close at the owner's terminal. Native CI only if dispatched.