---
manyhands_managed: true
manyhands_kind: ticket
id: "01K7F6H9J2N4Q6S8V0X2Z4B6DG"
title: "Wave 02 Cycle 08: One-Shot Polling And Materialization"
type: "cycle"
status: "open"
project: "manyhands"
team: "core"
wave: "02"
cycle: "08"
---

Safely observe a remote once, update clean local state, and materialize newly
discovered conforming shared contexts.

## Planning

- [x] Create [Cycle](../../../docs/Cycles/wave-02-cycle-08-one-shot-polling-and-materialization.md).
- [x] Create [design](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-design.md)
  and [detailed implementation plan](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-implementation.md).
- [x] Record planning/rebase and skill-based self-review checkpoints as comments.
- [x] Owner answers to the four decisions in the Cycle document, all as
  recommended (2026-10-11): keep stale tracking refs; resume only a
  branch-only materialization; refuse to poll during a pending conflict; a
  local action during an automatic poll requests yield and gets `RepositoryBusy`.
- [x] User approval of Cycle, design, and implementation plan (2026-10-11).
- [x] Explicit implementation authorization (2026-10-11): Tasks 0–8, rebase,
  local commits and canonical ticket/comment/ledger updates; execute Task 0
  only in this session, then return for independent spec and quality reviews.
- [x] Owner resume authorization conveyed by controller (2026-10-13): remaining
  execution resumed; this implementer assigned Task 1 only, inline, no subagents,
  local task commit and bookkeeping, then independent spec and quality review.
- [x] Fresh Task 2 assignment (2026-10-13): Task 2 only in this ticket worktree,
  no subagents, one local task commit and canonical bookkeeping, then return
  for the controller's independent reviews. Resume rebase evidence trusted.

The owner approved the planning documents and D1–D4 on 2026-10-11, then
separately authorized implementation and local commits. The
[execution ledger](../../../docs/plans/2026-10-11-wave-02-cycle-08-one-shot-polling-and-materialization-execution.md)
records the preflight, the decision matrix, the internal rulings and Task 0
execution evidence.

## Entry Gate

Wave 02 implementation is blocked until every Wave 02 entry-gate prerequisite
is approved and Wave 01 verification evidence is current. Cycles 01–07 are
present on the planning base `d1ca887`. Implementation Task 0 repeats the
rebase, records the decisions, runs the characterizations named in the plan
and the baseline.

## Implementation Progress

- [x] Task 0: fresh fetch/rebase and ancestry/clean-status checks; authorization
  and D1–D4 recorded; caller compatibility re-scan; all five throwaway
  characterizations executed and removed; all five Devenv baseline commands
  passed. No design contradiction or blocker found.
- [x] Independent Task 0 spec review (2026-10-11):
  `ses_ed4470e17ffeiE6Bsc6av876RR`; spec compliant, no blockers.
- [x] Independent Task 0 quality review (2026-10-11):
  `ses_ed443ab4effebJy3h12aBe4kYO`; approved, zero Critical/Important/Minor issues.
- [x] Task 1: nullable UTC attempt/deadline migration and accessors, pure policy
  and retry-deadline functions, transactional persisted rollback clamp, existing
  status deadline field and privacy scan. Red recorded before behavior; focused
  tests and all four required Rust checks passed (1,581 tests, 1 existing ignored).
- [x] Independent Task 1 spec review: `ses_eca3c077effeAOiqWMn8w5xeR5`, compliant.
- [x] Independent Task 1 quality review: `ses_eca397d08ffeZI2d2YhlbUJ99h`, approved;
  minor direct corrupt timestamp read test carried into Task 2.
- [x] Task 2: poll-kind reservation/local stage, atomic timestamps, D3/D4 gates;
  prior minor direct corrupt timestamp read coverage resolved. Recorded RED then
  focused GREEN and all four Rust checks (1,594 passed, 1 existing ignored).
- [x] Independent Task 2 spec review: `ses_ec9ebcd99ffekVz7ijwoL63NJx`, compliant.
- [x] Independent Task 2 quality review: `ses_ec9e9909cffeS4kt0Z0ENmyjss`, approved;
  minor suggested direct-SQL kind immutability regression carried to final review.
- [x] Task 3: exact parsed poll fetch set, guarded tracking refs and sticky stop
  hook; RED and corrected stop-fixture behavioral RED recorded. GREEN: 31 focused
  library, 9 poll SSH, 114 entire transport cases; four Rust checks passed
  (1,606 tests, 0 failures, 1 existing ignored). Combined frozen/automatic scope.
