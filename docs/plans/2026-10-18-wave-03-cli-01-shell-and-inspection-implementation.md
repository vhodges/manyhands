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

**Architecture:** A bin-private module tree under `src/bin/manyhands-cli/` builds a clap command tree from one static catalog. It starts the Git transport only for commands that open services, dispatches each available verb to one F1 `RepositoryService` read, and renders the F1 envelope as JSON or human text with a class-mapped exit status. The only library change is two result codes.

**Tech Stack:** Rust 2024; clap (builder API, new); serde_json; the existing F1 read services, result model, schemas and golden-test support.

**Spec:** the [Design](2026-10-18-wave-03-cli-01-shell-and-inspection-design.md), under the [Cycle](../Cycles/wave-03-cli-01-shell-and-inspection.md). Read both before any task. The Cycle governs the design, and the design governs this plan.

## Global Constraints

- **Devenv.** Run every Rust command through Devenv: `devenv shell -- cargo <command> --locked`. The one exception is Task 1 Step 1, which adds clap to `Cargo.lock`.
- **Branch.** Work only in the ticket worktree, on branch `manyhands/ticket/01M4CC0VMSZ7BRS740C8VSQ5ZK`. If `origin/main` moves, run `git merge origin/main`. Never rebase this branch.
- **Library.** Nothing CLI-specific goes into `src/lib.rs`. The CLI's code lives in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`.
- **`src/repository/`.** No change there except one owner-approved edit in Task 3: adding `ResultCode::UsageError` and `ResultCode::CommandUnavailable` to the no-action arm of the `match code` in `src/repository/mutation/outcome.rs`. Any other library gap stops the task. Report it, and the controller raises a separate ticket under Wave 03 track rule 5.
- **Runtime limits.** No network, no canonical write, no prompt, no thread before `initialize_git_transport_before_threads()`, and no process left behind.
- **Test isolation.** Tests never touch developer keys, global Git configuration or real application data. Every CLI child process gets the isolation environment from `tests/support/cli.rs` (Task 3).
- **JSON messages.** A JSON `message` is always `ResultCode::message()`. No backend error text, SQL, `Debug` rendering or clap text ever goes into JSON.
- **Pre-existing tests.** A semantic edit to a pre-existing test is allowed only where this plan names it. Any other one stops for owner review.
  - Named edit 1: `src/results_tests.rs` `DESIGN_CODES` gains the two codes (68 entries become 70), and its doc comment gains a sentence about them.
  - Named edit 2: `src/results_tests.rs` `result_code_messages_are_fixed` gains the two messages.
  - Named edit 3: `tests/mutation_contract.rs` `golden_envelopes()` also reads the JSON cases of `tests/fixtures/cli_v1`.
  - Named edit 4: `tests/mutation_contract.rs` `DATA_ROOTS` gains `cli_help.schema.json` and `cli_version.schema.json` (44 entries become 46).
  - Named edit 5: `tests/mutation_contract.rs` `CODES_NO_MUTATION_GOLDEN_HAS` gains both codes, with the reason `"a CLI parse result; no command of execute or prepare returns it"` (4 entries become 6).
  - Named edit 6: `tests/read_contract.rs` `contract_enumerations()` gains one entry, `("cli_help_command.schema.json", "repository", CLI_REPOSITORY_VALUES)`.
- **Test runs.** Between tasks, run only the tests relevant to the change. The full suite runs once, in Task 8.
- **No subagents.** Implementers never dispatch subagents.
- **Commits.** Commit at the end of every task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Authorization.** Never push, open a pull request, merge, close the ticket or clean up without separate owner authorization.

## Review Focus

These are inputs the spec implies but no table in it lists. Each has its test in the owning task.

1. **`--repo` spelled differently from the stored root.** A trailing slash, `.`, `..` from a subdirectory, or a path with spaces or non-ASCII characters should all resolve to the same repository. Task 4, Step 4.
2. **`--repo` naming a linked item worktree.** It should resolve to its owning repository, as F1 does. Task 4, Step 4.
3. **Output cut short by the reader.** Piping a large `document show` into a reader that closes early exits 1 with no panic message. Task 7, Step 3.
4. **`--json` given twice, or both before and after the verb.** It is accepted and produces one envelope. Task 1, Step 4 (parse) and Task 3, Step 6 (binary).
5. **`MANYHANDS_DATA_DIR` naming a regular file.** The CLI gives `internal_error`, exit 1, one envelope, and no panic. Task 3, Step 6.

---

## File Map And Task Order

| Task | Creates | Modifies |
| --- | --- | --- |
| 0 | `docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md` | `docs/research/wave-03-api-audit.md` |
| 1 | `src/bin/manyhands-cli/{catalog,parse}.rs`, `{catalog,parse}_tests.rs` | `Cargo.toml`, `Cargo.lock`, `src/bin/manyhands-cli.rs` |
| 2 | `src/bin/manyhands-cli/{startup,render,human,dispatch}.rs`, their `*_tests.rs` | `src/bin/manyhands-cli.rs` |
| 3 | `tests/support/cli.rs`, `tests/cli.rs`, `tests/cli/help.rs`, `schemas/v1/cli_help.schema.json`, `cli_help_command.schema.json`, `cli_help_option.schema.json`, `cli_version.schema.json`, `tests/fixtures/cli_v1/` | `src/results.rs`, `src/repository/mutation/outcome.rs`, `src/results_tests.rs`, `schemas/v1/envelope.schema.json`, `schemas/v1/request.schema.json`, `tests/mutation_contract.rs`, `tests/read_contract.rs`, `tests/support/mod.rs`, `.gitattributes`, bin modules |
| 4 | `tests/cli/admin.rs` | `dispatch.rs`, `human.rs`, `tests/cli.rs`, `tests/support/cli.rs` |
| 5 | `tests/cli/content.rs` | `dispatch.rs`, `human.rs`, `tests/cli.rs` |
| 6 | `tests/cli/relationships.rs` | `dispatch.rs`, `human.rs`, `tests/cli.rs` |
| 7 | `tests/cli/boundaries.rs` | `tests/cli.rs`, `.github/workflows/build.yml`, `docs/RFC/cli-contract.md`, `docs/Waves/wave-03-dogfooding.md` |
| 8 | — | Cycle doc, design, execution doc, ticket comment |

