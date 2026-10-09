# Wave 02 Cycle 06 — fresh-session handoff

## Current: Task 5 milestone 2 verified; M3 next — 2026-10-09

M2 source b9bd929 passed all five native targets in
https://github.com/vhodges/manyhands/actions/runs/37935757056
Owner clarified targeted local tests of changed code are fine; run them (and fmt/
strict clippy, which CI does not run) before each native dispatch. Next one writer:
M3 = remainder D/E/F (metadata retirement after external repair, publication
continuation envelope, post-merge race composition); then M4 = A/B/C/G coverage.
The ledger's M2 sections list the remainder. Save/Git-index gap is separate ticket
01M4GD0KKXW684QBA49F6EX3WE. Ticket open; Tasks 6/7 pending. Sections below are
historical.

## Current: Task 5 milestone 2 reviewed, native CI next — 2026-10-09

The previous session stalled after the M2 quality review; a fresh Claude Code
session recovered from WIP 489322f. M2 (local-first restart before preflight/
transport, offline pending-conflict inspection, exact-ordered-parent external
repair, released-checkpoint descendant continuation, latest-window stage routing)
has spec approval after two P1 fixes and quality approval after one P2 fix (HEAD
rechecked under the lease). Every M2 test is source-written and UNRUN. Next: inspect
the exact-source five-target run; local Rust only if Linux CI regresses. Then one
writer for the Task 5 remainder A–G listed in the ledger's M2 section. The native
workflow has no fmt/clippy step; those remain Task 7 local gates. Ticket open.
Sections below are historical.

## Current: Task 5 window foundation verified; orchestration next

Source 0fefeb5204f3b62bcaf5b2116a26979ff8ff984c passed all five native targets in
https://github.com/vhodges/manyhands/actions/runs/37899976043
The source-review and CI gate for milestone 1 is satisfied. Preserve that work;
continue local-first restart, external repair, live window/remaining-stage routing,
continuation/publication and discovery composition. Task 5 is still incomplete.
Owner reaffirmed CI-first: local Rust runs only for Linux CI regression.
Earlier pending sections below are historical. Ticket stays open.

## Current: Task 5 window foundation source-reviewed, CI next

Baseline 8f32144 is all-five-target green with F1 integrated (run 37889668738).
Task 5 first milestone adds append-only windows/frozen-pass bindings and transactional
ID/FK/native-provenance-preserving migration; mandatory-zero audit refuses evidence
loss. Both read-only reviews approve the bounded source conditionally on CI; new
tests are UNRUN, no local Rust command under CI-first. Live orchestration remains
window zero; Task 5 is incomplete. Next publish/verify this foundation, then continue
restart/external-repair/window continuation/publication/discovery composition.
Tasks 0–4 accepted, Tasks 6/7 pending, ticket open. Ledger records exact workers.

First-mile verification checkpoint published as 0fefeb5; current native run:
https://github.com/vhodges/manyhands/actions/runs/37899976043
Inspect exact-source result. This milestone does not complete Task 5.

## Current: Task 4 accepted, Task 5 next — 2026-10-09

Current Task 5 implementation base is 8f32144, fully verified with F1/Task-4
combined native SUCCESS on all five targets:
https://github.com/vhodges/manyhands/actions/runs/37889668738
Preflight and conditional integration sections below are historical; this gate
is complete. Next one writer implements approved Task 5 recovery/window/external-
repair behavior, with source-written tests and review before manual CI. Local
Rust verification only if Linux CI regresses. No accepted implementation redispatch.

Task 5 entry preflight now rebased ticket onto merged F1 main 6cf5d7f, preflight
HEAD 54874c4. Accepted Task 4 source maps to 905273e; Task 3 base to 1fbfc2e.
Original a68eba7 native green remains historical exact-source evidence. Source
integration preserves both read/native modules and the union of 21 native targets.
New immutable comment created_by guard and Windows Read import correction have
conditional source-review approval, with regressions written but unrun per CI-first.
Next publish reviewed integration preserving old remote history without force,
then fresh five-target CI before Task 5 implementation. No accepted work redispatch.

Reviewed integration is now published as 1a65a11, preserving old a68eba7 history
with an identical reviewed tree through non-force reconciliation. Integrated run:
https://github.com/vhodges/manyhands/actions/runs/37882898787
Next inspect this exact-source F1/Task-4 union matrix before new Task 5 code.

