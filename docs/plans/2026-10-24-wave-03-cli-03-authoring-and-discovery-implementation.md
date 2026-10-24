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

- `authoring.rs` is the per-verb glue for the eleven mutations, built like `admin.rs`;
- `fill.rs` is the save patch: a pure merge function plus one `show_item` read;
- `human_authoring.rs` is the human text of the two new reads.

Every change of state is one F2 `prepare` or `execute`, and every read is one F1 read. Nothing under `src/repository*` changes.

**Tech Stack:**

- Rust 2024 and clap's builder API (C1);
- `serde_json`;
- F2's mutation boundary, result model, schemas and test support;
- C1's and C2's CLI harness, goldens and pseudo-terminal helper.

**Spec:** the [Design](2026-10-24-wave-03-cli-03-authoring-and-discovery-design.md), under the [Cycle](../Cycles/wave-03-cli-03-authoring-and-discovery.md). Read both before any task. The Cycle governs the design, and the design governs this plan.

## Global Constraints

- **Devenv.** Run every Rust command through Devenv with `--locked`: `devenv shell -- cargo <command> --locked`.
  - No new crate.
  - No change to `Cargo.toml` or `Cargo.lock`.
  - No new `[[test]]` target. New tests are modules of `tests/cli.rs`.
- **Branch.** Work only in the ticket worktree `.manyhands/worktrees/01M4CC0VMV81DFRGJ4Z76489XM`, on `manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`.
  - If `origin/main` moves, `git merge origin/main`. Never rebase.
  - Before each checkpoint, run `git merge-tree --write-tree HEAD origin/main`.
- **Library.** No change under `src/repository.rs`, `src/repository/`, `src/lib.rs`, `src/results.rs` or `src/runtime.rs`. No new result code, recovery action or output schema.
  - A library gap stops the task. Report it, and the controller raises a separate ticket under track rule 5.
  - The known limits of Cycle decision 1 (`01M4PB82`, `01M4PB80`, `01M4PB7Y`) are not worked around.
- **Bin only.** CLI code lives in `src/bin/manyhands-cli.rs` and `src/bin/manyhands-cli/`. New modules follow C1's pattern: `#[path = "manyhands-cli/<name>.rs"] mod <name>;`, with `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`.
- **No dead code.** `cargo clippy -- -D warnings` fails on an unused function in the bin. Add each helper in the task that first calls it.
- **Runtime.**
  - The CLI spawns no thread and no child process.
  - The transport starts before the service opens. Help, version, usage errors and unavailable commands touch neither.
  - All errors before dispatch have null `request_id`. Refusals after opening keep a valid parsed or allocated ID (C2 owner clarification, 2026-10-22).
  - JSON mode writes one envelope to stdout and nothing to stderr.
- **Content and secrets.**
  - No type that holds input derives or implements `Debug`, `Display` or `Serialize`.
  - No input value, body, source, backend text or SQL reaches a diagnostic the CLI writes.
  - Library envelopes pass through unchanged. JSON `message` is always `ResultCode::message()`.
- **Test isolation.**
  - Every CLI child goes through `tests/support/cli.rs` or `tests/support/pty.rs`, and starts without a controlling terminal unless the pseudo-terminal helper gives it one.
  - Tests never touch developer keys, global Git configuration, the real terminal or real application data.
  - The identity-required cases need a machine whose system Git configuration sets no identity (design R-18).
- **Pre-existing tests.** Only named edit E1 may change a pre-existing test or golden. Any other semantic edit, including a new lint on an old test, stops for owner review.
- **Test runs (owner, 2026-10-17, 2026-10-19 and 2026-10-24).**
  - Each task runs only the targets and filters its steps name.
  - Cargo takes **one** positional test filter per command. Split commands rather than list two filters.
  - Clippy in a task step is `cargo clippy --all-targets --all-features --locked -- -D warnings`, so the bin's `cfg(test)` code is linted too.
  - The Part checkpoints (Tasks 4 and 7) run the checkpoint gate. The full suite runs once, in Task 9.
