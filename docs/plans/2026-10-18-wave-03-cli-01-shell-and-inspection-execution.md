---
title: "Wave 03 C1 CLI Shell And Inspection Execution"
date: 2026-10-19
status: in-progress
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M59YW21X1BF2KEMKJQ252S19"
---

# Wave 03 C1 CLI Shell And Inspection Execution

## Baseline

Task 0 only, executed on 2026-10-19 under the approved
[Cycle](../Cycles/wave-03-cli-01-shell-and-inspection.md),
[design](2026-10-18-wave-03-cli-01-shell-and-inspection-design.md),
[implementation plan](2026-10-18-wave-03-cli-01-shell-and-inspection-implementation.md)
and owner-approved [handoff](2026-10-19-wave-03-cli-01-handoff.md).
The owner authorized implementation and task-checkpoint commits. The controller
dispatched this bounded task; the implementer did not dispatch subagents.

| Item | Evidence |
| --- | --- |
| Ticket | `01M4CC0VMSZ7BRS740C8VSQ5ZK`; remains open |
| Worktree | `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMSZ7BRS740C8VSQ5ZK` |
| Branch | `manyhands/ticket/01M4CC0VMSZ7BRS740C8VSQ5ZK` |
| Initial/task base HEAD | `de28ab0512fefee8e0b0e987c9d7a98a3ab0ef20` |
| `origin/main` | `f6a4c06300f4be80eb18f843a9e4fcbde847e47e` |
| Initial status | Clean; required branch tracks its matching origin branch |
| Main synchronization | Controller already fetched and merged; expected refs confirmed; no further merge or rebase needed |
| `git merge-tree --write-tree HEAD origin/main` | Exit 0, tree `ac5b8cb0f4d75ede9772680b16f01f16c2120726`, identical to initial `HEAD^{tree}` |
| Baseline source drift | `git diff --name-only f6a4c06300f4be80eb18f843a9e4fcbde847e47e HEAD -- src tests Cargo.toml Cargo.lock schemas` prints nothing |

### Baseline verification

All commands ran in this ticket worktree through Devenv. No full-suite run,
desktop smoke test or native workflow was dispatched. The full-suite gate is
reserved for Task 8.

| Command | Result / counts |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | PASS, exit 0. Initial cold dependency build was terminated by the tool's 120-second timeout; identical command retried with a 600-second limit completed in 15.47 seconds. No compiler error was reported. |
| `devenv shell -- cargo fmt --check` | PASS, exit 0 |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | PASS, exit 0, 1m 33s |
| `devenv shell -- cargo test --locked --test read_contract --test mutation_contract --test read_boundary` | PASS, exit 0: `mutation_contract` **43**, `read_boundary` **20**, `read_contract` **71**; **134 passed**, 0 failed/ignored/measured/filtered |
| `devenv shell -- cargo test --locked --lib results::tests` | PASS, exit 0: **49 passed**, 0 failed/ignored/measured, **1147 filtered out** |

Total targeted tests: **183 passed, 0 failed, 0 ignored**. These results prove
the implementation baseline, not any future CLI dispatch. Counts include the
existing `read_boundary::index_lease_child` helper test.

## Confirmed Facts

Each row corresponds to a bullet in the design's **Baseline Facts**, in order.
Paths and line numbers below refer to the task base, not future task commits.