Integrated run 37882898787 passes Linux/Mac and reveals three F1 Windows read
classification/diagnostic expectations. Bounded source-written fixes and read-only
spec/quality reviews approve b1d39b8; no local Rust jobs. Current integrated run:
https://github.com/vhodges/manyhands/actions/runs/37887084202
Inspect this exact combined-base gate before Task 5 implementation.

Run 37887084202 passes Linux/Mac and reveals one later Windows ghost-worktree
fixture. Test-only Git encoding/physical parser controls now reviewed and published
as 8f32144, no local Rust job. Current combined-base gate:
https://github.com/vhodges/manyhands/actions/runs/37889668738
Inspect this source before Task 5 code. A read-only Task 5 readiness inventory
identifies append-only windows, local-first replay and external repair seams;
no material owner-policy question, but implementation has not begun.

Accepted source: a68eba79a41c6d083a3a12cb9dd6babc6d7f291a. Native run
https://github.com/vhodges/manyhands/actions/runs/37877620596
completed SUCCESS on all five targets (Linux x86-64/ARM64, Windows x86-64/ARM64,
macOS ARM64), with complete configured tests and release artifacts. Whole-task
and subsequent bounded reviews approve the code. Earlier paused/pending sections
below are historical; do not redispatch accepted Task 4.

Next: approved Task 5 reconciliation/external-repair/remaining-work implementation
and independent reviews, then Tasks 6/7. CI-first ruling: local Rust fixes/checks
only when Linux CI regresses. Best-effort robustness ruling and backend operator
exception remain in effect. Ticket stays open. Push/manual CI authorized; no PR,
main merge, ticket closure or cleanup authorization. Ledger records full evidence.

## Resumed after owner pause — 2026-10-08

Owner requested "Please continue". Fresh main remains `60b0324`; identified dirty
work was preserved in unaccepted WIP `fb6d234`, and clean ticket rebase was a no-op.
Accepted Task 3 review base remains `9c7253f`.

Unix-native and Windows primitive/protocol seams now have independent spec/quality
approval. Whole-Task-4 review found cross-seam merge-entry, owner-release, refresh,
validation-scope and completed-restart gaps; regression-first fixes closed them
in two rounds. Final whole-task spec and quality reviews approve the source under
the best-effort ruling, conditional on native gates. Latest Linux full verification
passed 292 library tests and all integration/SSH/doc suites plus required static
checks and CLI. Final test-only prior-CI fixture correction additionally passes
two new alias regressions, state 28/reservation 28 and static checks, independently
reviewed. No disabled tests or production canonical-root workaround.

Owner authorized push and MANUAL native CI, noting previous macOS/Windows failures.
Workflow stays dispatch-only; affected Task-4 integration tests were added to the
existing five-target headless command. Next: publish reviewed verification tree
without force, preserving sole equivalent remote checkpoint 2dbd3b4 via verified
history-only reconciliation; dispatch native matrix and inspect/fix actual failures.
Task 4 remains unaccepted until these gates are handled; Tasks 5–7 not started.
No PR/merge/closure/cleanup authorization. Ledger records all agents and decisions.

Published verification checkpoint d0a4989 through history-only merge a9593a0,
preserving the sole equivalent remote checkpoint and identical reviewed tree
without force. Manual native run is dispatched on a9593a0:
https://github.com/vhodges/manyhands/actions/runs/37788386532
Next inspect/fix its actual results. Pending jobs do not accept Task 4.

Run 1 failed Windows build, a macOS secondary fixture and Linux unborn creation
cases. Reviewed regression-first fixes are pushed as d108e73 (see ledger for
root causes and isolated/default/full-suite evidence). Current second native run:
https://github.com/vhodges/manyhands/actions/runs/37795672482
Next inspect this exact d108e73 matrix; Task 4 remains unaccepted while pending.

Run 2 built all five release targets but exposed additional test-fixture
portability failures. Independently reviewed test-only fixes are published as
c3ad08c; third native matrix is running:
https://github.com/vhodges/manyhands/actions/runs/37799832943
Current next action: inspect/fix this exact source run, then accept Task 4 only
after native gates are handled. The ledger records earlier failures and fixes.

