---
title: "Wave 03 C3 CLI Authoring And Discovery Maintenance Implementation Plan"
date: 2026-10-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5QY01B09GY7DQ6DPWE2733H"
---

# CLI Authoring And Discovery Maintenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add C3's thirteen verbs to `manyhands-cli`, so that an agent or a person can author documents and tickets, set relationships, repair and adopt files, and maintain the index without hand-editing canonical files. Then switch this project's ticket dogfooding to the CLI.

**Architecture:** The bin keeps C1's catalog, parse, startup, dispatch and render structure, and C2's mutation driver (`mutate::run`). New bin-private modules sit beside them:

- `authoring.rs`: per-verb glue for the eleven mutations, built like `admin.rs`;
- `fill.rs`: the save patch, a pure merge plus one `show_item` read;
- `human_authoring.rs`: human text for the two new reads.

Every change of state is one F2 `prepare` or `execute`, and every read is one F1 read. Nothing under `src/repository*` changes.

**Tech Stack:**

- Rust 2024 and clap's builder API (C1);
- `serde_json` and `git2`, both existing dependencies;
- F2's mutation boundary, result model, schemas and test support;
- C1's and C2's CLI harness, goldens and pseudo-terminal helper.

**Spec:** the [Design](2026-10-24-wave-03-cli-03-authoring-and-discovery-design.md), under the [Cycle](../Cycles/wave-03-cli-03-authoring-and-discovery.md). Read both before any task. The Cycle governs the design, and the design governs this plan. "Fact N" means the design's Baseline Fact N.

## Global Constraints

- **Devenv.** Run every Rust command through Devenv with `--locked`: `devenv shell -- cargo <command> --locked`.
  - No new crate.
  - No change to `Cargo.toml` or `Cargo.lock`.
  - No new `[[test]]` target; new tests are modules of `tests/cli.rs`.
- **Branch.** Work only in the ticket worktree `.manyhands/worktrees/01M4CC0VMV81DFRGJ4Z76489XM`, on `manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`.
  - If `origin/main` moves, `git merge origin/main`. Never rebase.
  - Before each checkpoint, run `git fetch origin` and then `git merge-tree --write-tree HEAD origin/main`.
- **Library.** No change under `src/repository.rs`, `src/repository/`, `src/lib.rs`, `src/results.rs` or `src/runtime.rs`. No new result code, recovery action or output schema.
  - A library gap stops the task. Report it, and the controller raises a separate ticket under track rule 5.
  - The known limits of Cycle decision 1 (`01M4PB82`, `01M4PB80`, `01M4PB7Y`) are not worked around.
- **Bin only.** CLI code lives in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`. New modules follow C1's pattern: `#[path = "manyhands-cli/<name>.rs"] mod <name>;`, with `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`.
- **No dead code.** `cargo clippy -- -D warnings` fails on an unused function in the bin, so add each helper in the task that first calls it.
- **Runtime.**
  - The CLI spawns no thread and no child process.
  - The transport starts before the service opens. Help, version, usage errors and unavailable commands touch neither.
  - All errors before dispatch have a null `request_id`. Refusals after opening keep a valid parsed or allocated ID (C2 owner clarification, 2026-10-22). JSON mode writes one envelope to stdout and nothing to stderr.
- **Content and secrets.**
  - No type that holds input derives or implements `Debug`, `Display` or `Serialize`.
  - No input value, body, source, backend text or SQL reaches a diagnostic the CLI writes.
  - Library envelopes pass through unchanged, and so does the fill read's answer (decision 5). JSON `message` is always `ResultCode::message()`.
- **Test isolation.**
  - Every CLI child goes through `tests/support/cli.rs` or `tests/support/pty.rs`, and starts without a controlling terminal unless the pseudo-terminal helper gives it one.
  - Each fixture writes `core.autocrlf = false` and no identity into the test home (fact 33).
  - Tests never touch developer keys, global Git configuration, the real terminal or real application data.
  - The identity cases call `require_no_system_identity()`.
- **Pre-existing tests.** Only named edit E1 may change a pre-existing test or golden. Any other semantic edit, including a new lint on an old test, stops for owner review.
- **Test runs (owner, 2026-10-17, 2026-10-19 and 2026-10-24).**
  - Each task runs only the targets and filters its steps name.
  - Cargo takes **one** positional test filter per command, so split rather than list two.
  - Clippy in a task step is `cargo clippy --all-targets --all-features --locked -- -D warnings`, so the bin's `cfg(test)` code is linted.
  - The Part checkpoints (Tasks 4 and 7) run the checkpoint gate. The full suite runs once, in Task 9, because this Cycle is not worked with the `working-on-a-ticket` skill.
  - C3 changes no desktop code, so the desktop-only gate does not apply. The CLI binary is checked with no features.
