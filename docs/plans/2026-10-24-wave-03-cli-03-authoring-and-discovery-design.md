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

**Draft, for product-owner approval.** The planning questions are answered
(Cycle decisions 1–4). Nothing is approved and nothing is implemented. No
build or test was run for this draft.

## Intent And Approval Boundary

This design turns the [Cycle](../Cycles/wave-03-cli-03-authoring-and-discovery.md)
into verbs, inputs, rules and tests. The Cycle governs this design, and this
design governs the
[implementation plan](2026-10-24-wave-03-cli-03-authoring-and-discovery-implementation.md).

Approval of this design covers:

- the per-verb grammar and input fields;
- the save patch's mechanics (R-4, R-5);
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

Read at branch head `9ed4a04` (main `e49f978` merged). Facts marked *(Task 0)*
came from read-only research. They were not re-read for this draft, and Task 0
re-checks them before any code.

**Library.**

1. `Mutation` (`src/repository/mutation/mod.rs:398`) has a variant for each of
   C3's eleven mutations: `FolderCreate`, `DocumentCreate`, `DocumentSave`,
   `DocumentMove`, `DocumentRepair`, `TicketCreate`, `TicketSave`,
   `TicketRepair`, `TicketSlugAssign`, `IndexRefresh` and `IndexRebuild`.
2. The inputs, all under `src/repository/mutation/bind/`:
   - `DocumentCreateInput { root, id: ItemId, path: PathBuf, draft: DocumentDraft }`
     (`document.rs:47`);
   - `DocumentSaveInput { root, id, draft, observation: Option<String> }`
     (`document.rs:59`), with `None` refused as `invalid_input`;
   - `DocumentMoveInput { root, id, observation: Option<String>, destination_path, destination_observation }`
     (`document.rs:70`). The destination token is `observe_path`'s "in the
     document's context";
   - `TicketCreateInput { root, id, draft: TicketDraft, deps: Vec<String>, parent: Option<String>, initials: Option<String> }`
     (`ticket.rs:44`);
   - `TicketSaveInput { root, id, draft, deps: RelationshipWrite<Vec<String>>, parent: RelationshipWrite<String>, observation: String }`
     (`ticket.rs:58`);
   - `TicketSlugAssignInput { root, id, observation: String, initials: Option<String> }`
     (`ticket.rs:948`);
   - `FolderCreateInput { root, path: String }` (`folder.rs:58`);
   - `DocumentRepairInput` and `TicketRepairInput`, each
     `{ root, path: PathBuf, observation: String, source: String }`
     (`repair.rs:100`, `:114`). The observation is "the token `show_path`
     gave for the file on primary";
   - `IndexRefreshInput { root }` and `IndexRebuildInput { root }`
     (`index.rs:36`, `:43`).
3. `DocumentDraft { title, body }` and
   `TicketDraft { title, ticket_type, status, project: Option<String>, team: Option<String>, body }`
   (`src/repository.rs:579`, `:585`). `RelationshipWrite` is
   `Unchanged | Clear | Set(T)`, default `Unchanged` (`:613`). The library
   takes a complete draft. F2's design leaves the save patch to the CLI.
4. Of C3's mutations, only `DocumentRepair` and `TicketRepair` need
   confirmation (`needs_confirmation`, `src/repository/mutation/command.rs`
   around line 436).
5. The save bindings read the item through `save::read_item`
   (`src/repository/mutation/bind/save.rs:89`). It answers `item_not_found`
   for an item of the other kind, one without a source or observation, one
   whose source does not parse, and one whose parsed ID differs.
6. `observe_path(&self, repo, context: Option<&Path>, path: &Path) -> Result<PathObservationDto, ReadError>`
   (`src/repository/read/items.rs:1963`). The context must be the repository
   root or an item context the index holds as active (`named_context`).
   Otherwise it is `invalid_path`. The path must be a canonical item path
   (`docs/**/*.md` or `.manyhands/tickets/<id>/ticket.md`) or a listed
   nonconforming entry (`readable_path`). Another item's canonical path in
   that context is allowed. `PathObservationDto { path, exists, observation, context }`
   is in `src/repository/read/dto.rs:20`.