Run 3 passes both complete Linux jobs; remaining Windows runtime/Mac authoring
corrections have independent source review and complete local verification.
Published current source 4776157, fourth native matrix:
https://github.com/vhodges/manyhands/actions/runs/37837512832
This is the current run to inspect. Native FileRenameInfoEx behavior and APFS
authoring cases remain pending until actual results; Task 4 stays unaccepted.

Run 4 verifies Windows native rename primitives and most resolution cases,
with both Linux jobs green. Remaining hook identity/own-authoring and Mac
foundation fixes are independently reviewed and fully locally verified.
Published current source 7b897cf; fifth native matrix:
https://github.com/vhodges/manyhands/actions/runs/37846555078
Inspect this current exact-source gate before Task 4 acceptance or Task 5.

Run 5 resolves original Windows core cases and advances Mac to one confirmed
fixture receipt race. Remaining metadata-fixture/receipt/concurrency/worker-cleanup
corrections have independent review and local verification; published 7123ade.
Current sixth native matrix:
https://github.com/vhodges/manyhands/actions/runs/37856496873
Inspect this run before accepting Task 4. Tasks 5–7 still have not begun.

Run 6 passes Windows library/metadata/core and Mac SSH synchronization, exposing
two diagnostic-cache path and five enablement fixture-key cases. Independently
reviewed fixes retain strict decoding and nullable unrepresentable diagnostics;
complete local gates pass. Published current bed07e4, seventh full native matrix:
https://github.com/vhodges/manyhands/actions/runs/37863696730
This is the current exact-source run to inspect before Task 4 acceptance.

Run 7 exposed remaining authoring fixture path/EOL policy and APFS constructor
assumptions. Independently reviewed test-only corrections preserve missing records,
exact bytes and nonmutation; complete local verification passes 981 tests/cases.
Current source 600e0ff, eighth full native matrix:
https://github.com/vhodges/manyhands/actions/runs/37868930176
This is the current run to inspect before Task 4 acceptance; Tasks 5–7 not begun.

Run 8 exposed legacy path/status fixture assumptions and one startup notification
watchdog. Narrow independently reviewed test-only corrections and complete local
gates pass. Current source 7b8949b, ninth full native matrix:
https://github.com/vhodges/manyhands/actions/runs/37872797196
Inspect this exact-source gate before Task 4 acceptance; Tasks 5–7 not begun.

Owner now requires CI-first verification: local Rust commands only if Linux CI
regresses. Reviewed final transport fixture corrections are published as a68eba7;
current tenth full native matrix:
https://github.com/vhodges/manyhands/actions/runs/37877620596
Inspect exact-source results. Mac diagnostics are numeric-only; its prior exact
child failure remains qualified. Task 4 unaccepted, Tasks 5–7 not begun.

## Current pause — after shared Unix native writer, 2026-10-08

Owner requested pause after the current subagent finishes; it has returned.
**Task 4 remains unaccepted; Tasks 5–7 have not begun. No job is scheduled.**

- Worktree and branch are the existing ticket checkout named below.
- HEAD: `409a6f9da1554b9c1646907494bbdc05811d5d23`, unaccepted preservation WIP.
- Accepted Task 3 / final whole-Task-4 review base: `9c7253f`.
- Last freshly fetched/rebased main: `60b0324f3993b32f783fcb98e0f6e24dd9f750dd`.
- Empty staging verified. Dirty: src/repository.rs, remote sync.rs/sync_tests.rs,
  tests/discovery_rebuild.rs, ledger/handoff and comment 01M4C900. **New untracked
  src/repository/native_resolution.rs must be preserved.** No rollback/cleanup.
- Owner authorized OpenCode sequential subagents and independent reviews, replacing
  unavailable retained Pi resume; preservation commit/rebase completed earlier.
- New owner ruling: **"Best efforts on robustness, but it does not need to be
  perfect."** Apply proportionate review/fixes; document robustness limitations
  instead of requiring exhaustive failure-window or power-loss perfection.

### Completed this session

1. Recovered the timed-out writer's evidence; preserved/rebased 20 patch-equivalent
   commits onto fresh main. Complete checkpoint mapping is in the ledger.
