---
title: "Wave 03 C3 CLI Authoring And Discovery Maintenance Design"
date: 2026-10-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5QY019WPP3AVCKD13XKMJNA"
---

# Wave 03 C3 CLI Authoring And Discovery Maintenance Design

## Execution Status

**Draft, for product-owner approval.** The planning questions are answered:
Cycle decisions 1–4 before review, 5–6 after the first review round, and 7
after the second. Both rounds' findings are applied (see the Cycle's Review
Record). Nothing is approved or implemented. No build or test was run for
this draft.

## Intent And Approval Boundary

This design turns the [Cycle](../Cycles/wave-03-cli-03-authoring-and-discovery.md)
into verbs, inputs, rules and tests. The Cycle governs this design, and this
design governs the
[implementation plan](2026-10-24-wave-03-cli-03-authoring-and-discovery-implementation.md).

Approval of this design covers:

- the per-verb grammar and input fields;
- the save patch and its refusals (decisions 2, 5 and 7);
- rulings R-1 to R-18;
- named edit E1;
- the document amendments listed below.

It does not authorize implementation, pushing beyond the ticket branch,
merging, closing or native CI.

C3 is CLI only. Every change of state is one F2 `prepare` or `execute`, and
every read is one F1 read. Nothing under `src/repository.rs`,
`src/repository/`, `src/lib.rs`, `src/results.rs` or `src/runtime.rs`
changes. C3 adds no result code, recovery action or output schema.

## Baseline Facts

Read at branch head `9ed4a04` (main `e49f978` merged).

- The second review round (at `521b91b`) confirmed facts 1–8, 12–16, 19,
  20, 22–31 and 33 against the source.
- It corrected facts 10, 11, 17, 18 and 21, as written below.
- Facts 36 and 37 were added after it.
- Fact 17 is still unconfirmed and is marked so.

Task 0 re-checks all 37 facts before any code.

**Library.**

1. `Mutation` (`src/repository/mutation/mod.rs:398`) has a variant for each of
   C3's eleven mutations: `FolderCreate`, `DocumentCreate`, `DocumentSave`,
   `DocumentMove`, `DocumentRepair`, `TicketCreate`, `TicketSave`,
   `TicketRepair`, `TicketSlugAssign`, `IndexRefresh` and `IndexRebuild`. Each
   input type is reachable as `manyhands::repository::<Name>`.
2. The inputs, all under `src/repository/mutation/bind/`:
   - `DocumentCreateInput { root, id: ItemId, path: PathBuf, draft: DocumentDraft }`
     (`document.rs:47`);
   - `DocumentSaveInput { root, id, draft, observation: Option<String> }`
     (`document.rs:59`), with `None` refused as `invalid_input`;
   - `DocumentMoveInput { root, id, observation: Option<String>, destination_path, destination_observation }`
     (`document.rs:70`);
   - `TicketCreateInput { root, id, draft: TicketDraft, deps: Vec<String>, parent: Option<String>, initials: Option<String> }`
     (`ticket.rs:44`);
   - `TicketSaveInput { root, id, draft, deps: RelationshipWrite<Vec<String>>, parent: RelationshipWrite<String>, observation: String }`
     (`ticket.rs:58`);
   - `TicketSlugAssignInput { root, id, observation: String, initials: Option<String> }`
     (`ticket.rs:948`);
   - `FolderCreateInput { root, path: String }` (`folder.rs:58`);
   - `DocumentRepairInput` and `TicketRepairInput`, each
     `{ root, path: PathBuf, observation: String, source: String }`
     (`repair.rs:100`, `:114`);
   - `IndexRefreshInput { root }` and `IndexRebuildInput { root }`
     (`index.rs:36`, `:43`).
3. `DocumentDraft { title, body }` and
   `TicketDraft { title, ticket_type, status, project: Option<String>, team: Option<String>, body }`
   (`src/repository.rs:579`, `:585`) derive only `Clone`. `RelationshipWrite`
   is `Unchanged | Clear | Set(T)`, with default `Unchanged` (`:613`). The
   library takes a complete draft, and F2's design leaves the save patch to
   the CLI.
4. Of C3's mutations, only `DocumentRepair` and `TicketRepair` need
   confirmation (`needs_confirmation`, `src/repository/mutation/command.rs`
   around line 436).
5. The save bindings read the item through `save::read_item`
   (`src/repository/mutation/bind/save.rs:89`). It answers `item_not_found`
   in each of these cases:
   - the item is of the other kind;
   - it has no source or observation;
   - its source does not parse;
   - its parsed ID differs.
6. `observe_path(&self, repo, context: Option<&Path>, path: &Path)`
   (`src/repository/read/items.rs:1963`).
   - The context must be the repository root or an item context the index
     holds as active. Anything else is `invalid_path`.
   - The path must be a canonical item path or a listed nonconforming entry.
     Another item's path in that context is allowed.
   - In a document's context it gives exactly the token that `prepare_move`
     compares (`document.rs:556–566`). On primary it gives the token that
     repair compares.
7. `check_ticket_relationships` (`items.rs:2033`) takes
   `ProposedRelationships { deps: Vec<ItemId>, parent: Option<ItemId> }`
   (`items.rs:113`). The check gives the ticket exactly the proposed `deps`
   and `parent` (`items.rs:368–415`), so an empty `deps` proposes no
   dependencies. It answers from the index.
8. `show_item` (`items.rs:1784`) returns `ItemDto`
   (`src/repository/read/dto.rs:426`):
   - a conforming item's `title` and `body` are `Some` (`items.rs:1131–1133`);
     an empty body is `Some("")`;
   - `project` and `team` are `None` only when the key is absent. A
     non-string value makes the ticket nonconforming
     (`optional_item_string`, `src/canonical.rs:1106`), so it reads as
     `item_not_found`.
9. **Every write re-emits the whole front matter.**
   - `serialize_item` (`src/canonical.rs:911`) writes unknown keys first, then
     the defined keys, in its own quoting and block style. The order is
     pinned by `tests/canonical_foundation.rs:449–461`.
   - The body is written byte for byte: `split_front_matter` (`:1026`)
     returns everything after the closing `---`.
   - A document save writes its file whenever the source is the destination
     (`will_write_destination`, `src/repository.rs:2265`). So saving a
     hand-written document commits a normalization, even with no value
     changed.
   - A ticket save compares parsed items (`src/repository.rs:2646–2658`). A
     save that changes no value is a no-op.
10. An index binding answers with no `data` and `effects.discovery` `current`
    on success. `IndexRebuild` restores an unavailable index before its
    binding runs (`src/repository/mutation/execute.rs:96–141`,
    `src/repository/mutation/command.rs:188–190`).
11. A save refused with `identity_required` deletes its request record. It
    keeps the new editing context's worktree, with no commit
    (`tests/mutation_replay/execute.rs:1100–1186`).
    - The observation token covers the context's branch
      (`src/repository/read/items.rs:1200`). Once the index holds the new
      context, the first observation is stale.
    - The library's own test therefore reads a fresh token before its
      retry (`tests/mutation_replay/execute.rs:1172`). The retry may keep the
      request ID, because the record was deleted.