Tasks run in order. Tasks 4–6 each add verbs to `dispatch.rs` and `human.rs`, so they cannot run in parallel.

The two codes land in Task 3, together with the CLI goldens that are their evidence in the shared contract tests. Until then the bin uses `ResultCode::InvalidInput` in their place, behind two constants in `render.rs` (`USAGE_CODE`, `UNAVAILABLE_CODE`). Task 3 points those constants at the new codes. So no commit leaves a contract test red.

---

## Task 0: Baseline, Audit Refresh And Ledger

**Files:**
- Create: `docs/plans/2026-10-18-wave-03-cli-01-shell-and-inspection-execution.md`
- Modify: `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Confirm the base.** If `origin/main` moved, merge it. Run `git merge-tree --write-tree HEAD origin/main`. Record HEAD and `origin/main` in the execution doc.
- [ ] **Step 2: Re-check every Baseline Fact in the design** against the source. Record each one as confirmed or changed. That includes:
  - every line number the design names;
  - the sizes of `DESIGN_CODES`, `DATA_ROOTS` and `CODES_NO_MUTATION_GOLDEN_HAS`;
  - the signature of every F1 method in the design's Dispatch table.

  A changed fact that alters a task stops here for a controller ruling.
- [ ] **Step 3: Run the baseline checks.**
  - `devenv shell -- cargo check --all-features --locked`
  - `devenv shell -- cargo fmt --check`
  - `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
  - `devenv shell -- cargo test --locked --test read_contract --test mutation_contract --test read_boundary`
  - `devenv shell -- cargo test --locked --lib results::tests`

  Record the counts.
- [ ] **Step 4: Refresh the API audit.** In `docs/research/wave-03-api-audit.md`, add a section titled "Wave 03 CLI Cycle C1 refresh (2026-10-18)". The audit already uses `C0`–`C6` as its own section codes, so the title must not be just "C1". In that section:
  - For each C1 command, update its existing row in the audit's CLI inventory in place: the F1 method, its signature at this baseline, whether it opens a read session, and the result codes it can return. Do not add a second row for the same verb.
  - Note that `path observe` and `ticket check` moved to C3.
  - Note that F2's `request show`, `request cancel`, `host observe` and `operation abandon` were added as planned commands.
- [ ] **Step 5: Create the execution doc.**
  - Give it front matter like the design's, with a new document ULID. Generate the ULID with the bash snippet earlier Cycles used.
  - Add sections for:
    - the baseline;
    - the confirmed facts;
    - the audit refresh;
    - the planning review: 40 findings at `2478f75`, and their resolution as Cycle decisions 11–13, rulings R-1 to R-5 and corrections;
    - an empty task log.
- [ ] **Step 6: Commit.** `docs: record C1 baseline and refresh the API audit`.

## Task 1: Clap, Catalog And Parsing

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`, `src/bin/manyhands-cli.rs` (add `mod catalog; mod parse;` and `#![allow(clippy::result_large_err)]` with the comment `src/repository/read/mod.rs` uses)
- Create: `src/bin/manyhands-cli/catalog.rs`, `catalog_tests.rs`, `parse.rs`, `parse_tests.rs`. Each `*_tests.rs` is included with `#[cfg(test)] #[path = "catalog_tests.rs"] mod tests;`.

**Interfaces (later tasks use these exact names):**

```rust
// catalog.rs
pub enum Availability { Available, Planned(&'static str) } // "C2".."C5"
pub enum RepoRule { Required, Refused }
pub struct OptionSpec { pub name: &'static str, pub value: Option<&'static str>,
                        pub required: bool, pub summary: &'static str }
pub struct CommandSpec {
    pub resource: &'static str, pub verb: &'static str, pub summary: &'static str,
    pub availability: Availability,
    pub repo: Option<RepoRule>,            // None for a planned command
    pub options: &'static [OptionSpec],    // empty for a planned command
}
pub const COMMANDS: &[CommandSpec];
pub fn find(resource: &str, verb: &str) -> Option<&'static CommandSpec>;
pub fn resource_commands(resource: &str) -> Vec<&'static CommandSpec>;
pub const REPOSITORY_VALUES: &[&str] = &["required", "refused"]; // RepoRule as JSON help writes it; Task 2 renders through it

// parse.rs
pub struct Invocation { pub json: bool, pub action: Action }
pub enum HelpTarget { Root, Resource(&'static str), Command(&'static CommandSpec) }
pub enum Action {
    Help(HelpTarget),
    Version,
    Usage { command: Option<&'static CommandSpec>, detail: String },
    Unavailable { command: &'static CommandSpec },
    Run { command: &'static CommandSpec, args: clap::ArgMatches },
}
pub fn parse(args: Vec<std::ffi::OsString>) -> Invocation;
pub fn json_requested(args: &[std::ffi::OsString]) -> bool; // exact "--json" before "--"
pub fn help_requested(args: &[std::ffi::OsString]) -> bool; // exact "--help" before "--"
pub fn authority(text: &str) -> Result<(String, u16), String>; // brackets stripped
```

- [ ] **Step 1: Add clap.**
  1. Add `clap = { version = "4", default-features = false, features = ["std", "help", "usage", "error-context"] }` to `[dependencies]`.
  2. Run `devenv shell -- cargo check` once without `--locked`, to update `Cargo.lock`.
  3. Run `git diff Cargo.lock`. The only additions may be `clap`, `clap_builder`, `clap_lex` and the dependencies they bring. Nothing already present may change version.
  4. Record the resolved version in the execution doc.
  5. Run `devenv shell -- cargo tree --locked -e normal --no-default-features -i clap` and record what depends on it.

  From here on, every command uses `--locked` again.
- [ ] **Step 2: Write `catalog_tests.rs`.** It holds the design's five-row "Command Catalog" table as `(cycle, &[&str])`. Assert that:
  - `COMMANDS` holds exactly those commands, with those availabilities and no duplicates;
  - every `Available` command has `repo: Some(..)` matching the design's per-verb table;
  - every `Planned` command has `repo: None` and no options.

  Run `devenv shell -- cargo test --locked --bin manyhands-cli catalog`. It fails.