- **Checkpoint gate.**

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract
  ```

  The test line includes `--all-features`, because C2's gate without it missed the `preserve_order` failures. After the gate:
  1. `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`;
  2. an independent review of the Part's commit range against the Cycle, the design and this plan;
  3. fix or rule every finding before the next Part;
  4. add a ticket comment.
- **Goldens.**
  - Update with `MANYHANDS_UPDATE_GOLDEN=1`, never under `CI`.
  - Read every regenerated golden's diff before committing. For E1, the diff may change only the newly available verb's availability, repository rule and options.
  - Register new cases in `CLI_CASES` or `CLI_TEXT_CASES` (`tests/support/cli.rs:217`, `:304`).
  - Each verb's goldens follow the design's naming: `<resource>_<verb>_ok`, at least one `<resource>_<verb>_<code>`, and `<resource>_<verb>_text`, plus `<resource>_repair_preview` for the repairs. Hyphens become underscores. Each task names the refusal codes it pins.
- **No subagents.** Implementers never dispatch subagents.
- **Commits.** Commit at the end of every task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Ticket comments.** Add one at Task 0, at each task's completion, at each Part checkpoint, after dogfooding and at review-ready.
  - Write each under `.manyhands/comments/01M4CC0VMV81DFRGJ4Z76489XM/<ULID>.md` in the canonical schema; `comment add` is C4's.
  - Take the ULID from the built CLI's `id new`, and `created_at` from the clock, in UTC.
- **Authorization.**
  - Pushing the ticket branch is the standing habit.
  - Never push another branch, open a pull request, merge, close the ticket or clean up without separate owner authorization. The exception is Task 9's dogfooding pushes, which its go-ahead names.
  - Native CI runs after merge, on main, only with the owner's approval of that run.

## Review Focus

These are inputs the spec implies but no table in it lists. Each has its test in the owning task.

1. **A body with CRLF line endings, a lone CR, non-BMP characters and no final newline.** `document create` and a body-only `document save` write it byte for byte. A save that omits `body` keeps it byte for byte. Task 3, Step 6.
2. **A path that escapes the repository** (`../x.md`, an absolute path, or `docs/../../x.md`), given to `path observe --path`, `document create`'s `path` or `document move`'s `destination_path`. The library refuses it, nothing outside the repository is read or written, and the diagnostic does not echo it. Task 1, Step 5 and Task 3, Steps 5 and 8.
3. **`document move` to the path the document already has.** With an editing context, it is a no-op and nothing is committed. A document on primary alone answers `external_change` (fact 15). Task 3, Step 8.
4. **`deps` that names the ticket itself, or repeats an entry.** A self-reference is `relationship_cycle` naming only the ticket. A repeat is removed silently and the rest sorted. `ticket check` agrees with both. Task 5, Step 5.
5. **A path holding terminal escape sequences or a newline,** given to `path observe`. The human output and every diagnostic escape it, so it cannot repaint the terminal or forge a line. Task 1, Step 5.

---

## File Map And Task Order

| Task | Part | Creates | Modifies |
| --- | --- | --- | --- |
| 0 | — | `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md` | `docs/research/wave-03-api-audit.md` |
| 1 | A | `human_authoring.rs`, `human_authoring_tests.rs`, `tests/cli/authoring_reads.rs` | `src/bin/manyhands-cli.rs`, `catalog.rs`, `parse.rs`, `parse_tests.rs` (additions only), `dispatch.rs`, `human.rs` (one helper made `pub(super)`), `tests/cli.rs`, `tests/support/cli.rs` (new golden names), help goldens (E1) |
| 2 | A | `authoring.rs`, `authoring_tests.rs`, `tests/cli/index.rs`, `tests/cli/folders.rs`, schema and example `folder_create` | `src/bin/manyhands-cli.rs`, `admin.rs` (two helpers made `pub(crate)`), `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E1) |
| 3 | A | `fill.rs`, `fill_tests.rs`, `tests/cli/documents.rs`, schemas and examples `document_create`, `document_save`, `document_move` | `src/bin/manyhands-cli.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E1) |
| 4 | A | `tests/cli/authoring_exits.rs` | `tests/cli.rs` |
| 5 | B | `tests/cli/ticket_authoring.rs`, schemas and examples `ticket_create`, `ticket_save`, `ticket_slug_assign` | `input.rs`, `input_tests.rs`, `fill.rs`, `fill_tests.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1) |
| 6 | B | `tests/cli/repair.rs`, schemas and examples `document_repair`, `ticket_repair` | `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1) |
| 7 | B | — | `tests/cli/authoring_exits.rs` |
| 8 | — | — | `docs/RFC/cli-contract.md`, `docs/RFC/ticket-relationships-and-short-codes.md`, `docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md` |
| 9 | — | — | The Cycle document, design, execution ledger, ticket and comments, `AGENTS.md`, `.agents/skills/file-ticket/SKILL.md`, and the six target tickets' branches (dogfooding) |

Tasks run in order. Tasks 1–6 each flip verbs from `planned(…, "C3")` to `Available("C3")` and edit the same files, so none can run in parallel. A verb becomes available only in the task that dispatches it, gives it its schema and example, and adds its goldens.

---

## Task 0: Baseline, Audit Refresh And Ledger

**Files:**
- Create: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`
- Modify: `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Confirm the base.**
  - `git fetch origin`, then `git merge origin/main`, then `git merge-tree --write-tree HEAD origin/main`.
  - Read `git show origin/main:AGENTS.md`.
  - For each of `01M4PB82G07RZSQMNJZN672S3W`, `01M4PB80T7SVB23TQ6466RDHCB` and `01M4PB7YZMBFAFFM94AZY08HWS`, run `git log --oneline origin/main --grep <ULID>`. A merged one changes a known limit and possibly R-12, and the controller rules before Task 1.
  - Record HEAD and `origin/main`.
- [ ] **Step 2: Re-check every Baseline Fact** (1–37) against the source, and record each as confirmed or changed. Also record:
  - fact 17: whether a no-op save of an item without an editing context creates that context;
  - whether `initials_required` frees its request record as `identity_required` does (fact 11);
  - the codes F2's cache-loss tests give for a committed save retried after the database is lost (`tests/mutation_replay/cache_loss.rs`) and for a confirmation presented after it (`tests/mutation_confirmation.rs::a_confirmation_is_not_found_once_the_cache_is_lost`). Tasks 3, 5 and 6 assert them.

  A changed fact that alters a task stops here for a controller ruling.
- [ ] **Step 3: Prove E1 is the only named edit.**

  ```sh
  grep -rln -E 'path observe|ticket check|index (refresh|rebuild)|folder create|document (create|save|move|repair)|ticket (create|save|repair|slug-assign)|"C3"' src/bin/manyhands-cli tests/cli tests/cli_remote tests/cli.rs tests/support/cli.rs tests/fixtures/cli_v1
  grep -rn -E 'Availability::(Available|Planned)|available.*len\(\)|"available"\] == true' src/bin/manyhands-cli tests/cli
  ```

  The first command is expected to hit `catalog.rs`, `catalog_tests.rs` and the three E1 goldens. Every hit of the second must be one of these:
  - production code in `catalog.rs`, `parse.rs` or `startup.rs`;
  - a test filtered to C1 or C2;
  - one of fact 32's unfiltered tests;
  - a test-local `CommandSpec` literal in `parse_tests.rs`, `startup_tests.rs`, `mutate_tests.rs` or `admin_tests.rs`.

  Any other hit stops for an owner ruling.
- [ ] **Step 4: Baseline checks.**

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract
  ```

  Record the counts.
- [ ] **Step 5: Refresh the API audit.**
  - Add a section, "Wave 03 CLI Cycle C3 refresh (2026-10-24)".
  - For each of the thirteen C3 verbs, update its existing row in place. Record the F1 or F2 call and its signature, whether it needs confirmation, its input type, and **every result code it can return**. Read the codes from its binding (`src/repository/mutation/bind/*.rs`), `outcome.rs` and the boundary (`execute.rs`, `confirm.rs`), or from the read.
  - Correct the stale "absent" claims. Do not add a second row for any verb.
  - Under `#### C3 exit-coverage union`, give the union of those codes in one fenced `text` block, space-separated.
  - Under `#### C3 no-op capability`, list which of the eleven mutations can be a no-op for a fresh request on an item with a context, with the binding rule (R-14, facts 14–17), and give the reason for each that cannot.
- [ ] **Step 6: Create the execution ledger.** Give it front matter like the design's and a new document ULID from `id new`. Its sections are:
  - baseline;
  - confirmed facts;
  - audit refresh;
  - Cycle decisions 1–7, rulings R-1 to R-18 and named edit E1;
  - the planning review record;
  - an empty task log.
- [ ] **Step 7: Commit** `docs: record C3 baseline and refresh the API audit`. Add a ticket comment and push the ticket branch.

## Task 1: Usage Privacy And The Two Reads

