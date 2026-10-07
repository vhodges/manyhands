---
title: "Wave 03 Readiness Exploration Execution Ledger"
date: 2026-10-06
status: in-progress
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48TSHGJ68MJV9NBK6WKPC7W"
---

# Wave 03 Readiness Exploration Execution Ledger

[Plan](2026-10-06-wave-03-readiness-exploration-implementation.md) |
[Design](2026-10-06-wave-03-readiness-exploration-design.md) |
[Ticket](../../.manyhands/tickets/01M48S808PF2D8ZWVYM918RK2M/ticket.md)

## Authority and current base

User: "Lets execute the plan using subagent driven development" (2026-10-06).
The approved plan is authorized for bounded executable exploration, using fresh
implementers and independent read-only reviewers. Publishing, PR/merge, ticket
closure, worktree cleanup, GPUI upgrades, forks/vendoring/editor-core rewrites
and executable Velotype extraction are not authorized.

- Canonical worktree: `.manyhands/worktrees/01M48S808PF2D8ZWVYM918RK2M`.
- Branch: `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`.
- Execution preflight fetched base: `29f3f5a25957836a8513cba8a318306e1b928063`.
- Before/after in-worktree rebase: `c70400a8b5abcfbfc80a49478abfdb12f2b66be4`.
- Rebase was a no-op; ancestry passed; local main and fetched origin/main agree.
- Worktree was clean; AGENTS.md unchanged. Main's unrelated `.superpowers/`
  and `devenv.nix~` and all other worktrees remain untouched.
- Environment advertises Linux Wayland (`DISPLAY=:1`, `WAYLAND_DISPLAY=wayland-1`);
  access, native input and launch remain to be verified.

## Implementation topology and lane board

**Multi-seam:** API inventory, dependency feasibility, fixture/session evidence
and native editor embedding have independently testable contracts. They have
separate bounded owners and handoffs. All implementation stays in the canonical
worktree as required by the plan; owners are serialized, never concurrent.
This worktree isolates the exploration from main and the active Wave 02 work.
No worker receives all major outcomes. The controller integrates reviewed
handoffs and owns this ledger/ticket comments between worker boundaries.

| Lane | Exact decision/owned files | Isolation/authority | Next gate and handoff | Independence |
| --- | --- | --- | --- | --- |
| baseline | Task 1 commands and environment evidence; no source edits | Canonical worktree; one gate child; Devenv only; managed log/output artifacts | Baseline report + fresh evidence review | Establishes runnable baseline, not editor implementation. |
| api-audit | Task 2 `docs/research/wave-03-api-audit.md` only | Serialized docs writer in canonical worktree; no live API calls; own-file local commit | Coverage/source review and committed inventory | Independent of editor build and unfinished Wave 02 APIs. |
| dependency | Task 3 Cargo feature/dependency/lockfile and editor report | Serialized dependency writer; no forks/upgrades/core work | Pin/graph/license review; stop report or committed handoff | Determines whether subsequent probe can legally build. |
| fixture-session | Task 4 fixtures, session/evidence helpers, test target | Serialized writer; dependency handoff required | Meaningful focused tests and fresh review | Pure preservation/evidence contract; does not claim real editor fidelity. |
| editor-host | Task 5 example/host/adapter and necessary target wiring | Serialized writer; reviewed fixture/dependency handoffs required | Type interoperability/containment checks and fresh review | Uses established evidence seam; no domain/production UI changes. |
| native-evidence | Task 6 actual editor journeys/measurement/report | Serialized gate/evidence owner; no claimed synthetic native success | Exact snapshots/actions, native limitations, independent evidence review | Measures candidate rather than implementing a product. |
| fallback | Task 7 pinned source-only Velotype assessment if blocked | Serialized report writer; no executable extraction | Recommendation review and user checkpoint | Negative findings do not authorize excluded adaptations. |
| final-integration | Task 8 controller bookkeeping/verification and fresh whole-branch review | Starts after component handoffs; no new component implementation | Requirements-to-evidence audit, final results and remaining decisions | Integrates evidence; does not absorb all feature work. |

Each child receives goal, exact cwd/ref, file authority, approved contracts,
covering checks, output/report and stop rules. Outputs/logs use managed runtime
artifacts, not scratch files in repository root. Native subagent notifications
reopen parent coordination; no polling/sleep loop or unnecessary blocking waits.

## Resumed running-spike authorization and lane board

User now requests integration/adoption cost and a running example with actual
repo documents, explicitly assuming byte changes acceptable **for this spike
only**, not making a product ruling. The
[running-spike amendment](2026-10-06-wave-03-readiness-running-editor-spike.md)
records authority, design, task ownership and acceptance. Original strict source
comparison evidence remains valid; source transformation no longer vetoes this
experiment but must never be relabeled production preservation success.

Fresh implementation-phase fetch/rebase: base `29f3f5a25957836a8513cba8a318306e1b928063`,
before/after `af380b01291d847c9661ecc1b6f958b3d0f504ec`; no-op, ancestry and clean
state verified. Main agrees, AGENTS unchanged; unrelated main edits and other
worktrees left untouched. Initial task/review commit IDs did not change.