- [ ] **Step 3: Write `catalog.rs`.** Use the options from the design's options table. Run the tests again; they pass.
- [ ] **Step 4: Write `parse_tests.rs`.**
  - **Per-verb rules:** every required, refused and conflicting rule.
  - **Action order:** the order in the design's "Parsing" section:
    - `[]` gives `Help(Root)`;
    - `["ticket"]` gives `Help(Resource("ticket"))`;
    - `["ticket","--help"]` gives `Help(Resource("ticket"))`;
    - `["ticket","list","--help","--json"]` gives `Help(Command(ticket list))` with `json` true;
    - `["--version"]` gives `Version`.
  - **Planned verbs:**
    - `["ticket","close","--id","X","--prepare","--json"]` gives `Unavailable` with `json` true;
    - `["ticket","close","--id","X","--help"]` gives `Help(Command(ticket close))`.
  - **Usage errors:**
    - `["frobnicate"]` gives `Usage` with `command: None`;
    - `["ticket","list","--bogus"]` gives `Usage` with `command: Some(ticket list)`;
    - `["ticket","list","--request-id","X"]` gives `Usage`.
  - **Repeated `--json`:** `["--json","ticket","list","--repo","r","--json"]` gives `Run` with `json` true (Review Focus 4).
  - **Help wins over a clap error:** `["frobnicate","--help"]` gives `Help(Root)`; `["ticket","deps","--help"]` gives `Help(Command(ticket deps))`, not a missing-argument error.
  - **Repeated value option:** `["ticket","show","--repo","r","--id","A","--id","B"]` gives `Run` whose `--id` is `B`.
  - **Raw scans:** `json_requested` with `--json` after `--`, and with `--json=1`, is false.
  - **`authority`:**
    - `example.com` gives `("example.com",22)`;
    - `example.com:2222` gives `(…,2222)`;
    - `[::1]` gives `("::1",22)`;
    - `[::1]:22` gives `("::1",22)`;
    - each of `host:0`, `host:70000`, `:22`, `[::1`, `::1` and an empty string is an error.
  - **Non-UTF-8:** on Unix (`#[cfg(unix)]`), a non-UTF-8 argument gives `Usage`.
- [ ] **Step 5: Write `parse.rs`.** Build the clap `Command` from `COMMANDS`:
  - The root has:
    - `.bin_name("manyhands-cli")`;
    - `.disable_help_flag(true)`, `.disable_help_subcommand(true)` and `.disable_version_flag(true)`;
    - `.args_override_self(true)` and `.subcommand_required(false)`;
    - global `--json`, `--non-interactive`, `--help` and `--version`, each `ArgAction::SetTrue` with `.global(true)`.
  - Each resource is a subcommand with `.subcommand_required(false)` and the same `disable_*` settings.
  - Each available verb declares its options from the catalog.
  - Each planned verb declares one `Arg::new("rest").num_args(0..).trailing_var_arg(true).allow_hyphen_values(true)`.
  - Do not call any colour method; without the `color` feature they do not exist.
  - Use `try_get_matches_from`. Decide the action in the design's order.
  - Set `json` as `get_flag("json") || json_requested(&args)` on every path. Detect help as `get_flag("help") || help_requested(&args)`.
  - On a clap error, `Usage { command, detail: error.render().to_string() }`, unless `help_requested(&args)`: then help wins, for the target the raw scan names. `command` comes from the first two arguments that do not start with `-`, looked up in the catalog.
  - Never mark an option `.required(true)`; the per-verb rules check required options after parsing.

  Run `devenv shell -- cargo test --locked --bin manyhands-cli`.
- [ ] **Step 6: Leave `main` alone.** `main` still does only what it did before; Task 2 wires it. Until then `catalog` and `parse` are unreachable from `main`, so put `#[allow(dead_code)] // wired into main in Task 2` on both `mod` declarations, and `#[allow(dead_code)] // used by host inspect in Task 4` on `parse::authority`. Task 2 Step 4 and Task 4 Step 2 remove them. Run `devenv shell -- cargo clippy --locked --bin manyhands-cli -- -D warnings` and `devenv shell -- cargo fmt --check`.
- [ ] **Step 7: Commit.** `feat: CLI command catalog and parser`.

## Task 2: Startup, Rendering And Main

**Files:**
- Create: `src/bin/manyhands-cli/startup.rs`, `render.rs`, `human.rs`, `dispatch.rs` and their `*_tests.rs`
- Modify: `src/bin/manyhands-cli.rs`

**Interfaces:**

```rust
// render.rs
pub const USAGE_CODE: ResultCode = ResultCode::InvalidInput;       // Task 3 changes it
pub const UNAVAILABLE_CODE: ResultCode = ResultCode::InvalidInput; // Task 3 changes it
pub struct Human { pub stdout: String, pub notes: Vec<String> }
pub struct Output { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub status: i32 }
pub fn exit_status(class: Option<manyhands::results::FailureClass>) -> i32;
pub fn envelope_output<T: serde::Serialize>(json: bool, envelope: &manyhands::results::Envelope<T>,
                                            human: impl FnOnce(&T) -> Human) -> Output;
pub fn failure_output(json: bool, envelope: &manyhands::results::Envelope<serde_json::Value>,
                      extra: Option<String>) -> Output; // extra = clap detail or "arrives with C3"
pub fn write_and_exit(output: Output) -> !;

// startup.rs
pub trait Startup {
    fn start_transport(&mut self) -> Result<(), ()>;
    fn open_service(&mut self) -> Result<RepositoryService, ReadError>;
}
pub fn data_directory(var: Option<std::ffi::OsString>, cwd: &std::path::Path)
    -> Result<Option<std::path::PathBuf>, ReadError>; // Ok(None) = platform default
pub fn run(invocation: Invocation, startup: &mut dyn Startup) -> Output;

// dispatch.rs
pub fn dispatch(command: &'static CommandSpec, args: &clap::ArgMatches, json: bool,
                service: &RepositoryService) -> Output;
pub fn placeholder_verbs() -> Vec<&'static str>; // available verbs not yet dispatched
```

