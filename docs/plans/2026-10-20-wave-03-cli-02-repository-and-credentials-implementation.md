---
title: "Wave 03 C2 CLI Repository And Credential Administration Implementation Plan"
date: 2026-10-20
status: approved
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5E2E02QVV1WWWQ1GE13HF8F"
---

# CLI Repository And Credential Administration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the CLI's mutation layer and the eighteen C2 administration verbs, so a person or an agent can onboard a repository and set up SSH from `manyhands-cli` alone.

**Architecture:** The bin keeps C1's catalog, parse, startup, dispatch and render structure. New bin-private modules sit beside them:

- `input.rs` reads and checks the JSON input;
- `mutate.rs` is the driver, written over a small `Boundary` trait so that it can be tested with a fake: request IDs, `prepare`, `confirm`, `execute`, the interactive flow, the bounded retry, and CLI-built refusals;
- `terminal.rs`, `prompt.rs` and `progress.rs` are the controlling terminal, its prompts and credential provider, and the human progress sink;
- `admin.rs` is the per-verb glue;
- `human_admin.rs` is the human text.

Every change of state is one F2 `prepare` or `execute`. Nothing under `src/repository*` changes.

**Tech Stack:**

- Rust 2024 and clap's builder API (from C1);
- `serde_json`;
- `libc` termios on Unix, and `windows-sys` with the `Win32_System_Console` feature on Windows, both existing dependencies;
- `zeroize`;
- F2's mutation boundary, result model, schemas and test support;
- C1's CLI harness and goldens.

**Spec:** the [Design](2026-10-20-wave-03-cli-02-repository-and-credentials-design.md), under the [Cycle](../Cycles/wave-03-cli-02-repository-and-credentials.md). Read both before any task. The Cycle governs the design, and the design governs this plan.

## Global Constraints

- **Prerequisite.** Defect `01M5B0Z6PNP7X4WH4JV3N0VQJ6` must be merged to `origin/main` before this plan is approved (Cycle decision 1). Task 0 verifies it and stops if it is not there.
- **Devenv.** Run every Rust command through Devenv with `--locked`: `devenv shell -- cargo <command> --locked`.
  - No new crate is added.
  - The one `Cargo.toml` dependency change is the `windows-sys` feature. Features are not locked, so `Cargo.lock` is unchanged, which `--locked` proves.
  - Task 10 also adds a `[[test]]` entry.
- **Branch.** Work only in the ticket worktree, on `manyhands/ticket/01M4CC0VMTTNKVYS6XWQG4CK1N`.
  - If `origin/main` moves, `git merge origin/main`.
  - Never rebase. Ignore the "rebase" wording in `.agents/skills/starting-a-cycle`; AGENTS.md says merge.
  - Before each checkpoint, run `git merge-tree --write-tree HEAD origin/main`.
- **Library.** No change under `src/repository.rs`, `src/repository/`, `src/lib.rs`, `src/results.rs` or `src/runtime.rs`. No new result code, recovery action or output schema.
  - A library gap stops the task. Report it, and the controller raises a separate ticket under track rule 5.
  - The wrong-passphrase code is already ticket `01M5E6DYJ6GN7DMW4424TH3V78` (decision 9). C2 does not wait for it.
- **Bin only.** CLI code lives in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`.
  - New modules follow C1's pattern in `src/bin/manyhands-cli.rs`: `#[path = "manyhands-cli/<name>.rs"] mod <name>;`.
  - A file loaded through `#[path]` resolves its own nested modules beside itself. So `terminal.rs` declares `#[cfg(unix)] #[path = "terminal/unix.rs"] mod platform;` and `#[cfg(windows)] #[path = "terminal/windows.rs"] mod platform;`.
  - Each `*_tests.rs` is included with `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`.
- **Runtime.**
  - The CLI spawns no thread and no child process.
  - The transport starts before the service opens. Help, version, usage errors and unavailable commands touch neither the transport nor the data directory.
  - JSON mode writes one envelope to stdout and nothing to stderr.
- **Secrets.**
  - No type that holds input or a secret derives or implements `Debug`, `Display` or `Serialize`.
  - Passphrases live in `Zeroizing` buffers and are moved into `SecretPassphrase` with `std::mem::take`.
  - No input value, secret, backend text or SQL reaches a diagnostic the CLI writes. Library envelopes pass through unchanged (Cycle contract item 2).
  - JSON `message` is always `ResultCode::message()`.
- **Test isolation.**
  - Every CLI child goes through `tests/support/cli.rs` or `tests/support/pty.rs`, and starts without a controlling terminal (R-11, E4) unless the pseudo-terminal helper gives it one.
  - Tests never touch developer keys, global Git configuration, the real terminal or real application data.
  - CLI tests that write or delete key files are `#[cfg(unix)]` (R-10), and so are pseudo-terminal tests.
- **Pre-existing tests.** Only the named edits may change a pre-existing test. Any other semantic edit stops for owner review.

  | Edit | Task | File and change |
  | --- | --- | --- |
  | E1 | 1 | `catalog_tests.rs` `catalog_matches_every_command_and_delivering_cycle_without_duplicates`: the delivering Cycle comes from `Availability::Available(cycle) => cycle`. The `DESIGN` table already maps the C2 verbs to `"C2"` and does not change. |
  | E2 | 5, 10 | `catalog_tests.rs` repository-rules test: `request show` and `request cancel` refuse `--repo` (Task 5); `host observe`, `host approve` and `host replace` require it (Task 10). |
  | E3 | 5–10 | Help goldens that list C2 verbs are regenerated as each verb becomes available. `--non-interactive`'s summary changes in Task 5. |
  | E4 | 5 | `tests/support/cli.rs` `cli_command`: children start without a controlling terminal, through a shared environment helper that `pty.rs` also uses. |
  | E5 | 5 | `dispatch_tests.rs`: `all_28_available_verbs_reach_a_real_dispatch_arm` filters to `Kind::Read` with `delivered_by() == "C1"`, keeping 28. If `dispatch`'s signature changes, every call in the file follows it, including `id_new_dispatches…`. |
  | E6 | 5 | `parse_tests.rs`, the count assertion tying `AVAILABLE` to the catalog (lines 97–103): restricted to `delivered_by() == "C1"`. The `AVAILABLE`-driven refusal test is unchanged; `AVAILABLE` lists only C1's reads. |
  | E7 | 5 | `startup_tests.rs` `every_available_run_starts_transport_before_service_and_maps_open_failure`: restricted to `Kind::Read`. |
  | E8 | 1 | `startup_tests.rs` `expected_command` helper: it reads `spec.delivered_by()` in place of `Available => "C1"`. |
  | E9 | 5 | `tests/cli/boundaries.rs` coverage test: its expectation becomes the C1 reads it calls. |

  Two changes are mechanical and keep every assertion:
  - wherever `Availability::Available` is used as a unit value, Task 1 changes it to `Available(_)` or `Available("C1")`;
  - Task 2 gives `startup_tests.rs`'s `FakeStartup` the new `open_terminal` method that the `Startup` trait gains.
- **Test runs (owner, 2026-10-17 and 2026-10-19).**
  - Each task runs only the targets and filters its steps name.
  - The Part checkpoints (Tasks 7, 9 and 11) run the **checkpoint gate** below.
  - The full suite runs once, in Task 12.