| Fact | Recheck and disposition |
| --- | --- |
| 1. Seven-line headless entry point | **Confirmed.** `src/bin/manyhands-cli.rs:1–7` initializes transport first, emits exactly `Git transport initialization failed` on stderr and exits 1 on failure; otherwise silent success. No argument read. |
| 2. Cargo bin/dependencies | **Confirmed.** `Cargo.toml:7` has `autobins = false`; `:42–44` declares the CLI without a required feature; no argument parser dependency. `serde_json` is direct at `:23`. Package version is `0.1.0` (`:3`), also confirming the design's version-golden baseline. |
| 3. Public F1 reads and DTOs | **Changed: blanket return-type claim corrected; API unchanged.** All dispatch methods are public `RepositoryService` methods. The 26 fallible methods return `Result<Dto, ReadError>`, but `read/mod.rs:466` is `new_item_id(&self) -> NewIdDto` directly. Task 2 Step 3 already specifies `service.new_item_id()` and needs no changed step; do not add `?`/`unwrap` to that call. DTO fields are public and serialize without `Deserialize`; `ProblemDto` and `IndexProblemDto` implement `Serialize` manually for fixed guidance. |
| 4. Service open/error conversion | **Confirmed.** `src/repository.rs:1257–1263`: `open_default() -> Result<Self, RepositoryError>` and `open_at(data_directory: &Path) -> Result<Self, RepositoryError>`. `read/mod.rs:223–238` implements `From<RepositoryError> for ReadError`. |
| 5. Envelope/class boundary | **Confirmed.** `src/results.rs:113`, `:125`, `:282` implement `read_success`, `failure`, `failure_class`; `read/mod.rs:170` implements `to_envelope<T>`. `failure_class` returns `Option<FailureClass>`. Source search found no exit-number mapper; that remains Task 2. |
| 6. Result-code registry/exhaustive recovery | **Confirmed.** `result_codes!` at `src/results.rs:534`; `src/repository/mutation/outcome.rs:719–939` matches `code` exhaustively without a wildcard. The existing no-action arm starts at `:843`. |
| 7. Code lists/messages/schema enums | **Confirmed.** `src/results_tests.rs:33–102` has **68** `DESIGN_CODES` entries; `result_code_messages_are_fixed` starts at `:151`. `tests/read_contract.rs:712–716` and `:943–947` tie envelope `code` and request `result.code` to `ResultCode::ALL`. Targeted tests passed unchanged. |
| 8. Canonical ID parsers | **Confirmed.** `canonical.rs:30–50` implements `ItemId::from_str`, checking 26 uppercase Crockford characters then ULID parsing. `keys/mod.rs:39–44` and `repository.rs:284–289` round-trip `SharedKeyId`/`OperationId` to reject noncanonical spelling. No conversion from their parse errors to `ReadError`; `read/mod.rs:139–141` provides `invalid_id()`. |
| 9. SSH authority | **Confirmed.** `transport/mod.rs:73–76` has public `host: String`, `port: u16`; no authority parser method/trait implementation. `read/credentials.rs:116–133,259–269` normalizes host/address through `stored_authority` in `inspect_host`. IPv6 is held without brackets. |
| 10. Golden separation | **Confirmed.** `tests/support/golden.rs:26,87–89,129–132` registers/refuses cases and fixes `read_v1`. `mutation_golden.rs:182–184,191–224` owns `mutation_v1`, checks its own cases, and reuses `golden::render`/`golden::update_requested`; CLI can follow that pattern. |
| 11. Five shared contract gates | **Confirmed.** All named definitions/line anchors are listed below. `golden_envelopes` scans only read/mutation goldens. `DATA_ROOTS` has **44** entries; `CODES_NO_MUTATION_GOLDEN_HAS` has **4**. `schema_enum` follows only `properties` and filters null; `count_enums` walks every object/array; `tests/support/schema.rs:99–162` rejects unsupported keywords including `const`. |
| 12. Child Git/data isolation | **Confirmed.** `tests/support/ssh_harness.rs:198–228` removes inherited Git/SSH/GCM variables and redirects home/data/config; `:31–43` redirects libgit2 search paths only inside the test child through its initializer. A production CLI child has no such search-path hook; R-5 remains applicable. |
| 13. Platform data directories | **Confirmed by dependency-source inspection.** Locked `directories 5.0.1`: `src/lin.rs:64–94` uses absolute `XDG_DATA_HOME` or home fallback; `src/mac.rs:65–97` uses home; `src/win.rs:67–99` uses known folders. `dirs-sys 0.4.1` reads `HOME` on Unix (`src/lib.rs:34`) and uses `SHGetKnownFolderPath` on Windows (`:154,181`). Windows ignores `LOCALAPPDATA`; R-3 stands. No native platform run claimed. |
| 14. Transport-start failure | **Confirmed.** `src/runtime.rs:24–35` covers the safety comment and initializer: only the two libgit2 timeout setters can return its error. The fake-startup evidence remains R-4. |
| 15. Resolution/index/open lock | **Confirmed.** `read/resolve.rs:228–240,268–282` resolves through registration lookup/read session. `read/mod.rs:430–451` takes the shared cache lock and requires a readable index. `read/status.rs:657–698` converts its own index-unavailable error to a DTO, but CLI resolution fails first on an already degraded index. `repository.rs:3896–3918` takes the cache **read** guard while opening/migrating, so service opening can return `busy` before dispatch. |