- [ ] **Step 1: Write `render_tests.rs`, then `render.rs`.**
  - **`exit_status`:** `None` is 0, Input 2, Blocked 3, Incomplete 4, Transient 5, Internal 1, Cancelled 130. Use one `match` with no wildcard arm.
  - **`envelope_output` in JSON mode:** `serde_json::to_string(envelope)` plus `\n` on stdout, nothing on stderr.
  - **`envelope_output` in human mode, success:** `Human.stdout` to stdout. Each note goes to stderr as `note: <text>\n`.
  - **`envelope_output` in human mode, failure:** stderr gets `error: <message>\ncode: <code>\n`, then one `next: <action> <key>=<value>...` line per recovery action.
  - **`failure_output`:** human mode adds `extra` on its own line after `code:`. JSON mode ignores it.
  - **`write_and_exit`:** writes both buffers with `write_all` and `flush`. On any error it calls `std::process::exit(1)` without writing more.
- [ ] **Step 2: Write `startup_tests.rs`, then `startup.rs`.**
  - **`data_directory`:** unset gives `Ok(None)`. Empty gives `invalid_path`. An absolute path is returned as given. A relative path is joined to `cwd`.
  - **`run`:** drive it with a `FakeStartup` that records `"transport"` and `"service"`. Assert that:
    - `Help`, `Version`, `Usage` and `Unavailable` record nothing;
    - `Run` records `["transport", "service"]` in that order;
    - a transport failure records only `["transport"]` and yields `internal_error` with exit 1, and human stderr includes `Git transport initialization failed`.
  - **Help output:**
    - Text help is clap's rendered help for the root, a resource or an available command.
    - A planned command's text help is its name, summary and `Arrives with <Cycle>.`.
    - JSON help is a success envelope with command `cli help`, `Scope::default()`, `Effects::not_requested()`, and data from a bin-private `HelpDto`/`HelpCommandDto`/`HelpOptionDto` shaped as in the design. A command's `repository` is written through `catalog::REPOSITORY_VALUES` (`Required` as index 0, `Refused` as index 1), or null for a planned command.
  - **Version output:** command `cli version`, data `{name: "manyhands-cli", version: env!("CARGO_PKG_VERSION")}` (the envelope already carries `schema_version`). Text is `manyhands-cli <version>`.
  - **Usage and unavailable:** `Envelope::failure(<command or "cli usage">, Scope::default(), USAGE_CODE, vec![])` with the clap detail as `extra`. Unavailable uses `UNAVAILABLE_CODE` with `extra = "<command> arrives with <Cycle>."`.
- [ ] **Step 3: Write `dispatch.rs`.** Implement `id new` only (`service.new_item_id()`, human: the ID and a newline). Every other available verb returns a `failure_output` with `UNAVAILABLE_CODE` for now, and is listed by `placeholder_verbs()`. A unit test asserts `placeholder_verbs()` lists exactly the available verbs other than `id new`. Tasks 4–6 shrink that list, and Task 6 asserts it is empty.
- [ ] **Step 4: Write `main`.** It does exactly this: (and remove the `#[allow(dead_code)]` Task 1 put on the `mod catalog;` and `mod parse;` declarations)
  1. `let args: Vec<OsString> = std::env::args_os().collect();`
  2. `let invocation = parse::parse(args);`
  3. `startup::run(invocation, &mut RealStartup)`, then `write_and_exit`.

  `RealStartup::start_transport` calls `unsafe { manyhands::runtime::initialize_git_transport_before_threads() }`. Its safety comment says only argument parsing has run, and that starts no thread. `RealStartup::open_service` uses `data_directory(std::env::var_os("MANYHANDS_DATA_DIR"), &std::env::current_dir()?)`, then `RepositoryService::open_at` or `open_default`, mapping `RepositoryError` through `ReadError::from`. A `current_dir` failure is `internal_error`.
- [ ] **Step 5: Run the checks.**
  - `devenv shell -- cargo test --locked --bin manyhands-cli`
  - Smoke test: `devenv shell -- cargo run --locked --bin manyhands-cli` exits 0 and prints help.
  - Clippy and fmt for the bin.
- [ ] **Step 6: Commit.** `feat: CLI startup, rendering, help and version`.

## Task 3: Result Codes, Schemas, The Test Harness And First Goldens

**Files:**
- Modify:
  - `src/results.rs` (the `result_codes!` table, after `InvalidInput`)
  - `src/repository/mutation/outcome.rs` (the no-action arm of the `match code` near line 719)
  - `src/results_tests.rs` (named edits 1 and 2)
  - `schemas/v1/envelope.schema.json` (`properties.code.enum`) and `schemas/v1/request.schema.json` (the `result.code` enum)
  - `tests/mutation_contract.rs` (named edits 3, 4 and 5)
  - `tests/read_contract.rs` (named edit 6)
  - `tests/support/mod.rs` (`pub mod cli;`)
  - `.gitattributes` (`tests/fixtures/cli_v1/** -text`)
  - `src/bin/manyhands-cli/render.rs` (the two constants)
- Create:
  - `schemas/v1/cli_help.schema.json`, `cli_help_command.schema.json`, `cli_help_option.schema.json` and `cli_version.schema.json`
  - `tests/support/cli.rs`
  - `tests/cli.rs` and `tests/cli/help.rs`
  - `tests/fixtures/cli_v1/`

**Interfaces:**

