---
title: "Wave 03 C3 Authoring And Discovery Execution Ledger"
date: 2026-10-25
status: "in-progress"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5RD6N5PEYMVGZNZ2HGPSHKD"
---

# Wave 03 C3 Authoring And Discovery Execution Ledger

## Baseline

Task 0 only, in ticket worktree
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMV81DFRGJ4Z76489XM`,
on `manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`. No subagents dispatched.
Task 0 is complete for independent review; Tasks 1–9 remain unstarted.
Actual verification date: 2026-10-25 UTC;
the filename retains the approved plan's date.

- Approved documents: `e7b2b71`; handoff/base HEAD:
  `ad4d3f0d9a255125d93fe25b4e060cb7c29d2d4a`.
- `git worktree list --porcelain`, branch, status and the last three commits
  confirmed the required worktree and branch. Ticket and main checkout were
  clean; main status was inspected read-only from the ticket workdir.
- Initial and resumed `git fetch origin` succeeded. `git merge origin/main`
  answered `Already up to date.` Both before/after HEADs remain
  `ad4d3f0d9a255125d93fe25b4e060cb7c29d2d4a`.
- `origin/main`: `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`.
  Its `AGENTS.md` was read with `git show origin/main:AGENTS.md`.
- `git merge-base --is-ancestor origin/main HEAD` passed.
  `git merge-tree --write-tree HEAD origin/main` succeeded, tree
  `6fe57e5bebadc7fdc141d565cd29335811f26946`.
- The three prescribed `git log --oneline origin/main --grep <ULID>`
  searches returned no matches for `01M4PB82G07RZSQMNJZN672S3W`,
  `01M4PB80T7SVB23TQ6466RDHCB` and `01M4PB7YZMBFAFFM94AZY08HWS`.

### Baseline checks

All commands ran from this worktree through Devenv, before any tracked edit.

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed. The first invocation was terminated by the shell's 120-second timeout during dependency compilation, with no compiler error. `pgrep -a -x cargo` then returned no process. A second invocation, with a longer timeout, completed in 19.49 seconds. |
| `devenv shell -- cargo fmt --check` | Passed. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed, 1m 42s. |
| `devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract` | Passed: 172 CLI unit + 177 CLI integration + 71 read-contract + 44 mutation-contract = **464**; zero failed, ignored, measured or filtered. |

The combined fmt/clippy/test tool transcript is
`tool_170d1b4a7001Sz6wPttnKvWHXW` in the harness tool-output directory.
These are Linux baseline checks, not C3 acceptance or native CI. Other tests
cited below were inspected as source, not executed.

The test build supplied `target/debug/manyhands-cli`. Two invocations of
`./target/debug/manyhands-cli id new` allocated this document's ID and
comment `01M5RD6N6KZYYQBQMGPNXR3B66`. No separate build was needed.

## Confirmed facts

All **37** design baseline facts were rechecked: **34 confirmed** (with the
scope qualifications stated below), **1 previously unknown fact resolved**
(17), and **2 corrected with explicit owner approval** (19 and 21).
Library paths abbreviated `mutation/`, `read/`, `discovery.rs` and
`recovery.rs` are under `src/repository/`; `bind/` abbreviates
`src/repository/mutation/bind/`. Bare filenames in grouped citations retain
the last named directory. CLI paths are under
`src/bin/manyhands-cli/`. Citations refer to the implementation baseline,
not line positions shifted by these documentation edits. “Confirmed” is
source inspection unless the baseline test run is explicitly named.

| Fact | Outcome | Evidence and conclusion |
| --- | --- | --- |
| 1 | Confirmed | All eleven variants at `mutation/mod.rs:398–428`, input re-exports at `:76–94`, and public repository re-export at `src/repository.rs:83`. |
| 2 | Confirmed | Input fields/types match `mutation/bind/document.rs:47–82`, `ticket.rs:44–67,948–955`, `folder.rs:58–61`, `repair.rs:100–123`, `index.rs:36–45`. Document save/move None observation is refused at `document.rs:341–347`. Full signatures and field types are in the refreshed audit. |
| 3 | Confirmed | Complete drafts derive Clone only; relationship writes default to Unchanged (`src/repository.rs:578–620`). No library patch input exists; the approved CLI fill remains necessary. |
| 4 | Confirmed | Only DocumentRepair/TicketRepair among C3 mutations need confirmation (`mutation/command.rs:436–474`). |
| 5 | Confirmed | `mutation/bind/save.rs:89–116` checks kind, source, observation, parse and parsed ID, returning item_not_found for these failures. |
| 6 | Confirmed | `read/items.rs:1963–1995,2115–2219` checks root/active context and canonical/listed-nonconforming path; no owner-item restriction. `mutation/bind/document.rs:556–566` compares the same context token; repair reads primary (`repair.rs:350–359`). |
| 7 | Confirmed | `ProposedRelationships` has typed complete deps/parent (`read/items.rs:107–116`); the read checks exactly that proposal (`:1998–2054`). Rejection is data in a successful read. |
| 8 | Confirmed, qualified | Conforming title/body are Some, including an empty body (`read/items.rs:1110–1154`); optional non-string project/team is invalid (`src/canonical.rs:1106–1122`). An already-indexed file that becomes nonconforming can return a bare DTO (`items.rs:1812–1822`) rather than directly failing. The approved fill step 4 already converts that unfillable DTO to item_not_found; no task change. |
| 9 | Confirmed for saves | Unknown values and exact body survive serialization, with front matter re-emitted (`src/canonical.rs:911–980,1026–1063`; order test `tests/canonical_foundation.rs:449–461`). Documents write their destination (`src/repository.rs:2265–2296`); tickets compare parsed items (`:2646–2678`). This save rule is not a claim that repair normalizes its input: repair writes source verbatim (`mutation/bind/repair.rs:579–582`). |
| 10 | Confirmed | Index success has no data, code ok and discovery current (`mutation/bind/index.rs:80–88`). Execute restores an unavailable index first (`mutation/execute.rs:83–141`). Rebuild root selection has the separately approved exception below. |
| 11 | Confirmed | Identity refusal deletes the save record and retains its uncommitted context; retry uses a fresh observation (`tests/mutation_replay/execute.rs:1100–1182`). Tokens include branch name (`read/items.rs:1187–1217`). |
| 12 | Confirmed | Per-service failure hook at `src/repository.rs:4490`, wrapped by `tests/support/mod.rs:277–292`. Actor helpers are `tests/support/mutation.rs:487–534`; scoped CLI actor precedent at `tests/cli/keys.rs:743–762`. |
| 13 | Confirmed | Listed code classes match `src/results.rs:537–661`. Envelope partial precedence is separate (`:273–288`); see fact 19. A class listing is not a claim that each C3 verb reaches every listed code. |
| 14 | Confirmed | Document occupied path at `mutation/bind/document.rs:415–446`; folder occupied/invalid/no-op at `folder.rs:100–177`. |
| 15 | Confirmed | Move-to-self uses only_at over the document branch and the committed-state rule (`mutation/bind/document.rs:512–553`). No document branch means external_change, not a primary-only no-op. |
| 16 | Confirmed | Matching composed and committed slug is already-applied; a different held code is slug_already_assigned (`mutation/bind/ticket.rs:1240–1290`). |
| 17 | Resolved | A current-token save with no context provisions one before content comparison/checkpointing (`mutation/bind/document.rs:489–503`, `ticket.rs:601–634`; `src/repository.rs:2034–2065,2453–2492,2646–2678`). It can therefore create a context without adding a commit. An early stale-token already-applied answer need not provision one. Existing planned no-op fixtures already use contextual items. Source-derived, not a new context-specific test run. |
| 18 | Confirmed | Filename `src/repository.rs:112`, WAL `discovery.rs:1539`; missing-registration read has no recovery (`read/resolve.rs:68–80,228–240`), mutation distinguishes never-enabled and offers rebuild for lost registration (`mutation/bind/mod.rs:93–123`, `outcome.rs:804–807`). Structural corruption detects non-database bytes (`src/repository.rs:5607–5621`). F2 removes sidecars but keeps locks (`tests/mutation_replay/index.rs:344–352`). |
| 19 | Corrected, approved | Hook/result are correct (`tests/mutation_bridges/repair.rs:650–678`), but partial internal_error maps to exit 4 (`src/results.rs:273–288`, `render.rs:85–90`). Shared execute re-entry uses recorded consent, not the newly presented one (`mutation/execute.rs:190–208`, `reenter.rs:159–174`). Repair Shelf fixture remains available in tests/support/repair.rs. |
| 20 | Confirmed | Index stores title (`discovery.rs:1623–1629`). Privacy scans only the eight request stores (`tests/support/mutation.rs:73–82,107–157`). |
| 21 | Corrected, approved | Pending fence applies to requests reaching a new domain operation (`recovery.rs:1366–1392`), not early no-ops/refusals or finished replay (`mutation/execute.rs:190–208,254–258`; `tests/mutation_replay/documents.rs:573–614`). Partial save offers operation.resume (`tests/mutation_replay/execute.rs:552–583`); driver retries only request.retry (`mutate.rs:74–83`). Index-pending refresh maps to busy (`mutation/outcome.rs:245–253`). Re-entered content accepts intended/original bytes only (`src/repository.rs:8194–8222`, called at `:2218–2226,2636–2644`). Task 3's retry expectations are unchanged. |
| 22 | Confirmed | Child lease holder `tests/support/mod.rs:477–504`; C2 binary busy fixture `tests/cli/exits.rs:513`. The baseline CLI target passed. |
| 23 | Confirmed | Thirteen planned C3 entries (`catalog.rs:662–679`), helper defaults (`:191–224`), mutation prompts (`:57–59`), mutation options (`:227`) and literal C2 catalog entries, including repo remove, unchanged. |
| 24 | Confirmed | C2-only clap suppression (`parse.rs:113–125`), required flags/context error (`:191–206`), last repeated value (`parse_tests.rs:310–341`, passed in baseline). Positional clap text remains unredacted outside C2, so the approved C3 privacy change is still required. |
| 25 | Confirmed | Boundary/retry delays (`mutate.rs:40–83`), build per prepare/attempt (`:120–252`), refusal empty scope (`:273–301`). No change to C2 driver required. |
| 26 | Confirmed | `InputObject` methods stop at one_of, with no list accessor; missing nullable string refused (`input.rs:110–181`). |
| 27 | Confirmed | Unhandled available dispatch is internal_error (`dispatch.rs:678–692`); wrong-kind show is item-scoped for ID selection (`:314–333`). |
| 28 | Confirmed | C3 mutation DTOs contain no title/status/body; success prints message, metadata/effects and DTO fields (`human_admin.rs:139–161`); Repair preview at `:62`. Note helpers private (`human.rs:24,30`), field escaping pub(super) (`render.rs:158`). |
| 29 | Confirmed | Help command DTO has no kind (`startup.rs:140–147`). C3 mutation entries will expose request-id through their options. |
| 30 | Confirmed | C2 exit test reads its own heading (`tests/cli/exits.rs:601–608`); C2 inventories filter delivery (`tests/cli/human.rs:50`, `privacy.rs:413–455`), C1 inventories filter C1 (`dispatch_tests.rs:58–62`, `tests/cli/boundaries.rs:175`). |
| 31 | Confirmed | Existing native command includes the CLI bin, cli and cli_remote (`.github/workflows/build.yml:115`). No dispatch performed. |
| 32 | Confirmed, search exception approved | Three E1 goldens and DESIGN row found by the exact first search. Unfiltered tests are `startup_tests.rs:181–216,269–313,318–388`, `catalog_tests.rs:130–160`, `parse_tests.rs:212–234`; their logic accommodates C3 unchanged. The owner additionally permits catalog_tests.rs:262–267 as a local literal, not an edit. All 57 second-search hits classified below. |
| 33 | Confirmed | Fixture disables autocrlf and rejects a system identity (`tests/cli/repository.rs:14–30,50–54`). Baseline identity cases passed on this machine. |
| 34 | Confirmed at inspection | `git show manyhands/ticket/<id>:.manyhands/tickets/<id>/ticket.md` for all eight fact-34 IDs showed open, quoted managed ticket metadata, no deps/parent/slug and the stated wave/cycle keys (gate has wave only). They reside on their own branches; no dogfooding save performed. |
| 35 | Confirmed at inspection | Read-only status of all eight worktrees was clean. G1 remained at 7b306d1. D3 independently advanced to 98f49b7, a main merge; its ticket metadata remained unchanged. This is not a Task 9 preflight and does not authorize later saves. |
| 36 | Confirmed at inspection | `git for-each-ref` showed no remote/upstream for the six save targets; C3 and gate have origin refs. `git log origin/main..<branch>` gave C4 2, C5 2, D4 1, D5 3, D6 1, G1 1 unpublished commits. These old target branches remain behind main. |
| 37 | Confirmed | Public path/slug/relationship fields are in `mutation/dto.rs:556–565,906–945`; human output prints all DTO fields (`human_admin.rs:139–161`). The approved privacy exemption remains necessary. |

### Other mandatory Task 0 resolutions

- **Initials:** a new create that cannot compose initials is refused before
  acceptance (`mutation/bind/ticket.rs:491–524,582–598`; `execute.rs:249–258`).
  No record exists to delete; the ID is free. The source-inspected test
  `tests/mutation_replay/execute.rs:152–198` asserts nothing exists and reuses
  that ID with changed initials.
- **Committed save/cache loss:** before rebuild, repository_not_registered;
  the F2 answer offers index.rebuild. After rebuild, unchanged intended
  content gives already_applied/noop and no second commit. Ticket fixture:
  `tests/mutation_replay/cache_loss.rs:23–49,115–132`. Document code follows
  `mutation/bind/document.rs:489–503` and shared `bind/save.rs:124–139`;
  its stale-token no-op is asserted at `tests/mutation_replay/documents.rs:249–286`.
  Document cache-loss is source-derived, not claimed as an executed fixture.
  Both C3 binary cache-loss tests remain owed. Decision 7's fill-read recovery
  difference remains unchanged.
- **Confirmation/cache loss:** `tests/mutation_confirmation.rs:304–331`
  asserts confirmation_not_found and request.prepare. Shared consent lookup
  precedes repair preparation (`mutation/execute.rs:249–254`,
  `confirm.rs:399–410`); no library call can recover lost consent from Git.
- **Repair:** invalid corrected source is invalid_input. Non-place path or
  a conforming ticket source with another directory ID is not_repairable
  (`bridges/repair.rs:146–179`, `mutation/bind/repair.rs:193–198`). A conforming
  current file is not_repairable; an absent/exact-case-missing file is
  path_not_found; an invalid/symlink path is invalid_path (`bind/repair.rs:18–45,350–371,446–464`).
  The audit distinguishes these cases instead of inventing one code for all
  paths lacking a nonconforming entry.

### Resolved ruling: fact 19 partial-outcome exit precedence

The originally approved design, Baseline Fact 19 (`:201–204`), said the actor stopped at
`AfterOwnedWriteBeforeLifecyclePersistence` answers `internal_error`, partial,
**exit 1**. The cited test actually asserts:

- `ResultCode::InternalError` (`tests/mutation_bridges/repair.rs:663`);
- `Outcome::Partial` (`:664`);
- written content and a pending checkpoint (`:665–666`).

`Envelope::failure_class()` explicitly maps every `Outcome::Partial` to
`FailureClass::Incomplete`, regardless of code (`src/results.rs:273–288`).
The CLI uses that envelope classification (`src/bin/manyhands-cli/render.rs:29`)
and maps `Incomplete` to **4** (`:85–90`). The actor itself has no process exit;
4 is the exit the envelope would produce through the CLI renderer.

The owner explicitly approved correcting fact 19 to exit 4, retaining
`internal_error` and `partial`, and using that classification for C3 repair
and exit evidence. The narrow design correction is applied. A search of the
C3 documents found no directly dependent inaccurate exit claim: the Task 6
reference describes a partial `internal_error` without stating exit 1.
No behavior or test changed. The earlier catalog-test exception also remains
approved.

### Resolved ruling: index rebuild root-handling exception

Found on the resumed source audit, 2026-10-25T02:22:01Z. The design's Grammar
And Per-Verb Rules originally said, for mutation `--repo`, that “The library refuses a
linked worktree's root with `not_repository_root`” (baseline lines 395–397).
The rebuild binding has a deliberate exception to ordinary root resolution:

1. `mutation/bind/mod.rs:93–103` resolves a linked root to its owner and
   returns `NotRepositoryRoot` when the supplied canonical path differs.
2. `mutation/bind/index.rs:213–233` propagates this for refresh, but rebuild
   propagates only `RepositoryNotEnabled`. A `NotRepositoryRoot` answer is
   discarded. The comments explicitly say rebuild asks for the root itself.
3. `mutation/bind/index.rs:148–151` then calls `rebuild_repository` with the
   accepted root. `repository.rs:1533–1544` uses
   `canonical_repository_root`, whose `9570–9620` implementation accepts the
   selected path when it equals that Git repository's own `workdir()`. It
   has no linked-worktree-owner rejection. An inside-root path is instead
   `InvalidPath` (`9611–9617`).
4. The rebuild re-entry has the same `RepositoryNotEnabled`-only check
   (`mutation/bind/index.rs:278–285`). The unavailable-index restoration
   path also uses `canonical_repository_root` (`repository.rs:1647–1656`).

This is source evidence of a missing promised refusal, not a claim that a
linked-root rebuild was executed against this repository. No dogfooding
service mutation was run. Adding a CLI root guard would depart from the
approved one-call adapter; changing the library is outside this task.

The owner approved documenting and passing through `index rebuild`'s existing
root-selection exception, rather than promising `not_repository_root` for
that verb. The design grammar now narrowly qualifies the rule and explicitly
distinguishes source-inspected behavior from fixture-proven success. This
does not promise success for every linked-root situation. No library or CLI
workaround was added. The API audit refresh carries the same qualification.

### Resolved ruling: fact 21's pending-request fence scope

Found during the no-op/code audit, 2026-10-25T02:42:41Z. Fact 21 and the
interrupted-saves required-outcomes table originally said **every other request** on the
repository answers `recovery_required` while the save is pending. The fence
is a domain-operation check; the mutation boundary can answer earlier.

Direct source-inspected counterexample:

- `tests/mutation_replay/documents.rs:573–614`,
  `a_create_s_own_file_in_a_context_the_index_has_not_seen_is_not_an_occupied_path`,
  stops document creation at `BeforeIndexTransactionCommit`. It asserts the
  first answer is partial (`:585–596`), the index does not hold the document
  (`:597–603`), and then submits the same creation under **REQUEST_2**
  (`:606–609`). The required second answer is `Noop`/`AlreadyApplied`
  (`:585–587,610`), with no operation, no request record and an unchanged
  branch tip (`:611–614`). The test source was inspected, not newly run.
- The mechanism is `bind/document.rs:421–443`: intended content committed
  in the deterministic context is answered by the stale-token/already-applied
  rule before any domain call. `mutation/execute.rs:254–258` returns this
  binding answer before accepting a request. Thus it does not ask whether
  another operation is pending. This applies to an already-applied create
  during a pending save as well: the binding does not inspect the pending
  operation or distinguish its action.
- Finished-request replay likewise happens before binding/domain execution
  (`mutation/execute.rs:190–208`; `mutation/replay.rs:19–53`). A reachable
  finished result is returned without entering a new domain operation.
- The recovery fence itself remains as inspected in
  `src/repository/recovery.rs:1366–1392`: a different operation reaching that
  check is refused. The existing test of another create that needs new work
  asserts `recovery_required` (`tests/mutation_replay/resume.rs:1337–1359`).

The owner approved limiting the fence to requests reaching a new domain
operation, preserving early no-op/refusal and finished-request replay.
Task 3's blocked-request fixtures request actual new work. Its interrupted-save
retry expectations are unchanged. Narrow updates are applied to Fact 21,
the directly dependent Cycle/design wording and table row, and Task 3 Step 0
in the plan. No test, behavior or library change was made. This resolves the
last stop; no owner question remains open in Task 0.

### Additional evidence gathered on the resumed audit

- Facts 12, 20 and 22: `FailOnce` opens the in-process failure-point service
  (`tests/support/mod.rs:277–292`); actors call execute directly
  (`tests/support/mutation.rs:487–534`). The request privacy scan uses its
  eight named stores (`:73–82,107–157`), not `discovered_items`, whose title
  column exists (`src/repository/discovery.rs:1623–1629`). The lease holder
  is a child process (`tests/support/mod.rs:477–504`) used by the binary exit
  fixture (`tests/cli/exits.rs:513`).
- Facts 27–29 and 37: fallback dispatch is internal error and wrong-kind
  show has the item scope (`dispatch.rs:314–333,678–692`). Human mutation
  output prints the message, metadata/effects and DTO fields
  (`human_admin.rs:139–161`); Repair preview is handled at `:62`. Document
  path and ticket slug/relationship IDs are DTO fields
  (`mutation/dto.rs:906–945`), consistent with the privacy exemption.
  Human note helpers remain private (`human.rs:24,30`) and `render::field`
  is `pub(super)` (`render.rs:158`).
- Facts 30–33: C2's exit equality test reads only its own audit heading
  (`tests/cli/exits.rs:601–608`). The native workflow includes the existing
  CLI bin, cli and cli_remote targets (`.github/workflows/build.yml:115`).
  The fixture disables autocrlf and rejects a system identity
  (`tests/cli/repository.rs:14–30,50–54`). The source-inspected unfiltered
  tests match fact 32 (`startup_tests.rs:181–216,269–313,318–388`,
  `catalog_tests.rs:130–160`, `parse_tests.rs:212–234`). C1 minimum-option
  inventory is filtered (`parse_tests.rs:95–106`); the DESIGN comparison is
  `catalog_tests.rs:106–127`. The admin-test hits are assertions about
  explicitly selected C2 verbs (`admin_tests.rs:7–9,52–55,104–107,140–178`),
  not a C3-sensitive inventory.

These notes supplement the completed 37-row audit above. Baseline Rust
checks were not repeated: subsequent edits are documentation only.

## Audit refresh

Completed in `docs/research/wave-03-api-audit.md`, under
`Wave 03 CLI Cycle C3 refresh (2026-10-24)`. Thirteen inventory rows updated
in place; exact F1/F2 signatures, all input types, confirmation requirements
and result sets are recorded with binding/domain/boundary/read citations.
The single C3 union block contains **40 unique library codes**, **42** after
the design's CLI_ONLY union. No C2 audit union or pre-existing test changed.

The eleven-mutation no-op inventory has **9 capable**, **2 incapable**
(index refresh/rebuild always answer ok for successful explicit work).
It distinguishes a fresh request from replay, contextual no-ops from context
provisioning, and no canonical change from cache/same-byte filesystem writes.
The root-handling and pending-fence rulings, partial exit precedence,
initials recovery, cache-loss limits and repair refusal distinctions are
recorded there. Source-derived cases are not labeled fixture-proven.

## Decisions 1–7, rulings R-1–R-18 and E1

The approved Cycle's decisions and design rulings remain the authority,
with only the explicit Task 0 owner qualifications recorded here.

| Decision | Retained requirement |
| --- | --- |
| 1 | Three library defects are known limits, not prerequisites; no fixes in C3. |
| 2 | CLI fills omitted save fields, accepting the documented replay limit. |
| 3 | Dogfooding is final-task work with a separate owner go-ahead; none performed. |
| 4 | One Cycle in two Parts, with all-feature gates and independent review. |
| 5 | Fill-read failures retain their code, scope and recovery, with the request ID. |
| 6 | Exit 4 has reason/library-actor evidence; exit 5 must be reached through the binary. |
| 7 | Accept and document missing-registration recovery differences; optional library ticket not filed. |

| Ruling | Retained requirement |
| --- | --- |
| R-1 | Literal C3 catalog entries become available only with their dispatch. |
| R-2 | Expand usage privacy beyond C1; positional sentinel test. |
| R-3 | Two list readers, without deduplication. |
| R-4 | CLI-owned save fill with the approved refusal and replay rules. |
| R-5 | Approved optional/nullable fields and create defaults. |
| R-6 | Complete ticket-check proposal; rejection data still exits 0. |
| R-7 | Path observation reports what exists in the named context. |
| R-8 | Unconfirmed explicit index mutations; request IDs required in JSON. |
| R-9 | Identity recovery passes through without prompting, with a fresh observation. |
| R-10 | Separate C3 exit table, exact set equality with this audit's union. |
| R-11 | In-process library actors for interruptions; CLI retries exercise fill/consent. |
| R-12 | Deleted/whole-file-corrupt index evidence only; WAL files removed first. |
| R-13 | Example inputs are schema-validated and exercised through the binary. |
| R-14 | Fresh-request no-op inventory on contextual items; reasons for incapable verbs. |
| R-15 | New tests stay modules of the existing cli target. |
| R-16 | Dogfooding preflight, go-ahead, clean/value checks, stop/resume and own review. |
| R-17 | Body/source privacy; request-store scan; approved public data echoes exempt. |
| R-18 | Isolated children and no-system-identity test machine. |

E1 remains exactly the three help goldens, changed only as verbs are
delivered in later tasks. Task 0 changes none. The additional search literal
exception below is not another named edit.

### Additional owner ruling during Task 0

The first prescribed grep returned exactly five files: production catalog,
catalog tests, and the three E1 help goldens. The second grep additionally
found a test-local `CommandSpec` at
`src/bin/manyhands-cli/catalog_tests.rs:262–267`, outside the named test-local
exceptions. Task 0 stopped without edits or pushes.

The owner answered **YES** to the precise ruling: add those lines as a
permitted test-local `CommandSpec` search hit; leave the test unchanged and
continue Task 0. This is an additional search classification, not a new
pre-existing-test edit. E1 remains the only authorized pre-existing test change.

### E1 search evidence

Both exact prescribed grep commands ran against the unchanged baseline:

```sh
grep -rln -E 'path observe|ticket check|index (refresh|rebuild)|folder create|document (create|save|move|repair)|ticket (create|save|repair|slug-assign)|"C3"' src/bin/manyhands-cli tests/cli tests/cli_remote tests/cli.rs tests/support/cli.rs tests/fixtures/cli_v1
grep -rn -E '\bAvailability::(Available|Planned)|available.*len\(\)|"available"\] == true' src/bin/manyhands-cli tests/cli
```

The first returned exactly catalog.rs, catalog_tests.rs and
`tests/fixtures/cli_v1/{help_root.json,help_root_text.txt,help_resource_ticket.json}`.
The second has **57 matching lines**, classified as follows (counts confirmed
with `rg --count`, not a new test run):

| Category | Hits and explanation |
| --- | --- |
| Production | catalog.rs 21, parse.rs 4, startup.rs 6: declaration/dispatch/help logic. |
| C1/C2-scoped tests | parse_tests.rs:102 (C1 filter); dispatch_tests.rs:59 (C1 filter); catalog_tests.rs:257 (DESIGN[..2], C1 check); admin_tests.rs:9,55,107,178 (explicit C2 verb selections); tests/cli/boundaries.rs:175 (C1); human.rs:50 (C2); privacy.rs:413,455 (C2 filter and its 18-count). |
| Fact 32 unfiltered tests | parse_tests.rs:216; startup_tests.rs:184,272,332,371; catalog_tests.rs:137,155. Each naturally includes or excludes a newly available C3 verb as intended. |
| DESIGN comparison | catalog_tests.rs:120–121 compares delivering Cycle for either availability. |
| Permitted local literals | parse_tests.rs:606,670; startup_tests.rs:477,635; mutate_tests.rs:13. |
| Owner-approved additional local literal | catalog_tests.rs:262 constructs a local CommandSpec at :263–267; unchanged. |

No unclassified hit remains. No pre-existing test or golden requires a Task 0
edit. Independent review must still check E1 diffs as later verbs ship.

## Planning review record

The Cycle Review Record retains three independent planning-review rounds:
38 contract-lens and 22 source-lens findings in round 1; 22 scoped/adversarial
findings in round 2; 16 in round 3. Decisions 1–7 and R-1–R-18 were approved
with the documents at `e7b2b71`. Round-3 edits had no further independent
review. The optional decision-7 library ticket was not filed. No Task 0
independent review has occurred; the controller must obtain it before Task 1.

## Task 0 self-review and checkpoint

Completed 2026-10-25T02:54:18Z. Scope reviewed: the original audit, ledger and
canonical comment plus only the owner-authorized Cycle/design/plan corrections.
All four stops were returned to the owner and resumed on explicit approval.
No unresolved blocker remains. The source/test/Cargo tree is unchanged from
the verified baseline. No subagents or later task work were started.

Documentation checks: 13 C3 inventory rows counted, 37 individually numbered
fact records, 57 classified search hits, 11 no-op rows. Expanding the audit's
13 row expressions independently gives exactly its 40-code block with no
missing/extra code; adding CLI_ONLY gives 42. The tracked diff whitespace
check passed. No repeated Rust gates were needed for these documentation edits.
Read-only behind/ahead counts additionally confirmed all six dogfooding save
targets are 865 commits behind origin/main and retain the unpublished counts
in fact 36. These observations do not replace Task 9's fresh preflight.

The Task 0 checkpoint is the commit containing this ledger and the canonical
comment. The pre-checkpoint fetch and merge were repeated: origin/main stayed
at `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`, the merge was already current,
and merge-tree was clean. The three defect searches still had no matches.
`git diff --exit-code ad4d3f0 -- src tests Cargo.toml Cargo.lock` confirmed
the source/test/Cargo baseline is unchanged.
Publication is authorized only to `manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`.
Independent spec/quality reviews remain the controller's next step before
Task 1; this is not a whole-Cycle review-ready or closure claim.

## Task log

| Task | State | Commit range | Verification | Review |
| --- | --- | --- | --- | --- |