The direct `NewIdDto` return is a correction to an overbroad sentence, not an
API change since `f6a4c06` or a task-altering gap. No rechecked Baseline Fact
requires a controller ruling to change the approved task steps.

### Every named design source-line anchor

| Design reference | Actual baseline | Disposition |
| --- | --- | --- |
| `src/repository/mutation/outcome.rs:719` | `let mut actions = match code` | Confirmed |
| `tests/mutation_contract.rs:1331` | `golden_envelopes` | Confirmed |
| `tests/mutation_contract.rs:1457` | `DATA_ROOTS: [&str; 44]` | Confirmed |
| `tests/mutation_contract.rs:1566` | `every_schema_is_reachable_from_a_published_root` | Confirmed |
| `tests/mutation_contract.rs:1610` | `every_data_root_describes_the_data_of_a_golden_and_every_data_has_one_root` | Confirmed |
| `tests/mutation_contract.rs:1658` | `CODES_NO_MUTATION_GOLDEN_HAS: [(ResultCode, &str); 4]` | Confirmed |
| `tests/mutation_contract.rs:1679` | `every_result_code_has_a_mutation_golden_or_a_reason_none_has_it` | Confirmed |
| `tests/mutation_contract.rs:1693` | `every_result_code_has_a_golden_or_a_reason_no_fixture_can_produce_it` | Confirmed |
| `tests/read_contract.rs:703` | `contract_enumerations` | Confirmed |
| `tests/read_contract.rs:1013` | `published_enumerations_list_exactly_the_registered_values` | Confirmed |
| `src/runtime.rs:24–35` | Initializer including its last safety-comment line | Confirmed |
| `tests/read_boundary.rs:34–58` | `#[test] fn index_lease_child` and its body through the loop's closing brace | Changed range endpoint for a complete copy: include **line 59**, the function's closing brace. No behavior/task change. |

### Dispatch signatures

All 27 distinct methods named by the design Dispatch table were checked.
Every declaration is public, on `RepositoryService`. Signatures below preserve
receiver, argument ownership/reference types and result types. Source paths
are relative to `src/repository/read/`. Session/result-code details for each
of the **28 commands** are in its single updated API-audit row.