12. The failure hook is per service and in process:
    `RepositoryService::open_at_with_failure_point_for_testing`
    (`src/repository.rs:4490`), wrapped by `FailOnce`
    (`tests/support/mod.rs:277`). A CLI test drives one on the CLI's data
    directory with `support::mutation::execute_with` (`:487`) or
    `execute_confirmed` (`:505`), scoped in a block as
    `tests/cli/keys.rs:744–753` does.
13. Result-code classes (`src/results.rs`):
    - Input (exit 2): `relationship_cycle`, `invalid_relationship`,
      `occupied_path`, `slug_already_assigned`, `not_repairable`,
      `item_not_found`, `path_not_found`, `invalid_path`;
    - Blocked (exit 3): `external_change`, `identity_required`,
      `initials_required`, `invalid_slug_configuration`,
      `identity_ambiguous`, `ticket_closed`, `index_unavailable`,
      `repository_not_registered`;
    - Incomplete (exit 4): `discovery_pending`;
    - Transient (exit 5): `busy`.
14. Codes the bindings give in these cases:
    - `document create` on a taken path: `occupied_path`;
    - `folder create` on an occupied path: `occupied_path`;
    - `folder create` outside `docs/`: `invalid_path`;
    - `folder create` of an existing folder: a no-op.
15. A move to the document's own path takes the already-applied rule
    (`document.rs:550–553`, `only_at` `:512–527`). It is a no-op only when
    the document's branch tip holds the document there and nowhere else. A
    document on primary alone answers `external_change`.
16. A slug assignment whose composed code equals the committed slug is
    already applied, a no-op (`ticket.rs:1247–1268`). `slug_already_assigned`
    needs a different code already in the file.
17. **Unconfirmed:** whether a no-op save of an item that has no editing
    context creates that context. The citation given in the first review
    was the `identity_required` test, and
    `tests/mutation_replay/execute.rs:714`'s no-op test asserts no commit,
    not the context. Task 0 records the answer. The no-op tests use items
    that already have a context, so they hold either way.
18. The database is `manyhands.sqlite3` in the data directory
    (`REGISTRY_FILE`, `src/repository.rs:112`), in WAL mode
    (`discovery.rs:1539`).
    - After the file is deleted, opening creates an empty registry. Item
      reads then answer `repository_not_registered` with **no** recovery
      action. `resolve_repository` builds it so
      (`src/repository/read/resolve.rs:234–238`). `at_root` (`:68–80`) adds
      `index.rebuild` only to `index_unavailable`, and the read golden
      `tests/fixtures/read_v1/failure_repository_not_registered.json` has
      `"recovery": []`.
    - A mutation's own answer is different:
      - for a lost registration it offers `index.rebuild`
        (`src/repository/mutation/outcome.rs:804–807`,
        `tests/mutation_replay/cache_loss.rs:22–39`);
      - for a root that was never enabled it is `repository_not_enabled`
        (`not_registered`, `src/repository/mutation/bind/mod.rs:111–121`).
    - Bytes that are not a database leave the index unavailable
      (`repository.rs:5607–5621`), so reads answer `index_unavailable` with
      `index.rebuild`.
    - F2's tests remove every `manyhands.sqlite3*` file except `.lock`
      first (`tests/mutation_replay/index.rs:344–352`).
19. The repair fault point F2 uses is
    `AfterOwnedWriteBeforeLifecyclePersistence`. The stopped call answers
    `internal_error`, partial, exit 1
    (`tests/mutation_bridges/repair.rs:650–678`).
    `support::repair::Shelf` (`tests/support/repair.rs:89`) builds the
    nonconforming fixtures and is already in the `cli` target. The
    re-entry ignores the confirmation presented, so the CLI can present
    the one the actor used.
20. The index stores item titles (`discovered_items.title`,
    `discovery.rs:1623–1637`). F2's privacy helper
    `support::mutation::assert_request_stores_exclude` (`:157`) scans only
    the request stores (`REQUEST_STORES`, `:73`).
21. A partial save offers `operation.resume`
    (`tests/mutation_replay/execute.rs:551–583`), which is C4's verb, and
    not `request.retry`. So C2's driver does not retry it within the
    invocation.
    - While its operation row is pending, **every other request on the
      repository** is refused with `recovery_required`, exit 3, offering
      `operation.resume`, `repo.inspect` and `operation.abandon`. That
      includes other saves, creates and both index verbs
      (`tests/mutation_replay/index.rs:270–290`,
      `tests/mutation_replay/resume.rs:1323–1346`,
      `src/repository/recovery.rs:779–780`, `:1366–1392`).
    - The domain's index-pending refresh reaches the CLI as `busy`
      (`src/repository/mutation/outcome.rs:245–253`).
    - A re-entered save accepts the file only when it holds the bytes the
      request intended or the original observation. Any other content is
      `external_change` (`ensure_replayed_owned_destination`,
      `src/repository.rs:8194–8222`, called at `:2217` and `:2636`).
22. `hold_lease_in_child_for_test` (`tests/support/mod.rs:477`) holds the
    repository lease from a child process. C2 reaches `busy`, exit 5,
    through the binary with it (`tests/cli/exits.rs:512`).

**CLI.**

23. The thirteen C3 verbs are `planned(…, "C3")` at
    `src/bin/manyhands-cli/catalog.rs:662–679`.
    - `planned()` (`:209`) gives them no repository rule and no options.
    - `available()` (`:191`) is fixed to `"C1"` and `Kind::Read`, so C2's
      entries are literal `CommandSpec`s.
    - `prompts()` is true for every `Kind::Mutation`.
    - `MUTATION_INPUT` is at `:227`. `repo remove` uses
      `&[REPO, REQUEST_ID, PREPARE, CONFIRM]` (`:437`).
24. `parse::parse` replaces clap's error text only for verbs delivered by C2
    (`parse.rs:120`).
    - clap's unknown-argument error names the option and not its value. A
      stray positional value is echoed: "unexpected argument '<value>'".
    - "--context requires --path." is at `parse.rs:206`.
    - `validate` (`:191`) enforces each option's `required` flag.
    - A value option given twice keeps the last value
      (`parse_tests.rs:310–343`).
25. C2's driver:
    - `mutate::run` (`mutate.rs:120`) calls its `build` closure on every
      attempt;
    - `mutate::refusal` (`:273`) always uses an empty scope;
    - the `Boundary` trait is at `:40` and `RETRY_DELAYS_MS` at `:73`.
26. `InputObject` offers `only`, `string`, `optional_string`,
    `nullable_string`, `tristate`, `optional_object` and `one_of`
    (`input.rs:110–181`). There is no accessor for a JSON array.
    `nullable_string` refuses a missing field.
27. In `dispatch`, an available verb with no arm answers `internal_error`
    (`dispatch.rs:678–688`). The read `show` builds `item_not_found` with the
    item scope for an item of the other kind (`dispatch.rs:330`).
