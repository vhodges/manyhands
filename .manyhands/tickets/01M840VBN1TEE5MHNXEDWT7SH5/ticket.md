---
slug: vh-7cmhd
deps:
- 01M7RDVNZDCCVSMWHDY9503852
id: 01M840VBN1TEE5MHNXEDWT7SH5
title: 'Leftovers from vh-f3wts''s final review: test gaps and small fixes'
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Follow-on from vh-f3wts (01M7RDVNZDCCVSMWHDY9503852, fast writes), from its final review (2026-11-23). Line numbers are at vh-f3wts's last branch head, d239e9e1, which merges into main as it closes.

The final reviewer sorted every minor finding the run had deferred into three groups: fix before merge (done in vh-f3wts), follow-on, and drop. These are the follow-on items not filed as tickets of their own. Each was judged real and safe to leave. Fix each one, or close it with a reason in a ticket comment. Locations are approximate; find each by name.

Script (scripts/measure-writes):
- the devenv fallback swallows the first build failure and rebuilds from the main checkout; make that fatal;
- trust-host is not bound to setup's fingerprint unless --fingerprint is given.

Index and discovery:
- restore an exhaustive match where `_ => OwnItem` stands, so that a new variant fails to compile;
- a missing path's basis records only its direct parent;
- one unreadable commit time disables the primary's activity map;
- an unverified primary rewrites its activity map on every refresh;
- load_activity_map re-resolves the repository ID on a fresh connection, and runs on every refresh even when the primary is not observed;
- list_again re-lists .manyhands/worktrees every time, where the design says only a directory whose stat changed;
- an item context's history is walked twice per observation;
- repeated head lookups in the stat-only walk, and an unused _repository parameter;
- the ignore checks fail open (they read more, so this is the conservative side);
- count_document_entries without ignores can refuse folder create;
- repo list is all-or-nothing on a corrupt context kind;
- a module-wide #![allow(dead_code)] in discovery.rs hides dead code;
- presentation and problems are read from the index when the primary moved;
- LeftDirty records no retry-required problem, and resume's RetryRequired names no context;
- a gone context's rows are deleted under the lease without a re-check (a narrow race);
- the item_id-to-ContextKey mapping is written twice, and contexts_indexed reads DirtyContexts twice;
- a hold stamp is left until the process exits after a refused reuse;
- a second journal read swallows errors (benign);
- /proc/locks reports a device mismatch on btrfs and overlayfs, where the tests fail closed;
- the show/related copy split (pre-existing).

Sync and lifecycle:
- decide and pin what a debt-only row does against a restart of the same item's sync (Table B's own-item column says it is held);
- the mark, fast-forward, mark block is duplicated;
- a conflict installation (sync.rs ~3871) is unmarked, as at base;
- the local sync's window between its merge and its index step leaves its context unmarked (its row blocks everything until resumed; base is the same);
- a completed sync's replay answers current without reading its context's marks (base is the same);
- the check after BeforePush is unpinned, and the live merge-push-index path is uncounted;
- acquire_push_target makes two connections, and the push connection is held open across work that is not network work;
- back-to-back inspections, and long closures;
- the trigger is still named stop_remote_discovery;
- the closure's no-remote window;
- the registry is opened for empty children;
- the exit table covers promotion only;
- a '' refresh runs under a child's ID;
- a replay offers operation.resume and not index.refresh;
- pre-reservation reads don't wait (the same as the plain sync);
- a preview's second registry read;
- the reservation refusal carries live: null;
- an exit row left AtWork traps the request's retry;
- close_pending_refresh does not call mark_owed_by_closed_row (defensive; no debt is hidden today);
- a peer parked before its row begins is invisible to a relationship save (pre-existing on main).

Tests and coverage:
- Tables A and B: missing seeds or rows, unchecked blockers, and the find and graph reads;
- the activity test does not tell Item from Checkout;
- no load or store test with two primary rows;
- untested: the budget rule, an unended listing, removed-subdirectory rows, and the ignore rules;
- lifecycle discovery call sites have no behaviour pin beyond promotion, and no test shows that a retirement marks a stored row;
- index_step_tests.rs test 2 and strand_refresh assert only code != Ok; pin the code;
- no test drives the racy re-hash's failing branch;
- command-table rows, storage assertions and copied helpers;
- R-SDD-25's mark-off cases, and the pre-effect mark, are unpinned;
- a dropped Accepted assertion that could be kept;
- 16b's weak table rows, the owed-versus-changed nit, and no conflict-resolve or republish rows;
- restart and pre-mark coverage gaps;
- a listing after a download returns the connect-time list (no production path does this);
- SSH coverage of deletion races;
- mutation_remote test tidy-ups;
- no pin for the abandoned refusal of an older synchronization-local-v1/ row, or for older-tag collision rows in the fixture;
- refresh_allowed's audit_record is untested;
- Task 20b's untested branches, thin wrappers, injected-failure kind, and a connection dropped outside the guard;
- listed_context against a partial listing, an extra repository open, and no owed-primary step;
- no test that the full cleanup check runs under the lease;
- the older-build window of E-73 is unpinned;
- stale case names;
- an older-build test comment claims a cascade that no test covers.

Acceptance: each item fixed, or closed with a reason in a ticket comment. Tests follow AGENTS.md's Testing Strategy: library tests first, and no new SSH case without a written reason.