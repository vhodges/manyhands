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

Tasks 0–5 have been executed. For each later task, record its actual starting
HEAD, ending commit/range, controller review disposition and targeted checks
when dispatched. Unstarted rows deliberately have no invented base or result.
The Task 0 endpoint is the checkpoint commit that introduces this ledger
(`docs: record C1 baseline and refresh the API audit`); its SHA is reported
to the controller after creation rather than embedding a self-referential SHA.

| Task | Status | Base / endpoint | Reviews | Verification |
| --- | --- | --- | --- | --- |
| 0 — Baseline, audit refresh and ledger | Complete; both controller reviews passed | `de28ab0512fefee8e0b0e987c9d7a98a3ab0ef20` → `2aa71e7a1c4895c1e62c65d37fc81116902642c9` | Spec and quality reviews passed, no findings (controller handoff for Task 1) | Five baseline commands above; 183 targeted tests; documentation checks |
| 1 — Clap, catalog and parsing | Complete; controller reviews passed | `2aa71e7a1c4895c1e62c65d37fc81116902642c9` → `bd01cce30dd70ea161c37b01ea3cf0e0c7f98747` → `371c294195c48d8357146136bd1782db2daf2538` | Spec PASS; important authority-validation finding fixed; quality re-review PASS (Task 2 controller handoff) | Catalog/parser RED/GREEN; authority regression RED/GREEN; 24 bin tests; bin clippy; fmt; dependency/lock inspection |
| 2 — Startup, rendering and main | Complete; controller reviews passed | `371c294195c48d8357146136bd1782db2daf2538` → `447c28dc85130f39b45ec89cf9a0fe40aee58831` | Spec PASS and quality PASS, no findings (Task 3 controller handoff) | Render RED/GREEN, startup/dispatch/human RED/GREEN; 48 bin tests; bare-call smoke; bin clippy; fmt |
| 3 — Codes, schemas, harness and first goldens | Complete; controller reviews passed | `447c28dc85130f39b45ec89cf9a0fe40aee58831` → `2e3849f31c0d7abd39b559ea15edce7312c2df03` | Spec PASS and quality PASS, no findings (Task 4 controller handoff) | Result-code and binary RED/GREEN; exhaustive-match RED; 323 targeted tests; 10 generated/read/verified fixtures; scoped clippy; fmt |
| 4 — Repository, credential, status and utility verbs | Complete; controller reviews passed | `2e3849f31c0d7abd39b559ea15edce7312c2df03` → `a45d99cb9ef0369cf1602bd9971f9b8799a58166` → `4fb690cb70f5f43566838dc26e1a7ea9ea3794a1` | Spec PASS; quality re-review PASS after metadata-escaping fix (Task 5 handoff) | Initial 138 targeted tests and 28 new JSON fixtures; fix: 4 regressions RED/GREEN, 66 bin + 33 CLI tests; scoped clippy/fmt PASS |
| 5 — Content verbs | Implemented; self-review PASS; controller review pending | `4fb690cb70f5f43566838dc26e1a7ea9ea3794a1` → checkpoint introducing the Task 5 entry below | Spec/quality self-review PASS; no library gap or unresolved finding | Binary and renderer/helper RED/GREEN; 73 bin + 44 CLI + 43 mutation-contract tests; 7 JSON + 2 text new goldens; scoped clippy/fmt PASS |
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

### Task 1 checkpoint — 2026-10-19

**Authorization and base.** The controller dispatched only Task 1 with the
owner's checkpoint-commit approval. Initial HEAD was
`2aa71e7a1c4895c1e62c65d37fc81116902642c9`, on the expected ticket branch, with
a clean worktree. No subagents were dispatched. Main synchronization remains
the controller's recorded baseline; no fetch, merge or rebase was performed in
this bounded task. `origin/main` remains
`f6a4c06300f4be80eb18f843a9e4fcbde847e47e`. The read-only
`git merge-tree --write-tree HEAD origin/main` check exited 0 with tree
`728ec6a67819a28f23e8ebc1858b9ef552ba1324` before the checkpoint commit.

**Task 0 review handoff.** The controller reports that both Task 0 spec and
quality reviews passed with no findings. The 183 baseline targeted tests,
check, fmt and clippy evidence above stands; it was not rerun. Carry forward
the two baseline corrections: `new_item_id` returns its DTO infallibly, and
the complete lease-child helper ends at line 59.

**Implementation.**

- Added clap with defaults disabled and only `std`, `help`, `usage` and
  `error-context`. Resolved **clap 4.6.7**, **clap_builder 4.6.7** and
  **clap_lex 1.1.1**. Lock diff adds only those packages and the root clap
  dependency; `anstyle` was already locked and no existing version changed.
- Added the catalog's exact **72 commands**: **28 C1**, **18 C2**, **13 C3**,
  **7 C4**, **6 C5**, with summaries, availability, repository rules and
  per-verb option metadata. Planned commands publish no rules or options.
- Added a clap builder derived from that catalog and the agreed `Invocation`,
  `Action`, `HelpTarget`, scans and `authority` interfaces. `parse::command()`
  exposes the same tree for Task 2's help rendering. IDs remain strings;
  closure/direction have the approved lists/defaults, depth is `u32`, and
  authority validation strips IPv6 brackets in its returned tuple.
- Help/error precedence, planned catch-all handling, exact raw scans up to
  `--`, last repeated values, UTF-8 validation and per-verb validation are
  covered by bin-private tests. Tests exercise each C1 minimum invocation,
  remove each required input, refuse mutation-only options on every C1 verb,
  and check planned options/help across all later-Cycle commands.