7. `check_ticket_relationships(&self, repo, id: &ItemId, proposed: &ProposedRelationships) -> Result<RelationshipCheckDto, ReadError>`
   (`items.rs:2033`), with `ProposedRelationships { deps: Vec<ItemId>, parent: Option<ItemId> }`
   (`items.rs:113`). It answers from the index and does not scan.
8. `show_item` (`items.rs:1784`) returns `ItemDto`
   (`src/repository/read/dto.rs:426`). Its `title`, `type`, `status`,
   `project`, `team` and `body` are each `Option<String>`.
9. A body round-trips byte for byte. `split_front_matter`
   (`src/canonical.rs:1026`) returns everything after the closing `---`
   line, and `serialize_item` (`:911`) writes `---\n{yaml}---\n{body}`.
10. *(Task 0)* An index binding answers with no `data`, `effects.discovery`
    `current` on success. `IndexRebuild` restores an unavailable index
    before its binding runs.
11. *(Task 0)* A save refused with `identity_required` deletes its request
    record and leaves the editing context's worktree with no commit. The same
    request ID succeeds once an identity is set
    (`tests/mutation_replay/execute.rs` around line 1100).
12. The failure hook is per service and in process:
    `RepositoryService::open_at_with_failure_point_for_testing`
    (`src/repository.rs:4490`), wrapped by `FailOnce`
    (`tests/support/mod.rs:277`). A CLI process cannot be given one.
13. Result-code classes (`src/results.rs`):
    - Input (exit 2): `relationship_cycle`, `invalid_relationship`,
      `occupied_path`, `slug_already_assigned`, `not_repairable`,
      `item_not_found`, `path_not_found`;
    - Blocked (exit 3): `external_change`, `identity_required`,
      `initials_required`, `invalid_slug_configuration`,
      `identity_ambiguous`, `ticket_closed`, `index_unavailable`;
    - Incomplete (exit 4): `discovery_pending`.

**CLI.**

14. The thirteen C3 verbs are `planned(…, "C3")` at
    `src/bin/manyhands-cli/catalog.rs:662–679`. `planned()` (`:209`) gives
    them no repository rule and no options. `available()` (`:191`) is fixed
    to `"C1"` and `Kind::Read`, so C2's entries are literal `CommandSpec`s.
    `prompts()` is true for every `Kind::Mutation`.
15. `parse::parse` replaces clap's error text only for verbs delivered by C2
    (`parse.rs:120`). "--context requires --path." is at `parse.rs:206`.
16. `mutate::run` (`mutate.rs:120`), `mutate::refusal` (`:273`), the
    `Boundary` trait (`:40`) and `RETRY_DELAYS_MS` (`:73`) are C2's driver.
    The `build` closure is called on every attempt, including bounded
    retries.
17. `InputObject` offers `only`, `string`, `optional_string`,
    `nullable_string`, `tristate`, `optional_object` and `one_of`
    (`input.rs:110–181`), with no accessor for a JSON array.
18. In `dispatch`, an available verb with no arm answers `internal_error`
    (`dispatch.rs:678–688`). The read `show` answers `item_not_found` for an
    item of the other kind (`dispatch.rs:330`).
19. `human_admin::mutation_success` prints every field of the
    `MutationDataDto` (`human_admin.rs:139`). `preview` already formats
    `PreviewDetailsDto::Repair` (`:62`).
20. C2's exit table is pinned to the audit section "C2 exit-coverage union"
    and to its counts (`tests/cli/exits.rs:601`). C2's no-op and privacy
    inventories filter on `delivered_by == "C2"` (`tests/cli/human.rs:50`,
    `tests/cli/privacy.rs:413`). C1's dispatch and boundary inventories
    filter on C1.
21. The native workflow already runs `--bin manyhands-cli --test cli --test cli_remote`
    (`.github/workflows/build.yml:115`).
22. The pre-existing CLI tests that name a C3 verb are the `DESIGN` table in
    `catalog_tests.rs:64`, which keeps its `"C3"` row, and three help goldens:
    `help_root.json`, `help_root_text.txt` and `help_resource_ticket.json`.
    Tests that iterate planned verbs (`parse_tests.rs:216`,
    `startup_tests.rs:332`, `:371`) still pass over C4 and C5.

**This repository (for the dogfooding step).**