| Stage | Exact seam / owner | State / evidence | Next gate |
| --- | --- | --- | --- |
| S1 pin + fixture/evidence | Serialized worker: Cargo feature/dependency/test registration, session/evidence, minimal synthetic fixtures/tests | complete, `d22d6f4..3f3bc11`; 9 pure tests/fmt/headless check passed | READY / OK with notes (`91d02bab`); no native claims. |
| S2 actual editor host | Separate serialized worker: example/host/adapter/catalog, example wiring, running report appendix | `41267b9..2c53127`; narrow P2 fixes `3ba5ff2..7ab466f`, final amended Rust gates pass; correction READY / OK (`d7536642`) | Both wording/count findings resolved without behavior changes; not adoption approval. |
| S3 user-visible desktop | Controller-owned actual launch and scratch observations | launched owned group `3247260`; 5 initial ActualEditorReadback captures; user confirmed Visible and working; later exit0/no signal recorded | Four repo docs equal originals on load, mixed math differs; separate unchanged desktop startup smoke done, group `3310636` stopped only. |
| S4 cost/review handoff | Controller integrates component/native evidence and fresh whole-branch review | complete; `29f3f5a..14ebcb4`, reviewer `93599d5b`: no issues, READY / OK with notes | Named adoption work/transformations/limits; no editor or product-policy selection. |

Small host adapters only; no GPUI upgrade, fork, vendoring, core rewrite or
executable Velotype extraction. No original repo-doc writes, production/draft
store, live domain/API/SSH, publication, merge, closure or cleanup. Scratch-only
native demo uses README, CLI RFC, Wave03 document and implementation plan;
canonical metadata remains outside the editable buffer. Preserve each draft
across selection/mode changes and record changed-on-load separately from edits.

### S1 accepted seam handoff

Implementer `0d8127a2-291c-4c5a-8e44-9bd4c22ce144` committed
`3f3bc1140636d627149eca9195bd3e838e4f24b7`, 16 exclusive Cargo/helper/fixture/test
files, clean and unstaged. Same reviewed three-package lock addition, identical
to prior Task3 trial; Kit 0.6.6/pre 0.3.6, syntax-only Markdown and no baseline
version/source/checksum/features/edges drift. Lock SHA-256
`51c46d1c4abb87e2ab011d8e1f4f6a58d709f8c099ab412dee8afa1201a8b5fb`.
All 83 original Task3 artifact hashes reverified read-only.

Nine focused pure-helper tests, fmt and headless CLI check passed on committed
S1. Candidate libraries compile, but no Editor entity was constructed. Initial
wrong fixture offset / absent initial observation test setup failures and CRLF
whitespace diagnostics are retained, fixed without rewriting originals/goldens
or bypassing hooks. Local fixture attributes preserve intentional CRLF and
retain other whitespace checks. Full final Rust/native gates remain S2/S3.

Fresh reviewer `91d02bab-066e-4ffd-8d5a-b841e1c5ae48` inspected exact committed
diff/source/logs: no issues, S1 READY / Merge verdict OK with notes. Parent
accepts the seam, not native evidence. Reviewer did not execute commands.
Notes carried to S2: actual readback provenance/per-doc callback routing must
be truthful; capture guards are local single-writer checks, not an OS sandbox;
initial normalization makes conservative dirty state block clean replacement.
S2 must show normalization separately from later user edits, preserve entities/
history, retain immutable metadata and never claim host-recombined metadata
round-tripped through the editor.

S1 evidence: `target/readiness-evidence/running-spike/s1/` exact task.diff,
review index/proof, graph audit, numbered/raw snapshots and numbered command
logs/meta. Workflow `9fac10dd-eb5e-46ff-869b-dbdb34353747`, outputs
`running-spike-s1/{handoff,review}.md`. Parent will dispatch the separate S2
host owner now. Earlier workflow launch rejection (missing script block) created
no child or code changes; clean `d22d6f4` verified before same-protocol retry.

### S2 accepted code and S3 first actual launch

S2 implementer `a3e098b3-2366-415c-938d-aa99b53ada40` committed only six owned
example/Cargo-registration/report files in `2c53127f656cad2db75714fc0b5d727934ab0a11`.
Exact compiled/tested tree `5cd2599d3faab428a018303a7f7529217e98beb0`, same S1
lock hash; no helper/production/domain/CLI/Devenv/CI changes or core adaptations.
Actual entity/context/render/style/action/table interfaces compile through Kit.
S2 is 638 Rust LOC including controls/tests; actual adapter 126 and catalog 66.
S1's 406 session/evidence bookkeeping LOC are separate from adoption adapters.

All final committed check/fmt/clippy/test gates, focused example check/build,
3 pure example tests, 9 probe tests, headless CLI check and CLI smoke passed.
Parent independently summed standard-harness summaries: **568** passes including
9 doctests/9 probe tests; custom SSH 15/31/66 remain separate. No native action,
IME/clipboard/rendered-p95 result follows from those commands. Two long bash
attention alerts were ordinary progressing all-feature test suites (311s then
286s), not hangs; no interruption. Command controls were unavailable, so parent
inspected retained logs/status without changing execution mode.

