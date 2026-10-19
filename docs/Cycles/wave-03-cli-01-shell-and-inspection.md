---
title: "Wave 03 C1: CLI Shell And Inspection"
date: 2026-10-18
status: "implemented, review-ready"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M58XQ59NBC37ENGH0BA9X40Q"
---

# Wave 03 C1: CLI Shell And Inspection

## Status

**Implemented, review-ready on 2026-10-19.** The final Linux Devenv gate at
`309c9e071994f55eb0b1fa9f44546cc2722d0398` passed: **3,468 tests/cases passed,
0 failed, 1 intentionally ignored**, including 11 doc tests. CLI and bounded
Wayland desktop launch evidence, every acceptance criterion, prior gate
resolutions and platform limitations are recorded in the
[execution ledger](../plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#task-8-final-verification-and-review-ready-checkpoint--2026-10-19).
**Independent final reviews approved (controller handoff, 2026-10-19):**
whole-branch spec re-review **PASS** at `2b84360` after P2/P3; whole-branch
quality **PASS** on `309c9e0`, with **no Critical, Important or Minor findings**.
The reviewer also confirmed spec PASS for decision 14's test-only baseline
reload, with all four assertions unchanged and no production change. See the
[final approval record](../plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md#final-independent-review-approvals--2026-10-19).
Task 8 is complete. The ticket remains **open**; these code-review approvals
do not authorize merge or closure. Native CI is post-merge on main and has
not been dispatched.

**Approved by the owner on 2026-10-19**, together with the design and the
implementation plan, for subagent-driven execution. The owner answered the planning questions on
2026-10-18 (decisions 1–13 below). The design and the implementation plan are
companions:

- [Design](../plans/2026-10-18-wave-03-cli-01-shell-and-inspection-design.md)
- [Implementation plan](../plans/2026-10-18-wave-03-cli-01-shell-and-inspection-implementation.md)

Ticket: `01M4CC0VMSZ7BRS740C8VSQ5ZK`.

## Purpose And Authority

Make existing state inspectable through the CLI without enabling mutation
commands prematurely ([Wave 03](../Waves/wave-03-dogfooding.md#c1-cli-shell-and-inspection)).

Authority, in order:

1. The Wave 03 document: C1's scope, exclusions and exit evidence, and track
   rules 1–7.
2. The [CLI contract RFC](../RFC/cli-contract.md), as amended by this Cycle.
3. The [ticket relationships and short codes RFC](../RFC/ticket-relationships-and-short-codes.md)
   for the relationship verbs.
4. The F1 Cycle and design: the read services, DTOs, result codes,
   redaction, schemas and golden-fixture conventions C1 consumes unchanged.

## Entry Evidence And Dependency Boundary

- Wave 02 Cycles 01–04 are closed on main.
- F1 is closed on main. C1 consumes only F1's read services.
- F2 is also merged. C1 uses nothing from it except that its result codes
  and recovery actions already exist in the registry.
- The branch was brought up to `origin/main` at `f6a4c06` on 2026-10-18.
- Entry-gate item 3 requires the API audit to be refreshed for the
  capabilities this Cycle consumes. Task 0 of the plan refreshes it against
  the implementation baseline and records the result in
  `docs/research/wave-03-api-audit.md`.
- Entry-gate item 7 requires the Rust checks to pass at the implementation
  baseline. Task 0 runs them.

## Scope

- **Command parsing.** Every command in the CLI RFC is recognized, using
  `clap`'s builder API (decision 1). Global options are accepted before or
  after the command. Unknown commands and options are usage errors.
- **Help, version and the bare call.** No arguments, `--help` and
  `--version` exit zero without starting the Git transport or opening the
  data directory. Help lists every command and says which are available now
  and which later Cycle delivers the rest. Under `--json`, help and version
  are JSON results (decision 4).
- **Read verbs.** Every F1 read the RFC names gets a verb:
  - repository: `repo list`, `repo inspect`, `repo identity`, `remote list`;
  - credentials: `key list`, `key show`, `key public`, `host list`,
    `host inspect`;
  - content: `document list`, `document show`, `folder list`, `ticket list`,
    `ticket show`, `comment list`;
  - status: `index status`, `poll status`, `operation list`,
    `operation show`;
  - relationships: `ticket ready`, `ticket blocked`, `ticket deps`,
    `ticket children`, `ticket cycles`, `ticket plan`,
    `ticket critical-path`, `ticket find`;
  - utility: `id new`.
- **By-path inspection.** `document show` and `ticket show` take `--path`
  (with an optional `--context`) as well as `--id`, so a nonconforming file
  can be inspected (decision 5).
- **Output.** Every verb has human output and a JSON v1 envelope built from
  the F1 DTOs. In JSON mode exactly one object goes to stdout. In human mode
  results go to stdout and failures to stderr (decision 8).
- **Exit statuses.** Each result maps to the RFC's exit class: 0 success,
  2 input, 3 blocked, 4 incomplete, 5 transient, 1 internal, 130 cancelled.
- **Two result codes.** `usage_error` and `command_unavailable`, both of the
  input class (decision 3).
- **Data directory override.** `MANYHANDS_DATA_DIR` selects the data
  directory (decision 2).
- **Startup.** The Git transport is still initialized before any thread or
  native Git activity, for every command that opens services. A handled
  startup failure becomes an envelope when `--json` is recognizable.

## Explicit Exclusions And Downstream Obligations

- No canonical, configuration or key writes; no network attempts; no request
  replay or confirmation; no prompts; no desktop views; no polling.
- `--input`, `--request-id`, `--prepare` and `--confirm` are not accepted by
  any C1 verb; supplying one is a usage error. `--non-interactive` is
  accepted and has no effect yet.
- `path observe` and `ticket check` move to C3, which owns the repair and
  authoring flows they feed (decision 5).
- F2's commands `request show`, `request cancel` and `host observe` arrive
  with C2, and `operation abandon` with C4 (decision 12). C1 lists them as
  planned.
- `index status` on a degraded index fails with `index_unavailable` through
  the CLI, because every repository-scoped command resolves its repository
  first and resolution needs a readable index. F1's `unavailable` status DTO
  is reachable only by a caller that resolved earlier (ruling R-2).
- `conflict show` stays deferred to the first of C4 or D5, as F1 decided.
- `index refresh` and `index rebuild` are C3's. A read reports a stale or
  never-refreshed index and does not refresh it.
- No change under `src/repository/` except the owner-approved no-action arm
  in `src/repository/mutation/outcome.rs` (decision 9) and the later, separately
  approved test-only baseline reload (decision 14). Any other library gap
  found during C1 becomes its own ticket under track rule 5.
- Native evidence: C1 is proven on Linux. Its test target joins the native
  workflow's list, and native CI runs on main after merge.

## Required Contract

1. **Grammar.** `manyhands-cli [--json] [--non-interactive] <resource> <verb>
   [options]`, with the per-verb options in the design. Read inputs are
   flags; `--input` is for mutations only (decision 6).
2. **Selectors.** `--id`, `--key-id` and `--operation-id` accept only an exact
   canonical ULID. A short code, a lowercase ULID or anything else is
   `invalid_id`.
3. **Repository.** `--repo` is required for repository-scoped verbs and
   refused for application-global ones: `repo list`, `key *`, `host *` and
   `id new` (decision 7 for `host inspect`). A relative path resolves against
   the invocation directory; F1 resolution does the rest.
4. **Envelope.** Every handled outcome under `--json` writes exactly one
   UTF-8 JSON object and a newline to stdout, including usage errors when
   `--json` can be recognized. `command` is the canonical two-word command;
   help, version and an unnameable usage error use `cli help`,
   `cli version` and `cli usage`.
5. **Codes.** An unknown command or option, or a malformed option value, is
   `usage_error`. A command the RFC approves but a later Cycle delivers is
   `command_unavailable`; its human output names that Cycle, and its JSON
   `message` is the code's fixed message. Both are of the input class,
   exit 2, with no recovery action.
6. **Never printed.** No secret, credential, backend error text, SQL or
   `Debug` rendering, in either mode. Remote locations are redacted by F1.
   Requested bodies and sources are printed only by `show`.
7. **No side effects beyond F1's.** A C1 command may create and migrate the
   application data directory, as F1 accepted. It writes nothing in a
   repository and leaves no process behind.
8. **Kind of item.** `document show` of a ticket's ID, or `ticket show` of a
   document's, is `item_not_found` (decision 10).

## Acceptance And Exit Evidence

- Every C1 verb runs through the real binary against a real fixture
  repository, in both modes, with a golden envelope and its schema check.
- Every exit class a C1 command can produce is tested, including each F1
  failure code reachable from the CLI and the two new codes.
- Help, version and the bare call open no data directory and do not start
  the transport. A no-argument `cargo run --bin manyhands-cli` still exits
  zero.
- `--json` is honored on usage errors.
- A short code passed as `--id` is `invalid_id`.
- Planted secrets, including key-material sentinels, never appear in stdout
  or stderr.
- The platform data directory is never touched by tests. The one test of the
  platform default runs only on Linux and macOS, where it can be redirected
  (ruling R-3).
- The binary has no display or desktop dependency, shown by a `cargo tree`
  check that fails if `gpui-kit` is in the CLI's normal dependency graph.
- No process outlives the command: the CLI spawns no thread or child, and
  each test child's output pipes reach end of file when it exits.
- A transport-start failure cannot be produced through the real binary; the
  startup-order unit test with a fake is the evidence (ruling R-4).
- The CLI RFC, the Wave document and the API audit carry the amendments
  listed in the design.

## Product-Owner Decisions (2026-10-18)

1. **Parser.** Add `clap`, using its builder API, with its errors mapped to
   our own result and exit status.
2. **Data directory.** The CLI honors `MANYHANDS_DATA_DIR`, documented as an
   advanced and testing override; otherwise the platform default.
3. **Usage codes.** Add `usage_error` and `command_unavailable` in C1.
4. **JSON help.** Under `--json`, help and version are JSON results.
5. **Extra reads.** Add `folder list` and `--path` on `document show` and
   `ticket show`. `path observe` and `ticket check` move to C3.
6. **Read inputs.** Reads take flags; `--input` stays for mutations.
7. **`host inspect`.** No `--repo`; the CLI RFC's line saying `--repo`
   identifies a repository for an observation is amended.
8. **Human streams.** Results to stdout, failures to stderr; JSON always to
   stdout.
9. **Rule 5 exception.** Adding the two codes requires a new line in the
   exhaustive recovery `match` in `src/repository/mutation/outcome.rs`. The
   owner approved making that one edit inside C1 rather than in a separate
   library ticket. The exception covers only that line.
10. **Kind mismatch.** Approved with the design: a `show` verb of the wrong
    kind is `item_not_found`.
11. **Named test edits 3–6.** The two codes and two schemas must satisfy
    existing contract tests that require every code and schema to have a
    golden or a listed reason. The owner approved:
    - `golden_envelopes` in `tests/mutation_contract.rs` also scanning
      `tests/fixtures/cli_v1`;
    - `DATA_ROOTS` gaining the two CLI schemas;
    - `CODES_NO_MUTATION_GOLDEN_HAS` gaining both codes with a reason;
    - `contract_enumerations` in `tests/read_contract.rs` tying the new
      schema enumerations to Rust constants.
12. **F2's commands.** `request show`, `request cancel` and `host observe`
    arrive with C2; `operation abandon` with C4. C1 catalogs them as planned
    and the CLI RFC gains them.
13. **Help for planned commands.** Name, summary and delivering Cycle only.
    JSON help gives a planned command `"repository": null` and
    `"options": []`; each later Cycle publishes its own rules.

## Rulings (2026-10-18, after the independent review)

- **R-1.** The transport-start ruling in the review record stands. The Wave's
  clarification row is amended to say so.
- **R-2.** `index status` on a degraded index is `index_unavailable` through
  the CLI. It is accepted as CLI behavior, not raised as a library change: the
  recovery it offers, `index.rebuild`, is the right next step. Cost if wrong:
  a library ticket to resolve without a readable index.
- **R-3.** On Windows, the `directories` crate finds the local application
  data folder through the Windows API and ignores `LOCALAPPDATA`. The
  platform-default test is therefore compiled only on Linux and macOS.
  Windows coverage of the default location is an inspection item.
- **R-4.** `initialize_git_transport_before_threads` fails only if libgit2
  rejects its own timeout options, which no test can arrange through the
  binary. The handled failure path is proven with the fake `Startup`.
- **R-5.** The CLI child cannot be isolated from libgit2's system-level
  configuration (`/etc/gitconfig`, `%PROGRAMDATA%`), because libgit2 is
  redirected only in-process. Fixtures set a repository-local identity,
  which wins, so the effect is limited to settings no fixture relies on.

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning | Wave 03 says transport bootstrap runs "before argument handling". | Ruling | C1 parses arguments first, because parsing starts no thread and no native Git. Commands that open services initialize the transport before anything else; help, version and usage errors never start it, as the RFC requires. The pre-thread boundary is preserved. |
| Planning | The Wave text calls Cycle 03 "unmerged". | Settled | It is merged. The Wave sentence is corrected on this branch. |
| Planning | `host inspect` in the RFC takes `--repo`; F1's read is application-global. | Decision | Decision 7. |
| Planning | No result code exists for usage errors or for approved but undelivered commands. | Decision | Decisions 3 and 9. |
| Planning | Tests cannot redirect the platform data directory on Windows; on macOS it follows `HOME`. | Decision | Decision 2; ruling R-3. |
| Independent review of the three drafts (2026-10-18, at `2478f75`) | 40 findings: 6 blocking, 18 important, 16 minor. The blocking ones were existing contract tests the codes and schemas break, a Windows test that would touch real data, a vacuous `cargo tree` check, and two clap behaviors (`--help` reported as an error; `--json` swallowed after a planned verb). | Settled | Decisions 11–13, rulings R-1 to R-5, and corrections throughout the design and plan. |
| Scoped re-review of the revision (2026-10-18, at `cd5e242`) | 38 of 40 resolved, 2 partial, 10 new (2 blocking). The clap mechanism was checked against clap 4.6.7. The blocking ones: named edit 6 could not register a nested or an integer enumeration. | Settled | The help command entry gets its own schema reached by `$ref`; `cli version` data drops `schema_version`, which the envelope already carries; help wins over a clap error; no clap-required options; a repeated value option keeps its last value; temporary dead-code allows until wiring; `mutation_contract` runs in each verb task; `repository_inaccessible` is attempted through the CLI or recorded as unreachable. |

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing,
merging, closing and worktree cleanup each need their own authorization.

## Additional Product-Owner Decision (2026-10-19)

14. **Narrow test-only Rule 5 exception for the Task 8 gate.** The owner
    approved reloading the persisted record immediately after `f.record` in
    `src/repository/closure/checkpoint_tests.rs`'s
    `existing_closure_writes_no_checkpoint`, before the hook and resume. Keep
    every assertion, including final whole-record equality checking
    `updated_at`. This adds only this named test to the earlier repository
    boundary and pre-existing-test exceptions; it permits no production change
    or shared-fixture behavior change. Decisions 9 and 11 retain their original
    scope and history.

    Verify the exact test and the closure checkpoint test group with
    all features and locked dependencies through Devenv, plus formatting,
    using sufficient command timeouts. Commit this correction separately with
    the required co-author trailer. The controller retains Task 8's full-gate
    rerun; this follow-up does not run the full suite or authorize publishing,
    merging, rebasing, ticket closure or subagents.
