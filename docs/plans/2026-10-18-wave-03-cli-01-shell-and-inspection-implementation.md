---
title: "Wave 03 C1 CLI Shell And Inspection Implementation Plan"
date: 2026-10-18
status: draft
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M58XQ5A2C0CEG8ZKW4YDR66J"
---

# CLI Shell And Inspection Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn `manyhands-cli` into a working read-only CLI over every F1 read, with help, JSON v1 envelopes, human output and exit statuses.

**Architecture:** A bin-private module tree under `src/bin/manyhands-cli/` builds a clap command tree from one static catalog, starts the Git transport only for commands that open services, dispatches each available verb to one F1 `RepositoryService` read, and renders the F1 envelope as JSON or human text with a class-mapped exit status. The only library change is two result codes.

**Tech Stack:** Rust 2024, clap (builder API, new), serde_json, the existing F1 read services, result model, schemas and golden-test support.

**Spec:** [Design](2026-10-18-wave-03-cli-01-shell-and-inspection-design.md), under the [Cycle](../Cycles/wave-03-cli-01-shell-and-inspection.md). Read both before any task. The Cycle governs the design, and the design governs this plan.

## Global Constraints

- Run every Rust command through Devenv: `devenv shell -- cargo <command> --locked`.
- Work only in the ticket worktree on branch `manyhands/ticket/01M4CC0VMSZ7BRS740C8VSQ5ZK`. If `origin/main` moves, `git merge origin/main`; never rebase this branch.
- Nothing CLI-specific goes into `src/lib.rs`. The CLI's code lives in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`.
- No change under `src/repository/` except the single owner-approved edit in Task 1: adding `ResultCode::UsageError` and `ResultCode::CommandUnavailable` to the no-action arm of the `match code` in `src/repository/mutation/outcome.rs`. Any other library gap stops the task: report it, and the controller raises a separate ticket under Wave 03 track rule 5.
- No network, no canonical write, no prompt, no thread before `initialize_git_transport_before_threads()`, and no process left behind.
- Tests never touch developer keys, global Git configuration or real application data. Every CLI child process gets the isolation environment from `tests/support/cli.rs` (Task 2).
- JSON messages are always `ResultCode::message()`. No backend error text, SQL, `Debug` rendering or clap text ever goes into JSON.
- Semantic edits to pre-existing tests are allowed only where this plan names them:
  - Named edit 1: `src/results_tests.rs`, `DESIGN_CODES` gains the two codes (68 to 70 entries).
  - Named edit 2: `src/results_tests.rs`, `result_code_messages_are_fixed` gains the two messages.

  Any other semantic edit to a pre-existing test stops for owner review.
- Between tasks, run only the tests relevant to the change. The full suite runs once, in Task 7.
- Implementers never dispatch subagents.
- Commit at the end of every task. Messages end with:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Never push, open a pull request, merge, close the ticket or clean up without separate owner authorization.

## Review Focus

These are inputs the spec implies but no table in it lists. Each has its test in the owning task.

1. **`--repo` spelled differently from the stored root.** A trailing slash, `.`, `..` from a subdirectory, or a path with spaces or non-ASCII characters should resolve to the same repository. Task 3, Step 4.
2. **`--repo` naming a linked item worktree.** It should resolve to its owning repository, as F1 does. Task 3, Step 4.
3. **Output cut short by the reader.** Piping a large `document show` into a reader that closes early must exit 1 without a panic message. Task 6, Step 3.
4. **`--json` given twice, or before and after the verb.** It is accepted and produces one envelope. Task 2, Step 7.
5. **`MANYHANDS_DATA_DIR` naming a regular file.** The CLI writes a failure envelope and exits nonzero without panicking. Task 2, Step 7.

---

## File Map And Task Order