- Preserved the original seven-line `main` body. Added the exact
  `result_large_err` rationale from `src/repository/read/mod.rs` and the
  specified temporary dead-code annotations. No library, repository,
  pre-existing test, schema or later-task module was edited.

**Mechanical correction.** The design's module-resolution sentence assumes
bare `mod catalog; mod parse;` beneath `src/bin/manyhands-cli.rs` locates the
approved directory. Explicit `#[path = "manyhands-cli/catalog.rs"]` and
`#[path = "manyhands-cli/parse.rs"]` on those declarations preserve that exact
layout and crate-private module names. Test modules use the required
`#[cfg(test)] #[path = "…_tests.rs"]` layout. No behavior or scope change.

**Verification (all Rust commands through Devenv).**

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check` | PASS; the single unlocked dependency-update command |
| `git diff -- Cargo.lock` | Reviewed: three new packages only; no existing version changes |
| `devenv shell -- cargo tree --locked -e normal --no-default-features -i clap` | PASS; `clap v4.6.7` has the single reverse edge `manyhands v0.1.0` |
| `devenv shell -- cargo test --locked --bin manyhands-cli catalog` | RED: 4 failed against the empty catalog/lookup scaffolding; GREEN: 4 passed after the catalog implementation |
| `devenv shell -- cargo test --locked --bin manyhands-cli parse` | RED: 20 failed at the unimplemented parser/scan/authority scaffolding; 4 catalog tests filtered out |
| `devenv shell -- cargo test --locked --bin manyhands-cli` | GREEN: 24 passed, 0 failed/ignored/filtered; also 24 passed after formatting patches |
| `devenv shell -- cargo clippy --locked --bin manyhands-cli -- -D warnings` | PASS before and after formatting |
| `devenv shell -- cargo fmt --check` | Initial formatting-only differences in the four new files corrected via `apply_patch`; final PASS |
| `git diff --check` | PASS |

**Self-review.** Compared the catalog and parser against the approved five-row
catalog, options, per-verb and action-order tables. All specified public
interfaces are present; command/option discovery has one production catalog;
no clap-required inputs or color APIs are used. The main body is unchanged,
the lock additions are isolated, and the tests invoke only parsing (no
transport, service, repository, key or filesystem access). No unresolved
findings or task-altering contradictions. Independent Task 1 controller
reviews remain pending.

Canonical completion comment:
`.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M5A0CB7E189YBKPMZF96NGPR.md`.
The endpoint SHA is returned to the controller after committing, avoiding a
self-referential commit ID here. Task 2 remains unstarted. The full suite is
reserved for Task 8; no native workflow, push, PR, closure or cleanup occurred.

### Task 1 quality-review fix — 2026-10-19

- Controller reports **Task 1 spec PASS** and one **important** quality finding:
  unbracketed authorities accepted `user@example.com` and `example.com/path`.
  This supersedes the original self-review's no-findings assessment for that
  boundary. Fix base: `bd01cce30dd70ea161c37b01ea3cf0e0c7f98747`; initial
  worktree clean on the same ticket branch.
- Independently verified design lines 264–270: only host/optional-port and
  bracketed IPv6 forms fit the syntax. `authority` checked empty hosts,
  brackets and whitespace but omitted `@` and `/`. In F1,
  `src/repository/read/credentials.rs:116–120` rejects both delimiters;
  `inspect_host` at lines 259–266 opens a read session before returning
  `authority_not_found` on that rejection (or a session error). Thus letting
  these inputs become `Action::Run` would cross the startup boundary after
  wiring. No library change is needed.
- Added four cases to the existing malformed-authority test: both reported
  forms and each with an explicit `:2222` port. Every case asserts helper
  `Err` and `Action::Usage` naming `host inspect` with a nonempty detail.
- **RED:** `devenv shell -- cargo test --locked --bin manyhands-cli
  parse::tests::malformed_authorities_are_usage_errors_before_dispatch --
  --exact` failed at `user@example.com` before changing production code:
  1 failed, 23 filtered out.
- Minimal fix: add `@` and `/` to the unbracketed host's rejected characters.
  Valid host text is still returned unchanged; port and IPv6 handling are
  unchanged. **GREEN:** the identical targeted command passed, 1 passed and
  23 filtered out.
- Regression verification: `devenv shell -- cargo test --locked --bin
  manyhands-cli` **24 passed**, including valid-host case preservation and
  IPv6/default-port coverage; targeted bin clippy with `-D warnings` and
  `cargo fmt --check`, both through Devenv, **PASS**. No full suite run.
- Self-review: the only production change is the two-character rejection
  extension; the existing helper is invoked before `Action::Run`, and the
  regression covers both helper and action boundaries. Controller quality
  re-review remains pending. Task 2 remains unstarted.
- Canonical comment:
  `.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M5A0S7NC3VH4V9T2BDV6P84X.md`.
  New commit subject: `fix: reject invalid CLI authority delimiters`; endpoint
  SHA is returned after commit. No amend, push, PR, merge, closure or cleanup.

### Task 2 checkpoint — 2026-10-19

**Authorization and base.** The controller dispatched only Task 2 with the
owner's checkpoint-commit approval, test-first execution and self-review, and
no subagents. Initial HEAD was
`371c294195c48d8357146136bd1782db2daf2538`, with a clean worktree on
`manyhands/ticket/01M4CC0VMSZ7BRS740C8VSQ5ZK`. Read the current AGENTS, ticket,
prior checkpoint comments, approved Cycle/design/implementation plan and this
ledger. The required `git fetch origin main && git merge origin/main` ran
inside the ticket worktree and returned **Already up to date**; no commit or
base change. `origin/main` remains `f6a4c06300f4be80eb18f843a9e4fcbde847e47e`.
No rebase or merge into main occurred.

**Task 1 review handoff.** The controller reports **spec PASS** and **quality
re-review PASS** after `371c294` rejected `@` and `/` in unbracketed authority
hosts. This resolves the pending re-review recorded above. Task 1 source and
tests were not changed in Task 2.

**Implementation.**

- Added `render.rs` and its tests: exhaustive exit mapping, compact JSON plus
  newline, human stdout/notes, fixed failure/code/recovery lines, human-only
  extra detail, and `write_all`/flush on both streams. A private writer helper
  permits short-write and all four write/flush failure cases to be tested;
  the process wrapper exits 1 on error without printing a diagnostic.
- Serialization failure maps to a fixed `internal_error` envelope, retaining
  command/scope and discarding the serializer's error text. A sentinel test
  proves that backend text is not printed. Both temporary code constants
  remain `ResultCode::InvalidInput`, pending Task 3's codes and goldens.
- Added `startup.rs` and its tests: service-free actions, transport-before-open
  for all 28 available Run actions, transport failure, service-open failure,
  successful ID dispatch, and unset/empty/absolute/relative data paths. Unix
  coverage also preserves non-UTF-8 data-directory paths.
- Help uses the existing clap tree. Its catalog-derived `after_help` epilogue
  exposes all 72 full command names and their availability/delivering Cycle,
  because clap's default root help stops at resources. Resource help filters
  that epilogue; available-command help inherits global flags and full binary
  names after `Command::build`. Planned-command help is exactly name, summary
  and `Arrives with <Cycle>.`, with no speculative options. Root/resource help
  documents `MANYHANDS_DATA_DIR` as an advanced/testing override.
- JSON help uses the approved bin-private DTO fields, default scope and
  not-requested effects. `RepoRule` serializes through `REPOSITORY_VALUES[0/1]`;
  planned entries retain null repository and empty options. Version has only
  `name` and package `version` data. Usage/unavailable canonical commands,
  empty recovery and human-only detail follow the approved contract.
- Added `dispatch.rs`, `human.rs` and their tests. Only `id new` calls a read
  service, using the direct `NewIdDto` return without fallible wrapping.
  Its human form is only ID plus newline. The exact other 27 available verbs
  are explicit placeholders, each tested in both modes. An unlisted Run is
  classified as an internal routing error rather than accidentally succeeding.
- Wired `main` to args_os → parse → run with RealStartup → write_and_exit.
  RealStartup retains the pre-thread unsafe-call safety comment, uses the
  data-directory helper, converts RepositoryError through ReadError, and maps
  current-directory failure to `internal_error`. Explicit module paths are
  retained; the catalog/parser dead-code allows are removed. The authority
  annotation remains for Task 4.

**RED/GREEN and final verification.** All Rust commands ran inside this
worktree through Devenv, locked except fmt. Only the CLI bin was tested/linted;
the full suite remains reserved for Task 8.

| Command / stage | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --bin manyhands-cli render` before implementation | RED: 8 tests failed at the unimplemented render/writer scaffolding; 24 filtered out |
| Same render command after implementation | GREEN: 8 passed, 0 failed; 24 filtered out |
| `devenv shell -- cargo test --locked --bin manyhands-cli` with new startup/dispatch/human tests and scaffolding | RED: 16 failed at unimplemented entry points; existing 32 passed |
| Same bin command after startup/dispatch/human implementation | GREEN: 48 passed, 0 failed/ignored/filtered |
| `devenv shell -- cargo run --locked --bin manyhands-cli` before main wiring | Exit 0, no application output: observed missing-help baseline |
| Same smoke command after main wiring | PASS: exit 0, clap root help, all 72 catalog entries and override documentation printed |
| `devenv shell -- cargo fmt --check` | Initial formatting-only differences corrected via apply_patch; final PASS |
| `devenv shell -- cargo test --locked --bin manyhands-cli` after wiring/formatting | PASS: **48 passed**, 0 failed/ignored/filtered (24 existing + 24 new) |
| `devenv shell -- cargo clippy --locked --bin manyhands-cli -- -D warnings` | PASS before and after formatting; final exit 0 |
| `git diff --check` | PASS |
| `git merge-tree --write-tree HEAD origin/main` before checkpoint | PASS, exit 0; tree `d9f4da4a338237e4535f57937fa843d3e531e203` |