2. Milestone 2c independently spec/quality reviewed after one fix round: refreshed
   log proof now checks frozen branch and exact live candidate before completion.
3. Linux actual SIGKILL/fresh-process recovery tests at 21 boundaries, plus foreign
   controls; child timeout cannot unwind (`_exit(86)` and Drop-canary regression).
   Independently spec/quality reviewed after one fix round; final death suite 6.
4. Discovery failures traced to legitimate bounded Git/cache Busy against flawed
   unconditional-success tests. Test-only fix independently spec/quality approved;
   controlled slow-fsync regression 3 passed, discovery suite 69 passed.
5. Latest native writer `ses_ee60ad283ffet61HIC7U2OlkIL` implemented shared
   Linux/macOS helper functionality: pinned traversal, nonblocking regular-file
   reads, native macOS rename APIs, Unix anchored lock/ref/log protocol, validated
   private-path libgit2 serialization, ordinary object/ref/log storage barriers.
   **This new seam is NOT independently reviewed.**

### Latest writer evidence and next steps

Latest worker reports Linux 103 sync tests, all-feature locked check, fmt check,
strict all-target/all-feature clippy, **full all-feature locked suite**, CLI smoke
and diff check passing through Devenv. These are attributed passes, not native
macOS/Windows evidence. No macOS compile/runtime, Windows or Linux ARM result;
macOS target libraries unavailable. Packed/alternate ODB flush policy, macOS full
device-cache flush and power-loss durability are documented limits. Do not repeat
an unchanged broad suite just to recover session context.

Next session: read current AGENTS/ledger and preserve dirty work during required
current-main preflight before new implementation. First obtain independent
specification and code-quality review of latest Unix-native changes/new module.
Earlier m2c/death/discovery seams are reviewed, not candidates for redispatch.
Then Windows implementation: retained ancestor/reparse-safe reads; volume/file
identity/private anchors and absent-only hard links; native output install,
metadata retirement and verified release; fixed-role ref/log proof, serialization
and appropriate barriers. Keep native verification factual/pending; final review
of all Task 4 uses `9c7253f`. Accept/checkpoint Task 4 before Task 5.

No accepted Task-4 commit or delivery authority. Ticket remains open. No follow-on
agent/test job launched after pause; narrow process-name check found no Rust
validation process. Preserve approved policies and the best-effort ruling.

## Earlier OpenCode resume — historical entry

This section supersedes the paused execution state below, which remains the
historical entry handoff. Owner authorized sequential OpenCode subagents and
independent reviews instead of unavailable retained Pi resume, plus unaccepted
preservation/rebase. Missing latest-writer evidence was reconstructed read-only.

Fresh main `60b0324` was fetched; preservation/rebase completed without conflicts.
Current HEAD is unaccepted WIP `409a6f9`, accepted Task 3 review base `9c7253f`.
All 20 commits are patch-equivalent; main's effective-copy indexing fix is included.
The ledger records the complete checkpoint mapping and recovered command evidence.

Milestone 2c received specification and independent code-quality approval after
one fix round. The current dirty sync.rs/sync_tests.rs add final frozen-branch and
exact-candidate checks after refreshed log observation. Three new stale-state
regressions demonstrated red/green; final 16 ref-log and 92 sync tests plus
Devenv check/fmt/strict clippy passed. The scoped verdict does not accept Task 4.

Next approved work: native helpers, actual child death without Drop and storage-
ordering characterization, discovery contention and final whole-Task-4 review
against `9c7253f`. Tasks 5–7 have not begun. No accepted Task-4 checkpoint or
delivery authorization; retain the open ticket and pending native evidence.

## Original paused state — historical Pi handoff

Paused at owner request after Task 4 milestone 2c writer timeout. **Task 4 is
unaccepted; Tasks 5–7 have not begun.** Preserve partial implementation, obtain
an attributed handoff, and independently review before accepting anything.
Do not redispatch accepted Tasks 0–3 or restart the architecture discussion.

