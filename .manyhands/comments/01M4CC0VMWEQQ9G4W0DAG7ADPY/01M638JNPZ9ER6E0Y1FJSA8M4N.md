---
manyhands_managed: true
manyhands_kind: comment
id: "01M638JNPZ9ER6E0Y1FJSA8M4N"
item_id: "01M4CC0VMWEQQ9G4W0DAG7ADPY"
parent_id: "01M62XBG3E9FFXQR2BP08C1CSM"
created_at: "2026-10-29T07:21:50Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 4a complete; the task review is pending

- **Durable state (R-14, R-35, R-41).** A new table,
  `remote_resolution_owners`, records each attempt's request and the
  digest of the input it presented. It has no foreign key, has its own
  migration step, and changes no audited table. An attempt's ID is derived
  from the request, the synchronization, the window and the stage. The
  library now finishes an attempt:
  - at `ResolutionPrepared`, from the presented sources and the live
    inspection;
  - at `CommitPrepared`/`Applied`, from its durable rows alone. Tokens are
    rebuilt for `reconcile_resolution_candidate` only, with each path
    found by hashing the candidate's tree against `path_digest`.

  A new request supersedes an attempt that has only written paths while
  they are unchanged (decision 18). The old attempt's sentinel is released
  and its rows are deleted.
- **The binding.** `conflict resolve` (`Mutation::ConflictResolve`) takes
  the steps in the design's order:
  1. syntax;
  2. replay first, where a different input under the same request ID is
     `request_mismatch`;
  3. association;
  4. validation;
  5. resolve;
  6. the continuation, which publishes under the same request: a comment's
     synchronization through the comment's retry, any other through a
     restart with the merging settings;
  7. outcome mapping, with `ConflictResolutionMutationDto` as the data.

  `conflict show` now names the request whose attempt is unfinished.
- **Resume.** It finishes a prepared commit from durable state before it
  restarts the synchronization (row 9). This is the one route Task 4b also
  names. The rest of 4b's routes are left to it.
- **Verification.**
  - 30 `c4_resolution_*` SSH cases: one per row of the interrupted-resolution
    table, Review Focus 3 (a duplicate token is `invalid_input`; a foreign
    token is `external_change`; neither writes, and no source reaches an
    envelope or a store), validation, data, and the conformance row.
  - The whole `mutation_remote` target: 200 of 201 passed. The resolution
    conformance case met the 100 s watchdog; it now builds a conflict only
    for the rules that need one, and passes in 47 s.
  - `mutation_replay` 383, `mutation_bridges` 129, `mutation_contract` 44,
    `read_contract` 72, `repository_enablement` 95 (E-12 now passes),
    `remote_merge_recovery` 25, `--lib` mutation 123, remote 477,
    local_sync 164, integration 26; clippy, fmt, and the desktop-alone
    check and clippy.
  - `origin/main` is still `a372edb`, and `merge-tree` is clean.
- **For review.** The concerns are in the report and the ledger's Task 4a
  entry: the resume route's overlap with 4b, deleting a superseded
  attempt's rows, the staging of the unfinished-local-sync row, and a host
  stop's data.