**Self-review.** Compared the actual Task 2 changes with every interface and
Step 1–5 requirement, including startup short-circuiting, all help targets,
planned-help restrictions, scope/effects, exact failure prefixes, recovery
arguments, newline/stream behavior, write failure exit handling and the unsafe
initialization boundary. Inspected final production files and main diff. No
unresolved spec or quality finding. This is self-review only; independent
controller review remains pending. The merge-tree check covers committed
HEAD against main, not the uncommitted Task 2 additions.

**Boundaries and remaining evidence.** Only the nine intended CLI Rust files,
this ledger and the canonical checkpoint comment change. No library,
repository, existing test, schema, dependency or lockfile changes. The real
binary golden/schema harness belongs to Task 3; real broken-pipe process
coverage belongs to Task 7 (writer failure behavior is unit-tested here).
Native evidence and the full Cycle gate remain future work. Task 3 was not
started. Ticket remains open; no push, PR, closure or cleanup.

Canonical checkpoint comment:
`.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M5A1FZQYZV2DNHRTBJT9B8MV.md`.
The endpoint is the commit introducing this entry; its SHA is returned after
creation to avoid a self-referential commit ID.

### Task 3 checkpoint — 2026-10-19

**Authorization and base.** Only Task 3 was dispatched, with owner approval for
the checkpoint commit, TDD/apply_patch and inline self-review. Initial HEAD:
`447c28dc85130f39b45ec89cf9a0fe40aee58831`, clean on the ticket branch in the
ticket worktree. Read local AGENTS, the canonical ticket and latest comment,
approved Cycle/design/plan and this ledger. Required main preflight fetched
`origin/main`, verified it is an ancestor of HEAD and ran the already-up-to-date
merge; no commit or base change. Main remains
`f6a4c06300f4be80eb18f843a9e4fcbde847e47e`.

