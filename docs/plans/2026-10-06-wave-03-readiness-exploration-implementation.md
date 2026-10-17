---
title: "Wave 03 Readiness Exploration Implementation Plan"
date: 2026-10-06
status: approved
author: "Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M48S808QSMSGWP5B2NQ65AQC"
---

# Wave 03 Readiness Exploration Implementation Plan

> The user approved the exploration contract, design and plan on 2026-10-06
> at reviewed revision `aa51f96`, then explicitly authorized execution using
> subagent-driven development. Execute only within the approved boundaries;
> publication, merge, closure and worktree cleanup remain unauthorized.

**Goal:** Obtain reproducible editor feasibility evidence and a complete
provisional CLI/desktop API inventory without starting Wave 03 implementation.

**Contract:** [Exploration](../research/wave-03-readiness-exploration.md).
**Design:** [Architecture and decision audit](2026-10-06-wave-03-readiness-exploration-design.md).
**Ticket:** [01M48S808PF2D8ZWVYM918RK2M](../../.manyhands/tickets/01M48S808PF2D8ZWVYM918RK2M/ticket.md).

**Execution method:** User-authorized subagent-driven development, with bounded
fresh implementers and independent fresh read-only review after each task.
API inventory and fixture preparation are independent of the candidate build
and can advance while build/native access is blocked. Keep one writer at a time
in the canonical ticket worktree; do not run competing Cargo writers or mutate
other Wave 02 worktrees. The controller owns integration, decisions and ticket
checkpoints. See the [execution ledger](2026-10-06-wave-03-readiness-exploration-execution.md).

## Running-spike amendment (2026-10-06)

After the initial reviewed negative-result handoff, the user explicitly asks to
assume byte changes acceptable **only for this spike**, learn adoption/integration
cost and see representative repo docs running. This is execution authorization,
not a product ruling. The [running-spike amendment](2026-10-06-wave-03-readiness-running-editor-spike.md)
governs resumed S1–S4, including the temporary preservation-stop exception,
scratch-only repo documents, actual pinned editor host and renewed Rust/native
verification. Do not rewrite original goldens or claim R3/R4 production fidelity
passes. Original approval/task records below remain historical; all other
adaptation, resource, production and lifecycle boundaries remain in force.

## Authorization and baseline ledger

| Item | Recorded state |
| --- | --- |
| Current user authorization | All three documents approved on 2026-10-06 at reviewed revision `aa51f96`; user subsequently authorized execution using subagent-driven development. Publication, merge, closure, worktree cleanup and excluded adaptations remain unauthorized. |
| User scope decision | Editor plus provisional API audit. |
| Adaptation boundary | Small host adapters only; stop before GPUI upgrades, vendoring, forks or editor-core rewrites. |
| Fallback boundary | Source-assess Velotype after Zorite blockage; approval before extraction/executable evaluation. |
| Ticket branch | `manyhands/ticket/01M48S808PF2D8ZWVYM918RK2M`. |
| Ticket worktree | `.manyhands/worktrees/01M48S808PF2D8ZWVYM918RK2M`. |
| Fetched base | `29f3f5a25957836a8513cba8a318306e1b928063`. |
| Planning rebase | Before/after HEAD equal base; in-worktree no-op rebase and ancestry passed. |
| Grounding done | Read Wave/RFCs, schema, existing source/exports, lockfile, CLI/desktop scaffold and workflow; inspect published upstream documentation only. |
| Rust/runtime verification | Not run during documentation-only planning; all executable evidence below is pending. |
| Known platform qualification | Workflow is `workflow_dispatch` only; Cycle 04 native failures were deferred by the user. Do not claim automatic/native CI success. |

## Global constraints

- Work on this ticket branch/worktree only; repeat fresh fetch/rebase/ancestry
  before later execution. Leave main's `.superpowers/`, `devenv.nix~` and every
  other worktree untouched. Re-audit a changed baseline instead of trusting
  today's API/lockfile assessment.
- Do not create a numbered readiness Cycle or amend any Wave/RFC gate.
- Keep one root package, no direct GPUI dependency and no domain/library GPUI
  types. Use the opt-in example/test described in the design; preserve default
  headless builds and production entry points.
