---
title: "Wave 03 C1: CLI Shell And Inspection"
date: 2026-10-18
status: draft
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M58XQ59NBC37ENGH0BA9X40Q"
---

# Wave 03 C1: CLI Shell And Inspection

## Status

Draft for owner review. The owner answered the planning questions on
2026-10-18 (decisions 1–10 below). The design and the implementation plan are
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
- `--input`, `--request-id`, `--prepare` and `--confirm` are recognized
  global options but are refused on every C1 verb as usage errors.
  `--non-interactive` is accepted and has no effect yet.
- `path observe` and `ticket check` move to C3, which owns the repair and
  authoring flows they feed (decision 5).
- `conflict show` stays deferred to the first of C4 or D5, as F1 decided.
- `index refresh` and `index rebuild` are C3's. A read reports a stale or
  never-refreshed index and does not refresh it.
- No change under `src/repository/` except the one owner-approved line in
  `src/repository/mutation/outcome.rs` (decision 9). Any other library gap
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
   `command_unavailable`, and its message names that Cycle. Both are of the
   input class, exit 2, with no recovery action.
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
- Planted secrets never appear in stdout or stderr.
- The platform data directory is never touched by tests.
- The binary has no display or desktop dependency. No process outlives the
  command.
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

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning | Wave 03 says transport bootstrap runs "before argument handling". | Ruling | C1 parses arguments first, because parsing starts no thread and no native Git. Commands that open services initialize the transport before anything else; help, version and usage errors never start it, as the RFC requires. The pre-thread boundary is preserved. |
| Planning | The Wave text calls Cycle 03 "unmerged". | Settled | It is merged. The Wave sentence is corrected on this branch. |
| Planning | `host inspect` in the RFC takes `--repo`; F1's read is application-global. | Decision | Decision 7. |
| Planning | No result code exists for usage errors or for approved but undelivered commands. | Decision | Decisions 3 and 9. |
| Planning | Tests cannot redirect the platform data directory on macOS or Windows. | Decision | Decision 2. |

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing,
merging, closing and worktree cleanup each need their own authorization.