- **Checkpoint gate.**

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo check --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract
  ```

  The test line includes `--all-features`, because C2's gate without it missed `preserve_order` failures. After the gate:
  - run `git merge-tree --write-tree HEAD origin/main`;
  - get an independent review of the Part's commit range against the Cycle, the design and this plan;
  - fix or rule every finding before the next Part;
  - add a ticket comment.
- **Goldens.**
  - Update with `MANYHANDS_UPDATE_GOLDEN=1`, never under `CI`.
  - Read every regenerated golden's diff before committing. For E1, the diff may change only the newly available verb's entries.
  - New cases are registered in `CLI_CASES` or `CLI_TEXT_CASES` (`tests/support/cli.rs:217`, `:304`).
- **No subagents.** Implementers never dispatch subagents.
- **Commits.** Commit at the end of every task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- **Ticket comments.** Add a comment at Task 0, at each task's completion, at each Part checkpoint and at review-ready.
  - Write each under `.manyhands/comments/01M4CC0VMV81DFRGJ4Z76489XM/<ULID>.md` in the canonical schema. `comment add` is C4's.
  - Take the ULID from the built CLI's `id new`, and `created_at` from the clock in UTC.
- **Authorization.**
  - Pushing the ticket branch is the standing habit.
  - Never push another branch, open a pull request, merge, close the ticket or clean up without separate owner authorization. The exception is Task 9's dogfooding pushes, which its go-ahead names.
  - Native CI runs after merge, on main, only with the owner's approval of that run.

## Review Focus

These are inputs the spec implies but no table in it lists. Each has its test in the owning task.

1. **A body with CRLF line endings, a lone CR, non-BMP characters and a final line without a newline.** `document create` and a body-only `document save` write it byte for byte, and a save that omits `body` keeps it byte for byte. Task 3, Step 6.
2. **A path that escapes the repository**: `../x.md`, an absolute path, or `docs/../../x.md` given to `path observe --path`, `document create` `path` or `document move` `destination_path`. The answer is the library's refusal (`invalid_path` or `invalid_input`), nothing is read or written outside the repository, and the diagnostic does not echo the path. Task 1, Step 5 and Task 3, Step 8.
3. **`document move` to the path the document already has.** The library's already-applied rule gives a no-op, not a collision, and nothing is committed. Task 3, Step 8.
4. **`deps` that names the ticket itself, or repeats an entry.** A self-reference is `relationship_cycle` naming the ticket alone; a repeat is removed silently and the rest sorted. Both match `check_ticket_relationships`. Task 5, Step 5.
5. **A title or status holding terminal escape sequences or control characters.** Human output escapes them, so a title cannot repaint the terminal or forge a line. Task 3, Step 9 and Task 5, Step 8.

---

## File Map And Task Order

| Task | Part | Creates | Modifies |
| --- | --- | --- | --- |
| 0 | — | `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md` | `docs/research/wave-03-api-audit.md` |
| 1 | A | `human_authoring.rs`, `human_authoring_tests.rs`, `tests/cli/authoring_reads.rs` | `src/bin/manyhands-cli.rs`, `catalog.rs`, `parse.rs`, `parse_tests.rs` (additions only), `dispatch.rs`, `tests/cli.rs`, help goldens (E1) |
| 2 | A | `authoring.rs`, `authoring_tests.rs`, `tests/cli/index.rs`, `tests/cli/folders.rs`, schema and example `folder_create` | `src/bin/manyhands-cli.rs`, `admin.rs` (two helpers made `pub(crate)`), `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, help goldens (E1) |
| 3 | A | `fill.rs`, `fill_tests.rs`, `tests/cli/documents.rs`, schemas and examples `document_create`, `document_save`, `document_move` | `src/bin/manyhands-cli.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, help goldens (E1) |
| 4 | A | `tests/cli/authoring_exits.rs` | `docs/research/wave-03-api-audit.md` (only if Task 0's union was wrong, with a ledger note), `tests/cli.rs` |
| 5 | B | `tests/cli/ticket_authoring.rs`, schemas and examples `ticket_create`, `ticket_save`, `ticket_slug_assign` | `input.rs`, `input_tests.rs`, `fill.rs`, `fill_tests.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1) |
| 6 | B | `tests/cli/repair.rs`, schemas and examples `document_repair`, `ticket_repair` | `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1) |
| 7 | B | — | `tests/cli/authoring_exits.rs` |
| 8 | — | — | `docs/RFC/cli-contract.md`, `docs/RFC/ticket-relationships-and-short-codes.md`, `docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md` |
| 9 | — | — | Cycle document, design, execution ledger, ticket and comments, `AGENTS.md`, `.agents/skills/file-ticket/SKILL.md`; the six target tickets' branches (dogfooding) |

Tasks run in order. Tasks 1–6 each flip verbs from `planned(…, "C3")` to `Available("C3")` and edit the same files, so none can run in parallel. A verb becomes available only in the task that dispatches it, gives it its schema and example, and its goldens.

Cross-cutting tests grow with each verb task, from Task 4 on:

- `tests/cli/authoring_exits.rs`: the exit table;
- the human-mode case of each verb;
- the no-op inventory.

---

## Task 0: Baseline, Audit Refresh And Ledger

**Files:**
- Create: `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-execution.md`
- Modify: `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: Confirm the base.**
  - `git fetch origin`, then `git merge origin/main` and `git merge-tree --write-tree HEAD origin/main`.
  - Read `git show origin/main:AGENTS.md`, not the worktree's copy.
  - Check that none of the decision-1 tickets (`01M4PB82G07RZSQMNJZN672S3W`, `01M4PB80T7SVB23TQ6466RDHCB`, `01M4PB7YZMBFAFFM94AZY08HWS`) has merged since planning, with `git log --oneline origin/main --grep <ULID>` for each. If one has, record it: the known limits and possibly R-12 change, and the controller rules before Task 1.
  - Record HEAD and `origin/main`.
- [ ] **Step 2: Re-check every Baseline Fact in the design** against the source. These came from research and were not re-read at drafting:
  - fact 4: `needs_confirmation`'s C3 rows;
  - fact 10: an index binding's data and effects;
  - fact 11: `identity_required` frees the request, which the test around `tests/mutation_replay/execute.rs:1100` shows.

  Also confirm:
  - `manyhands::repository::{PathObservationDto, RelationshipCheckDto, ProposedRelationships, DocumentDraft, TicketDraft, RelationshipWrite}` and the eleven input types are reachable from the bin;
  - the fault point F2's `a_repair_stopped_after_its_write_is_completed_by_its_retry_with_the_same_id` (`tests/mutation_bridges/repair.rs`) injects. Task 6 uses it;
  - the database file's name under the data directory, from `src/repository.rs`. Task 2 uses it;
  - how `parse::validate` (`parse.rs:191`) enforces an option's `required` flag, and whether an option that takes a value can appear twice.

  Record each fact as confirmed or changed. A changed fact that alters a task stops here for a controller ruling.
- [ ] **Step 3: Prove E1 is the only named edit.**

  ```sh
  grep -rln -E 'path observe|ticket check|index (refresh|rebuild)|folder create|document (create|save|move|repair)|ticket (create|save|repair|slug-assign)|"C3"' src/bin/manyhands-cli tests/cli tests/cli_remote tests/cli.rs tests/support/cli.rs tests/fixtures/cli_v1
  ```

  Expected hits:
  - `catalog.rs`;
  - `catalog_tests.rs`, whose `DESIGN` table is unchanged;
  - the three help goldens of E1.

  Also list tests that count available commands or iterate `Availability::Available` without a `delivered_by` filter:

  ```sh
  grep -rn -E 'Availability::Available|available.*len\(\)|"available"\] == true' src/bin/manyhands-cli tests/cli
  ```

  Every hit not filtered to C1 or C2, and not covered by E1, stops for an owner ruling.
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
  - For each of the thirteen C3 verbs, update its existing row in place:
    - the F1 or F2 call and its signature at this baseline;
    - whether it needs confirmation;
    - its input type;
    - **the complete set of result codes it can return.** Read these from its binding (`src/repository/mutation/bind/*.rs`), `src/repository/mutation/outcome.rs` and the boundary (`execute.rs`, `confirm.rs`), or from the read.
  - Do not add a second row for a verb. Correct the stale "absent" claims.
  - Under a heading `#### C3 exit-coverage union`, put the union of the C3 code sets in one fenced `text` block, space-separated. Task 4's test pins it.
  - Under `#### C3 no-op capability`, list which of the eleven mutations can be a no-op for a fresh request, with the binding rule that makes it one, and the reason for each that cannot (R-14).