```rust
// tests/support/cli.rs
pub struct CliEnv { /* TempDir home, data: PathBuf, current_dir: Option<PathBuf> */ }
impl CliEnv { pub fn new(data: PathBuf) -> Self; pub fn in_dir(self, dir: PathBuf) -> Self;
              pub fn without_data_dir(self) -> Self; }
pub struct CliRun { pub stdout: Vec<u8>, pub stderr: Vec<u8>, pub status: i32 }
pub fn cli_command(args: &[&OsStr], env: &CliEnv) -> std::process::Command; // isolated, unstarted
pub fn run_cli(args: &[&OsStr], env: &CliEnv) -> CliRun; // 30 s timeout; asserts EOF on both pipes
pub fn json(run: &CliRun) -> serde_json::Value; // exactly one stdout line, empty stderr
pub fn assert_cli_golden(name: &str, value: &Value, placeholders: &[(&str, &str)]);
pub fn assert_cli_text_golden(name: &str, text: &str, placeholders: &[(&str, &str)]);
pub fn cli_fixture_directory() -> PathBuf; // tests/fixtures/cli_v1
pub const CLI_CASES: &[&str];
pub const CLI_TEXT_CASES: &[&str];
pub const CLI_REPOSITORY_VALUES: &[&str] = &["required", "refused"]; // the registry for named edit 6
pub fn born_repository_in(parent: &Path, name: &str) -> TestRepository;
```

- [ ] **Step 1: Make named edits 1 and 2 first.**
  - In `DESIGN_CODES`, insert these entries directly after `("invalid_input", Some(FailureClass::Input))`, and change the length to 70:

    ```rust
    ("usage_error", Some(FailureClass::Input)),
    ("command_unavailable", Some(FailureClass::Input)),
    ```

  - Add a sentence to its doc comment: the two CLI codes follow `invalid_input`.
  - In `result_code_messages_are_fixed`, insert `"The command line is not valid; see --help."` and `"That command is not available in this version of the CLI."` at the same position.
  - Run `devenv shell -- cargo test --locked --lib results::tests`. Both tests fail.
- [ ] **Step 2: Add the codes** directly after `InvalidInput` in `result_codes!`:

  ```rust
      UsageError => "usage_error", Some(FailureClass::Input),
          "The command line is not valid; see --help.";
      CommandUnavailable => "command_unavailable", Some(FailureClass::Input),
          "That command is not available in this version of the CLI.";
  ```

  `cargo check` now fails in `src/repository/mutation/outcome.rs`. Add both codes to the no-action arm (the arm that starts `ResultCode::Ok | ResultCode::AlreadyApplied`), with this comment:

  ```rust
          // A command line the CLI could not run: no action corrects it.
          | ResultCode::UsageError
          | ResultCode::CommandUnavailable
  ```

  This is the only edit allowed under `src/repository/`. Add both strings to the two schema `code` enums, after `invalid_input`. Then set `render::USAGE_CODE = ResultCode::UsageError` and `render::UNAVAILABLE_CODE = ResultCode::CommandUnavailable`.
- [ ] **Step 3: Write the four schemas** in the F1 style. All are closed, with every property required.
  - `cli_help.schema.json` is the data root. Its `commands` array's `items` is a `$ref` to `cli_help_command.schema.json`.
  - `cli_help_command.schema.json` has `command`, `available`, `delivered_by` and `summary`. Its `repository` is `{"type": ["string","null"], "enum": ["required","refused",null]}`. Its `options` array's `items` is a `$ref` to `cli_help_option.schema.json`.
  - `cli_help_option.schema.json` has `name`, `value` (`["string","null"]`), `required` and `summary`.
  - `cli_version.schema.json` has `name` and `version`, both strings.

  The enumeration helper in `tests/read_contract.rs` walks only `properties`, not an array's `items`. That is why the command entry is its own schema.
- [ ] **Step 4: Write `tests/support/cli.rs`** to the interface above.
  - The isolation list is the design's, exactly. Remove `MANYHANDS_DATA_DIR` before setting it from `CliEnv`.
  - Close stdin.
  - Read stdout and stderr on two threads in the test process, then `wait`. After `wait`, both readers must reach end of file within one second; that is the "no process outlives the command" check.
  - Fail after 30 seconds.
  - Write the golden functions the way `tests/support/mutation_golden.rs` does, reusing `golden::render` and `golden::update_requested`. Do not edit `golden.rs`.
  - `born_repository_in` builds a repository the same way `born_repository` does (`tests/support/mod.rs:292`), under a caller-chosen directory.
- [ ] **Step 5: Write `tests/cli.rs`.**
  - It declares `mod support;` and the `tests/cli/*.rs` modules. Follow the pattern `tests/mutation_replay.rs` uses for its directory of modules.
  - Add a test that `cli_fixture_directory()` holds exactly `CLI_CASES` (`.json`) and `CLI_TEXT_CASES` (`.txt`), like `tests/read_contract.rs:634-648` does for `read_v1`.
  - Add a top-level `#[test] fn index_lease_child()`, copied from `tests/read_boundary.rs:34-58`, for Task 4's `busy` case.
- [ ] **Step 6: Write `tests/cli/help.rs`.** Check every envelope against `envelope.schema.json`, and its data against its own schema, with `tests/support/schema.rs`'s `check_published`.
  - **Root help in both modes:** golden `help_root` (JSON) and text golden `help_root_text`. The bare call prints the same text and exits 0.
  - **Command help:** `ticket --help --json` is golden `help_resource_ticket`. `ticket list --help --json` and `ticket close --help --json` are goldens `help_ticket_list` and `help_ticket_close`.
  - **Version in both modes:** goldens `version` and `version_text`. A version bump regenerates them.
  - **No data directory:** help, version, the bare call, a usage error and an unavailable command each leave `CliEnv`'s data directory absent.
  - **Usage errors:**
    - `frobnicate --json` gives `cli usage`, exit 2, golden `usage_unknown_command`;
    - `ticket list --bogus --json` gives command `ticket list`, exit 2, golden `usage_unknown_option`.
  - **Unavailable:** `ticket close --id 01K7F6H9J2N4Q6S8V0X2Z4B6DJ --prepare --json` gives `command_unavailable`, exit 2, golden `unavailable_ticket_close`. In human mode, stderr names C5.
  - **Review Focus 4:** `--json id new --json` gives one envelope. `id new` is the verb already dispatched at this point.
  - **Review Focus 5:** `repo list --json` with `MANYHANDS_DATA_DIR` naming a regular file gives `internal_error`, exit 1, one envelope, and no `panicked` in stderr.

  Generate fixtures once with `MANYHANDS_UPDATE_GOLDEN=1`. Read every generated file, then run again without the variable.