**Task 2 review handoff.** The controller reports **spec PASS** and **quality
PASS**, **no findings**, with **48 bin tests passing** at `447c28d`. This
supersedes the pending controller-review status in the historical Task 2 entry.

**Implementation.**

- Added `UsageError` and `CommandUnavailable` directly after `InvalidInput`,
  with the approved Input classes, fixed messages and schema enum ordering.
  The two rendering constants now name them. The only `src/repository/` edit
  is the approved comment and two patterns in the no-action recovery arm.
- Added four closed, all-properties-required help/version schemas; command
  and option entries are reached by `$ref`. The help repository enum is
  `required`, `refused`, null, tied to the test registry and real binary output.
- Added the CLI harness with an owned temporary home, configurable data/cwd,
  explicit override removal, all seven redirected home/config/data variables,
  inherited Git/SSH/GCM removal and isolated Git configuration variables.
  Default cwd is the temporary home. R-5 is documented at the environment
  boundary; the repository factory sets fixture-local identity and LF policy.
- Child stdin is closed. Two test-process threads drain output concurrently;
  `try_wait` reaps the child with a 30-second bound (kill/wait on timeout),
  then both readers must report EOF within a shared one-second deadline.
  JSON checking requires exactly one newline-terminated object and no stderr.
- Added the separate JSON/text golden registry and helpers reusing
  `golden::render` and `update_requested`; `golden.rs` is unchanged. Added
  exact-directory membership checking and the entire top-level lease-child
  function copied from `read_boundary.rs:34–59` for later tasks.
- Real-binary coverage checks root/resource/available/planned help, version,
  the bare invocation, unknown commands/options, all four mutation-only options
  on a read in both modes, planned-command trailing JSON and C5 human detail,
  repeated JSON around `id new`, and a data-directory regular-file failure.
  Every JSON result is schema checked, with its data schema when present;
  service-free paths leave the requested data directory absent.
- Applied exactly named existing-test edits 1–6. Counts are now **70** design
  codes, **46** data roots and **6** mutation-golden exceptions. The shared
  golden scanner includes only CLI JSON cases under the `cli_v1` label.

**RED/GREEN and verification.** Every Rust command ran through Devenv, locked
except fmt. Only Task 3's named filters and targets ran.