- Probe data is synthetic. No real repos/remotes/keys, API execution,
  credential prompts, network-loaded content or production file saves.
- Keep source intact on parse/measurement failure. Stop on any excluded change
  rather than work around it silently. Failed or untested cases remain visible.
- Local Rust commands always use `devenv shell -- cargo ...`. Dependency
  resolution is the only intentional unlocked Cargo step; run all subsequent
  check/test/smoke commands with `--locked`, except the mandated fmt invocation.
- Record coherent task checkpoints and comments. Never mark a pending test or
  native matrix as passed. Worktree deletion, publishing and ticket closure
  remain later authorizations; ordinary fixture-owned temporary-file teardown
  is allowed only during an authorized probe run.

## Task 1 — Repeat preflight and establish runnable baseline

**Files:** this plan's ledger, ticket and new canonical ticket comment.

1. Inspect ticket identity, branch, registered worktree and both statuses.
2. Fetch authoritative main; inspect any local-main divergence; rebase this
   ticket from inside its clean worktree and verify ancestry. Preserve user
   edits; no automatic stash/reset/clean. Record base and old/new HEAD.
3. Re-read changed requirements/lockfile and current Wave 02 completion evidence.
   No unfinished Wave 02 feature is a dependency of the editor probe.
4. Run baseline mandated Rust checks, headless smoke and default headless
   check before adding dependencies:

   ```sh
   devenv shell -- cargo check --all-features --locked
   devenv shell -- cargo fmt --check
   devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
   devenv shell -- cargo test --all-features --locked
   devenv shell -- cargo check --no-default-features --bin manyhands-cli --locked
   devenv shell -- cargo run --locked --bin manyhands-cli
   ```

5. Record actual display/OS/architecture and CLI result. Desktop launch occurs
   only with an active display; existing scaffold behavior does not prove the
   future probe's Root or input contract. If a relevant baseline check fails,
   classify it and resolve/obtain a decision before dependent executable work;
   source-only audit can continue.

**Checkpoint:** base, commands/exits, baseline failures and platform availability.
No fix to unrelated domain/CI code is part of this task.

## Task 2 — Inventory actual APIs and ownership

**Files:** `docs/research/wave-03-api-audit.md` (new managed document); ticket comment.

1. Read the CLI taxonomy and desktop/runtime RFC sections into a checklist.
   Expand grouped taxonomy entries to one row per verb. Add desktop-only draft,
   focus/editor, scheduler and shutdown capabilities rather than pretending the
   CLI list is the whole desktop.
2. Read `src/{canonical,repository,runtime}.rs` and
   `src/repository/{keys,transport,remote,discovery,recovery}` plus relevant
   authoring modules and tests. Follow public re-exports to their implementations.
3. Fill the design's eight-column coverage contract: symbols/types/tests;
   target and observations; request/replay/cache loss; preflight/confirmation;
   progress/cancellation; credentials/redaction; evidence status; owner/re-audit.
4. Inspect side effects and errors, not just names. Distinguish public API from
   private/test helpers; local comment checkpoint from publication; remote
   advertisement from poll/materialization; partial outcomes from simple Result.
5. Assign missing Wave 02 behavior to its named Cycle and adapter/read additions
   to the owning early Wave 03 Cycle. Unresolved ownership blocks a consumer
   plan. Do not design an alternate journal, lease authority or direct UI SQL/Git.
6. Cross-check every checklist entry has exactly one inventory row; list
   unsupported/missing capabilities explicitly. Record inspected main SHA and
   existing tests without executing APIs or claiming prior tests ran today.

**Acceptance:** R9; no omitted verb/capability, no invented service and no
unqualified guarantee based on a method name. Explicit final-main audit required.
**Checkpoint:** coverage counts, gaps, owners and provisional Wave 02 rows.

## Task 3 — Pin the candidate and gate dependencies

**Files:** `Cargo.toml`, `Cargo.lock`,
`docs/research/wave-03-editor-feasibility.md` (new managed report), ticket comment.