| Task | Creates | Modifies |
| --- | --- | --- |
| 0 | `docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md` | `docs/research/wave-03-api-audit.md` |
| 1 | — | `src/results.rs`, `src/repository/mutation/outcome.rs`, `src/results_tests.rs`, `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json` |
| 2 | `src/bin/manyhands-cli/{catalog,parse,startup,render,human,dispatch}.rs` and their `*_tests.rs`, `tests/cli.rs`, `tests/cli/help.rs`, `tests/support/cli.rs`, `schemas/v1/cli_help.schema.json`, `schemas/v1/cli_version.schema.json`, `tests/fixtures/cli_v1/` | `Cargo.toml`, `Cargo.lock`, `src/bin/manyhands-cli.rs`, `tests/support/golden.rs`, `tests/support/mod.rs`, `.gitattributes` |
| 3 | `tests/cli/admin.rs` | `dispatch.rs`, `human.rs`, `tests/cli.rs` |
| 4 | `tests/cli/content.rs` | `dispatch.rs`, `human.rs`, `tests/cli.rs` |
| 5 | `tests/cli/relationships.rs` | `dispatch.rs`, `human.rs`, `tests/cli.rs` |
| 6 | `tests/cli/boundaries.rs` | `tests/cli.rs`, `.github/workflows/build.yml`, `docs/RFC/cli-contract.md`, `docs/Waves/wave-03-dogfooding.md` |
| 7 | — | Cycle doc, design, execution doc, ticket comment |

Tasks run in order. Tasks 3, 4 and 5 each add verbs to `dispatch.rs` and `human.rs`, so they cannot run in parallel.

---

## Task 0: Baseline, Audit Refresh And Ledger

**Files:**
- Create: `docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md`
- Modify: `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Confirm the base.** Run `git fetch origin && git merge origin/main` (only if main moved) and `git merge-tree --write-tree HEAD origin/main`. Record HEAD and `origin/main` in the execution doc.
- [ ] **Step 2: Re-check every Baseline Fact in the design** against the source, including:
  - the line of the `match code` in `src/repository/mutation/outcome.rs`;
  - the size of `DESIGN_CODES`;
  - the signature of every F1 method in the design's Dispatch table.

  Record each fact as confirmed or changed. A changed fact that alters a task stops here for a controller ruling.
- [ ] **Step 3: Choose the clap version.** Run `devenv shell -- cargo search clap --limit 1` and record the current 4.x version. Do not add it yet; Task 2 does.
- [ ] **Step 4: Run the baseline checks:**
  - `devenv shell -- cargo check --all-features --locked`
  - `devenv shell -- cargo fmt --check`
  - `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
  - `devenv shell -- cargo test --locked --test read_contract --test read_boundary`
  - `devenv shell -- cargo test --locked --lib results`

  Record the counts. The full suite is not run here.
- [ ] **Step 5: Refresh the API audit.** In `docs/research/wave-03-api-audit.md`, add a section "C1 refresh (2026-10-18)". It has one row per C1 command, giving the F1 method, its signature at this baseline, whether it uses a read session, and the result codes it can return. It ends with a note that `path observe` and `ticket check` moved to C3. Do not rewrite older rows.
- [ ] **Step 6: Create the execution doc.** Give it front matter like the design's, with a new document ULID from `id new` logic. If the CLI cannot do this yet, generate the ULID with the same bash snippet earlier Cycles used. Add sections for the baseline, the confirmed facts, the audit refresh, and an empty task log.
- [ ] **Step 7: Commit.** `docs: record C1 baseline and refresh the API audit`.

## Task 1: `usage_error` And `command_unavailable`

**Files:**
- Modify: `src/results.rs` (the `result_codes!` table, after `InvalidInput`)
- Modify: `src/repository/mutation/outcome.rs` (no-action arm of the `match code` beginning near line 719)
- Modify: `src/results_tests.rs` (named edits 1 and 2)
- Modify: `schemas/v1/envelope.schema.json` (`properties.code.enum`), `schemas/v1/request.schema.json` (`result.code` enum)

**Interfaces:**
- Produces: `ResultCode::UsageError` (`"usage_error"`, Input), `ResultCode::CommandUnavailable` (`"command_unavailable"`, Input).

- [ ] **Step 1: Make the named test edits first.** In `DESIGN_CODES`, insert these entries directly after `("invalid_input", Some(FailureClass::Input))`, and change the array length to 70:

  ```rust
  ("usage_error", Some(FailureClass::Input)),
  ("command_unavailable", Some(FailureClass::Input)),
  ```

  In `result_code_messages_are_fixed`, insert these messages at the same position:

  ```rust
  "The command line is not valid; see --help.",
  "That command is not available in this version of the CLI.",
  ```