| Command / stage | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --lib results::tests`, after named test edits and before codes | RED: **2 failed**, **47 passed**, 1147 filtered; exactly the code table and fixed-message tests |
| `devenv shell -- cargo check --locked --lib`, after codes and before recovery-arm edit | Expected RED: E0004 at `mutation/outcome.rs:719`, naming both new variants |
| `devenv shell -- cargo test --locked --test cli mutation_only_options_on_reads_are_usage_errors_in_both_modes`, before rendering constants changed | RED: **1 failed**, 9 filtered; human code was still the temporary `invalid_input` |
| `devenv shell -- cargo test --locked --test cli planned_command_recognizes_trailing_json_and_names_its_cycle_in_human_mode`, before constants changed | RED: **1 failed**, 9 filtered; unavailable fixed message was still the temporary alias |
| `MANYHANDS_UPDATE_GOLDEN=1 devenv shell -- cargo test --locked --test cli help::` | GREEN: **8 passed**, 2 filtered; single fixture-generation run |
| `devenv shell -- cargo test --locked --lib results::tests` | **49 passed**, 1147 filtered |
| `devenv shell -- cargo test --locked --lib read::` | **65 passed**, 1131 filtered |
| `devenv shell -- cargo test --locked --lib mutation::outcome` | **37 passed**, 1159 filtered |
| `devenv shell -- cargo test --locked --test read_contract --test mutation_contract --bin manyhands-cli --test cli` | **71 + 43 + 48 + 10 passed**; golden update variable absent |
| `devenv shell -- cargo clippy --locked --lib --bin manyhands-cli --test read_contract --test mutation_contract --test cli -- -D warnings` | PASS |
| `devenv shell -- cargo fmt --check` | Initial new-file formatting differences corrected via apply_patch; final PASS |
| `git merge-tree --write-tree HEAD origin/main` before checkpoint | PASS; tree `56f6512c05d6fefe98ef7f8d99f9260f71052c05` (committed base, before Task 3 additions) |

Final scoped total: **323 passed**, zero failures/ignored. The CLI count
includes the exact-directory test and inactive lease-child helper. Generated
**8 JSON + 2 text** fixtures once, read every generated file, then verified
without the update variable. Root text deliberately preserves clap's literal
padding spaces in its resource list; fixtures are byte contracts, marked
`-text`, not hand-trimmed output.
The staged `git diff --cached --check` reports exactly those 16 generated
resource-list lines; the check excluding only `help_root_text.txt` passes.

**Self-review.** Inspected the actual source/test/schema diff against Task 3
Steps 1–9, reviewed all new harness/test files and all generated fixtures.
Confirmed the only existing semantic test changes are the six named edits,
the repository exception is exactly the approved no-action addition, and no
additional library gap was found. All required interfaces and isolation
variables are present; planned help retains null repository and empty options;
fixed failure messages, empty scope/data/recovery and stream/exit behavior
match the design. Spec/quality self-review PASS, no unresolved findings.
Independent controller review is pending; native behavior is not claimed.

Canonical checkpoint comment:
`.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M5A29F5AVHDZNAHG3NAV62AS.md`.
Endpoint is the commit introducing this entry, returned after creation. Task 4
remains unstarted, the ticket stays open, and the full suite remains reserved
for Task 8. No subagents, push, PR, merge into main, rebase, closure or cleanup.

### Task 4 checkpoint — 2026-10-19

**Authorization and base.** The controller dispatched only Task 4, with the
owner's task-checkpoint commit approval, inline TDD/apply_patch and self-review.
Initial HEAD was `2e3849f31c0d7abd39b559ea15edce7312c2df03`, clean on
`manyhands/ticket/01M4CC0VMSZ7BRS740C8VSQ5ZK` in its existing ticket worktree.
Read AGENTS, the canonical ticket and Task 3 comment, approved Cycle/design/plan
and this ledger. The required AGENTS preflight fetched main and ran the
already-up-to-date merge: no base or commit change. `origin/main` remains
`f6a4c06300f4be80eb18f843a9e4fcbde847e47e`.

**Task 3 review handoff.** The controller reports **spec PASS, quality PASS,
no findings, 323 targeted tests** at `2e3849f`. This supersedes the pending
review status in the historical Task 3 checkpoint. Its tests were not rerun
outside Task 4's three authorized targets.

**Implementation and acceptance mapping.**

- Added one typed dispatch function per new verb: `repo list/inspect/identity`,
  `remote list`, `key list/show/public`, `host list/inspect`, `index status`,
  `poll status`, and `operation list/show`. Each calls its existing F1 read;
  errors go through `ReadError::to_envelope` and successes through
  `Envelope::read_success`. Only the 14 content/relationship placeholders
  remain; their Task 2 tests now pin that exact set.
- `repo inspect` calls `inspect_repository(path)` without registration-required
  resolution; the real-binary unregistered inspection test succeeds. All other
  repository verbs call public `resolve(service, args)` first. Relative repo
  paths join the invocation cwd. Key IDs use `SharedKeyId::parse` with default
  failure scope; operation IDs use `OperationId::parse` after resolution with
  `repo.scope()`. The unregistered-plus-bad-operation-ID case proves precedence.
- Public `item_id(args, repo)` uses the canonical parser and scoped invalid-ID
  error. A bin test covers a canonical ID, lowercase ULID and short code. Its
  sole temporary dead-code allow is explicitly for Tasks 5–6 dispatch; those
  tasks remove it when wiring. `parse::authority`'s old allow is removed.
- Host inspection reuses the already-validated authority parser, stripping IPv6
  brackets before constructing `SshAuthority`. Real pins cover `[::1]:2222`,
  hostname case normalization and default port 22.
- Added 13 DTO human renderers and one unit test per renderer. Tests cover
  metadata, fixed enum/code strings, optional fields, stored times, empty lists,
  incomplete/stale/never-refreshed notes and host reapproval. `key public`
  remains exactly public text plus newline; `id new` remains exactly ULID plus
  newline. Binary tests cover every Task 4 verb in both modes, one line per
  repository registration, empty global lists and stderr-only failure with
  `error:`, `code:` and `next: repo.inspect root=…`.
- `tests/cli/admin.rs` checks success status, envelope/data schemas, canonical
  command and full scope for all 13 reads plus utility `id new`. Goldens use
  F1 names prefixed `cli_`. Scope follows `read_contract.rs`: repo list,
  credentials, hosts and ID generation use default scope; inspection uses
  DTO.root; identity/remotes/status/operations use the resolved repository scope.
- Failure goldens cover `invalid_path`, `not_repository`, `not_repository_root`,
  `bare_repository`, `repository_not_registered` (also before a bad ID),
  `invalid_id` for both key and operation selectors, `key_not_found`,
  `authority_not_found`, `operation_not_found`, `public_key_unavailable`,
  `repository_inaccessible` and `index_unavailable`. Blocked results exit 3;
  input results exit 2. The public-key case uses `keys.without_public`.
- The Unix mode-000 configuration route **does reach**
  `repository_inaccessible` through `remote list`, exit 3, with repository
  scope. Permissions applied in this Linux run; the test restores them before
  assertions and returns early only when they are bypassed (root/capability).
- R-2 is preserved: degraded-service data gives `index_unavailable` from
  resolution, null data and `index.rebuild` recovery. There is deliberately no
  CLI `index_status_unavailable` success golden. The exclusive cache-write
  lease child gives `busy`, exit 5; only code/status are pinned because opening
  and resolution can encounter the lock with different scopes.
- Path review covers trailing slash, `.` from root, `..` from root/docs,
  canonical path beneath `with space é`, and a linked item worktree created
  by `items::create_document_context`. Every `repo identity --json` result has
  the exact canonical owner scope, with no branch/worktree/item scope added.

**RED/GREEN and verification.** All Rust commands ran in this worktree through
Devenv, locked except fmt. No test target beyond bin/CLI/mutation_contract ran.

| Command / stage | Evidence |
| --- | --- |
| `devenv shell -- cargo test --locked --test cli admin::` before dispatch | All **13 new verbs failed with `command_unavailable`**. The already-implemented ID test reached its missing new golden. Initial run: 14 failures, 10 filtered. |
| Same command after adding failure/path tests, before dispatch | 22 failed, 1 passed, 10 filtered. The lease test already passed because startup can return `busy` before dispatch. Remaining new tests failed at unavailable dispatch; ID still needed its fixture. |
| `devenv shell -- cargo test --locked --bin manyhands-cli human::` against renderer scaffolding | **13 failed**, existing ID renderer passed, 47 filtered. Every new renderer failed at its unimplemented entry point. |
| `devenv shell -- cargo test --locked --bin manyhands-cli dispatch::` before wiring | **2 failed**, 2 passed, 58 filtered: new helper failed at scaffolding; placeholder count was still 27 instead of 14. |
| `devenv shell -- cargo test --locked --bin manyhands-cli` after implementation | **62 passed**, no failures/ignored/filtered. |
| `MANYHANDS_UPDATE_GOLDEN=1 devenv shell -- cargo test --locked --test cli admin::` | **23 passed**, 10 filtered; generated 28 new JSON fixtures. |
| `devenv shell -- cargo test --locked --test cli admin::repository_failures_and_resolution_precedence -- --exact` | Golden-review finding reproduced: **1 failed**, 32 filtered, solely because bare-repository scope retained the temporary path. |
| Same exact filter with `MANYHANDS_UPDATE_GOLDEN=1`, after placeholder correction | **1 passed**, 32 filtered; regenerated the six failure-case goldens in that test, with the bare root now normalized. |
| `devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test mutation_contract` (update variable absent) | **62 + 33 + 43 = 138 passed**, zero failures/ignored/filtered. CLI includes 23 admin tests, the fixture-directory check and inactive lease-child helper. |
| `devenv shell -- cargo clippy --locked --bin manyhands-cli --test cli --test mutation_contract -- -D warnings` | Initial test-only lint required octal permission notation; corrected `0` to `0o000`. Final **PASS**. |
| `devenv shell -- cargo fmt --check` | Formatting-only differences corrected via apply_patch; final **PASS**. |
| `git diff --check` | **PASS**. |
| `git merge-tree --write-tree HEAD origin/main` | Exit 0, tree `708e2e34b2acb41e6dd8cc104460b8472d2f4809`; this precommit check covers committed Task 3 HEAD. |

Read all **28** generated JSON files before verification. Fixture review found
the bare-root normalization omission; its focused reproduction identified the
test placeholder selection, not production scope, as the cause. Corrected only
that test's root mapping, read the corrected golden, then verified all three
authorized targets without the update variable. CLI fixtures now total
**36 JSON + 2 text**. Existing goldens are unchanged.

**Design codes with no C1 envelope route.** These are Cycle-wide exclusions,
not missing Task 4 implementations. Content failures `item_not_found` and
`path_not_found` remain reachable through Task 5 and are not exclusions.

| Code(s) | Why C1 cannot produce them |
| --- | --- |
| `repository_not_enabled` | F1's resolver returns `repository_not_registered` for an unregistered root; it does not use the mutation gate. |
| `relationship_cycle`, `invalid_relationship` | Rejection results belong to `ticket check`/authoring, moved to C3. Relationship read DTOs report stored problems without returning these envelope failures. |
| Transport-start `internal_error` route | Cannot arrange libgit2 rejecting its own timeout options through the binary (R-4); Task 2 fake-startup test is evidence. The code itself **is reachable**, as Task 3's regular-file data-directory test proves. |
| `invalid_input`, `request_mismatch`, `request_not_found`, `confirmation_mismatch`, `not_confirmable`, `confirmation_required`, `confirmation_expired`, `confirmation_not_found`, `confirmation_used`, `already_applied` | Request parsing/replay and prepare/confirm/execute flows are absent. CLI grammar errors are `usage_error`; planned verbs are `command_unavailable`. |
| `slug_already_assigned`, `occupied_path`, `not_repairable`, `external_change`, `recovery_required`, `original_request_required`, `identity_required`, `initials_required`, `invalid_slug_configuration`, `identity_ambiguous`, `ticket_closed`, `wrong_branch`, `worktree_not_clean`, `worktree_conflicted`, `invalid_configuration` | Authoring/configuration mutation gates and recovery do not run. Reads report stored/configuration/identity state in DTOs. |
| `remote_name_conflict`, `invalid_remote`, `publication_remote_required`, `publication_remote_in_use`, `merge_required`, `remote_branch_deleted`, `push_rejected` | Remote configuration, synchronization and publication mutations do not run; C1 makes no network attempt. |
| `key_not_deletable`, `no_selected_key`, `key_unavailable`, `key_rejected`, `selected_key_in_use`, `unlock_required`, `unlock_failed`, `host_approval_required`, `host_replacement_required`, `host_mismatch` | Key mutation/unlock and transport trust/authentication gates do not run. Public-key reading has its separate `public_key_unavailable` code. Stored operation/poll outcomes can contain similar names as data, not envelope failures. |
| `discovery_pending`, `registration_pending`, `poll_yielding`, `remote_unavailable`, `transport_unavailable`, `cancelled` | No mutation hand-off, poll scheduler, transport attempt or cancellable mutation runs. Thus C1 produces neither incomplete nor cancelled exit classes (mapping remains unit-tested). |

**Self-review.** Compared actual source/test changes with Task 4 Steps 1–6,
the design Dispatch/scope tables and the F1 golden scopes. Reviewed every new
golden, path/ID precedence, error conversion, human stream behavior and the
remaining placeholder set. No SQL, backend error text, Debug rendering, key-file
read or domain implementation is added to the bin. All implementation changes
are in the approved bin/tests files; no library, repository, schema, dependency,
lockfile or pre-C1 semantic test changes. No library gap was found. Spec/quality
self-review PASS with no unresolved findings; independent controller review
remains pending. Native platform behavior is not claimed.

Canonical checkpoint comment:
`.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M5A3JMZCNDJB7EA6WJQJTZB1.md`.
Endpoint is the commit introducing this entry and is returned after creation.
Task 5 remains unstarted; the ticket remains open. No subagents, rebase, push,
PR, merge into main, closure or cleanup. Full-suite verification stays in Task 8.

### Task 4 quality-review fix — 2026-10-19

**Review and technical judgment.** The controller reports **Task 4 spec PASS**
and an **important/P2 quality finding** at
`a45d99cb9ef0369cf1602bd9971f9b8799a58166`: raw control characters in human
metadata can add forged lines or terminal sequences, and break one-line-per-
registration output. This supersedes the original self-review's no-findings
assessment. Initial worktree was clean on the ticket branch; required main
preflight again returned Already up to date with no base change.

Verified the finding against unchanged domain sources:

- `src/repository/keys/registry.rs:124–126` and `1059–1061` reject only
  `label.trim().is_empty()`, allowing embedded newline, CR and ESC. The read
  DTO copies `registration.label` at `read/credentials.rs:49`.
- `src/repository.rs:10525–10528` accepts nonempty Git name/email strings
  (strictly nonempty, not a control-character or whitespace validator).
  `read/admin.rs:174–180` copies those strings into the identity DTO.
- The original `human.rs` directly interpolated roots, labels, identity
  optionals and other metadata. No domain guarantee justified treating them
  as single-line display text.

This is a CLI display-boundary defect within Task 4, not a library gap or a
domain-validation change. Added one shared field-level helper that preserves
ordinary Unicode and escapes characters satisfying `char::is_control()` via
`escape_default()`: LF as `\n`, CR as `\r`, ESC as `\u{1b}`, and similarly
tab/C1 controls. Required metadata strings use it directly; optional metadata
uses it through `optional`. Branch-list elements are escaped before the
renderer joins them. Fixed enum strings, numbers and booleans remain literal.

Applied consistently across repository roots/selection/configuration/problem
paths, identities, remote names/locations, key metadata/paths, host metadata,
and index/poll/operation metadata. Layout is never escaped as a whole.
`key public` and `id new` retain exact output; the helper explicitly excludes
canonical public-key text and future requested bodies/sources. No Task 5
behavior or domain/library edit was introduced.

**Test-first evidence and checks.**

| Command / stage | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --bin manyhands-cli metadata_escape` before production edit | **RED: 4 failed**, 62 filtered. Raw field newlines added metadata lines: identity 7 rather than 5, key show 12 rather than 9, repository list 2 rather than 1, host inspect 7 rather than 4. |
| Same filter after field-level escaping | **GREEN: 4 passed**, 62 filtered. Tests also require literal escaped LF/CR/ESC/tab/C1 values, preserved `é 中文`, renderer-owned newlines and no residual field controls. |
| `devenv shell -- cargo test --locked --bin manyhands-cli --test cli` | **66 + 33 = 99 passed**, zero failed/ignored/filtered. Includes unchanged exact public-key/ID unit and binary tests; golden update variable absent. |
| `devenv shell -- cargo clippy --locked --bin manyhands-cli --test cli -- -D warnings` | **PASS** |
| `devenv shell -- cargo fmt --check` | Initial two formatting differences corrected via apply_patch; final **PASS** |
| `git diff --check` | **PASS** |