- [x] Independent Task 3 spec review: `ses_ec9b35ee6ffeBdb0HkkfCCp9R4`, passed.
- [x] Independent Task 3 quality review: `ses_ec9b0a1d9ffeWP162PBxjUK6l4`,
  controller reports cumulative approval after atomic-absence correction through
  `34b97c178a48b39b6ae32e30de52ff5722d1dd06`; prior Important resolved.
- [x] Task 4: delivered after owner-approved amendment (2026-10-13). Historical
  discovery-agreement RED: 8 passed, 1 failed, 523 filtered. Existing discovery
  accepts a valid executable document tree that poll must reject; two-way
  agreement conflicted with visibility-only changes. Historical probe removed;
  Rust was unchanged at that blocked checkpoint.
  Independent read-only review `ses_ec92a184effe7ndQKcvXtxKxkg` CONFIRMED the
  contradiction. Owner subsequently approved accepted⇒active-only agreement,
  retaining executable rejection and unchanged discovery. Approved documents
  updated before production; resume comment `01M4VE4298RCK58VXV4YT3M6P8`.
  Classification, fixed guidance, guarded clean fast-forward and tree conformity
  complete. RED recorded before behavior; final GREEN 23 poll tests and four
  Rust gates (1,623 passed, 1 existing ignored). Shared helper bodies unchanged.
- [x] Independent Task 4 spec review: `ses_ec8bd9b3effeS3OD0g7achFQNy` passed
  through `6daaaec520aeb37e777185cc28b8576e51b43383` after correcting its P1:
  ignored descendants of a tracked directory replaced by a file/symlink escape
  preflight, causing partial checkout. Poll-only correction authorized; meaningful
  snapshot RED recorded before production in comment `01M4VMJE8B5XZAF6KRT7YYRSBY`.
  Correction now implemented and verified: 20 obstruction cases/4 clean controls,
  25 poll tests, 322 remote tests, four Rust gates (1,625 passed, 1 ignored).
  Approval reported by controller; comment `01M4VPNGXTADT2ASHG97ZQH66Q` retains
  correction evidence.
- [x] Independent Task 4 spec re-review after quality fixes:
  `ses_ec8bd9b3effeS3OD0g7achFQNy`, controller reports approved through
  `8cb5d777758287b200e09255dc1fa7f254fc4da3`, cumulative range from
  `961b3c21db11f797e5bd1cdac0bf85d748cd1dec`.
- [x] Independent Task 4 quality review: `ses_ec892692effePXdmvv7I1Ys7ER`,
  controller reports approved through `8cb5d777758287b200e09255dc1fa7f254fc4da3`
  after fixing two Important findings: foreign context ownership and docs/.md
  file selection. Poll-only fixes implemented and verified; RED for each recorded in
  comment `01M4VQFGNCG4TE2MMA6WD27C3Z`. GREEN: 29 poll/326 remote tests and
  required Rust gates (1,629 passed, 1 ignored). Historical review-ready comment
  `01M4VSWK7DJCKF0FWEA82VZC6Q`; approved agreement unchanged.
- [x] Task 5: delivered after owner's narrow exception. Originally assigned at
  clean base `8cb5d777758287b200e09255dc1fa7f254fc4da3`;
  historical blocker before production/test edits. Required unchanged
  `validate_context_worktree` in step 5 unconditionally scans canonical files,
  conflicting with no scan under the single lease. Historical blocker evidence
  and prior review approvals in comment `01M4VTCVFT1CJHJ0YQ3WC77KTR` and ledger.
  Owner subsequently answered "yes": ONLY step 5's unchanged validator canonical
  collection may run under the SAME single lease before complete; all discovery/
  index-refresh scans, network and prompts outside. D1–D4 unchanged. Approved
  documents reconciled before production. Resumed at review base
  `ab07d0d3096a0e6129add1ba32a2f977a131b80a`, controller fetch/rebase a clean
  no-op at main `d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`.
  Resume comment `01M4VWTXC6JGWH7CXZQ5XPRHF8`; RED before behavior in
  `01M4VX5NM6TMJ35B0048FWBBN7`. Additive audited intent envelope, one-lease
  materialization, Git-first conservative D2 reconciliation and public failure
  boundaries complete. Final focused GREEN 40 poll/50 state/78 enablement;
  four required Rust gates PASS (1,644 passed, 0 failed, 1 existing ignored).
  Actual Linux failed checkout retains all partial artifacts; helper bodies,
  dependencies and frontend/versioned schemas unchanged. GREEN/review-ready
  comment `01M4W0G54ZF0M98PXS90NJFEJH`.
