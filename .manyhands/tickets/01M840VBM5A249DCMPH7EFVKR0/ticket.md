---
slug: vh-xsd0t
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M840VBM5A249DCMPH7EFVKR0
title: Survey write paths whose answers to registry damage changed when reads stopped re-auditing
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, fast writes), from its review-ready test tiers and its final review. Line numbers are at vh-f3wts's last branch head, d239e9e1, which merges into main as it closes.

vh-f3wts Task 20 made registry open migrate and audit once per process (a memo keyed by the file's identity and PRAGMA schema_version), and made each read audit only the rows it uses (audit_unfinished). The review-ready tiers found two hand-damage cases whose answers moved. Each now uses an existing code, and both were accepted:

- T-1 (named edit E-84): a service that opened the registry before a local synchronization's cascade marker row was deleted keeps listing operations until its next open. On main its operation list refused. Pinned by tests/read_repository.rs repo_inspect_answers_with_a_problem_when_the_operations_cannot_be_read.
- T-3 (named edit E-85): a polling policy row deleted inside poll pause's path answers recovery_required (repo.inspect) where main answered internal_error. Pinned by tests/mutation_remote/c5_poll.rs policy_row_gone_inside_the_path.

Both were found late, in whole-target runs. Survey the other write paths for mid-path registry damage whose answer depended on the old per-read audit: the write_records callers, the mutation binds, and the poll and remote paths. List each answer that moved, and either pin it with a library test that deletes or corrupts a row through the registry, or restore the old answer.

Doc only: the fast-writes design's R-5 known limit (docs/plans/2026-11-19-fast-writes-design.md ~220) does not say what E-84 records, that a marker deleted under an open service is not re-audited until the next open. The same limit should also say that CHECK constraints are re-evaluated once per memo. Add both.

Acceptance: a ticket comment with a table of the paths checked and each one's answer at main 790a71ed and after. Every newly found change is pinned or reverted. No change to a JSON shape, result code or recovery action without an owner decision.