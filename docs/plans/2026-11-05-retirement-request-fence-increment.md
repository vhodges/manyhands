# Retirement Current-Request Fence — Internal Execution Notes

> Internal notes for the bounded increment under the existing approved Cycle
> contract and the owner's chosen same-session implementation/review method.
> This is not a new Cycle plan, approval gate or separate-session instruction.
> The current explicit no-agent restriction applies; the controller owns the
> subsequent limited SPEC then QUALITY reviews.

**Goal:** Connect the approved current accepted-F2/exact-operation fence to the real retirement protocol, without a Task 3 mutation binding or Task 4 runtime.

**Architecture:** A private per-call request capability is minted from `bind::Accepted` only after checking the actual request record, current accepted attempt, canonical scope, `context retire` command, item target, salted `kind` input digest and the unique ordinal-0/local operation mapping. Domain wrappers pass no request authority and continue rejecting mapped operations. Existing request columns retain historical epochs; audit accepts historical attempts no newer than the request, never treating audit success as live authority. No DTO, Command variant or CLI binding.

**Index correction:** The compiled invalidation RED proved that F2 attempt and index epoch cannot be one counter. The existing schema has no durable single-use index claim stamp. One narrow append-only `context_retirement_index_claims` table now binds each spent F2 attempt to its independently incremented index epoch; no existing column or ordinary-refresh counter changes meaning. The late additive marker is `context_retirement_index_v1`. Upgrade validates the exact published v2 catalog (independently pinned digest), old data and dependency closure before adding an empty table and two guards, atomically; published v1 remains pinned and supported. Unknown/intermediate profiles are not reinterpreted or repaired.

**Tech Stack:** Existing Rust/rusqlite, mutation request records/digests, sealed ObserverExecution, retirement and index ownership paths; serial Devenv gates.

## 1. Test the actual current request identity (test first)

- Files: new `src/repository/context_retirement/request_scope.rs`, private exports in `src/repository/mutation/mod.rs`, new registered source-included SSH cases under `tests/mutation_remote/`.
- Project canonical accepted F2 records/mappings in explicitly inert fixtures; do not claim Task 3 end-to-end coverage.
- Add private typed execution entrypoints (initially fail-closed so tests compile), then observe behavioral RED on an otherwise valid request-bound fresh journey.
- Verify malformed/missing/duplicate/foreign mappings, scope/command/kind/item/digest mismatch, finished requests, stale attempts and domain API rejection with unchanged rows/resources.
- Mint no capability from cached fields alone. Re-read exact current identity on every use.

## 2. Connect fresh ownership and effect boundaries

- Files: `authority.rs`, `run.rs`, `instance.rs`, `execution.rs`, `record.rs`.
- Thread explicit Domain/Request scope through private common paths. Bind request IDs/attempts to S/O/provenance/receipts at their real atomic creation points.
- Existing ObserverExecution service/root/S/O/configuration validation remains; add current request checks to its ownership predicate and every remote write path it already guards.
- Repeat current scope at token SQL verification, proof/intent, final W/B callbacks, effect settlement and D14 handoff. No lease survives transport.
- Test F2 advance/finish while transport is held, after observation, before W, after W/before B, and callback/index boundaries; stale error/receipt/completion/refusal writes must fail.

## 3. Connect reentry, index-only authority and cancellation

- Files: `reentry.rs`, `record.rs`, `recovery.rs`, targeted refresh hooks in `src/repository.rs`.
- Validate new caller's current F2 before any replacement; preserve old request attempts as immutable history. New S attempt carries the new request epoch, exact old O/S fenced atomically, terminal O outcomes unchanged.
- Carry a sealed request fence with claimed index ownership; check it at claim, touch/error/reclaim, every persistence transaction and finalization. Generic public refresh cannot borrow request-bound retirement identity.
- Explicit current-request M/I-only paths avoid network/proof/effects; terminal replay is read-only, never reexecution. Do not reconsume recreated materialization/context.
- Current cancellation is ownership, not request finish/fencing: before-effect stop may acknowledge with current attempt and Final/no effect; partial effect remains truthful Owed. Stale attempts cannot acknowledge cancellation.
- Test new-attempt reentry, stale reentry without fencing, exact-mapping loss, wrong-request index attempts, current M/I-only, cancellation and atomic replacement rollback.
- Before final handoff, exercise explicit index invalidation followed by the immediately next current F2 call; an invalidation tombstone must not collide with that call's single-use claim. Check the immutable operation envelope at persistence, not just the request/mapping and finalizer.
- Preserve the approved domain-only reclaim CAS: validate the envelope/handoff, then require exactly its own invalidation tombstone in the existing UPDATE. Only request-bound auto-reclaim is closed; the old domain path must not be required to still own a live epoch after explicit invalidation.

## 4. Freeze, verify, record and return for limited review

- All commands serial through Devenv, no deadlines, concurrent rebuilds or zero-case claims.
- Run focused RED/GREEN, context-retirement and registered retirement SSH; mutation records/replay/confirmation and remote ownership gates according to changed paths; migration/recovery/foundation gates as affected. Run whole-tree check/fmt/strict Clippy.
- Preserve approved 88 units/91 SSH assertions; no broad six-target SSH repetition without changed-path reason. Record exact logs/counts/source inventory and remaining E/O/D rows in the existing execution ledger.
- One coauthored coherent green milestone only if the connected contract passes; otherwise preserve owned incomplete work/evidence explicitly. No push/status/PR/main integration/native CI/agents/real-profile actions.
- New comment parent `01M6P6WWBE6YXJ9FYB6YSFV689`; unique ID via supported isolated CLI data directory. Same limited SPEC then QUALITY before another authority increment. Full Task 2/Task 8 and source/batch contracts remain owed.
