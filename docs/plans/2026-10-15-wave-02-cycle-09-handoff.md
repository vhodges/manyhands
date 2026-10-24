---
title: "Wave 02 Cycle 09: Fresh-Session Handoff"
date: 2026-10-15
author: "Vince Hodges <vince@imbas.ca>"
manyhands_managed: true
manyhands_kind: document
id: "01M5MHD4J8VFNYSP9HFZ3C1V5S"
---

# Wave 02 Cycle 09: Fresh-Session Handoff

## Closure And Main Merge Authorized — 2026-10-17

Owner approved finishing Cycle 09 and merging it into main, then running the
full restored Build workflow on main. This supersedes the earlier dispatch hold.
Ticket closure is recorded in the final pre-merge checkpoint with owner identity
and UTC timestamp. Independent reviews are approved; native-correction focused
runs are green. Final full-native verification remains a **post-merge** check,
not a passing pre-merge claim. Preserve the ticket branch/worktree and unrelated
main-checkout `.superpowers/`; cleanup was not requested.

## Current Native Status — Full Workflow Restored; Final Dispatch Held

**2026-10-17 16:19 UTC:** Native corrections are reviewed and pushed. The final
Windows-only verification passed. Owner then selected **Restore without
dispatch**: the full four-platform matrix and complete configured test list are
restored and pushed as `6181930`, byte-for-byte equal to the pre-narrowing workflow
at `923cac8`. **Do not dispatch the final full run without new authorization.**