- [ ] **Step 6: Create the execution ledger.** Use front matter like the design's, with a new document ULID from `id new`. Its sections:
  - baseline;
  - confirmed facts;
  - audit refresh;
  - Cycle decisions 1–4, rulings R-1 to R-18 and named edit E1;
  - the planning review record;
  - an empty task log.
- [ ] **Step 7: Commit** `docs: record C3 baseline and refresh the API audit`, add a ticket comment, and push the ticket branch.

## Task 1: Usage Privacy And The Two Reads

**Files:**
- Create: `src/bin/manyhands-cli/human_authoring.rs`, `human_authoring_tests.rs`, `tests/cli/authoring_reads.rs`
- Modify: `src/bin/manyhands-cli.rs`, `catalog.rs`, `parse.rs`, `parse_tests.rs` (new tests only), `dispatch.rs`, `tests/cli.rs`, `tests/fixtures/cli_v1/help_root.json`, `help_root_text.txt`, `help_resource_ticket.json` (E1)

**Interfaces:**
- Consumes:
  - `RepositoryService::observe_path(&self, &ResolvedRepository, Option<&Path>, &Path) -> Result<PathObservationDto, ReadError>`;
  - `RepositoryService::check_ticket_relationships(&self, &ResolvedRepository, &ItemId, &ProposedRelationships) -> Result<RelationshipCheckDto, ReadError>`;
  - `dispatch::resolve`, `dispatch::item_id`, `dispatch::read_output`.
- Produces:
  - `human_authoring::path_observation(&PathObservationDto) -> render::Human`;
  - `human_authoring::relationship_check(&RelationshipCheckDto) -> render::Human`;
  - the R-2 rule.

- [ ] **Step 1: Write the failing R-2 test** in `parse_tests.rs`. It mirrors `tests/cli/mutation.rs`'s `a_mode_refusal_keeps_request_id_null_and_c2_usage_never_echoes_values` for a C3 verb. Do not edit the C2 test.

  ```rust
  #[test]
  fn a_c3_verbs_usage_error_never_echoes_a_value() {
      let invocation = invoke(&["ticket", "check", "--repo", "r", "--id", "SECRET-VALUE", "--bogus", "SECRET-VALUE"]);
      let Action::Usage { detail, .. } = invocation.action else { panic!("usage expected") };
      assert!(!detail.contains("SECRET-VALUE"), "{detail}");
  }
  ```

  `invoke` is `parse_tests.rs:4`, and `Action::Usage { command, detail }` is `parse.rs:24`.
- [ ] **Step 2: Make the two reads available in the catalog, and see the test fail.** A planned verb accepts any trailing arguments, so the test cannot fail until `ticket check` is a real command. In `catalog.rs`, replace the two `planned` lines for `path observe` and `ticket check` with the literal entries below, then run:

  `devenv shell -- cargo test --locked --bin manyhands-cli a_c3_verbs_usage_error_never_echoes_a_value`

  Expected: FAIL, the detail is clap's text and contains the value.
- [ ] **Step 3: Implement.**
  - In `parse.rs:120`, change `spec.delivered_by()=="C2"` to `spec.delivered_by() != "C1"`.
  - The catalog entries, from Step 2:

  ```rust
  const PATH_OBSERVE: &[OptionSpec] = &[
      REPO,
      option("--path", Some("path"), true, "Repository-relative path to observe."),
      option("--context", Some("path"), false, "Context worktree path; the repository root when omitted."),
  ];
  const TICKET_CHECK: &[OptionSpec] = &[
      REPO,
      ID,
      option("--deps", Some("ULID,..."), false, "Proposed dependencies, comma-separated."),
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

  - In `parse::validate`, add C3's rules after the `show` rule:
    - `path observe` requires `--path`;
    - `ticket check` requires `--id` and at least one of `--deps` and `--parent`, with the message "ticket check needs --deps or --parent.".
- [ ] **Step 4: Run the test and see it pass,** with the same command as Step 2.
- [ ] **Step 5: Write the binary tests** in `tests/cli/authoring_reads.rs`, registered in `tests/cli.rs`. Copy `run`, `checked` and `golden` from `tests/cli/content.rs:20–60`. Use `support::items::contract_repository()` for documents and `relationship_repository()` for tickets. Cases:
  - `path_observe_gives_the_absent_token_for_an_empty_document_path`: `exists` is false, the schema is `path_observation.schema.json`, and there is a golden.
  - `path_observe_gives_the_same_token_as_document_show_for_a_file`: the `observation` equals `document show --path`'s.
  - `path_observe_reports_another_items_file_in_a_named_context`: create a document context with `items::create_document_context`, then observe a different document's canonical path with `--context <that worktree>`. The answer is `ok` and `exists` is true. It is not refused and not "outside the context" (Cycle contract item 6).
  - `path_observe_refuses_an_inactive_context_and_an_escaping_path`: a directory that is not an active context gives `invalid_path`, exit 2. `../x.md`, an absolute path and `docs/../../x.md` each give the library's refusal. Stderr never contains the path text (Review Focus 2).
  - `ticket_check_resolves_reports_unresolved_and_rejects_a_cycle`:
    - B already depends on A, so proposing that A depend on B gives `ok`, exit 0, and `data.rejection.code == "relationship_cycle"`, with no file change;
    - an unknown ULID is listed in `unresolved`;
    - the data validates against `relationship_check.schema.json`.
  - `ticket_check_refuses_a_malformed_or_empty_list_element`: `--deps A,,B` and `--deps not-a-ulid` give `invalid_id`, exit 2. Neither `--deps` nor `--parent` gives `usage_error`.
  - `both_reads_write_nothing`: the worktree bytes and `git for-each-ref` output are equal before and after every case above.
  - Human mode for both verbs, with text goldens, split streams and empty JSON stderr.
- [ ] **Step 6: Implement the dispatch arms and human text.**

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

  `human_authoring.rs` writes the lines from the design's Reads section, escaping every value with C1's `render::field` (`render.rs:158`, `pub(super)`, so reachable from a sibling module as `human_admin.rs` reaches it). It adds C1's incomplete-read note when `complete` is false or the index is not current. Its unit tests cover:
  - an escape sequence in a path;
  - a rejection line;
  - an empty `unresolved`.
- [ ] **Step 7: Regenerate the E1 goldens** with `MANYHANDS_UPDATE_GOLDEN=1`, through `tests/cli/help.rs`'s three golden tests. Read each diff: only the `path observe` and `ticket check` entries may change.
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
- Modify: `src/bin/manyhands-cli.rs`, `admin.rs` (make `input_failure` and `name` `pub(crate)`), `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, help goldens (E1)