23. The open Wave 03 Cycle tickets have no `deps`, `parent` or `slug` key:
    - C4 `01M4CC0VMWEQQ9G4W0DAG7ADPY`;
    - C5 `01M4CC0VMXTNXNH78C11DYXS8H`;
    - D3 `01M4CC0VN0MFTGKQ4HGTTC5H63`;
    - D4 `01M4CC0VN1KTQFSVR99378TDHV`;
    - D5 `01M4CC0VN2K8DBHNMZ46NTX5JJ`;
    - D6 `01M4CC0VN3VPVVQ3RNWJKXWEVX`;
    - G1 `01M4CC0VN4H9AC0Q3KA6E1QERC`;
    - the gate ticket `01M57SKVF6TQ3QM6AXJ35JGGSY`.

    Each lives only on its own branch and worktree.
24. On 2026-10-24 the G1 worktree held an uncommitted edit of its title
    (appending " External Edit") and an untracked `ticket.md~`. Every other
    worktree in fact 23 was clean.

## Module Layout

New bin-private modules, declared in `src/bin/manyhands-cli.rs` with
`#[path = "manyhands-cli/<name>.rs"] mod <name>;`, each with a
`*_tests.rs` beside it:

- `authoring.rs`: per-verb glue for the eleven mutations, built as
  `admin.rs` is. It reuses `admin::flags`, `admin::input_object`,
  `admin::mutation_root` and C2's input-failure refusal.
- `fill.rs`: the save patch (R-4). It holds a pure function from
  "the input's fields plus the item read" to a draft, unit-tested without a
  repository, and a thin wrapper that does the read.
- `human_authoring.rs`: the human text for `path observe` and
  `ticket check`.

Changed:

- `catalog.rs`: thirteen literal `CommandSpec`s (R-1).
- `parse.rs`: the usage-text rule (R-2) and C3's selector rules.
- `input.rs`: list readers (R-3).
- `dispatch.rs`: one arm per verb.

`admin.rs` changes only by making its private helpers `input_failure` and `name`
`pub(crate)`.

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
input schema, example and goldens. Until then it stays
`planned(…, "C3")` and answers `command_unavailable`. All thirteen require
`--repo`, so `catalog_repository_rules_and_planned_metadata_match_the_design`
holds without an edit. The mutations carry C2's mutation options
(`--input` where required, `--request-id`, `--prepare`, `--confirm`), and
those with `--id` add C1's `ID`.

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
  so that a valid request ID is retained.
- `--repo` for a mutation is joined to the invocation directory as C2 does
  (`admin::mutation_root`). The library refuses a linked worktree's root
  with `not_repository_root`.
- `--path` on `path observe` is repository-relative. `--context` is a
  worktree path, relative to the invocation directory, passed to the library
  as `document show --path --context` passes it.
- `--deps` on `ticket check` takes one value: a comma-separated list of
  canonical ULIDs. `--parent` takes one ULID. A malformed or empty element is
  `invalid_id` (R-6).

## Input

Reading, BOM handling, duplicate-key refusal and error details are C2's
(`input::read`, `input::parse`), unchanged. Content validation stays the
library's. The CLI checks shape, and it parses `item_id` because the library
input is typed `ItemId`.

**List readers (R-3).** `InputObject` gains two readers:

- `optional_string_list(field) -> Result<Option<Vec<String>>, InputError>`;
- `nullable_string_list(field) -> Result<ListChange, InputError>`, where
  `ListChange` is `Missing | Null | Value(Vec<String>)`.

Both refuse a non-array or a non-string element as a wrong type. Neither
de-duplicates or sorts, since that is the library's canonical form.

**Fields.** "Optional" means the key may be missing. "Nullable" means `null`
is a value with its own meaning (R-5).

| Verb | Fields |
| --- | --- |
| `folder create` | `path`: string, required. |
| `document create` | `item_id`: string, required, a canonical ULID. `path`, `title`, `body`: strings, required. |
| `document save` | `observation`: string, required. `title`, `body`: strings, optional. |
| `document move` | `observation`, `destination_path`, `destination_observation`: strings, required. |
| `document repair`, `ticket repair` | `path`, `observation`, `source`: strings, required. |
| `ticket create` | `item_id`: string, required, a canonical ULID. `title`, `type`, `status`: strings, required. `project`, `team`, `body`: strings, optional; a missing `body` is empty. `deps`: array of strings, optional. `parent`: string, optional. `initials`: string, optional. |
| `ticket save` | `observation`: string, required. `title`, `type`, `status`, `body`: strings, optional. `project`, `team`: string or `null`, optional; `null` clears. `deps`: array of strings or `null`, optional; missing leaves them as they are, `null` or `[]` clears them. `parent`: string or `null`, optional; `null` clears it. |
| `ticket slug-assign` | `observation`: string, required. `initials`: string, optional. |