- [ ] **Step 2: Run it and see it fail.** `devenv shell -- cargo test --locked --lib results_tests` fails: both tests report the missing codes.
- [ ] **Step 3: Add the codes** to `result_codes!` directly after `InvalidInput`:

  ```rust
      UsageError => "usage_error", Some(FailureClass::Input),
          "The command line is not valid; see --help.";
      CommandUnavailable => "command_unavailable", Some(FailureClass::Input),
          "That command is not available in this version of the CLI.";
  ```

- [ ] **Step 4: Fix the exhaustive match.** `cargo check` now fails in `src/repository/mutation/outcome.rs`. Add both codes to the no-action arm (the arm that starts `ResultCode::Ok | ResultCode::AlreadyApplied | ...`), with this comment:

  ```rust
          // A command line the CLI could not run: no action corrects it.
          | ResultCode::UsageError
          | ResultCode::CommandUnavailable
  ```

  This is the only edit allowed under `src/repository/`.
- [ ] **Step 5: Add both strings to the two schema enums**, in the same position (after `invalid_input`).
- [ ] **Step 6: Run the tests that loop over every code:**
  - `devenv shell -- cargo test --locked --lib results`
  - `devenv shell -- cargo test --locked --lib read::`
  - `devenv shell -- cargo test --locked --lib mutation::outcome`
  - `devenv shell -- cargo test --locked --test read_contract --test mutation_contract`

  All must pass with no other test edit.
- [ ] **Step 7: Commit.** `feat: add usage_error and command_unavailable result codes`.

## Task 2: CLI Skeleton, Help, Version, Usage And The Test Harness

**Files:**
- Modify: `Cargo.toml` (add `clap = { version = "<Task 0 version>", default-features = false, features = ["std", "help", "usage", "error-context"] }` under `[dependencies]`), `Cargo.lock`
- Modify: `src/bin/manyhands-cli.rs`
- Create:
  - `src/bin/manyhands-cli/catalog.rs`, `parse.rs`, `startup.rs`, `render.rs`, `human.rs`, `dispatch.rs`, each with a sibling `*_tests.rs` (`#[cfg(test)] #[path = "catalog_tests.rs"] mod tests;`);
  - `tests/support/cli.rs`;
  - `tests/cli.rs`, `tests/cli/help.rs`;
  - `schemas/v1/cli_help.schema.json`, `schemas/v1/cli_version.schema.json`;
  - `tests/fixtures/cli_v1/`.
- Modify: `tests/support/golden.rs`, `tests/support/mod.rs` (add `pub mod cli;`), `.gitattributes` (add `tests/fixtures/cli_v1/** -text`)

**Interfaces (later tasks consume these exact names):**

```rust
// catalog.rs
pub enum Availability { Available, Planned(&'static str) } // "C2".."C5"
pub enum RepoRule { Required, Refused }
pub struct OptionSpec { pub name: &'static str, pub value: Option<&'static str>,
                        pub required: bool, pub summary: &'static str }
pub struct CommandSpec { pub resource: &'static str, pub verb: &'static str,
                         pub summary: &'static str, pub availability: Availability,
                         pub repo: RepoRule, pub options: &'static [OptionSpec] }
pub const COMMANDS: &[CommandSpec];
pub fn find(resource: &str, verb: &str) -> Option<&'static CommandSpec>;

// parse.rs
pub struct Invocation { pub json: bool, pub action: Action }
pub enum Action {
    Help { command: Option<&'static CommandSpec> },
    Version,
    Usage { command: Option<&'static CommandSpec>, detail: String },
    Unavailable { command: &'static CommandSpec },
    Run { command: &'static CommandSpec, args: clap::ArgMatches },
}
pub fn parse(args: Vec<std::ffi::OsString>) -> Invocation;
pub fn json_requested(args: &[std::ffi::OsString]) -> bool; // exact "--json" before "--"

// startup.rs
pub trait Startup {
    fn start_transport(&mut self) -> Result<(), ()>;
    fn open_service(&mut self) -> Result<manyhands::repository::RepositoryService, manyhands::repository::ReadError>;
}
pub fn data_directory(var: Option<std::ffi::OsString>, cwd: &std::path::Path)
    -> Result<Option<std::path::PathBuf>, manyhands::repository::ReadError>; // None = platform default
pub fn run(invocation: Invocation, startup: &mut dyn Startup) -> Output;

// render.rs
pub struct Output { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub status: i32 }
pub fn exit_status(class: Option<manyhands::results::FailureClass>) -> i32;
pub fn envelope_output<T: serde::Serialize>(json: bool, envelope: &manyhands::results::Envelope<T>,
                                            human: impl FnOnce(&T) -> String) -> Output;
pub fn write_and_exit(output: Output) -> !;

// dispatch.rs
pub fn dispatch(command: &'static CommandSpec, args: &clap::ArgMatches, json: bool,
                service: &manyhands::repository::RepositoryService) -> Output;
```