**Interfaces:**
- Consumes: `admin::{flags, input_object, mutation_root, input_failure}`, `mutate::{run, Plan}`, `Mutation::{IndexRefresh, IndexRebuild, FolderCreate}`.
- Produces: `authoring::{index_refresh, index_rebuild, folder_create}`, each with C2's per-verb signature `fn(&RepositoryService, &mut Session, &'static CommandSpec, &ArgMatches) -> Output`.

- [ ] **Step 1: Write the failing binary tests** in `tests/cli/index.rs`. Build the fixture from `support::items::contract_repository()` and `CliEnv::new(enabled.data_directory.path().to_owned())`. Cases:
  - `index_refresh_and_rebuild_succeed_and_change_nothing_canonical`: the JSON for each has `ok`, `discovery: current` and null data. The worktree and refs are equal before and after.
  - `index_verbs_need_a_request_id_without_a_terminal`: `--json` without `--request-id` gives `usage_error`, exit 2.
  - `index_rebuild_restores_a_deleted_database` (R-12): remove the database file Task 0 named. `document list --json` then answers its index error. `index rebuild` gives `ok`. `document list` and `ticket list` list the fixture's items again. Worktree bytes and refs are equal to before.
  - `index_rebuild_restores_a_database_replaced_by_other_bytes` (R-12): the same, after writing 4096 bytes of `0xA5` over the file.
  - `index_verbs_refuse_input_and_id`: `--input -` and `--id` are usage errors.
  - Human mode for both verbs, with text goldens.
- [ ] **Step 2: Write the failing binary tests** in `tests/cli/folders.rs`:
  - `folder_create_makes_an_empty_folder_and_lists_it`: `{"path": "docs/notes"}` gives `ok`. The directory exists with no file, nothing is committed, and `folder list` lists it.
  - `folder_create_of_an_existing_folder_is_a_noop`: a fresh request gives `noop`.
  - `folder_create_refuses_an_occupied_path_and_a_path_outside_docs`: the library's codes, as recorded at Task 0, with nothing made.
  - Human mode and goldens.
- [ ] **Step 3: Run them and see them fail** (`command_unavailable`):

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli index
  devenv shell -- cargo test --all-features --locked --test cli folders
  ```

- [ ] **Step 4: Implement.** Add `authoring.rs`:

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

  Then add the rest:
  - catalog entries: `Kind::Mutation { confirmed: false, input: InputRule::Refused }` for the index verbs, with `options: &[REPO, REQUEST_ID, PREPARE, CONFIRM]` as `repo remove` has them (`catalog.rs:437`); `input: InputRule::Required` with `MUTATION_INPUT` (`catalog.rs:227`) for `folder create`;
  - the three dispatch arms;
  - the schema:

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

  - the example `{"path": "docs/notes"}`.
- [ ] **Step 5: Run Step 3's commands and see them pass,** then run `devenv shell -- cargo test --all-features --locked --test cli input_schemas`.
- [ ] **Step 6: Regenerate the E1 goldens.** Read the diffs: only the three verbs' entries.
- [ ] **Step 7: Clippy and commit.** Run `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings`. Commit `feat(cli): index refresh, index rebuild and folder create`, and add a ticket comment.

## Task 3: The Save Patch And The Document Verbs