`item_id` that is not a canonical ULID is `invalid_input`, naming the field.
An input selector stays `invalid_id`. `deps` and `parent` text is passed
through, and the library refuses a malformed entry with
`invalid_relationship`. No input field may carry a closure field or `slug`.
`closed_at`, `closed_by`, `slug` and every other unknown key are refused as
unknown.

**Schemas.** One closed schema per input-taking verb in `schemas/input/v1/`,
named `<resource>_<verb>.schema.json` with hyphens as underscores: nine files,
`folder_create` to `ticket_slug_assign`. `index_refresh` and `index_rebuild`
take no input and have none. Nullable fields use `"type": ["string", "null"]`
or `["array", "null"]`, and lists use `items`. The checker supports both
(`tests/support/schema.rs:93`, `:237`), and C2's `remote_select.schema.json`
already uses a type list. One example input per schema is in
`tests/fixtures/cli_input_v1/<stem>.json` (R-13).

## The Save Patch (R-4, Cycle Decision 2)

`fill.rs` builds the draft that `document save` and `ticket save` give the
library.

1. **When.** A read happens only when at least one draft field is omitted:
   - for documents: `title` or `body`;
   - for tickets: `title`, `type`, `status`, `body`, `project` or `team`.

   With every draft field present, no read happens and the draft is the
   input's.
2. **The read.** The CLI calls
   `service.resolve_repository(root)`, then `service.show_item(&repo, &id)`.
   Each happens once per invocation, before `mutate::run`, so bounded retries
   reuse the same draft.
3. **Filling.** Each omitted field takes the item's value:
   - `title`, `type`, `status` and `body` must be present in the read;
   - an omitted `project` or `team` takes the read's value, which may be
     absent;
   - a `null` `project` or `team` is `None`;
   - `deps` and `parent` are never filled. Missing is
     `RelationshipWrite::Unchanged`, which the library carries through as
     the file has it.
4. **Refusals.** The CLI answers `item_not_found` in these cases, matching
   `save::read_item`:
   - a read of an item of the other kind;
   - a read without a field the draft needs (a nonconforming item has none).

   A read that fails passes its code, message, scope and recovery actions
   through under the mutation's command name. The CLI keeps the parsed or
   allocated request ID, as C2 does for refusals after opening, and sets every
   effect to `not_requested`.
5. **The observation.** The caller's `observation` goes to the library
   unchanged. The CLI never compares it with the read's token. The library
   alone decides staleness, no-op, replay and mismatch.

**Required outcomes.** Each row has a test in Task 3 or Task 5.

| Sequence | Outcome |
| --- | --- |
| Save with only `body`, the observation current | One checkpoint; the other fields and every unknown key byte-identical. |
| Save with every draft field omitted | `noop`, no commit, the file byte-identical. |
| A third party changed `title`; the caller saves only `body` with the old token | `external_change`, exit 3; nothing written. |
| The output of a successful save is lost; same request, same input | The stored result is replayed; one commit. |
| As the row above, but a third party changed an omitted field after the save | `request_mismatch`; one commit; `request show` returns the stored result. This is the replay limit of decision 2. |
| `ticket save --id <a document's ID>` with a field omitted | `item_not_found`, exit 2, no request record. |
| `project: null` | The `project` key is removed; nothing else changes. |
| `deps: []` and `deps: null` | Both remove the `deps` key. |
| `deps` omitted | The `deps` key is byte-identical. |

Why the CLI does not compare tokens: a retry after a lost result reads the
file the request itself wrote. A comparison would turn that retry into an
`external_change` instead of a replay.

## Reads

**`path observe` (R-7).** `observe_path(&repo, context, path)`, as
`document show --path` resolves its repository. The data is
`PathObservationDto` (schema `path_observation`). The CLI adds no rule: a
canonical path that another item holds in the named context is reported as
what is there. Human output, one escaped field per line:

```text
path: docs/plans/x.md
exists: no
observation: <token>
context: <worktree> (<branch>)
```