Only human rendering and its unit tests change in code. CLI integration is
affected because it exercises those human outputs; it was rerun. JSON dispatch,
schemas and all goldens are unchanged, so mutation-contract tests were not
affected and were not rerun. No full suite or native workflow ran.

**Self-review and checkpoint.** Checked every dynamic string interpolation in
the Task 4 renderers: metadata passes through the helper, while canonical
public key/ID output and renderer layout bypass it. The four new tests exercise
all 12 metadata renderers; the thirteenth Task 4 renderer is exact public-key
text. The P2 finding is fixed; independent quality
re-review remains pending. New commit subject:
`fix: escape control characters in CLI metadata` with the required co-author
trailer. Its SHA is returned after creation; no amend.

Canonical comment:
`.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M5A44MX856GSKHZA76TQ1F49.md`.
Task 5 remains unstarted. No subagents, rebase, push, PR, merge into main,
closure or cleanup; ticket remains open and full-suite verification stays in
Task 8.

### Task 5 checkpoint — 2026-10-19

**Authorization and base.** Only Task 5 was dispatched, with owner approval for
the checkpoint commit, inline TDD/apply_patch and self-review. Initial HEAD was
`4fb690cb70f5f43566838dc26e1a7ea9ea3794a1`, clean on the specified ticket branch
in its existing worktree. Read current AGENTS, the canonical ticket and latest
comment, approved Cycle/design/plan and ledger. The required AGENTS preflight
fetched main and ran `git merge origin/main`: **Already up to date**, no base
or commit change. Main remains `f6a4c06300f4be80eb18f843a9e4fcbde847e47e`.