**Files:**
- Create: `src/bin/manyhands-cli/human_authoring.rs`, `human_authoring_tests.rs`, `tests/cli/authoring_reads.rs`
- Modify: `src/bin/manyhands-cli.rs`, `catalog.rs`, `parse.rs`, `parse_tests.rs` (new tests only), `dispatch.rs`, `human.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E1)

**Interfaces:**
- Consumes:
  - `RepositoryService::observe_path(&self, &ResolvedRepository, Option<&Path>, &Path) -> Result<PathObservationDto, ReadError>`;
  - `RepositoryService::check_ticket_relationships(&self, &ResolvedRepository, &ItemId, &ProposedRelationships) -> Result<RelationshipCheckDto, ReadError>`;
  - `dispatch::{resolve, item_id, read_output, value}`.
- Produces:
  - `human_authoring::path_observation(&PathObservationDto) -> render::Human`;
  - `human_authoring::relationship_check(&RelationshipCheckDto) -> render::Human`;
  - the R-2 rule.

- [ ] **Step 1: Write the failing R-2 test** in `parse_tests.rs`. A stray positional value is what clap echoes (fact 24):

  ```rust
  #[test]
  fn a_c3_verbs_usage_error_never_echoes_a_value() {
      let invocation = invoke(&["ticket", "check", "--repo", "r", "--id", "X", "SECRET-VALUE"]);
      let Action::Usage { detail, .. } = invocation.action else { panic!("usage expected") };
      assert!(!detail.contains("SECRET-VALUE"), "{detail}");
  }
  ```

  `invoke` is `parse_tests.rs:4`, and `Action::Usage { command, detail }` is `parse.rs:24`.
- [ ] **Step 2: Make the two reads available, and see the test fail.** A planned verb accepts trailing arguments, so first replace the two `planned` lines for `path observe` and `ticket check` in `catalog.rs` with the entries in Step 3. Then run:

  `devenv shell -- cargo test --locked --bin manyhands-cli a_c3_verbs_usage_error_never_echoes_a_value`

  Expected: FAIL. The detail is clap's "unexpected argument 'SECRET-VALUE'".
- [ ] **Step 3: Implement.**
  - In `parse.rs:120`, change `spec.delivered_by()=="C2"` to `spec.delivered_by() != "C1"`.
  - The catalog entries, which keep the planned summaries:

    ```rust
    const PATH_OBSERVE: &[OptionSpec] = &[
        REPO,
        option("--path", Some("path"), true, "Repository-relative path to observe."),
        option("--context", Some("path"), false, "Context worktree path; the repository root when omitted."),
    ];
    const TICKET_CHECK: &[OptionSpec] = &[
        REPO,
        ID,
        option("--deps", Some("ULID,..."), false, "Proposed dependencies, comma-separated; give once."),
        option("--parent", Some("ULID"), false, "Proposed parent."),
    ];
    // in COMMANDS, in place of the planned lines:
    CommandSpec { resource: "path", verb: "observe", summary: "Observe a path for authoring or repair.",
        availability: Availability::Available("C3"), repo: Some(RepoRule::Required),
        options: PATH_OBSERVE, kind: Kind::Read },
    CommandSpec { resource: "ticket", verb: "check", summary: "Check ticket relationships.",
        availability: Availability::Available("C3"), repo: Some(RepoRule::Required),
        options: TICKET_CHECK, kind: Kind::Read },
    ```

  - In `parse::validate` (`parse.rs:191`), after the `show` rule: `ticket check` requires at least one of `--deps` and `--parent`, with the message "ticket check needs --deps or --parent.". The `required` flags on `--path` and `--id` are enforced by the existing rule.
- [ ] **Step 4: Run the test and see it pass.** Use the same command as Step 2.
- [ ] **Step 5: Write the binary tests** in `tests/cli/authoring_reads.rs`, registered in `tests/cli.rs`. Copy `run`, `checked` and `golden` from `tests/cli/content.rs:20–60`. Documents come from `support::items::contract_repository()`, and tickets from `relationship_repository()`. Each case:
  - `path_observe_gives_the_absent_token_for_an_empty_document_path`: `exists` is false; the data validates against `path_observation.schema.json`; golden `path_observe_absent`.
  - `path_observe_gives_the_same_token_as_document_show_for_a_file`.
  - `path_observe_reports_another_items_file_in_a_named_context`: make a context with `items::create_document_context` (`tests/support/items.rs:156`), then observe a different document's canonical path with `--context <that worktree>` (`items::context_worktree`, `:179`). The answer is `ok` with `exists` true (Cycle contract item 6).
  - `path_observe_refuses_an_inactive_context_and_escaping_paths`: a directory that is not an active context is `invalid_path`, exit 2. `../x.md`, an absolute path and `docs/../../x.md` each get the library's refusal. Stderr never contains the path text (Review Focus 2). Refusal golden `path_observe_invalid_path`.
  - `path_observe_escapes_a_path_holding_control_characters`: the path is `docs/a\u{1b}[2J\nb.md`. In human mode stdout shows it escaped, with no raw ESC byte and no extra line (Review Focus 5).
  - `ticket_check_reports_resolved_unresolved_and_a_dependency_cycle`. D (`RELATED_D`) already depends on C, and both are open, so proposing that C depend on D gives `ok`, exit 0, with `data.rejection.code == "relationship_cycle"`. An unknown ULID is listed in `unresolved`. The data validates against `relationship_check.schema.json`. Golden `ticket_check_rejected`.
  - `ticket_check_reports_a_parent_cycle`.
  - `ticket_check_proposes_exactly_what_it_is_given`. E and F (`RELATED_E`, `RELATED_F`) already form a dependency cycle and a parent cycle. For E, `--parent <RELATED_B>` alone reports no rejection, because the omitted `--deps` proposes none (fact 7). `--deps <RELATED_F>` reports the dependency cycle.
  - `ticket_check_refuses_malformed_or_empty_elements`: `--deps <RELATED_A>,,<RELATED_B>` (two valid IDs around an empty element) and `--deps not-a-ulid` are `invalid_id`, exit 2. Neither option given is `usage_error`. Golden `ticket_check_invalid_id`.
  - `both_reads_write_nothing`: worktree bytes and `git for-each-ref` output are equal before and after every case above.
  - Human mode for both verbs, with text goldens `path_observe_text` and `ticket_check_text`, split streams and empty JSON stderr.
- [ ] **Step 6: Implement the dispatch arms and the human text.**

  ```rust
  fn path_observe(service: &RepositoryService, args: &ArgMatches) -> Result<Envelope<PathObservationDto>, ReadError> {
      let repo = resolve(service, args)?;
      let context = args.get_one::<String>("context").map(PathBuf::from);
      let data = service.observe_path(&repo, context.as_deref(), Path::new(value(args, "path")?))?;
      Ok(Envelope::read_success("path observe", repo.scope(), data))
  }

  fn ticket_check(service: &RepositoryService, args: &ArgMatches) -> Result<Envelope<RelationshipCheckDto>, ReadError> {
      let repo = resolve(service, args)?;
      let id = item_id(args, &repo)?;
      let parse = |text: &str| text.parse::<ItemId>().map_err(|_| ReadError::invalid_id().with_scope(repo.scope()));
      let deps = match args.get_one::<String>("deps") {
          Some(list) => list.split(',').map(parse).collect::<Result<Vec<_>, _>>()?,
          None => Vec::new(),
      };
      let parent = args.get_one::<String>("parent").map(|text| parse(text)).transpose()?;
      let scope = Scope { item_id: Some(id.to_string()), ..repo.scope() };
      let data = service.check_ticket_relationships(&repo, &id, &ProposedRelationships { deps, parent })?;
      Ok(Envelope::read_success("ticket check", scope, data))
  }
  // arms:
  "path observe" => return read_output(&name, json, path_observe(service, args), crate::human_authoring::path_observation),
  "ticket check" => return read_output(&name, json, ticket_check(service, args), crate::human_authoring::relationship_check),
  ```

  - `human_authoring.rs` writes the lines in the design's Reads section, escaping each value with `render::field` (fact 28).
  - Make `human.rs`'s two note helpers, `incomplete` and `index_note` (`human.rs:24–34`), `pub(super)`. Use them as C1's list formatters do, when `complete` is false or the index is not current.
  - Unit tests: an escape sequence in a path, a rejection line, and an empty `unresolved`.
- [ ] **Step 7: Regenerate the E1 goldens** with `MANYHANDS_UPDATE_GOLDEN=1`, through `tests/cli/help.rs`'s three golden tests. Read each diff: only the two verbs' availability, repository rule and options may change.
- [ ] **Step 8: Run.**

  ```sh
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli
  devenv shell -- cargo test --all-features --locked --test cli authoring_reads
  devenv shell -- cargo test --all-features --locked --test cli help
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  ```

- [ ] **Step 9: Commit** `feat(cli): path observe and ticket check`, and add a ticket comment.

## Task 2: Index Verbs And Folder Creation

**Files:**
- Create: `src/bin/manyhands-cli/authoring.rs`, `authoring_tests.rs`, `tests/cli/index.rs`, `tests/cli/folders.rs`, `schemas/input/v1/folder_create.schema.json`, `tests/fixtures/cli_input_v1/folder_create.json`
- Modify: `src/bin/manyhands-cli.rs`, `admin.rs` (`input_failure` and `name` made `pub(crate)`), `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E1)