Its uses:

- the absent token that `document move` needs for its destination, observed
  in the document's context (the worktree `document show --id` reports, or
  the root when it has none);
- the token of a nonconforming file on primary that repair needs. That is the
  same token `document show --path` gives.

**`ticket check` (R-6).**
`check_ticket_relationships(&repo, &id, &ProposedRelationships { deps, parent })`.
The data is `RelationshipCheckDto` (schema `relationship_check`).

- A proposed cycle is reported in `data.rejection`. It is still a read
  success: `ok`, exit 0.
- Human output lists the proposed deps and parent with their resolution and
  the unresolved IDs. When `rejection` is set, it ends with the line
  `rejected: <code>: <id> <id> …`.
- When `complete` is false or the index is not current, the note C1 writes
  for incomplete reads goes to stderr.

## Index Verbs (R-8)

`index refresh` and `index rebuild` build
`Mutation::IndexRefresh(IndexRefreshInput { root })` and
`Mutation::IndexRebuild(IndexRebuildInput { root })` through `mutate::run`,
with `Plan { identity_prompt: false, retry: true }`.

- They take no input. JSON and non-interactive calls need `--request-id`,
  like every F2 mutation.
- Human success is `mutation_success`: the envelope message and the effects
  line. The data is null.
- The recovery actions `index.refresh` and `index.rebuild` that C1 reads
  offer now name available commands. A caller following one supplies a
  request ID.

## Repair

`document repair` and `ticket repair` are C2's confirmed flow, unchanged:

- `--prepare` returns the repair preview (schema `repair_preview`), which
  states whether an ID is adopted;
- `--confirm <id> --request-id <id>` executes it;
- an interactive human invocation shows the preview on the terminal and asks
  `Proceed? [y/N]`.

The corrected `source` is never echoed. The preview and result carry `id`,
`path`, `adoption` or `adopted`, and `primary_changed`. A repaired document is
listed twice until its context merges, as F2 recorded.

## Identity Recovery (R-9)

No C3 verb prompts for identity: `Plan { identity_prompt: false }`
throughout.

- A save, create, move, slug assignment or repair with no usable identity
  answers the library's `identity_required`, exit 3, with the
  `repo.identity_set` recovery action. Human stderr lists that action, as for
  any recovery.
- After `repo identity-set`, the same command with the same request ID
  succeeds. F2 deletes the record of a request refused this way (fact 11).
- `initials_required` on `ticket create` is the same pattern. Its remedies
  are `initials` in the input or `repo identity-set` with `initials`.

## Human Output

- Mutations use C2's `mutation_success`, `failure_extra` and `preview`,
  unchanged. Their data DTOs are content-free:
  - `DocumentMutationDto { id, path }`;
  - `TicketMutationDto { id, path, slug, unresolved, rejected }`;
  - `RepairMutationDto { id, path, adopted }`.
- `path observe` and `ticket check` get formatters in `human_authoring.rs`,
  escaping each value with C1's field helper.
- Progress stages go to stderr as in C2.

## Exit Statuses

Unchanged: C1's class mapping (`render::exit_status`). The codes each verb
can return are Task 0's audit refresh. Notable ones:

| Code | Exit |
| --- | --- |
| `relationship_cycle`, `occupied_path`, `slug_already_assigned` | 2 |
| `external_change`, `identity_required`, `ticket_closed` | 3 |
| `discovery_pending` | 4 |

## Dogfooding (R-16, Cycle Decision 3)

This runs in the final task, after the full gate, and only with the owner's
go-ahead at that step. The go-ahead names the repository
(`/home/vhodges/work/src/manyhands`), the default application data
directory, the six saves and the six branch pushes.

1. **Preflight, read-only.** For each target ticket in decision 3, check that
   its worktree is clean with `git -C <worktree> status --short`. A worktree
   that is not clean stops the step for the owner: at planning, G1's was not
   (fact 24). Nothing is reverted or committed by hand.
2. **Build.** `devenv shell -- cargo build --locked --bin manyhands-cli` in
   the C3 worktree. Every following command is
   `<worktree>/target/debug/manyhands-cli --json` with
   `--repo /home/vhodges/work/src/manyhands`.