`dispatch` returns an `Output` for `id new` only in this task. Every other available verb returns a `command_unavailable` placeholder until Tasks 3–5 replace it. A unit test lists which verbs are still placeholders, and Task 5 ends with that list empty.

- [ ] **Step 1: Write the catalog test first** (`catalog_tests.rs`). It holds the five-row table from the design's "Command Catalog" as `(cycle, &[&str])` and asserts that `COMMANDS` holds exactly those commands with those availabilities, with no duplicates. It also asserts that every `Available` command has the `RepoRule` from the design's per-verb table.
- [ ] **Step 2: Write `catalog.rs`** to pass it, with the options from the design's options table on each available command, and run `devenv shell -- cargo test --locked --bin manyhands-cli catalog`.
- [ ] **Step 3: Write `parse_tests.rs`.** Cover:
  - each per-verb required, refused and conflicting rule;
  - an unknown command or option gives `Usage`;
  - `ticket close --id X --prepare` gives `Unavailable`;
  - `--request-id` on `ticket list` gives `Usage`;
  - `--json` before and after the verb;
  - `json_requested` with `--json` after `--`, and with `--json=1`;
  - `--authority` cases `example.com`, `example.com:2222`, `[::1]`, `[::1]:22`, `host:0`, `host:70000`, `:22`, `[::1`;
  - a non-UTF-8 argument (Unix only, `#[cfg(unix)]`) gives `Usage`.

  Then write `parse.rs`:
  - Build the clap `Command` from `COMMANDS`, using `.disable_colored_help(true)`, `.color(clap::ColorChoice::Never)` and `.disable_version_flag(true)`. Handle `--version` as our own global flag.
  - Give each planned verb `Arg::new("rest").num_args(0..).trailing_var_arg(true).allow_hyphen_values(true)`.
  - Use `try_get_matches_from`. On error, `Usage` with `detail = error.render().to_string()`, where `json` is `json_requested(&args)`.
- [ ] **Step 4: Write `render_tests.rs` and then `render.rs`.**
  - `exit_status`: `None` is 0, Input 2, Blocked 3, Incomplete 4, Transient 5, Internal 1, Cancelled 130. Use one exhaustive `match`.
  - `envelope_output`: JSON mode writes `serde_json::to_string(envelope)` plus `\n` to stdout and nothing to stderr. Human mode on success writes the human text to stdout. Human mode on failure writes `error: <message>\ncode: <code>\n` and one `next: <action> <key>=<value>...` line per recovery action to stderr.
  - `write_and_exit` writes both buffers with `std::io::Write::write_all` and `flush`. Any error ends the process with `std::process::exit(1)` without writing more.
- [ ] **Step 5: Write `startup_tests.rs` and then `startup.rs`.**
  - `data_directory`: unset gives `Ok(None)`; empty gives `invalid_path`; absolute is unchanged; relative is joined to `cwd`.
  - `run`: test with a `FakeStartup` that records `"transport"` and `"service"` calls. Assert:
    - `Help`, `Version`, `Usage` and `Unavailable` record nothing;
    - `Run` records `["transport", "service"]` in that order;
    - a transport failure records only `["transport"]` and yields `internal_error` with exit 1.
  - The real `Startup` in `src/bin/manyhands-cli.rs` calls `unsafe { manyhands::runtime::initialize_git_transport_before_threads() }`. Its safety comment says no thread exists yet, because only argument parsing has run. It then opens with `open_at` or `open_default` from `data_directory`, mapping `RepositoryError` through `ReadError::from`.
