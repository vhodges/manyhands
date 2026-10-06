# Cycle document review matrix

Use the rows that apply. Inspect actual project evidence before asking; omit a
row only when the Cycle cannot reach it.

| Area | Review for | Evidence or decision record |
| --- | --- | --- |
| Scope and interfaces | Exact entry points, ownership, compatibility, exclusions, and downstream work | Cycle scope/exclusions plus design interface and error contracts |
| Identity and trust | Explicit usernames/authorities, defaults, host/key identity, approval replacement, and precedence | User decision and adversarial acceptance cases |
| Secrets and prompts | Source selection, locked/rejected ambiguity, retry bounds, cancellation, caching, redaction | Prompt policy, typed failures, privacy tests |
| Persistence and recovery | Corruption, migration, durability, concurrent readers/writers, recovery approval, and rollback limits | State-transition design and preservation tests |
| Time and failure | Per-attempt versus total limits, DNS, cleanup, partial progress, cancellation, retries, and ambiguous mutation outcomes | Documented limit semantics and targeted fault tests |
| Backend and platform | Locked dependency behavior, native ABI/runtime differences, unavailable capabilities, and CI coverage | Source characterization, native matrix, explicit evidence gap |
| Fixture and tests | Whether fixtures faithfully model the boundary, readiness/teardown races, privacy of test output, and whether tests can prove claims | Fixture design, fault injection, negative tests, native execution |
| Verification | Every acceptance statement has a meaningful command/test; planned evidence is not described as complete | Implementation tasks, required local checks, native gates |
| Lifecycle | Ticket checkpoints, review-ready state, PR evidence, closure before merge, and cleanup authorization | Ticket/comment plan and final handoff |

## Classifying a concern

Ask the user when answers have a different product/security/recovery contract or
meaningful user cost. Offer the preferred answer and consequence, for example:

> Should recovery require renewed approval after lost trust state? Requiring it
> preserves prior pin strictness; accepting a known-host match reduces friction.

Record an internal ruling when the choice preserves the same contract and is
reversible. State what would force reconsideration. A concern is settled only
when the documents, implementation evidence, and prior user decision agree.

## Review record

For each material concern, record:

| Source | Concern | Classification | Resolution | Evidence or follow-up |
| --- | --- | --- | --- | --- |

Questions and rulings should link to this record rather than repeating the full
review in every document.