3. **Refresh.** `index refresh` with a request ID from `id new`.
4. **Record.** For each target ticket:
   - `ticket show --id <id>` for its observation;
   - `ticket save --id <id> --request-id <new> --input -` with only
     `observation` and `deps`. Every draft field is omitted, so the save
     patch fills them.

   Each save commits one checkpoint on that ticket's own branch.
5. **Check.** `ticket blocked` lists each target with its recorded blockers.
   `git -C <worktree> log -1 --format=%H` names each checkpoint, and
   `git -C <worktree> diff HEAD~1 -- <ticket.md>` shows only the `deps` key
   added.
6. **Push** each changed ticket branch with
   `git push origin manyhands/ticket/<id>`. Ticket branches are pushed by
   standing habit, and the go-ahead names these pushes.
7. **Switch.**
   - Amend AGENTS.md's Dogfooding Workflow: ticket create and update use
     `manyhands-cli ticket create` and `ticket save` against the main
     checkout; comments stay canonical files until C4.
   - Rewrite `.agents/skills/file-ticket/SKILL.md` around `id new`,
     `ticket create`, `ticket show` and `ticket save`.

Any answer other than success, or a no-op for a ticket that already has
those deps, stops the sequence:

- it is recorded in the ledger and reported to the owner;
- nothing is worked around with Git or a hand edit;
- the switch in step 7 is not made until every save has succeeded.

The library writes the checkpoint commits with its own message, so they carry
no `Co-Authored-By` trailer.

## Testing

All new tests are modules of the existing `cli` target (R-15), so the native
workflow's list needs no change. Each verb task adds its own files:

| File | Covers |
| --- | --- |
| `tests/cli/authoring_reads.rs` | `path observe` (absent and present paths, a named context, another item's file in a context, an inactive context's `invalid_path`), `ticket check` (resolved, unresolved, cycle rejection, malformed `--deps`). |
| `tests/cli/index.rs` | Refresh and rebuild; index loss and whole-file corruption (R-12); nothing canonical changes. |
| `tests/cli/folders.rs` | Create, no-op, occupied path, path outside `docs/`. |
| `tests/cli/documents.rs` | Create, save (the save-patch table), move (collision, stale destination, no-op), offline checkpoints, stale writes. |
| `tests/cli/ticket_authoring.rs` | Create with deps, parent and initials; save (the save-patch table, relationships); slug-assign; cycles; unresolved targets; a closed ticket; `slug_already_assigned`; identity and initials recovery (R-18). |
| `tests/cli/repair.rs` | Prepare/confirm, interactive consent through the pseudo-terminal (Unix), adoption, adoption retry (R-11), invalid and malformed repair, `identity_ambiguous`, a closed ticket. |
| `tests/cli/authoring_exits.rs` | The C3 exit table (R-10), the no-op inventory (R-14), privacy (R-17), human mode and the examples (R-13). |

Fixtures reuse:

- `tests/support/cli.rs`: `CliEnv`, `run_cli`, `run_cli_with_stdin`,
  `assert_cli_golden`;
- `tests/support/mod.rs`: `enabled_repository`;
- `tests/support/items.rs`: `contract_repository`, `relationship_repository`,
  `ticket_source`, `document_source`;
- `FailOnce`.

**Library-level actors (R-11).** For a case the binary cannot produce, the
test opens `FailOnce::at(point).open_service(<the CLI's data directory>)`,
runs the library call with the request ID and input the CLI will then send,
and asserts the partial result. It then runs the CLI with the same request
and input, and asserts completion and the commit count. There are two cases:

- commit success, refresh failure: a `document save` at
  `BeforeIndexTransactionCommit`;
- adoption retry: a `document repair` at the fault point that F2's
  `a_repair_stopped_after_its_write_is_completed_by_its_retry_with_the_same_id`
  uses. Task 0 reads which it is.

**Index loss and corruption (R-12).** Both cases run in a repository holding
one document and one ticket:

- remove the database file under `MANYHANDS_DATA_DIR`;
- overwrite it with 4 KiB of non-SQLite bytes.

Each runs `index rebuild --repo` and asserts `ok`. `document list` and
`ticket list` then list both items. A recursive byte comparison of the
worktree and `git for-each-ref` output are equal before and after. Task 0
reads the database file's name.

