---
id: 01M8F40ZPERZYGCWH97BEHFVS9
item_id: 01M87684C9GM9BJP5QQ429KMAK
created_at: 2026-11-27T18:24:35.083552506Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready (2026-11-27). The branch is at 4d436d9d. The code ends at 9867a1a6; the later commits are the ledger only. Main has not moved (8e5d9832), and git merge-tree against origin/main is clean.

What landed. In pull-request mode, document promote is a promotion record run by Cycle 03's branch path with the context child only. It publishes the document's branch and answers with the link, and it never writes the primary. Promoting again synchronizes (it publishes saves and merges the remote's primary in) and answers already_applied with the same link. The link ends by the content proof: once the primary holds the document, promote answers already_applied with no link and document show gives null. document show uses a cheaper, entries-only twin of the proof, and item sync gives a document's outstanding link. A stopped promote re-runs, ends or waits by the re-run table RR1 to RR8. Retirement is fenced while a pull-request promotion is unfinished (F1 to F4). The CLI has a preview and a mutation shape, two schemas and four goldens. The contract, both RFCs, the audit and both skill trees are amended.

Static gates on df5cdc96: cargo check, fmt --check and clippy -D warnings are clean.

Fast tier, once, on df5cdc96, with --no-fail-fast: 97 min. Every target is green except the three known desktop-scheduler load flakes: check_now_failure_becomes_attention, unlock_after_cancelled_prompt and user_job_waits_for_retirement. Each passed when rerun alone by exact name (6 s, 1 s, 10 s). The branch does not touch the scheduler; its only desktop change is in src/desktop/view/confirm.rs.

SSH tier, once, on df5cdc96, all green, 92 min in all. AGENTS.md's table selects these targets for src/repository/promotion/, mutation/, closure/ and remote/: document_promotion 415 s, mutation_remote 3026 s, ticket_closure 397 s, cli_remote 769 s, remote_polling 243 s, ssh_transport 143 s, remote_observation 12 s, remote_synchronization 319 s, remote_merge_recovery 176 s.

SSH steps (decision 6, no new case). Cycle 03's a_pull_request_close_publishes_only_the_ticket_branch gains document steps. Promote answers ok with the link and published. Promote again answers already_applied with the same link and current. After each step the server's primary, the ticket branch and the local primary are unchanged, and only the document's branch was pushed. This case writes the golden mutation_v1 document_promote_pull_request_ok.

Final whole-branch review (Opus): 0 Critical, 3 Important, 6 Minor. It found no pull-request path that reaches the primary. The direct promote, the close and the withdraw are unchanged where they share code. The one fix wave (5bc766bd..9867a1a6) addressed:

I-1: item sync of a document now reads the link row before running the content proof.

I-2: RR8's answer is tested at the binding.

I-3: four contract and skill statements corrected.

Minor findings M-1, M-2, M-4 and M-6.

A scoped re-review found everything addressed and nothing new above Minor. The reviewer also confirmed, by a read-only, immutable-mode look, that the real registry has not been migrated.

Named test edits: E-1 to E-22 all applied; E-10 and E-14 needed no change. E-15 to E-17 came from Task 0's sweep. E-18 to E-22 came from Task 4's checkpoint, under the owner's test-edit approval; all are additive.

Owner decisions during execution (2026-11-27), recorded in the Cycle document:

17: a held child and a changed remote wait, then finish once the remote is configured back.

18: RR7 over a retained child answers recovery_required and waits (known limit K-19).

19: a configuration change registered while a conflict is pending is a known limit here. Its pre-existing defect is filed as vh-zn1fd.

K-20 is by ruling: a rename while an attempt may have pushed waits, and a crash-and-save instant before the child's push check is the residual.

Acceptance A1 to A14 are all met; the evidence is in the Cycle document (4e297cea, published).

Left for follow-on work, not merge blockers:

M-3: move hold_refusal out of the closure module.

M-5: the ticket arm's format! tracking ref.

Cycle 03's manyhands-tickets/SKILL.md:286 (both trees) says a fenced closure retirement names "the request" rather than the operation.

vh-zn1fd.

About 50 deferred Minor findings, each triaged by the final review as able to stay, with its reason in the execution ledger.

Native CI has not been dispatched; it runs after merge, on main, only when the owner dispatches it.

Next: the owner's review. After approval, the status is set to done, item sync, then ticket close --prepare, shown to the owner before confirming. Then D-6: Cycle 04's boxes in the Wave 06 document, and the three documents set to done and promoted, each with the owner's go-ahead.