**Files:**
- Create: `src/bin/manyhands-cli/fill.rs`, `fill_tests.rs`, `tests/cli/documents.rs`, schemas and examples `document_create`, `document_save`, `document_move`
- Modify: `src/bin/manyhands-cli.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, help goldens (E1)

**Interfaces:**
- Consumes: `RepositoryService::{resolve_repository, show_item}`, `ItemDto`, `ItemDtoKind`, `DocumentDraft`.
- Produces:

  ```rust
  // fill.rs
  pub struct DocumentFields { pub title: Option<String>, pub body: Option<String> }
  pub enum FillError { NotFound }
  pub fn document_draft(fields: DocumentFields, read: Option<&ItemDto>) -> Result<DocumentDraft, FillError>;
  pub fn needs_read_document(fields: &DocumentFields) -> bool;
  pub fn read_item(service: &RepositoryService, session: &Session, spec: &CommandSpec,
      root: &Path, id: &ItemId, request_id: Option<RequestId>) -> Result<ItemDto, Output>;
  ```

  `read_item` turns a `ReadError` into an envelope via `error.to_envelope(name)`, sets its `request_id` to the parsed or allocated ID, and renders it with `render::failure_output`. Task 5 adds `TicketFields` and `ticket_draft` beside these.

- [ ] **Step 1: Write the failing unit tests** in `fill_tests.rs` (pure, no repository):
  - both fields given: no read needed, and the draft is the input's;
  - `title` omitted: it comes from the read;
  - `body` omitted: it comes from the read, byte for byte;
  - the read is a ticket: `NotFound`;
  - the read lacks `title`: `NotFound`.
- [ ] **Step 2: Run them and see them fail:**

  `devenv shell -- cargo test --locked --bin manyhands-cli fill::`
- [ ] **Step 3: Implement `fill.rs`.**

  ```rust
  pub fn needs_read_document(f: &DocumentFields) -> bool { f.title.is_none() || f.body.is_none() }

  pub fn document_draft(f: DocumentFields, read: Option<&ItemDto>) -> Result<DocumentDraft, FillError> {
      if let Some(item) = read { if item.kind != ItemDtoKind::Document { return Err(FillError::NotFound) } }
      let from = |given: Option<String>, current: fn(&ItemDto) -> Option<&String>| match given {
          Some(value) => Ok(value),
          None => read.and_then(current).cloned().ok_or(FillError::NotFound),
      };
      Ok(DocumentDraft { title: from(f.title, |i| i.title.as_ref())?, body: from(f.body, |i| i.body.as_ref())? })
  }
  ```

  `FillError::NotFound` renders as `ResultCode::ItemNotFound` with the item scope, through `mutate::refusal`.
- [ ] **Step 4: Run Step 2's command and see it pass.**
- [ ] **Step 5: Write the failing binary tests** in `tests/cli/documents.rs`, from `contract_repository()` with no remote. Create:
  - `document_create_commits_one_checkpoint_offline`: `id new` gives the ID. The input is `{"item_id", "path": "docs/new.md", "title", "body"}`. The result is `ok`, exit 0, `checkpoint: committed`, `publication: not_requested`, with one commit on `manyhands/document/<id>`. The schema is `document_mutation.schema.json`, and there is a golden.
  - `document_create_retry_replays_and_a_used_path_is_refused`: the same request again replays with no second commit. A fresh request for an existing path gives the library's code (Task 0). A malformed `item_id` gives `invalid_input` without echoing it.
- [ ] **Step 6: Add save tests to the same file:** one per row of the design's save-patch required-outcomes table that concerns documents:
  - `a_body_only_save_keeps_the_title_and_unknown_keys`;
  - `a_save_that_omits_every_field_is_a_noop`;
  - `a_title_change_by_another_actor_makes_a_body_only_save_external_change`. The second actor is `enabled.service` calling the library directly;
  - `a_retry_after_lost_output_replays`;
  - `a_retry_after_another_actor_changed_an_omitted_field_is_request_mismatch`. It also asserts one commit and that `request show` gives the stored result;
  - `a_document_save_of_a_ticket_id_is_item_not_found`.

  Also add `a_body_round_trips_byte_for_byte` (Review Focus 1). The body is `"line one\r\nline two\rthree 🦀\nno final newline"`. After a create and a title-only save, the file's bytes after the front matter equal the body exactly.
- [ ] **Step 7: Add the interrupted-refresh case** `commit_success_refresh_failure_is_completed_by_the_same_request` (R-11):

  ```rust
  let failing = support::FailOnce::at(FailurePoint::BeforeIndexTransactionCommit)
      .open_service(enabled.data_directory.path());
  let envelope = failing.execute(MutationCall { request_id, confirmation: None, mutation: Mutation::DocumentSave(
      DocumentSaveInput { root: root.clone(), id, draft: DocumentDraft { title, body: body.clone() }, observation: Some(token.clone()) }) },
      &mut SessionCredentials::new(NoCredentials), &mut NoProgress, &CancellationToken::new());
  assert_eq!(envelope.outcome, Outcome::Partial);
  assert_eq!(envelope.effects.checkpoint.as_str(), "committed");
  let tip = branch_tip(&root, id);
  // The CLI sends the same request, with the same complete input, so the digest matches.
  let value = checked(&args("document", "save", &id, &input_with(&token, &title, &body), request_id), …);
  assert_eq!(value["effects"]["discovery"], "current");
  assert_eq!(branch_tip(&root, id), tip); // no second commit
  ```

  `SessionCredentials::new(NoCredentials)`, `NoProgress` and `CancellationToken::new()` are unverified. Copy them from the test at `tests/mutation_replay/documents.rs:585`. A second variant repeats the setup, completes the discovery with a plain `index refresh` instead of the retry, and asserts the item is listed.
- [ ] **Step 8: Add the move tests:**
  - `document_move_checkpoints_both_paths`;
  - `a_move_onto_a_file_is_occupied_path` (exit 2, source and destination unchanged);
  - `a_destination_token_from_the_wrong_context_is_external_change`: the document has a context, and the token is taken from the root;
  - `a_move_to_its_own_path_is_a_noop` (Review Focus 3);
  - `a_move_to_an_escaping_destination_is_refused` (Review Focus 2);
  - `a_stale_source_token_is_external_change`.
- [ ] **Step 9: Add human mode** for each document verb: one success, one refusal and one no-op, with text goldens. Include a title holding `"\u{1b}[2J"` and a newline; the human line shows them escaped (Review Focus 5).
- [ ] **Step 10: Run them and see them fail:** `devenv shell -- cargo test --all-features --locked --test cli documents`.
- [ ] **Step 11: Implement the three verbs in `authoring.rs`**:
  - `document_create` reads `item_id`, `path`, `title` and `body`, with `item_id` parsed into `ItemId` and a failure as `invalid_input` naming the field. It builds `Mutation::DocumentCreate`.
  - `document_save` reads `observation` and the optional `title` and `body` into `DocumentFields`. When `fill::needs_read_document` holds, it calls `fill::read_item` once, then `fill::document_draft`. It builds `Mutation::DocumentSave { observation: Some(observation), .. }`, with the draft computed once, outside the `build` closure.
  - `document_move` reads `observation`, `destination_path` and `destination_observation`, and builds `Mutation::DocumentMove`.

  The `--id` selector is parsed with C1's rule; a malformed one is `invalid_id` through `mutate::refusal`, keeping the request ID. Add the catalog entries (`MUTATION_INPUT` plus `ID` for save and move), the dispatch arms, the three schemas and the examples:

  ```json
  {"$schema": "https://json-schema.org/draft/2020-12/schema", "title": "Document save input", "type": "object",
   "properties": {"observation": {"type": "string"}, "title": {"type": "string"}, "body": {"type": "string"}},
   "required": ["observation"], "additionalProperties": false}
  ```

- [ ] **Step 12: Run them and see them pass:**

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli documents
  devenv shell -- cargo test --all-features --locked --test cli input_schemas
  devenv shell -- cargo test --locked --bin manyhands-cli fill::
  ```