| Run | Scope / result |
| --- | --- |
| [39023714430](https://github.com/vhodges/manyhands/actions/runs/39023714430) | Initial full run: Linux x86_64 passed; Windows 20 snapshot cases, Linux ARM one concurrent-save case, macOS one F11 case failed. |
| [39036192892](https://github.com/vhodges/manyhands/actions/runs/39036192892) | Focused original failures at `35d4ef7`: all three affected platforms passed, 20 Windows + one ARM + one macOS case. |
| [39036630383](https://github.com/vhodges/manyhands/actions/runs/39036630383) | Restored full run at `310d5ee`: both Linux targets and macOS passed. Windows 956 unit tests passed; only the newly added lease-snapshot regression failed. Windows integration targets passed. |
| [39048549766](https://github.com/vhodges/manyhands/actions/runs/39048549766) | Windows-only regression at `213c04b`: success after the fixture-path spelling correction. |

Corrections preserve exact snapshot/lease assertions, recover concurrent save
commit evidence conservatively, and avoid deadlocking the F11 fixture's cleanup
when an expected hook is not reached. The original macOS initiating cause was not
reproduced; both subsequent native runs of that case passed, including the full
run. Windows' last correction changes only the regression fixture, not production
locking. Independent spec/quality reviews accepted these changes; only affected
local tests ran through Devenv.

Ticket remains open. Final full-native green for the latest code is **not yet
established**. Earlier publication/native-pending descriptions below are history;
the table and owner dispatch hold above are current.

## Publication Update — Native Build Dispatched 2026-10-17

The owner authorized pushing this branch and one Build dispatch, then explicitly
approved history-only reconciliation of the equivalent published checkpoint.
`5161f1d8882bb30204dba18576469a200a9b4693` preserves remote checkpoint
`f017c738161b8acc7b54b88055471cf7ae551765` without changing the verified file tree:
`e5ad7abdb9475b6ecc4008f58318dae29cd97b8e`. The normal push succeeded without force.

Native Build run [39023714430](https://github.com/vhodges/manyhands/actions/runs/39023714430)
is dispatched against that exact merge SHA. Linux x86_64, Linux aarch64, Windows
x86_64 and macOS aarch64 jobs were confirmed queued/running. No result is claimed
yet. This publication-record checkpoint changes documentation/comments only; it
does not change the implementation under test or dispatch another run.

Ticket remains open. A subsequent native dispatch, PR, merge, closure and worktree
cleanup still require their applicable owner authorizations. Historical statements
below that publication/native authorization is pending are superseded by this update.

## Current: Tasks 0–8 Reviewed, Verified And Committed — Owner Review Next

**2026-10-17 UTC: DONE at the local implementation/review/verification gate.**
Controller reports whole-branch **SPEC PASS**, then the same final QUALITY reviewer
**APPROVE, no findings**, after two IMPORTANT authority/cancellation corrections.
The five new regressions had targeted RED/GREEN and promotion 193/0 evidence;
they now also pass in the fresh complete gate below. No review redispatch needed
or performed by this worker. Ticket remains **OPEN awaiting owner review** and
applicable native/outward authorization.

Required exact Devenv check, fmt check, all-target/all-feature Clippy with warnings
denied, full all-feature test and CLI no-argument smoke all pass. **3,054 passed /
0 failed**: **1,000 library + 1,496 standard integration + 547 SSH + 11 doctests**.
One pre-existing opt-in performance characterization remains ignored; no failed
test was waived. Full suite ran once this verification pass, with sufficient outer
allowance and unchanged SSH watchdog. Prior 3,049 gate is historical pre-correction.
One new Clippy closure-call lint was narrowly refactored before running all gates;
reviewed authority/cancel behavior and test fault hook are unchanged.

Final verified code/test/manifest/toolchain/workflow diff fingerprint (relative to
main): **2549bc99929448e16b837045fcbc2e40127ce09a**. Reproduction command and scope,
all counts/commands/hashes and the 18-row named acceptance mapping plus five new
quality cases are durable in the ledger's **Task 8 Final Post-Review Full Gate**
section. The source fingerprint matched before/after the run; only evidence edits
followed. Both lockfile SHA-256 values match before/after.

The controller committed the final reviewed/verified implementation as
**62e8e207aa31b72ec87e27dac8699378d0d7c70f** (`62e8e20`). A subsequent fresh
`git fetch origin main` still observed
**89d04f901636229392a311241f8e777f0a00962b**. On the committed final implementation,
`git merge-tree --write-tree HEAD origin/main` passed with tree
**7077b29a643f785f8577f351dbc90318d218c4ec**, and main ancestry passed. Status
was clean. The following checkpoint records only this evidence/status update;
it changes no verified source, tests or workflow.

**Next: owner review and separately authorized native/outward process.**
Four-target manual native execution
remains pending authorization. Earlier real Wayland launch/surface smoke remains
valid for the unchanged GUI; required CLI smoke was rerun. No native dispatch,
push, PR, merge, closure or cleanup is authorized/performed here.

All 17 final implementation/evidence paths are committed in `62e8e20`.
Comments `01M5227Z00N4Q6S8V0X2Z4B6EK` (historical first gate) and
`01M5237Z00N4Q6S8V0X2Z4B6EM` (review-fix handoff) are preserved; latest comment
`01M5247Z00N4Q6S8V0X2Z4B6EN` records final reviews/fresh gate; comment
`01M5257Z00N4Q6S8V0X2Z4B6EP` records the committed integration proof. Approved T0/T2/T3/
T4/T6/T7 bounds remain intact. Published ticket-ref reconciliation is still due
before future authorized push; no force push implied. Main .superpowers/ untouched.

## Historical Task 8 Local Verification — Before Final Quality Corrections

**2026-10-17 UTC: READY for controller-dispatched final spec and whole-branch
quality review. Final approval remains pending; ticket stays open.** Worktree
and branch are unchanged, HEAD **504cb44aa7f8e1eebd5996666851ebe34c47f78d**
(reviewed/committed Tasks 0–7). Task 8 changes are unstaged/uncommitted.

The explicitly authorized saved full gate is now executed. Required Devenv
check/fmt/all-target Clippy (`-D warnings`)/all-feature full tests and no-argument
CLI smoke pass. Final full run: **3,049 passed / 0 failed**, including **995 library,
1,496 standard integration, 547 SSH and 11 doctests**. One existing opt-in read
performance characterization is ignored; no failed test was disabled or waived.
Fresh SSH counts: promotion **74**, synchronization **63**, merge recovery **25**,
polling **84**, comment publication **41**, transport **122**, fixture **35**,
observation **15**, mutation remote **88**. This run includes all late Task 7
corrections; the old focused-only qualification below is historical.

Task 8 resolved the carried lints (fresh probe: 28 diagnostics) via a boxed new
promotion transport payload, private already-discarded error-detail reduction,
borrow/control-flow refactors and a test tuple alias. Full runs then exposed
three stale assertions: shared migration-table ownership across separately
committed transactions, and two public Push-history projection expectations.
Focused RED/GREEN and final full GREEN are durable in the ledger; original
rollback, unknown-history, deletion/preservation and reopen controls are retained
and supplemented. No new product policy/dependency, blanket lint allow or new
test case. The first full attempt exceeded its outer 30-minute allowance;
later runs used an adequate outer allowance with the per-case watchdog unchanged.

Both lockfile SHA-256 hashes are unchanged (see ledger). Fresh fetch still sees
**origin/main 89d04f901636229392a311241f8e777f0a00962b**. No mid-task rebase.
Read-only merge-tree returns **9fa0a194c03bd43640b82b9bf0bfa59f67a18d7e**, exit 0.
Desktop smoke used the available active display: Wayland toplevel/configure,
buffer attach/surface commit, then deliberate bounded timeout (124). No human
visual/button-interaction claim. No smoke process remains after termination.
**Native execution still pending authorization**: actual four-target manual
matrix, including promotion wiring; no Windows/macOS runtime proof is inferred.

**Controller review scope:** the entire **`origin/main..HEAD` plus `git diff`**,
not `HEAD~1`. Reuse Task 0–7 component approvals, but perform fresh independent
final spec and whole-branch quality reviews including all Task 8 corrections.
The ledger maps all 18 Cycle acceptance rows to named tests in this final run.
Keep T0 ownership-first, T2 mode/type binding, T3 RFC-local identity persistence,
T4 Git-first/SQL-atomic split, T6 quiesced operator-repair exception and T7
retained-only unknown-history R1/accepted discovery-title bounds intact.

Task 8 change boundary: eight Rust lint-refactor files, two existing integration
test files, Cycle/ledger/plan/handoff and canonical comment
`01M5227Z00N4Q6S8V0X2Z4B6EK`. Controller owns checkpointing **after** independent
reviews. No subagents, commit/push/PR/native dispatch/merge/closure/cleanup by this
worker. Published ticket ref reconciliation remains due before any future
authorized push; no force-push authorization exists. Main `.superpowers/` untouched.

## Historical Task 7 Reviewed Complete At The Local Gate (2026-10-17 UTC)

The controller reports **Task 7 SPEC PASS and QUALITY FINAL APPROVE, no remaining
findings**, including the final `index_invalidated` public-resume correction.
These approvals cover the entire uncommitted Task 7 change against exact base
**c0e972655f04ea67bb9411877b0627e7eb8439ac**, not merely the last correction.
Task 7 is delivered and reviewed complete at the local gate. No further Task 7
review or owner decision remains pending. Earlier pending-review states below
are historical and superseded by this controller-reported approval.

**Next: controller checkpoint of Task 7, then Task 8.** HEAD remains c0e9726;
the worker has made no checkpoint commit. Task 8 must fix the **22 carried
Clippy findings** and run the required full Rust/CLI verification, whole-branch
review and authorized native evidence process. Those gates are still pending;
Task 7 approval does not establish full/native readiness or Cycle completion.
The ticket remains open.

Retained local evidence: **74 real-SSH promotion cases**; affected targets
**63 synchronization / 25 merge recovery / 84 polling / 41 comment publication**;
targeted promotion library **188**. These full-target results precede the final
narrow resume correction. The final correction was verified with **33 public
resume / 12 recovery / 3 actual SSH F11 / 1 envelope-schema** tests, not a fresh
full-target rerun. The recovery fixtures seed the durable fence and reopen;
genuine fence creation is separately covered by the F11 races. Unix-only image
coverage remains qualified for other platforms. Exact evidence is in the ledger.

This approval-recording pass edits only ledger/handoff/plan/Cycle evidence and
canonical comment `01M5217Z00N4Q6S8V0X2Z4B6EJ`. No source/test/wiring changes or
Cargo/test reruns. All **31 pending paths** are unstaged/uncommitted; the complete
inventory is in the ledger and controller return. Production diff witness remains
`5f4704976c6dcae4fad323727042b3329acafc5c`.

## Historical Invalidated Local Handoff Recovery Correction (review now approved)

The controller's latest QUALITY feedback accepts R1, the image minor and the F11
mechanism, but identified a remaining recovery-classification integration gap.
`index_invalidated` is now in `HAND_OFF_STATES`, still gated by the existing
authoritative-step proof in `LocalRow::owes_only_its_hand_off`. Public
`operation.resume` can complete that local handoff after executor loss/reopen,
with or without an original request record. It neither repeats the write/commit
nor leaves subsequent work blocked. No-step/non-authoritative states remain
refused. Promotion remains NotResumable even with an authoritative invalidated
local shadow; no F2 promotion binding was added.

Targeted evidence: **33** public resume tests, **12** recovery tests, **3** actual
SSH F11 races, and the **1** existing resume envelope/schema golden test passed.
The owned and unowned regressions ran RED against the old classifier. Recovery
tests explicitly seed the durable invalidation fence and reopen; they do not
claim a new process-kill journey. Existing real F11 races prove fence creation.
Exact commands, fixture correction and proof boundaries are appended to the ledger.

All changes remain unstaged/uncommitted at c0e9726, ready for QUALITY recheck;
Task 7 approval/completion and Task 8/full/native gates remain pending. Historical
74-case/four-affected-target results below were not rerun as full targets in this
narrow correction. Latest production diff witness:
`5f4704976c6dcae4fad323727042b3329acafc5c`.

## Historical QUALITY2 handoff (superseded by recovery correction above)

SPEC PASS was conveyed on the preceding Task 7 revision. Both important QUALITY2
findings and its minor are corrected, targeted-verified and uncommitted against
c0e9726; **QUALITY approval/Task 7 completion remains pending**. Latest full target:
**74 SSH cases**, including all prior SPEC3-P2 controls. The changed production
paths justify fresh affected-target runs: synchronization **63**, merge recovery
**25**, polling **84**, comment publication **41**, all passed. Promotion library
**188**, state **67**, reservation **45**, focused discovery ownership/failure/lease
and schema inventory checks passed. Exact RED/GREEN is in the ledger.

Review the new `quality.rs` actual-service regressions and associated implementation:

- Completion explicitly invalidates live index observations with existing owner
  epochs/state in the same SQL transaction, preserving journal identities and
  authoritative steps. Exact fenced owners may reclaim once and re-observe;
  foreign/reclaimed/completed records are never adopted. No network/scan under
  lease. Ordinary/active bound-child F11 races, shared-registry foreign-root
  protection and legitimate same-ID takeover are tested. The older parent returns
  stored complete authority after the child's refreshed handoff.
- All verified publication transitions consume retirement in their own transaction,
  including legacy reconciliation, settlement verification and already-current
  binding. Distinct-endpoint tests stop before classification and force SQL rollback.
  Qualified Push proof produces ObservedPublished without inventing Fetch data;
  internal Fetch-only planning is separated. New context-only index/migration is
  additive/idempotent and marked loss requires recovery. A late classifier cannot
  delete retirement earned since its earlier verification.
- Fixture `image` uses kind tags, framed path/payload/entry counts, Git executable
  mode and native permissions; meaningful type/mode/boundary collision controls
  pass. This is not a consent-digest change. The type/exec test is Unix-only;
  local Linux total is 74, native Windows total is expected to omit that one case.

Original SSH regression sources and shared fixtures are untouched. One legacy
state unit fixture now clears the new index marker when deliberately rebuilding
its table as authentic Cycle 04; all terminal-row assertions are retained.
No new product policy, dependencies, GPUI/global config, sleeps, subagents or
outward actions. Task 8/full/native gates and the previously recorded Clippy
carryover remain pending. Return the complete uncommitted change for QUALITY
recheck, not a commit or closure.

## Historical SPEC3-P2 handoff (SPEC later passed; QUALITY2 above supersedes)

All original Task 7 journeys and approved amendment controls are implemented and
targeted-verified, uncommitted against exact base
`c0e972655f04ea67bb9411877b0627e7eb8439ac`. **Not approved/complete until fresh
reviews.** Final evidence: document_promotion **64 SSH cases after SPEC3-P2**, affected
remote_synchronization **63**, remote_merge_recovery **25**, remote_polling **84**,
comment_publication **41**, and targeted promotion library **188 passed**. Formatter/
check passed. Full proof mapping, historical RED, fixes, files and remaining gates
are in the execution ledger and review-ready ticket comment.

SPEC3-P2 corrected evidence gaps in the earlier 63-case handoff: offline tests now
restore a real SSH advertisement disconnect at the same endpoint/key and retry
identical requests through retired/new actual children; privacy positives and
transactional actual-operation-row negative controls cover live A/live B and both
backups; the rejected title now crosses public canonical validation and its error/
discovery/raw-store absence is checked. The final full target passes 64. Only
`tests/document_promotion.rs`, `tests/document_promotion/journeys.rs` and evidence
documents/comment changed in this correction. Production diff fingerprint stays
`a9d8060139dd15f7a76b3e125ae31e08d900b7d4`; shared fixtures are unchanged, so the
four prior affected-target results were not rerun. Resubmit to SPEC, then QUALITY.

Review the entire Task 7 diff: public-only SSH target/submodules, deletion
authority versus fetched tracking, raced replacement fetch, allowed pre-F6
tracking advance, endpoint-qualified retained-R1 old/new journal handling and
permission consumption at verified publication, durable F11 refresh handoff,
two hidden inside-F10/F11 exit seams, equivalent relative ancestor visibility
for original source-included regression harnesses, and amended documents. No old
regression test was weakened or edited. Four-target native wiring stays manual.

**Task 8 remains next after review/checkpoint, not executed here.** Full Rust/CLI/
native gates are pending. An optional targeted Clippy command stopped in existing
library code with **22 pre-existing findings** (14 large-error, four needless-
borrow, three collapsible-if, one unnecessary-unwrap); no lint category was
suppressed and no Clippy success is claimed. Carry these into Task 8's gate.
No subagents, staging, commits or outward actions; ticket stays open.

### Approved Task 7 bounded amendments

Task 6 is reviewed/committed at c0e9726. Task 7 is review-ready, uncommitted, inline
with no subagents. The owner approved both reviewed amendments through the question
tool: R1 under unknown history requires **retained validated retirement + no masking
state**; lost registry evidence stays HistoryUnknown pending explicit recovery/
republication (no new API, no absence/old-merge inference). Only existing accepted
`discovered_items.title` metadata and DB/WAL/backup copies allow titles; every
workflow/recovery/operation/promotion/retirement/remote row and formatted result/
diagnostic/child output stays title-free. Whole stores/sidecars/backups remain free
of bodies/comments/endpoints/credentials/key material and rejected non-indexed
title sentinels. No new title-bearing workflow write. Historical RED is retained.

Task 7 now covers all original journeys plus real public recovery/retained-R1,
loss-across-reopen, other-item barrier, publication-consumption/deletion, F7 new
retirement and positive/negative title privacy controls. The deletion-OID fix
already has absent/stale tracking SSH regressions; fresh full Task 7 spec/quality
reviews are still due. Full target and four affected SSH targets passed;
Task 8 broad gates/native execution remain deferred. No commit or outward action.

## Historical Task 6 handoff (superseded)

Tasks 0–5 are reviewed/committed; Task 5 checkpoint is `8d06577`, the current
unchanged HEAD. Full Task 6 work is uncommitted in this ticket worktree and has
**SPEC PASS and QUALITY FINAL APPROVE, with no remaining findings**, conveyed by
the controller/owner. Review covers the complete Task 6 change against exact base
**`8d06577f6af17e2532d072d7ee14419dbf8a84b4`**, including the concurrency fence,
authoritative-completion fix, helper extractions and all earlier corrections,
not merely the last diff. No Task 6 review or owner decision remains pending.
Final recorded targeted gate:
`devenv shell -- cargo test --locked --lib promotion` **188 passed/0 failed/807
filtered**, 188.09s, warning-free. Formatter/check pass. No Task 7/full/native gate
or worker commit; **controller checkpoint is next, then Task 7**. Exact file
inventory, changes, RED/GREEN, acceptance mapping and final reviews are recorded
in the execution ledger. This coordination pass changes only documentation and
a canonical comment; it runs no Rust commands/tests and makes no commit. Earlier
handoff states below are historical and superseded by this reviewed-complete status.

The owner approved T6-1 through the question tool: inherit Cycle 06's existing
backend-lock exception (Cycle 06 contract 228–242; approved 2026-10-08 Task 4
resolution protocol 36–46). Independent review confirmed the current refusal is
safe. Ambiguous libgit2 locks after process exit must be preserved with `Ok`
cleanup pending RecoveryRequired plus refresh/IndexPending. Only after an operator
quiesces writers and verifies/repairs metadata may the identical request resume
unfinished SQL. No automatic lock removal/adoption, new lock journal or new owner
question. The rare inside-F10/F11 interval explicitly costs operator intervention.

Retain the real crash case and assert full preservation/no repeated effect
counters before **any** repair. In the disposable fixture only, simulate quiesced
verified operator removal of the exact known lock, then complete under the same
request. Existing SQL history/retirement/reservation and committed-outcome replay
invariants remain those of T4-1. This qualifies every inside-F10/F11 immediate-
retry/finish requirement below; normal recovery remains automatic. Task 6's complete
implementation and qualified recovery tests are now reviewed accepted; T6-1 is
approved history, not a pending owner question. Task 7 starts only after the
controller checkpoint; Task 8/full/native and C5/D5 frontend gates remain later work.
Exact approval, RED/GREEN and final reviews are in the execution ledger.

## Historical planning and Task 5 handoff (superseded)

You are picking up Wave 02 Cycle 09, **Confirmed Managed-Document
Promotion**. It is tracked by Manyhands ticket `01K7F6H9J2N4Q6S8V0X2Z4B6DH`.

The owner approved the Cycle, the design and the implementation plan on
2026-10-15. The owner also accepted the recommended answer to owner decisions
D1–D7 and internal ruling R1.

Implementation and checkpoint commits are authorized in subagent-driven
development mode, with independent spec then quality review. Targeted tests
only until Task 8 override the full Task 0 baseline and Task 6 library run.
Task 0's delete/prune probes are verified; its interrupted-retained-conflict
blocker was reviewed and the owner approved the ownership-first amendment.
The remaining Task 0 characterizations are recorded in the ledger, including
the full replay/restart matrix, merge-tree equivalence and fresh edit context.
Task 0 passed independent spec and quality review and is committed as `25514a3`.
Task 1 guarded remote context deletion passed both reviews and is committed as
`3d47027`. Task 2 observation/preview, including its spec corrections and approved
T2-1 mode/type amendment, passed both final reviews and is committed as
`6d4ad1664462c42c24b4a820024d76d7c86ca8db`.

**Task 3 primary integration is reviewed complete at the local gate and committed
as `3b925e141bec9ae309024bdbfbf73aa6ab48caae` (`3b925e1`).** Its prior
spec review PASS is retained; the same quality reviewer approved Q3-1 resolved
with no findings. Those reviews covered the Task 3 change set, including the
include-context correction, against base `6d4ad16` (full SHA above); the controller
has since committed it. **T4-1 is owner-approved; Task 4 passed both final reviews
and is committed as `f641f917113cc252f429860ba5092efcbbfa260c`. Task 5 is now
reviewed complete at the local gate, uncommitted for the controller's checkpoint.
Task 6 is next after that checkpoint.** Current HEAD and full Task 5 review base
are `f641f917113cc252f429860ba5092efcbbfa260c`.

Final controller-reported Task 5 implementation reviews:

- **SPEC PASS**, including the P2 pre-effect refresh identity correction.
- **QUALITY APPROVED**, no fatal issues; its single nonblocking lookup minor is
  resolved in the current tree and targeted-verified.

The review boundary is the full uncommitted Task 5 change set against that exact
base, not merely the last correction. Prior pending-SPEC states are superseded;
historical RED/GREEN and review evidence remain in the execution ledger and
canonical comment `01M51H7Z00N4Q6S8V0X2Z4B6E1`. Approvals are controller-reported;
no additional reviewer execution or minor-fix re-review is inferred. Full Task
8/native gates remain deferred; controller owns the pending checkpoint commit.

### Task 5 delivery and Task 6 handoff

Task 5 delivers audited document_promotions storage, typed steps/categories and
stored refusals, persisted child attempts and ownership-first retry seams,
cross-journal IDs, promote arbitration/restart/epoch fencing, F1–F12 names and
parent-step read status. T4-1 completion composes retirement, complete and
applicable reservation finish in one SQL transaction, with actual-state commit
reconciliation and the private Git evidence/ref lock retained by the caller.
SPEC P2 closes create_folder's pre-effect guard hole while preserving the exact
authoritative child refresh exception.

The final quality minor replaces full-history work in two bounded lookups:
begin_record selects the requested item's unfinished row with the partial-index
predicate; explicit-ID status uses read_record. Listing and startup audit retain
full history. Identity guards, stored refusal replay and outcome semantics stay
unchanged. Latest exact checks:

- `devenv shell -- cargo fmt` — exit 0.
- `devenv shell -- cargo test --locked --lib promotion::record` — **23 passed,
  0 failed, 897 filtered**.
- `devenv shell -- cargo test --locked --lib read::status` — **10 passed,
  0 failed, 910 filtered**.

Earlier 45 reservation, 43 recovery, required inventory and focused actual-folder
checks remain recorded evidence; no broad rerun is inferred. No test was added
to mirror the query implementation, as requested. Task 6 has not started. After
the controller's checkpoint, compose the real runner with Task 3's integration
stages and Task 5's storage/transaction seams; preserve ownership-first child
selection, integrating/branch_deleted-before-effect ordering and evidence lifetime
through SQL commit/reconciliation. Tasks 6/7 still prove real F1–F12/F10/F11 and
SSH orchestration; Task 8 owns the full local gates.

### Task 4: historical blocker and owner-approved recovery boundary

The original completion transaction required atomicity across a Git
tracking-ref deletion and SQLite history/retirement/completion state. Existing
Git and SQLite transactions commit independently. The temporary real-resource
probe confirmed SQL rollback cannot restore an already-deleted Git ref; its
exact commands/counts and source anchors are retained in the ledger. The probe
was removed before implementation. The historical reviews below apply to that
blocker evidence; Task 4's new RED/GREEN evidence is in the ledger's resume entry.

- **Independent SPEC confirmed** a material contract contradiction requiring an
  **owner amendment**.
- **Independent QUALITY approved the evidence without findings.**
- Both reviews were read-only; no reviewer reruns or code changes occurred.
  These are blocker-evidence reviews, not implementation approval. Full Task 8
  and native gates remain deferred.
- The controller checkpointed this reviewed evidence as `79463c5`. Its historical
  STOP/NOT APPROVED status is superseded by the owner's approval below. See the
  ledger and canonical comment `01M51F7Z00N4Q6S8V0X2Z4B6DZ` for the original record.

**T4-1 amendment — OWNER APPROVED 2026-10-16.** The owner answered “yes” to
the controller's final Git-first proposal. The historical STOP/review evidence
above and in the ledger is superseded by this approval. Resume the full original
Task 4 test/helper/R1 scope. Fresh controller fetch/rebase was a clean no-op at
`79463c53beb400d95a311e558e64caf8efd2beac`; origin/main remains `89d04f9`,
ancestry passed. No additional worker rebase is needed in this execution.

1. Persist the existing planned durable **`branch_deleted` intent** before the
   tracking-ref effect. No new phase or table is needed by this amendment.
2. Under the lease and ownership/frozen-configuration checks, delete the tracking
   ref only at the expected **direct OID**, or proceed on verified true absence.
   Wrong/new conflicting OIDs, symbolic refs (including dangling symbolic refs)
   and unreadable refs stop cleanup.
3. Then use **one SQL transaction** for history deletion, retirement insertion,
   promotion completion and applicable reservation finish. This transaction does
   not atomically include or roll back the preceding Git effect.
4. During the split interval the ref may be absent, but applicable **existing**
   SQL history remains intact (`observed_published` when present), no retirement
   exists, the durable record is pending and the applicable reservation stays
   held. Do not manufacture `observed_published` history for local-only or
   cache-loss cases. SQL statement failure followed by rollback of the uncommitted
   transaction preserves SQL, not an already-deleted Git ref. An uncertain commit
   result does not prove SQL unchanged: reopen and reconcile actual completion.
   If completion and applicable reservation finish committed, replay the stored
   outcome. Git deletion may remain whether SQL rolled back or committed.
5. After-integration failures return **`Ok` with cleanup pending
   `RecoveryRequired`, plus refresh**. Same-ID retry performs actual ref,
   ownership and configuration checks and finalizes SQL without repeating
   completed lifecycle work, or replays the stored outcome when actual completion
   already committed. **Cache loss or ref absence alone is not retirement
   proof**; do not infer retirement without the required durable intent/checks.
6. Add tests for split boundaries **inside F10/F11**, SQL statement and commit
   failures/uncertainty, split-state existing-history preservation, same-ID
   finalization and wrong/conflicting/symbolic/unreadable ref refusals. Cover
   reopening/reconciling uncertain commit results and replay of committed
   completion/reservation finish. Tests for rollback of uncommitted SQL must
   distinguish SQL preservation from the surviving Git deletion.

This is the approved recovery boundary. Task 4 supplies helpers/private evidence
and R1 migration; Tasks 5/6 later supply durable intent/completion/reservation
composition, with inside-F10/F11 lifecycle tests in Tasks 5–7. Do not implement
fake Task 5 storage in Task 4. That reviewed implementation is committed as
`f641f91`; Task 5 supplies the real storage/completion composition above.

Delivered Task 4: lease-owning generic cleanup helpers for both document/ticket
contexts; ignored/untracked/locked/partial preservation; guarded branch deletion;
Git-first direct tracking deletion with private evidence/absent-slot ref lock;
SQL-only retirement; additive migrated/audited R1 table and per-item publication
evidence. Observed publication consumes retirement in the same SQL batch; other
history rules stay unchanged. Current observation projection rotates to a new
batch excluding the retired item, preserving the original pinned synchronization
pass. Synchronization/poll/merge production code was not changed. Standalone tests
also prove SQL trigger/outer rollback/deferred-commit failure, fresh Edit from
primary HEAD, foreign .git-link refusals and unchanged snapshot invariants.
Reviewed corrections also preserve branches when registrations are incomplete
through an unfiltered physical admin inventory, reject hidden index flags without
index mutation, and explicitly prove populated submodule cleanliness recursively
under the existing lease with bounded/fail-closed inspection.

The runner must retain evidence/ref lock through SQL commit/reconciliation and
compose real branch_deleted intent, ownership/generation/configuration fencing,
promotion completion and reservation finish. Task 5 supplies the SQL composition;
Tasks 6/7 still own actual runner F10/F11 and uncertain-result outcome/refresh
tests. Task 4 does not
implement those records or claim native Windows pruning was executed. Exact
commands/counts and final review approvals are in the ledger and comment
`01M51G7Z00N4Q6S8V0X2Z4B6E0`.
Retained evidence includes final **34 cleanup tests passed**, **67 state tests**,
**10 refs tests**, inventory and focused poll/SSH regressions. These are prior
executions, not reruns by this status update.

### Task 3: reviewed integration and identity composition

- Delivered leased re-observation/digest/base guards, exact merge subject/tree/
  parent order, one guarded primary update, contained handling and first-parent
  reconciliation with pending recovery after uncertain effects.
- Immutable request/evidence and lease-owned preparation/candidate stages are
  the later storage/runner boundary. Task 5 persists integrating evidence before
  `write_commit`; Task 6 composes the lifecycle and maps pending recovery.
- **T3-1 is resolved as an internal engineering ruling, not a pending owner
  decision.** Existing RFC policy requires atomic local identity persistence
  before commit. The historical commit-only suggestion is superseded. Shared
  selection retains sync validation/journaling; promotion uses the bridge's
  lease-held atomic primitive without its differing public-setter policy.
- **Q3-1 is reviewed resolved.** Before installation, proposed identity is checked
  against native include expansion in the original repository context, including
  relative/nested/conditional includes. Preserve the approved correction.
- Reuse recorded evidence: final integration **31 passed / 0 failed**, focused
  include regressions **3 passed / 0 failed**, shared bridge identity **42 passed /
  0 failed**. The earlier promotion selector passed all 53 Task 2 observation
  tests plus then-current integration tests, and the existing sync identity SSH
  regression passed. Exact commands/intermediate results are in the ledger;
  these are prior executions, not reruns or inferred reviewer test results.
- Final review status is recorded in the ledger and comment
  `01M51E7Z00N4Q6S8V0X2Z4B6DY`. Full Task 8/native gates remain deferred.

### Retained Task 2 history: T2-1 approved, implemented and reviewed resolved

The original approved digest encoded changed path/category/new blob OID but not resulting
Git mode/type. With a remote, moving context OIDs are deliberately excluded.
Temporary real-Git probes confirmed equal digests for `extra` changing from
regular `100644` to symlink `120000`, or to executable `100755`, with the same
blob OID. Both Added and Modified cases were reproduced against an exactly
unchanged primary; both previews stayed Ready and their path/category change
vectors were equal. All four expected digest-inequality assertions failed.

Exact RED: `devenv shell -- cargo test --locked --lib promotion::observe::tests::t2_1_ -- --nocapture`
→ **0 passed / 4 failed / 822 filtered**. The temporary probes were removed;
`devenv shell -- cargo test --locked --lib promotion::observe`
→ **48 passed / 0 failed / 774 filtered**. That historical preservation pass did
not clear T2-1; the owner subsequently approved the encoding amendment below.

**Approved and implemented:** immediately after each optional new OID, bind the
resulting Git entry mode including type bits as a four-byte **u32 big endian**.
Added/Modified use the result Git tree entry; Deleted encodes absent OID and
**zero u32**, not its old mode. Evidence is private; public DTOs are unchanged.
Keep domain `manyhands promotion v1`, finalizing an unshipped encoding. D6 still
excludes moving commit/tree OIDs and unrelated primary-only paths.

Retained regression RED/GREEN:
`devenv shell -- cargo test --locked --lib promotion::observe::tests::t2_1_`
→ **0 passed / 5 failed / 822 filtered** before the hash fix, then **5 passed /
0 failed / 822 filtered**. This includes all four mode transitions plus an
independent fixed-width/deletion-sentinel protocol vector. Final
`devenv shell -- cargo test --locked --lib promotion::observe`
→ **53 passed / 0 failed / 774 filtered**, preserving prior spec fixes and
primary-only/context-primary-merge invariance. Formatter/check passed.

Cycle/design/plan now include the approved amendment. Full historical blocker,
approval, implementation and verification evidence is in the execution ledger;
comments `01M5187Z00N4Q6S8V0X2Z4B6DR` (blocker) and
`01M5197Z00N4Q6S8V0X2Z4B6DS` (approved delivery). The prior spec path/mode-policy
findings remain resolved. Final quality and spec rechecks both passed, recorded
in the ledger's final review section and comment `01M51A7Z00N4Q6S8V0X2Z4B6DT`.
That historical coordination-only update changed no source and reran no tests.

### Where things are

| Item | State |
| --- | --- |
| Worktree | `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DH`. Work only here, never in the main checkout. |
| Branch | `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` |
| Current HEAD / Task 5 review base | `f641f917113cc252f429860ba5092efcbbfa260c`: reviewed Task 4 checkpoint. Task 5's full change set is uncommitted against it. |
| Task 0 resume base | `df4fe56ce0548290293c3f6d96f23f1ddbfc51a2` (reviewed blocker evidence); planning checkpoint `d027d98`; `origin/main` remains `89d04f901636229392a311241f8e777f0a00962b`. Fresh controller fetch/rebase was a clean no-op; ancestry passed. |
| Published ticket ref | `origin/manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DH` still at the pre-rebase `f017c73`. A push must reconcile it. Never force-push without explicit owner authority bound to the observed remote SHA. |
| Checkpoints | Tasks 0–4 reviewed/committed (`25514a3`, `3d47027`, `6d4ad16`, `3b925e1`, `f641f91`); T4-1 owner-approved. Task 5 reviewed complete: SPEC PASS, QUALITY approved, nonblocking lookup minor resolved/verified. Controller checkpoint pending; Task 6 next afterward. |
| Ongoing-task preflight | Reuse the controller's completed fetch/rebase/ancestry preflight against `origin/main` at `89d04f901636229392a311241f8e777f0a00962b`. This status pass performs no fetch, rebase or checkpoint commit. |
| Main checkout | It has an unrelated untracked `.superpowers/`. Leave it alone. |

### Read in this order

1. `AGENTS.md` (repository rules), then this brief.
2. `docs/Cycles/wave-02-cycle-09-confirmed-document-promotion.md`: the
   contract, the decisions with their accepted costs, and the acceptance
   table.
3. `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-design.md`:
   the interfaces, the change set and digest, the records, the F1–F12
   lifecycle, the reconciliation, and the child-attempt table.
4. `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-implementation.md`:
   Tasks 0–8. Follow it exactly.
5. `docs/plans/2026-10-15-wave-02-cycle-09-confirmed-document-promotion-execution.md`:
   the grounding facts, planning reviews, Task 0 evidence/T0-1 approval, Task 1
   reviewed delivery, Task 2's approved T2-1 correction, and Task 3's internal
   ruling, Q3-1 correction and final independent review results, then Task 4's
    historical T4-1 blocker/reviews, owner approval and full Task 4 resume evidence.
   Record your evidence here as you go.

Cycle 08's documents (`docs/Cycles/wave-02-cycle-08-*` and
`docs/plans/2026-10-11-*`) are the closest precedent for style, ledger entries
and the shape of the real-SSH test target.

### What is being built (one paragraph)

Two operations are being added:

- `preview_document_promotion`. It is local and read-only: no network, no
  lease, no writes. It returns either every blocker or an exact preview: the
  primary-path change set, the integration kind, the publication and cleanup
  effects, and a BLAKE3 `PromotionExpectation`.
- `promote_document`. It is bound to that digest. With a publication remote,
  it does the following in order:
  1. Runs a context-synchronization child.
  2. Re-checks under the lease.
  3. Makes one `Promote document <ULID>` merge with parents `[base,
     context]`.
  4. Runs a primary-synchronization child.
  5. Deletes the remote context branch, guarded by `push_negotiation`.
  6. Removes the worktree.
  7. Deletes the local branch.
   8. After durable branch_deleted intent and fresh guards, delete tracking first;
      then atomically retire history, insert `retired_contexts`, record completion
      and finish applicable reservation in SQL (approved T4-1).
  9. Refreshes the index on every `Ok` return after integration.

Without a remote, it merges locally, cleans up, and reports publication
pending. The parent record `document_promotions` binds child attempts by ID,
the same way comment publication binds its child synchronization.

### Accepted decisions, in brief

| ID | Accepted answer |
| --- | --- |
| D1 | Promotion takes no draft. It requires a clean, conforming context and validates the document in the integrated tree. |
| D2 | A local primary that is ahead of or diverged from `T(P)` blocks (`PrimaryAheadOfRemote`). |
| D3 | Without a remote, the merge is computed in memory, and a conflict is refused before the primary changes. |
| D4 | New context content arriving during synchronization gives `ReconfirmationRequired`, and the record ends `abandoned`. |
| D5 | A deleted, or history-unknown, remote context blocks only when `T(C)` is absent. An already-contained context is exempt. |
| D6 | The digest binds the change set and the D2 relation, not the moving OIDs (when a remote is configured). This refines the Git RFC's "refs/OIDs" binding, and the owner accepted it. |
| D7 | Ignored files in the context worktree block cleanup (`pending(ignored_files)`). Never delete them. |
| R1 | `retired_contexts` lets a fresh context of a promoted item publish for the first time, even when repository `history_unknown` is set. It is written only in the completion transaction. |

### Invariants that the review proved easy to break

- **Integration base.** With a remote, require `base ≤ context_oid` under the
  integration lease. A poll can advance `T(P)` between the children. Without
  this check, a merge that takes the context tree silently drops primary
  commits. The failure is `PrimaryAdvanced`, and the retry re-synchronizes
  with a new attempt.
- **Nothing strands.** Pre-integration refusals end `abandoned`.
  `context_changed`, `ignored_files` and `remote_context_changed` rows are
  superseded by a newly confirmed promotion. For context and primary children,
  authoritative completion is never rerun. Retained conflict/owned-resolution
  evidence takes precedence over errors: an interrupted retained child keeps
  its original attempt ID with restart; retained `RecoveryRequired` also
  restarts that ID. Only confirmed absence of retained ownership permits a
  new attempt for cancelled/interrupted/rejected/offline/busy/yielding or
  recovery failures. Uncertainty is not absence. Task 0's evidence confirms
  the real child behavior and passed independent reviews.
- **Applicable existing history stays until completion** (`observed_published`
  when present). Never fabricate it in local-only/cache-loss cases. Early cleanup
  must not grant new first-publication eligibility.
- **Errors mean the primary is unchanged.** Integration reconciliation
  searches for a reachable matching merge. Recovery after integration is a
  pending category, never an error.
- **Lease rules.** No network call, prompt or discovery scan holds the lease.
  Every local step re-observes under the lease first.
- **Records stay content-free.** They hold only IDs, ref names, OIDs,
  timestamps and fixed categories. Never a title, body, path content,
  endpoint or secret.

### How to resume

1. Preserve existing authorization and reviewed evidence; read the latest
   execution ledger. The controller owns checkpoint commits after reviews.
2. **Task 5 is reviewed complete at the local gate against `f641f91`: SPEC PASS,
   QUALITY approved and the nonblocking lookup minor resolved/verified.**
   Controller checkpoint is pending; **Task 6 is next after it**. Reuse evidence
   and approvals rather than redispatching completed work. T4-1 remains approved;
   no pending owner decision. This worker makes no checkpoint commit or Task 6 change.
3. Tasks 0–5 passed their independent reviews. Before subsequent
   implementation in a fresh session:
   1. Fetch, then rebase the ticket branch onto `origin/main` from inside
      the worktree. Never rebase main onto the branch.
   2. Re-scan callers if the source changed.
   3. Reuse the recorded guarded-delete, prune, in-memory merge equivalence,
      fresh-context and child replay/restart findings. Review the execution
      ledger's Task 6 integration notes, especially row existence, cancellation
      acknowledgement when retiring an active reservation, and ownership.
   4. Run targeted tests only until Task 8.

   A characterization that contradicts the design stops work for an owner
   amendment. Do not adapt silently.
4. After the controller's Task 5 checkpoint, continue Tasks 6–8
   in order, retaining the approved T4-1 boundary. Each task
   starts with a failing test and its recorded red run. Each ends with a commit
   (only when authorized), a ticket
   comment, and an independent review before the next task. The plan's
   dependency notes allow Tasks 1, 2, 4 and 5 to proceed independently after
   Task 0.

### Repository rules you must follow

- Run every Rust command as `devenv shell -- cargo …`. The full Task 8 gate is:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

  Until Task 8, the owner requires targeted tests only, including Tasks 0
  and 6. Do not run the cargo gate on Sprites: that experiment was
  abandoned.
- **Ticket comments.** Write them directly as canonical files at
  `.manyhands/comments/01K7F6H9J2N4Q6S8V0X2Z4B6DH/<ULID>.md`, with the same
  front matter as the existing comments. Add one at every plan checkpoint:
  progress, decisions, blockers, verification and review-ready. Use the
  Manyhands CLI instead once it supports comments.
- **Native CI.** `.github/workflows/build.yml` runs only by manual dispatch.
  Ask the owner before **each** run, since approval does not carry over. The
  current matrix has four targets: Windows aarch64 was dropped by owner
  decision during Cycle 08.
  - On a platform failure: narrow the matrix in a commit of its own, fix the
    failure, run only the affected tests locally, then re-dispatch.
  - Once green, restore the matrix in its own commit and run the full set
    once more.
  - Add `--test document_promotion` to the workflow's test list (Task 7).
- **Before calling the Cycle review-ready,** run `git merge-tree` against
  current `origin/main` to prove the branch still integrates.
- **Each needs its own owner authorization:** commit, push, pull request,
  native dispatch, merge, closure and worktree cleanup. The ticket closes
  only after code-review or pull-request approval, normally in the final
  pre-merge push.

### Known downstream items (not this Cycle)

- **Wave 03 C5/D5 (whichever comes first)** binds this operation at the F2
  boundary: `prepare`/`execute`, the ten-minute expiry, the preview schema
  values, and changing `bind/resume.rs` `NotResumable` for promote. Task 6
  updates `docs/research/wave-03-api-audit.md` to say so.
- **Cycle 10 (ticket closure)** reuses `src/repository/cleanup.rs` and
  `retired_contexts` unchanged.
  **Correction 2026-10-18 (Cycle 10):** not unchanged. Cycle 10 Task 1
  made the runner generic over a lifecycle seam, and Task 5 generalized
  `RetirementEvidence::matches_promotion` to a kind-checked `matches`, added
  the `Close` reservation arms and made the `exact_mappings` Delete arm
  generic. See the
  [Cycle 10 execution ledger](2026-10-17-wave-02-cycle-10-confirmed-ticket-closure-execution.md),
  Tasks 1 and 5. `retired_contexts` is reused with `kind='ticket'`.
- **Collaborators' clones.** A collaborator's clone keeps its local context
  after promotion, and its poll reports it `remotely_deleted`. Explicitly
  discarding that context is later work.