Fresh reviewer `80df0c32-e07e-4373-9e02-9bef1eb79d94` inspected exact committed
diff/source/logs: S2 READY / Merge verdict OK with notes. Two **valid P2** fixes
accepted: report/handoff incorrectly said 668 instead of 568; report and host
comment incorrectly implied formatting lacks Changed (Bold/Italic/Code do emit
Changed; undo/redo do not). They do not change behavior or block the already
reviewed native launch. Parent sends narrow corrections to the retained owner,
then targeted retained-reviewer follow-up; no new adapter work authorized.

Controller launched at reviewed Rust HEAD `2c53127` using Devenv with explicit
`--capture-initial`, isolated owned PID/process group **3247260**. Evidence:
`target/readiness-evidence/running-spike/s3/launch-1791317716369/`, including
launch command/head/binary/display/original-source hashes and full startup logs.
GUI remains running for user inspection; parent has not seen screen pixels or
performed keyboard/table/IME/clipboard/resource-negative/performance journeys.

Real captures: `target/editor-feasibility/native-3247260-1791317722697818257/`.
All five manifests state ActualEditorReadback; parent compared actual captured
original/candidate bytes independently. README (1790B), CLI RFC (22992B), Wave03
(38177B) and implementation plan (22438B) match on initial load. Mixed math
changes bytes despite unchanged 29B length, correctly marked normalization and
dirty, not preservation success. Original repo files still match launch hashes.
Full snapshots combine protected host header with real editor body; they do not
prove metadata traveled through the editor. Initial-readback-summary.json names
that limitation. Scope remains temporary byte tolerance, no product ruling.

### Corrected final source and user/native follow-up

Retained owner continuation `8dbc0896-eecd-415f-98dd-59551bb63bed` fixed only three
report lines/two host comment lines at `3ba5ff2..7ab466f`. Preserved all 385 prior
S2 evidence/handoff artifacts byte-for-byte. Final staged tree
`400e8dfa80f87db9784c50ce9629ff747c38d373` was tested then committed unchanged
as `7ab466ff048f60af9413a058bb2033024934e6e1`. Devenv fmt, focused example check,
all-feature check/clippy/test all exit0; fresh 568 standard and SSH 15/31/66,
no failures/ignored. Separate example/probe/headless/CLI proofs reused honestly
from identical source closure, not claimed rerun. Evidence:
`target/readiness-evidence/running-spike/s2-fixes/` and workflow
`cec192b0-9232-49d7-bda4-b197f9ae2190` bound handoff/review artifacts.

Retained reviewer `d7536642-4b50-4c46-8f28-62089ba2ccb3`: both P2 findings
resolved, no issues, S2 correction READY / OK. Latest worker attention was a
queued 240s warning for the normal 290s suite; status/transcript inspection
showed complete, no current in-flight command. No cancellation or fallback.

User answered **Visible and working** to controller's visibility/exploration
question. Record as reported basic usability, not a formal native journey or
performance test. Parent did not inspect desktop pixels/clipboard. Cross-mode
history/selection, native typing/IME/clipboard/tables, negative resources,
platform/high-DPI/accessibility and 100KiB/rendered-p95 remain unverified.

Unchanged desktop scaffold launched at `7ab466f` with Devenv desktop-only
`cargo run`, after cold feature build (78s). Actual manyhands binary observed
alive without app startup diagnostics; parent terminated **only** isolated smoke
group `3310636` using SIGTERM, not user editor `3247260`. Startup/observation/exit
proofs under `target/readiness-evidence/running-spike/s3/desktop-smoke-1791321361661/`.
Controlled termination is not normal/graceful close acceptance.