- Ticket: `01K7F6H9J2N4Q6S8V0X2Z4B6DE`, still open.
- Worktree: `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01K7F6H9J2N4Q6S8V0X2Z4B6DE`.
- Branch: `manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DE`.
- HEAD: `45a139b0d799d4129e5af45688a33938be375e8b`, **unaccepted preservation WIP**.
- Accepted Task 3 whole-Task-4 review base: `51fe6f2`.
- Last rebased main: `ceb1be49477cfec5e14093082d00cf01a5367fb3`.
- Expected branch/HEAD and empty staging verified after timeout. Source/docs are
  dirty; no new commit, push, PR, CI dispatch, merge, closure or cleanup occurred.
- Original authorization: sequential subagent implementation, checkpoint commits
  after acceptance, independent reviews. No delivery authorization.

Read current AGENTS and implementing-a-cycle skill. For a genuine new
implementation boundary, follow its current-main preflight while preserving this
unaccepted dirty work; do not rebase or discard it blindly. Prior phase preflight
was completed, with accepted checkpoint mapping recorded in the execution ledger.

## Approved policy — do not reopen or silently weaken

Owner approved cooperative API/CLI writers respecting reservation/lease/authoring
guards; concurrent direct mutation bypassing coordination during bounded apply
or reconciliation is unsupported. Reads/unrelated safe work remain allowed.

After independently verified stock-API limitations, owner **broadened operator
recovery** from stale backend locks alone to interrupted/failed libgit2 ref/reflog
effects, including partial logs. Operator quiescence and verification/repair may
precede identical retry. This is NOT permission for agents to repair logs,
truncate/normalize/rewrite history, delete/adopt ambiguous locks, or implement a
custom backend. No ownership inference from contents, age or PID. Other owned
sentinel/path/index/metadata effects still recover automatically.

Retry must prove original baseline plus old ref, or intended log images plus
exact candidate ref; partial/mixed/foreign/unproved states remain recovery-required.
Reuse frozen candidates/parameters; no blind duplicate append. Native five-target,
real process-death/storage ordering, privacy, exact bytes, no-follow and all-side
closure/comment invariants remain mandatory. No new dependency is approved.

## Authoritative documents

In this ticket worktree:

- `docs/Cycles/wave-02-cycle-06-merge-and-conflict-recovery.md`
- `docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-design.md`
- `docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-implementation.md`
- `docs/plans/2026-10-07-wave-02-cycle-06-merge-and-conflict-recovery-execution.md`
- `docs/plans/2026-10-08-wave-02-cycle-06-task-04-resolution-protocol-amendment.md`
  (approved; includes superseding backend-effects operator exception).
- Ticket and comment `01M4C900000000000000000000.md` under canonical `.manyhands/` paths.

## Accepted work and milestone evidence

Tasks 0–3 accepted; rebased checkpoints: Task0 `151b0cc`, Task1 `e125f8c`, Task2
`22d1406`, Task3 `51fe6f2`. Complete mapping/evidence is in the ledger.

Task4 milestone1: independently reviewed stable anchored index sentinel,
libgit2 serialization, fenced evidence and verified release; no custom SHA-1
serializer or exchange/placeholder lifecycle. Milestone2a: exact-result path
replay without rewriting and all-side closure/comment validation; independent
review OK with notes, 70 sync/9 merge/112 authoring tests and static gates passed.
Neither is whole Task4 acceptance.

Milestone2b: seven test-only stock ref/reflog characterizations; 163 remote tests,
check/fmt/strict clippy passed; independent diagnosis review OK with notes.
Backend appends branch then HEAD before installing ref. Supplied reflogs replace
whole files and suppress implicit logging; public reads create absent log files.
Old ref + complete branch append + partial HEAD append cannot safely replay
through the previously approved API policy. This justified the owner amendment.
Candidate-HEAD separate-log authentication was still missing before milestone2c.