28. `human_admin::mutation_success` prints only the envelope message, the
    effects line and the data DTO's fields (`human_admin.rs:139`), so a
    title or status is never printed by a mutation. `preview` already
    formats `PreviewDetailsDto::Repair` (`:62`). C1's incomplete-read note
    helpers are private to `human.rs` (`:24–34`). `render::field` is
    `pub(super)` (`render.rs:158`).
29. The help JSON has no `kind` field (`startup.rs:140–147`). A mutation is
    recognized by its `--request-id` option.
30. C2's exit table is pinned to its own audit section and counts
    (`tests/cli/exits.rs:601`). C2's no-op and privacy inventories filter on
    `delivered_by == "C2"` (`tests/cli/human.rs:50`,
    `tests/cli/privacy.rs:413`). C1's dispatch and boundary inventories
    filter on C1.
31. The native workflow already runs `--bin manyhands-cli --test cli --test cli_remote`
    (`.github/workflows/build.yml:115`).
32. Pre-existing CLI tests that a C3 verb touches:
    - the `DESIGN` table (`catalog_tests.rs:64`), which keeps its `"C3"` row;
    - three help goldens: `help_root.json`, `help_root_text.txt` and
      `help_resource_ticket.json`;
    - tests that iterate every available or planned verb without a Cycle
      filter. They extend to C3's verbs and pass unchanged:
      `startup_tests.rs:181–184`, `:272`, `:332`, `:371`;
      `catalog_tests.rs:137`; `parse_tests.rs:216`.
33. C2's repository fixture writes `core.autocrlf = false` into the test home
    (`tests/cli/repository.rs:50–54`). `require_no_system_identity`
    (`tests/cli/repository.rs:14`) refuses a machine whose system Git
    configuration sets an identity.

**This repository (for the dogfooding step).**

34. The open Wave 03 Cycle tickets have no `deps`, `parent` or `slug` key:
    - C4 `01M4CC0VMWEQQ9G4W0DAG7ADPY`;
    - C5 `01M4CC0VMXTNXNH78C11DYXS8H`;
    - D3 `01M4CC0VN0MFTGKQ4HGTTC5H63`;
    - D4 `01M4CC0VN1KTQFSVR99378TDHV`;
    - D5 `01M4CC0VN2K8DBHNMZ46NTX5JJ`;
    - D6 `01M4CC0VN3VPVVQ3RNWJKXWEVX`;
    - G1 `01M4CC0VN4H9AC0Q3KA6E1QERC`;
    - the gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY`.

    Each lives only on its own branch and worktree. Its file holds quoted
    values, `manyhands_*` keys first, and unknown `wave` and `cycle`.
35. On 2026-10-24 the G1 worktree held an uncommitted title edit and an
    untracked `ticket.md~`. The owner cleaned it the same day, and it was
    re-checked clean at `7b306d1`. Every worktree in fact 34 was then clean.
36. None of the six branches to be saved has a remote ref or an upstream:
    C4, C5, D4, D5, D6 and G1. Only C3's branch and the gate ticket's are
    on origin.
    - Pushing them creates six remote branches.
    - The pushes also publish the commits each branch already holds beyond
      main: C4 2, C5 2, D4 1, D5 3, D6 1 and G1 1, checked on 2026-10-24
      with `git log origin/main..<branch>`.
    - Each branch is far behind main.

**Mutation data.**

37. Mutation data echoes some input. `DocumentMutationDto.path` and
    `RepairMutationDto.path` echo the path
    (`src/repository/mutation/dto.rs:906`, `:556`). `TicketMutationDto.slug` contains the initials, and its
    `unresolved` and `rejected` contain `deps` IDs (`:930–944`). Human
    `mutation_success` prints them all.

## Module Layout

New bin-private modules, declared in `src/bin/manyhands-cli.rs` with
`#[path = "manyhands-cli/<name>.rs"] mod <name>;`, each with a `*_tests.rs`
beside it:

- `authoring.rs`: per-verb glue for the eleven mutations, built as
  `admin.rs` is.
- `fill.rs`: the save patch (R-4). It is a pure merge from "the input's fields
  plus the item read" to a draft, plus one read wrapper.
- `human_authoring.rs`: the human text for `path observe` and
  `ticket check`.

Changed:

- `catalog.rs`: thirteen literal `CommandSpec`s (R-1).
- `parse.rs`: the usage-text rule (R-2) and C3's selector rules.
- `input.rs`: list readers (R-3).
- `dispatch.rs`: one arm per verb.
- `admin.rs`: makes its private helpers `input_failure` and `name`
  `pub(crate)`.
- `human.rs`: makes its two note helpers, `incomplete` and `index_note`,
  `pub(super)`.

## Command Catalog (R-1)

| Verb | Kind | Selectors | Delivering task |
| --- | --- | --- | --- |
| `path observe` | `Read` | `--repo`, `--path`, optional `--context` | 1 |
| `ticket check` | `Read` | `--repo`, `--id`, `--deps` and/or `--parent` | 1 |
| `index refresh` | `Mutation { confirmed: false, input: Refused }` | `--repo` | 2 |
| `index rebuild` | `Mutation { confirmed: false, input: Refused }` | `--repo` | 2 |
| `folder create` | `Mutation { confirmed: false, input: Required }` | `--repo` | 2 |
| `document create` | `Mutation { confirmed: false, input: Required }` | `--repo` | 3 |
| `document save` | `Mutation { confirmed: false, input: Required }` | `--repo`, `--id` | 3 |
| `document move` | `Mutation { confirmed: false, input: Required }` | `--repo`, `--id` | 3 |
| `ticket create` | `Mutation { confirmed: false, input: Required }` | `--repo` | 5 |
| `ticket save` | `Mutation { confirmed: false, input: Required }` | `--repo`, `--id` | 5 |
| `ticket slug-assign` | `Mutation { confirmed: false, input: Required }` | `--repo`, `--id` | 5 |
| `document repair` | `Mutation { confirmed: true, input: Required }` | `--repo` | 6 |
| `ticket repair` | `Mutation { confirmed: true, input: Required }` | `--repo` | 6 |

Every C3 entry is `Available("C3")` from the task that dispatches it, with its
input schema, example and goldens. Until then it stays `planned(…, "C3")` and
answers `command_unavailable`. Each literal entry keeps its planned summary
text. All thirteen require `--repo`, so
`catalog_repository_rules_and_planned_metadata_match_the_design` holds
without an edit.

## Grammar And Per-Verb Rules

The rules below are checked after clap and the mode decision. Each breach is
`usage_error`, as in C2.

| Verbs | Required | Not declared (clap usage error) |
| --- | --- | --- |
| `folder create`, `document create`, `ticket create`, `document repair`, `ticket repair` | `--repo`, `--input` | `--id` |
| `document save`, `document move`, `ticket save`, `ticket slug-assign` | `--repo`, `--id`, `--input` | — |
| `index refresh`, `index rebuild` | `--repo` | `--id`, `--input` |
| `path observe` | `--repo`, `--path` | `--id`, mutation options |
| `ticket check` | `--repo`, `--id`, at least one of `--deps`, `--parent` | `--path`, mutation options |
| every C3 mutation, not interactive, without `--prepare` | `--request-id` | — |
| every C3 mutation | — | `--request-id` with `--prepare`; `--prepare` with `--confirm` |