- **Checkpoint gate.**

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract
  ```

  From Task 10 on, also run `--test cli_remote`. After the gate:
  - run `git merge-tree --write-tree HEAD origin/main`;
  - get an independent review of the Part's commit range against the Cycle, the design and this plan;
  - fix or rule every finding before the next Part;
  - add a ticket comment.
- **Goldens.**
  - Update with `MANYHANDS_UPDATE_GOLDEN=1`, never under `CI`.
  - Read every regenerated golden's diff before committing.
  - New cases are registered in `CLI_CASES` or `CLI_TEXT_CASES`.
- **No subagents.** Implementers never dispatch subagents.
- **Commits.** Commit at the end of every task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Ticket comments.** Add a comment at Task 0, at each task's completion, at each Part checkpoint and at review-ready. Write it under `.manyhands/comments/01M4CC0VMTTNKVYS6XWQG4CK1N/<ULID>.md` in the canonical schema, with the ULID from the built CLI's `id new`.
- **Authorization.**
  - Never push, open a pull request, merge, close the ticket or clean up without separate owner authorization.
  - Pushing the ticket branch is the owner's standing habit; ask once at Task 0.
  - Native CI runs after merge, on main, only with the owner's approval of that run.

## Review Focus

These are inputs the spec implies but no table in it lists. Each has its test in the owning task.

1. **A missing repository's stored root spelled with a trailing separator.** `repo remove` answers `invalid_path` and removes nothing. The exact stored root from `repo list` still works. Task 6, Step 5.
2. **`--input` naming a directory or a missing file, or `--input -` with empty stdin.** `invalid_input`, exit 2, one envelope, no panic. Task 4, Step 2.
3. **An input file saved as UTF-16** (PowerShell 5's default). `invalid_input`, and the human detail says the input is not UTF-8. Task 4, Step 2.
4. **A key label holding control characters or escape sequences.** Prompts and human output escape them, so a label cannot repaint the terminal. Task 3, Step 1 (prompt) and Task 8, Step 4 (output).
5. **Ctrl-C at the consent prompt.** The terminal's echo and line mode are as they were before the command. Task 5, Step 9.

---

## File Map And Task Order

| Task | Part | Creates | Modifies |
| --- | --- | --- | --- |
| 0 | — | `docs/plans/2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md` | `docs/research/wave-03-api-audit.md` |
| 1 | A | — | `catalog.rs`, `parse.rs`, `startup.rs`, and the mechanical tuple change plus E1 and E8 in `catalog_tests.rs`, `parse_tests.rs`, `startup_tests.rs`, `dispatch_tests.rs` |
| 2 | A | `terminal.rs`, `terminal/unix.rs`, `terminal/windows.rs`, `terminal_tests.rs` | `Cargo.toml` (feature), `startup.rs`, `startup_tests.rs`, `src/bin/manyhands-cli.rs` |
| 3 | A | `prompt.rs`, `prompt_tests.rs` | `src/bin/manyhands-cli.rs` |
| 4 | A | `input.rs`, `input_tests.rs`, `schemas/input/v1/commit_identity.schema.json`, `tests/cli/input_schemas.rs`, `tests/fixtures/cli_input_v1/` | `src/bin/manyhands-cli.rs`, `tests/cli.rs`, `.gitattributes` |
| 5 | A | `mutate.rs`, `mutate_tests.rs`, `admin.rs`, `admin_tests.rs`, `progress.rs`, `progress_tests.rs`, `human_admin.rs`, `human_admin_tests.rs`, `tests/support/pty.rs`, `tests/cli/mutation.rs`, `tests/cli/requests.rs`, `tests/cli/prompts.rs`, `tests/cli/human.rs`, schemas `repo_identity_set` and `remote_add` | `render.rs`, `dispatch.rs`, `catalog.rs`, `catalog_tests.rs` (E2), `dispatch_tests.rs` (E5), `parse_tests.rs` (E6), `startup_tests.rs` (E7), `tests/cli/boundaries.rs` (E9), `tests/support/cli.rs` (E4), `tests/support/mod.rs`, `tests/cli.rs`, help goldens (E3) |
| 6 | A | `tests/cli/repository.rs`, schemas `repo_create` and `repo_enable` | `admin.rs`, `catalog.rs`, `human_admin.rs`, `tests/cli/prompts.rs`, `tests/cli/human.rs`, `tests/cli/mutation.rs`, `tests/cli/input_schemas.rs`, help goldens (E3) |
| 7 | A | `tests/cli/remotes.rs`, `tests/cli/exits.rs`, schemas `remote_remove` and `remote_select` | `admin.rs`, `catalog.rs`, `tests/cli/human.rs`, `tests/cli/repository.rs`, help goldens (E3) |
| 8 | B | `tests/cli/keys.rs`, schema `key_import` | `admin.rs`, `catalog.rs`, `human_admin.rs`, `tests/cli/exits.rs`, `tests/cli/human.rs`, help goldens (E3) |
| 9 | B | schema `key_generate`, `tests/cli/privacy.rs` | `admin.rs`, `catalog.rs`, `tests/cli/keys.rs`, `tests/cli/prompts.rs`, `tests/cli/input_schemas.rs`, help goldens (E3) |
| 10 | C | `tests/cli_remote.rs`, `tests/cli_remote/hosts.rs`, `tests/cli_remote/unlock.rs`, schemas `host_approve` and `host_replace` | `Cargo.toml` (`[[test]] name = "cli_remote"`, `harness = false`), `admin.rs`, `dispatch.rs`, `catalog.rs`, `catalog_tests.rs` (E2), `human_admin.rs`, `tests/cli/exits.rs`, `tests/cli/human.rs`, help goldens (E3) |
| 11 | C | `tests/cli_remote/journeys.rs` | `tests/cli/exits.rs`, `tests/cli/privacy.rs`, `tests/cli/prompts.rs`, `tests/cli/mutation.rs`, `.github/workflows/build.yml`, `docs/RFC/cli-contract.md`, `docs/RFC/authentication-and-credential-handling.md`, `docs/RFC/ticket-relationships-and-short-codes.md`, `docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md` |
| 12 | — | — | Cycle doc, design, execution doc, ticket and comments |

Tasks run in order. Tasks 5–10 each flip verbs from `Planned("C2")` to `Available("C2")` and edit the same files, so none can run in parallel.

A verb becomes available only in the task that dispatches it, gives it input and a schema, and its goldens. Until then it stays planned and answers `command_unavailable`, so no commit advertises a verb that does not work.

Each verb task also grows three cross-cutting tests with its verbs:

- `tests/cli/human.rs`: the human-mode run of each verb, and its no-op;
- `every_mutation_verb_s_confirmed_flag_matches_prepare` in `tests/cli/mutation.rs`;
- the `tests/cli/exits.rs` table, from Task 7.

---

## Task 0: Baseline, Prerequisite, Audit Refresh And Ledger

**Files:**
- Create: `docs/plans/2026-10-20-wave-03-cli-02-repository-and-credentials-execution.md`
- Modify: `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Confirm the base.**
  - `git fetch origin main`.
  - Check that `origin/main` contains `7d6dba4` (`01M5CVB2`), and the merge of `01M5B0Z6PNP7X4WH4JV3N0VQJ6`, found with `git log --oneline origin/main --grep 01M5B0Z6`.
  - If the `01M5B0Z6` merge is missing, **stop**: the Cycle's prerequisite is not met.
  - Run `git merge origin/main` and then `git merge-tree --write-tree HEAD origin/main`.
  - Record HEAD, `origin/main` and both prerequisite merges.
- [ ] **Step 2: Re-check every Baseline Fact in the design** against the source, including:
  - every signature in the Boundary bullet;
  - the `Mutation` input field list;
  - `startup.rs:143`;
  - `bind/repo.rs:707-709` and `1241`;
  - `PreviewEffect`'s 26 values;
  - the schema-directory tests;
  - the wrong-passphrase mapping (`transport/remote.rs:762-772`);
  - host authentication only when pinned (`transport/callbacks.rs:124-141`);
  - consent before input (`execute.rs:249-258`).

  Record each fact as confirmed or changed. A changed fact that alters a task stops here for a controller ruling.
- [ ] **Step 3: Confirm the prerequisite behavior.** Read the merged `01M5B0Z6` change and confirm that create and enable leave `.manyhands/config.toml` in the Git index, so `remote select` follows them without `worktree_not_clean`. Record the commit and the test that proves it.
- [ ] **Step 4: List the pre-existing tests C2 touches.** Run `grep -rn` over `src/bin/manyhands-cli/*_tests.rs`, `tests/cli/` and `tests/fixtures/cli_v1/` for:
  - each of the eighteen verbs and `"C2"`;
  - `Availability::Available`;
  - counts of available commands (`28`, `AVAILABLE`, `count`).

  Every hit not covered by E1–E9 or by the mechanical tuple change stops for an owner ruling.