| Command(s) | Confirmed signature | Source |
| --- | --- | --- |
| `repo list` | `list_repositories(&self) -> Result<RepositoryListDto, ReadError>` | `admin.rs:210` |
| `repo inspect` | `inspect_repository(&self, path: &Path) -> Result<RepositoryInspectionDto, ReadError>` | `admin.rs:273` |
| `repo identity` | `repository_identity(&self, repo: &ResolvedRepository) -> Result<IdentityDto, ReadError>` | `admin.rs:325` |
| `remote list` | `list_remotes_redacted(&self, repo: &ResolvedRepository) -> Result<RemoteListDto, ReadError>` | `admin.rs:360` |
| `key list` | `list_keys(&self) -> Result<KeyListDto, ReadError>` | `credentials.rs:190` |
| `key show` | `show_key(&self, id: SharedKeyId) -> Result<KeyDto, ReadError>` | `credentials.rs:203` |
| `key public` | `public_key_text(&self, id: SharedKeyId) -> Result<PublicKeyDto, ReadError>` | `credentials.rs:218` |
| `host list` | `list_host_pins(&self) -> Result<HostPinListDto, ReadError>` | `credentials.rs:248` |
| `host inspect` | `inspect_host(&self, authority: &SshAuthority) -> Result<HostPinDto, ReadError>` | `credentials.rs:259` |
| `document list` | `list_documents(&self, repo: &ResolvedRepository) -> Result<ItemListDto, ReadError>` | `items.rs:1662` |
| `folder list` | `list_document_folders(&self, repo: &ResolvedRepository) -> Result<DocumentFolderListDto, ReadError>` | `items.rs:1672` |
| `document show`, `ticket show` by ID | `show_item(&self, repo: &ResolvedRepository, id: &ItemId) -> Result<ItemDto, ReadError>` | `items.rs:1784` |
| `document show`, `ticket show` by path | `show_path(&self, repo: &ResolvedRepository, context: Option<&Path>, path: &Path) -> Result<ItemDto, ReadError>` | `items.rs:1853` |
| `ticket list` | `list_tickets(&self, repo: &ResolvedRepository, filter: &TicketFilter) -> Result<ItemListDto, ReadError>` | `items.rs:1696` |
| `ticket ready`, `ticket blocked` | `ticket_readiness(&self, repo: &ResolvedRepository, filter: &TicketFilter) -> Result<ItemListDto, ReadError>` | `graph.rs:885` |
| `ticket deps` | `ticket_dependencies(&self, repo: &ResolvedRepository, id: &ItemId, direction: DependencyDirection, depth: Option<u32>) -> Result<DependencyTreeDto, ReadError>` | `graph.rs:913` |
| `ticket children` | `ticket_children(&self, repo: &ResolvedRepository, id: &ItemId) -> Result<ItemListDto, ReadError>` | `graph.rs:989` |
| `ticket cycles` | `ticket_cycles(&self, repo: &ResolvedRepository) -> Result<CycleListDto, ReadError>` | `graph.rs:1018` |
| `ticket plan` | `ticket_plan(&self, repo: &ResolvedRepository, filter: &TicketFilter) -> Result<PlanDto, ReadError>` | `graph.rs:1042` |
| `ticket critical-path` | `ticket_critical_path(&self, repo: &ResolvedRepository) -> Result<ItemListDto, ReadError>` | `graph.rs:1093` |
| `ticket find` | `find_tickets_by_slug(&self, repo: &ResolvedRepository, slug: &str) -> Result<ItemListDto, ReadError>` | `graph.rs:1109` |
| `comment list` | `list_comments(&self, repo: &ResolvedRepository, item: &ItemId) -> Result<CommentListDto, ReadError>` | `comments.rs:258` |
| `index status` | `index_status(&self, repo: &ResolvedRepository) -> Result<IndexStatusDto, ReadError>` | `status.rs:657` |
| `poll status` | `polling_status(&self, repo: &ResolvedRepository) -> Result<PollingStatusDto, ReadError>` | `status.rs:706` |
| `operation list` | `list_operations(&self, repo: &ResolvedRepository) -> Result<OperationListDto, ReadError>` | `status.rs:755` |
| `operation show` | `show_operation(&self, repo: &ResolvedRepository, id: OperationId) -> Result<OperationDto, ReadError>` | `status.rs:799` |
| `id new` | `new_item_id(&self) -> NewIdDto` | `mod.rs:466`; direct DTO exception |

The shared prerequisite is also confirmed:
`resolve_repository(&self, path: &Path) -> Result<ResolvedRepository, ReadError>`
at `resolve.rs:228`. `repo inspect` uses its own selection/registration lookup,
not this registration-required resolver. Key/operation IDs pass by value;
item IDs pass by reference; dependency depth is `Option<u32>`; by-path context
is `Option<&Path>`.

## Audit Refresh

Updated [the API audit](../research/wave-03-api-audit.md) with the required
**Wave 03 CLI Cycle C1 refresh (2026-10-18)** section. Existing C1 rows are
replaced in place; formerly absent `folder list` and eight relationship reads
are added once. Each records its F1 signature, read-session behavior, result
codes and current CLI-pending status. Invocation rows now distinguish the
current skeleton from C1's planned service-free help/version.

The inventory separates method failures from CLI repository resolution,
typed-ID parsing and service startup. This preserves the no-session nature of
identity/remotes/ID generation without incorrectly promising a CLI invocation
cannot be `busy`. It also preserves R-2's degraded-index behavior.

Recorded `path observe`/`ticket check` as C3, F2's three C2 commands and
`operation abandon` as C4, and the previously absent planned C3
`ticket slug-assign`. The refreshed inventory has one row per approved verb;
historical C0–C6 findings outside these rows retain their dated scope. No
production source, schema, lockfile or pre-existing test was changed.

## Planning Review

The Cycle review record is the authority for the earlier reviews:

- At **`2478f75`**, the independent review found **40 findings: 6 blocking,
  18 important, 16 minor**. These were resolved by **decisions 11–13**,
  **rulings R-1 through R-5**, and corrections in the design and plan.
- Decision 11 permits named test edits 3–6 to connect the new codes/schemas
  with CLI goldens and the enumeration registry. Codes, schemas and their
  first CLI goldens land together in Task 3.