- [x] Independent Task 5 spec review of complete range from
  `ab07d0d3096a0e6129add1ba32a2f977a131b80a`, including approved exception.
  Reviewer `ses_ec7f49995ffeVTGZBqRN4rzEoX`: BLOCKED P2, unfinished but fully
  valid owned worktree incorrectly requires clean/original tip before Present.
  Fix authorized at `4dc35de84fab433a299c4de8c640ad5647ceb15c`; RED before
  predicate correction: 1 passed/3 failed/567 filtered, comment
  `01M4W126TJWXJJCPW4FMSJD99K`. Historical blocker, subsequently corrected.
  P2 technically corrected: fully valid owned worktree finishes unchanged final
  validator and complete without cleanliness/original-tip gate; original intent
  owner/OID and Git preserved. Strict branch-only resume unchanged. GREEN:
  4 regressions/44 poll tests; four Rust gates PASS (1,648 passed, 1 existing
  ignored). Re-review comment `01M4W3AQQGAF0YJ28908ABW64Q`; no amendment.
  Controller/user now reports `ses_ec7f49995ffeVTGZBqRN4rzEoX` passed the
  corrected range through `ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`.
- [x] Independent Task 5 quality review after spec approval:
  `ses_ec7c4bbc3ffensuQh7z1f1Gqul`, controller/user reports APPROVE, no findings.
- [x] Task 6: assigned at exact clean base
  `ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`; stopped before Rust/test edits
  for an atomic-batch interface/ownership ruling. The required
  `commit_observation_batch` opens and commits its own transaction without a
  disposition payload/hook. Initial ordered NotReached rows must commit with
  that batch; a second transaction violates the contract. Narrow proposal:
  authorize a reservation.rs transaction-composition extension while preserving
  the existing helper and observation/synchronization behavior. Not approved;
  approved documents unchanged. Evidence and requested ruling in ledger/comment
  `01M4W44NN8N2BRMZBC1H4AG4HN`. Historical entry issue now resolved by controller:
  file list is a planning map; necessary minimal shared transaction extraction is
  within approved Task 6 scope, no owner amendment. Implementation resumed at
  `a70a2abcebb4c84694629bb7bd79e048ea1c83c1`, review base remains original ff43d2c.
  Before behavior: poll RED 44 passed/5 failed/529 filtered, state RED
  50 passed/2 failed/526 filtered. Details/comment `01M4W4GZ1NQT9JYSJP1YM2ZEJQ`.
  Delivered public one-shot poll, frozen SSH fetch composition, atomic initial
  dispositions, ordered same-lease local stages, typed partial reports and fenced
  replay/restart, fresh-ID index refresh and durable stale flag, audited metadata
  table/replay times/snapshot accessors, API audit. Added 27 library tests.
  Final GREEN: 66 poll/54 state/598 library; all four required Rust gates PASS,
  full total 1,675 passed, 0 failed, 1 existing ignored. CLI smoke exit 0.
  Incremental meaningful RED/corrections/trials and exact commands in ledger;
  review-ready comment `01M4WAV9ECDNZ8NFSP7CGSXK85`. Complete review range starts
  original ff43d2c and includes a70a2ab docs checkpoint. Approved contracts unchanged.
- [x] Independent Task 6 spec review of full range from
  `ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7` through returned feature HEAD.
  `ses_ec74e7b1cffe52NiF8TdyiLNkR` BLOCKED four issues: automatic restart bypasses
  all eligibility gates; terminal replay consults current config before stored ID;
  original result loses targets when another poll replaces the set after release;
  separately probed schema sub-envelopes recreate identifiable partial loss.
  Corrections authorized at 72a6260; meaningful RED for R1/R2/R4 recorded before
  fixes, R3 fixture plumbing precedes actual race RED. Details in ledger/comment
  `01M4WBV4WAGBZ4GEWTE583GF8X`; quality not started.
  All four technically corrected, awaiting SPEC re-review: all automatic restart
  gates; stored-ID-first config-independent terminal replay; transaction-captured
  original reports and stop handoff state; one all-or-nothing schema envelope with
  no modern-time fallback. Added 15 meaningful regression/expanded library tests.
  Each finding has recorded behavioral RED before correction; additional R3 foreign
  fence/index-repair RED also corrected. Final GREEN 76 poll/59 state/40 reservation,
  613 library; four required Rust gates PASS, full total **1,690 passed, 0 failed,
  1 existing ignored**. Re-review comment `01M4WJP4VF6W2FPCZ7ZHM9229A` and dated
  ledger retain commands/counts/trials. Fix base 72a6260; complete base ff43d2c.
  Return to `ses_ec74e7b1cffe52NiF8TdyiLNkR` before any quality review.