- [ ] **Step 7: Make named edits 3–6.**
  - **Edit 3:** `golden_envelopes()` chains a third source: `support::cli::CLI_CASES` from `cli_fixture_directory()`, labelled `cli_v1`.
  - **Edit 4:** `DATA_ROOTS` gains the two schema file names. Change the length to 46.
  - **Edit 5:** `CODES_NO_MUTATION_GOLDEN_HAS` gains both codes with the reason in Global Constraints. Change the length to 6.
  - **Edit 6:** `contract_enumerations()` gains one entry: `("cli_help_command.schema.json", "repository", CLI_REPOSITORY_VALUES.to_vec())`. The helper leaves `null` out of the expected list itself (`tests/read_contract.rs:676`), so the list has only the two strings.
  - In `tests/cli/help.rs`, assert that the `repository` values across the root JSON help's commands are exactly `CLI_REPOSITORY_VALUES` plus null. That ties the binary's output to the registry.
- [ ] **Step 8: Run the tests.**
  - `devenv shell -- cargo test --locked --lib results::tests`
  - `devenv shell -- cargo test --locked --lib read::`
  - `devenv shell -- cargo test --locked --lib mutation::outcome`
  - `devenv shell -- cargo test --locked --test read_contract`
  - `devenv shell -- cargo test --locked --test mutation_contract`
  - `devenv shell -- cargo test --locked --bin manyhands-cli`
  - `devenv shell -- cargo test --locked --test cli`

  All pass with no edits beyond named edits 1–6.
- [ ] **Step 9: Run clippy and fmt, then commit.** `feat: usage_error and command_unavailable, CLI test harness and help goldens`.

## Task 4: Repository, Credential, Status And Utility Verbs

**Files:**
- Modify: `dispatch.rs`, `human.rs` and their tests, `tests/cli.rs`, `tests/support/cli.rs` (new `CLI_CASES`)
- Create: `tests/cli/admin.rs`

**Verbs:** `repo list`, `repo inspect`, `repo identity`, `remote list`, `key list`, `key show`, `key public`, `host list`, `host inspect`, `index status`, `poll status`, `operation list` and `operation show`. Task 2 already did `id new`.

**Interfaces:**
- Produces two helpers, used by Tasks 5–6:
  - `fn resolve(service: &RepositoryService, args: &ArgMatches) -> Result<ResolvedRepository, ReadError>`. It reads `--repo`, joins a relative path to the current directory, and calls `resolve_repository`.
  - `fn item_id(args: &ArgMatches, repo: &ResolvedRepository) -> Result<ItemId, ReadError>`. It maps a parse failure to `ReadError::invalid_id().with_scope(repo.scope())`.

- [ ] **Step 1: Write the failing tests in `tests/cli/admin.rs`.**
  - Run each verb in JSON mode against fixtures built with `tests/support`: `born_repository`, `enabled_repository`, `credentials::register_keys`, `credentials::pin_host` (including an IPv6 pin for `host inspect --authority [::1]:2222`) and the `operations` helpers.
  - Assert, for each verb:
    - the exit status;
    - the envelope and data schemas;
    - a golden named after F1's golden for the same read, prefixed `cli_`;
    - the scope, which equals the scope F1's golden test for that read uses in `tests/read_contract.rs` (copy it from there).
  - Do not mirror `index_status_unavailable`; see Cycle ruling R-2.

  The tests fail with `command_unavailable`.
- [ ] **Step 2: Implement each verb** with the F1 call in the design's Dispatch table.
  - A repository-scoped verb resolves first, then parses its IDs.
  - `--key-id` uses `SharedKeyId::parse`, with failure `ReadError::invalid_id()` and the default scope.
  - `--operation-id` uses `OperationId::parse`, with failure `invalid_id` and the repository's scope.
  - `host inspect` builds `SshAuthority { host, port }` from `parse::authority`. Remove the `#[allow(dead_code)]` Task 1 put on it.
  - Remove each verb from `placeholder_verbs()`.
- [ ] **Step 3: Add failure cases** through the binary, each with its exit status and a golden:
  - `invalid_path`: `--repo` names a missing path;
  - `not_repository`: an empty directory;
  - `not_repository_root`: a subdirectory. Human stderr has a `next: repo.inspect root=…` line;
  - `bare_repository`;
  - `repository_not_registered`, alone and with a bad `--operation-id`; the repository code wins;
  - `invalid_id`: `--key-id` and `--operation-id` with a lowercase ULID;
  - `key_not_found`, `authority_not_found`, `operation_not_found`;
  - `public_key_unavailable`, exit 3: `key public` for the key `register_keys` registers without a public key (`keys.without_public`, as `tests/read_credentials.rs:636` uses);
  - `repository_inaccessible`, exit 3, `#[cfg(unix)]`, if the binary can reach it. Try a registered repository whose `.manyhands/config.toml` is mode `000`, read by `remote list`; F1 reads that file through a guarded reader and reports an unopenable file as `repository_inaccessible`. Skip the case when the test runs as root, because permissions do not apply. If no route gives the code, record it in the execution doc as unreachable through the CLI, with the reason;
  - `index_unavailable`, exit 3, from `index status`: `items::degraded_service(enabled)` returns its own data directory, so point `MANYHANDS_DATA_DIR` at it;
  - `busy`, exit 5: start the `index_lease_child` from Task 3 with `support::hold_lease_in_child_for_test(…, "index_lease_child")` and `LeaseKind::CacheWrite`, then run `index status`. Assert the code and exit only, because the scope depends on where the lock was met.

  Record each code no C1 command can produce in the execution doc, with the reason the design gives.
- [ ] **Step 4: Review Focus 1 and 2.**
  - Run `repo identity --json` with each of these: `<root>/`; `.` from the root; `..` from `<root>/docs`; and a repository built with `born_repository_in(<temp>/"with space é", "repo")`, then enabled. Each succeeds with the same `scope.repository` as its canonical root.
  - Run it with a linked item worktree path made by `items::create_document_context`. It succeeds with the owner's root.
