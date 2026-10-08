# Wave 03 F1 execution ledger

Ticket: `01M4CC0VMQ7R15A7M9SPN3KB67`
Branch: `manyhands/ticket/01M4CC0VMQ7R15A7M9SPN3KB67`
Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMQ7R15A7M9SPN3KB67`
Plan: [approved implementation plan](2026-10-07-wave-03-foundation-01-read-boundary-and-results-implementation.md)
Design: [approved design](2026-10-07-wave-03-foundation-01-read-boundary-and-results-design.md)
Cycle: [approved Cycle](../Cycles/wave-03-foundation-01-read-boundary-and-results.md)

## Authority and preflight — 2026-10-07

The product owner approved the Cycle, design and plan on 2026-10-07 and then
asked to start implementing F1 using **subagent-driven development**.
Implementation and local checkpoint commits on this branch are authorized.
Push, pull request, merge, ticket closure and worktree cleanup are not.

The plan's precondition is met: defect ticket `01M4CKWWRA1DHFPMWKPNK7CQ1G` (one
effective copy per item) was merged to main as `60b0324` and pushed.

Fresh fetch observed `origin/main` at
`60b0324f3993b32f783fcb98e0f6e24dd9f750dd`; local main matches it. The clean
ticket head rebased from `0362239f78e376f1577e1b1dd724648ec5153409` to
`6346caabfe6c6d469d2c6eba6618cc7f4648b8bf` without conflicts. Ancestry and
clean status verified. `AGENTS.md` is unchanged by the rebase. Main's untracked
`.superpowers/` and all other worktrees are untouched.

## Audit re-check against the rebased source

The design's read-surface audit cites lines at `ceb1be4`. Since then only the
defect fix changed `src/`: `discovery.rs` gained the scoped active-context
collector and `repository.rs` lost its exclusion set. Every helper the design
names still exists with the same visibility. Current locations:

| Helper | Now at |
| --- | --- |
| `open_registry_read_only` | `src/repository/discovery.rs:1200` |
| `migrate_registry` | `src/repository/discovery.rs:1209` |
| `observe_items` | `src/repository/discovery.rs:316` |
| `cache_read_guard` | `src/repository/coordination.rs:62` |
| `read_host_trust` | `src/repository/transport/trust.rs:106` |
| `bounded_public_key_contents` | `src/repository/keys/registry.rs:402` |
| `repository_snapshot` | `src/repository.rs:1165` |
| `inspect`, `list_remotes` | `src/repository.rs:2990`, `3003` |
| `persist_context` | `src/repository.rs:4973` |
| `validation_code_name` | `src/repository.rs:5070` |
| `read_repository_snapshot` | `src/repository.rs:5094` |
| `owned_file_bytes` | `src/repository.rs:5939` |
| `collect_canonical_sources` | `src/repository.rs:6813` |
| `canonical_repository_root` | `src/repository.rs:7218` |
| `resolve_identity` | `src/repository.rs:8010` |

Two design statements are now facts and need no further work in F1:

- The index holds one row per item ID. The design's "Effective Copy" section
  described the fix as proposed; it is merged.
- The defect review settled one detail the design did not state: primary
  attributes a file to an active item only by that item's ticket and comment
  directories, so an unparseable primary document remains a primary problem.
  This is consistent with the design's nonconforming-entry rule and is covered
  in Task 5.

No design change is required before coding.

## Implementation topology and ownership

All stages run sequentially in this worktree. Exactly one writer is active at a
time. Each task is implemented by a fresh subagent given a bounded brief, then
reviewed by a separate fresh, read-only subagent against the task's contract.
Code fixes go back to an implementer, not the controller. The controller owns
this ledger, ticket comments and checkpoint commits of bookkeeping.

| Lane | Contract and files while active | Gate and handoff |
| --- | --- | --- |
| Baseline | This ledger and a ticket comment only | Required Devenv gate at the rebased head |
| Task 1 | Result model and redaction: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/results.rs`, `src/results_tests.rs` | Focused unit tests, commit, independent review |
| Task 2 | Read module, DTO base, contract harness: `src/repository.rs` wiring, `src/repository/read/{mod,dto}.rs`, `tests/support/{schema,golden}.rs`, `schemas/v1/`, `tests/read_contract.rs`, `tests/read_boundary.rs`, `.gitattributes` | Focused tests, commit, independent review |
| Task 3 | Resolution and repository reads: `read/{resolve,admin}.rs`, schemas, goldens | Focused tests, commit, independent review |
| Task 4 | Credential reads: `read/credentials.rs`, `keys/registry.rs`, `keys/mod.rs` visibility | Focused and key regression tests, commit, review |
| Task 5 | Item lists and complete reads: `read/items.rs`, migration and persistence of `closed_by`, `unknown_metadata`, `refreshed_at` | Focused and discovery regression tests, commit, review |
| Task 6 | Comment reads: `read/comments.rs` | Focused tests, commit, review |
| Task 7 | Status and operation reads: `read/status.rs` | Focused and recovery regression tests, commit, review |
| Task 8 | Relationship view and index edges: `canonical.rs`, `discovery.rs`, `repository.rs` persistence | Canonical and discovery tests, commit, review |
| Task 9 | Relationship queries: `read/graph.rs`, `tests/read_relationships.rs` | Unit and integration tests, commit, review |
| Task 10 | Workflow test list, full gate, characterization, whole-branch review | Required local gate, fresh whole-branch review |

## Task state

- Baseline: passed at `6346caa` (evidence below).
- Tasks 1–10: pending.

## Decisions and rulings

None yet beyond the approved documents.

## Verification and review

Per-task evidence is recorded below as it is produced. Nothing below this line
is evidence until a task records it.

### Baseline — 2026-10-07, head `6346caa`

Through Devenv on Linux, each command exit 0:

- `cargo check --all-features --locked`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features --locked -- -D warnings`
- `cargo test --all-features --locked`: 622 passed, 0 failed across 15
  standard binaries; 15, 35, 31 and 103 SSH cases passed in the four
  custom-harness suites.
- `cargo run --locked --bin manyhands-cli`

The effective-copy precondition is covered by the nine tests the defect ticket
added to `tests/discovery_rebuild.rs`, which ran in this suite.