**Task 4 review handoff.** The controller reports **spec PASS and quality PASS**,
including fix `4fb690c` for field-owned control escaping, with **66 bin + 33 CLI
tests passing**. This supersedes the pending quality re-review recorded in the
historical Task 4 entry. The shared `field`/`optional` helpers are reused by
Task 5 for metadata, paths and titles; requested body/source bytes bypass them.

**Implementation and acceptance mapping.**

- Added the six content dispatches: `document list/show`, `folder list`,
  `ticket list/show` and `comment list`. Each uses the existing F1 service and
  DTO unchanged; all repository resolution precedes ID parsing. Removed only
  these six placeholders, leaving the exact eight relationship verbs for Task 6.
- Both show verbs use the shared selector/kind-check helper. ID success and
  kind-mismatch scopes contain the item ID; by-path scopes are repository-only,
  even with an explicit context. F1 read failures retain their own scopes.
  Tests cover both successful selector paths, primary and linked explicit
  contexts, both directions of ID/path kind mismatch, short-code invalid ID,
  absent path, parent-component invalid path and absent ticket. Unregistered
  repositories beat bad item IDs for document, ticket and comment reads.
- `ticket_filter` forwards all exact-text options, defaults closure to All and
  maps ready/blocked flags. It uses `try_get_one`, so the narrower future
  relationship argument sets do not panic on undeclared closure/slug/flags.
  A helper-only unit test exercises those argument sets without implementing
  or dispatching Task 6. Twelve real-binary filter cases compare ordered IDs
  with in-process F1 `list_tickets` using the same `TicketFilter`; conflicting
  readiness flags return `usage_error`, exit 2, empty scope.