Report directory:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/1fe78e41-782d-4673-a7ce-258e1cd9c25e/`
Files: `task4-ref-m2b-worker.md`, `task4-ref-m2b-review.md`.

## Failed milestone2c and preserved partial changes

- Workflow: `c65ad8a8-a967-4447-89c9-38336033372e` — failed.
- Latest writer: `dcefc734-637f-432e-8955-783b74ec8206` — failed after
  **1,800,000 ms**, process terminal observed. No requested handoff; independent
  review never launched. Its predecessor `e8f91cb6-f196-4086-a30e-c9f810dabca7`
  was the successful diagnosis-only writer; resume the latest run, not that one.
- Saved output contains only timeout/recovery notice; **no milestone2c validation
  result is attributed or accepted**. Inspect retained command state/logs rather
  than guessing which checks passed or rerunning every suite.
- Partial changed files: remote `sync.rs`, `sync_tests.rs`, `state.rs`,
  `state_tests.rs`, `reservation.rs`, `reservation_tests.rs`, and exact inventory
  test `tests/repository_enablement.rs`.
- Preliminary source inspection finds `remote_resolution_ref_log_artifacts`,
  artifact prepare/lookup functions, and RefLogSnapshot/Manifest/Context helpers.
  This is scope identification, NOT correctness review or proof of completion.
- Timeout snapshot: `/tmp/manyhands-task4-m2c-timeout-f20R2v/`:
  `partial.diff`, `staged.diff`, `status.txt`, exact copies of seven source files,
  and scoped untracked archive/list. Changes preserved; no rollback attempted.
- Exact pre-milestone2c source baseline:
  `/tmp/manyhands-task4-m2b-reviewed-7t3NUH/source/`.
  Its reviewed diff/status also reside in that directory.
- Earlier m2a source baseline: `/tmp/manyhands-task4-m2a-reviewed-5w6HFi/source/`.
- Retained writer directory:
  `/tmp/pi-subagents-uid-1000/async-subagent-runs/dcefc734-637f-432e-8955-783b74ec8206/`.
- Retained writer session:
  `/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/2026-10-07T00-12-42-774Z_01a113b4-8f96-759b-9c4a-9d29d0c614bd/9c4c3621-9513-418b-bf4a-300766693390/run-0/session.jsonl`.
- Mission: `4a659fc5-fb84-4a0d-bc70-e79d4562fc56`.

No Cargo/Rust process was found with ticket cwd in the narrow safe process check.
This does not establish absence of every detached job. Unrelated MCP processes
were left untouched. Never dump process command lines/environment: a credential
was exposed earlier and must not be repeated or persisted.

## Original next-session sequence — superseded by current pause above

1. Inspect exact latest writer status and retained commands; ensure one writer and
   no outstanding owned validation job. Do not silently switch execution protocol.
2. Resume the retained writer for a concise recovery handoff FIRST: inventory
   completed/incomplete seams, actual command results/logs and remaining fixes.
   Existing partial diff is already preserved. Same-protocol recovery is allowed;
   fallback to another role/runner is not automatic authorization.
3. Continue only the scoped ref/log evidence/classification seam. Artifact-bind
   immutable baseline/result/signature authority; mutable SQLite digests alone
   must not authorize effects. Use nonmutating no-follow observations, stock
   explicit-signature normal updates and expected-old exclusion. Do not use
   supplied whole reflogs or invent legacy evidence.
4. Run the smallest meaningful changed-seam regressions first; attribute already
   completed checks and avoid duplicate broad runs on an unchanged tree. Obtain
   fresh read-only independent review against pre-milestone2c source. At most
   three fix-review rounds; escalate genuine policy/feasibility gaps.
5. Then native helpers, actual child death without Drop/storage-ordering tests,
   discovery-contention diagnosis and final whole-Task4 review against `51fe6f2`.
   Only after Task4 acceptance take a scoped local checkpoint and begin Task5.

**Owner timing guidance:** 30 minutes is insufficient for implementation plus
validation; a full suite alone takes 15–20 minutes or more. Use targeted suites
while iterating. For a combined implementation/validation resume, choose a longer
budget (e.g. 90 minutes) with a meaningful early checkpoint/handoff. Write that
handoff before long validation starts and allow appropriate per-tool timeouts.
Budget broad final validation separately; do not remove mandatory final checks.

All Rust commands through `devenv shell -- cargo ...`. Final required gates:
locked all-feature check, fmt check, strict all-target/all-feature clippy,
all-feature locked tests, and CLI smoke. Full-suite contention previously failed
`services_sharing_a_corrupt_cache_replace_it_once`; an isolated retry is not a
full-suite pass. Native Windows/macOS evidence remains pending; no CI dispatch is
authorized and Linux success is not a native waiver.

Execution remains paused for the owner’s fresh session. No recovery writer or
validation job was launched after the pause request.