- [ ] **Step 6: Help and version.**
  - Text help is clap's, for the root or one command. A planned command's help adds a line `Arrives with <Cycle>.`.
  - JSON help is a success envelope, command `cli help`, `Scope::default()`, `Effects::not_requested()`. Its data is serialized from a bin-private `HelpDto` matching the design's `cli help` shape.
  - Version is `cli version` with `{name, version: env!("CARGO_PKG_VERSION"), schema_version: manyhands::results::SCHEMA_VERSION}`; text is `manyhands-cli <version>`.
  - The bare call is root help.
  - Usage gives `Envelope::failure("cli usage" or the command, Scope::default(), ResultCode::UsageError, vec![])`. Human stderr adds `detail`.
  - Unavailable gives `Envelope::failure(<command>, ..., ResultCode::CommandUnavailable, vec![])`. Human stderr adds `<command> arrives with <Cycle>.`.
  - Write `schemas/v1/cli_help.schema.json` and `cli_version.schema.json` in the F1 schema style: closed, every property required, nullable `value` as `["string","null"]`.
- [ ] **Step 7: Build the integration harness and the first tests.**
  - `tests/support/golden.rs`: add `pub fn assert_golden_in(directory: &Path, cases: &[&str], name: &str, value: &Value, placeholders: &[(&str, &str)])`. The existing `assert_golden` calls it with `fixture_directory()` and `CASES`, unchanged in behavior. Add `pub fn cli_fixture_directory() -> PathBuf` (`tests/fixtures/cli_v1`).
  - `tests/support/cli.rs`:
    - `pub struct CliRun { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub status: i32 }`.
    - `pub fn run_cli(args: &[&OsStr], env: &CliEnv) -> CliRun`. `CliEnv` holds a `TempDir` home, a data directory `PathBuf`, and an optional current directory.
    - Apply the design's isolation list exactly, and remove `MANYHANDS_DATA_DIR` before setting it from `CliEnv`.
    - Close stdin. Fail the test after 30 seconds.
    - Also add `pub fn json(run: &CliRun) -> serde_json::Value` (asserts exactly one line on stdout and an empty stderr) and `pub const CLI_CASES: &[&str]`.
  - `tests/cli.rs` declares `mod support; mod cli { pub mod help; }` (or the `#[path]` form the repository's other multi-file test targets use; follow `tests/mutation_replay.rs`).
  - `tests/cli/help.rs` covers:
    - Root help, `ticket list --help` and `ticket close --help`, in both modes. The JSON forms are goldens `help_root`, `help_ticket_list`, `help_ticket_close`. The root text is golden `help_root_text`, compared byte for byte as a `.txt` fixture through a small `assert_text_golden` in the same support file.
    - Version in both modes: goldens `version` and `version_text`.
    - The bare call exits 0 and prints the root help text.
    - Help, version, the bare call, a usage error and an unavailable command each leave `CliEnv`'s data directory absent.
    - Usage errors: `cli usage` for `manyhands-cli frobnicate --json`, and `ticket list` for `ticket list --bogus --json`. Both exit 2, goldens `usage_unknown_command`, `usage_unknown_option`.
    - `ticket close --id 01... --prepare --json` is `command_unavailable`, exit 2, golden `unavailable_ticket_close`.
    - Help text is identical under `COLUMNS=40` and `COLUMNS=200`.
    - Review Focus 4: `--json ticket list --json --repo <r>` gives one envelope.
    - Review Focus 5: `MANYHANDS_DATA_DIR` naming a regular file, with `repo list --json`, gives one envelope, a nonzero exit and no `panicked` in stderr.
  - Every envelope is checked against `envelope.schema.json`, and its data against its own schema, with `tests/support/schema.rs`'s `check_published`.
- [ ] **Step 8: Run** `devenv shell -- cargo test --locked --bin manyhands-cli` and `devenv shell -- cargo test --locked --test cli`. Generate fixtures once with `MANYHANDS_UPDATE_GOLDEN=1`, read every generated file, then run again without the variable.
- [ ] **Step 9: Smoke test.** `devenv shell -- cargo run --locked --bin manyhands-cli` exits 0 and prints help.
- [ ] **Step 10: Run clippy and fmt** for the bin and tests, then commit: `feat: CLI shell with help, version, usage errors and exit statuses`.

## Task 3: Repository, Credential, Status And Utility Verbs

**Files:**
- Modify: `src/bin/manyhands-cli/dispatch.rs`, `human.rs`, their tests
- Create: `tests/cli/admin.rs`; modify `tests/cli.rs`

**Verbs:** `repo list`, `repo inspect`, `repo identity`, `remote list`, `key list`, `key show`, `key public`, `host list`, `host inspect`, `index status`, `poll status`, `operation list`, `operation show`. `id new` is done in Task 2.

**Interfaces:**
- Consumes: `dispatch`, `envelope_output`, `run_cli`, `json`, `assert_golden_in`.
- Produces: `fn resolve(service, args) -> Result<ResolvedRepository, ReadError>`, used by Tasks 4–5. It reads `--repo`, joins a relative path to the current directory, and calls `resolve_repository`. Also `fn item_id(args) -> Result<ItemId, ReadError>`, which maps a parse failure to `ReadError::invalid_id()`.

- [ ] **Step 1: Write the failing integration tests in `tests/cli/admin.rs`.** For each verb, run it in JSON mode against fixtures built with `tests/support` (`born_repository`, `enabled_repository`, `credentials::register_keys`, `credentials::pin_host`, the operations helpers). Assert:
  - the exit status;
  - the schema;
  - the golden, named after F1's golden of the same read with a `cli_` prefix;
  - the scope, which must equal the scope F1's golden test for that read uses in `tests/read_contract.rs` (copy it from there).

  They fail with `command_unavailable` from the Task 2 placeholders.
- [ ] **Step 2: Implement each verb** with the F1 call in the design's Dispatch table. `--key-id` uses `SharedKeyId::parse` and `--operation-id` uses `OperationId::parse`; a failure becomes `ReadError::invalid_id()`. `host inspect` builds `SshAuthority { host, port }` from the parsed `--authority`.
- [ ] **Step 3: Add failure cases** through the binary, each with its exit status and a golden:
  - `invalid_path` (missing `--repo` path);
  - `not_repository` (empty directory);
  - `not_repository_root` (a subdirectory; human stderr names the `repo.inspect` root);
  - `bare_repository`;
  - `repository_not_registered`;
  - `invalid_id` (`--key-id` and `--operation-id` with a lowercase ULID);
  - `key_not_found`, `authority_not_found`, `operation_not_found`;
  - `index_unavailable`, exit 3 (see below);
  - `busy`, with exit 5. Hold the exclusive lock in a separate process, then run `index status`. Copy the approach of the private `hold_index_lock` at `tests/read_boundary.rs:61` into `tests/support/cli.rs`. Do not move it: that would edit a pre-existing test file.
  - `index_unavailable` uses `items::degraded_service(enabled)`, which returns its own data directory. Point `MANYHANDS_DATA_DIR` at that directory.

  If a code cannot be produced through the binary, record why in the execution doc rather than leaving it untested silently.
- [ ] **Step 4: Review Focus 1 and 2.**
  - Run `repo identity --json` with each of these: `<root>/`, `.` from the root, `..` from `<root>/docs`, and a root whose path contains a space and `é`, created with `born_repository`'s directory under a renamed parent. Each succeeds with the same `scope.repository`.
  - Run it with a linked item worktree path (create one with `items::create_document_context`). It succeeds with the owner's root.
- [ ] **Step 5: Human mode.**
  - `key public` prints only the key text and a newline.
  - `id new` prints only a ULID.
  - `repo list` prints one line per registration.
  - A failure prints `error:`, `code:` and `next:` lines on stderr and nothing on stdout.

  Add unit tests in `human_tests.rs` for each renderer, built from DTOs constructed in the test.
- [ ] **Step 6: Run** `devenv shell -- cargo test --locked --bin manyhands-cli` and `--test cli`. Clippy, fmt, commit: `feat: CLI repository, credential and status reads`.

## Task 4: Content Verbs

**Files:**
- Modify: `dispatch.rs`, `human.rs`, their tests
- Create: `tests/cli/content.rs`; modify `tests/cli.rs`

**Verbs:** `document list`, `document show` (`--id` or `--path` with optional `--context`), `folder list`, `ticket list` (all filters), `ticket show` (`--id` or `--path`), `comment list`.

- [ ] **Step 1: Write the failing tests** in `tests/cli/content.rs`, using `items::contract_repository()`.
  - JSON goldens for each verb, mirroring F1's `document_list`, `document_show`, `document_show_nonconforming`, `document_folder_list`, ticket list, show and comment goldens, with a `cli_` prefix.
  - `ticket list` with each of `--closure open|closed|all`, `--status`, `--type`, `--project`, `--slug`, `--ready` and `--blocked`. Assert the IDs returned equal F1's `list_tickets` with the same `TicketFilter`, called in the test process.
  - `--ready --blocked` together is `usage_error`.
  - `document show --id <ticket ULID>` and `ticket show --id <document ULID>` are `item_not_found`, exit 2.
  - `ticket show --id mh-vh-k9x2b` (`items::SHARED_SLUG`) is `invalid_id`, exit 2.
  - `document show --path docs/absent.md` is `path_not_found`.
  - `document show --path docs/../outside.md` is `invalid_path`.
  - A stale index (change a file after refresh) gives exit 0 and `data.index.state == "stale"`. In human mode stderr has the stale note.
- [ ] **Step 2: Implement.**
  - `TicketFilter` is built from the flags, with `closure` defaulting to `ClosureFilter::All`. `--ready` and `--blocked` set `readiness`.
  - `show` uses `show_item`, or `show_path(&repo, context.as_deref(), path)`. `--context` is joined to the current directory when relative.
  - The kind check compares the DTO's `kind` with the verb. On a mismatch, return `ReadError::new(ResultCode::ItemNotFound)` with the scope `show` would have used, turned into an envelope.
- [ ] **Step 3: Human mode.**
  - Lists show one line per item: ULID, short code (or `-`), `closed` or `open` for tickets, status, title. A nonconforming line shows `nonconforming`, the path and its problem codes.
  - `show` prints `Name: value` lines, a blank line, then the body, or the source when nonconforming.
  - `comment list` indents two spaces per depth.

  Pin `ticket list` as golden text `ticket_list_text`. Unit-test the other renderers.
- [ ] **Step 4: Run** `--bin manyhands-cli` and `--test cli`. Clippy, fmt, commit: `feat: CLI document, ticket and comment reads`.

## Task 5: Relationship Verbs

**Files:**
- Modify: `dispatch.rs`, `human.rs`, their tests
- Create: `tests/cli/relationships.rs`; modify `tests/cli.rs`

**Verbs:** `ticket ready`, `ticket blocked`, `ticket deps`, `ticket children`, `ticket cycles`, `ticket plan`, `ticket critical-path`, `ticket find`.

- [ ] **Step 1: Write the failing tests** using `items::relationship_repository()`.
  - JSON goldens mirroring F1's relationship goldens (same command names), with a `cli_` prefix.
  - `ticket deps` with `--direction up|down|both` and `--depth 0|1`.
  - `--depth -1` and `--direction sideways` are `usage_error`.
  - `ticket find --slug` with the shared slug in upper case returns every match; an unknown slug and `--slug ""` return an empty list with exit 0.
  - `ticket deps --id <document ULID>` is `item_not_found`.
  - `ticket ready --status <s>` filters as F1 does.
- [ ] **Step 2: Implement** with the F1 calls in the design's Dispatch table. `ticket ready` and `ticket blocked` build a `TicketFilter` from `--status`, `--type` and `--project`, with `readiness` set to `Ready` or `Blocked`.
- [ ] **Step 3: Human mode.**
  - `deps` indents by depth and marks `(repeated)`.
  - `plan` prints `Batch N:` groups, then `Unplannable:` with reasons.
  - `blocked` prints reasons under each ticket.
  - `cycles` prints one line per cycle.

  Unit-test each renderer.
- [ ] **Step 4: Remove the placeholder list.** The unit test from Task 2 that lists placeholder verbs now asserts the list is empty. This changes a test written in this Cycle, so it is not a pre-existing-test edit.
- [ ] **Step 5: Run** `--bin manyhands-cli` and `--test cli`. Clippy, fmt, commit: `feat: CLI relationship reads`.

## Task 6: Boundaries, Workflow And Document Amendments

**Files:**
- Create: `tests/cli/boundaries.rs`; modify `tests/cli.rs`
- Modify: `.github/workflows/build.yml` (append `--test cli` to the test list on the `cargo test` line)
- Modify: `docs/RFC/cli-contract.md`, `docs/Waves/wave-03-dogfooding.md`

- [ ] **Step 1: Redaction.**
  - Build a fixture whose remote URL holds a sentinel password and query.
  - Plant stored problem guidance with a sentinel, as `tests/read_contract.rs` does.
  - Run every C1 verb in both modes against it.
  - Assert that no sentinel appears in any stdout or stderr. Use the sentinel scan helper from `tests/support/golden.rs` on JSON, and a substring search on text.
- [ ] **Step 2: No repository change.** Take `repository_and_worktree_snapshot` before and after running every C1 verb against `items::contract_repository()`, and assert they are equal.
- [ ] **Step 3: Review Focus 3, broken pipe.**
  - Create a document of about 4 MiB.
  - Spawn `document show --id <id>` with stdout piped, read 16 bytes, and drop the pipe.
  - Assert the exit status is 1 and stderr does not contain `panicked`. On Windows a closed pipe is also an error, so the same assertion holds.
- [ ] **Step 4: Platform default directory.**
  - Run `repo list --json` with `MANYHANDS_DATA_DIR` removed and the platform variables pointing at `CliEnv`'s temporary directories.
  - Assert success, and that a Manyhands data directory now exists under the temporary data location and nowhere else the test can see.
  - Gate the location assertion per platform with `cfg`: `XDG_DATA_HOME` on Linux, `HOME/Library/Application Support` on macOS, `LOCALAPPDATA` on Windows. These are the `directories` crate's documented locations.
- [ ] **Step 5: Exit-class coverage.** Add one table-driven test listing every `(command line, expected code, expected exit)` pair the suite already exercises. Assert that every input, blocked, transient and internal class reachable by a C1 command appears at least once, and that success appears.
- [ ] **Step 6: Workflow.** Add `--test cli` to the explicit test list in `.github/workflows/build.yml`. Do not dispatch the workflow.
- [ ] **Step 7: Amend the documents** as listed in the design's "Document Amendments":
  - Each amendment is a short dated note in the section it changes, for example "Amended 2026-10-18 by C1:".
  - Correct the Wave's "unmerged Cycle 03" sentence.
  - In the Wave's C1 section, add that `path observe` and `ticket check` moved to C3.
- [ ] **Step 8: Run** `--test cli`. Clippy, fmt, commit: `test: CLI boundaries; docs: C1 contract amendments`.

## Task 7: Full Verification, Review And Handoff

- [ ] **Step 1:** If `origin/main` moved, merge it. Then run the full gate:
  - `devenv shell -- cargo check --all-features --locked`
  - `devenv shell -- cargo fmt --check`
  - `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
  - `devenv shell -- cargo test --all-features --locked --no-fail-fast`

  Record the counts in the execution doc.
- [ ] **Step 2: Run the smoke tests.**
  - `devenv shell -- cargo run --locked --bin manyhands-cli` exits 0.
  - `devenv shell -- cargo tree --locked --bin manyhands-cli -e normal | grep -i gpui` prints nothing.
- [ ] **Step 3:** Run `git merge-tree --write-tree HEAD origin/main` and confirm it is clean.
- [ ] **Step 4: Update the documents.**
  - Fill in the execution doc's acceptance mapping: each Cycle acceptance line, and the test or evidence for it.
  - Set the Cycle doc's status to "implemented, review-ready".
  - Add a review-ready ticket comment.
- [ ] **Step 5: Commit.** `docs: mark C1 review-ready`.

## Plan Self-Review

- **Spec coverage.** Every Cycle scope line maps to a task:
  - parsing, help and version: Task 2;
  - read verbs: Tasks 3–5;
  - by-path show: Task 4;
  - output streams and exit statuses: Task 2, with coverage in Task 6;
  - codes: Task 1;
  - data directory: Tasks 2 and 6;
  - startup order: Task 2.

  Every acceptance line maps to a test in Tasks 2–6 or a step in Task 7. The design's amendments are in Task 6, and the audit refresh is in Task 0.
- **Placeholders.** None. Each place where a value must come from the baseline (the clap version, F1 golden scopes) names the source to copy from.
- **Names.** `run_cli`, `json`, `assert_golden_in`, `resolve`, `item_id`, `dispatch`, `envelope_output` and `exit_status` are defined in Tasks 2–3 and used with the same signatures later.
- **Review Focus.** Each of the five has its step: Task 3 Step 4 (items 1 and 2), Task 6 Step 3 (item 3), Task 2 Step 7 (items 4 and 5).
