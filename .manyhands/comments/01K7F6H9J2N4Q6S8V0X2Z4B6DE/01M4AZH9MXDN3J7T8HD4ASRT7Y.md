---
manyhands_managed: true
manyhands_kind: comment
id: "01M4AZH9MXDN3J7T8HD4ASRT7Y"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DE"
created_at: "2026-10-07T11:12:51Z"
---

Plan-ready for owner review; all three artifacts remain proposed:

- [Cycle](../../../docs/Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md)
- [Design](../../../docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md)
- [Detailed implementation plan](../../../docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md)
- [Review/ruling ledger](../../../docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-execution.md)

Completed `review-cycle-docs` self-review against the approved Wave/RFCs, actual
merged Cycle 05 APIs/record fields, owned-path and identity helpers, prior
comments/evidence and manual-only CI. Checked scope/interfaces, identity/trust,
privacy, persistence/recovery, cancellation/time, backend/platform, fixture
fidelity, verification and lifecycle. Re-read the artifacts after corrections.

One material scope question arose: partial canonical resolution within mixed
conflicts versus whole-merge external repair. Owner selected **Whole merge
external**: no canonical writes to mixed canonical/code/binary/unsupported
sets; external tools resolve/commit everything, then deliberate observed resume.
Explicit accepted cost: those canonical portions cannot be resolved in-process.
Recorded in all three artifacts and ledger. This scoped answer does not approve
the whole Cycle, design or plan. No material question remains open.

Internal rulings: dedicated two-parent resolution candidate; ordered stages in
one existing envelope; attempt/per-path digests for partial write replay; fixed
primary subjects; strict recorded-parent proof for external repair; auxiliary
identity-confirmation IDs/observations; one current-ref integration pass and
one push attempt per invocation. Ledger records sources, costs and A1–A9 proofs.

Backend self-review found a concrete feasibility trap: libgit2 v1.9.7 content
merge writes ODB blobs despite returning an in-memory index. Revised all three
artifacts to require a separate preparation-only transient mempack ODB handle,
verify no destination writes/other-handle effects, then import result objects
under the common-Git lease. Locked git2 0.20.4 exposes the backend API; actual
characterization remains Task 1, not claimed proof. Added foreign merge/rebase/
cherry-pick state checks rather than trusting a clean-looking index.

Static checks passed for frontmatter/ULID shape and uniqueness, local links/
anchors, fenced blocks, trailing whitespace and tracked `git diff --check`.
Main's unrelated `.superpowers/` and `devenv.nix~` remain unchanged. No Rust
source, Cargo/Devenv dependency, CLI/desktop or CI configuration was changed;
no Rust tests or native verification run during this planning phase.

Next: owner artifact approval, then explicit implementation authorization and
execution method. Proposed sequential direct Tasks 0–7 in this existing
worktree: fresh rebase/baseline; backend contracts; persistence; merges;
canonical resolution; replay/external repair; real SSH/privacy evidence;
final required local checks/code review. Native evidence requires separate
execution authority or explicit Cycle-specific deferral, not Cycle 05's waiver.
Ticket stays open; push/PR, CI dispatch, pre-merge closure after code-review/PR
approval, merge and cleanup all retain their separate authorization boundaries.