Values:

- `--id` parses as C1's `item_id`. A malformed one is `invalid_id`, exit 2,
  after the service opens. For a mutation it is built with `mutate::refusal`,
  keeping a valid request ID.
- `--repo` for a mutation is joined to the invocation directory
  (`admin::mutation_root`). The library refuses a linked worktree's root
  with `not_repository_root`.
- `--path` on `path observe` is repository-relative. `--context` is a
  worktree path, relative to the invocation directory, passed as
  `document show --path --context` passes it.
- `--deps` takes one comma-separated list of canonical ULIDs, and `--parent`
  one ULID. A malformed or empty element is `invalid_id`. As with every C1
  option, a repeated `--deps` keeps its last value; its help summary says to
  give it once.

## Input

Reading, BOM handling, duplicate-key refusal and error details are C2's,
unchanged. Content validation stays the library's. The CLI checks shape,
and it parses `item_id` because the library input is typed `ItemId`.

**List readers (R-3).** `InputObject` gains:

- `optional_string_list(field) -> Result<Option<Vec<String>>, InputError>`,
  which refuses `null`;
- `tristate_list(field) -> Result<ListChange, InputError>`, where `ListChange`
  is `Missing | Null | Value(Vec<String>)`.

Both refuse a non-array, and a non-string element, as a wrong type. Neither
de-duplicates or sorts.

**Fields.** "Optional" means the key may be missing. "Nullable" means `null`
is a value with its own meaning (R-5).

| Verb | Fields |
| --- | --- |
| `folder create` | `path`: string, required. |
| `document create` | `item_id`: string, required, a canonical ULID. `path`, `title`, `body`: strings, required. |
| `document save` | `observation`: string, required. `title`, `body`: strings, optional. |
| `document move` | `observation`, `destination_path`, `destination_observation`: strings, required. |
| `document repair`, `ticket repair` | `path`, `observation`, `source`: strings, required. |
| `ticket create` | `item_id`: string, required, a canonical ULID. `title`, `type`, `status`: strings, required. `project`, `team`, `body`, `initials`: strings, optional; a missing `body` is empty. `deps`: array of strings or `null`, optional; missing, `null` and `[]` all mean none. `parent`: string or `null`, optional; missing and `null` mean none. |
| `ticket save` | `observation`: string, required. `title`, `type`, `status`, `body`: strings, optional. `project`, `team`: string or `null`, optional; `null` clears. `deps`: array of strings or `null`, optional; missing leaves them as they are, `null` or `[]` clears them. `parent`: string or `null`, optional; `null` clears it. |
| `ticket slug-assign` | `observation`: string, required. `initials`: string, optional. |

- `item_id` that is not a canonical ULID is `invalid_input`, naming the
  field. A selector stays `invalid_id`.
- `deps` and `parent` text passes through, and the library refuses a
  malformed entry with `invalid_relationship`. The same text in
  `ticket check`'s flags is `invalid_id`, because flags are parsed by the
  CLI.
- `closed_at`, `closed_by`, `slug` and every other unknown key are refused
  as unknown.

**Schemas.** One closed schema per input-taking verb in `schemas/input/v1/`,
named `<resource>_<verb>.schema.json` with hyphens as underscores: nine
files. The index verbs take no input. Nullable fields use
`"type": ["string", "null"]` or `["array", "null"]`, and lists use `items`.
The checker supports both (`tests/support/schema.rs:93`, `:237`), and C2's
`remote_select.schema.json` already uses a type list. One example input per
schema is in `tests/fixtures/cli_input_v1/<stem>.json` (R-13).

## The Save Patch (R-4, Cycle Decisions 2, 5 And 7)

`fill.rs` builds the draft that `document save` and `ticket save` give the
library.

1. **When.** A read happens only when at least one draft field is omitted:
   - for documents, `title` or `body`;
   - for tickets, `title`, `type`, `status`, `body`, `project` or `team`.

   With every draft field present, no read happens.
2. **Order.** `admin::flags` parses the request ID first and, for an
   interactive call without one, allocates it and writes `request: <ULID>`
   to stderr. The input is read next. Only then does the CLI call
   `service.resolve_repository(root)` and `service.show_item(&repo, &id)`,
   once per invocation and before `mutate::run`, so bounded retries reuse
   the same draft.
3. **Filling.**
   - Each omitted field takes the read's value.
   - `title`, `type`, `status` and `body` are present on any conforming item
     (fact 8).
   - An omitted `project` or `team` takes the read's value, which is absent
     only when the key is absent.
   - A `null` `project` or `team` is `None`.
   - `deps` and `parent` are never filled: missing is
     `RelationshipWrite::Unchanged`.
4. **Refusals (decisions 5 and 7).** The read's answer passes through under
   the mutation's command name.
   - A read that fails gives `ReadError::to_envelope(<command>)`, with its
     code, message, scope and recovery actions.
   - An item of the other kind, or one whose read lacks a draft field (a
     nonconforming file reads as a bare item, `items.rs:1811–1822`), gives
     `item_not_found` with the item scope, built as C1's `show` builds it
     (`dispatch.rs:330`).
   - In both cases the CLI keeps the parsed or allocated request ID and sets
     every effect to `not_requested`. The CLI RFC's refusal rule is amended
     for this.
   - **Decision 7.** The read can answer differently from the library's own
     save path (fact 18). For a root that was never enabled, a save that
     omits a field answers `repository_not_registered`, exit 3, with no
     recovery action. A save that sends every field answers
     `repository_not_enabled`, exit 2. After a cache loss, both answer
     `repository_not_registered`, but only the full-field save offers
     `index.rebuild`. This is accepted, tested both ways, and documented
     in Known Limits and the CLI RFC amendment.
   - **The read comes before the replay.** A retry of a finished request
     whose fill read now fails gets that read's answer, not the stored
     result. An example is `item_not_found` once another item sits at the
     item's path (`items.rs:1800–1802`). `request show` still gives the
     stored result.
5. **The observation.** The caller's `observation` goes to the library
   unchanged. The CLI never compares it with the read's token: a retry after
   a lost result reads the file the request itself wrote, and a comparison
   would turn that replay into `external_change`.

**Required outcomes.** "Equal" means equal values of these `show_item`
fields:

- `id`, `kind`, `path`, `title`, `type` and `status`;
- `project`, `team` and `closure`;
- `slug`, `parent` and `deps`;
- `unknown_metadata`, `problems` and `body`.

`observation`, `source`, `changed_at`, `change_source`, `context`,
`readiness` and `index` change with a write, and are not compared. Bytes are compared only for the
body, and for files the library itself wrote (fact 9). Each row has a test in
Task 3 (documents) and Task 5 (tickets), on an item the library created
unless the row says otherwise.

