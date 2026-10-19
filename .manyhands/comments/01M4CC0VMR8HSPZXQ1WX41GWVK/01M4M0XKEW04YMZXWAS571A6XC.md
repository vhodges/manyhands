---
manyhands_managed: true
manyhands_kind: comment
id: "01M4M0XKEW04YMZXWAS571A6XC"
item_id: "01M4CC0VMR8HSPZXQ1WX41GWVK"
created_at: "2026-10-10T23:03:41Z"
---

Design checkpoint after Task 7 (Part B). Work is paused here at the product
owner's request; Task 8 has not started.

**Done in Part B so far.** Task 5 (result model for mutations), Task 6
(request identity, records and the journal lookup), Task 7 in two halves
(`execute`, settling, replay, changed input; re-entry, commit evidence,
concurrency, cache loss), through `ticket create` and `ticket save`.
Commits `4c54d99..c3ef549`.

**Verification at `3cf9b07`** (last code commit), through Devenv on Linux
x86_64: the full suite with `--no-fail-fast`, 1480 passed, 0 failed. At
`c3ef549`: `cargo check`, `cargo fmt --check`, `cargo clippy -D warnings`
and the CLI smoke run, all exit 0. No workflow was dispatched.

**Review.** Every task had an independent review. Task 5, Task 6 and Task 7a
each needed one fix round; Task 7b needed two. Task 7b's review was also the
independent adversarial review this checkpoint requires: it walked ten
families of timelines through the code and found no sequence that repeats
or loses an effect, reports a clean success for a half-finished request, or
leaks content.

**Required outcomes.** All fifteen rows that a ticket create or save can
produce have a test. The rows for confirmed commands, synchronization and
keys are not built yet.

**The mechanism changed.** The design's replay rules were changed in 21
places to meet the table; the design's replay sections now describe what
was built, list each change with the sequence that forced it, and list ten
known limits. That rewrite was not independently reviewed against the code.

**To decide before the next binding.**

1. The rule that releases a record and runs the request as new when the
   earlier attempt ended before its checkpoint lives in the boundary and
   holds for authoring operations only. It must become something a binding
   opts into before `index refresh` or any confirmed command is bound.
2. Structural cleanups the reviews asked for before the machinery is
   copied: a closed command enum in place of string lookups; destructuring
   the ticket draft so a new field cannot be left out of the digest;
   moving generic effect derivation out of the ticket binding; splitting
   `mutation/mod.rs`.
3. After a crash between a rejection and the record's deletion, the same
   request ID with corrected input is `request_mismatch` until the original
   input is sent once more. The Required outcomes table says the ID can be
   used with corrected input.
4. The CLI RFC's envelope example uses the code `publication_pending`,
   which the design's code table does not list.
5. Seven statements elsewhere in the design now contradict the built
   mechanism; they are listed in the execution ledger and not yet edited.

**Open cases of the journal defect, in practice.** Case 1 (a pending row
that only its own repeat can clear) was met: a request in flight whose file
is then changed from elsewhere is `external_change` on every retry and the
repository stays blocked, until `operation abandon` (Task 12A) exists. Case
2 (a kill between a configuration write and its commit) and case 3 (a
synchronization holding its reservation) are not reachable through the two
ticket commands and were not exercised.