- Human lists have one line per entry, IDs and slug-or-dash, ticket lifecycle
  closure distinct from status, and titles. Nonconforming entries show their
  escaped path and fixed problem codes. Folder lines mark empty directories;
  comments indent two spaces per depth and show metadata, not bodies.
- Show renders field-escaped metadata, a blank line, then the requested body
  or nonconforming source exactly, without adding a final newline or escaping
  its controls. DTO unit tests cover multiline/control metadata, ordinary
  Unicode, exact requested content, missing source, empty lists, thread depth,
  lifecycle/status disagreement, and separate stale/never-refreshed/incomplete
  notes. Real-binary stale tests use `refresh_required = 1` and a document mtime
  moved 30 seconds ahead; both remain successful reads.
- Added `tests/cli/content.rs`, registered it in `tests/cli.rs`, and registered
  all nine new fixtures in `CLI_CASES`/`CLI_TEXT_CASES`. Every content JSON
  result checks canonical command, full scope, exit/code and envelope/data
  schemas. Fixtures use `items::contract_repository`, comment helpers and the
  existing isolated CLI harness. The seven success JSON files mirror F1 exactly;
  text fixtures pin `ticket_list_text` and absent-ticket `item_not_found_text`.

**RED/GREEN and verification.** All Rust commands ran through Devenv, locked
except fmt. Only the three authorized targets ran; the full suite stays in Task 8.

| Command / stage | Result |
| --- | --- |
| `devenv shell -- cargo test --locked --test cli content::` before dispatch | **RED: 11 failed**, 33 filtered; all failures reported `command_unavailable` |
| `devenv shell -- cargo test --locked --bin manyhands-cli` with new unit tests and scaffolding | **RED: 8 failed, 65 passed**: six renderer tests and filter helper at unimplemented entry points, plus placeholder count 14 instead of 8. An initial test DTO field typo (`item_id` instead of `target_id`) was corrected before this behavioral RED run. |
| Same bin command after implementation | **GREEN: 73 passed** |
| Content filter after implementation, before fixture generation | **5 passed, 6 failed** solely on missing new fixture files |
| `MANYHANDS_UPDATE_GOLDEN=1 devenv shell -- cargo test --locked --test cli content::` | **11 passed**, 33 filtered; generated **7 JSON + 2 text** fixtures once |
| `devenv shell -- cargo test --locked --bin manyhands-cli --test cli --test mutation_contract`, update variable absent | **73 + 44 + 43 = 160 passed**, zero failed/ignored/filtered |
| `devenv shell -- cargo clippy --locked --bin manyhands-cli --test cli --test mutation_contract -- -D warnings` | **PASS** |
| `devenv shell -- cargo fmt --check` | Formatting-only differences corrected via apply_patch; final **PASS** |
| `git diff --no-index` for each new JSON fixture against its corresponding `read_v1` file | All **7 byte-identical**, exit 0 |
| `git diff --check` | **PASS** |
| `git merge-tree --write-tree HEAD origin/main` before checkpoint | Exit 0, tree `9c36360e1a8e64ea585de3717a3a730cdb478e39`; covers committed base before the Task 5 additions |

Read every generated fixture, then verified without update mode. CLI fixtures
now total **43 JSON + 4 text**. Existing goldens are unchanged. An intermediate
parallel Devenv invocation reported a `.devenv/load-exports` cleanup/startup
race; Cargo still ran, but subsequent Devenv runs were serialized and all final
commands completed cleanly. No source change was needed for that harness issue.

**Self-review.** Reviewed the complete Task 5 production/test diff against all
four plan steps and the controller's additional constraints, including exact
F1 scopes, repository-first precedence, narrow-filter safety, body/source
preservation and metadata escaping. The only existing-test behavior changes
are the C1-authored placeholder tests shrinking to eight, as the plan requires.
No library, `src/repository/`, schema, dependency, lockfile or pre-C1 semantic
test change; no library gap. Spec/quality self-review **PASS**, no unresolved
findings. Independent controller review remains pending; native platform
behavior is not claimed.

Canonical checkpoint comment:
`.manyhands/comments/01M4CC0VMSZ7BRS740C8VSQ5ZK/01M5A4ZYP8BF57QZCFQ5XXVJWF.md`.
Endpoint is the commit introducing this entry, returned after creation to avoid
a self-referential SHA. Commit subject: `feat: CLI document, ticket and comment
reads`, with the owner-required co-author trailer. Task 6 remains unstarted;
ticket stays open. No subagents, rebase, push, PR, merge into main, closure or
cleanup.