| Sequence | Outcome |
| --- | --- |
| Save with only `body`, the observation current | One checkpoint. Every other value and unknown key equal; the body byte for byte. |
| Save with every draft field omitted, on an item the library wrote | `noop`, no commit, the file byte-identical. |
| Save with every draft field omitted, on a hand-written document | One checkpoint that only normalizes the front matter: values equal. |
| A third party changed `title`; the caller saves only `body` with the old token | `external_change`, exit 3; nothing written. |
| The output of a successful save is lost; same request, same input | The stored result is replayed; one commit. |
| As the row above, but a third party then changed an omitted field | `request_mismatch`; one commit; `request show` returns the stored result (decision 2's replay limit). |
| A library actor's save stops after its checkpoint; the CLI retries the same request with a field omitted | The fill reads what that attempt committed, the digest matches, and discovery completes with no second commit. |
| Never-enabled root, a field omitted versus every field sent | `repository_not_registered` with no recovery action, versus `repository_not_enabled` (decision 7). |
| Lost cache, a field omitted versus every field sent | `repository_not_registered` with no recovery action, versus the same code with `index.rebuild` (decision 7). |
| `ticket save --id <a document's ID>` with a field omitted | `item_not_found`, exit 2, item scope, no request record. |
| `project: null` | `project` absent afterwards; every other value equal. |
| `deps: []` and `deps: null` | `deps` absent afterwards. |
| `deps` omitted | `deps` equal. |

**Interrupted saves: required outcomes (test-first, with a design
checkpoint).** Three review rounds each found the documents' account of
this case wrong, so this table now states the contract. Task 3 writes these
tests **first**, against the library as it is, before any `document save`
code. If the library answers any row differently, the task stops for a
controller ruling at a design checkpoint. The text is not rewritten to fit.

Each row starts from the same state: a library actor's save of a document
with a context stops after its checkpoint, so the request's record is
accepted and its row pending (fact 21).

| Sequence after that | Required outcome |
| --- | --- |
| Nothing else happens | Every other request on the repository, including `index refresh` and a save of another item, answers `recovery_required`, exit 3, with nothing written. |
| The CLI retries the same request with a field omitted; the file is unchanged | Discovery completes. There is no second commit, and the other requests then succeed. |
| Someone edits an omitted field in the worktree, uncommitted, then the CLI retries with that field omitted | The fill reads the edit, so the input differs: `request_mismatch`. Nothing is written, and the record stays accepted. |
| As the row above, but the CLI retries with every field sent | `external_change` (fact 21: the file is neither the request's bytes nor its original observation). Nothing is written, and the record stays accepted. |
| As either row above, after the file is restored to the bytes the request committed | The retry with a field omitted completes, with no second commit. |

Recovery past these rows is C4's `operation abandon`. C3 offers no other way
out of a stuck request. A caller following a CLI answer is told
`recovery_required` with `operation.abandon`, which `command_unavailable`
names as C4's.

## Reads

**`path observe` (R-7).** `observe_path(&repo, context, path)`, resolving the
repository as `document show --path` does. The data is `PathObservationDto`
(schema `path_observation`). The CLI adds no rule: another item's path in
the named context is reported as what is there. Human output, one escaped
field per line:

```text
path: docs/plans/x.md
exists: no
observation: <token>
context: <worktree> (<branch>)
```

Its uses:

- the absent token that `document move` needs, observed in the document's
  context. That is the worktree `document show --id` reports, or the root
  when the document has none;
- the token of a nonconforming file on primary that repair needs. That is
  the same token `document show --path` gives.

**`ticket check` (R-6).**
`check_ticket_relationships(&repo, &id, &ProposedRelationships { deps, parent })`.
The data is `RelationshipCheckDto` (schema `relationship_check`).

- It checks the ticket with exactly the proposed relationships (fact 7). An
  omitted `--deps` proposes no dependencies and an omitted `--parent` no
  parent, as `ticket create` writes them.
- To preview a `ticket save` that changes one of them, pass the other's
  current value too, as `ticket show` gives it. For the same complete
  proposal, `ticket check` and the save give the same rejection.
- A proposed cycle is reported in `data.rejection`, and the answer is still
  a read success: `ok`, exit 0.
- Human output lists the proposed deps and parent, the unresolved IDs and,
  when set, a final line `rejected: <code>: <id> <id> …`.
- When `complete` is false or the index is not current, C1's incomplete-read
  note goes to stderr.

## Index Verbs (R-8)

`index refresh` and `index rebuild` build
`Mutation::IndexRefresh(IndexRefreshInput { root })` and
`Mutation::IndexRebuild(IndexRebuildInput { root })` through `mutate::run`,
with `Plan { identity_prompt: false, retry: true }`.

- They take no input. JSON and non-interactive calls need `--request-id`.
- Human success is `mutation_success`, and the data is null.
- The recovery actions `index.refresh` and `index.rebuild` now name
  available commands. A caller following one supplies a request ID.
- Neither contacts a remote. The test configures a remote whose refs differ,
  and asserts that `refs/remotes` is unchanged.

## Repair

`document repair` and `ticket repair` use C2's confirmed flow, unchanged:

- `--prepare` returns the repair preview (`repair_preview`), which states
  whether an ID is adopted;
- `--confirm <id> --request-id <id>` executes it;
- an interactive human invocation shows the preview on the terminal and asks
  `Proceed? [y/N]`.

The source may come from `--input -` while consent comes from the terminal
(C2 contract item 6). The source is never echoed. A repaired document is
listed twice until its context merges (F2).

## Identity Recovery (R-9)

No C3 verb prompts for identity: `Plan { identity_prompt: false }`.

- A save, create, move, slug assignment or repair with no usable identity
  answers `identity_required`, exit 3, with `repo.identity_set`.
- After `repo identity-set`, the caller reads a fresh observation with
  `ticket show` or `document show`, because the refused save left a new
  context that the token covers. It then retries under the same request ID,
  which is free because the record was deleted (fact 11), and the retry
  succeeds.
- `initials_required` on `ticket create` follows the same pattern. Task 0
  confirms that its record is freed too. The retry adds `initials` to the
  input under the same request ID, or sets them with `repo identity-set`.

## Human Output

- Mutations use C2's `mutation_success`, `failure_extra` and `preview`,
  unchanged. Their data DTOs are content-free:
  - `DocumentMutationDto { id, path }`;
  - `TicketMutationDto { id, path, slug, unresolved, rejected }`;
  - `RepairMutationDto { id, path, adopted }`.
- `path observe` and `ticket check` use `human_authoring.rs`, which escapes
  every value with `render::field`.
- Progress stages go to stderr in human mode, as in C2. A test asserts one.

## Exit Statuses

C1's class mapping is unchanged. The codes each verb can return are the
output of Task 0's audit refresh.

| Exit | Reached by |
| --- | --- |
| 0, 2, 3 | Through the binary, for every class a C3 verb can produce. |
| 4 | Listed in the exit table's reasons, backed by the R-11 test in which a library actor leaves a save partial and the CLI completes it (decision 6). |
| 5 | Through the binary, holding the lease with `hold_lease_in_child_for_test` while a C3 save runs (fact 22). |
| 130 | Through the pseudo-terminal, declining a repair's consent. |
| 1 | A row or a reason, as Task 0 records. |

As in C2, the audit block lists the library's codes. The test adds the
CLI-built codes C3 verbs can give as its own `CLI_ONLY` list, and the union
of the two is what the rows and reasons must equal. The CLI-built codes are
`usage_error`, `invalid_input`, `invalid_id`, `invalid_path`, `cancelled` and
`internal_error`.

## Dogfooding (R-16, Cycle Decision 3)

This runs in the final task, after the full gate. The ledger records each
request's ID and input before it is sent, and each mutation's envelope with
the compared values after it. It never records `body` or `source`, because
these tickets are partly unpublished.

1. **Build.** `devenv shell -- cargo build --locked --bin manyhands-cli` in
   the C3 worktree. Every following CLI command is
   `<worktree>/target/debug/manyhands-cli --json` with
   `--repo /home/vhodges/work/src/manyhands`.
2. **Preflight, read-only, before asking.** For each target ticket in
   decision 3, record:
   - `git -C <worktree> status --porcelain`;
   - whether `origin/manyhands/ticket/<id>` exists;
   - `git log --oneline origin/main..manyhands/ticket/<id>`: the commits a
     push would publish (fact 36).

   Also record `repo inspect` on the main checkout with the default data
   directory. A worktree that is not clean is reported, and nothing is
   reverted.
3. **The go-ahead.** Ask the owner, presenting the preflight's results and
   naming:
   - the repository `/home/vhodges/work/src/manyhands`;
   - the default application data directory;
   - the six saves;
   - six pushes that create six remote branches and publish the commits
     listed;
   - that each save re-emits that ticket's whole front matter in the
     library's order and style (fact 9), with values unchanged;
   - the risk that a save which stops part-way, and whose ticket is then
     edited, blocks every CLI mutation on this repository until its file is
     restored or C4's `operation abandon` arrives (fact 21, the
     interrupted-saves table).

   Without the go-ahead, the step is owed. The Cycle may become review-ready,
   but it is not closed or merged while the Wave's dogfooding evidence is
   owed, unless the owner amends that evidence.
4. **Refresh.** `index refresh` with a request ID from `id new`.
5. **Record.** For each target in turn, in this order:
   1. **Read.** `ticket show --id <id>` gives the observation and the values.
   2. **Clean check.** `git -C <worktree> status --porcelain` must be empty.
      Otherwise stop, because `show_item` reads the worktree's file and the
      save would commit an uncommitted edit (`items.rs:1790`). An edit made
      after the read makes the observation stale, so the save refuses it.
   3. **Ledger.** Take a request ID from `id new`. Write the ID, the input
      `{observation, deps}`, and the read's compared fields to the ledger.
      The body is recorded as its SHA-256 digest (`sha256sum`) and byte length, never its
      text.
   4. **Save.** `ticket save --id <id> --request-id <that ID> --input -`.
      Every draft field is omitted, so the save patch fills them.

   Each save commits one checkpoint on that ticket's branch. A `noop` for a
   ticket that already holds exactly those deps counts as done.
6. **Check by value.** For each target, `ticket show` gives these results,
   checked against the ledger's record of step 5:
   - every compared field (Save Patch, Required outcomes) equal, except
     `deps`, with the body checked by digest and length;
   - the `deps` IDs exactly decision 3's list.

   This check reads `deps` directly. `ticket blocked` lists only open
   blockers, so it would fail falsely once a dependency closes.
7. **Push.** Push every target whose local tip is not on origin, with
   `git push origin manyhands/ticket/<id>`. This includes a branch whose save
   was a `noop` on a resume.
8. **Switch.** Only after every save, check and push succeeded:
   - **AGENTS.md's Dogfooding Workflow:**
     - ticket create and ticket update use `manyhands-cli ticket create` and
       `ticket save` against the main checkout, run with
       `devenv shell -- cargo run --locked --bin manyhands-cli -- …` from
       an up-to-date main;
     - before a `ticket save` of one's own Cycle ticket, commit one's own
       edits to its `ticket.md`, because the save commits into that active
       worktree;
     - closure stays a canonical-file edit until C5's `ticket close`, and
       comments stay canonical files until C4's `comment add`.
   - **`.agents/skills/file-ticket/SKILL.md`:** rewritten around `id new`,
     `ticket create`, `ticket show` and `ticket save`.
   - **`working-on-a-ticket`:** deliberately unchanged in C3. Its status
     edits and closure are commits inside the ticket's own worktree, made
     from sandbox clones in loop mode, where the main checkout's
     registration is absent. Moving them to the CLI changes that skill's
     own procedure, which is separate work. AGENTS.md names it as the
     exception until that skill is revised.
   - **`implementing-a-cycle`:** already says "using supported CLI
     operations", so it needs no edit.
   - **`planning-a-cycle`:** keeps comments as canonical files, which stays
     true until C4.

   The switch's commit gets its own independent review before
   review-ready.

**Stop rules.** Any of the following stops the sequence before any further
save or push:

- a worktree that is not clean;
- an answer other than `ok`, or other than step 5's `noop`;
- a failed value check;
- a rejected push.

When the sequence stops:

- the failure is recorded in the ledger and reported to the owner;
- nothing is worked around with Git or a hand edit;
- the switch in step 8 is not made.

**Resume.** A resume repeats the preflight and asks for a fresh go-ahead.
It then works in this order:

1. For every request the ledger holds without an `ok` or `noop` answer,
   first run `request show --request-id <id>`. Its state decides what
   follows:
   - **Finished:** record the stored result as the answer. The request is
     settled and nothing is resent.
   - **Accepted:** the request has a pending row, which blocks every other
     request on the repository (fact 21). Run step 5's clean check, then
     resend with the **same** ID and input. Any answer but `ok` stops the
     sequence for the owner, as the interrupted-saves table describes.
   - **`request_not_found`:** the request was never accepted, or it was
     refused and its record deleted. Redo step 5 in full, with a fresh read,
     clean check, ledger entry and request ID, so that a stale observation is
     never resent.
2. Then run step 5 for every target that has no ledger entry.
3. Step 7 then pushes every target whose tip is not on origin.

The library writes the checkpoint commits with its own message, so they carry
no `Co-Authored-By` trailer.

## Testing

All new tests are modules of the existing `cli` target (R-15). Fixtures
write `core.autocrlf = false` and no identity into the test home, as C2's
repository fixture does (fact 33). Each verb task adds:

| File | Covers |
| --- | --- |
| `tests/cli/authoring_reads.rs` | `path observe`: an absent path, a present path, a named context, another item's file in a context, an inactive context, escaping paths, and a path holding escape sequences shown escaped. `ticket check`: resolved, unresolved, a dependency cycle and a parent cycle, an omitted option proposing none, malformed and empty elements. Both reads write nothing. |
| `tests/cli/index.rs` | Refresh and rebuild; never fetching; index loss and whole-file corruption (R-12); a replay of each. |
| `tests/cli/folders.rs` | Create, no-op, occupied, outside `docs/`, replay. |
| `tests/cli/documents.rs` | Create (with an escaping path and a used path); save (the save-patch table for documents, including decision 7's two pairs, and the interrupted-saves table, test-first); move (collision, stale destination, a destination that gained a file after it was observed, a document with no context, its own path, an escaping destination); offline checkpoints; stale writes; replays; the interrupted save; cache loss; progress. |
| `tests/cli/ticket_authoring.rs` | Create with deps, parent and initials, including `null`; save (the save-patch table for tickets, relationships); a ticket in this project's format (fact 34) saved with only `observation` and `deps`; slug-assign; dependency and parent cycles on create and on save; `ticket check` agreeing with a save; unresolved targets; a closed ticket; `slug_already_assigned`; closure and `slug` keys refused; replays; identity recovery with a fresh observation, and initials recovery (R-18). |
| `tests/cli/repair.rs` | Prepare and confirm, previews for both repairs, adoption, adoption retry (R-11), invalid and malformed repair, `identity_ambiguous`, a closed ticket, cache loss, and pseudo-terminal consent with the source piped through `--input -` (Unix). |
| `tests/cli/authoring_exits.rs` | The C3 exit table (R-10), `busy` with the lease holder, the confirmed-flag check for all eleven mutations, `invalid_input` for each input-taking verb, the no-op inventory (R-14), human mode for all thirteen verbs, privacy (R-17) and the examples (R-13). |

**Replays.** Each of the eleven mutations has a lost-output case: the same
request and input sent again, answered from the record, with no second
effect.

**Cache loss.** These follow F2's cache-loss rows; Task 0 records their
codes:

- a committed `document save` and a committed `ticket save`, retried under
  the same request after the database is deleted and `index rebuild` runs,
  write no second commit;
- a repair's `--confirm` after cache loss answers `confirmation_not_found`.

**Library-level actors (R-11).** Built with `execute_with` or
`execute_confirmed` on a `FailOnce` service over the CLI's data directory,
scoped in a block (fact 12):

- commit success, refresh failure: a `document save` at
  `BeforeIndexTransactionCommit`, of a document that `document create`
  made, so that it has a context. The CLI retries with `title` omitted, and
  the branch tip is read with `git2` before and after;
- adoption retry: a `document repair` at
  `AfterOwnedWriteBeforeLifecyclePersistence`, built with
  `support::repair::Shelf` (fact 19).

**Index loss and corruption (R-12).** In `contract_repository`:

- remove `manyhands.sqlite3-wal` and `manyhands.sqlite3-shm`, then delete
  `manyhands.sqlite3`. `document list` answers `repository_not_registered`
  with an empty recovery list (fact 18);
- remove the same two files, then overwrite `manyhands.sqlite3` with 4096
  bytes of `0xA5`. `document list` answers `index_unavailable` with
  `index.rebuild`.

Each then runs `index rebuild --repo` and asserts:

- `ok`;
- every item the fixture holds listed again;
- worktree bytes and `git for-each-ref` output equal to before.

**Goldens.** As C2:

- registered in `CLI_CASES` or `CLI_TEXT_CASES`;
- updated with `MANYHANDS_UPDATE_GOLDEN=1`, never under `CI`;
- every diff read before commit.

Each verb has a JSON success golden `<resource>_<verb>_ok`, at least one
refusal golden `<resource>_<verb>_<code>`, and a human text golden
`<resource>_<verb>_text`. Both repairs also have `<resource>_repair_preview`.
Hyphens become underscores. Each task names the refusal codes it pins.

**Isolation (R-18).**

- Children start through `cli_command` without a controlling terminal. The
  pseudo-terminal helper is used only for the repair consent cases.
- No test touches the developer's keys, global Git configuration or
  application data.
- The identity-required cases call `require_no_system_identity()`.

**Native.** Linux locally; the `cli` target is already in the native list.
Pseudo-terminal cases are `#[cfg(unix)]`, and the `pty` import is gated too.
Windows repair consent is a native manual check, as C2's prompts were.
Native CI runs on main after merge.

## Named Edits To Pre-Existing Tests

| Edit | Task | File and change |
| --- | --- | --- |
| E1 | 1–6 | The help goldens `tests/fixtures/cli_v1/help_root.json`, `help_root_text.txt` and `help_resource_ticket.json` are regenerated as each C3 verb becomes available. The diff may change only that verb's availability, repository rule and options. Its summary text and delivering Cycle stay as they were. |

No other pre-existing test changes. The unfiltered tests in fact 32 extend
to C3's verbs and pass unchanged. Task 0 proves it with the plan's grep, and
a hit outside fact 32 stops for an owner ruling.

## Document Amendments

Task 8 makes these, except where noted.

**CLI contract RFC**, "Amended 2026-10-24 by C3":

- the taxonomy rows of the thirteen verbs, with selectors and input fields;
- "C3 verbs are available; C4–C5 remain planned";
- the save patch, its refusals and its replay limit (decisions 2 and 5);
- the CLI-built refusal rule (R-6 section): the fill read's answer passes
  through under the mutation's name (decision 5), including decision 7's
  difference for a repository the index does not hold;
- `path observe` and `ticket check` grammar and semantics;
- index verbs needing `--request-id`;
- identity recovery without a prompt;
- the C3 rows of the input-schema table;
- the cache-loss paragraph: F2's codes replace `recovery_required` for C3's
  verbs, citing F2's "Departures from the RFCs";
- `folder create` works on primary only (F2);
- the evidence paragraph: the C3 test files and the example inputs.

**Relationships RFC:**

- `ticket check`'s flags and semantics;
- `deps` and `parent` accepting `null` on create and save.

**Wave 03:** the C3 section, "Amended 2026-10-24 by C3", records decisions
1–7, the delivery and the known limits.

**API audit:** the C3 refresh section (Task 0) and the C3 rows updated in
place.

**AGENTS.md** and the **`file-ticket` skill:** the dogfooding switch, in
Task 9 after the saves succeed.

**The C3 ticket:** its Added Scope is corrected, since initials and the
prefix shipped in C2, and its planning boxes are ticked (Task 9).

## Rulings

Each ruling is listed with its cost if wrong.

- **R-1 Catalog.** Literal `Available("C3")` entries, flipped in the task
  that dispatches the verb, with planned summaries kept. *Cost:* none.
- **R-2 Usage privacy.** `parse.rs:120` becomes
  `spec.delivered_by() != "C1"`. Its test uses a stray positional value,
  which clap does echo. *Cost:* C3's reads lose clap's detailed usage hint.
- **R-3 List readers.** `optional_string_list` and `tristate_list`, with no
  de-duplication. *Cost:* small.
- **R-4 The save patch.** The fill rules above, with decision 5's refusals.
  *Cost if wrong:* decision 2's replay limit. A wrong fill would change a
  value the caller did not name, which the required-outcomes table guards
  against.
- **R-5 Optionality.** Where the RFCs leave room:
  - `document save`'s `body` is optional, as `ticket save`'s is;
  - `ticket create`'s `body` defaults to empty;
  - `null`, `[]` and missing mean "none" on create, as the relationships
    RFC's "an empty list or null clears it" reads for a new ticket.

  *Cost:* narrowing an optional field later needs a new input version.
- **R-6 `ticket check`.**
  - It proposes exactly what it is given (fact 7).
  - `--deps` is a comma-separated list, and a repeat keeps the last, as
    C1's options do.
  - At least one of `--deps` and `--parent` is required.
  - A rejection is a read success with exit 0, reported in
    `data.rejection`.

  *Cost:* a script must inspect `rejection`, not the exit code. Changing the
  exit later changes a published outcome.
- **R-7 `path observe`.** Flags as `show --path --context`, with no CLI rule
  about whose file is at the path. *Cost:* none.
- **R-8 Index verbs.** Unconfirmed mutations with no input, needing
  `--request-id` in JSON. *Cost:* agents following `index.rebuild` must
  allocate an ID.
- **R-9 Identity.** A pass-through, with no prompt. The retry reads a fresh
  observation and may reuse the request ID (fact 11). *Cost:* an interactive
  user runs `repo identity-set`, reads the item again and repeats the
  command.
- **R-10 Exit table.** `tests/cli/authoring_exits.rs`, pinned by set
  equality to the audit section "C3 exit-coverage union". Exit 4 is a
  reason backed by the R-11 test (decision 6). C2's table is untouched.
  *Cost:* two tables to keep.
- **R-11 Library-level actors.** Used for the two interrupted cases. The CLI
  retry of the interrupted save omits a field, so the fill is exercised.
  *Cost:* the interruption itself is not produced by the binary.
- **R-12 Corruption evidence.** Deletion and whole-file replacement only,
  with the WAL files removed first. The partial form belongs to `01M4PB82`
  (decision 1). *Cost:* that form remains unrecoverable from the CLI.
- **R-13 Documented examples.** The example inputs are the documented
  examples. `every_c3_example_input_runs` owns the outcome table, and the
  CLI RFC's amendment copies and links it. *Cost:* the examples are files,
  not prose.
- **R-14 No-op inventory.** A no-op is a fresh request that writes nothing,
  on an item that has an editing context (fact 17). Task 0 lists which of
  the eleven mutations can be one; slug-assign can (fact 16). Each that
  cannot has a reason. *Cost:* none.
- **R-15 Test layout.** New modules in the `cli` target. *Cost:* the target
  grows. A new target would need a CI list change.
- **R-16 Dogfooding.** The procedure above: build, preflight, go-ahead, a
  clean check before each save, checks by value, stop and resume rules, the
  switch's scope and its own review. *Cost:* real-repository effects, bounded
  to six ticket branches and gated by the go-ahead. Those effects are six
  new remote branches publishing their existing commits (fact 36), and each
  changed ticket's front matter re-emitted.
- **R-17 Privacy.** The data directory is checked with
  `assert_request_stores_exclude`, because the index rightly stores titles
  (fact 20).
  - Sentinels in `body` and `source` are checked in stdout, stderr and the
    request stores.
  - Sentinels in `title`, `path`, `initials` and `deps` are checked only in
    the diagnostics: CLI-built refusals, human stderr and the JSON
    `message`.
  - Mutation data that echoes input by design is exempt: `path`, `slug`,
    `unresolved` and `rejected` (fact 37). So are the reads that return
    these values.

  This covers every input-taking verb in both modes, including a refusal
  after the fill read. *Cost:* none.
- **R-18 Identity machine.** The identity cases call
  `require_no_system_identity()` (C2 R-17), which fails with its message, not
  a skip, on a machine whose system configuration sets an identity.
  *Cost:* such a machine cannot run them.

## Known Limits

- Decision 1's three defects, each on its ticket.
- Decision 2's replay limit.
- **Decision 7.** With a field omitted, a save on a repository the index does
  not hold answers the read's `repository_not_registered` with no recovery
  action. A full-field save answers `repository_not_enabled`, or
  `repository_not_registered` with `index.rebuild` (fact 18).
- The fill read comes before the replay. A retry of a finished request
  whose fill read now fails gets the read's answer; `request show` still
  gives the stored result.
- Every write re-emits the front matter (fact 9), and saving a hand-written
  document commits that normalization.
- Whether a no-op save of an item without an editing context creates that
  context is unconfirmed (fact 17).
- A partial save offers `operation.resume`, which is C4's verb (fact 21).
  While its row is pending, every other request on the repository answers
  `recovery_required`. The same request's retry completes it while the file
  holds what the request wrote. If the file was edited meanwhile, the only
  way out in C3 is to restore those bytes; otherwise it waits for C4's
  `operation abandon`. The interrupted-saves table is the contract.
- A repaired document is listed twice until its context merges (F2).
- An empty folder is not visible to Git. `folder list` lists it after the
  refresh that `folder create` makes (F2).
- `identity_required` leaves an editing context with no commit, which the
  next successful save reuses (fact 11).
- Item reads answer `item_not_found` for an item whose context is
  duplicated or mismatched (ticket `01M5B163PQTE9C1T6NMHATR3KX`). The save
  patch inherits that answer.

## Alternatives Rejected

- **Complete drafts on save.** Exact replay, but it departs from the CLI RFC
  (decision 2).
- **A library patch.** It needs a prerequisite ticket and contradicts F2's
  split (decision 2).
- **Forcing fill refusals into R-6's closed shape.** A wrong-kind ID has no
  fitting code there (decision 5).
- **`body_path` or `--body-file`.** A second body channel. Bodies are JSON
  strings, and `jq --rawfile` builds the input.
- **A repeatable `--dep`.** `OptionSpec` has no repeatable form.
- **Exit 3 for a `ticket check` rejection.** It would make a read fail for
  answering what it was asked (R-6).
- **A separate test target.** It needs a CI list change and gains nothing.

## Risks

| Risk | Control |
| --- | --- |
| The fill changes a value the caller did not name. | The required-outcomes table, compared by value. |
| A body or source reaches a diagnostic. | R-17 over every verb and mode. Mutation output prints only content-free DTOs. |
| A verb is advertised before it works. | R-1: availability flips in the dispatching task. |
| Help goldens drift beyond the verb's own lines. | E1's diff rule. |
| Feature-dependent failures surface late (C2's `preserve_order`). | Part gates include `--all-features`. |
| The full suite outlasts a tool timeout. | It runs detached, with a two-hour budget. |
| Dogfooding commits a stray edit, reformats files unexpectedly, or leaves a request unsettled. | A preflight before the go-ahead, which names the re-emission and the new remote branches; a clean check before each save; checks by value; resume resends unsettled requests under their own IDs. |
| A library gap appears mid-task. | Stop, and raise a separate ticket under track rule 5. |
