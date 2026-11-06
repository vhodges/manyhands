---
slug: vh-3azs7
id: 01M6R79XHD9MTQCBDF27JAC351
title: Lifecycle retry reports an unapplied merge after main moves
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
A close or promote stopped at `integrating`, after the closure checkpoint and before main moves, then retried after main has moved on, reports a merge that never happened.

Found during Wave 03 D5 Task 15c (ticket 01M4CC0VN2K8DBHNMZ46NTX5JJ, known limit K-26), confirmed at the library level.

## Reproduction

Local-only ticket close:

1. Commit the ticket on main, save it with a body, confirm `ticket close`.
2. Cancel the request when it reaches `ProgressStage::Integrating`. The answer is `cancelled`; the row is `step=integrating`, `integration_kind=merged`, with `integration_oid` set to a candidate merge commit that main does not contain.
3. Commit on main, so main moves past the recorded base.
4. Retry the same request.

The retry answers Partial / `recovery_required` with effects `integration: Complete` and `commit_oid` set to the unapplied candidate, and data `integration: merged`. Main's tip is the new commit, and the context tip is not an ancestor of main. The row stays at `integrating`. With main unmoved, the retry merges correctly.

The desktop shows "Merged into main" and "Not tidied up yet" with no Repair locally…; the CLI reports the same false effects.

## Root cause

- `src/repository/promotion/run.rs:765-773` records the candidate merge (`set_integration_oid`) before the cancel check at `:781-783` and before `L::apply`.
- On retry, `promote_inner` (`run.rs:588-622`) calls `reconcile_integration_for` (`promotion/integrate.rs:480-515`). With main past base it returns `RecoveryRequired`, and `run.rs:617-620` returns `Ok(recovery::<L>(…))`.
- `make_outcome` (`run.rs:1653-1713`) builds `PromotionIntegration::Merged { oid }` from `r.integration_oid()` without checking main holds it. `bind/lifecycle.rs` `reached_effects` (`:345-372`) and `reached_data` (`:532-536`) then report integration Complete.
- The same false outcome can come from the base-mismatch branch (`run.rs:756-758`) and the error-path reconcile in `run_observed` (`run.rs:360-397`).
- `parent_integrated` and `evidence::still_reachable` are correct.

## Proposed fix

At step Integrating, never return a post-integration outcome unless main contains `integration_oid`:

- If main strictly descends from `base_oid`, the candidate is stale. Reset the record to before integration (clear `integration_oid`, `integration_kind` and `base_oid`; step back to Closing or ContextSynchronized) and return `primary_advanced`, as the Closing/ContextSynchronized path does (`run.rs:733-739`). This needs an Integrating → Closing/ContextSynchronized transition in `closure/record.rs:66-89` and its promotion counterpart, allowed only while the ref is unmoved, and a check of the schema constraints.
- On any other divergence, return `recovery_required` and leave the row alone.

It touches `ticket close`, `document promote` and `operation resume` of a lifecycle parent, local-only and remote.

## Tests

- `closure/run_tests.rs` and `promotion/run_tests.rs`: cancel at Integrating, commit on main, retry. Expect `primary_advanced` and the row reset, then a clean finish with one merge.
- A `c5_lifecycle` local-only case asserting integration NotRequested and no `commit_oid`.
- Recheck `promotion/run_tests.rs` `integrating_retry_with_partial_checkout_is_pending_until_verified_baseline_repair`, which expects the current outcome shape for an unapplied candidate.
- SSH tier: `mutation_remote`, `document_promotion`, `ticket_closure`.