1. Record published Zorite 0.10.0 manifest/source/checksum/licenses and transitive
   packages. Verify versioned EditorState/Event APIs and required providers;
   current 0.11 host docs are background only. Do not execute upstream examples.
2. Add optional exact `zorite-editor = "=0.10.0"`. Define
   `editor-probe = ["desktop", "dep:zorite-editor"]`. Register the explicit test
   target in Task 4 and example in Task 5 when their files exist, each with
   `required-features = ["editor-probe"]`; do not declare missing target paths
   before metadata resolution. No production imports/wiring.
3. Resolve the graph intentionally, inspect generated lockfile before builds:

   ```sh
   devenv shell -- cargo metadata --format-version 1 --features editor-probe
   devenv shell -- cargo tree --locked --features editor-probe -i gpui-pre@0.3.6
   devenv shell -- cargo tree --locked --features editor-probe --duplicates
   ```

   Capture/process metadata outside conversation output. Verify one resolved
   core GPUI package ID shared by Kit/editor, retained Kit 0.6.6/GPUI-pre 0.3.6,
   and no unexpected baseline dependency drift. Do not add direct `gpui` to
   make imports/tests compile. Unrelated pre-existing duplicates are not failure.
4. Inspect licenses/notices and build scripts in the new closure before build.
   Record transitive runtime side effects/resources. Download failure is blockage,
   not incompatibility. If preservation requires patches/upgrades, stop here.
5. If blocked, retain reproducible graph/diagnostic evidence in the report and
   restore only ticket-owned manifest/lockfile edits to a buildable baseline;
   proceed to source-only fallback assessment and API audit, not editor execution.

**Acceptance:** R1 dependency identity and provenance characterized; actual type
interoperability still requires Task 5 compilation.
**Checkpoint:** exact pin, graph/lockfile delta, licenses and proceed/stop decision.

## Task 4 — Golden fixtures and evidence/session contracts

**Files:** `tests/fixtures/editor_feasibility/{README.md,cases.toml,*.md}`,
`examples/editor_feasibility/{session,evidence}.rs`,
`tests/editor_feasibility.rs`, `Cargo.toml`; report and ticket comment.

1. Create synthetic full-canonical originals with fixed valid IDs and exact
   byte-sensitive cases from the design. Store CRLF deliberately; verify it
   survives checkout. Keep source and independent expected results, never
   regenerate goldens from candidate output.
2. Define case IDs, exact body boundaries, target edit byte ranges and expected
   changed-block/untouched ranges in `cases.toml`. Include empty/no-final-newline,
   Unicode/bidi, unsupported syntax, nested YAML and tables.
3. Generate a deterministic 100 KiB mixed body and one larger case from a
   documented seed/pattern; assert size/hash and no silent truncation. These
   do not claim the integrated app's 1,000-item fixture is exercised.
4. Implement pure session/evidence helpers: immutable original/header,
   generation/base, dirty draft, explicit clean replacement, UTF-8-safe range
   validation, exact byte comparison, event sequence and evidence status.
   Persist synthetic snapshots only on explicit capture into the run directory.
5. Register the explicit `editor_feasibility` test target at
   `tests/editor_feasibility.rs`, requiring `editor-probe`. Test helpers against
   independently specified counterexamples: changed untouched byte, CRLF
   normalization, stale-generation replacement, partial
   UTF-8 range, metadata change and absent candidate snapshot must fail.
   Missing evidence must never be interpreted as a passed no-op case.

   ```sh
   devenv shell -- cargo test --locked --features editor-probe --test editor_feasibility
   ```

**Acceptance:** meaningful fixture/comparator tests; **not** actual-editor/native
acceptance yet. Feature-disabled test absence is not counted as passing coverage.
**Checkpoint:** case manifest, hashes, test failures fixed and evidence limitations.

## Task 5 — Small native host and actual candidate adapter

**Files:** `examples/editor_feasibility.rs`,
`examples/editor_feasibility/{host,adapter}.rs`, `Cargo.toml`;
 tests/report/ticket comment.

