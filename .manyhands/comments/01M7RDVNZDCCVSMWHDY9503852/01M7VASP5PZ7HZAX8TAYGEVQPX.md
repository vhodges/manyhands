---
id: 01M7VASP5PZ7HZAX8TAYGEVQPX
item_id: 01M7RDVNZDCCVSMWHDY9503852
created_at: 2026-11-20T01:58:06.980110157Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Implementation started, 2026-11-19, subagent-driven (a fresh implementer and a task reviewer per task). Owner approval of the plan, its amendments P-1–P-5 and owner questions 1–4 (all recommended answers) is recorded in the design as decisions 15–18 (`ad3527dd`). Base `ad3527dd` (main `790a71ed` merged, not rebased). Execution ledger: `docs/plans/2026-11-19-fast-writes-execution.md`.

**Tasks:** 0 preflight; 1 measurement script and baseline; Phase 1 (index) 2 migration, 3 Tables A and B tests first with the owner checkpoint, 4 activity, 5 per-context observation, 6 readers, 7 store, 8 dirty marks, 9 freshness on read, 10 a write's index step, 11 commands, 12 whole refresh, 13 relationship overlay, 14 per-item holds, 15 phase close; Phase 2 (publishing) 16 sync hand-off, 17 one connection per remote call, 18 listings and the SSH count, 19 phase close; Phase 3 (promote and close) 20 registry audits, 21 previews, 22 exits and cleanup, 23 phase close and skills; 24 review-ready.

**Acceptance → proof:** measurements before and after on this repository, release build → `scripts/measure-writes` (Task 1 baseline; Tasks 15, 19, 23); every item's `activity_at` and `activity_source` unchanged → `activity_matches_latest_commit_touching_for_every_history` (Task 4); commits or tree diffs per refresh pinned → `a_refresh_of_an_unchanged_repository_diffs_no_commits` and `a_refresh_after_k_new_primary_commits_diffs_k` (Task 12); profiles without `latest_commit_touching`, a second observation or the image-proof audit → sampled profiles in Tasks 15, 19, 23; no recovery change beyond R-3 and today's index results except outside changes seen at the next refresh → Tables A and B tests (Task 3) and the crash tests (Task 10).

**Task 0 done:** reference re-check (357 references, two drifted: `persist_rebuild_observation` at `src/repository.rs:6153`, `FailurePoint` in `src/repository.rs:931`); tests that pin the kept checks listed in the execution ledger. **Task 1 in review:** `scripts/measure-writes` (`66e320e0`) with phases `setup`, `trust-host` and `run`. Local-only baseline at `ad3527dd`, release build, scratch replica with 64 contexts, one run: ticket save 13.2 s, document save 13.3 s, ticket create 13.6 s, comment add local checkpoint 13.9 s, index refresh 13.1 s. The SSH rows wait for the owner to add the throwaway key to localhost and approve the fingerprint.