- [x] Controller/user reports Task 6 SPEC re-review APPROVE:
  `ses_ec74e7b1cffe52NiF8TdyiLNkR`, through `4d89295070715d6a15ced2c08d72aff0dd0ee46f`.
- [x] Independent Task 6 quality review after spec approval:
  `ses_ec6bed51effeNWgnoaPfQ9pdlV`, APPROVE, one nonblocking minor:
  duplicate disposition attachment in state.rs (5208–5219/5256–5267); the first
  audit return needs attachment, normal return currently performs two passes.
  Carry explicitly to final review alongside Task 2 SQL-kind regression minor.
- [x] Task 7: real-SSH acceptance target, authorized inline at exact base
  `4d89295070715d6a15ced2c08d72aff0dd0ee46f`; no subagents. **68 new SSH cases**
  pass, including byte-barrier in-transfer yield, real divergence D3, D4 save,
  conservative D2 partial checkout, stops/restarts and whole-store/output privacy.
  Fixture-only optional short paths/transfer hold/connection counter; public API,
  no production changes. Five dedicated targets pass **68/15/63/25/41**.
  Required sequential Rust gates PASS: **1,758 passed, 0 failed, 1 existing ignored**.
  No invented behavioral RED; fixture failures and outer-chain timeout recorded
  honestly in dated ledger. Manual-only native workflow wired, not dispatched.
  Review-ready comment `01M4WTNE2QC4091RCJ9KM6YYNM`; complete range starts above.
- [ ] Independent Task 7 spec review, then quality review.

Task 0 is completed and independently reviewed. The owner resumed execution on
2026-10-13; the controller's fresh fetch/rebase was a clean no-op at main
`d1ca88772c5768d55aadb2ca4dcac1c69dab0d17`, before/after ticket HEAD
`4c1590d73b9dbab17030670989bd8825328f1f55`. Task 1 is implemented and independently
reviewed. Task 2 is implemented and independently reviewed. Fresh Task 3 assignment
authorizes Task 3 only, inline, no subagents, one local commit and bookkeeping.
Controller reports Task 3 spec and quality approval of the cumulative corrected
range. Fresh implementer assigned Task 4 only, no subagents. Task 4 stopped at
confirmed discovery-agreement contradiction before production work; the
historical blocker is recorded in comment `01M4VD7G36JQ3T0D3VXEX7YDTQ`. Owner
approved the focused agreement amendment; Task 4 resumed at clean review base
`961b3c21db11f797e5bd1cdac0bf85d748cd1dec`, controller preflight a clean no-op.
Task 4 spec and quality are now controller-reported approved after all corrections,
through `8cb5d777758287b200e09255dc1fa7f254fc4da3`. Fresh Task 5-only assignment
permits a local task commit and canonical bookkeeping, no subagents/network/rebase.
Task 5 is delivered after the owner's narrow step-5 exception; historical blocked
checkpoints remain separate. Review range starts `ab07d0d3096a0e6129add1ba32a2f977a131b80a`
and includes approved-document reconciliation plus Task 5 implementation.
Spec P2 corrected; Task 5 spec and quality now controller/user-reported approved
through `ff43d2c3873e4c9cbd9f9f579c5cdd0ef4c513a7`. Fresh Task 6-only assignment
permits local commits and canonical bookkeeping, with no subagents. Task 6 entry
inspection found the atomic-batch interface/ownership blocker recorded above;
controller resolved it as an implementation detail within existing authorization.
Task 6 delivered locally after recorded RED, self-review regressions and final
verification, ready for independent spec then quality review. Prior Task 0/preflight
trusted; no repeated characterization. Complete range includes a70a2ab.
Task 6 initial spec blocked four findings; corrective implementation and final
verification now complete; controller/user reports **SPEC and quality APPROVE**.
Deferred Task 2 direct-SQL kind-immutability and Task 6 duplicate disposition
attachment minors remain for final review. Task 7 delivered locally for independent
spec then quality review; Task 8 unstarted.
Ticket remains
open; native matrix evidence remains pending separate authorization.

## Exit Evidence

Real remote tests prove clean updates and new-context materialization occur
once, while dirty, divergent, conflicted, malformed, inaccessible,
unrecognized, renamed, and remotely deleted contexts remain unchanged and
actionable.