- [ ] **Step 13: Regenerate the E1 goldens,** run Clippy, and commit `feat(cli): document create, save and move with the save patch`. Add a ticket comment.

## Task 4: Part A Evidence And Checkpoint

**Files:**
- Create: `tests/cli/authoring_exits.rs`
- Modify: `tests/cli.rs`

- [ ] **Step 1: Write the C3 exit table.** It follows the shape of `tests/cli/exits.rs` (`ROWS`, `REASONS`, `How`), but in its own module with its own constants:
  - `the_c3_table_and_reasons_account_for_every_code_of_the_audit`. It reads the `#### C3 exit-coverage union` block from `docs/research/wave-03-api-audit.md` with `include_str!`. It asserts that the rows' codes and `REASONS`' codes are disjoint, and that their union equals the block by **set equality**, not by count.
  - `every_c3_exit_class_is_covered`: the exit classes 0, 2, 3, 4 and 5 that C3's Part A verbs can produce each have a row run through the binary. Class 1 has a row or a reason. Class 130 is Task 6's.

  Each row names a test function that exists in `tests/cli/*.rs`, checked as `every_cli_remote_reference_names_an_existing_case` does. Rows added now cover Part A's verbs. A code that only Part B's verbs produce goes in `REASONS` as "Part B, Task 5/6" for now, and Tasks 5–7 move it to a row.
- [ ] **Step 2: Add the Part A no-op inventory,** `c3_noop_inventory_accounts_for_part_a`. For each Part A mutation listed under Task 0's `#### C3 no-op capability`, it names the no-op test. For each that cannot be a no-op, it names the reason. It checks against `--help --json` filtered to `delivered_by == "C3"` and kind mutation.
- [ ] **Step 3: Run the checkpoint gate** (Global Constraints). Record the counts in the ledger.
- [ ] **Step 4: Run `git merge-tree --write-tree HEAD origin/main`,** then the independent Part A review. Fix or rule every finding.
- [ ] **Step 5: Commit** `test(cli): C3 Part A exit table and no-op inventory`. Add the Part A checkpoint ticket comment and push the ticket branch.

## Task 5: List Readers And The Ticket Verbs