**Interfaces:**
- Consumes: `admin::{flags, input_object, mutation_root, input_failure}`, `mutate::{run, Plan}`, `Mutation::{IndexRefresh, IndexRebuild, FolderCreate}`.
- Produces: `authoring::{index_refresh, index_rebuild, folder_create}`, each with C2's per-verb signature `fn(&RepositoryService, &mut Session, &'static CommandSpec, &ArgMatches) -> Output`.

- [ ] **Step 1: Write the failing binary tests** in `tests/cli/index.rs`. The fixture is `support::items::contract_repository()` with `CliEnv::new(enabled.data_directory.path().to_owned())`. Cases:
  - `index_refresh_and_rebuild_succeed_and_change_nothing_canonical`: `ok`, `discovery: current`, null data; worktree and refs equal. Goldens `index_refresh_ok` and `index_rebuild_ok`.
  - `index_verbs_never_fetch`: configure a remote with `git2` that points at a local bare repository holding a branch the fixture does not have. After refresh and rebuild, `refs/remotes/*` is unchanged.
  - `index_verbs_need_a_request_id_without_a_terminal`: `--json` without `--request-id` is `usage_error`, exit 2.
  - `index_rebuild_restores_a_deleted_database` (R-12, fact 18). Remove `manyhands.sqlite3-wal` and `manyhands.sqlite3-shm`, then delete `manyhands.sqlite3`. `document list --json` answers `repository_not_registered` with `"recovery": []`, as the read gives it. `index rebuild` gives `ok`. `document list` and `ticket list` then list the fixture's items again. Worktree bytes and refs equal what they were before.
  - `index_rebuild_restores_a_database_replaced_by_other_bytes`: remove the WAL files, then write 4096 bytes of `0xA5` over `manyhands.sqlite3`. `document list` answers `index_unavailable` with `index.rebuild`, and the rest is as above.
  - Goldens: `index_refresh_ok`, `index_rebuild_ok`, `index_refresh_usage_error`, `index_refresh_text` and `index_rebuild_text`.
  - `index_verbs_replay_a_finished_request`: the same request again is answered from the record.
  - `index_verbs_refuse_input_and_id`: `--input -` and `--id` are usage errors.
  - Human mode for both verbs, with text goldens.
- [ ] **Step 2: Write the failing binary tests** in `tests/cli/folders.rs`:
  - `folder_create_makes_an_empty_folder_and_lists_it`: `{"path": "docs/new-folder"}` gives `ok`. The directory exists with no file, nothing is committed, and `folder list` lists it. Golden `folder_create_ok`.
  - `folder_create_of_an_existing_folder_is_a_noop`: `docs/notes`, which the fixture holds, with a fresh request.
  - `folder_create_refuses_an_occupied_path_and_a_path_outside_docs`: `occupied_path` and `invalid_path` (fact 14), with nothing made. Golden `folder_create_occupied_path`.
  - `folder_create_replays_a_finished_request`.
  - Human mode, with golden `folder_create_text`.
- [ ] **Step 3: Run the tests and see them fail** (`command_unavailable`):

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli index
  devenv shell -- cargo test --all-features --locked --test cli folders
  ```

- [ ] **Step 4: Implement.** In `authoring.rs`:

  ```rust
  pub fn index_refresh(service: &RepositoryService, session: &mut Session, spec: &'static CommandSpec, args: &ArgMatches) -> Output {
      root_only(service, session, spec, args, |root| Mutation::IndexRefresh(IndexRefreshInput { root }))
  }
  pub fn index_rebuild(service: &RepositoryService, session: &mut Session, spec: &'static CommandSpec, args: &ArgMatches) -> Output {
      root_only(service, session, spec, args, |root| Mutation::IndexRebuild(IndexRebuildInput { root }))
  }
  fn root_only(service: &RepositoryService, session: &mut Session, spec: &'static CommandSpec, args: &ArgMatches,
      build: impl Fn(PathBuf) -> Mutation) -> Output {
      let (flags, root) = match (|| {
          let flags = admin::flags(session, spec, args)?;
          let root = admin::mutation_root(session, spec, args, flags.request_id)?;
          Ok((flags, root))
      })() { Ok(v) => v, Err(out) => return out };
      mutate::run(service, session, spec, flags, Plan { identity_prompt: false, retry: true },
          &mut |_| build(root.clone()), &mut std::thread::sleep)
  }
  pub fn folder_create(service: &RepositoryService, session: &mut Session, spec: &'static CommandSpec, args: &ArgMatches) -> Output {
      let (flags, path, root) = match (|| {
          let flags = admin::flags(session, spec, args)?;
          let object = admin::input_object(session, spec, args, flags.request_id)?;
          let path = (|| { object.only(&["path"])?; object.string("path") })()
              .map_err(|e| admin::input_failure(session, spec, flags.request_id, e))?;
          let root = admin::mutation_root(session, spec, args, flags.request_id)?;
          Ok((flags, path, root))
      })() { Ok(v) => v, Err(out) => return out };
      mutate::run(service, session, spec, flags, Plan { identity_prompt: false, retry: true },
          &mut |_| Mutation::FolderCreate(FolderCreateInput { root: root.clone(), path: path.clone() }),
          &mut std::thread::sleep)
  }
  ```

  Then:
  - **Catalog.** The index verbs are `Kind::Mutation { confirmed: false, input: InputRule::Refused }` with `options: &[REPO, REQUEST_ID, PREPARE, CONFIRM]`, as `repo remove` has (`catalog.rs:437`). `folder create` is `input: InputRule::Required` with `MUTATION_INPUT` (`:227`).
  - **Dispatch.** Add the three arms.
  - **Schema.**

    ```json
    {
      "$schema": "https://json-schema.org/draft/2020-12/schema",
      "title": "Document folder creation input",
      "type": "object",
      "properties": {"path": {"type": "string"}},
      "required": ["path"],
      "additionalProperties": false
    }
    ```

  - **Example.** `{"path": "docs/new-folder"}`.
- [ ] **Step 5: Run Step 3's commands and see them pass,** then run `devenv shell -- cargo test --all-features --locked --test cli input_schemas`.
- [ ] **Step 6: Regenerate the E1 goldens and read the diffs.**
- [ ] **Step 7: Run Clippy and commit** `feat(cli): index refresh, index rebuild and folder create`. Add a ticket comment.

## Task 3: The Save Patch And The Document Verbs

**Files:**
- Create: `src/bin/manyhands-cli/fill.rs`, `fill_tests.rs`, `tests/cli/documents.rs`, schemas and examples `document_create`, `document_save`, `document_move`
- Modify: `src/bin/manyhands-cli.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, help goldens (E1)

**Interfaces:**
- Consumes: `RepositoryService::{resolve_repository, show_item}`, `ItemDto`, `ItemDtoKind`, `DocumentDraft`, `ReadError::to_envelope`, `render::failure_output`.
- Produces:

  ```rust
  // fill.rs
  pub struct DocumentFields { pub title: Option<String>, pub body: Option<String> }
  pub enum FillError { Unfillable }
  pub fn needs_read_document(fields: &DocumentFields) -> bool;
  pub fn document_draft(fields: DocumentFields, read: Option<&ItemDto>) -> Result<DocumentDraft, FillError>;
  /// The one read: a ReadError, or an item of the other kind, becomes the
  /// read's answer under `spec`'s command name, with `request_id` kept
  /// (decision 5).
  pub fn read_item(service: &RepositoryService, session: &Session, spec: &CommandSpec,
      root: &Path, id: &ItemId, kind: ItemDtoKind, request_id: Option<RequestId>) -> Result<ItemDto, Output>;
  /// `item_not_found` with the item scope and `request_id` kept: the answer
  /// for any `FillError`, and for a read of the other kind.
  pub fn item_not_found(session: &Session, spec: &CommandSpec, root: &Path, id: &ItemId,
      request_id: Option<RequestId>) -> Output;
  ```

  - `read_item` builds a failure as `ReadError::to_envelope(<command>)`, with `request_id` set. A wrong kind is `ReadError::new(ResultCode::ItemNotFound).with_scope(<item scope>)`, as `dispatch.rs:330` builds it. It renders through `render::failure_output`.
  - A `FillError` from `document_draft` is turned into that same `item_not_found` by `fill::item_not_found(session, spec, root, id, request_id)`.
  - Task 5 adds `TicketFields` and `ticket_draft` beside these.

- [ ] **Step 1: Write the failing unit tests** in `fill_tests.rs`. They are pure, with no repository, and compare fields one by one, because drafts derive only `Clone` (fact 3):
  - both fields given: no read is needed, and the draft is the input's;
  - `title` omitted: it comes from the read;
  - `body` omitted: it comes from the read, byte for byte, including `""`;
  - the read is a ticket, or lacks a field: `Unfillable`.
- [ ] **Step 2: Run them and see them fail:** `devenv shell -- cargo test --locked --bin manyhands-cli fill::`.
- [ ] **Step 3: Implement `fill.rs`.**

  ```rust
  pub fn needs_read_document(f: &DocumentFields) -> bool { f.title.is_none() || f.body.is_none() }

  pub fn document_draft(f: DocumentFields, read: Option<&ItemDto>) -> Result<DocumentDraft, FillError> {
      if read.is_some_and(|item| item.kind != ItemDtoKind::Document) { return Err(FillError::Unfillable) }
      let take = |given: Option<String>, current: Option<&String>| given.or_else(|| current.cloned()).ok_or(FillError::Unfillable);
      Ok(DocumentDraft {
          title: take(f.title, read.and_then(|i| i.title.as_ref()))?,
          body: take(f.body, read.and_then(|i| i.body.as_ref()))?,
      })
  }
  ```

  A nonconforming file reads as a bare item without `title` or `body` (`items.rs:1811–1822`). So the `ok_or` can fail. `document_save` maps every `FillError` to `fill::item_not_found`, which gives the same envelope as a wrong kind.
- [ ] **Step 4: Run Step 2's command and see it pass.**
- [ ] **Step 5: Write the failing binary tests for create** in `tests/cli/documents.rs`. The fixture is `contract_repository()`, with no remote and C2's `autocrlf` home.
  - `document_create_commits_one_checkpoint_offline`: the input is `{"item_id": <id new>, "path": "docs/new.md", "title", "body"}`. The answer is `ok`, exit 0, with `checkpoint: committed`, `publication: not_requested`, and one commit on `manyhands/document/<id>`. The data validates against `document_mutation.schema.json`. Golden `document_create_ok`.
  - `document_create_replays_and_refuses_a_used_or_escaping_path`:
    - the same request again replays, with no second commit;
    - a fresh request for `docs/guide.md` is `occupied_path`, with golden `document_create_occupied_path`;
    - `../x.md` and `docs/../../x.md` are refused, and stderr does not contain them;
    - a malformed `item_id` is `invalid_input`, and stderr does not contain it.
- [ ] **Step 6: Add the save tests,** one per document row of the design's save-patch table. The second actor is `enabled.service` calling the library directly.
  - `a_body_only_save_keeps_every_other_value`: the item is made by `document create`. Compare by value, and the body byte for byte.
  - `a_save_that_omits_every_field_is_a_noop_for_a_library_written_document`: `noop`, no commit, the file byte-identical.
  - `a_save_that_omits_every_field_normalizes_a_hand_written_document`: `docs/guide.md`. One commit, with values equal (fact 9).
  - `a_title_change_by_another_actor_makes_a_body_only_save_external_change`.
  - `a_retry_after_lost_output_replays`.
  - `a_retry_after_another_actor_changed_an_omitted_field_is_request_mismatch`: one commit, and `request show` gives the stored result.
  - `a_document_save_of_a_ticket_id_is_item_not_found_with_its_scope`: `scope.item_id` is set and the request ID is kept (decision 5). Golden `document_save_item_not_found`.
  - `a_never_enabled_root_answers_by_whether_a_field_is_omitted` (decision 7): a `born_repository` that was never enabled. With `title` omitted the answer is `repository_not_registered`, exit 3, with `"recovery": []`. With every field sent it is `repository_not_enabled`, exit 2. Golden `document_save_repository_not_registered`.
  - `a_lost_cache_answers_by_whether_a_field_is_omitted` (decision 7): delete the database as in Task 2. With `title` omitted the answer is `repository_not_registered` with `"recovery": []`. With every field sent it is `repository_not_registered` with `index.rebuild`.
  - `a_partial_save_whose_omitted_field_was_then_edited_completes_when_every_field_is_resent`: set up as in Step 7, then edit the title in the context's worktree without committing. The CLI retry with `title` omitted answers `request_mismatch`. A retry with every field, valued from `git show <commit>:<path>`, completes with no second commit.
  - `a_body_round_trips_byte_for_byte` (Review Focus 1): the body is `"line one\r\nline two\rthree 🦀\nno final newline"`. After create and after a title-only save, the bytes after the front matter equal it exactly.
  - `a_committed_save_retried_after_cache_loss_makes_no_second_commit`: delete the database as in Task 2, run `index rebuild`, then retry the same request. The answer is Task 0's recorded code, and there is no second commit.
  - `a_human_save_reports_its_progress_on_stderr`: stderr has at least one `progress:` line. In JSON mode stderr is empty.
- [ ] **Step 7: Add the interrupted-refresh case** `commit_success_refresh_failure_is_completed_by_the_same_request` (R-11):

  ```rust
  let tip = |root: &Path| git2::Repository::open(root).unwrap()
      .find_reference(&format!("refs/heads/manyhands/document/{id}")).unwrap().target().unwrap();
  {
      let failing = support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit)
          .open_service(&f.env.data);
      let stopped = crate::support::mutation::execute_with(&failing, REQUEST,
          Mutation::DocumentSave(DocumentSaveInput { root: root.clone(), id, observation: Some(token.clone()),
              draft: DocumentDraft { title: title.clone(), body: new_body.clone() } }));
      assert_eq!(stopped.outcome, Outcome::Partial);
      assert_eq!(stopped.effects.checkpoint.as_str(), "committed");
  }
  let before = tip(&root);
  // The CLI omits `title`: the fill reads what the stopped attempt committed.
  let value = checked(&["document", "save", "--repo", root_str, "--id", &id_str, "--json",
      "--request-id", REQUEST, "--input", "-"], &f.env, json!({"observation": token, "body": new_body}).to_string().as_bytes(), 0, "ok", Some("document_mutation.schema.json"));
  assert_eq!(value["effects"]["discovery"], "current");
  assert_eq!(tip(&root), before);
  ```

  The document is one that `document create` made, so it has a context. `execute_with`'s signature is `tests/support/mutation.rs:487`. Copy its block scoping from `tests/cli/keys.rs:744–753`. The item listed afterwards is the saved one.
- [ ] **Step 8: Add the move tests.** Each starts from a document made by `document create`, so it has a context, unless the case says otherwise.
  - `document_move_checkpoints_both_paths`.
  - `a_first_move_of_a_document_on_primary_uses_the_roots_token`: `docs/guide.md` has no context. Its destination token comes from `path observe` without `--context`, and the move succeeds.
  - `a_move_onto_a_file_is_occupied_path`: exit 2, with source and destination unchanged. Golden `document_move_occupied_path`.
  - `a_destination_token_from_the_wrong_context_is_external_change`.
  - `a_destination_that_gained_a_file_after_it_was_observed_is_external_change`.
  - `a_move_to_its_own_path_is_a_noop`. With a context; also a document on primary alone, which answers `external_change` (fact 15, Review Focus 3).
  - `a_move_to_an_escaping_destination_is_refused` (Review Focus 2).
  - `a_stale_source_token_is_external_change`.
  - `a_move_replays_a_finished_request`.
- [ ] **Step 9: Add human mode for each document verb.** Each gets one success, one refusal and one no-op, with text goldens `document_create_text`, `document_save_text` and `document_move_text`. The JSON goldens are:
  - `document_create_ok`, `document_create_occupied_path`;
  - `document_save_ok`, `document_save_item_not_found`, `document_save_repository_not_registered`;
  - `document_move_ok`, `document_move_occupied_path`.
- [ ] **Step 10: Run the tests and see them fail:** `devenv shell -- cargo test --all-features --locked --test cli documents`.
- [ ] **Step 11: Implement the three verbs in `authoring.rs`.**
  - **`document_create`** reads `item_id`, `path`, `title` and `body`. It parses `item_id` into `ItemId`; a failure is `invalid_input` naming the field. It builds `Mutation::DocumentCreate`.
  - **`document_save`**:
    - read `observation` and the optional `title` and `body` into `DocumentFields`;
    - when `fill::needs_read_document` holds, call `fill::read_item` once with `ItemDtoKind::Document`;
    - call `fill::document_draft`;
    - build `Mutation::DocumentSave { observation: Some(observation), .. }`. The draft is computed before `mutate::run`, and the `build` closure only clones it.
  - **`document_move`** reads `observation`, `destination_path` and `destination_observation`, and builds `Mutation::DocumentMove`.
  - **`--id`** parses with C1's rule. A malformed one is `invalid_id` through `mutate::refusal`, which keeps the request ID.
  - **Catalog.** Add the entries: `MUTATION_INPUT`, plus `ID` for save and move.
  - **Dispatch.** Add the arms.
  - **Schemas and examples.** For example:

    ```json
    {"$schema": "https://json-schema.org/draft/2020-12/schema", "title": "Document save input", "type": "object",
     "properties": {"observation": {"type": "string"}, "title": {"type": "string"}, "body": {"type": "string"}},
     "required": ["observation"], "additionalProperties": false}
    ```

- [ ] **Step 12: Run the tests and see them pass.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli documents
  devenv shell -- cargo test --all-features --locked --test cli input_schemas
  devenv shell -- cargo test --locked --bin manyhands-cli fill::
  ```

- [ ] **Step 13: Regenerate the E1 goldens, run Clippy, and commit** `feat(cli): document create, save and move with the save patch`. Add a ticket comment.

## Task 4: Part A Evidence And Checkpoint

**Files:**
- Create: `tests/cli/authoring_exits.rs`
- Modify: `tests/cli.rs`

- [ ] **Step 1: Write the C3 exit table.** It has the shape of `tests/cli/exits.rs` (`ROWS`, `REASONS`, `How`), in its own module with its own constants.
  - **`the_c3_table_and_reasons_account_for_every_code_of_the_audit`.** It reads the `#### C3 exit-coverage union` block from `docs/research/wave-03-api-audit.md` with `include_str!`. It asserts that the rows' codes and `REASONS`' codes are disjoint, and that their union equals the block by **set equality**.
  - **`busy_is_reached_with_the_lease_held`.** It runs a `document save` while `support::hold_lease_in_child_for_test` holds the lease, as `tests/cli/exits.rs:512` does. The answer is `busy`, exit 5, after the CLI's bounded retries.
  - **`every_c3_exit_class_is_covered`.**
    - Classes 0, 2, 3 and 5 each have a row run through the binary.
    - Class 4 is in `REASONS`, citing `commit_success_refresh_failure_is_completed_by_the_same_request` (decision 6).
    - Class 1 has a row or a reason.
    - Class 130 is Task 6's.

  Each row names a test function that exists in `tests/cli/*.rs`, checked as `every_cli_remote_reference_names_an_existing_case` does. A code that only Part B's verbs produce goes in `REASONS` as "Part B, Task 5/6" for now.
- [ ] **Step 2: Add `c3_confirmed_flags_match_prepare` for Part A.** `--prepare` with a valid input gives `not_confirmable` for each Part A mutation.
- [ ] **Step 3: Add the no-op inventory for Part A,** `c3_noop_inventory_accounts_for_part_a`.
  - For each Part A mutation in Task 0's `#### C3 no-op capability`, it names the no-op test. For each that cannot be a no-op, it names the reason.
  - It enumerates the verbs from `--help --json` with `delivered_by == "C3"`, and recognizes a mutation by its `--request-id` option (fact 29).
- [ ] **Step 4: Run the checkpoint gate** (Global Constraints), and record the counts in the ledger.
- [ ] **Step 5: Run `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`,** then get the independent Part A review. Fix or rule every finding.
- [ ] **Step 6: Commit** `test(cli): C3 Part A exit table and inventories`. Add the Part A checkpoint ticket comment, and push the ticket branch.

## Task 5: List Readers And The Ticket Verbs

**Files:**
- Create: `tests/cli/ticket_authoring.rs`, schemas and examples `ticket_create`, `ticket_save`, `ticket_slug_assign`
- Modify: `input.rs`, `input_tests.rs`, `fill.rs`, `fill_tests.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1)

**Interfaces:**
- Produces:

  ```rust
  // input.rs
  pub enum ListChange { Missing, Null, Value(Vec<String>) }
  impl InputObject {
      pub fn optional_string_list(&self, field: &'static str) -> Result<Option<Vec<String>>, InputError>;
      pub fn tristate_list(&self, field: &'static str) -> Result<ListChange, InputError>;
  }
  // fill.rs
  pub struct TicketFields { pub title: Option<String>, pub ticket_type: Option<String>, pub status: Option<String>,
      pub body: Option<String>, pub project: Tristate, pub team: Tristate }
  pub fn needs_read_ticket(fields: &TicketFields) -> bool;
  pub fn ticket_draft(fields: TicketFields, read: Option<&ItemDto>) -> Result<TicketDraft, FillError>;
  ```

  `Tristate` is `input::Tristate` (`Missing | Null | Value(String)`).

- [ ] **Step 1: Write the failing unit tests for the readers.**
  - An array of strings is read.
  - A missing field is read as missing.
  - `null` is `ListChange::Null` for `tristate_list`, and a wrong type for `optional_string_list`.
  - A non-array is a wrong type.
  - An array holding a number, a null or an object is a wrong type. The error names the field and never the value.
- [ ] **Step 2: Write the failing unit tests for `ticket_draft`.**
  - A missing `project` takes the read's value, including absent.
  - `project: null` is `None`.
  - Each of `title`, `type`, `status` and `body` omitted comes from the read.
  - A document read is `Unfillable`.
  - When every field is given, no read is needed.
- [ ] **Step 3: Run them and see them fail.**

  ```sh
  devenv shell -- cargo test --locked --bin manyhands-cli input::
  devenv shell -- cargo test --locked --bin manyhands-cli fill::
  ```

- [ ] **Step 4: Implement the readers and `ticket_draft`,** in the shape of `document_draft`. `needs_read_ticket` is true when any of `title`, `ticket_type`, `status` or `body` is `None`, or when `project` or `team` is `Tristate::Missing`. Run Step 3's commands and see them pass.
- [ ] **Step 5: Write the failing binary tests** in `tests/cli/ticket_authoring.rs`. Tickets come from `relationship_repository()`, with `contract_repository()`'s `TICKET_A` for `project`.
  - `ticket_create_writes_relationships_and_a_short_code_offline`.
    - The input has `deps` (two IDs, one repeated), `parent` and `initials`.
    - The answer is `ok` with one checkpoint.
    - `ticket show` gives the `deps` sorted and unique, the `parent`, and a `slug` composed from the initials.
    - The data validates against `ticket_mutation.schema.json`. Golden `ticket_create_ok`.
  - `ticket_create_reads_null_deps_and_parent_as_none`.
  - `ticket_create_refuses_a_dependency_cycle_and_a_parent_cycle_before_any_context_exists`: `relationship_cycle`, exit 2. No `manyhands/ticket/<id>` branch and no worktree exist. Golden `ticket_create_relationship_cycle`.
    - **Dependency cycle.** D already depends on `TICKET_ABSENT`, so creating `TICKET_ABSENT` with `deps: [RELATED_D]` closes the cycle.
    - **Parent cycle.** First create a ticket P whose `parent` is a fresh ID X (an unresolved parent, accepted). Then create X with `parent: P`.
  - `ticket_save_refuses_a_dependency_cycle_and_a_parent_cycle`.
  - `ticket_check_and_ticket_save_agree`: for one complete proposal (deps and parent), the rejection IDs are equal.
  - `a_self_reference_is_a_cycle_of_one_and_a_repeat_is_removed` (Review Focus 4).
  - `an_unknown_target_is_accepted_and_listed_unresolved`.
  - The `ticket save` rows of the design's save-patch table, compared by value:
    - `project: null`;
    - `deps: []` and `deps: null`;
    - `deps` omitted;
    - a body-only save;
    - every field omitted, giving `noop` on a library-written ticket;
    - a third party's change;
    - lost output and `request_mismatch`;
    - a document ID, giving `item_not_found`.
  - `a_ticket_in_this_projects_format_takes_deps_from_a_save_of_only_observation_and_deps`. The ticket exists only on its context branch, as the real targets do. Build it this way:
    1. `ticket create` through the CLI, which makes the context;
    2. overwrite its `ticket.md` in the context worktree with the format fact 34 describes: quoted values, `manyhands_*` keys first, unknown `wave` and `cycle`, and the same values;
    3. commit it there with `git2`;
    4. run `index refresh`.

    Then save `{"observation", "deps": [<two IDs>]}`. Every compared field is equal, and `deps` is added.
  - `status_closed_is_ordinary_metadata`: `status: "closed"` saves, and `ticket list --closure open` still lists the ticket.
  - `a_lifecycle_closed_ticket_refuses_save_and_slug_assign`: `ticket_closed`, exit 3. Use `relationship_repository`'s closed ticket.
  - `slug_assign_adds_a_code_and_is_a_noop_when_repeated`: a ticket written without `slug` gets one. The same initials again give `noop` (fact 16).
  - `slug_assign_on_a_ticket_with_another_code_is_slug_already_assigned`: `relationship_repository`'s hand-written `mh-vh-k9x2b` ticket, exit 2.
  - `a_stale_token_on_save_and_slug_assign_is_external_change`.
  - `closure_and_slug_keys_are_refused`: `closed_at`, `closed_by` and `slug` on create and on save are each `invalid_input`, naming the field. Nothing is written.
  - `each_ticket_verb_replays_a_finished_request`.
  - `a_committed_ticket_save_retried_after_cache_loss_makes_no_second_commit`.
- [ ] **Step 6: Add identity and initials recovery** (R-9, R-18). Each test first calls `require_no_system_identity()` (`tests/cli/repository.rs:14`), imported as `tests/cli/remotes.rs:1–4` imports it. The fixture's home holds no `.gitconfig` identity, and the repository has none.
  - `a_save_without_identity_is_identity_required_and_the_same_request_succeeds_after_identity_set`:
    - `ticket save` answers `identity_required`, exit 3, with `repo.identity_set` in its recovery;
    - run `repo identity-set` with prepare and confirm;
    - read a fresh observation with `ticket show`, because the refused save left a new context (fact 11);
    - the same `ticket save`, with the fresh observation and the same request ID, answers `ok`.
  - `a_create_without_initials_is_initials_required_and_the_same_request_succeeds_with_initials`: the identity's name yields no initials. Task 0 confirms that the record is freed.
- [ ] **Step 7: Implement the three verbs,** reading fields as the design's Input table gives them.
  - **`ticket_create`:**
    - `item_id` → `ItemId`;
    - `type` → `ticket_type`;
    - `body` defaults to `""`;
    - `deps` via `tristate_list`: `Missing | Null | Value([])` → empty, `Value(v)` → `v`;
    - `parent` via `tristate`: `Missing | Null` → `None`;
    - `initials` via `optional_string`.
  - **`ticket_save`:**
    - `TicketFields`;
    - `deps` via `tristate_list`: `Missing` → `RelationshipWrite::Unchanged`, `Null | Value([])` → `Clear`, `Value(v)` → `Set(v)`;
    - `parent` via `tristate`: `Missing` → `Unchanged`, `Null` → `Clear`, `Value(s)` → `Set(s)`;
    - the draft comes from `fill::ticket_draft`, after at most one `fill::read_item` with `ItemDtoKind::Ticket`.
  - **`ticket_slug_assign`:** `observation`, and optional `initials`.

  Add the catalog entries, dispatch arms, schemas and examples. The create and save schemas give `project` and `team` on save, and `parent` on both, the type `["string", "null"]`. `deps` gets `["array", "null"]` with `"items": {"type": "string"}`.
- [ ] **Step 8: Add human mode for each ticket verb.** Each gets a success, a refusal and a no-op.
  - Text goldens: `ticket_create_text`, `ticket_save_text` and `ticket_slug_assign_text`.
  - JSON goldens:
    - `ticket_create_ok` and `ticket_create_relationship_cycle`;
    - `ticket_save_ok`, `ticket_save_item_not_found` and `ticket_save_ticket_closed`;
    - `ticket_slug_assign_ok` and `ticket_slug_assign_slug_already_assigned`.
- [ ] **Step 9: Run.**

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli ticket_authoring
  devenv shell -- cargo test --all-features --locked --test cli input_schemas
  devenv shell -- cargo test --all-features --locked --test cli authoring_exits
  ```

  Move the codes these tests reach from `REASONS` to rows, and extend `c3_confirmed_flags_match_prepare` to the ticket verbs.
- [ ] **Step 10: Regenerate the E1 goldens, run Clippy, and commit** `feat(cli): ticket create, save and slug-assign`. Add a ticket comment.

## Task 6: Repairs

**Files:**
- Create: `tests/cli/repair.rs`, schemas and examples `document_repair`, `ticket_repair`
- Modify: `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/support/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1)

- [ ] **Step 1: Write the failing binary tests** in `tests/cli/repair.rs`. Build the nonconforming fixtures with `support::repair::Shelf` (`tests/support/repair.rs:89`). Import `support::pty` only under `#[cfg(unix)]`.
  - `prepare_then_confirm_adopts_the_supplied_id`:
    - `path observe --path` gives the token;
    - `document repair --prepare` gives a preview that validates against `repair_preview.schema.json`, with `adoption` set (golden `document_repair_preview`);
    - `--confirm <id> --request-id <id>` answers `ok`, with one commit in the new context and `adopted` true;
    - primary's file is byte-identical.
  - `ticket_repair_prepare_and_confirm`, with golden `ticket_repair_preview`.
  - `a_stale_token_at_confirm_is_external_change`.
  - `an_invalid_corrected_source_is_refused_at_prepare` and `a_path_with_no_nonconforming_entry_is_refused`, each with the code Task 0 recorded.
  - `an_id_another_item_holds_is_identity_ambiguous_and_makes_no_context`.
  - `a_closed_ticket_is_repaired_keeping_its_closure`.
  - `a_repair_interrupted_after_its_write_is_completed_by_the_same_request` (R-11):
    - a `FailOnce` service at `AfterOwnedWriteBeforeLifecyclePersistence` takes a preview and calls `execute_confirmed` (`tests/support/mutation.rs:505`), getting the partial `internal_error` of fact 19;
    - the CLI repeats `--confirm <same> --request-id <same>`;
    - the result is one commit and the same ID;
    - a further replay writes nothing.
  - `a_confirmation_presented_after_cache_loss_is_confirmation_not_found`.
  - `#[cfg(unix)] interactive_consent_reads_the_source_from_stdin_and_the_answer_from_the_terminal`:
    - the source is piped through `--input -` and the answer is typed on the pseudo-terminal;
    - `n` answers `cancelled`, exit 130, with nothing written;
    - `y` answers `ok`.

    Copy the shape from `tests/cli/prompts.rs`.
  - `without_a_terminal_a_repair_needs_a_confirmation`: `--request-id` without `--confirm` answers `confirmation_required`, exit 3.
  - `each_repair_replays_a_finished_request`.
  - Human mode.
  - Goldens:
    - `document_repair_preview`, `ticket_repair_preview`;
    - `document_repair_ok`, `ticket_repair_ok`;
    - `document_repair_identity_ambiguous`, `ticket_repair_external_change`;
    - `document_repair_text`, `ticket_repair_text`.
- [ ] **Step 2: Run the tests and see them fail:** `devenv shell -- cargo test --all-features --locked --test cli repair`.
- [ ] **Step 3: Implement `document_repair` and `ticket_repair` in `authoring.rs`.**
  - Each reads `path`, `observation` and `source`, and builds `Mutation::DocumentRepair` or `Mutation::TicketRepair` with `Plan { identity_prompt: false, retry: true }`.
  - The catalog entries are `Kind::Mutation { confirmed: true, input: InputRule::Required }` with `MUTATION_INPUT`.
  - Add the dispatch arms, schemas and examples.
- [ ] **Step 4: Run the tests and see them pass.**
  - Move the codes the tests reach from `REASONS` to rows, including exit 130 from the consent case.
  - Extend `c3_confirmed_flags_match_prepare`: a preview for both repairs, and `not_confirmable` for the other nine mutations.
- [ ] **Step 5: Regenerate the E1 goldens.** Confirm that `help_root.json` no longer shows any C3 verb as planned. Then run Clippy, commit `feat(cli): document and ticket repair`, and add a ticket comment.

## Task 7: Part B Evidence And Checkpoint

**Files:**
- Modify: `tests/cli/authoring_exits.rs`

- [ ] **Step 1: Complete the exit table.**
  - `REASONS` keeps only codes no fixture can reach, each with its reason. Exit 4 stays there (decision 6).
  - As in C2, a `CLI_ONLY` list adds the CLI-built codes C3 verbs can give: `usage_error`, `invalid_input`, `invalid_id`, `invalid_path`, `cancelled` and `internal_error`. The rows and reasons must equal the audit block plus `CLI_ONLY`.
  - Rename the no-op test to `c3_noop_inventory_accounts_for_all_eleven_mutations`, and extend it to cover Part B.
- [ ] **Step 2: Add `every_input_taking_c3_verb_refuses_malformed_input`.** For each of the nine input-taking verbs, an unknown field and a wrong type each answer `invalid_input`, exit 2. Stderr names the field and never its value.
- [ ] **Step 3: Add `every_c3_verb_runs_in_human_mode_with_split_streams`.** It reads the C3 verbs from `--help --json` (`delivered_by == "C3"`), and asserts by set equality that they are the design's thirteen. It names each verb's human test.
- [ ] **Step 4: Add the privacy test** `no_c3_verb_echoes_a_sentinel` (R-17).
  - Every input-taking C3 verb runs in JSON and human mode, including a refusal after the fill read: `ticket save --id <a document's ID>` with `title` omitted.
  - The `body` and `source` sentinel `MANYHANDS-SENTINEL-7f3a` appears in no stdout or stderr, and `assert_request_stores_exclude` (fact 20) finds it in no request store.
  - The `title`, `path`, `initials` and `deps` sentinels appear in no diagnostic: no CLI-built refusal, no human stderr and no JSON `message`. Data fields that echo input by design are exempt (fact 37): `path`, `slug`, `unresolved` and `rejected`. So are the reads that return those values.
  - The inventory counts **successful executions**, as C2's privacy test does.
- [ ] **Step 5: Add `every_c3_example_input_runs` (R-13).**
  - Each `tests/fixtures/cli_input_v1/<C3 stem>.json` runs through the binary against a fresh fixture, prepared as its row in the test's own outcome table says.
  - Each answer has its expected code.
  - Observation fields are replaced at run time by the token the fixture reads, and the example's other fields are unchanged.
  - Task 8's RFC amendment copies this table.
- [ ] **Step 6: Run the checkpoint gate,** then `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`. Get the independent Part B review, and fix or rule every finding.
- [ ] **Step 7: Commit** `test(cli): C3 exit table, input, human, privacy and example evidence`. Add the Part B checkpoint ticket comment, and push the ticket branch.

## Task 8: Document Amendments

**Files:**
- Modify: `docs/RFC/cli-contract.md`, `docs/RFC/ticket-relationships-and-short-codes.md`, `docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Amend the CLI contract RFC.** Add "Amended 2026-10-24 by C3" at each item of the design's Document Amendments list for this RFC, and at no other place.
- [ ] **Step 2: Amend the relationships RFC.**
  - `ticket check`'s flags and semantics;
  - `null` for `deps` and `parent` on create and save.
- [ ] **Step 3: Amend Wave 03.** Add "Amended 2026-10-24 by C3" to the C3 section, recording decisions 1–7, the delivery and the known limits.
- [ ] **Step 4: Check the API audit.** Every C3 row matches the code as built.
- [ ] **Step 5: Check for stale text.** Grep the four files for "Planned for C3", "C3–C5 verbs remain cataloged" and "recovery_required" in the cache-loss paragraph. Commit `docs: amend the RFCs, Wave and audit for C3`, and add a ticket comment.

## Task 9: Full Verification, Review, Dogfooding And Handoff

- [ ] **Step 1: Run the full gate.** The full suite runs detached, because it outlasts a tool timeout. Watch its log with a two-hour budget and record every target's counts. A failure outside C3's files is recorded with its evidence; do not retry it until it passes.

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  nohup setsid devenv shell -- cargo test --all-features --locked --no-fail-fast > /tmp/claude-1000/c3-full-suite.log 2>&1 &
  devenv shell -- cargo run --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  ```

- [ ] **Step 2: Run `git fetch origin` and `git merge-tree --write-tree HEAD origin/main`,** then the whole-branch reviews: spec, and quality of the actual code. Fix or rule every finding.
- [ ] **Step 3: Dogfooding** (R-16, Cycle decision 3). Run the design's Dogfooding steps 1–7 exactly, with its stop and resume rules.
  - **Before asking:**
    - the build (step 1);
    - the read-only preflight (step 2);
    - the go-ahead (step 3), presenting the preflight's results and naming the six new remote branches with the commits they publish, and the front-matter re-emission.
  - **Without the go-ahead:** stop. Record the step as owed and continue to Step 5. The Cycle is not closed or merged while the step is owed.
  - **With it:**
    - write each request ID and input to the ledger before sending it, and each mutation's envelope with the compared values after. Never record `body` or `source`;
    - add a ticket comment with the outcome.

    A stop rule stops the step. A resume follows the design's Resume rule.
- [ ] **Step 4: Switch** (design Dogfooding step 8), only after every save, value check and push succeeded.
  - Amend AGENTS.md's Dogfooding Workflow, naming the binary, the commit-first rule, closure until C5, comments until C4, and the `working-on-a-ticket` exception.
  - Rewrite `.agents/skills/file-ticket/SKILL.md`.
  - Correct the C3 ticket's Added Scope and tick its planning boxes.

  Commit `docs: switch ticket dogfooding to the CLI`, then get an independent review of that commit alone, against the design's step 8. Fix or rule every finding.
- [ ] **Step 5: Mark review-ready.**
  - Run `git fetch origin` and `git merge-tree --write-tree HEAD origin/main` again.
  - Update the Cycle document, design and ledger with the outcome.
  - Add the review-ready ticket comment, and push the ticket branch.
- [ ] **Step 6: Close before merge.** After the owner approves the review, close the ticket in the final push before merge, as AGENTS.md requires. This waits until the dogfooding evidence is met, or the owner has amended it. Merging, pull requests and cleanup need their own authorization.

## Plan Self-Review

| Cycle acceptance row, contract item or ruling | Task |
| --- | --- |
| Offline checkpoints and no-ops | 3, 5; inventory 4, 7 |
| Stale writes | 3, 5, 6 |
| Invalid and malformed repair | 6 |
| Adoption retry | 6 (R-11) |
| Move collision | 3 |
| Index loss and corruption | 2 (R-12) |
| Commit success, refresh failure | 3 (R-11) |
| Schemas and documented examples | 2, 3, 5, 6 (schemas); 7 (R-13) |
| Dogfooding switch | 9 (R-16) |
| Every exit class; unreachable codes listed (decision 6) | 4, 6, 7 (R-10) |
| Replay after lost output, every mutation | 2, 3, 5, 6 |
| Cache loss | 3, 5, 6 |
| Human mode and no-ops for every verb | 1–7 (R-14) |
| Input failures for every input-taking verb; closure and `slug` keys | 5, 7 |
| Identity recovery | 5 (R-9, R-18) |
| Relationships: both cycle kinds on create and save, `ticket check` agreeing | 1, 5 |
| Never fetch | 2 |
| Progress | 3 |
| Prompts never read stdin | 6 |
| Fill refusals (decisions 5 and 7), fill before replay, partial-then-edited | 3, 5 |
| Mutating-Cycle evidence row | 3, 5, 6 (stale writes, retry); 6 (consent, cancellation); 3 (progress); 7 (redaction) |
| Help, version and C4/C5 unavailable | 1–6 (E1); pre-existing `help.rs` tests unchanged |
| No display dependency, thread or process | 4, 7, 9 (`check-cli-no-gpui.sh` and the no-feature `--bin` check) |
| Document amendments | 8, 9 |
| R-1 catalog | 1, 2, 3, 5, 6 |
| R-2 usage privacy | 1 |
| R-3 list readers | 5 |
| R-4, R-5 save patch and optionality | 3, 5 |
| R-6, R-7 reads | 1 |
| R-8 index verbs | 2 |
| R-9 identity pass-through | 5 |
| R-10 C3 exit table | 4, 7 |
| R-11 library-level actors | 3, 6 |
| R-12 corruption evidence | 2 |
| R-13 documented examples | 7 |
| R-14 no-op inventory | 0 (capability list), 4, 7 |
| R-15 layout | all |
| R-16 dogfooding | 9 |
| R-17 privacy | 7 |
| R-18 identity machine | 5 |

**Placeholder scan.** No "TBD", "TODO" or "similar to Task N" remains. Values that Task 0 reads from the source, rather than this plan stating them, are:

- the codes of an invalid corrected source and a path with no nonconforming entry;
- the cache-loss codes;
- whether `initials_required` frees its record.