- [ ] **Step 5: Baseline checks.** These are targeted, by the owner's decision of 2026-10-17; the full suite runs once, in Task 12.

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract
  ```

  Record the counts.
- [ ] **Step 6: Refresh the API audit.**
  - Add a section, "Wave 03 CLI Cycle C2 refresh (2026-10-20)".
  - For each of the eighteen C2 verbs, update its existing CLI-inventory row in place:
    - the F2 call and its signature at this baseline;
    - whether it needs confirmation;
    - its input type;
    - **the complete set of result codes it can return**, read from its binding and `outcome.rs`.
  - Do not add a second row for a verb.
  - The union of these code sets is the set `tests/cli/exits.rs` must account for (Task 7).
  - Note `01M5CVB2`, `01M5B0Z6` and `01M5E6DY`.
- [ ] **Step 7: Create the execution ledger**, with front matter like the design's and a new document ULID from `id new`. Its sections:
  - baseline;
  - confirmed facts;
  - prerequisite evidence;
  - audit refresh;
  - Cycle decisions 1–9, rulings R-1 to R-19 and named edits E1–E9;
  - the planning review record;
  - an empty task log.
- [ ] **Step 8: Commit** `docs: record C2 baseline and refresh the API audit`. Add a ticket comment. Ask the owner once whether to push the ticket branch.

## Task 1: Catalog Kinds, Delivering Cycle And Mutation Options

**Files:**
- Modify: `src/bin/manyhands-cli/catalog.rs`, `parse.rs`, `startup.rs` (`delivered_by` at line 143)
- Modify: `catalog_tests.rs` (E1), `startup_tests.rs` (E8), and the mechanical tuple change in `parse_tests.rs`, `startup_tests.rs` and `dispatch_tests.rs`

**Interfaces (later tasks use these exact names):**

```rust
// catalog.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Available, with the Cycle that delivered it: "C1" or "C2".
    Available(&'static str),
    Planned(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputRule { Required, Refused }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Read,
    /// A read that can ask to unlock a key: `host observe`.
    UnlockingRead,
    Mutation { confirmed: bool, input: InputRule },
}

pub struct CommandSpec {
    pub resource: &'static str,
    pub verb: &'static str,
    pub summary: &'static str,
    pub availability: Availability,
    pub repo: Option<RepoRule>,
    pub options: &'static [OptionSpec],
    pub kind: Kind, // planned commands: Kind::Read, unused
}

impl CommandSpec {
    pub fn delivered_by(&self) -> &'static str; // the Cycle in either variant
    pub fn prompts(&self) -> bool;              // Mutation or UnlockingRead
}

pub const INPUT: OptionSpec;       // "--input", Some("file|-"), required: false
pub const REQUEST_ID: OptionSpec;  // "--request-id", Some("ULID"), required: false
pub const PREPARE: OptionSpec;     // "--prepare", None
pub const CONFIRM: OptionSpec;     // "--confirm", Some("ULID")

// parse.rs
/// The rules that depend on the mode, checked by startup after the mode
/// is known. `Err` holds the usage detail for human stderr.
pub fn mode_rules(spec: &CommandSpec, args: &clap::ArgMatches, interactive: bool)
    -> Result<(), String>;
```

Each verb's own option list later uses a copy of `INPUT` with `required: true` where the design's rule table says so, so help shows it. The parser still marks nothing clap-required.

- [ ] **Step 1: Write the failing tests.**
  - `delivered_by_reads_the_cycle_from_either_availability`: every C1 command gives `"C1"`, and every planned C2 command gives `"C2"`.
  - E1: in `catalog_matches_every_command_and_delivering_cycle_without_duplicates`, the derivation becomes `Availability::Available(cycle) => cycle`. The `DESIGN` table is unchanged.
  - E8: `startup_tests.rs`'s `expected_command` reads `spec.delivered_by()`.
  - `prompts_is_true_only_for_mutations_and_unlocking_reads`, using test-local `CommandSpec` values.
  - `mode_rules` over a test-local mutation spec (`Kind::Mutation { confirmed: true, input: InputRule::Required }`) with the four new options:
    - no `--request-id` and not interactive → `Err`;
    - no `--request-id` and interactive → `Ok`;
    - `--prepare` without `--request-id` and not interactive → `Ok`;
    - `--prepare --request-id X` → `Err`;
    - `--prepare --confirm X` → `Err`.
  - A test-local `Kind::Read` spec declaring `--request-id` gets `Ok` from `mode_rules`, with no mutation rule applied.
- [ ] **Step 2: Run them and see them fail.** `devenv shell -- cargo test --locked --bin manyhands-cli catalog parse startup`.
- [ ] **Step 3: Implement.**
  - Change `Availability`, add `Kind` and `InputRule`. Give every C1 entry `Available("C1")` and `Kind::Read`. C2 entries stay `planned(.., "C2")`.
  - `startup.rs:143` uses `spec.delivered_by()`.
  - Make the mechanical tuple change in every test file that uses `Availability::Available` as a value.
  - Add the four `OptionSpec` constants. The parser declares `--input`, `--request-id`, `--prepare` and `--confirm` only for options in a command's own list, as C1 declares every option.
  - `mode_rules` implements the last two rows of the design's per-verb table. The verb-specific rows live in the existing per-verb rule function and are added with each verb.
- [ ] **Step 4: Run them and see them pass.** Also run `devenv shell -- cargo test --locked --bin manyhands-cli` and `devenv shell -- cargo test --locked --test cli help`. No golden may change in this task.
- [ ] **Step 5: Commit** `refactor: give catalog entries a kind and a delivering Cycle`. Add a ticket comment.

## Task 2: The Controlling Terminal And The Mode Decision

**Files:**
- Create: `src/bin/manyhands-cli/terminal.rs`, `terminal/unix.rs` (`#[cfg(unix)]`), `terminal/windows.rs` (`#[cfg(windows)]`), `terminal_tests.rs`
- Modify: `Cargo.toml` (add `"Win32_System_Console"` to the `windows-sys` features), `src/bin/manyhands-cli.rs` (`#[path = "manyhands-cli/terminal.rs"] mod terminal;` and `RealStartup::open_terminal`), `startup.rs`, `startup_tests.rs`

**Interfaces:**

```rust
// terminal.rs
use zeroize::Zeroizing;

pub trait Terminal {
    /// Text that asks nothing, such as a preview.
    fn write(&mut self, text: &str) -> std::io::Result<()>;
    /// Sets the reading mode, writes `prompt`, reads one line, restores
    /// the mode. `echo: false` masks the line. `Ok(None)` is a cancel.
    fn ask(&mut self, prompt: &str, echo: bool) -> std::io::Result<Option<Zeroizing<String>>>;
}

/// The controlling terminal, or `None` when the process has none.
pub fn open() -> Option<Box<dyn Terminal>>;

/// The platform-independent key handling both platforms feed.
pub struct LineEditor { /* bytes: Zeroizing<Vec<u8>>, echo: bool */ }
pub enum Step {
    /// Bytes to write back to the terminal; empty when masked.
    Pending(Vec<u8>),
    Done,
    Cancel,
}
pub const MAX_LINE_BYTES: usize = 4096;
impl LineEditor {
    pub fn new(echo: bool) -> Self;
    pub fn feed(&mut self, byte: u8) -> Step;
    /// The line, or `None` when its bytes are not UTF-8.
    pub fn finish(self) -> Option<Zeroizing<String>>;
}

#[cfg(test)]
pub struct FakeTerminal {
    pub answers: std::collections::VecDeque<Option<String>>, // None = cancel
    pub written: String,          // every write and every prompt, in order
    pub asks: Vec<(String, bool)>, // each prompt and its echo flag
}

// startup.rs: `Startup` gains
fn open_terminal(&mut self) -> Option<Box<dyn Terminal>>;
```

- [ ] **Step 1: Write the failing `LineEditor` tests** in `terminal_tests.rs`. Each test feeds a byte script and asserts the `Step` sequence and `finish()`:
  - `enter_ends_a_line_with_cr_or_lf`;
  - `ctrl_c_cancels_and_ctrl_d_cancels_only_an_empty_line`;
  - `backspace_removes_a_whole_multibyte_character`;
  - `ctrl_u_clears_the_line`;
  - `other_control_bytes_are_ignored`;
  - `bytes_past_the_limit_are_ignored`;
  - `invalid_utf8_finishes_as_none`;
  - `masked_lines_echo_nothing`, which also asserts every `Pending` is empty;
  - `echoed_lines_draw_and_erase`: `\b \b` on backspace.

  ```rust
  #[test]
  fn backspace_removes_a_whole_multibyte_character() {
      let mut editor = LineEditor::new(false);
      for byte in "é".bytes().chain([0x7f, b'\r']) {
          let _ = editor.feed(byte);
      }
      assert_eq!(editor.finish().as_deref().map(String::as_str), Some(""));
  }
  ```
- [ ] **Step 2: Run them and see them fail.** `devenv shell -- cargo test --locked --bin manyhands-cli terminal`.
- [ ] **Step 3: Implement `LineEditor`** exactly as the design's key-handling list, then `FakeTerminal`.
- [ ] **Step 4: Implement `terminal/unix.rs`.**
  - `open()` opens `/dev/tty` with `libc::open(c"/dev/tty".as_ptr(), O_RDWR | O_CLOEXEC)`. A negative return is `None`.
  - `ask`, in this order:
    1. `tcgetattr`, then a guard whose `Drop` runs `tcsetattr(TCSANOW, saved)`.
    2. `tcsetattr(TCSAFLUSH)` with `ICANON | ECHO | ISIG` cleared, `VMIN = 1` and `VTIME = 0`.
    3. Only then write the prompt.
    4. Read one byte at a time with `libc::read` (`EINTR` retried) into `LineEditor`, writing each `Pending` back.

    End of file is a cancel.
  - Every `unsafe` block has a `SAFETY:` comment.
- [ ] **Step 5: Implement `terminal/windows.rs`.**
  - `CreateFileW` on `CONIN$` and on `CONOUT$`. Failure is `None`.
  - `ask`, in this order:
    1. Save the mode with `GetConsoleMode`; a guard restores it with `SetConsoleMode`.
    2. Clear `ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT`.
    3. `FlushConsoleInputBuffer`.
    4. Write the prompt with `WriteConsoleW`.
    5. `ReadConsoleW` reads one UTF-16 unit at a time. Surrogate pairs are joined into a `char`, which is fed to `LineEditor` as its UTF-8 bytes.
  - This module is compiled only by native CI (R-16). Keep it under 150 lines, using only the functions named here.
- [ ] **Step 6: Write the failing startup tests** in `startup_tests.rs`. Extend C1's fake with `terminal_present: bool` and a call-log entry `"terminal"`.
  - `a_prompting_command_opens_the_terminal_before_the_transport`, with a test-local mutation spec: the log is `["terminal", "transport", "service"]`.
  - `no_read_opens_the_terminal`.
  - `json_and_non_interactive_never_open_the_terminal`.
  - `a_mode_rule_failure_is_usage_and_starts_nothing`: no `--request-id` and no terminal give a usage envelope and exit 2, with nothing in the log after `"terminal"`.
- [ ] **Step 7: Implement the startup step** as the design's Startup Order step 3.
  - `run` receives the terminal as `Option<Box<dyn Terminal>>`, opened only for `spec.prompts()` when not `--json` and not `--non-interactive`.
  - `spec.prompts()` is checked before `--non-interactive` is read. clap's debug `get_flag` panics on an undeclared argument, and E7's reads run with `ArgMatches::default()`.
  - `interactive` is `terminal.is_some()`.
  - `mode_rules` runs before `start_transport`.
  - `RealStartup::open_terminal` calls `terminal::open()`.
- [ ] **Step 8: Run** `devenv shell -- cargo test --locked --bin manyhands-cli terminal startup` and `devenv shell -- cargo clippy --locked --bin manyhands-cli -- -D warnings`.
- [ ] **Step 9: Commit** `feat: read the controlling terminal and decide the interactive mode`. Add a ticket comment.

## Task 3: Prompts And The Terminal Credential Provider

**Files:**
- Create: `src/bin/manyhands-cli/prompt.rs`, `prompt_tests.rs`
- Modify: `src/bin/manyhands-cli.rs` (`#[path = "manyhands-cli/prompt.rs"] mod prompt;`)

**Interfaces:**

```rust
// prompt.rs
pub enum Consent { Yes, No }
pub enum Asked<T> { Answer(T), Cancelled, Exhausted } // Exhausted: three failed attempts

pub fn consent(terminal: &mut dyn Terminal, preview_text: &str) -> Consent; // cancel or I/O error: No
pub fn identity(terminal: &mut dyn Terminal) -> Asked<manyhands::repository::CommitIdentity>;
pub fn new_passphrase(terminal: &mut dyn Terminal, label: &str)
    -> Asked<manyhands::repository::keys::SecretPassphrase>;

pub struct TerminalProvider<'a> {
    terminal: Option<&'a mut dyn Terminal>,
    /// Set when a passphrase is supplied (R-18). Owned by the caller,
    /// because `SessionCredentials` keeps the provider and offers no
    /// accessor. Holds no value.
    supplied: &'a std::cell::Cell<bool>,
}
impl<'a> TerminalProvider<'a> {
    pub fn new(terminal: Option<&'a mut dyn Terminal>, supplied: &'a std::cell::Cell<bool>) -> Self;
}
impl manyhands::repository::keys::SessionCredentialProvider for TerminalProvider<'_> { /* … */ }
```

Callers pass `session.terminal.as_deref_mut().map(|t| t as &mut dyn Terminal)`. `Option` does not coerce its contents' object lifetime on its own. The caller reads its `Cell` after the library call returns.

- [ ] **Step 1: Write the failing tests** with `FakeTerminal`. Build each `UnlockRequest` with a `KeySourceToken` from the public `KeySourceToken::observe(path)` over a temporary file, and a `key_id` from `SharedKeyId::parse` of a fixed ULID.
  - `consent_accepts_y_and_yes_in_any_case_and_nothing_else`: `y`, `YES`, ` yes ` are `Yes`; `n`, the empty line, `ye` and a cancel are `No`.
  - `consent_writes_the_preview_then_asks`: the preview goes through `write`, and the question through `ask` with echo on.
  - `identity_asks_name_then_email_echoed`.
  - `identity_asks_again_after_an_empty_answer_and_gives_up_after_three`.
  - `identity_cancel_is_cancelled`.
  - `new_passphrase_is_masked_and_asked_twice`.
  - `a_mismatch_or_an_empty_passphrase_asks_again_and_says_why`.
  - `three_failed_passphrase_attempts_are_exhausted`.
  - `a_cancel_at_either_passphrase_prompt_is_cancelled`.
  - `the_provider_asks_once_masked_and_supplies_and_records_that_it_did`.
  - `the_provider_cancels_on_ctrl_c_and_on_an_empty_line`.
  - `the_provider_shows_the_guidance_for_ambiguous_authentication`.
  - `the_provider_without_a_terminal_is_unavailable`.
  - Review Focus 4, `a_label_with_control_characters_is_escaped_in_every_prompt`: the label `"key\x1b[2J"` appears in `written` only as `render::field` escapes it.
  - `no_secret_appears_in_what_a_prompt_writes`: feed the sentinel `"pass-sentinel-ÆØ"`; `written` does not contain it.
- [ ] **Step 2: Run them and see them fail.** `devenv shell -- cargo test --locked --bin manyhands-cli prompt`.
- [ ] **Step 3: Implement** the design's Prompts table.
  - Answers stay `Zeroizing` until they are moved into `SecretPassphrase::new(std::mem::take(&mut *line))`.
  - A failed write or ask is a cancel.
  - Passphrase comparison uses the two `Zeroizing<String>` values directly; neither is copied.
- [ ] **Step 4: Run them and see them pass.**
- [ ] **Step 5: Commit** `feat: ask for consent, identity and passphrases on the terminal`. Add a ticket comment.

## Task 4: Input Reading, Field Checks And The Input Schema Contract

**Files:**
- Create: `src/bin/manyhands-cli/input.rs`, `input_tests.rs`, `schemas/input/v1/commit_identity.schema.json`, `tests/cli/input_schemas.rs`, `tests/fixtures/cli_input_v1/.gitkeep`
- Modify: `src/bin/manyhands-cli.rs`, `tests/cli.rs` (`#[path = "cli/input_schemas.rs"] mod input_schemas;`), `.gitattributes` (`tests/fixtures/cli_input_v1/** -text`)

**Interfaces:**

```rust
// input.rs
pub struct InputObject { /* BTreeMap<String, serde_json::Value>, built by a duplicate-refusing visitor */ }

pub enum Tristate { Missing, Null, Value(String) }

pub enum InputError {
    Unreadable, NotUtf8, Empty, Malformed, NotObject,
    DuplicateKey(String), UnknownField(String),
    Missing(&'static str),
    WrongType { field: &'static str, expected: &'static str },
    InvalidValue { field: &'static str, allowed: &'static [&'static str] },
}
impl InputError {
    /// Human detail: names a field, escaped with `render::field`, never a value.
    pub fn detail(&self) -> String;
}

/// `source` is "-" for stdin, or a path relative to `cwd`.
pub fn read(source: &str, cwd: &std::path::Path, stdin: &mut dyn std::io::Read)
    -> Result<InputObject, InputError>;
pub fn parse(bytes: &[u8]) -> Result<InputObject, InputError>;

impl InputObject {
    pub fn only(&self, allowed: &[&str]) -> Result<(), InputError>;
    pub fn string(&self, field: &'static str) -> Result<String, InputError>;
    pub fn optional_string(&self, field: &'static str) -> Result<Option<String>, InputError>; // null → WrongType
    pub fn nullable_string(&self, field: &'static str) -> Result<Option<String>, InputError>; // required
    pub fn tristate(&self, field: &'static str) -> Result<Tristate, InputError>;
    pub fn optional_object(&self, field: &'static str) -> Result<Option<InputObject>, InputError>; // null → WrongType
    pub fn one_of(&self, field: &'static str, allowed: &'static [&'static str])
        -> Result<&'static str, InputError>;
}

/// The schema file of a verb: `remote_add.schema.json` for ("remote", "add").
pub fn schema_name(resource: &str, verb: &str) -> String;
```

- [ ] **Step 1: Write the failing unit tests** in `input_tests.rs`:
  - `one_object_parses` and `a_leading_byte_order_mark_is_skipped`;
  - each `InputError` variant from its byte input;
  - `duplicate_keys_are_refused_at_any_depth`, at the top level and inside `identity`;
  - the accessors: required; optional and object with `null` refused; nullable; the three tristate cases; `one_of`;
  - `no_detail_echoes_a_value`: every string value in every failing case is `"VALUE-SENTINEL-41"`, and no `detail()` contains it;
  - `a_key_name_is_escaped_in_its_detail`: the key `"k\u001b[2J"`;
  - `utf16_input_is_not_utf8`: bytes `FF FE 7B 00 7D 00`.
- [ ] **Step 2: Write the Review Focus 2 and 3 tests**, still unit-level:
  - `read` of a directory, of a missing file, and of an empty stdin are `Unreadable`, `Unreadable` and `Empty`;
  - `read` of a UTF-16 file is `NotUtf8`, and its detail says "not UTF-8".
- [ ] **Step 3: Run them and see them fail.** `devenv shell -- cargo test --locked --bin manyhands-cli input`.
- [ ] **Step 4: Implement `input.rs`.** The visitor:

  ```rust
  /// The CLI's own JSON value. Its map visitor refuses a repeated key at
  /// any depth; `serde_json::Value` would keep the last one.
  enum Json { Null, Bool(bool), Number(serde_json::Number), String(String), Array(Vec<Json>), Object(BTreeMap<String, Json>) }
  impl<'de> serde::Deserialize<'de> for Json { /* visit_map: a repeated key is a custom error carrying the key */ }
  ```

  The visitor keeps a duplicate's key for `DuplicateKey`, without any value. `InputObject` wraps the top-level `Json::Object`.
- [ ] **Step 5: Write `commit_identity.schema.json`** with `title`, `$schema` 2020-12, `type: object`, `name` and `email` strings, `required: ["name","email"]` and `additionalProperties: false`.
- [ ] **Step 6: Write `tests/cli/input_schemas.rs`.** It reads the binary's `--json --help` data.
  - `every_input_taking_command_has_exactly_one_schema_and_every_schema_is_reached`:
    - an available command whose options include `--input` must have `schemas/input/v1/<schema_name>`;
    - every file in the directory is such a schema, or is reached from one by `$ref`;
    - the test has its own two-line copy of `schema_name`;
    - `commit_identity` sits in a `PENDING_REFERENCES` constant until Task 6 empties it.
  - `every_input_schema_uses_only_the_checked_keywords_and_is_closed`: `tests/support/schema.rs`'s `validate` over the directory, plus `additionalProperties: false` on every object.
  - `every_example_input_validates_against_its_schema`, over `tests/fixtures/cli_input_v1/<schema>.json`. Each verb task's golden test feeds the same file to the binary.
- [ ] **Step 7: Run** `devenv shell -- cargo test --locked --bin manyhands-cli input` and `devenv shell -- cargo test --locked --test cli input_schemas`.
- [ ] **Step 8: Commit** `feat: read and check JSON mutation input`. Add a ticket comment.

## Task 5: The Mutation Driver, Rendering, Request Verbs And The First Two Mutations

The largest task of Part A. It wires the layer end to end through two verbs:

- `repo identity-set`, confirmed;
- `remote add`, not confirmed;

and the two request reads. It applies E2 for the request verbs, and E4–E7 and E9.

**Files:** see the File Map row for Task 5.

**Interfaces:**

```rust
// mutate.rs
pub enum Flow { Prepare, Execute { confirmation: Option<ConfirmationId> }, Interactive }
pub fn flow(prepare: bool, confirm: Option<ConfirmationId>, confirmed: bool,
            interactive: bool, record_exists: bool) -> Flow;

/// What the driver asks of the library, so that unit tests can fake it.
pub trait Boundary {
    fn prepare(&self, mutation: &Mutation) -> Envelope<PrepareDataDto>;
    fn execute(&self, call: MutationCall,
               credentials: &mut SessionCredentials<TerminalProvider<'_>>,
               progress: &mut dyn ProgressSink) -> Envelope<MutationDataDto>;
    /// `show_request` answered success.
    /// `Ok(true)`: `show_request` succeeded. `Ok(false)`: it answered
    /// `request_not_found`. `Err`: any other failure, which `run` renders.
    fn request_exists(&self, request_id: RequestId) -> Result<bool, Envelope<RequestDto>>;
}
impl Boundary for RepositoryService { /* execute passes CancellationToken::new() */ }

/// Decision 7: whether `envelope` is retried automatically for `request_id`.
pub fn should_retry(envelope: &Envelope<MutationDataDto>, request_id: RequestId) -> bool;
pub const RETRY_DELAYS_MS: [u64; 3] = [100, 200, 400];

pub struct Session {
    pub json: bool,
    pub interactive: bool,
    pub terminal: Option<Box<dyn Terminal>>,
}
/// One line straight to stderr, ignoring a write failure (R-9). A free
/// function, because the terminal stays borrowed by the credentials across
/// the retry loop.
pub fn note(line: &str);

pub struct MutationFlags {
    pub request_id: Option<RequestId>,
    pub prepare: bool,
    pub confirm: Option<ConfirmationId>,
}

/// What a verb's glue may receive from a prompt before it builds.
#[derive(Default)]
pub struct Extras { pub identity: Option<CommitIdentity> }

pub struct Plan {
    pub identity_prompt: bool, // repo create/enable whose input had no identity
    pub retry: bool,           // false for a protected key generation
}

pub fn run(
    boundary: &dyn Boundary,
    session: &mut Session,
    spec: &'static CommandSpec,
    flags: MutationFlags,
    plan: Plan,
    build: &mut dyn FnMut(&Extras) -> Mutation, // called once per call of the library
    sleep: &mut dyn FnMut(std::time::Duration),
) -> Output;

/// A refusal made before `execute` (R-6). Echoes `request_id` when given.
pub fn refusal(session: &Session, command: &str, request_id: Option<RequestId>,
               code: ResultCode, detail: Option<String>) -> Output;

// progress.rs
pub struct HumanProgress { last: Option<ProgressStage> }
impl ProgressSink for HumanProgress { /* "progress: <stage>" to stderr once per change; write errors ignored */ }

// render.rs
pub fn mutation_output(json: bool, envelope: &Envelope<MutationDataDto>, passphrase_supplied: bool) -> Output;
pub fn prepare_output(json: bool, envelope: &Envelope<PrepareDataDto>) -> Output;

// human_admin.rs
pub fn preview(preview: &PreviewDto) -> String;               // exhaustive over PreviewDetailsDto
pub fn effect_phrase(effect: PreviewEffect) -> &'static str;   // exhaustive
pub fn mutation_success(envelope: &Envelope<MutationDataDto>) -> Human;
pub fn failure_extra(envelope: &Envelope<MutationDataDto>, passphrase_supplied: bool) -> String; // request/operation/effects/data lines, R-18 hint
pub fn request(dto: &RequestDto) -> Human;
pub fn cancel(dto: &CancelDto) -> Human;

// admin.rs: every verb function has this signature
pub fn repo_identity_set(service: &RepositoryService, session: &mut Session,
                         spec: &'static CommandSpec, args: &ArgMatches) -> Output;
pub fn remote_add(/* same */) -> Output;
pub fn input_object(session: &Session, spec: &CommandSpec, args: &ArgMatches,
                    request_id: Option<RequestId>) -> Result<InputObject, Output>;
pub fn flags(args: &ArgMatches) -> Result<MutationFlags, Output>; // invalid_id refusal; order: --request-id, --confirm
pub fn mutation_root(args: &ArgMatches) -> Result<PathBuf, Output>; // joined, never canonicalized

// tests/support/cli.rs
pub fn cli_environment(command: &mut Command, env: &CliEnv); // shared by cli_command and PtyCli
pub fn run_cli_with_stdin(args: &[&OsStr], env: &CliEnv, stdin: &[u8]) -> CliRun; // pipes, then closes, stdin

// tests/support/pty.rs (Unix)
pub struct PtyCli { /* child, master fd, kept slave fd, transcript */ }
impl PtyCli {
    pub fn spawn(args: &[&OsStr], env: &CliEnv, stdin: &[u8]) -> Self;
    pub fn expect(&mut self, text: &str);    // reads until `text`, 10 s deadline
    pub fn send(&mut self, bytes: &[u8]);
    pub fn finish(self) -> (CliRun, String); // run result and transcript
    pub fn lflag_before_and_after(&self) -> (libc::tcflag_t, libc::tcflag_t); // read through the master
}
```

- [ ] **Step 1: Write the failing `flow` and `should_retry` tests** in `mutate_tests.rs`.
  - One `flow` test per row of the design's Flow table, including that `record_exists` matters only when the command is confirmed and interactive.
  - `should_retry`:
    - true for `busy` with `request.retry` of this ID;
    - true for `partial` with that action first;
    - false for another request's ID;
    - false for `transport_unavailable`, built as the library gives it: outcome `error`, with `request.retry` first. So the test proves the code and outcome gate, not the action;
    - false for a `partial` `request_mismatch`, which has no action;
    - false for `success`.
- [ ] **Step 2: Write the failing driver tests** with a `FakeBoundary` that returns scripted envelopes and records calls, and a recording `sleep`.
  - `a_busy_answer_is_retried_three_times_at_most_with_the_set_delays`: the sleeps are 100, 200 and 400 ms; the last envelope is returned.
  - `a_retry_presents_the_same_confirmation`: a confirmed command answered `busy` and then `ok`. The second `execute` carries the first one's confirmation.
  - `a_show_request_failure_other_than_not_found_is_rendered_and_starts_nothing`.
  - `a_retry_stops_at_the_first_answer_that_does_not_qualify`.
  - `no_retry_when_the_plan_says_so`.
  - `interactive_prepare_then_consent_yes_executes_with_the_preview_s_confirmation`.
  - `consent_no_is_a_cancelled_refusal_echoing_the_request_id`.
  - `identity_required_then_asked_then_prepared_again`: two `prepare` calls; the second mutation carries the identity; the stderr note is written without values.
  - `an_existing_record_skips_prepare_and_consent`.
  - `prepare_flag_calls_prepare_only`.
  - `refusals_have_null_operation_and_data_empty_scope_no_recovery`.
- [ ] **Step 3: Implement** `flow`, `should_retry`, `Boundary` (and its `RepositoryService` impl), `Session`, `refusal` and `run`, following the design's Mutation Driver section.
  - `run` notes `request: <id>` before the first library call when it allocated the ID.
  - It creates a `Cell<bool>` and `SessionCredentials::new(TerminalProvider::new(.., &supplied))` once, before the first `execute`, and keeps both for the retries.
  - Each retry presents the confirmation the attempt before it presented.
  - After the last call it passes `supplied.get()` to `mutation_output`.

  Run `devenv shell -- cargo test --locked --bin manyhands-cli mutate`.
- [ ] **Step 4: Write the failing renderer and progress tests.**
  - In `human_admin_tests.rs`:
    - `every_preview_effect_has_a_phrase`, over `PreviewEffect::ALL` (`src/results.rs:29`), with no two phrases equal;
    - `every_details_shape_renders_its_fields`, with one hand-built `PreviewDto` per variant;
    - `a_failure_adds_request_operation_effects_and_data_lines`;
    - `the_wrong_passphrase_hint_appears_only_after_a_supplied_passphrase_and_transport_unavailable`;
    - `a_noop_says_already_applied`.
  - In `progress_tests.rs`:
    - `a_stage_is_written_once_per_change`;
    - `a_failed_write_is_ignored`, using a writer that always fails.
- [ ] **Step 5: Implement** `human_admin.rs`, `render::{mutation_output, prepare_output}` and `progress.rs`. Then run the tests.
- [ ] **Step 6: Wire the request verbs and the two mutations.**
  - `catalog.rs`:
    - `request show` and `request cancel` become `Available("C2")`, `Kind::Read`, `repo: Some(Refused)`, with options `[REQUEST_ID required]`;
    - `repo identity-set` becomes `Kind::Mutation { confirmed: true, input: Required }`, with options `[REPO, INPUT required, REQUEST_ID, PREPARE, CONFIRM]`;
    - `remote add` becomes `confirmed: false`, with the same options.
  - Add their per-verb rule rows. `dispatch.rs` routes them.
  - `admin::repo_identity_set` reads `name`, `email` and `initials` (`Tristate` → `InitialsChange`). `admin::remote_add` reads `name` and `url`.
  - Write the two input schemas and their example inputs.
  - Apply E2 for the request verbs, and E5, E6, E7 and E9, each exactly as the table in Global Constraints says.
- [ ] **Step 7: Apply E4 and add the pseudo-terminal helper.**
  - Move `cli_command`'s environment setup into `cli_environment`.
  - `cli_command` then adds, on Unix, `unsafe { command.pre_exec(|| { libc::setsid(); Ok(()) }) }`, with a `SAFETY:` comment (`setsid` is async-signal-safe). On Windows it adds `creation_flags(0x0000_0008)` (`DETACHED_PROCESS`).
  - `pty.rs` follows the design's Isolation section:
    - it uses `cli_environment` without E4's `pre_exec`;
    - the master is opened with `posix_openpt(O_RDWR | O_NOCTTY | O_CLOEXEC)`;
    - the slave path comes from `ptsname_r` on Linux, and from `ptsname` under a process-wide `Mutex` on macOS. It is a `CString` prepared before spawning;
    - its own `pre_exec` calls `setsid()`, `open(slave, O_RDWR)`, `ioctl(fd, TIOCSCTTY as _, 0)` and `close(fd)`;
    - the parent keeps a slave fd open with `O_NOCTTY | O_CLOEXEC` until the child is reaped;
    - a reader thread copies the master into a channel and treats `EIO` as end of input only after the child has exited;
    - `finish` reaps the child, then closes the kept slave, then joins the reader;
    - `lflag_before_and_after` reads `tcgetattr` through the master;
    - every test `expect`s each prompt before it `send`s that prompt's answer, because `ask` flushes typed-ahead input.
  - Add `run_cli_with_stdin`, built on `cli_command` with `Stdio::piped()` stdin, which is written and then closed.
- [ ] **Step 8: Write the integration tests, then run them and fix until they pass.**
  - In `tests/cli/mutation.rs`:
    - `a_child_has_no_controlling_terminal`: `repo identity-set` in human mode without `--request-id` is `usage_error`, exit 2, with no prompt;
    - `json_needs_a_request_id`;
    - `prepare_refuses_a_request_id_and_confirm`;
    - `prepare_then_confirm_sets_the_identity`: goldens `repo_identity_set_prepare` and `repo_identity_set`; global configuration is byte-identical;
    - `without_confirm_a_first_call_is_confirmation_required`;
    - `waiting_stdin_is_never_read_as_consent`: with no terminal, `repo identity-set --request-id R --input <file>` with stdin `y\n` is `confirmation_required`, exit 3, and the identity is unchanged;
    - `the_same_request_again_is_answered_from_its_record`;
    - `the_same_request_id_with_other_input_is_request_mismatch`;
    - `another_confirmation_or_other_input_is_confirmation_mismatch`;
    - `remote_add_from_stdin`: `--input -` with a piped object, golden `remote_add`;
    - `initials_missing_null_and_set`;
    - `input_refusals_through_the_binary`: an unknown field, a `passphrase` field, a duplicate key, a `null` name;
    - `every_mutation_verb_s_confirmed_flag_matches_prepare`: a preview for `repo identity-set`, and `not_confirmable` for `remote add`. Each later verb task adds its rows;
    - `human_progress_lines_and_empty_json_stderr`: human mode shows `progress: accepted` and `progress: writing`, and JSON stderr is empty;
    - `json_stderr_is_empty_in_every_case`.
  - In `tests/cli/requests.rs`:
    - `request_show_of_a_finished_request` (golden `request_show`);
    - `request_show_of_an_accepted_request`: a library-level actor with `FailurePoint::AfterRemoteMutation` leaves a `remote add` accepted (golden `request_show_accepted`);
    - `request_show_of_an_unknown_id_is_request_not_found`;
    - `request_cancel_of_a_finished_local_request` (golden `request_cancel`);
    - `request_cancel_of_an_accepted_local_request_is_not_requested`;
    - `request_verbs_refuse_repo`.
  - In `tests/cli/human.rs`, the table's first rows: `repo identity-set` and `remote add` in human mode, and the identity-set no-op.
- [ ] **Step 9: Write the first pseudo-terminal tests** in `tests/cli/prompts.rs` (`#[cfg(unix)]`):
  - `consent_yes_sets_the_identity`: `repo identity-set` without `--request-id`. `request: ` appears on stderr, the preview appears on the terminal, and the identity is set.
  - `consent_no_is_cancelled_exit_130_and_changes_nothing`.
  - Review Focus 5, `ctrl_c_at_consent_is_cancelled_and_restores_the_terminal`: send `0x03` after `expect("Proceed?")`; exit 130; `lflag_before_and_after` are equal.
  - `an_existing_request_id_skips_consent`: complete a request through JSON, then run it again in the pseudo-terminal with the same `--request-id`. No `Proceed?` appears, and the stored result is replayed.
  - `stdin_y_with_n_on_the_terminal_is_cancelled`: input from a file, stdin `y\n`, and `n` answered on the terminal: `cancelled`, exit 130.
- [ ] **Step 10: Regenerate the help goldens (E3)** and read the diff: four verbs move to available, and `--non-interactive`'s summary changes.
- [ ] **Step 11: Run** `devenv shell -- cargo test --locked --bin manyhands-cli` and `devenv shell -- cargo test --locked --test cli`, then `devenv shell -- cargo clippy --locked --all-targets -- -D warnings`.
- [ ] **Step 12: Commit** `feat: add the mutation driver, request verbs, repo identity-set and remote add`. Add a ticket comment.

## Task 6: Repository Verbs

**Files:** see the File Map row for Task 6.

**Interfaces:**

```rust
// admin.rs: same signature as remote_add
pub fn repo_create(..) -> Output;   // fields: primary_branch, identity?, ticket_slug_prefix?
pub fn repo_enable(..) -> Output;
pub fn repo_remove(..) -> Output;   // no input; root from mutation_root, never canonicalized
```

`ticket_slug_prefix` maps to `slug_prefix`. `Plan.identity_prompt` is true exactly when the input had no `identity`.

- [ ] **Step 1: Write the failing integration tests** in `tests/cli/repository.rs`. Every missing-identity test first calls a helper, `require_no_system_identity()` (R-17). It reads `git2::Config::find_system()` and fails with "this test needs a system Git configuration without user.name or user.email" when either key is set.
  - `create_in_an_empty_directory` and `create_in_an_absent_directory`: `--prepare` (golden `repo_create_prepare`), then `--confirm` (golden `repo_create`). `repo list` holds the root, and `.manyhands/config.toml` is in the Git index.
  - `create_in_an_occupied_directory_is_occupied_path`, through `--prepare`.
  - `enable_an_existing_repository` (goldens `repo_enable_prepare`, `repo_enable`).
  - `enable_an_enabled_but_unregistered_repository_registers_it` (`01M5CVB2`): enabled through one data directory, then through a second.
  - `remote_select_follows_create_and_enable_without_worktree_not_clean` (decision 1). It uses the CLI's `remote add`, and the library's `RemoteSelect` through `execute` in the test process until Task 7 switches it to the CLI.
  - `the_prefix_is_previewed_and_written`.
  - `identity_required_then_resubmitted_with_identity`: `--prepare` without an identity is `identity_required` (golden). With `identity` it succeeds, written to the repository only. Global configuration is byte-identical.
  - `a_global_identity_is_previewed_with_its_source_and_enable_writes_none`.
  - `remove_an_accessible_registration` (goldens `repo_remove_prepare`, `repo_remove`): files unchanged.
  - `remove_the_exact_stored_root_of_a_deleted_directory`.
- [ ] **Step 2: Run them and see them fail** (`command_unavailable`).
- [ ] **Step 3: Implement** the three glue functions and flip the catalog entries:
  - create and enable: `confirmed: true, input: Required`, options `[REPO, INPUT req, REQUEST_ID, PREPARE, CONFIRM]`;
  - remove: `input: Refused`, options `[REPO, REQUEST_ID, PREPARE, CONFIRM]`.

  Add their per-verb rule rows and schemas. Empty `PENDING_REFERENCES` in `input_schemas.rs`. Add their rows to `every_mutation_verb_s_confirmed_flag_matches_prepare`, and their human and no-op rows (already enabled and registered) to `human.rs`.
- [ ] **Step 4: Run them and see them pass.**
- [ ] **Step 5: Write the Review Focus 1 test** `a_deleted_root_spelled_with_a_trailing_separator_is_invalid_path`: nothing is removed, and the exact root then works.
- [ ] **Step 6: Write the pseudo-terminal identity tests** in `tests/cli/prompts.rs`, each after `require_no_system_identity()`:
  - `create_asks_for_an_identity_and_previews_it`: the identity is local only, and global configuration is byte-identical;
  - `enable_asks_for_an_identity_and_previews_it`: the same, for an existing repository;
  - `ctrl_c_at_the_identity_prompt_is_cancelled_and_creates_nothing`: the target directory still does not exist;
  - `three_empty_names_are_invalid_input`.
- [ ] **Step 7: Regenerate the help goldens (E3)** and read the diff. Run `devenv shell -- cargo test --locked --test cli` and `--bin manyhands-cli`.
- [ ] **Step 8: Commit** `feat: add repo create, enable and remove`. Add a ticket comment.

## Task 7: Remote Verbs, The Exit Table And Part A Checkpoint

**Files:** see the File Map row for Task 7.

**Interfaces:**
- `admin::remote_remove(..)`: confirmed; field `name`.
- `admin::remote_select(..)`: field `name`, required and nullable.

- [ ] **Step 1: Write the failing tests** in `tests/cli/remotes.rs`:
  - `select_then_clear` (goldens `remote_select`, `remote_select_clear`);
  - `remove_a_remote` (goldens `remote_remove_prepare`, `remote_remove`);
  - `removing_the_publication_remote_is_publication_remote_in_use`, through `--prepare`;
  - `a_taken_name_is_remote_name_conflict`;
  - `an_invalid_url_is_invalid_remote`;
  - `a_non_ssh_remote_cannot_be_selected`;
  - `select_without_an_identity_is_identity_required_with_repo_identity_set` (golden), after `require_no_system_identity()`; then `repo identity-set`, then select succeeds;
  - `a_url_password_is_never_printed`: stdout and stderr of add, select and remove, in both modes;
  - `a_partial_remote_add_is_completed_by_the_same_request`: a library-level actor opens the data directory with `RepositoryService::open_at_with_failure_point_for_testing(data, FailurePoint::AfterRemoteMutation)` and executes `RemoteAdd` under request R; the CLI with `--request-id R` then completes it;
  - `the_same_request_id_with_another_url_over_the_stopped_request_is_partial_request_mismatch_exit_4`;
  - `selecting_the_selected_remote_is_a_no_op`.
- [ ] **Step 2: Run them and see them fail. Implement and flip** `remote remove` (confirmed) and `remote select` (not confirmed). Add their schemas, rule rows, `confirmed`-flag rows and human rows. Switch `tests/cli/repository.rs`'s decision-1 regression to the CLI's `remote select`.
- [ ] **Step 3: Write `tests/cli/exits.rs`.**
  - One table of `(name, args, input, code, exit, how)`. `how` is either "run here", or "covered by `cli_remote::<case>`" for codes that need the SSH server.
  - Each run-here row goes through the binary, using `--prepare` for validation refusals of confirmed commands (`occupied_path`, `invalid_path`, `publication_remote_in_use`).
  - `busy` comes from a child holding `LeaseKind::Repository` on the target repository: `support::hold_lease_in_child_for_test(root, data, LeaseKind::Repository, "index_lease_child")`, which sends `kind.as_str()`. The row uses an unconfirmed verb, `remote add`. With the automatic retries a run takes about 1.7 s, inside the holder's 10 s deadline.
  - `REASONS` lists each unreachable code with its reason. It starts with:
    - `confirmation_expired`;
    - `registration_pending` and `discovery_pending`;
    - `remote_unavailable`;
    - `host_approval_required`, with the design's reason.
  - `the_table_and_reasons_account_for_every_code_of_the_audit`: the table's codes plus `REASONS` equal the C2 code set Task 0 recorded in the audit, written into the test as a constant.
  - `every_cli_remote_reference_names_an_existing_case`: reads `tests/cli_remote/*.rs` for each named function.
  - `every_exit_class_is_covered`. `cancelled` is covered by `prompts.rs` on Unix, and the table records it so.
  - Part A's rows:
    - input: `usage_error`, `invalid_input`, `invalid_id`, `not_confirmable`, `request_mismatch`, `confirmation_mismatch`, `request_not_found`, `occupied_path`, `invalid_path`, `remote_name_conflict`, `invalid_remote`, `not_repository`, `not_repository_root`, `bare_repository`, `repository_not_enabled`;
    - blocked: `confirmation_required`, `confirmation_not_found`, `confirmation_used`, `external_change`, `identity_required`, `publication_remote_in_use`, `worktree_not_clean`, `wrong_branch`, `invalid_configuration`, `repository_not_registered`;
    - incomplete: the partial `request_mismatch`;
    - transient: `busy`;
    - internal: `internal_error`, with `MANYHANDS_DATA_DIR` naming a file.

    A code the audit lists that no fixture reaches goes to `REASONS`, with its reason, in the commit that finds it.
- [ ] **Step 4: Run** `devenv shell -- cargo test --locked --test cli remotes exits repository`. Regenerate and read the help goldens (E3).
- [ ] **Step 5: Commit** `feat: add remote remove and select, and the exit table`.
- [ ] **Step 6: Part A checkpoint.** Run the checkpoint gate in Global Constraints over the range from Task 1's base. Record the counts in the ledger.

## Task 8: Key Verbs Without Secrets

**Files:** see the File Map row for Task 8.

**Interfaces:**

```rust
pub fn key_import(..) -> Output;  // label, private_key_path, public_key_path?; paths joined to cwd
pub fn key_select(..) -> Output;  // --key-id
pub fn key_clear(..) -> Output;   // no selector, no input
pub fn key_remove(..) -> Output;  // --key-id
pub fn key_delete(..) -> Output;  // --key-id; confirmed; KeyStore::for_current_user() or the internal_error refusal
```

Every key verb refuses `--repo`.

- [ ] **Step 1: Write the failing tests** in `tests/cli/keys.rs`:
  - `import_a_key_by_relative_paths`: the child runs in the fixture directory with relative paths; golden `key_import`; `key list` holds the absolute path.
  - `select_clear_and_remove` (goldens `key_select`, `key_clear`, `key_remove`): the files remain.
  - `selecting_the_selected_key_and_clearing_no_selection_are_no_ops`.
  - `removing_the_selected_key_is_selected_key_in_use`.
  - `deleting_an_imported_key_is_key_not_deletable_and_its_files_are_unchanged`: through `--prepare`; file bytes and modification times are compared before and after.
  - `#[cfg(unix)] delete_a_generated_key` (goldens `key_delete_prepare`, `key_delete`). A library-level actor generates the key with `KeyStore::for_home(env.home)`, matching the child's `HOME`.
  - `#[cfg(unix)] deleting_the_selected_generated_key_is_selected_key_in_use`, through `--prepare`.
  - `#[cfg(unix)] an_interrupted_deletion_is_completed_by_the_same_request`, following the design: the `FailurePoint::DeletionAfterPrivatePhase` actor, then the CLI without `--confirm`, then a replay that deletes nothing more.
  - `a_key_id_that_is_a_short_code_or_lowercase_is_invalid_id`.
- [ ] **Step 2: Run them and see them fail. Implement and flip** the five catalog entries: `key delete` with `confirmed: true`, the rest not. Add `key_import`'s schema, and the rule, `confirmed`-flag and human rows.
- [ ] **Step 3: Add exits rows:** `key_not_deletable`, `selected_key_in_use`, `key_not_found` (an unknown key ID) and `key_unavailable` (`key import` of a missing file, which maps through `SourceMissing`), and any other key code the audit lists.
- [ ] **Step 4: Write the Review Focus 4 output test** `a_label_with_control_characters_is_escaped_in_human_output`. Import a key labelled `"k\x1b[2J"` and list it in human mode; stdout holds the escaped form and no raw `ESC`.
- [ ] **Step 5: Run** `devenv shell -- cargo test --locked --test cli keys exits human`. Regenerate and read the help goldens.
- [ ] **Step 6: Commit** `feat: add key import, select, clear, remove and delete`. Add a ticket comment.

## Task 9: Key Generation, Passphrase Entry And Part B Checkpoint

**Files:** see the File Map row for Task 9.

**Interfaces:** `admin::key_generate(..)`:

- `protection: "none"` builds `KeyProtection::Unencrypted`.
- `protection: "passphrase"`:
  - interactive, without `--prepare`: `prompt::new_passphrase`. `Answer` goes on; `Cancelled` gives the `cancelled` refusal; `Exhausted` gives `invalid_input`;
  - not interactive: the `unlock_required` refusal, with no library call (R-19 for a retry);
  - `--prepare`: no prompt, and the mutation is built with `KeyProtection::Unencrypted`. F2's `prepare` reads no input of an unconfirmed command and answers `not_confirmable` (`confirm.rs:295-301`); a code comment says so.
- `Plan.retry` is false for a protected generation. `build` moves the secret in once.

- [ ] **Step 1: Write the failing tests.**
  - In `keys.rs`:
    - `a_protected_generation_without_a_terminal_is_unlock_required` (golden `key_generate_unlock_required`): nothing in the key store and no record;
    - `prepare_of_key_generate_is_not_confirmable_and_asks_nothing`;
    - `#[cfg(unix)] generate_an_unprotected_key` (golden `key_generate`): the files exist under `HOME/.ssh/manyhands/`, with modes 0700 and 0600.
  - In `input_schemas.rs`, `protection_accepts_passphrase_and_none_only`, through `--prepare`: each allowed value answers `not_confirmable`, and `"secret"` answers `invalid_input`. No key file is written on any platform.
  - In `prompts.rs` (`#[cfg(unix)]`):
    - `a_protected_generation_asks_twice_masked`, with the sentinel `"pass-sentinel-ÆØ"`: the key is generated, and its private file opens with `ssh_key` only with the sentinel;
    - `mismatched_passphrases_ask_again_and_three_failures_are_invalid_input`;
    - `ctrl_c_at_the_passphrase_prompt_is_cancelled_and_writes_no_key`;
    - R-19, `a_protected_generation_retried_without_a_terminal_is_unlock_required_and_request_show_has_its_result`.
  - In `privacy.rs`:
    - `#[cfg(unix)] no_passphrase_reaches_output_data_home_or_the_transcript`: every file under the data directory and the test home, both streams and the transcript, searched as UTF-8 and UTF-16LE;
    - `no_url_credential_or_key_material_sentinel_reaches_any_output`, reusing C1's sentinels across every C2 verb run in this target.
- [ ] **Step 2: Run them and see them fail. Implement and flip** `key generate`: not confirmed, input required, `--repo` refused. Add its schema and its rule, `confirmed`-flag and human rows.
- [ ] **Step 3: Run** `devenv shell -- cargo test --locked --test cli keys prompts privacy input_schemas`. Regenerate and read the help goldens.
- [ ] **Step 4: Commit** `feat: add key generate with masked passphrase entry`.
- [ ] **Step 5: Part B checkpoint.** Run the checkpoint gate over the range from Task 8's base.

## Task 10: Host Verbs Over SSH, And Unlock

**Files:** see the File Map row for Task 10.

**Interfaces:**

```rust
// dispatch.rs
pub fn host_observe(service: &RepositoryService, session: &mut Session, args: &ArgMatches) -> Output;
// `--repo` joined, not resolved. Optional `--authority`, parsed with C1's parser.
// Human mode writes `progress: connecting` first.
// `observe_host` is given `SessionCredentials::new(TerminalProvider::new(.., &supplied))`.
// After the call, `supplied.get()` goes to the observation's renderer for R-18's hint.
// admin.rs
pub fn host_approve(..) -> Output; // authority (C1 parser; bad → invalid_input), algorithm, fingerprint
pub fn host_replace(..) -> Output; // as above, plus old_fingerprint
```

**`tests/cli_remote.rs` layout.** It follows `tests/remote_observation.rs:1-22`: `pub use manyhands::{repository, runtime}; mod support;` plus `#[path]` for the `ssh_*` files, `support/remote_world.rs` and `support/ssh_privacy.rs`, as `document_promotion` and `ticket_closure` do. Its `main` calls `ssh_harness::run(&CASES)`.

Two worlds:

- **Protected key.** `tests/support/remote_world.rs::World::new()`, as `tests/mutation_remote/host.rs` uses it. It encrypts the fixture client key with `remote_world::PASSWORD`, clones and enables a repository, and opens a service at `World::data()`. The CLI child gets `MANYHANDS_DATA_DIR = World::data()`.
  - Step 1 records which key registration and selection `World` makes; any it does not make, the case makes through the CLI's `key import` and `key select`.
  - Cases that need a pinned host pin it first through a library actor. It calls `verify_ssh_transport` with a `HostApproval` and `remote_world::Provider`, as the `approve` helper in `tests/mutation_remote.rs:661-678` does. Copy that helper; it is not in `support`.
- **Unprotected key.** `SshRemoteFixture::start()` with the plain fixture key, and repositories the case makes through the CLI.

SSH goldens replace the server's port with `0`.

- [ ] **Step 1: Write the failing cases** in `hosts.rs`:
  - `observe_an_unpinned_host` (golden `host_observe`): it reports the presented key, and `host list` is empty afterwards;
  - `observe_names_only_the_remote_s_authorities`: another authority is `invalid_input`;
  - `observe_without_a_publication_remote_is_publication_remote_required`;
  - `observe_a_pinned_host_without_a_selected_key_is_no_selected_key`;
  - `approve_by_prepare_and_confirm` (golden `host_approve_prepare`): the pin is stored, `fixture.receive_updates()` is empty, every local reference is unchanged, and the success envelope is checked against its schema;
  - `approving_the_pinned_key_is_a_no_op`;
  - `approve_a_fingerprint_the_host_does_not_present_is_host_mismatch` (golden), with no pin;
  - `approve_over_another_pin_is_host_replacement_required` at prepare (golden);
  - `replace_after_the_host_key_rotates`, arranged as F2's `a_rotated_host_key_is_replaced_by_naming_the_old_and_the_new_fingerprint`: `--prepare` gives the golden `host_replace_prepare`, the exact old and new fingerprints replace the pin, and the success is checked against its schema;
  - `replace_with_a_wrong_old_fingerprint_is_refused_at_prepare`: pin unchanged;
  - `replace_naming_a_key_the_host_does_not_present_is_host_mismatch`: `presented_*` names the pin the host still presents, and the pin is unchanged;
  - `observe_prints_the_next_command_s_input_line`: in human mode, feeding that line to `host approve --input - --prepare` gives a preview.
- [ ] **Step 2: Write the failing cases** in `unlock.rs`, in the protected-key world. The right passphrase is `remote_world::PASSWORD`, which is also the privacy scans' sentinel.
  - `json_approve_with_a_protected_key_is_unlock_required`, with no prompt.
  - `json_observe_of_a_pinned_host_with_a_protected_key_is_unlock_required`.
  - `observe_of_an_unpinned_host_needs_no_unlock`.
  - `#[cfg(unix)] the_right_passphrase_unlocks_approve_and_observe_of_a_pinned_host`.
  - `#[cfg(unix)] a_wrong_passphrase_is_today_transport_unavailable_with_the_hint`: approve, with the wrong passphrase at every prompt. Exit 5, nothing pinned, and the R-18 hint on stderr. There is no automatic retry, which these show: exactly one `Passphrase for key` prompt in the transcript, and no `progress: retrying` on stderr.
  - `#[cfg(unix)] observe_of_a_pinned_host_with_a_wrong_passphrase_shows_the_hint`: `transport_unavailable`, exit 5, the hint on stderr.
  - `#[cfg(unix)] a_rejected_key_after_the_right_passphrase_is_unlock_failed`: `server.reject_client()`; exit 3.
  - `#[cfg(unix)] ctrl_c_at_the_unlock_prompt_of_approve_is_unlock_required`: exit 3, nothing pinned, and `request show` answers `request_not_found`.
  - `#[cfg(unix)] ctrl_c_at_the_unlock_prompt_of_observe_is_unlock_required`: on a pinned host; exit 3, pin unchanged.
  - `#[cfg(unix)] no_passphrase_reaches_the_transcript_the_data_or_the_home`.
- [ ] **Step 3: Run them and see them fail.** `devenv shell -- cargo test --locked --test cli_remote`.
- [ ] **Step 4: Implement and flip** the three host verbs:
  - `host observe`: `Kind::UnlockingRead`, `--repo` required, `--authority` optional;
  - `host approve` and `host replace`: confirmed, input required.

  Add their schemas. Apply E2 for the host verbs. Add their `confirmed`-flag and human rows, the latter through `cli_remote` where a server is needed. In `exits.rs`, add the rows that point at `cli_remote`: `host_mismatch`, `host_replacement_required`, `unlock_required`, `unlock_failed`, `transport_unavailable`, `no_selected_key` and `publication_remote_required`.
- [ ] **Step 5: Run** `devenv shell -- cargo test --locked --test cli_remote` and `devenv shell -- cargo test --locked --test cli input_schemas exits`. Regenerate and read the help goldens.
- [ ] **Step 6: Commit** `feat: add host observe, approve and replace`. Add a ticket comment.

## Task 11: Journeys, Evidence, Documents And Part C Checkpoint

**Files:** see the File Map row for Task 11.

- [ ] **Step 1: Write the journeys** in `journeys.rs`. Every step runs through the binary with `--json --request-id`, and with `--prepare`/`--confirm` where the command needs confirmation.
  - `onboard_an_empty_directory`, in the unprotected world:
    1. `repo create` with `identity`;
    2. `remote add` of `fixture.url()`;
    3. `remote select`;
    4. `key import` of the fixture key;
    5. `key select`;
    6. `host observe`;
    7. `host approve` with the observed key;
    8. a final `host observe`, which authenticates (`fixture.authentication_counts()` grows) and reports the pin.

    `repo list` holds the root. Every CLI child's pipes reach end of file at its exit. The test does not count processes, because the server's Git helpers are processes.
  - `onboard_an_existing_repository`: the same journey from `repo enable`, on a repository with history built in the test with `git2`.
  - `#[cfg(unix)] onboard_with_a_generated_key`: `key generate` with `"none"`; then `key public` and `fixture.allow_client_public_key(parsed)`; then the observe and approve steps.
- [ ] **Step 2: Complete the evidence tests.**
  - `exits.rs`: every code of the audit's set is a row or in `REASONS`, and every class is covered.
  - `prompts.rs`, `piped_input_and_terminal_prompts_are_separate`: `repo create --input -` with JSON on a pipe, identity and consent answered on the terminal.
  - `privacy.rs` runs once more over the whole target's outputs.
- [ ] **Step 3: Run** `devenv shell -- cargo test --locked --test cli_remote --test cli`.
- [ ] **Step 4: Amend the documents** as the design's Document Amendments section lists:
  - the CLI RFC;
  - the authentication RFC;
  - the relationships RFC;
  - the Wave: the C2 scope line, the C3 scope line, the C2 section notes, and the wrong-unlock exit evidence;
  - the API audit, whose C2 rows are closed with their test files.

  Each amendment is marked "Amended 2026-10-20 by C2". In the CLI RFC, keep the existing table rows, and replace "Planned for C2" where it appears.
- [ ] **Step 5: Add `--test cli_remote`** to the test command at `.github/workflows/build.yml:115`, after `--test cli`.
- [ ] **Step 6: Commit** `test: prove the C2 journeys and amend the contract documents`.
- [ ] **Step 7: Part C checkpoint.** Run the checkpoint gate, including `--test cli_remote`, over the range from Task 10's base.

## Task 12: Full Verification, Review And Handoff

- [ ] **Step 1: Merge and check.** `git fetch origin main`, merge `origin/main` if it moved, then `git merge-tree --write-tree HEAD origin/main`.
- [ ] **Step 2: Full gate**, once, with sufficient timeouts:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked --no-fail-fast
  devenv shell -- cargo check --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  devenv shell -- cargo check --locked --features desktop --bin manyhands
  devenv shell -- cargo clippy --locked --features desktop --bin manyhands -- -D warnings
  devenv shell -- cargo run --locked --bin manyhands-cli
  ```

  The desktop smoke test, `devenv shell -- cargo run --locked --features desktop --bin manyhands`, runs only if a display is active. Record it as skipped otherwise.
- [ ] **Step 3: Manual Linux checks**, in a real terminal against a throwaway repository and an isolated data directory:
  - interactive `repo create` with the identity prompt;
  - consent declined with Ctrl-C, then `stty -a` showing echo on;
  - a protected `key generate`.

  Record what was seen, with no secret.
- [ ] **Step 4: Map every acceptance row** of the Cycle to its test, in the ledger, including the Wave's per-Cycle evidence row as the Cycle maps it.
- [ ] **Step 5: Independent final review** of the whole branch, spec and quality, with deferred findings carried in. Fix or rule every finding.
- [ ] **Step 6: Mark review-ready.** Update the Cycle and design status lines and the ledger. Add a review-ready ticket comment that lists:
  - the native checks owed:
    - Windows prompting;
    - Windows key files;
    - the macOS pseudo-terminal behavior;
    - the post-merge native CI run;
  - the Windows terminal module, first compiled natively (R-16);
  - the open library defect `01M5E6DYJ6GN7DMW4424TH3V78` (decision 9).
- [ ] **Step 7: Lifecycle.** With owner approval of the review:
  - close the ticket in the final push before merge, as AGENTS.md requires;
  - each of push, pull request, merge and cleanup needs its own authorization;
  - native CI runs after merge, on main, only with the owner's approval of that run.

## Plan Self-Review

- **Spec coverage.**
  - Every Cycle acceptance row maps to a task:
    - onboarding (11);
    - missing identity (6, 7);
    - cancelled and wrong unlock (10);
    - host replacement (10);
    - imported-key non-deletion and the deletion retry (8);
    - configuration failures (7, 8, 10, 11);
    - schemas (4 to 10);
    - stdin (5, 11);
    - secrets (9, 10, 11);
    - human mode and no-ops (5 to 10);
    - the Wave's evidence row (5, 7, 12).
  - Every ruling has its owning step:
    - R-1 (4);
    - R-2, R-11 and E4 (2, 5);
    - R-3, R-4, R-6, R-8 and R-9 (5);
    - R-5 (6);
    - R-7 (3, 9);
    - R-10 (8, 9, 11);
    - R-12 (10);
    - R-13 (5);
    - R-14 (5, 9);
    - R-15 (4);
    - R-16 (2, 12);
    - R-17 (6, 7);
    - R-18 (5, 10);
    - R-19 (9).
- **Placeholders.** None. Each step names its tests, files and commands. Layouts not pinned by goldens are left to the implementer, by design.
- **Types.** These names are used consistently from their defining task:
  - `Availability::Available(cycle)`, `Kind`, `InputRule` (Task 1);
  - `Terminal::ask`, `LineEditor`, `FakeTerminal` (Task 2);
  - `TerminalProvider`, `Asked`, `Consent` (Task 3);
  - `InputObject`, `Tristate`, `InputError` (Task 4);
  - `Boundary`, `Session`, `MutationFlags`, `Extras`, `Plan`, `run`, `refusal`, `should_retry` (Task 5).
- **Review Focus.** Five lines, each with its test in Tasks 3, 4, 5, 6 and 8.