**Goldens.** As C2: `tests/fixtures/cli_v1/`, registered in `CLI_CASES` or
`CLI_TEXT_CASES`, and updated with `MANYHANDS_UPDATE_GOLDEN=1`, never under
`CI`. Every regenerated golden's diff is read before commit. Each verb gets a
JSON success golden and at least one refusal golden. The two repairs also get
a preview golden. Human text goldens cover the two reads and one mutation
success.

**Isolation (R-18).** As C2:

- children start through `cli_command`, without a controlling terminal;
- the pseudo-terminal helper is used only for the repair consent cases;
- no test touches the developer's keys, global Git configuration or
  application data.

The identity-required cases need a machine whose system Git configuration
sets no identity (C2 R-17). Elsewhere they fail with a message saying so.

**Native.** Linux locally. The `cli` target is already in the native list.
Pseudo-terminal cases are `#[cfg(unix)]`. Native CI runs on main after merge.

## Named Edits To Pre-Existing Tests

| Edit | Task | File and change |
| --- | --- | --- |
| E1 | 1–6 | The help goldens `tests/fixtures/cli_v1/help_root.json`, `help_root_text.txt` and `help_resource_ticket.json` are regenerated as each C3 verb becomes available. The diff of each may change only that verb's availability, delivering Cycle, repository rule and options. |

No other pre-existing test changes (fact 22). Task 0 proves it again with the
grep in the plan. A hit outside E1 stops for an owner ruling. New tests are
additions, including the C3 counterparts of C2's usage-privacy, no-op, privacy
and exit inventories.

## Document Amendments

Made in Task 8, except where noted:

- **CLI contract RFC** (`docs/RFC/cli-contract.md`), "Amended 2026-10-24 by
  C3":
  - the taxonomy rows of the thirteen verbs, with selectors and input fields;
  - the save patch and its replay limit;
  - `path observe` and `ticket check` grammar (R-6, R-7);
  - index verbs needing `--request-id`;
  - identity recovery without a prompt;
  - the input-schema table extended;
  - "C3 verbs are available; C4–C5 remain planned".
- **Relationships RFC**: `ticket check`'s flags, and `deps: []` or `null`
  clearing, restated as the CLI's input.
- **Wave 03**: C3 section, "Amended 2026-10-24 by C3". It records:
  - decisions 1–4;
  - `ticket check` and `path observe` as delivered;
  - the known limits.
- **API audit**: the C3 refresh section (Task 0), and the C3 rows updated in
  place.
- **AGENTS.md** and **`file-ticket` skill**: the dogfooding switch (Task 9,
  after the saves succeed).
- **The C3 ticket**: its Added Scope corrected, since initials and the prefix
  shipped in C2; its planning boxes ticked (Task 9).

## Rulings

Each ruling is listed with its cost if wrong.

- **R-1 Catalog.** Literal `Available("C3")` entries, flipped in the task
  that dispatches the verb. *Cost:* none beyond C2's pattern.
- **R-2 Usage privacy.** The condition at `parse.rs:120` becomes
  `spec.delivered_by() != "C1"`. C1 reads keep clap's text, which C1 goldens
  pin. *Cost:* C3's read verbs lose clap's detailed usage hint, though their
  values are not secret.
- **R-3 List readers.** Two accessors in `input.rs`, with no
  de-duplication. *Cost:* small.
- **R-4 The save patch.** The fill rules and refusals above. *Cost if wrong:*
  a retry after a third party changed an omitted field answers
  `request_mismatch` (decision 2). A wrong fill would change a field the
  caller did not name; the required-outcomes table guards it.
- **R-5 Optionality.** Where the CLI RFC is ambiguous:
  - `document save`'s `body` is optional, as `ticket save`'s is;
  - `ticket create`'s `body` defaults to empty;
  - `deps: []` clears, as the relationships RFC says.

  *Cost:* narrowing an optional field later is a new input version.
- **R-6 `ticket check`.**
  - `--deps` is comma-separated, because `OptionSpec` has no repeatable
    form.
  - At least one of `--deps` and `--parent` is required.
  - A rejection is a read success, exit 0, reported in `data.rejection`.

  *Cost:* a script must inspect `rejection`, not the exit code. Changing the
  exit later changes a published outcome.
- **R-7 `path observe`.** Flags as `show --path --context`, with no CLI
  rule about whose file is at the path (multi-item note). *Cost:* none.