1. Register the explicit `editor_feasibility` example at
   `examples/editor_feasibility.rs`, requiring `editor-probe`. Verify pinned Kit
   Root construction; initialize transport before threads,
   Kit inside app.run, Root first. Create fixture selector, rich/source toggle,
   labeled keyboard controls, status/dirty indicator and explicit capture.
2. Keep one EditorState per draft. Use versioned style toggle/readback and
   actual formatting/table/undo operations. Initial loading may use set_text;
   mode changes must not recreate/reload the entity. Audit subscriptions,
   lifetimes, selection mapping and stale events.
3. Implement the deny-by-default resource seam. If testing local resources,
   allow only explicit fixture-owned paths; reject traversal, symlink escapes,
   network URLs, private-key locations and executable schemes. Link events
   are visible requests, not automatic OS opens. Inspect internal access too.
4. Feed newer synthetic canonical state/stale completion to the session: dirty
   text survives; show explicit comparison; clean replacement is deliberate.
   No real save/draft store/network/Git behavior is added.
5. Compile and confirm actual Kit/editor type interoperability:

   ```sh
   devenv shell -- cargo check --locked --features editor-probe --example editor_feasibility
   devenv shell -- cargo run --locked --features editor-probe --example editor_feasibility
   ```

6. Where Kit exposes a supported test context, add actual EditorState readback
   edit/mode/undo tests to the explicit test target. If unavailable, emit native
   candidate-produced snapshots and action traces for Task 6 comparisons.
   Do not substitute a host String edit or enable direct GPUI test dependencies.

**Acceptance:** R1 compiled identity, R2 containment, R7 dirty seam. Any forbidden
core work or uncontrolled access stops the probe and preserves evidence.
**Checkpoint:** compiled pin, API mapping, containment results, adapter size/cost
and actual-editor versus model-test coverage.

## Task 6 — Fidelity, native input and responsiveness evidence

**Files:** fixture expectations/test target only if corrections are justified;
run artifacts under `target/editor-feasibility/<run-id>/`; report/ticket comment.

1. Run every fixture through actual load/readback and repeated rich/source
   switching. Capture before/after snapshots and assert R3 exact bytes/header.
2. For each vocabulary case, perform a native focused edit; compare independent
   expectations, untouched ranges, source switch, undo-to-original and redo.
   Include task toggles, code/quote/list edits and table cell/alignment/row/column
   operations. Describe changed-block formatting boundaries explicitly.
3. Exercise unsupported and parse-failing source: preserved usable fallback,
   visible limitations, no simplification. Source-only fallback does not satisfy
   a required rich construct. A failed CRLF/undo case is a candidate blocker,
   not permission to normalize or rebuild the editor.
4. Perform Linux Wayland keyboard-only focus/toggle/format/table/clipboard/close
   journey. Test real IME composition (record input method), Unicode/combining/
   bidi selection, rich and plain paste. Record absent IME/access as untested.
5. Perform resource negative interactions and dirty/stale completion cases,
   recording actual event/readback and inspected internal side effects.
6. Measure at least 100 local input samples after warm-up on the 100 KiB case.
   Record hardware/display, timing boundaries and p95 against <100 ms. If only
   handler timing is accessible, label it limited and keep rendered-input target
   unverified. Record opening/scrolling/larger-source usability separately.
7. Populate the native obligation table: Linux observations, Windows/macOS,
   remaining built architectures, high-DPI/accessibility, real draft/crash/save
   integration and downstream integrated performance. Missing access is not pass.
8. Stop on data loss or excluded adaptation. Record partial evidence accurately;
   do not keep extending thin adapters until they become a custom editor.

**Acceptance:** R2–R8 with exact tested/blocked/not-tested distinctions.
**Checkpoint:** case/action results, performance method and unresolved native gaps.

## Task 7 — Bounded fallback assessment and recommendation

**Files:** report/API audit, optional
`docs/research/wave-03-velotype-assessment.md`; ticket comment.

1. If Zorite meets tested boundaries, recommend adoption-with-obligations and
   list adapters, source/native gaps, maintenance/license costs and later work.
   This is not product-owner selection or production integration approval.
2. If blocked, pin Velotype to an immutable revision and inspect its licenses,
   GPUI identity, editor module/dependency closure, serializer/source-span model,
   file/window/network ownership, undo/mode/table/input behavior.