- [ ] **Step 5: Human mode.**
  - `key public` prints only the key text and a newline.
  - `id new` prints only a ULID.
  - `repo list` prints one line per registration.
  - A failure prints `error:`, `code:` and `next:` lines on stderr, and nothing on stdout.

  Add unit tests in `human_tests.rs` for each renderer, using DTOs built in the test.
- [ ] **Step 6: Run** `devenv shell -- cargo test --locked --bin manyhands-cli` and `--test cli`. Also run `--test mutation_contract`, which reads every CLI golden. Run clippy and fmt, then commit: `feat: CLI repository, credential and status reads`.

## Task 5: Content Verbs

**Files:**
- Modify: `dispatch.rs`, `human.rs` and their tests, `tests/cli.rs`, `tests/support/cli.rs`
- Create: `tests/cli/content.rs`

**Verbs:** `document list`, `document show` (`--id`, or `--path` with an optional `--context`), `folder list`, `ticket list` (all filters), `ticket show` (`--id` or `--path`) and `comment list`.

- [ ] **Step 1: Write the failing tests** in `tests/cli/content.rs`. Use `items::contract_repository()`, and add comments with `items::write_comment` and `items::COMMENT_*`, as `tests/read_contract.rs` sets up its comment golden.
  - **Goldens:** JSON goldens for each verb, mirroring F1's `document_list`, `document_show`, `document_show_nonconforming`, `document_folder_list`, ticket list/show and `comment_list` goldens, prefixed `cli_`.
  - **`ticket list` filters:** run it with each of `--closure open|closed|all`, `--status`, `--type`, `--project`, `--slug`, `--ready` and `--blocked`. The IDs returned must equal those of F1's `list_tickets` with the same `TicketFilter`, called in the test process.
  - **Conflicting flags:** `--ready --blocked` is `usage_error`.
  - **Kind mismatch (decision 10):**
    - `document show --id <ticket ULID>` and `ticket show --id <document ULID>` are `item_not_found`, exit 2, with the item's scope;
    - `document show --path .manyhands/tickets/<id>/ticket.md` is `item_not_found`, with `repo.scope()`.
  - **Bad selectors:**
    - `ticket show --id mh-vh-k9x2b` (`items::SHARED_SLUG`) is `invalid_id`, exit 2;
    - `document show --path docs/absent.md` is `path_not_found`;
    - `document show --path docs/../outside.md` is `invalid_path`.
  - **Stale reads:**
    - Set `refresh_required = 1` through `items::index(data_dir)`. `ticket list` then exits 0 with `data.index.state == "stale"`, and in human mode stderr has a `note:` line.
    - Move a document's modification time 30 seconds ahead (`set_modified`, as `tests/read_comments.rs:533` does). `document show` then reports the item behind its index.
- [ ] **Step 2: Implement the verbs.**
  - Build `TicketFilter` from the flags, with `closure` defaulting to `ClosureFilter::All`. `--ready` and `--blocked` set `readiness`.
  - `show` uses `show_item`, or `show_path(&repo, context.as_deref(), path)`.
  - The kind check compares the DTO's `kind` with the verb. On a mismatch, return `ReadError::new(ResultCode::ItemNotFound)` with the scope the success would have used.
  - Remove each verb from `placeholder_verbs()`.
- [ ] **Step 3: Human mode.**
  - A list prints one line per item: ULID, short code (or `-`), `closed` or `open` for tickets, status and title.
  - A nonconforming line prints `nonconforming`, the path and its problem codes.
  - `show` prints `Name: value` lines, a blank line, then the body. A nonconforming item prints its source instead of the body.
  - `comment list` indents two spaces per depth.
  - Renderers return notes for stale, never-refreshed and incomplete lists.

  Pin `ticket list` as text golden `ticket_list_text`. Pin the stderr of `ticket show --id <absent ULID>` as text golden `item_not_found_text`. Unit-test the other renderers.
- [ ] **Step 4: Run** `--bin manyhands-cli` and `--test cli`. Also run `--test mutation_contract`. Run clippy and fmt, then commit: `feat: CLI document, ticket and comment reads`.

## Task 6: Relationship Verbs

**Files:**
- Modify: `dispatch.rs`, `human.rs` and their tests, `tests/cli.rs`, `tests/support/cli.rs`
- Create: `tests/cli/relationships.rs`

**Verbs:** `ticket ready`, `ticket blocked`, `ticket deps`, `ticket children`, `ticket cycles`, `ticket plan`, `ticket critical-path` and `ticket find`.

- [ ] **Step 1: Write the failing tests.** Use `items::relationship_repository()` unless a case says otherwise.
  - **Goldens:** JSON goldens mirroring F1's relationship goldens, with the same command names, prefixed `cli_`.
  - **`ticket deps`:**
    - each of `--direction up|down|both`, and `--depth 0` and `--depth 1`;
    - `--depth -1` and `--direction sideways` are `usage_error`;
    - `ticket deps --id <document ULID>` is `item_not_found`. Use `items::contract_repository()` for this case, because the relationship fixture has no documents.
  - **`ticket find`:**
    - `--slug` with the shared slug in upper case returns every match;
    - an unknown slug and `--slug ""` each return an empty list with exit 0.
  - **Filters:** `ticket ready --status <s>` filters as F1 does.
- [ ] **Step 2: Implement** with the F1 calls in the design's Dispatch table. `ticket ready` and `ticket blocked` build a `TicketFilter` from `--status`, `--type` and `--project`, with `readiness` set.
- [ ] **Step 3: Human mode.**
  - `deps` indents by depth and marks `(repeated)`.
  - `plan` prints `Batch N:` groups, then `Unplannable:` with reasons.
  - `blocked` prints reasons under each ticket.
  - `cycles` prints one line per cycle.

  Unit-test each renderer.
- [ ] **Step 4: Empty the placeholder list.** The `placeholder_verbs()` test now asserts the list is empty. This changes a test written in this Cycle, not a pre-existing one.
- [ ] **Step 5: Run** `--bin manyhands-cli` and `--test cli`. Also run `--test mutation_contract`. Run clippy and fmt, then commit: `feat: CLI relationship reads`.