- Decision 12 assigns F2's four newly cataloged verbs to C2/C4; decision 13
  limits planned help to name, summary and Cycle, with null repository and
  empty options in JSON.
- R-1 preserves the pre-thread boundary while parsing before transport;
  R-2 accepts degraded-index resolution failure; R-3 excludes the Windows
  platform-default test; R-4 proves transport failure with a fake; R-5 records
  child system-Git-config isolation limits.
- Scoped re-review at **`cd5e242`** found 38/40 resolved, two partial, and
  ten new findings (two blocking). The approved corrections use a separate
  referenced help-command schema, remove duplicate integer schema-version
  data, make help win over parser errors, avoid clap-required options, accept
  last repeated values, allow temporary unwired code explicitly, run
  mutation-contract tests with verb tasks, and investigate the CLI route to
  `repository_inaccessible`. The approved documents record these as settled.

Task 0 self-review checks the baseline and this documentation; it is not a
new independent approval of implementation Tasks 1–8. No additional subagent
review was dispatched by this implementer.

## Task Log

### Durable task ledger

Only Task 0 has been executed. For each later task, record its actual starting
HEAD, ending commit/range, controller review disposition and targeted checks
when dispatched. Unstarted rows deliberately have no invented base or result.
The Task 0 endpoint is the checkpoint commit that introduces this ledger
(`docs: record C1 baseline and refresh the API audit`); its SHA is reported
to the controller after creation rather than embedding a self-referential SHA.

| Task | Status | Base / endpoint | Reviews | Verification |
| --- | --- | --- | --- | --- |
| 0 — Baseline, audit refresh and ledger | Complete; current controller checkpoint | `de28ab0512fefee8e0b0e987c9d7a98a3ab0ef20` → introducing checkpoint commit | Implementer self-review; controller review pending | Five baseline commands above; 183 targeted tests; documentation checks |
| 1 — Clap, catalog and parsing | Not started | Not established | Not requested | Not run |
| 2 — Startup, rendering and main | Not started | Not established | Not requested | Not run |
| 3 — Codes, schemas, harness and first goldens | Not started | Not established | Not requested | Not run |
| 4 — Repository, credential, status and utility verbs | Not started | Not established | Not requested | Not run |
| 5 — Content verbs | Not started | Not established | Not requested | Not run |
| 6 — Relationship verbs | Not started | Not established | Not requested | Not run |
| 7 — Boundaries, workflow and amendments | Not started | Not established | Not requested | Not run |
| 8 — Full verification, review and handoff | Not started | Not established | Not requested | Full suite reserved here |

### Task 0 checkpoint — 2026-10-19

- Baseline matches the controller's expected refs; merge-tree is clean.
- Rechecked all 15 design Baseline Facts, all 12 named source-line references,
  the three pinned counts (**68 / 44 / 4**), 27 dispatch methods and the
  resolver. The direct DTO exception and lease-helper closing-brace locator
  are recorded above; neither changes a task's behavior or approved scope.
- Generated distinct document/comment ULIDs with the existing Node.js
  timestamp-plus-80-random-bits Crockford snippet (also preserved in
  `.agents/skills/file-ticket/SKILL.md`), invoked from Bash in this worktree.
- Added canonical comment
  `.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M59YW21Z857BP44D0QT0R1NT.md`.
- Self-review: audit rows distinguish direct call, read session, resolution
  and startup; F1 presence is not mislabeled as delivered CLI behavior;
  existing unrelated historical findings are retained.
- Documentation validation **PASS** with read-only Node assertions: 72 unique
  catalog verbs plus three invocations; all 28 C1 rows have signature/session/
  status evidence; all 27 dispatch signatures and their line anchors match
  source; actual source-entry counts are 68/44/4; both new managed IDs are
  canonical and unique; new-document/comment local links and whitespace
  pass. `git diff --check` passes. The initial Python checker could not run
  because Python is absent; the Node equivalent's first signature comparison
  needed whitespace/trailing-comma normalization before passing. Neither
  validation-tool issue required source changes or another Rust test run.
- Remaining evidence belongs to later tasks: real CLI acceptance, the full
  Task 8 gate, controller reviews and post-merge authorized native CI. No
  Task 1 work, push, PR, merge to main, closure or cleanup was performed.
