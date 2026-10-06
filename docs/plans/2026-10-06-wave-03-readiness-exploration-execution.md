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

## Task state

| Task | State | Implementation range / evidence | Independent review | Next action |
| --- | --- | --- | --- | --- |
| 1 preflight/baseline | complete | Six commands passed at `a416d83`, unchanged source/lockfile; evidence below | READY (`c0df9b7e`) | No repeat broad checks until Rust/dependencies change. |
| 2 API inventory | complete for provisional baseline | Inventory `83572f7`; fixes `6d4e631..0906c3e`, sole-file correction | READY / OK (`b91fbf14`), exact committed-range diff inspected | Final-main re-audit remains before W3 Cycle 01; Task 3 may start. |
| 3 candidate/graph | complete; pinned candidate stopped | `9060ac1..44d2990`, report-only; graph/source evidence below | BLOCKED-CANDIDATE / report OK (`fe6ed76c`) | Approved source-only fallback assessment. |
| 4 fixture/session | not executed: preservation stop | none; no probe/test dependency retained | not applicable | Requires a separately accepted candidate/path before executable continuation. |
| 5 native host | not executed: preservation stop | none | not applicable | No workaround, core patch or alternative version authorized. |
| 6 native evidence | not executed: preservation stop | no editor-native evidence | not applicable | Native/fidelity/performance obligations remain unverified. |
| 7 recommendation/fallback | dispatching source-only assessment | pinned Velotype source/report only | pending | Identify exact extraction/compatibility/preservation costs; checkpoint before execution. |
| 8 verification/handoff | pending | none | pending | Final checks, whole-branch review; keep ticket open. |

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

Task 7's already-approved source-only Velotype assessment is next. Tasks 4–6
are not executed, not called passing or unnecessary native acceptance. A
candidate-version change, fork/core adaptation, extraction or product-scope
change requires the prescribed new user decision.

## Verification policy

Task reports record command, exit, head/tree/lockfile, full log reference,
expected/observed result and limits. Do not duplicate broad checks on an
unchanged verified Rust tree. Source-only API inventory does not call services
or create registry state. Tests before behavior changes; data loss is evidence
to stop, not normalize away.