3. Name exact extraction modules and required patches, likely untouched-source
   risks and test work. Size work by named deliverables/unknowns rather than
   claiming an unsupported duration. No clone build, executable extraction,
   vendoring or patch is authorized by the fallback assessment boundary.
4. Ask the user to choose the next bounded investigation. Neither a new custom
   editor nor source-only product fallback is automatically permitted.
5. Complete R9 inventory and R10 recommendation even if executable editor work
   is blocked. Identify whether results are adoptable, require new approval,
   reject a pin, or lack enough evidence; never call an environment failure a
   definitive product incompatibility.

**Checkpoint:** recommendation, stop evidence, fallback cost and decision needed.

## Task 8 — Verification, review and handoff

**Files:** all ticket-owned artifacts, new ticket comment and ticket checklist.

1. If Rust/dependency changes remain, run the four mandated checks again with
   final lockfile, plus explicit probe tests and headless/production smokes:

   ```sh
   devenv shell -- cargo check --all-features --locked
   devenv shell -- cargo fmt --check
   devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
   devenv shell -- cargo test --all-features --locked
   devenv shell -- cargo test --locked --features editor-probe --test editor_feasibility
   devenv shell -- cargo check --no-default-features --bin manyhands-cli --locked
   devenv shell -- cargo run --locked --bin manyhands-cli
   devenv shell -- cargo run --locked --features desktop --bin manyhands
   ```

   Desktop smoke requires display and proves only the unchanged scaffold starts.
   Probe native evidence remains separately recorded. If blocked dependency
   edits were restored and only docs remain, perform documentation verification
   instead; preserve actual baseline/build failures and do not invent probe passes.
2. Validate managed frontmatter/ULIDs/global uniqueness, canonical ticket/comment
   linkage, relative links and Git whitespace. Ensure reports include evidence
   provenance and unmet requirements, and no probe result changes an RFC gate.
3. Audit every R1–R10 row to its command/action/artifact or explicit blocker.
   Re-run the design review matrix and inspect actual source/manifest diff for
   production/domain changes, secrets, direct GPUI, unsafe resources and test
   claims. Report any open question before requesting approval.
4. Present contract/design/plan and resulting evidence for code/document review.
   Mark ticket review-ready, not closed. No review approval or native CI result
   is implied. If publication is later authorized, manual CI scheduling and
   deferred failures need their own explicit evidence/decision.
5. Request editor-selection/probe-retention decisions separately from code
   approval. Closure is only after review/PR approval under explicit lifecycle
   authorization; no cleanup or production integration follows automatically.

**Final checkpoint:** check exits, evidence limitations, review-ready findings,
remaining decisions and exact downstream handoff.

## Downstream handoff checklist

- [ ] Cycle 01: repeat final-main API audit after all Wave 02 integration evidence.
- [ ] Cycles 01/02: assign missing reads/typed outcomes and replay/confirmation
  before any dependent CLI/desktop adapter planning.
- [ ] Cycle 08: real protected atomic draft store, restart/migration/error behavior.
- [ ] Cycle 09: selected editor, actual save/metadata/repair/resource policy,
  external-change handling, no-op commit proof and full native fidelity.
- [ ] Cycle 12: final transport startup/cancellation/stall characterization;
  ten-second stopping feedback is not a total process-exit promise.
- [ ] Cycle 13: all integrated journeys, real SSH, collaborator evidence and
  approved OS/architecture/keyboard/IME/performance matrix.

## Planning review result

The draft has checked scope/ownership, trust/resources, source preservation,
replay versus provisional APIs, test feasibility, GPUI/native constraints,
performance claims and lifecycle authorization. User scope questions are
answered. Material unknowns are named execution gates, not hidden assumptions:
compiled GPUI compatibility, pinned editor fidelity/undo/input, safe resource
hooks, test-context availability and actual rendered-response measurements.
No Rust probe, native journey, editor selection or final API readiness is
claimed complete by planning. All three documents are approved and the user has
explicitly authorized subagent-driven execution; record actual task evidence
in the execution ledger.
