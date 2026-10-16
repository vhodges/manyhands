---
title: "Wave 02 Cycle 09 Confirmed Managed-Document Promotion Execution Ledger"
date: 2026-10-15
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M500Q0ZEMW43YTJBVHFBH3X6"
---

# Confirmed Managed-Document Promotion Execution Ledger

## Planning Preflight — 2026-10-15

- **Worktree.** Reused the registered ticket worktree
  `.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH` on
  `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Status was clean.
- **Base.** `git fetch origin` observed `origin/main` at `89d04f9` (docs:
  future-ideas notebook). This follows the Cycle 08 merge `df73c54`, and
  Wave 03 F1 and F2 are present.
- **Rebase.** The single ticket checkpoint rebased without conflict from
  `f017c73` to `69176ba`. The published ticket ref still holds `f017c73`, so a
  later push must reconcile it. No force push is authorized.
- **Main checkout.** Its untracked `.superpowers/` was left untouched.

## Grounding

The Cycle and design are built on these source facts, each observed at
`89d04f9`:

| Fact | Location |
| --- | --- |
| `promote` already in the remote action `CHECK`, enum, parser and DTO; no executor | `src/repository/remote/state.rs:637,1499,2487,5075`; `read/dto.rs` |
| Promotion classified not resumable at the F2 boundary and in status text | `mutation/bind/resume.rs:177,289`; `read/status.rs:295`; `remote/state.rs:2898,2937` |
| No-remote synchronization reports `PublishPending` without local integration | `remote/sync.rs:6091-6104` |
| Non-writing in-memory merge | `remote/sync.rs:702` `prepare_clean_merge` |
| Guarded fast-forward with safe checkout | `remote/sync.rs:7332` `fast_forward` |
| Only exact fetch/push mappings admitted; no delete; no `push_negotiation` | `transport/remote.rs:173,473,696,720` |
| No worktree prune, context branch deletion or remote ref deletion anywhere | repository-wide search |
| Pending local journal row blocks remote reservations | `recovery.rs:633` `require_no_pending_local` |
| Parent/child binding precedent | `comment_publication_bindings`, `recovery.rs:47` |
| Edit refuses a leftover branch whose tip differs from the primary | `repository.rs:1764-1780` |
| Batch marks absent `observed_published` rows `remotely_deleted` | `remote/state.rs` observation batch |
| Fixture receipt proof cannot prove a deletion | `tests/support/ssh_server.rs:692` |
| Digest precedent uses BLAKE3 | `remote/merge.rs:219` `ExpectedConfiguration` |
| Documents have no lifecycle front matter | `canonical.rs:91`; canonical schema RFC |

## Decision Audit

| Area | Settled by | Open → decision |
| --- | --- | --- |
| Confirmation before the first step | Git RFC, MVP RFC, PRD COLLAB-007 | Domain digest form, internal ruling |
| Final save/checkpoint | PRD "when needed"; CLI RFC "save drafts first" | **D1** |
| Synchronize before integration when a remote exists | Git RFC, PRD | — |
| Unpublished local primary commits | — | **D2** |
| Non-fast-forward merge | MVP RFC | Subject `Promote document <ULID>`, internal ruling |
| Primary conflict without a remote | — | **D3** |
| New remote observation invalidates consent | Desktop RFC, Git RFC Wave 03 boundary | When synchronization brings new content, **D4** |
| Remote branch deleted before promotion | Git RFC republish rule | **D5** |
| Publish before cleanup; remote before local; remote OID precondition | Git RFC | — |
| No remote: local cleanup proceeds, publish pending | Git RFC, MVP RFC | — |
| Retry only unfinished work; cache loss → recovery | Git RFC, index RFC, CLI RFC | Reconciliation table, internal ruling |
| Consent binding of refs/OIDs vs a collaborator's primary advance | Git RFC Wave 03 boundary | **D6** |
| Ignored files in the context worktree at cleanup | — | **D7** |
| Partial worktree removal | — | Recovery required (mirrors Cycle 08 D2), internal ruling |
| Fresh context after promotion | MVP RFC, PRD | `retired_contexts` written at completion, internal ruling R1 |
| Front-end binding, expiry, preview schema | F2, Wave 03 track rule 6 | Deferred to C5/D5 |

## Planning Review — 2026-10-15

An independent reviewer read the first drafts against the code and the
authorities, without the author's reasoning. It confirmed:

- git2 0.20.4 provides `RemoteCallbacks::push_negotiation` (with
  `PushUpdate::src`/`dst`) and `Worktree::prune` (with `valid`,
  `working_tree` and `locked`);
- `require_no_pending_local` checks only `operation_records`, so child
  synchronizations can reserve;
- every sampled grounding line reference is accurate.

The author verified findings 3a and 6 in the source before revising.

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | Critical | A poll between children could advance `T(P)`, giving a merge with the context tree that drops primary commits | Under the integration lease: recompute the digest; with a remote, require base ≤ context, otherwise `PrimaryAdvanced` and re-synchronize with a new attempt. Tests in Tasks 3, 6, 7 |
| 2 | Critical | Records could strand the item: pre-integration refusals, a repeated `RemoteContextChanged`, cancelled children replaying as `Interrupted` | Terminal `abandoned` step; supersession also covers `remote_context_changed` and `ignored_files`; per-attempt child IDs with an end-state table; Task 0 characterization |
| 3 | Important | Deleting the history row does not give `never_published` when the repository history flag is set; resetting early allowed silent republish; the poll claim was wrong | R1 `retired_contexts`, written only in the final transaction; `promote` reservation held from F6 to completion; claim corrected |
| 4 | Important | Cache-loss path blocked by D5; `HistoryUnknown` stricter than `plan_clean_integration` | D5 now applies only when `T(C)` is absent; `AlreadyContained` is exempt; tests added |
| 5 | Important | Reconciling integration by the exact primary tip was too strict, and its error broke the "error means unchanged" invariant | Reachability search; recovery reported as a pending category, never as an error |
| 6 | Important | Ignored files would be pruned without a check; locked worktrees not considered | `include_ignored` pre-check (new D7), `locked` unset, tests |
| 7 | Important | Partial returns skipped the refresh | Refresh on every `Ok` return after integration |
| 8 | Important | Leaving OIDs out of the digest reinterprets the RFC's "refs/OIDs" binding; a stale `T(C)` made reconfirmation certain | Raised as owner decision D6; new `ContextBehindRemote` blocker |
| 9 | Minor | Deletion outcomes did not use the RFC's recovery-required wording | Mapping stated in the design |
| 10 | Minor | Cancellation after integration had no category | `cancelled` added to both pending enums |
| 11 | Minor | Boundary lists disagreed | Single F1–F12 list in the design, referenced by Cycle and plan |
| 12 | Minor | `ExactMapping` does not exist; OID wording; own reservation in cross-check; Task 4 dependency; superseded edge case | `SshDirection::Delete`; wording fixed; cross-check accepts own row; order fixed; edge case noted |

## Owner Approval — 2026-10-15

The owner approved the Cycle, the design and the implementation plan as
revised above. The owner accepted the recommended answer to D1–D7 and ruling
R1. Implementation authority, commits, pushes, native dispatch, pull request,
merge and closure were not granted by this approval. The owner asked for a
[fresh-session handoff brief](2026-10-15-wave-02-cycle-09-handoff.md).
