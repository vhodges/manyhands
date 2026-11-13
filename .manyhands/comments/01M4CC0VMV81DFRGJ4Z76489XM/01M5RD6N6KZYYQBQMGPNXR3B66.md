---
id: 01M5RD6N6KZYYQBQMGPNXR3B66
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-25T02:12:58Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Task 0 complete for independent review

Completed 2026-10-25T02:54:18Z. All four Task 0 owner rulings below are
resolved and recorded; no owner question remains open. Task 1 is not started.

The owner authorized the additional test-local `CommandSpec` search hit at
`src/bin/manyhands-cli/catalog_tests.rs:262–267`, leaving the test unchanged.
Task 0 resumed under that ruling.

Preflight fetch/merge succeeded (already current). HEAD before and after is
`ad4d3f0d9a255125d93fe25b4e060cb7c29d2d4a`; origin/main is
`e49f978b499ee34d6b77061caaa3a6fb9d37a88b`. Merge-tree is clean. No decision-1
defect ULID matched the required main-history searches.

The four prescribed baseline gates passed through Devenv: all-feature locked
check, fmt check, all-target/all-feature locked clippy with warnings denied,
and targeted tests. Counts: **172 CLI unit + 177 CLI integration + 71 read
contract + 44 mutation contract = 464**, zero failed/ignored/filtered.
The first check invocation timed out during compilation; no Cargo process
remained before the successful retry. No baseline failure was fixed.

The initial fact audit found a contract conflict. Design Baseline Fact 19 said
the repair stopped at `AfterOwnedWriteBeforeLifecyclePersistence` is
`internal_error`, partial, exit 1. The cited test asserts `InternalError` and
`Partial` (`tests/mutation_bridges/repair.rs:663–666`). Every partial envelope
has `FailureClass::Incomplete` (`src/results.rs:273–288`), which the CLI maps
to exit **4** (`src/bin/manyhands-cli/render.rs:85–90`). The library actor
itself has no process exit.

The owner approved recording fact 19 as exit 4, keeping its `internal_error`
code and partial outcome, for C3's repair and exit evidence. The narrow
design correction is applied; no directly dependent inaccurate exit claim
was found. No behavior or test changed.

### Resumed audit, 2026-10-25T02:22:01Z

The next source audit found a contract mismatch. The design's mutation
grammar said a linked-worktree root is refused with `not_repository_root`.
Ordinary `resolve_root` does that (`src/repository/mutation/bind/mod.rs:93–103`),
but `IndexRebuild` deliberately ignores all its errors except
`RepositoryNotEnabled` (`bind/index.rs:213–233`, repeated at `278–285`). It
then calls `rebuild_repository` (`:148–151`), whose root validation accepts
the selected repository's own workdir, without rejecting linked worktrees
(`src/repository.rs:1533–1544,9570–9620`). Unavailable-index restoration uses
that same root validation (`:1647–1656`).

The owner approved documenting and passing through `index rebuild`'s existing
root-selection exception. The design grammar now qualifies the rule, marking
it source-inspected rather than fixture-proven, without promising success in
every linked-root situation. No root guard was added and no real-repository
rebuild was executed. The API refresh retains that qualification.

### Resolved finding, 2026-10-25T02:42:41Z: pending-operation fence scope

The no-op audit found a direct exception to Fact 21 and the interrupted-saves
table's blanket “every other request” claim. The source of
`tests/mutation_replay/documents.rs:573–614` stops a creation at
`BeforeIndexTransactionCommit`, asserts partial, then sends the same intended
creation under REQUEST_2 and expects `already_applied`/`noop`, no operation or
request record, and no new commit. This is source-inspected, not newly run.

`bind/document.rs:421–443` returns that no-op during binding preparation;
`mutation/execute.rs:254–258` returns it before accepting or entering a domain
operation. The binding does not check the pending operation, so an
already-applied create can also answer early during a pending save. Finished
request replay also precedes the fence (`execute.rs:190–208`,
`replay.rs:19–53`). The domain fence itself is unchanged
(`src/repository/recovery.rs:1366–1392`).

The owner approved narrowing Fact 21 and directly dependent contract/table
wording to requests reaching a new domain operation, retaining F2's early
no-op/refusal and finished-request replay. Task 3's other-request fixtures
request actual new work; interrupted-save retry expectations are unchanged.
Narrow Cycle/design/plan edits apply that ruling. No tests or behavior changed.

### Completed evidence

- All 37 facts rechecked with citations: 34 confirmed (qualifications stated
  in the ledger), fact 17 resolved, facts 19 and 21 corrected with approval.
- Fact 17: a current-token value-equal save can provision a context before
  the no-change decision. The approved no-op fixtures already have contexts.
- Initials refusal occurs before acceptance, leaving the request ID free.
- Committed-save cache-loss codes and repair refusal cases are recorded;
  source-derived document behavior is distinguished from ticket fixture
  assertions and from future C3 binary evidence.
- Thirteen existing API inventory rows updated in place, including exact
  signatures, input types, confirmation and full result-code sets.
- Exit union: **40 library codes / 42 CLI-inclusive codes**. Independent
  set expansion of the documented row expressions has no missing/extra code.
- No-op inventory: all eleven mutations, **9 capable / 2 incapable**.
- Exact E1 searches: five expected files in the first, all 57 matching lines
  classified in the second, including the owner-approved local literal.
- Baseline check/fmt/clippy and **464 tests** retained; no broad rerun for
  documentation-only changes. The tracked diff whitespace check passed.

Task 0 is ready for the controller's independent reviews, not Cycle closure.
The implementation task log remains empty as required. Only documentation
and this canonical comment changed; no Rust, Cargo, library or pre-existing
test edits. No subagent, Task 1 work, native CI or dogfooding mutation was
started. The completion commit and push are restricted to this ticket branch.

Evidence: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`.