[Editor report S3/cost handoff](../research/wave-03-editor-feasibility.md#s3-native-observations-and-adoption-cost-handoff)
now records actual per-file loads, unchanged-length counterexample, host-only
metadata reconstruction, bounded adapter/prototype LOC and seven named adoption
areas. Embedding works without a port; production state/services/resource/rich
contracts/fidelity/native/performance/finished-Wave02 API evidence still cost
work. No justified time estimate or product editor ruling follows.

Further changes are report/ledger/canonical-comment bookkeeping only; final
Rust source remains exactly `7ab466f`. Fresh whole-branch review completed;
ticket is review-ready and lifecycle-open, not closed or published.

### Final whole-branch approval and delivery checkpoint

Fresh reviewer `93599d5b-0d44-44a3-a887-89c0a1d6d8a4`, workflow
`ba4b88db-f883-4325-804c-a529edf4e7fa`, inspected full exact range
`29f3f5a25957836a8513cba8a318306e1b928063..14ebcb410811716a1956af024a0802f272dc7d02`,
all44 changed paths, source/snapshots and retained evidence. **No issues found**;
running-spike final gate **READY**, report/prototype Merge verdict **OK with
notes**. Authoritative bound output:
`/home/vhodges/.pi/agent/sessions/--home-vhodges-work-src-manyhands--/subagent-artifacts/outputs/ba4b88db-f883-4325-804c-a529edf4e7fa/running-spike-final/review.md`.

Notes are material production obligations, not spike defects: R3 remains
blocked; R4/native input/tables/IME/clipboard/selection/resources/platform/
accessibility/high-DPI, native100KiB/p95 and production drafts/save/lifecycle
remain unverified. Final-main API re-audit remains required. Reviewer read
artifacts without executing tests/hash recomputation, inspecting pixels or
manipulating the app; parent validator and recorded user feedback are explicitly
bounded. No product, publishing, merge or lifecycle authority follows.

At delivery, recorded demo exit is code0/no signal at2026-10-06T21:13:37Z;
current process is gone. Earlier left-open/alive statements describe launch-time
intent/observations, not current window availability or an inferred exit cause.
No relaunch was performed. Original repo hashes remain unchanged. Prototype may
be relaunched manually using the report command; source/native provenance still
names original2c53127, not a pretend run on latest docs-only HEAD.

Final checkpoint touches only ledger/ticket/comment and exit-status report prose;
no Rust/Cargo/fixture changes or redundant Rust suite. Ticket remains lifecycle-
open/review-ready; no push/PR/merge/closure/cleanup. Final delivery validation is
saved separately from the immutable reviewer input under running-spike/final.

## Scroll-performance continuation (2026-10-07)

[Approved performance plan](2026-10-07-wave-03-editor-scroll-performance.md)
continues the same ticket. User reports README fast, Wave03 laggiest, exited
previous demo and approved release Rich/Source comparison with edit-driven
updates. Previous basic usability approval is not scrolling/performance approval.

Fresh origin/main40fc971 (31 newer commits) rebased clean8b53d25→ba750a3;
one Cargo conflict retained BOTH remote_synchronization and probe registrations.
Ancestry/clean state verified; main .superpowers/ and devenv.nix~ untouched.
Pins Kit0.6.6/pre0.3.6/Zorite0.10.0 unchanged. Old source proofs remain historical.
Mapping: 3f3bc11→b429740,2c53127→239a79a,7ab466f→9c88f93,
14ebcb4→fdc3d38,8b53d25→ba750a3. Source/API reports remain old-base provisional;
this task does not substitute for final-main API audit.

| Step | Owner/scope | State |
| --- | --- | --- |
| P1 | One serialized owner: edit-only draft update guard/hooks, optional counters, pure regressions/report; final Devenv gates and release build | complete e1fc6cb..fee2439; tested/committed tree94c833c; all gates/release passed |
| P2 | Fresh independent exact-task review; preserve notify-only undo/redo and cheap fallback honesty | reviewer132b8cc2: no issues, performance-host READY / OK with notes |
| P3 | Controller-owned release launch, same-doc Rich/Source user comparison and explicit counters | complete manual observation: user happy with scrolling;14 snapshots/doc, zero extra owned readbacks/drafts; later native edit/undo/redo+dirty smoke confirmed, not timing/full-native-suite acceptance |

Scrolling/caret/focus/blink must not copy/update drafts or cause extra host notify.
Changed covers many edits, but undo/redo only notify; supported hooks or a cheap
borrowed comparison fallback are necessary, explicitly measured, no lost edits.
Diagnostics default off, aggregate only on request, no per-frame logging/timers.
No SDK/dependency/core/production/policy/lifecycle scope expansion.

P1 worker54245563 committed only five owned files atfee2439ad7b9953f661cce4d072e87b696a25246;
final staged/committed tree94c833c5050d5f4a753998cebcad03497faf22fc. Generic
observation remains because undo/redo only notify, but borrowed equality precedes
owned readback; unchanged hints never copy/replace draft/notify Host. Non-edit
hints skip getter, cached status avoids body comparisons in render. Inactive
edits route correctly without active redraw. Counts are aggregate on explicit
Ctrl-Alt-D only, no timers/per-frame logging or automatic snapshots.

Fresh Devenv gates/CLI passed:614 standard including9doctests/9S1, SSH15/35/31/103
separate,9 focused pure example tests (six new policy simulations, not native).
Full suite724s, release482s; no failures or interrupted commands. The late
stall concern was prolonged handoff preparation after build; source was already
committed/clean. Attempted guidance found child already complete; no steer or
cancellation delivered. Workflow43f48e0c bound worker/reviewer outputs under
scroll-performance/{handoff,review}.md. Fresh reviewer132b8cc2 inspected exact
newdiff/integration/evidence, no commands/native manipulation; no issues,
READY / OK with notes for comparison, not production/publishing/lifecycle.

Controller verified release SHA256
`ee170dd61cce2538468e784cc1d47887aa614ddfb7385ea8656af0b36c99efaa`, clean fee2439
checkout and launched built binary through Devenv with --diagnostics,
unsetting ZORITE_WHEEL_DEBUG. Owned isolated PID/group3493901. Evidence:
`target/readiness-evidence/running-spike/scroll-performance/release-launch-1791335631182/`.
Actual release executable observed alive, five initial statuses emitted; no
initial capture requested. Initial originals/header/body policy remains; do
not infer native callback/performance acceptance from these startup results.

User is asked for same-Wave03 Rich/Source scroll-only intervals, Ctrl-Alt-D
before/after each interval, excluding mode/select/capture actions. Incremental
owned readbacks/draft changes/Host notify should be zero for unchanged scroll;
borrowed checks may rise, not zero GPUI work or measured frames/p95. Actual
counters/user feedback remain pending. Do not restart/automate/stop user's app.
Rust remains exactlyfee2439; further checkpoint is docs-only, no redundant gates.

Controller relaunched release PID3501399 after user acknowledged instructions
(the first launch stole focus). Frozen logs/summary under
`scroll-performance/release-relaunch-1791335947220/{observed-counters.log,observation.json}`.
User says **It felt fine** after requested Rich/Source protocol; not mode-labeled
or timed A/B. All14 snapshots/doc show owned readbacks/draft changes remain1,
no Changed/edit/capture events. Wave03 notifications/borrowed checks0→15,
active snapshot pairs6/6/1 and15/15/3 stable; chrome actions explain intervening
Host notify totals, not an attribution to scroll. No per-frame/rendered-p95 or
native edit/undo/IME/table acceptance follows. Release+host fixes changed together,
so no isolated causal claim. P1/P2/P3 bounded work delivered; ticket returns to
review-ready/lifecycle-open, remaining product/native/API gates unchanged.

Remaining planned native edit/undo/redo check then explicitly authorized by user.
Owned release launches3507533 and relaunch3508562, reviewed binary unchanged;
user confirms editor and dirty state updating correctly. Frozen observed logs
and native-edit-observation.json under scroll-performance evidence. First Wave03
snapshots1→2→5→6 owned/draft with multiple edit hooks; latest two snapshots
identical owned/draft7, Changed2/hooks1. Counters show mutation observation, not
isolated per-key coalescing or body fidelity; visible transitions are user proof.
Both exit0/no signal, no parent termination; no native automation/body captures.
Basic native history/status smoke now complete; final implementation source and
review/gate proofs unchangedfee2439. Remaining full native/production gates still
unverified, closure/publication requires explicit authorization/approval.

### Approved local closure (2026-10-07)

User now explicitly approves documents/recommendations, closing the ticket in
this branch, and local main merge. Earlier no-closure/local-merge restrictions
are superseded only for those operations. No push/PR/remote merge/cleanup or
editor adoption/fidelity policy authority follows. Ticket closes in final
pre-merge checkpoint; scratch worktree/artifacts remain retained.

Local main is3ea8c6e, with two Cycle05 publication-receipt documentation paths
added since base40fc971 and no Rust/dependency/Devenv/test/CI delta. Normal local
merge preserves both histories, reviewed fee2439 identity and upstream receipts.
No source/gate rerun needed for docs-only closure/integration; frozen614-standard,
separate SSH15/35/31/103 and9 pure example proofs remain correctly scoped. All
full-native/production/preservation/completed-main API obligations stay separate.
User-approved recommendation remains feasible Zorite experimental host, not
production editor selection; strict normalization blocker remains unresolved.
Main's unrelated .superpowers/ and devenv.nix~ are hash-preserved. No worktree
cleanup, reset, history rewrite or remote operations are performed.

## Task state

The following table records the **initial, independently reviewed stop-path
phase**. Current resumed execution is tracked in S1–S4 above, not silently
reinterpreted as passing initial native/preservation acceptance.

| Task | State | Implementation range / evidence | Independent review | Next action |
| --- | --- | --- | --- | --- |
| 1 preflight/baseline | complete | Six commands passed at `a416d83`, unchanged source/lockfile; evidence below | READY (`c0df9b7e`) | No repeat broad checks until Rust/dependencies change. |
| 2 API inventory | complete for provisional baseline | Inventory `83572f7`; fixes `6d4e631..0906c3e`, sole-file correction | READY / OK (`b91fbf14`), exact committed-range diff inspected | Final-main re-audit remains before W3 Cycle 01; Task 3 may start. |
| 3 candidate/graph | complete; pinned candidate stopped | `9060ac1..44d2990`, report-only; graph/source evidence below | BLOCKED-CANDIDATE / report OK (`fe6ed76c`) | Approved source-only fallback assessment. |
| 4 fixture/session | not executed: preservation stop | none; no probe/test dependency retained | not applicable | Requires a separately accepted candidate/path before executable continuation. |
| 5 native host | not executed: preservation stop | none | not applicable | No workaround, core patch or alternative version authorized. |
| 6 native evidence | not executed: preservation stop | no editor-native evidence | not applicable | Native/fidelity/performance obligations remain unverified. |
| 7 recommendation/fallback | complete; unchanged fallback blocked | `94672ab..f040268`, pinned source-only report | READY / report OK (`283fbe40`) | User chooses any new bounded investigation; no executable extraction. |
| 8 verification/handoff | complete at approved negative-result boundary | Docs/evidence checks passed; exact `29f3f5a..44a469a` full-branch review | READY / report OK (`4b944110`) | User reviews findings and approves any new bounded direction; ticket remains open. |

## Decisions, blockers and review dispositions

- Settled: user changed execution method from default direct execution to
  subagent-driven development; approved product/scope requirements unchanged.
- Ruling: serial component ownership in the existing canonical worktree avoids
  concurrent writers and unauthorized alternate ticket infrastructure. Cost:
  less build/write parallelism; benefit: authoritative ticket branch evidence.
- Stop: no candidate-version/GPUI upgrade or executable Velotype extraction
  without the prescribed user checkpoint.
- Unverified: pinned API/type compatibility, native fidelity/resources/input,
  supported test context and rendered-response measurement seam.
- Known qualification: native CI is manual-only; previous native failures were
  deferred. Local passing commands will not be labeled native-matrix success.

## Task 1 baseline evidence

Run `6fb049df-bd71-4275-97da-f8d82ebfc980`; independent review
`c0df9b7e-11bd-4ae1-a2fa-fbbb7ba077fa` returned READY. Tested HEAD
`a416d836a10702afeba8f25000b3aa58fbed13a2`, unchanged before/after.
Cargo.lock SHA-256 `ac2e1977c8311a607b15a4329008ffd1fba9245ac56d37f1c0b4d3c2cef7f26c`;
devenv.lock SHA-256 `770a1b63f55bed5ad23a3e1d922a7c95c473084ffa483be71ebf03f777dc64a7`.
Full logs and exact-command/exit `.meta` siblings are retained under this
worktree's ignored `target/readiness-evidence/baseline/`.

| Command (all through `devenv shell --`) | Exit | Seconds | Log |
| --- | --- | --- | --- |
| `cargo check --all-features --locked` | 0 | 105 | `01-check.log` |
| `cargo fmt --check` | 0 | 1 | `02-fmt.log` |
| `cargo clippy --all-targets --all-features --locked -- -D warnings` | 0 | 54 | `03-clippy.log` |
| `cargo test --all-features --locked` | 0 | 520 | `04-test.log` |
| `cargo check --no-default-features --bin manyhands-cli --locked` | 0 | 12 | `05-headless-check.log` |
| `cargo run --locked --bin manyhands-cli` | 0 | 24 | `06-cli-run.log` |

Tests: 559 standard-harness passes (including 9 doctests), zero failed/ignored,
plus separately reported custom SSH harness counts 15/31/66. CLI exited zero
without a window. NixOS 26.11, Linux 6.18.54, x86_64; advertised Wayland/X11
sockets exist and permit filesystem access. No actual GUI/IME/renderer or
Windows/macOS/native CI acceptance was established. Source/index remained clean.
Reports are in the session's managed artifact directory under
`outputs/a6903b69-0690-4963-be85-c86aa81cc5da/baseline/{results,review}.md`.

## Task 2 first implementation and review

Worker `4216daf2-dfc7-4b78-85c0-dc0479f9d434` committed sole-file inventory
`83572f7400981e83a381f6716f828376a42c7dff`: 56 CLI verbs + 3 invocation
capabilities, 31 desktop and 16 runtime rows; 106 total. Named source/tests are
inspected, not executed by the inventory task. Final-main re-audit remains
required after Wave 02 completion.

Reviewer `33978e7d-7ebf-4fe2-84aa-2e891d128c3d` returned BLOCK. Parent verified
all three findings against actual baseline source and accepted them:

- P1: `SaveDocumentRequest.expected_source` is optional; save/move enforcement
  must not be claimed mandatory. Assign missing-observation rejection and
  covering evidence to W3-02/04 rather than modify domain code in exploration.
- P2: `list_key_material_recovery` takes an exclusive cache guard and opens
  writable SQLite; document its read-side effects with operation-list evidence.
- P2: key-clear outcome is `AlreadyCleared`, not `NoSelection`.

Same implementer applied all three corrections in
`0906c3e3edba6cb9c779afdd9e405054be7ab6b6`, sole-file diff from `6d4e631`.
Coverage remains 106 rows; source/link/anchor/whitespace validation passed.
Reports from the first pass live under that workflow artifact directory's
`api-audit/{handoff,review}.md`. No project source/Cargo/lockfile changes occurred.

### Review delivery infrastructure blocker and same-protocol retry

Workflow `e6282a86-cd83-4c72-bcbd-93249f9d7fce`, reviewer child
`44c2bbf7-f44a-49db-9bfb-05a432250667`, failed terminal output delivery:
`Required file-only output was not produced` at its configured
`api-corrections/review.json` artifact path. Preserved reviewer output reports
ready/no findings with source references, but a failed run is not a successful
workflow gate. Task 3 never launched.

Parent inspected terminal workflow state, correction handoff, preserved review
and clean tracked/staged checkout at `0906c3e`. No partial source mutation
requires recovery. Same-protocol retry uses a fresh read-only reviewer and
ordinary Markdown file output, without the failed structured/file-only pairing.
Resuming the failed reviewer would retain that same output contract; the new
launch is a corrected-contract retry, not a change of execution mode/model or
an attempt to redo completed implementation. Do not repeat baseline/fixes.

Artifacts for the failed pass are retained under workflow
`e6282a86-cd83-4c72-bcbd-93249f9d7fce`: correction handoff and terminal workflow
receipt, plus child `44c2bbf7`'s preserved reviewer output.

Retry workflow `3755eb5f-7b1f-47b1-8a7e-f233e5198a83`, reviewer
`b91fbf14-0281-4c89-abac-ff4be4952930`, successfully delivered ordinary Markdown
and returned API correction gate READY / Merge verdict OK with no issues.
The parent supplied the exact `6d4e631..0906c3e` correction diff when requested;
SHA-256 `3b88ca71b492e595bd70c252bf62395b1e9c2afb8e98111d2de5320fc34389ef`.
Reviewer inspected that committed blast radius and current source, not merely
worker scope assertions or a clean working-tree diff. Parent accepts Task 2
for the provisional baseline; final-main audit and missing APIs remain future
obligations. Candidate dependency gate can now proceed.

## Task 3 graph/source gate and preservation stop

Implementer `0508c7d7-9836-4e67-a215-b24355bd58d7` committed
`44d2990a36382b5d6ff4cd9b178d874ed2e92b1c` from base `9060ac1`, containing only
[editor feasibility report](../research/wave-03-editor-feasibility.md).
Trial exact optional editor 0.10.0 resolved with Kit 0.6.6 and the identical
GPUI-pre 0.3.6 package ID. Only gpui-bidi 0.1.1, zorite-editor 0.10.0 and
zorite-markdown 0.9.0 were added; no existing package/feature drift, all three
MIT and no new build scripts. This is graph/source proof, not compiled editor
interoperability. The three published archives/checksums and copied sources
are retained under ignored `target/readiness-evidence/editor-dependency/`.

Source blocker: published editor `src/lib.rs:324–331,1103–1107,1151–1167`
unconditionally routes `with_text` and `set_text` through `normalize_loaded`.
Published markdown `src/syntax.rs:1040–1043` specifies that
`What if words $$E=mc^2$$ more` becomes `What if words\n$$E=mc^2$$\nmore`.
Presentation toggles do not disable loading normalization. This conflicts with
exact no-op/unsupported-source preservation. Parent confirmed the stop, then
independently inspected these source excerpts; no executed editor/native result
is claimed. R3 is blocked by source inspection; R1 compile and R2/R4–R8
executable/native evidence remain not-tested.

Four Devenv graph commands passed: intentional unlocked probe metadata, locked
inverse core tree, locked duplicate tree, and locked desktop metadata comparator.
Exact logs/exits/hash manifests and failed/corrected evidence-helper attempts
are retained; helper failures were not hidden as successful commands.

The implementer restored only trial Cargo edits to exact task-base contents.
Parent confirms no Cargo.toml/Cargo.lock diff remains. Cargo.lock SHA-256 stays
`ac2e1977c8311a607b15a4329008ffd1fba9245ac56d37f1c0b4d3c2cef7f26c`.
No Rust probe, example, test target or production change remains.

Fresh reviewer `fe6ed76c-f83b-4214-856c-31969b5ded6f` inspected exact
`9060ac1..44d2990` report diff and pinned sources; no issues, Task3 gate
BLOCKED-CANDIDATE / Merge verdict OK (report only). Parent accepts the negative
feasibility result and stop/rollback, not candidate adoption. Review artifacts:
workflow `7d0447ab-7950-4bc4-8ea6-af3f132b5b46`,
`editor-dependency/{handoff,review}.md`.

Task 7's already-approved source-only Velotype assessment followed this gate.
Tasks 4–6 were not executed, not called passing or unnecessary native acceptance.
A candidate-version change, fork/core adaptation, extraction or product-scope
change requires the prescribed new user decision.

## Task 7 source-only fallback and recommendation

Implementer `ff51d0a1-451b-4081-946b-3b4e0afa2ca9` committed only
[Velotype assessment](../research/wave-03-velotype-assessment.md), range
`94672ab7585f558d87d998ed29d8f984c85e7054..f040268ad5d0e6f57dced2ff7721f08140c94677`.
Inspected upstream pin `ed65977be94f2f2703037fcb8b6cbab2e7579571`, manifest 0.7.2.
Immutable source/tree/registry provenance, exact source snapshots, module hashes,
license qualifications and committed diff are retained under ignored
`target/readiness-evidence/velotype-assessment/`.

Source findings: CRLF normalization, whole-tree serialization and lost original
native-block spellings/final newline violate unchanged-source preservation.
Outer shared history exists but cannot recover bytes discarded before snapshots.
Upstream registry GPUI 0.2.2 is a different package ID from Kit's GPUI-pre 0.3.6;
intact editor owns save/drop/image-paste/cache/file/HTTP/URL/export behavior.
A viable extraction would require preservation/core/ownership/port design and
maintenance beyond the approved thin adapters. This is source characterization,
not an executed fidelity test, compiler failure or proof of product impossibility.

Fresh reviewer `283fbe40-7ff9-475e-9970-f25017765c29` inspected exact committed
diff and pinned sources: no issues, Task7 report READY / Merge verdict OK.
Parent accepts report/recommendation; no dependency or product fallback selected.
Worker documentation/hash/source checks passed; no candidate build, Rust command,
upstream test or live API/SSH operation was run. The upstream 818-node lock
inventory is not a resolved extracted dependency closure or full legal clearance.
Reports: workflow `1296377a-621b-4a77-8318-4758a51f8fa3`,
`velotype-assessment/{handoff,review}.md`.

Recommendation: retain both negative findings; do not extract Velotype merely
to reconfirm explicit source blockers. A smaller Zorite source-preservation
API/revision investigation is worthwhile only with a concrete immutable lead
and new approval. Alternatively authorize a separate design-only Velotype
preservation/ownership/GPUI-port investigation if fork maintenance is acceptable,
name another bounded source-only candidate, or pause for review. No actual
revision/API fix is promised and no executable adaptation follows automatically.

## Final requirements-to-evidence handoff

This matrix records the initial reviewed stop-path handoff. The resumed spike
must add its own measured results without changing these historical source
findings or interpreting byte tolerance as production acceptance.

| Requirement | Actual evidence/status | Remaining gate |
| --- | --- | --- |
| R1 pin/core graph | Zorite exact published graph fit source/metadata-inspected; Velotype source core identity differs | No compiled Kit/editor entity proof; changed candidate/port needs new approval. |
| R2 host containment | Pinned source/API/licenses inspected; Velotype direct I/O ownership identified | No native containment negatives; extracted policy/closure unresolved. |
| R3 no-op bytes | Both pinned candidates have source-inspected preservation blockers | No workaround authorized; actual-editor snapshots not executed. |
| R4 edits/undo | Source paths characterized; Velotype snapshots cannot restore already-lost original bytes | No executable focused-edit/cross-mode undo evidence. |
| R5 rich vocabulary | Candidate API/module surfaces characterized only | No native per-construct/table/keyboard acceptance. |
| R6 native usability | Baseline Linux sockets inspected only | No editor launch/IME/clipboard/native matrix proof. |
| R7 dirty seam | Host implementation stopped before Task 4 | No production draft store or toy-host preservation claim. |
| R8 responsiveness | Not measured | No rendered-input p95/native proof; final performance target unchanged. |
| R9 API inventory | 106 provisional source rows, corrections independently reviewed | Repeat against completed Wave 02 main before W3 Cycle 01. |
| R10 recommendation | Both source reports independently reviewed; costs and next approval named | User selects any further bounded investigation; not adoption. |

## Task 8 verification and delivery boundary

All retained branch changes are canonical Markdown: planning/authorization,
API inventory, pinned editor reports, execution ledger, ticket and comments.
No changes remain to Cargo.toml/Cargo.lock, src, tests, Devenv or CI compared to
baseline `29f3f5a`. Required Rust baseline commands already passed on the
identical retained Rust/lockfile tree; do not rerun them for docs-only bookkeeping.
No desktop/probe/native smoke was performed because the editor lane stopped
before host implementation. Baseline CLI smoke is distinct from editor evidence.

Controller `target/readiness-evidence/final/doc-check.cjs` passed (exit 0):
restricted scalar frontmatter/required fields/ULID uniqueness, relative links
and line/heading anchors, Markdown/Git whitespace, unchanged baseline
Cargo/Devenv/source/test/CI tree, plus **all 83 Task3 and 873 Task7 evidence file
hashes** independently recomputed. Results are in `final/doc-validation.json`
and `final/doc-validation.stderr`. This is documentation/evidence validation,
not general YAML parsing through the domain parser or actual editor testing.
A final rerun accompanies the complete committed handoff. No effective Git hook
was configured; only conventional sample hook files existed at validation time.

Fresh final reviewer `4b944110-052a-4b97-a994-dede2f66c605` inspected the complete
2,920-line exact `29f3f5a25957836a8513cba8a318306e1b928063..44a469a38da57fc97ff27679b164cf83c64daa63`
committed diff and consequential API/pinned-source claims: **no issues, Task8
READY / Merge verdict OK** (report/branch content only). Parent accepts the
gate. Workflow `5d8ca8e9-a770-49c7-be09-90db8e39182d`, output
`readiness-final/review.md`; a preserved local copy is
`target/readiness-evidence/final/whole-branch-review.md`. Reviewer inspected
validation scripts/logs but did not execute commands or independently rerun
hashes, domain parsing, Rust or native editor tests. Exact submitted diff,
indexed line intervals, hashes, head/tree/branch and clean state are in
`final/{whole-branch.diff,review-index.json,review-proof.json}`.

The subsequent delivery-only amendment records this accepted review and ticket
checkpoint; it does not change the research findings, recommendation or scope.
Final review does not authorize publishing,
merging, ticket closure, cleanup, candidate selection or excluded adaptations.
At that initial handoff the ticket was review-ready and lifecycle-open; its
next material decision was a new bounded investigation direction, not a claim
that Wave 03 was implementable or complete. The user has now authorized the
running-spike continuation recorded above. The ticket is in-progress again;
the already-approved RFC/Wave gates remain unchanged.

## Verification policy

Task reports record command, exit, head/tree/lockfile, full log reference,
expected/observed result and limits. Do not duplicate broad checks on an
unchanged verified Rust tree. Source-only API inventory does not call services
or create registry state. Tests before behavior changes; data loss is evidence
to stop, not normalize away.