## Task 7: Boundaries, Workflow And Document Amendments

**Files:**
- Create: `tests/cli/boundaries.rs`
- Modify: `tests/cli.rs`, `.github/workflows/build.yml`, `docs/RFC/cli-contract.md`, `docs/Waves/wave-03-dogfooding.md`

- [ ] **Step 1: Redaction.** Build one fixture that plants:
  - a remote URL with a sentinel password and query;
  - stored problem guidance with a sentinel, as `tests/read_contract.rs` does;
  - keys from `credentials::register_keys`, whose `SECRET_SENTINELS` are scanned;
  - key material from `operations::insert_key_material`, whose `KEY_MATERIAL_SENTINEL` is scanned.

  Run every C1 verb in both modes against it. Assert that no sentinel appears in any stdout or stderr. Use `golden::find_sentinel` on JSON and a substring search on text.
- [ ] **Step 2: No repository change.** Take `repository_and_worktree_snapshot` before and after running every C1 verb against `items::contract_repository()`, and assert the two snapshots are equal.
- [ ] **Step 3: Review Focus 3, broken pipe.**
  - Write a document of about 4 MiB with `items::write`, then `items::commit` and `items::refresh_completely`.
  - Spawn `document show --id <id>` with `cli_command`, with stdout piped. Read 16 bytes, then drop the pipe.
  - Assert exit status 1, and that stderr does not contain `panicked`.
- [ ] **Step 4: Platform default directory**, `#[cfg(any(target_os = "linux", target_os = "macos"))]` (ruling R-3).
  - Run `repo list --json` with `CliEnv::without_data_dir()`, and with `XDG_DATA_HOME` (Linux) or `HOME` (macOS) pointing at the env's temporary directories.
  - Assert success, and that the Manyhands data directory now exists under that temporary location: `<XDG_DATA_HOME>/manyhands` on Linux, `<HOME>/Library/Application Support/com.manyhands.Manyhands` on macOS. These are the names `directories` 5 gives `ProjectDirs::from("com","manyhands","Manyhands")`; the test does not compute them in-process, because changing the environment in-process is unsafe in edition 2024 and races with parallel tests.
  - Add a comment for Windows saying why the test is not compiled there.
- [ ] **Step 5: Exit-class coverage.** Write one table-driven test. Each row is `(args, expected code, expected exit)` and runs through the binary inside the test. Assert that the table covers success and every class a C1 command can produce: input, blocked, transient and internal.
- [ ] **Step 6: Workflow.** On the `cargo test` line of `.github/workflows/build.yml`, append `--bin manyhands-cli --test cli`. Do not dispatch the workflow.
- [ ] **Step 7: Amend the documents**, as listed in the design's "Document Amendments".
  - Each amendment is a short dated note in the section it changes, for example "Amended 2026-10-18 by C1:".
  - The Wave's clarification row about transport bootstrap records Cycle ruling R-1.
- [ ] **Step 8: Run** `--test cli`. Run clippy and fmt, then commit: `test: CLI boundaries; docs: C1 contract amendments`.

## Task 8: Full Verification, Review And Handoff

- [ ] **Step 1: Run the full gate.** If `origin/main` moved, merge it first.
  - `devenv shell -- cargo check --all-features --locked`
  - `devenv shell -- cargo fmt --check`
  - `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`
  - `devenv shell -- cargo test --all-features --locked --no-fail-fast`

  Record the counts in the execution doc.
- [ ] **Step 2: Smoke tests.**
  - `devenv shell -- cargo run --locked --bin manyhands-cli` exits 0.
  - Run `set -o pipefail; devenv shell -- cargo tree --locked -e normal --no-default-features | grep -ci gpui` and expect `0`. `grep` exits 1 when it finds nothing, so check the printed `0`, not the exit status. Record the command and its output.
- [ ] **Step 3: Check main.** `git merge-tree --write-tree HEAD origin/main` is clean.
- [ ] **Step 4: Update the documents.**
  - Fill in the execution doc's acceptance mapping: each Cycle acceptance line, and the test or evidence for it. Include the inspection items:
    - Windows platform default (ruling R-3);
    - transport-start failure (ruling R-4);
    - system Git configuration (ruling R-5).
  - Set the Cycle doc's status to "implemented, review-ready".
  - Add a review-ready ticket comment.
- [ ] **Step 5: Commit.** `docs: mark C1 review-ready`.

## Plan Self-Review

- **Spec coverage.** Every Cycle scope line maps to a task:
  - parsing: Task 1;
  - help, version and startup order: Task 2;
  - codes: Task 3;
  - read verbs: Tasks 4–6;
  - by-path show: Task 5;
  - output streams and exit statuses: Task 2, with coverage in Task 7;
  - data directory: Tasks 2, 3 and 7.

  Every acceptance line maps to a test in Tasks 1–7, or to a step or inspection item in Task 8. The design's amendments are in Task 7, and the audit refresh is in Task 0.
- **Planning review.** Each of the 40 findings maps to:
  - a Cycle decision (11–13);
  - a ruling (R-1 to R-5);
  - or a change in the design or in this plan.

  The blocking ones are handled here:
  - **F1, F2:** named edits 3–6, with the codes landing with their goldens in Task 3;
  - **F3:** Task 7 Step 4;
  - **F4:** Task 8 Step 2;
  - **F5, F6:** Task 1 Steps 4–5.
- **Placeholders.** None. Where a value must come from the baseline (the clap version, F1 golden scopes), the step names the source to copy from.
- **Names.** These are defined in Tasks 1–4 and used with the same signatures later: `parse`, `Action`, `HelpTarget`, `authority`, `Human`, `envelope_output`, `failure_output`, `exit_status`, `run_cli`, `cli_command`, `json`, `assert_cli_golden`, `resolve`, `item_id`, `placeholder_verbs`.
- **Review Focus.** Each item has its step:
  - item 1 and 2: Task 4 Step 4;
  - item 3: Task 7 Step 3;
  - item 4: Task 1 Step 4 and Task 3 Step 6;
  - item 5: Task 3 Step 6.
