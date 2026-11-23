---
slug: vh-zmagc
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M840VBMSEHPS1JJVN3KA9CWH
title: Split src/repository.rs and src/repository/remote/sync.rs
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, fast writes), from its final review and its task reviews. Line numbers are at vh-f3wts's last branch head, d239e9e1, which merges into main as it closes.

src/repository.rs is 14,570 lines (12,290 at main 790a71ed) and src/repository/remote/sync.rs is 9,037 (8,272). vh-f3wts added the index step, the dirty marks, the stored contexts and Table B's holds to these files. Split them by pure moves, with no change in behaviour.

Candidates named in its reviews:
- the index step (index_contexts, contexts_indexed and recorded_contexts_indexed; the last duplicates contexts_indexed's tail) into its own module;
- StoredContexts, StoredContext and TRANSIENT_PROBLEM;
- index_retirement, into a shared module;
- Table B's decision, into recovery/holds.rs, and the holds module split up;
- discovery's pass_over guards, and its module;
- test-only lines and cfg(test) scaffolding in repository.rs and sync.rs, into test modules;
- the large test files src/repository/closure/run_tests.rs and src/repository/promotion/run_tests.rs;
- the ~15 items of command_index_tests that whole_refresh_tests needs as pub(super), into a shared fixture module.

Acceptance: git diff -M shows moves, plus only the visibility changes and use lines the moves need. The static gates and the fast tier pass, and the SSH tier runs for each path touched, per AGENTS.md (sync.rs is under src/repository/remote/).