- **R-8 Index verbs.** Unconfirmed mutations with no input, needing
  `--request-id` in JSON. *Cost:* agents following `index.rebuild` must
  allocate an ID.
- **R-9 Identity.** A pass-through, with no prompt. *Cost:* an interactive
  user runs `repo identity-set` and repeats the command.
- **R-10 Exit table.** `tests/cli/authoring_exits.rs`, pinned to the audit
  section "C3 exit-coverage union" by set equality. C2's table is
  untouched. *Cost:* two tables to keep.
- **R-11 Library-level actors.** For interrupted cases, as C2's key-deletion
  retry. *Cost:* the interruption is not produced by the binary itself.
- **R-12 Corruption evidence.** Only deletion and whole-file replacement.
  The partial form is `01M4PB82`'s (decision 1). *Cost:* that form remains
  unrecoverable from the CLI.
- **R-13 Documented examples.** The example inputs are the documented
  examples. `every_c3_example_input_runs` runs each against a fixture and
  checks the outcome the CLI RFC's amendment gives for it. *Cost:* the
  examples are files, not prose in the RFC, which links to them.
- **R-14 No-op inventory.** A no-op is a fresh request that writes nothing.
  Task 0 lists which of the eleven mutations can be one, from their bindings.
  Each is given a case, and each that cannot gets a reason. *Cost:* none.
- **R-15 Test layout.** New modules in the `cli` target. *Cost:* the target
  grows. A new target would need a CI list change.
- **R-16 Dogfooding.** The procedure above, with its stop rules. *Cost:*
  real-repository effects. These are bounded to six ticket branches and
  gated by the owner's go-ahead.
- **R-17 Privacy.** A sentinel body, title and source run through every
  input-taking C3 verb in both modes, including a refusal after the fill
  read. The sentinel appears in no stdout, stderr or file under the data
  directory. It does appear in `document show` / `ticket show` output, which
  are reads of the source. *Cost:* none.
- **R-18 Identity machine.** The identity cases need no system identity
  (C2 R-17). Elsewhere they fail with an explanatory message, not a skip.
  *Cost:* a developer machine with `/etc/gitconfig` identity cannot run
  them.

## Known Limits

- Decision 1's three defects, each on its ticket.
- Decision 2's replay limit.
- A repaired document is listed twice until its context merges (F2).
- An empty folder is invisible to Git; `folder list` lists it after the
  refresh `folder create` makes (F2).
- `identity_required` leaves an editing context's worktree with no commit,
  which the next successful save reuses (F2, fact 11).
- Item reads answer `item_not_found` for an item whose context is
  duplicated or mismatched (ticket `01M5B163PQTE9C1T6NMHATR3KX`). The save
  patch inherits that answer.

## Alternatives Rejected

- **Complete drafts on save.** Exact replay, but it departs from the CLI RFC
  and makes every edit name every field (decision 2).
- **A library patch.** A prerequisite ticket, contradicting F2's split
  (decision 2).
- **`body_path` or `--body-file`.** It would be a second body channel. The
  CLI RFC carries bodies only as JSON strings, and `jq --rawfile` builds the
  input.
- **A repeatable `--dep`.** `OptionSpec` has no repeatable form, and adding
  one changes every option constant for one verb.
- **Exit 3 for a `ticket check` rejection.** That would make a read fail
  for reporting what it was asked (R-6).
- **A separate test target.** It would need a CI list change and gives
  nothing a module does not.

## Risks

| Risk | Control |
| --- | --- |
| The fill changes a field the caller did not name. | The required-outcomes table, byte comparisons, and unknown keys preserved. |
| A body or source reaches a diagnostic. | R-17 sentinel test over every verb and mode; `fields()` sees only content-free DTOs. |
| A verb is advertised before it works. | R-1: availability flips in the dispatching task, and `internal_error` for an unrouted verb is caught by its tests. |
| Help goldens drift beyond the verb's own lines. | E1's diff rule, read before commit. |
| Feature-dependent failures surface only at the end (C2's `preserve_order`). | Part gates include `--all-features`. |
| The full suite outlasts a tool timeout. | It runs detached, with a two-hour budget (C2 ledger). |
| Dogfooding commits a stray edit. | Preflight stops on an unclean worktree (fact 24). |
| A library gap appears mid-task. | Stop; a separate ticket under track rule 5. |