**Files:**
- Create: `tests/cli/ticket_authoring.rs`, schemas and examples `ticket_create`, `ticket_save`, `ticket_slug_assign`
- Modify: `input.rs`, `input_tests.rs`, `fill.rs`, `fill_tests.rs`, `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1)

**Interfaces:**
- Produces:

  ```rust
  // input.rs
  pub enum ListChange { Missing, Null, Value(Vec<String>) }
  impl InputObject {
      pub fn optional_string_list(&self, field: &'static str) -> Result<Option<Vec<String>>, InputError>;
      pub fn nullable_string_list(&self, field: &'static str) -> Result<ListChange, InputError>;
  }
  // fill.rs
  pub struct TicketFields { pub title: Option<String>, pub ticket_type: Option<String>, pub status: Option<String>,
      pub body: Option<String>, pub project: Tristate, pub team: Tristate }
  pub fn needs_read_ticket(fields: &TicketFields) -> bool;
  pub fn ticket_draft(fields: TicketFields, read: Option<&ItemDto>) -> Result<TicketDraft, FillError>;
  ```

  `Tristate` is `input::Tristate` (`Missing | Null | Value(String)`).

- [ ] **Step 1: Write the failing unit tests** for the readers:
  - an array of strings;
  - a missing field;
  - `null` for the nullable reader, and as a wrong type for the optional one;
  - a non-array;
  - an array holding a number, a null or an object: each a wrong type naming the field and never the value.
- [ ] **Step 2: Write the failing unit tests** for `ticket_draft`:
  - `project` missing takes the read's value, including absent;
  - `project: null` is `None`;
  - each of `title`, `type`, `status` and `body` omitted comes from the read;
  - a document read is `NotFound`;
  - every field given needs no read.
- [ ] **Step 3: Run them and see them fail:**

  ```sh
  devenv shell -- cargo test --locked --bin manyhands-cli input::
  devenv shell -- cargo test --locked --bin manyhands-cli fill::
  ```

- [ ] **Step 4: Implement the readers and `ticket_draft`,** in the shape of Task 3's `document_draft`. `needs_read_ticket` is true when any of `title`, `ticket_type`, `status` or `body` is `None`, or `project` or `team` is `Tristate::Missing`. Run Step 3's commands and see them pass.
- [ ] **Step 5: Write the failing binary tests** in `tests/cli/ticket_authoring.rs`:
  - `ticket_create_writes_relationships_and_a_short_code_offline`: the input has `deps` (two IDs, one repeated), `parent` and `initials`. The result is `ok` with one checkpoint. The file's `deps` are sorted and unique, `parent` is written, and `slug` is composed from the initials. The data validates against `ticket_mutation.schema.json`.
  - `ticket_create_refuses_a_cycle_before_any_context_exists`: `relationship_cycle`, exit 2. No branch `manyhands/ticket/<id>` and no worktree.
  - `a_self_reference_is_a_cycle_of_one` (Review Focus 4).
  - `an_unknown_target_is_accepted_and_listed_unresolved`.
  - The `ticket save` rows of the design's save-patch table:
    - `project: null` removes `project`;
    - `deps: []` and `deps: null` each remove `deps`;
    - `deps` omitted keeps the key byte-identical;
    - a body-only save keeps every other key, including unknown `wave` and `cycle`.
  - `status_closed_is_ordinary_metadata`: `status: "closed"` saves, and `ticket list --closure open` still lists the ticket.
  - `a_lifecycle_closed_ticket_refuses_save_and_slug_assign`: `ticket_closed`, exit 3. The fixture ticket carries `closed_at` and `closed_by`, as `relationship_repository`'s `CLOSURE` does.
  - `slug_assign_adds_a_code_and_refuses_a_second`: a hand-written ticket without `slug` gets one. Then `slug_already_assigned`, exit 2.
  - `a_stale_token_on_save_and_slug_assign_is_external_change`.
- [ ] **Step 6: Add identity recovery** (R-9, R-18): `a_save_without_identity_is_identity_required_and_the_same_request_succeeds_after_identity_set`.
  - The fixture writes no global `.gitconfig`, unlike `tests/cli/mutation.rs`'s `Fixture`, and has no repository-local identity.
  - The test first calls C2's `require_no_system_identity()` (`tests/cli/repository.rs:14`), imported as `tests/cli/remotes.rs:1–4` imports it (`use super::repository::require_no_system_identity;`). It reads the system configuration with `git2` and panics with its explanatory message if an identity is set. It never changes the machine's configuration.
  - `ticket save` gives `identity_required`, exit 3, and the recovery list holds `repo.identity_set`. `repo identity-set` with prepare and confirm follows. The same `ticket save` with the same request ID then gives `ok`.
  - A sibling test makes `ticket create` with no initials and an identity whose name yields none give `initials_required`.
- [ ] **Step 7: Implement the three verbs,** reading fields as the design's Input table gives them:
  - `ticket_create`: `item_id` to `ItemId`; `type` to `ticket_type`; `body` defaults to `""`; `deps` via `optional_string_list` (missing is an empty `Vec`); `parent` and `initials` via `optional_string`.
  - `ticket_save`: `TicketFields`; `deps` via `nullable_string_list`, mapped `Missing → RelationshipWrite::Unchanged`, `Null | Value([]) → Clear`, `Value(v) → Set(v)`; `parent` via `tristate`, mapped `Missing → Unchanged`, `Null → Clear`, `Value(s) → Set(s)`. The draft comes from `fill::ticket_draft` after at most one `fill::read_item`.
  - `ticket_slug_assign`: `observation`; `initials` optional.

  Add the catalog entries, dispatch arms, schemas and examples. The `ticket_save` schema uses `"type": ["string", "null"]` for `project`, `team` and `parent`, and `"type": ["array", "null"], "items": {"type": "string"}` for `deps`.
- [ ] **Step 8: Add human mode** for each ticket verb: success, refusal and no-op, with text goldens. Include a status holding control characters (Review Focus 5).
- [ ] **Step 9: Run:**

  ```sh
  devenv shell -- cargo test --all-features --locked --test cli ticket_authoring
  devenv shell -- cargo test --all-features --locked --test cli input_schemas
  devenv shell -- cargo test --all-features --locked --test cli authoring_exits
  ```

  Move the Part B codes these tests reach from `REASONS` to rows in `authoring_exits.rs`.
- [ ] **Step 10: Regenerate the E1 goldens,** run Clippy, and commit `feat(cli): ticket create, save and slug-assign`. Add a ticket comment.

## Task 6: Repairs

**Files:**
- Create: `tests/cli/repair.rs`, schemas and examples `document_repair`, `ticket_repair`
- Modify: `authoring.rs`, `catalog.rs`, `dispatch.rs`, `tests/cli.rs`, `tests/cli/authoring_exits.rs`, help goldens (E1)

- [ ] **Step 1: Write the failing binary tests** in `tests/cli/repair.rs`. The fixture is a repository with a committed marker-only document on primary and a nonconforming `ticket.md`. Copy their setup from `tests/mutation_bridges/repair.rs`.
  - `prepare_then_confirm_adopts_the_supplied_id`:
    - `path observe --path` gives the token;
    - `document repair --prepare` gives a `repair_preview.schema.json` preview with `adoption` set, and a golden;
    - `--confirm <id> --request-id <id>` gives `ok` with one commit in the new context and `adopted` true;
    - primary's file is byte-identical.
  - `a_stale_token_at_confirm_is_external_change`: the file is changed between prepare and confirm.
  - `an_invalid_corrected_source_is_refused_at_prepare`, `a_path_with_no_nonconforming_entry_is_refused`, and `an_id_another_item_holds_is_identity_ambiguous_and_makes_no_context`.
  - `a_closed_ticket_is_repaired_keeping_its_closure`.
  - `a_repair_interrupted_after_its_write_is_completed_by_the_same_request` (R-11). The fault point is the one Task 0 recorded. A `FailOnce` service executes with the confirmation the CLI prepared and a fixed request ID. The CLI repeats `--confirm <same> --request-id <same>`. The result is one commit and the same ID. A further replay writes nothing.
  - `#[cfg(unix)] interactive_consent_shows_the_preview_and_no_cancels`: through `tests/support/pty.rs`, the preview is on the terminal. `n` gives `cancelled`, exit 130, with nothing written; `y` gives `ok`. Copy the shape from `tests/cli/prompts.rs`.
  - `without_a_terminal_a_repair_needs_a_confirmation`: `--request-id` without `--confirm` gives `confirmation_required`, exit 3.
  - Human mode, with goldens.
- [ ] **Step 2: Run them and see them fail:** `devenv shell -- cargo test --all-features --locked --test cli repair`.
- [ ] **Step 3: Implement** `document_repair` and `ticket_repair` in `authoring.rs`. Each reads `path`, `observation` and `source`, and builds `Mutation::DocumentRepair` or `Mutation::TicketRepair` with `Plan { identity_prompt: false, retry: true }`. The catalog has `Kind::Mutation { confirmed: true, input: InputRule::Required }` and `MUTATION_INPUT`. Add the dispatch arms, schemas and examples.
- [ ] **Step 4: Run them and see them pass.** Move the codes reached from `REASONS` to rows, including exit 130 from the consent case.
- [ ] **Step 5: Regenerate the E1 goldens.** Confirm that `help_root.json` now shows no C3 verb as planned. Run Clippy, commit `feat(cli): document and ticket repair`, and add a ticket comment.

## Task 7: Part B Evidence And Checkpoint

**Files:**
- Modify: `tests/cli/authoring_exits.rs`

- [ ] **Step 1: Complete the exit table.** `REASONS` now holds only codes that no fixture can reach, each with its reason. Rename the Part A test to `c3_noop_inventory_accounts_for_all_eleven_mutations` and extend it to Part B.
- [ ] **Step 2: Add `every_c3_verb_runs_in_human_mode_with_split_streams`.** It reads the C3 verbs from `--help --json` (`delivered_by == "C3"`, thirteen of them, asserted by set equality with the design's catalog table). It names the human test of each.
- [ ] **Step 3: Add the privacy test** `no_c3_verb_echoes_a_sentinel_body_title_or_source` (R-17).
  - Every input-taking C3 verb runs in JSON and human mode with the sentinel `MANYHANDS-SENTINEL-7f3a` in `title`, `body` and `source`.
  - That includes a refusal after the fill read: `ticket save --id <a document's ID>` with `title` omitted and the sentinel body.
  - It asserts the sentinel is in no stdout or stderr, and in no file under the data directory. The inventory counts **successful executions**, not refusals, as C2's privacy test does.
- [ ] **Step 4: Add `every_c3_example_input_runs` (R-13).** Each `tests/fixtures/cli_input_v1/<C3 stem>.json` runs through the binary against a fresh fixture, prepared as its row says, with its expected code. Observation fields are replaced at run time by the token the fixture reads. The test asserts the example's other fields are unchanged.
- [ ] **Step 5: Run the checkpoint gate,** then `git merge-tree --write-tree HEAD origin/main`, then the independent Part B review. Fix or rule every finding.
- [ ] **Step 6: Commit** `test(cli): C3 exit table, no-op, human, privacy and example evidence`. Add the Part B checkpoint ticket comment and push the ticket branch.

## Task 8: Document Amendments

**Files:**
- Modify: `docs/RFC/cli-contract.md`, `docs/RFC/ticket-relationships-and-short-codes.md`, `docs/Waves/wave-03-dogfooding.md`, `docs/research/wave-03-api-audit.md`

- [ ] **Step 1: CLI contract RFC.** Add "Amended 2026-10-24 by C3" at each place the design's Document Amendments list names:
  - the taxonomy rows;
  - the save patch and its replay limit;
  - `path observe` and `ticket check` grammar;
  - index verbs needing `--request-id`;
  - identity recovery;
  - the C3 rows of the input-schema table;
  - the evidence paragraph: `tests/cli/` files and the example directory.
- [ ] **Step 2: Relationships RFC.** `ticket check`'s flags, and the clearing forms of `deps`.
- [ ] **Step 3: Wave 03.** In the C3 section, add "Amended 2026-10-24 by C3": decisions 1–4, delivery, and the known limits.
- [ ] **Step 4: API audit.** Make sure every C3 row matches the code as built.
- [ ] **Step 5: Check.** Grep the four files for "Planned for C3", "C3 verbs remain cataloged" and similar stale text. Commit `docs: amend the RFCs, Wave and audit for C3`.

## Task 9: Full Verification, Review, Dogfooding And Handoff

- [ ] **Step 1: Run the full gate.** Run the four required checks. The full suite runs detached, because it outlasts a tool timeout:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  nohup setsid devenv shell -- cargo test --all-features --locked --no-fail-fast > /tmp/claude-1000/c3-full-suite.log 2>&1 &
  devenv shell -- cargo run --locked --bin manyhands-cli
  devenv shell -- sh scripts/check-cli-no-gpui.sh
  ```

  Watch the log with a two-hour budget, and record every target's counts. A failure outside C3's files is recorded with its evidence; it is not retried until green.
- [ ] **Step 2: Run `git merge-tree --write-tree HEAD origin/main`,** then the whole-branch reviews: spec, and quality of the actual code. Fix or rule every finding.
- [ ] **Step 3: Dogfooding (R-16, Cycle decision 3).**
  - Ask the owner for the go-ahead, naming:
    - the repository;
    - the default application data directory;
    - the six saves;
    - the six branch pushes;
    - fact 24's state of the G1 worktree, as found by the preflight.
  - Without it, stop: record the step as owed and continue to Step 5.
  - With it, run the design's Dogfooding steps 1–6 exactly, and record every command's envelope in the ledger. A stop rule stops the step.
- [ ] **Step 4: Switch.** Only after every save in Step 3 succeeded:
  - amend AGENTS.md's Dogfooding Workflow;
  - rewrite `.agents/skills/file-ticket/SKILL.md` around `id new`, `ticket create`, `ticket show` and `ticket save`;
  - correct the C3 ticket's Added Scope;
  - tick its planning boxes.

  Commit `docs: switch ticket dogfooding to the CLI`.
- [ ] **Step 5: Mark review-ready.** Update the Cycle document, design and ledger with the outcome, and add the review-ready ticket comment. Push the ticket branch.
- [ ] **Step 6: Close before merge.** After the owner approves the review, close the ticket in the final push before merge, as AGENTS.md requires. Merging, pull requests and cleanup need their own authorization.

## Plan Self-Review

| Cycle acceptance row or ruling | Task |
| --- | --- |
| Offline checkpoints and no-ops | 3 (documents), 5 (tickets), 4 and 7 (inventory) |
| Stale writes | 3, 5, 6 |
| Invalid and malformed repair | 6 |
| Adoption retry | 6 (R-11) |
| Move collision | 3 |
| Index loss and corruption | 2 (R-12) |
| Commit success, refresh failure | 3 (R-11) |
| Schemas and documented examples | 2, 3, 5, 6 (schemas); 7 (R-13) |
| Dogfooding switch | 9 (R-16) |
| Every exit class; unreachable codes listed | 4, 7 (R-10) |
| Human mode and no-ops for every verb | 1–7 (R-14) |
| Identity recovery | 5 (R-9, R-18) |
| Relationships | 1 (check), 5 |
| Mutating-Cycle evidence row | 3, 5, 6 (stale, retry), 6 (consent, cancellation), 7 (redaction) |
| Help, version and C4/C5 unavailable | 1–6 (E1 goldens), pre-existing `help.rs` tests unchanged |
| No display dependency, no thread or process | 4, 7, 9 (`check-cli-no-gpui.sh`, `--bin` check) |
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

Placeholder scan: no "TBD", "TODO" or "similar to Task N" remains. Names this plan could not confirm by reading are marked unverified in their step, with the existing test to copy them from:

- the mutation-replay credential and progress types, `SessionCredentials::new(NoCredentials)` (Task 3, Step 7, from `tests/mutation_replay/documents.rs:585`);
- the repair fault point (Task 0, Step 2);
- the database file's name (Task 0, Step 2);
- the library codes for a used create path and for folder refusals (Task 0's audit refresh).
