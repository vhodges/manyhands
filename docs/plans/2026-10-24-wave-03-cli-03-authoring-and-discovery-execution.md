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

Task 0 baseline, in ticket worktree
`/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMV81DFRGJ4Z76489XM`,
on `manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`. No subagents dispatched.
Tasks 0–3 are approved (see task log; Task 3 approvals conveyed in the Task 4
controller brief). Task 4 is implemented and locally verified for independent
controller Part A reviews. Task 3's two stops are resolved by explicit owner
rulings: Partial/Incomplete assertion precedence, and primary-only committed
move-to-self no-op. The interrupted-save checkpoint also passes through real
CLI retries with its approved assertions unchanged. Tasks 5–9 remain unstarted.
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

All **37** design baseline facts were rechecked during Task 0: 33 confirmed,
1 previously unknown fact resolved (17), and 3 corrected with explicit owner
approval (9, 19 and 21, including the post-quality-review correction to fact 9).
Task 3's binary evidence subsequently corrected fact 15 with owner approval;
the current inventory is **32 confirmed / 1 resolved / 4 corrected**.
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
| 9 | Corrected, approved | Unknown values and exact body survive serialization, with front matter re-emitted (`src/canonical.rs:911–980,1026–1063`; order test `tests/canonical_foundation.rs:449–461`). Documents write their destination (`src/repository.rs:2265–2296`); tickets compare parsed items to decide rewriting, then still checkpoint (`:2646–2691`). Both save no-ops require intended content already committed (HEAD-equal); the checkpoint compares HEAD (`:3561–3590`). Matching uncommitted ticket content can be committed without a rewrite (`tests/mutation_replay/execute.rs:1041–1078`). Quality review found the missing qualification; the owner approved it as ruling 5 below. Repair still writes source verbatim (`mutation/bind/repair.rs:579–582`). |
| 10 | Confirmed | Index success has no data, code ok and discovery current (`mutation/bind/index.rs:80–88`). Execute restores an unavailable index first (`mutation/execute.rs:83–141`). Rebuild root selection has the separately approved exception below. |
| 11 | Confirmed | Identity refusal deletes the save record and retains its uncommitted context; retry uses a fresh observation (`tests/mutation_replay/execute.rs:1100–1182`). Tokens include branch name (`read/items.rs:1187–1217`). |
| 12 | Confirmed | Per-service failure hook at `src/repository.rs:4490`, wrapped by `tests/support/mod.rs:277–292`. Actor helpers are `tests/support/mutation.rs:487–534`; scoped CLI actor precedent at `tests/cli/keys.rs:743–762`. |
| 13 | Confirmed | Listed code classes match `src/results.rs:537–661`. Envelope partial precedence is separate (`:273–288`); see fact 19. A class listing is not a claim that each C3 verb reaches every listed code. |
| 14 | Confirmed | Document occupied path at `mutation/bind/document.rs:415–446`; folder occupied/invalid/no-op at `folder.rs:100–177`. |
| 15 | Corrected during Task 3, owner-approved | Move-to-self uses only_at and the committed-state rule (`mutation/bind/document.rs:512–553`). `Position::of` falls back to primary HEAD when the document branch is absent (`mutation/evidence.rs:19–44`), so a primary-only committed move-to-self also returns already_applied/noop, exit 0, without context creation. Task 3 binary evidence exposed the missing fallback in the earlier audit; the owner approved the narrow correction. |
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
review. The optional decision-7 library ticket was not filed. Task 0's
implementation-review status is recorded below.

## Task 0 review and fifth owner ruling

Recorded 2026-10-25T03:27:38Z, as conveyed by the controller/owner:

- **Spec review:** approved `d9e961df04ee3b338247b0032d5194ecbc805923`.
- **Quality review:** must-fix finding on the audit's document/ticket save
  no-op wording and design Fact 9. Equality with the worktree file was
  incorrectly sufficient for “no new checkpoint.”
- **Verified mechanism:** ticket save conditionally rewrites at
  `src/repository.rs:2646–2678`, then calls the checkpoint at `:2680–2691`.
  The checkpoint compares the resulting tree against HEAD (`:3561–3590`).
  `tests/mutation_replay/execute.rs:1041–1078` explicitly covers committing
  matching uncommitted content without rewriting it. This test was
  source-inspected, not rerun for the documentation correction.
- **Fifth owner ruling:** qualify both save no-ops to require intended
  content already committed (HEAD-equal). Matching uncommitted content may
  skip rewriting yet still checkpoint. No new behavior or test is requested.
- **Resolution:** the audit's two save no-op rows and shared precondition,
  design Fact 9/save-patch row/R-14, directly dependent Cycle wording and
  the plan's existing no-op fixture description now state that condition.
  Fact 9 is marked corrected in the fact inventory. The **9 capable / 2
  incapable** inventory and **40 library / 42 CLI-inclusive codes** are unchanged.
- **Evidence retained:** baseline check/fmt/clippy and **464 tests**; no broad
  rerun for documentation-only edits. Self-review `git diff --check` passed;
  comparison with `d9e961d` confirms src/tests/Cargo unchanged. A separate fix commit follows
  `d9e961d`, with its own diff/whitespace review and ticket-only push.
  Canonical review comment: `01M5RHJZQ672X4JG8VPYVPM3P3` (ID from the built CLI).
- **Final Task 0 approval:** the controller's Task 1 handoff explicitly
  reports quality re-review approval of
  `7368902a677a8228813f21da8f74d3cafff68ec3`, after the fifth owner ruling's
  correction. Recorded during Task 1 on 2026-10-25. Together with spec
  approval of `d9e961df04ee3b338247b0032d5194ecbc805923`, this clears the
  Task 0 review gate. All five owner rulings above remain binding.

## Task 0 self-review and checkpoint

Initial checkpoint `d9e961d`, completed 2026-10-25T02:54:18Z. Scope reviewed:
the original audit, ledger and
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

The initial Task 0 checkpoint is `d9e961df04ee3b338247b0032d5194ecbc805923`.
Its pre-checkpoint fetch and merge were repeated: origin/main stayed
at `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`, the merge was already current,
and merge-tree was clean. The three defect searches still had no matches.
`git diff --exit-code ad4d3f0 -- src tests Cargo.toml Cargo.lock` confirmed
the source/test/Cargo baseline is unchanged.
Publication is authorized only to `manyhands/ticket/01M4CC0VMV81DFRGJ4Z76489XM`.
Current review status is in the review record above; the controller has
reported final Task 0 approval. This is not a whole-Cycle
review-ready or closure claim.

## Task log

| Task | State | Commit range | Verification | Review |
| --- | --- | --- | --- | --- |
| 0 | Complete | `ad4d3f0..7368902` | Baseline gates and 464 tests; documentation-only correction | Spec approved `d9e961d`; quality re-review approved `7368902`, per controller handoff |
| 1 | Complete | `7368902a677a8228813f21da8f74d3cafff68ec3..8ae22ab9b34e52cf3404f74841cc5e1348ad393a` | 177 CLI unit + 10 authoring-read + 9 help = **196 passing**; check/fmt/clippy passed | Spec and quality both APPROVED, explicitly conveyed in the owner's Task 2 brief |
| 2 | Complete | `8ae22ab9b34e52cf3404f74841cc5e1348ad393a..66efd0629d73fa8d762921fd987d143604178175` | 13 index + 7 folders + 7 input-schema + 9 help = **36 passing**; check/fmt/clippy passed | Spec and quality both APPROVED, explicitly conveyed in the Task 3 Step 0 brief |
| 3 Step 0 | Complete; binary conversion passed | Base `66efd0629d73fa8d762921fd987d143604178175`; included in Task 3 completion | Exact checkpoint filter initially **1 passed / 0 failed / 201 filtered**, 5.72s; all five rows, six independent fixtures; passes again inside final documents filter through real CLI children | Owner approved correcting only extra failure-class assertions to partial/Incomplete precedence; table expectations unchanged |
| 3 | Complete | `66efd0629d73fa8d762921fd987d143604178175..90c8e3ae15f9085c74b283c95bc557d60a7abc00` | **27 document + 7 input-schema + 4 fill + 9 help = 47 passing**; all-feature check, all-target/all-feature clippy, fmt check passed | Spec and quality both **APPROVED**, explicitly conveyed in the controller's Task 4 brief; prior owner rulings retained |
| 4 | Implemented; Part A independent review pending | Base `90c8e3ae15f9085c74b283c95bc557d60a7abc00`; head is the `test(cli): C3 Part A exit table and inventories` commit carrying this entry | **10 new focused tests passed**; all six checkpoint gates passed; **182 CLI unit + 238 CLI integration + 71 read-contract + 44 mutation-contract = 535 passing** | Self-reviewed only; controller independently reviews actual Part A range `7368902a677a8228813f21da8f74d3cafff68ec3..Task4HEAD` after return, then records the returned SHA and review outcome |
| 4 P2 follow-up | Implemented; re-review pending | Base/initial Task 4 head `329a1436f3671586382ca4c1f41b2c9a8ab8a699`; follow-up head is the `test(cli): prove C3 index-unavailable exit through refresh` commit carrying this entry | Expected red C1-command attribution failure; **11 authoring-exits passed**; all six checkpoint gates passed, **182 + 239 + 71 + 44 = 536** | Part A SPEC P2 corrected with a new Task 4 binary test; independent acceptance remains with controller |

## Task 1: path observe and ticket check

Checkpoint recorded **2026-10-25T03:36:25Z**. Fresh implementer worked only
in the ticket worktree/branch; no subagents. The supplied Task 1 specification
was the execution brief. Task 2 was not started.

### Preflight and authority

- Initial clean HEAD was `7368902a677a8228813f21da8f74d3cafff68ec3`.
  Fetched origin, read `git show origin/main:AGENTS.md`, and merged
  `origin/main`: already up to date; HEAD unchanged.
- Read the handoff, ticket/comments, Cycle/design and this ledger's five
  approved corrections. Final Task 0 quality approval is recorded above.
- Before this checkpoint, fetched again. `origin/main` remains
  `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`; its ancestry check passed.
  `git merge-tree --write-tree HEAD origin/main` succeeded with tree
  `2d628583a17bdf4d0e006477e7359e6be8b5d693` (committed task base).

### Implementation and red/green evidence

1. Added `a_c3_verbs_usage_error_never_echoes_a_value`, then made
   `ticket check` available before the red run, as required to bypass
   planned-command argument swallowing. The exact prescribed no-feature
   command failed **0 passed / 1 failed / 172 filtered**, printing
   `unexpected argument 'SECRET-VALUE' found`.
2. Regenerated E1 with only `ticket check` made available. The help filter
   passed **9** tests. Read its diff: only that verb's availability,
   repository rule and options changed in the three authorized fixtures.
3. Made `path observe` available; changed the usage mask from C2-only to
   non-C1, and added the ticket-check proposal requirement. The same privacy
   command passed **1 / 172 filtered**. Regenerated E1 again, adding only
   path observe's metadata. Help again passed **9**. Both summaries and
   `delivered_by: C3` are preserved; no other old fixture changed.
4. Added ten real-binary cases before dispatch/formatters. The authoring-read
   filter failed **0 passed / 10 failed / 177 filtered**, all on the expected
   missing dispatch (`internal_error`, exit 1). Added the two thin F1 adapters
   and escaped human formatters, with four formatter unit cases.
5. The new fixtures were generated only with `MANYHANDS_UPDATE_GOLDEN=1`.
   All ten authoring-read cases passed during generation and again without
   the update variable. Reviewed all four new JSON and two new text fixtures;
   only roots, refresh times and HEAD OIDs are substituted. Observation
   tokens and relationship/rejection IDs remain exact.

The read harness uses `CliEnv` and its detached terminal/config/data isolation,
with test-home `core.autocrlf=false` and no home identity. Every invocation,
including errors and human mode, snapshots root and linked-worktree bytes/state
and runs isolated `git for-each-ref` before/after. All comparisons passed.
The cases cover absent/file tokens, another item's file with a relative context,
inactive context and escaping paths, control escaping, dependency and parent
cycles, unresolved IDs, complete-proposal semantics, malformed/empty IDs, usage
refusal, scopes, schemas, human goldens and empty JSON stderr. Rejections remain
successful reads with exit 0. Incomplete/stale/never-refreshed notes reuse C1's
helpers and are covered by the new formatter tests.

### Final verification

All commands ran through Devenv in this worktree; no broad/background suite.

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --bin manyhands-cli` | **177 passed**, 0 failed/ignored/filtered (172 existing + 5 new). |
| `devenv shell -- cargo test --all-features --locked --test cli authoring_reads` | **10 passed**, 0 failed/ignored, 177 filtered. |
| `devenv shell -- cargo test --all-features --locked --test cli help` | **9 passed**, 0 failed/ignored, 178 filtered. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed. |
| `devenv shell -- cargo check --all-features --locked` | Passed. |
| `devenv shell -- cargo fmt --check` | Passed, after `devenv shell -- cargo fmt`. |

### Files and self-review

- Created `src/bin/manyhands-cli/human_authoring.rs`, its adjacent
  `human_authoring_tests.rs`, and `tests/cli/authoring_reads.rs`.
- Changed only Task 1's named existing Rust files: binary module declaration,
  catalog, parser, dispatch, two human-helper visibilities, appended parser
  test, integration module registration and golden-name registration.
- Created `tests/fixtures/cli_v1/{path_observe_ok,path_observe_invalid_path,
  ticket_check_ok,ticket_check_invalid_id}.json` and
  `{path_observe_text,ticket_check_text}.txt`.
- E1 only: `help_root.json`, `help_root_text.txt`, `help_resource_ticket.json`.
- Updated this ledger and canonical comment `01M5RJ31V8TA9KFF44GJZD77H6`.
  Its ID came from `./target/debug/manyhands-cli id new`, timestamp from
  `date -u`; no dogfooding save was performed.

Self-review checked the thin adapters against the specified F1 calls, scope
construction and privacy, the test coverage above, all golden content, and the
complete task diff. `git diff --check` passed. Existing tests are unchanged
apart from the authorized E1 fixtures; source-test files only add new tests or
registrations. No library, Cargo, output-schema or new-test-target change.
No unresolved blocker. Ticket-only commit/push is authorized. Independent
review of this task is the next gate; no Cycle closure or approval is claimed.

## Task 2: index refresh, index rebuild and folder create

Checkpoint recorded **2026-10-25T03:48:25Z**. The owner's fresh Task 2 brief
explicitly reports **Task 1 spec and quality both APPROVED** at
`8ae22ab9b34e52cf3404f74841cc5e1348ad393a`; the task table now records that
approval. Fresh implementer, sequential work confined to this ticket worktree
and branch, no subagents. Task 3 has not started.

### Preflight and authority

- Clean initial HEAD exactly matched the supplied base `8ae22ab9b34e52cf3404f74841cc5e1348ad393a`.
  Fetched origin, read `git show origin/main:AGENTS.md`, and merged
  `origin/main`: already up to date, HEAD unchanged. Worktree registration and
  main ancestry were checked.
- Read the handoff, ticket and all comments, Cycle, design, Task 2 plan and
  execution ledger, retaining all five approved Task 0 corrections.
- Checkpoint fetch and merge again found main unchanged at
  `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`.
  `git merge-tree --write-tree HEAD origin/main` succeeded, tree
  `8c97e736aeb0810c6d56217b626be7138b6784d5` (committed task base).

### Implementation and source facts

- Added the three public bin-private adapters in `authoring.rs`, its module
  declaration and adjacent `cfg(test)` module, three dispatch arms, and literal
  `Available("C3")` catalog entries preserving the planned summaries.
- Both index adapters parse flags first, then `admin::mutation_root`, and
  build the corresponding root-only mutation through `mutate::run` with
  `identity_prompt: false`, `retry: true`, root cloning and the sleep hook.
  They refuse input and ID selectors. Folder creation checks the closed input
  object's required string `path` before root selection and builds
  `FolderCreateInput` through the same driver. Admin changes are only the two
  prescribed helper visibilities.
- Added the closed draft-2020-12 `folder_create` input schema and example.
  The actual example bytes run through the binary in the success fixture.
  No new input-holding type or diagnostic formatting was introduced.
- Task 0's index source facts remain authoritative: successful index mutations
  have null data and discovery current (`mutation/bind/index.rs:80–88`), and
  rebuild restores an unavailable index before binding (`mutation/execute.rs:83–141`).
  The adapter passes through the approved rebuild root-selection exception.
  Folder source inspected at `mutation/bind/folder.rs:100–177` maps invalid
  paths, occupied paths and already-present folders as the binary tests assert.
- Request replay checks use the published stored-result DTO, not an envelope:
  `tests/fixtures/cli_v1/request_show.json` demonstrates the distinction. The
  replay envelope is still compared in full to the original invocation.

### Red/green and golden evidence

1. Added eight index binary tests before implementation. The prescribed index
   filter ran **5 existing passed / 8 new failed / 187 filtered**, each new
   failure on `command_unavailable`. Both cache-loss tests first verified the
   prescribed read refusal/recovery before reaching unavailable rebuild.
2. Added five folder binary tests: **1 existing passed / 5 new failed / 194
   filtered**, all on `command_unavailable`. Added the closed-input/privacy
   case before implementation too; rerun **1 passed / 6 failed / 194 filtered**,
   again only expected unavailable-command failures.
3. Implemented the adapters. E1 was regenerated using only
   `MANYHANDS_UPDATE_GOLDEN=1`, one newly available verb at a time: refresh,
   rebuild, folder create. The help filter passed **9** each time. Read every
   diff before enabling the next verb: only availability, repository rules and
   options changed; summaries and `delivered_by: C3` stayed exact.
   `help_resource_ticket.json` regenerated byte-identically.
4. First index generation run had **11 passed / 2 failed**: the new replay and
   human tests incorrectly treated `request show.data.result` as a full
   envelope. Systematic debugging compared its actual DTO to the existing
   request-show golden. Corrected only the new tests to compare the stored
   result fields and use the replay envelope's operation ID. No production or
   old-test fix was needed; no library/contract gap was found.
5. Generated all nine new goldens (six JSON, three human text) using the update
   variable. Read all nine files; only repository/request/operation IDs are
   substituted. Generation passed **13 index / 7 folders**. Repeated both
   filters without the variable: both passed.
6. Self-review strengthened replay evidence: an unindexed new document stays
   undiscovered after replay of either index verb, and an externally removed
   empty folder is not recreated by finished-request replay. This proves no
   second effect independently of clock granularity. Final scoped reruns
   passed, with the original envelopes and request records unchanged.

Every fixture uses `contract_repository`, an isolated `CliEnv` data directory
and home with `core.autocrlf=false`, and detached-terminal CLI children.
Services are dropped before children run or SQLite sidecars/database are
removed. Cache tests remove WAL/SHM first, then delete the database or replace
it with exactly 4096 bytes of `0xA5`. They pin respectively
`repository_not_registered` with empty recovery and `index_unavailable` with
`index.rebuild`, then recover both fixture documents and both tickets.
Canonical/worktree snapshots and isolated `git for-each-ref` output stay equal.
The bare-remote test supplies a branch absent locally and proves remote refs
do not change. Folder tests prove actual emptiness, listing, no commit, the
existing `docs/notes` fresh-request no-op, occupied/outside-docs refusals,
replay, human output and malformed/duplicate/unknown/mistyped input privacy.

### Final verification

Only Task 2's integration filters and E1 help filter ran, all through Devenv
with one positional filter per command. No full suite or native CI.

| Command | Result |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --test cli index` | **13 passed**, 0 failed/ignored, 188 filtered; 8 new + 5 existing. |
| `devenv shell -- cargo test --all-features --locked --test cli folders` | **7 passed**, 0 failed/ignored, 194 filtered; 6 new + 1 existing. |
| `devenv shell -- cargo test --all-features --locked --test cli input_schemas` | **7 passed**, 0 failed/ignored, 194 filtered; includes the new schema/example automatically. |
| `devenv shell -- cargo test --all-features --locked --test cli help` | **9 passed**, 0 failed/ignored, 192 filtered, final run without the update variable. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed, including the final new replay assertions and the bin's cfg(test) module. |
| `devenv shell -- cargo check --all-features --locked` | Passed. |
| `devenv shell -- cargo fmt --check` | Passed after formatting. |

Total: **36 passing integration tests**, including **14 new**. The one new
adjacent unit case pins confirmation-flag grammar; it was compiled/linted by
all-target clippy, but not executed because Task 2 authorizes only the named
integration filters. Its execution remains with the planned CLI-unit checkpoint;
it is not included in the 36 passing count.

### Files and self-review

- Created `src/bin/manyhands-cli/{authoring,authoring_tests}.rs`,
  `tests/cli/{index,folders}.rs`, the folder schema/example and nine registered
  output goldens.
- Changed the specified binary module/admin/catalog/dispatch files and test
  module/golden registrations, plus the two root-help E1 fixtures. The third
  E1 fixture has no resulting diff.
- Updated this ledger and canonical comment `01M5RJS0SF0XY1BYSADTTG5HEB`, whose
  ID came from `./target/debug/manyhands-cli id new` and timestamp from `date -u`.
- Self-review checked the adapter ordering and plans, closed input and error
  privacy, actual filesystem/Git invariance, complete fixture item recovery,
  finished replay effects, exact golden registrations and all E1 diffs.
  No library, Cargo, output schema, new test target or unrelated old-test edit.
  No unresolved blocker. Independent spec/quality review is the next gate.

Task-only commit and ticket-only push are authorized. No PR, main integration,
closure, cleanup, native CI, dogfooding or Task 3 work was performed.

## Task 3 Step 0: interrupted-save checkpoint stopped

Recorded **2026-10-25T03:57:05Z**. The controller's fresh Task 3 brief reports
**Task 2 spec and quality both APPROVED** at
`66efd0629d73fa8d762921fd987d143604178175`. Only Step 0 was authorized for this
session segment, with a mandatory return before the remaining task.

### Preflight and test scope

- Work stayed in the named ticket worktree and branch. Initial status was clean
  and HEAD matched the supplied base. Read handoff, current main AGENTS via
  `git show origin/main:AGENTS.md`, ticket/comments, Cycle, design, Step 0 plan
  and ledger. Worktree registration and main ancestry were checked.
- Initial and checkpoint fetch/merge found `origin/main` unchanged at
  `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`; both merges answered already up
  to date. HEAD remains the base. Checkpoint merge-tree succeeded with tree
  `97a1668b094b6f87a28664248164abb992973546`.
- Added only `tests/cli/documents.rs` and its module registration in the
  existing `tests/cli.rs` target. The single test
  `interrupted_saves_follow_the_required_outcomes` contains all five table
  cases, with row 5 split into restoration after each refusal (six fixtures
  planned). Execution is sequential and stops on the first failed assertion.
- Each fixture uses library `DocumentCreate` to create the saved document
  and a second contextual document, then arms `FailOnce` at
  `BeforeIndexTransactionCommit` for a body-changing save. Assertions prove
  partial/Incomplete, checkpoint committed, bytes equal to the Git blob,
  accepted request and one pending journal row. A plain reopened service over
  the same data drives retries through `support::mutation::execute_with`.
- The omitted-title retry fills title from one `show_item` call, sends the
  requested body, and preserves the original observation. The all-fields retry
  sends the original full draft without a read. Other-work probes are index
  refresh and a body-changing save of the second contextual document, not
  early no-ops/refusals or finished replay. Snapshots include canonical and
  linked-worktree bytes, HEADs, refs, indexes and status.

### Commands and actual evidence

`devenv shell -- cargo fmt` ran. The first exact test invocation did not
compile because the new diagnostic attempted to serialize `RequestRecord`,
which derives Debug/PartialEq but not Serialize. Source inspection at
`mutation/records.rs:213–235` identified this as a new-test logging error.
Only the diagnostic changed to print state/attempt/result; no assertion changed.
Formatting and the exact command were repeated:

```sh
devenv shell -- cargo test --all-features --locked --test cli interrupted_saves_follow_the_required_outcomes
```

Result: **0 passed, 1 failed, 0 ignored, 201 filtered**, runtime **3.06s**.

| Row | Observed evidence before stopping |
| --- | --- |
| 1: no further edit | Both new index refresh and another-item save returned `blocked` / `recovery_required`, class Blocked (CLI mapping 3). Full snapshots unchanged; original request Accepted, attempt 1, no result, one pending row. Both other request records absent. Original document branch stayed at two commits (create plus interrupted save). |
| 2: unchanged omitted-field retry | Returned `success` / `ok`, discovery current; original request Finished, attempt 2, pending rows zero. Same branch tip, two commits, exact committed bytes retained; full snapshot also reported unchanged. Subsequent new refresh and another-item save both succeeded and finished their own records. Other document gained exactly one save commit. |
| 3: edit omitted title then retry omitted | Returned `partial` / `request_mismatch`, **Incomplete**, with historical written/committed/pending effects. Snapshot unchanged; original request Accepted, attempt 1, result absent; one pending row. This matches the table's stated code/no-new-write/accepted-state outcome, but the new test additionally asserted **Input** and failed there. |
| 4: edited file, all original fields | Authored, not executed. |
| 5: restore after row 3 or 4 | Both variants authored, not executed. |

Captured Git/file evidence from this run:

| Row | Primary HEAD | Saved document branch checkpoint | File bytes after actor |
| --- | --- | --- | --- |
| 1 | `b2da376de44ffaafcfb6bffc65dd945f91d0c969` | `3cbbbc5af0dd1f0ad6c7d7f231dabc2e56491d91` | 157, equal to committed blob |
| 2 | `636f38bc0fa1e507ef9dfe1e6d4337de6a27e584` | `96543ea8de0283e6a2b03f437b939a67729c1e10` | 157, equal to committed blob |
| 3 | `2dd38bf4d481851a39e9e8a0781d8c33ec8bb75f` | `93c3a5168c97c2dae1575fee01d5b2f5cf791200` | 157, equal to committed blob before the deliberate title edit |

The interrupted request ID is `01ARZ3NDEKTSV4RRFFQ69G5FX3`; other-work request
IDs end in `FX4` and `FX5`. Row 2's other-document save checkpoint is
`29f9b3a941c236aff83147e608475d817db78ad7`. Row 3's mismatch has no operation
ID or recovery actions and reports commit `93c3a5168c97c2dae1575fee01d5b2f5cf791200`.
These are temporary-fixture OIDs, not ticket-branch commits. Temporary fixture
directories are dropped by the test harness; the evidence is retained here
and in the failed-run transcript, not as retained runtime databases.

### Diagnosis, self-review and controller question

No genuine library/table discrepancy has yet been demonstrated. The new test
over-specified row 3 by treating the code's Input classification as the
envelope's class. `src/repository/mutation/replay.rs:123–153` explicitly carries
durable effects from an accepted request into a mismatched reply, producing
partial. `src/results.rs:273–288` gives Partial precedence as Incomplete;
the existing CLI renderer maps this to exit 4. The actor itself has no process
exit. The design row specifies `request_mismatch`, nothing written, record
accepted; it does not specify Input or exit 2. Those stated facts held in the
captured diagnostics, but the assertion stopped before their following checks.

**Question for the controller/owner:** may the extra row-3 classification
assertion (including the row-5 replay of that path) be corrected to the existing
partial/Incomplete envelope rule, then the exact Step 0 test rerun with every
approved table expectation unchanged? The test and table have not been changed
to fit the runtime answer. The full-field path's extra class assertion has not
been exercised and also awaits the controller's direction.

Self-review compared all five authored cases to the table and reviewed the
diagnostic source above. Final `git diff --check` and
`devenv shell -- cargo fmt --check` passed; status confirmed exactly the four
intended files and HEAD unchanged.
No document-save implementation, library/Cargo/output-schema changes, other
Task 3 steps, subagents, broader tests, native CI, commit or push. The new test,
registration, this ledger and canonical comment `01M5RK8WM0C2S94ZZS19A7RXVQ`
are left uncommitted. The comment ID came from the built CLI's `id new` and
its timestamp from the actual UTC clock. This checkpoint is **STOPPED**, not
READY, pending controller direction.

## Task 3 Step 0 resumed: owner ruling and passing checkpoint

Recorded **2026-10-25T04:01:33Z**. The owner explicitly approved correcting the
extra failure-class assertions to the existing Partial/Incomplete precedence,
including the unexecuted full-field path according to source, and rerunning
only Step 0. Every approved table expectation must remain unchanged. The
preceding stopped checkpoint and its diagnosis are retained as history; its
controller question is now resolved.

### Authorized correction and source check

Only four class values changed in the test: the row-3 mismatch and its row-5
restoration precondition changed Input to Incomplete; the row-4 full-draft
refusal and its row-5 restoration precondition changed Blocked to Incomplete.
No result code, file/HEAD/commit assertion, request-state assertion, fixture,
table or production code changed.

The full-field source trace was checked before editing:

- `mutation/bind/document.rs:916–933` delegates re-entry/rerun to shared save
  handling.
- `mutation/bind/save.rs:388–496` reports the domain error together with
  recorded commit evidence. Its stopped-effects helper (`:548–568`) retains
  the checkpoint while the operation is in flight.
- `mutation/outcome.rs:521–542` maps that checkpoint to written/committed/
  discovery-pending effects. `mutation/reply.rs:154–161` classifies the
  envelope from code and effects, and `results.rs:273–288` maps Partial to
  Incomplete regardless of the code. The mismatch trace from the preceding
  checkpoint remains applicable to the omitted-field path.

### Verification and all-row evidence

Ran exactly:

```sh
devenv shell -- cargo test --all-features --locked --test cli interrupted_saves_follow_the_required_outcomes
```

**Passed: 1 test, 0 failed, 0 ignored, 0 measured, 201 filtered; 5.72s.**
This one sequential test executed **five table rows / six independent
interruption fixtures**, covering both restoration paths in row 5.

| Row | Passing assertions |
| --- | --- |
| 1 | New index refresh and another-item save both return recovery_required / Blocked (renderer class for exit 3). Full canonical/worktree snapshots unchanged; original request accepted and one pending row; original checkpoint/primary HEAD unchanged. |
| 2 | Same request with omitted title filled by show_item returns success/ok and discovery current. Request finishes, pending count becomes zero, committed file bytes and checkpoint stay unchanged, and both subsequent new-work requests succeed. The other document gains its one new save commit. |
| 3 | Uncommitted title edit followed by omitted-title retry returns request_mismatch / Incomplete. Full snapshot unchanged; request stays accepted with one pending row and unchanged checkpoint. |
| 4 | The same edit followed by every original draft field returns external_change / Incomplete. Full snapshot unchanged; request stays accepted with one pending row and unchanged checkpoint. |
| 5 | After each of rows 3 and 4, restoring the exact committed bytes then retrying with title omitted completes discovery and finishes the request, with zero pending rows and no second save commit. |

Every fixture first proves the actor's partial/Incomplete result, committed
checkpoint, file equality with the committed Git blob, accepted request and
one pending operation. Saved-document commit count stays **two** (create plus
interrupted save) through each retry; primary HEAD remains unchanged. The
actor has no process exit. Step 12's binary retry evidence remains owed.

### Return state and self-review

- Resumed/checkpoint fetch and merge both found main unchanged at `e49f978`;
  HEAD remains `66efd0629d73fa8d762921fd987d143604178175`. Merge-tree succeeded
  with tree `97a1668b094b6f87a28664248164abb992973546`.
- `devenv shell -- cargo fmt --check` and `git diff --check` passed. Self-review
  confirms the sole Rust changes in this resumed segment are the four
  owner-authorized class values; all approved table assertions are preserved.
- Added canonical passing-checkpoint comment `01M5RKH2MWH74YBP39KX97MQBZ`,
  using the built CLI's `id new` and the actual UTC clock. The earlier stop
  comment remains as history.
- Five files remain uncommitted: the new document test module, its existing
  target registration, this ledger, and the two checkpoint comments. No
  commit/push, broader test run, subagent or remaining Task 3 work occurred.

**READY:** Step 0 passed, with no unresolved checkpoint issue. Return to the
controller now; continue remaining Task 3 only when this same session is resumed.

## Task 3 remaining steps: stopped on primary-only move-to-self

Recorded **2026-10-25T04:11:35Z**. The controller authorized remaining Steps
1–13 in this same session, with the Step 0 assertions (including the approved
Incomplete correction) retained, Task 3 commit/ticket-only push after passing
checks, and independent controller reviews before Task 4. The earlier Task 2
approvals and Step 0 owner ruling remain recorded above.

### Work completed before the stop

- Added bin-private `fill.rs` and four pure tests in `fill_tests.rs`, plus the
  module declaration. The prescribed red command
  `devenv shell -- cargo test --locked --bin manyhands-cli fill::` failed on
  the absent fill types/functions. After implementation the same command
  passed **4 / 0 failed / 178 filtered**. The read wrapper was not yet called
  by dispatch, so that intermediate run reported three dead-code warnings;
  subsequent adapter compilation had no warnings.
- Fill tests cover complete input/no read, omitted title, omitted body with
  exact CRLF/lone-CR/non-BMP/no-final-newline bytes and empty body, and
  wrong-kind/unfillable reads. The wrapper preserves read errors and request
  IDs, scopes wrong-kind/missing-field refusals to the item, and sets all
  refusal effects to not_requested.
- Added 26 binary tests beside the existing Step 0 test. Exact red run:
  `devenv shell -- cargo test --all-features --locked --test cli documents`:
  **1 passed / 26 failed / 201 filtered**, 8.73s. All 26 new tests failed on
  command_unavailable; the original Step 0 test passed.
- Implemented the three authoring adapters/dispatch arms, closed input schemas
  and examples. Flags precede input; save fills only omitted draft fields once
  before the driver, preserves the caller's observation, and clones the full
  draft for bounded retries. No identity prompts or library workaround.
- Delivered literal catalog entries and regenerated E1 one verb at a time:
  create, save, move. Each exact help-filter generation command used
  `MANYHANDS_UPDATE_GOLDEN=1` and passed **9 tests / 219 filtered**. Inspected
  each cumulative diff before enabling the next verb. Only the three document
  commands' availability/repository/options changed in root JSON/text;
  summaries and C3 delivery remain exact. Ticket help is byte-identical.
- Converted Step 0's retries and other-new-work probes to isolated CLI children.
  Its assertions are unchanged. A test-only BinaryAnswer decodes code,
  outcome and discovery from schema-checked JSON, retains the raw envelope in
  diagnostics, and maps the actual process status to the asserted failure
  class. It does not execute a second library request. The interrupted actor
  still uses FailOnce/DocumentCreate and plain services for state inspection.

### First implemented run and mandatory stop

```sh
MANYHANDS_UPDATE_GOLDEN=1 devenv shell -- cargo test --all-features --locked --test cli documents
```

Result: **24 passed / 3 failed / 0 ignored / 201 filtered**, 11.20s.
The binary `interrupted_saves_follow_the_required_outcomes` passed, including
all five rows and both restoration paths; no Step 0 regression occurred.

**Genuine contract discrepancy:**
`documents::a_move_to_its_own_path_is_a_noop` first passes the contextual no-op
case, then sends a fresh request for primary-only document
`01ARZ3NDEKTSV4RRFFQ69G5FD0` (`docs/guide.md`) to its own path. The approved
design Fact 15, implementation Step 8 and controller brief require
external_change / exit 3 for this primary-only case. Actual binary output:

```json
{
  "command": "document move",
  "request_id": "01ARZ3NDEKTSV4RRFFQ69G5FX3",
  "operation_id": null,
  "outcome": "noop",
  "code": "already_applied",
  "scope": {
    "repository": "/tmp/.tmpNwXXU5",
    "item_id": "01ARZ3NDEKTSV4RRFFQ69G5FD0",
    "branch": null,
    "worktree": null,
    "remote": null
  },
  "effects": {
    "write": "unchanged",
    "checkpoint": "unchanged",
    "discovery": "not_requested",
    "publication": "not_requested",
    "integration": "not_requested",
    "cleanup": "not_requested",
    "commit_oid": null
  },
  "data": { "id": "01ARZ3NDEKTSV4RRFFQ69G5FD0", "path": "docs/guide.md" },
  "recovery": []
}
```

Actual process status **0**, expected **3**. The error is returned from the
test's existing checked helper before its final snapshot comparison, so no
post-refusal snapshot assertion is claimed for this failed primary-only call.
The contextual no-op snapshot assertion immediately before it passed.

Source diagnosis confirms this is not a fixture's accidental editing context:

1. `tests/support/items.rs:252–293` commits DOCUMENT_A on primary and refreshes;
   it creates no item worktree. This test creates only DOCUMENT_NEW, a
   different ID, before the primary-only call.
2. `mutation/bind/document.rs:550–552` detects move-to-self and calls
   `only_at`, then the already-applied rule.
3. `only_at` (`:512–526`) calls `Position::of` for the document branch and
   reads files from the returned base OID.
4. Crucially, `mutation/evidence.rs:19–44` explicitly falls back to primary
   HEAD when that branch does not exist (`:34–38`). Thus a missing document
   branch does **not** make `only_at` false; it examines primary's committed
   file. This detail was missed by the earlier Fact 15 audit.
5. `document.rs:381–388` then applies the committed-content no-op rule in the
   item's primary context. No new context/operation is reported.

**Owner ruling requested:** approve documenting and testing the existing
primary-only committed move-to-self no-op (and narrowly correcting Fact 15
and directly dependent plan/Cycle/audit wording), or provide the intended
resolution of this library/contract discrepancy. No production workaround,
library change or expectation/table edit has been made to fit this answer.

### The other two new-test failures in that same run

These are recorded separately from the genuine contract discrepancy. No fix
was made after the mandatory stop:

- `document_verbs_have_human_success_refusal_and_noop_output` expects the
  literal code `already_applied` in human stdout. Existing
  `human_admin.rs:139–147` prints **“Already applied; nothing changed.”** and
  metadata/data, not the code. The first (create) success golden was written;
  the test stops on its human no-op assertion before save/move text cases.
  This is a new-test output assumption, not a required-contract failure.
- `document_create_replays_and_refuses_a_used_or_escaping_path` reaches the
  malformed item_id case and correctly gets invalid_input, but incorrectly
  expects JSON `message` to contain `item_id`. The existing refusal adapter
  (`mutate.rs:273–301`, `render.rs:124–133`) passes the field-specific detail
  only to human output; JSON keeps the fixed envelope message. The adapter
  uses the existing InputError::InvalidValue with field item_id. The new test
  should check the field-only detail in human output on resume, while keeping
  the JSON invalid_input/request-ID/privacy assertions. No C2 driver change.

### Preserved state, remaining work and self-review

- Initial/resumed/checkpoint fetches and merges leave main at `e49f978`, HEAD
  at `66efd0629d73fa8d762921fd987d143604178175`. Checkpoint merge-tree succeeds
  with tree `97a1668b094b6f87a28664248164abb992973546`.
- `git diff --check` passed. Self-review traced the genuine mismatch through
  source and distinguished the two test-output assumptions above. Full final
  fmt/check/clippy and the Step 12 input_schemas/fill reruns have **not** run
  after these changes; the task is not review-ready.
- Seven new JSON goldens and create's text golden were generated. The two
  save/move text goldens are still owed. New output goldens have not completed
  final review. The three input examples exist; create's actual example ran
  through the binary, while save/move example runtime evidence remains owed
  to the planned example checks. No output schema changed.
- All work is uncommitted; no Task 3 commit/push. The Step 0 comments remain
  preserved, and this stop adds canonical comment `01M5RM3EP5798TPZQQRB2H3456`,
  ID from the built CLI and timestamp from the actual UTC clock.
- No subagent, broad test suite, native CI, PR, main integration, ticket
  closure, cleanup or Task 4 work occurred. No existing test body changed;
  existing-test changes are registrations/additions and E1 only.

Resume this same implementer only after the owner resolves the genuine
primary-only move-to-self discrepancy. Do not change its expected outcome or
the approved documents without that ruling.

## Task 3 completion: owner ruling resolved, local gates passed

Recorded **2026-10-25T04:19:30Z**. The owner explicitly approved accepting the
existing **primary-only committed move-to-self `already_applied`/`noop`, exit
0**, correcting Fact 15 and dependent wording/tests, with no library workaround.
The owner also authorized correcting the two new-test output assumptions using
the existing human/JSON contract and continuing the remaining Task 3 gates,
goldens, ledger/comment, task commit and ticket-only push. This resolves the
stop immediately above; its original source/binary evidence remains history.

### Narrow corrections and verification

- Corrected design Fact 15, the implementation plan's Review Focus 3 and
  Step 8 bullet, the API audit's move no-op/inventory wording, and this ledger's
  fact inventory. Added a superseding owner note beside the Cycle's historical
  review record. No other approved policy or code-set count changed.
- The primary-only test now requires the approved noop/exit 0, no commit OID,
  no document branch or worktree before/after, and an identical repository/
  linked-worktree snapshot. Its contextual no-op assertions remain intact.
  This adds executed state evidence beyond the original stopped-run output.
- Human no-op tests now require the existing exact leading message,
  “Already applied; nothing changed.” Malformed item_id retains the JSON
  invalid_input/request-ID/privacy checks and verifies the item_id-only detail
  through human stderr. No production behavior changed for these corrections.
- Regenerated the document goldens only with `MANYHANDS_UPDATE_GOLDEN=1`:
  **27 passed / 0 failed / 201 filtered**, 10.41s, including the two previously
  owed human cases. Read all **7 JSON + 3 text** goldens. Only root, request ID,
  operation ID and checkpoint OID are replaced; item IDs, paths, effects,
  outcome/code and recovery remain literal. All are registered.
- The next input-schema run found an in-scope new-schema error: this task had
  added a regex `pattern` on item_id, unsupported by the existing schema
  checker's keyword set (`tests/support/schema.rs:99–166`). Result was **4
  passed / 3 failed / 221 filtered**. Removed only that added keyword from
  the new schema, retaining the planned required-string shape and the CLI's
  canonical ItemId validation. The checker and every old test are unchanged.
  The same filter then passed all 7 tests.
- All-target clippy identified one needless borrow in the new fill read-error
  wrapper. Removed that borrow only; the repeated prescribed clippy passed.
  No pre-existing test lint or outside-file fix was needed.

### Final commands and counts

All Rust commands ran through Devenv in this ticket worktree. Tests used only
the task's named targets/filters, one positional filter per command. Final
golden-checking runs had no update variable.

| Command | Final result |
| --- | --- |
| `devenv shell -- cargo test --all-features --locked --test cli documents` | **27 passed**, 0 failed/ignored, 201 filtered; final post-lint run 10.24s. Includes the binary Step 0 test and all five required rows/six fixtures, plus Step 7's separately named interruption case. |
| `devenv shell -- cargo test --all-features --locked --test cli input_schemas` | **7 passed**, 0 failed/ignored, 221 filtered; includes all three new closed schemas and examples. |
| `devenv shell -- cargo test --locked --bin manyhands-cli fill::` | **4 passed**, 0 failed/ignored, 178 filtered; final post-lint run. |
| `devenv shell -- cargo test --all-features --locked --test cli help` | **9 passed**, 0 failed/ignored, 219 filtered; final run without regeneration. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed. |
| `devenv shell -- cargo check --all-features --locked` | Passed. |
| `devenv shell -- cargo fmt --check` | Passed after formatting. |

Total **47 passing tests** in these final filters: 43 integration + 4 unit;
**31 new tests** in Task 3 (27 document integration + 4 fill unit). Earlier
baseline/task tests are not counted again. No broad/full suite or native CI.

### Self-review, scope and return

- Reviewed the thin adapters, flag/input/root/ID/fill ordering, one show read
  only when needed, caller-token preservation, draft cloning before retries,
  no identity prompts, closed input schemas and all ten output goldens.
- The source-level fill tests remain repository-free. Binary tests cover
  offline checkpoints, metadata/body preservation, exact unusual body bytes,
  normalization, stale observations, replay/mismatch, wrong-kind scope and
  request ID, both missing-registration paths, cache-loss replay, interrupted
  saves, move collisions/context tokens, primary-only moves, malformed input,
  human success/refusal/no-op and progress. Snapshots prove required refusal/
  replay/no-op invariance; explicit branch/count/byte assertions prove no
  second checkpoint. Step 0's approved assertions survived binary conversion.
- The three input examples validate. Create's actual example bytes also run
  through the binary. Task 7 owns the full R-13 examples table/runtime checks,
  including the token-placeholder save/move examples; this is not claimed as
  completed ahead of that task.
- Reviewed E1's per-verb generation diffs and final golden reads: only the
  root-help JSON/text files changed; ticket help remained identical. Existing
  Rust tests/support only gained module/golden registrations. No library,
  Cargo, output-schema, shared mutation-driver, old-test semantic edit, or
  new integration target.
- Final checkpoint fetch/merge leaves `origin/main` at
  `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`; merge already current.
  `git merge-tree --write-tree HEAD origin/main` succeeds with tree
  `97a1668b094b6f87a28664248164abb992973546` at the unchanged task base.
  Status, diff, whitespace and last ten commits inspected before committing.
- This completion includes all earlier uncommitted Step 0 evidence/comments,
  Task 2's reported spec/quality approvals, both owner rulings and canonical
  completion comment `01M5RMHYBJED90MVAY9KR0S6PZ`. ID allocated by the built
  CLI; timestamp taken from the actual UTC clock.

No unresolved Task 3 blocker. Commit message:
`feat(cli): document create, save and move with the save patch`, with the
required co-author trailer. Publication is restricted to this ticket branch.
Independent controller spec/quality reviews are next; Task 4 remains unstarted.
No subagents, PR, main merge, ticket closure, cleanup or dogfooding operation.

## Task 4: Part A evidence and checkpoint

Recorded **2026-10-25T04:35:48Z**. Fresh implementer, working only in the named
ticket worktree/branch, without subagents. The controller's brief explicitly
reports Task 3 spec and quality **APPROVED** at
`90c8e3ae15f9085c74b283c95bc557d60a7abc00`; the task log now records those
approvals. They are not approval of Task 4 or of the complete Part A range.

### Preflight and brief clarification

- Clean initial HEAD matched the supplied Task 4 base. Read current main
  AGENTS with `git show origin/main:AGENTS.md`, handoff, ticket and comments,
  Cycle/design/Task 4 plan, audit and the ledger's approved rulings.
- Initial and resumed fetch/merge both answered already up to date. Main
  remains `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`.
- Before editing, stopped on the brief's phrase “expected all capable.” The
  controller clarified that this was an erroneous summary subordinate to the
  approved audit, not a requirement or contract change. Followed the existing
  audit: all six Part A mutations, **four capable / two incapable**. This is
  a **controller-brief clarification**, not a new owner ruling. Approved
  Cycle/design/plan/audit documents are unchanged.

### Test artifacts and evidence boundaries

Added `tests/cli/authoring_exits.rs` and its registration in `tests/cli.rs`:

- Separate C3 `ROWS`, `REASONS`, `How` and `CLI_ONLY`; C2's table is unchanged.
  The audit's fenced union is read with `include_str!`. Exact set equality
  proves **40 library / 42 CLI-inclusive codes**, **25 binary-backed rows +
  17 reasons**, with unique and disjoint row/reason codes.
- Every binary row names an existing `#[test]` function in `tests/cli/*.rs`.
  New binary cases cover root selection, configuration, dirty configuration,
  wrong branch, missing identity and unopenable application data; existing
  Part A cases supply the remaining normal codes. These reachable cases are
  not padded with unreachable-code reasons.
- Binary classes **0, 1, 2, 3 and 5** are represented. Exit 4 remains the
  explicit decision-6 actor/CLI-completion reason referencing
  `commit_success_refresh_failure_is_completed_by_the_same_request`.
  Registration failure is a source-backed library-seam reason. **15** Part B
  obligations are labeled Task 5 or Task 6, including PTY cancellation/130.
  Reason membership is accounting, not a claim those binary cases passed.
- `busy_is_reached_with_the_lease_held` obtains a valid observation, sends a
  complete document draft (avoiding an earlier fill-read refusal), holds the
  repository lease in the existing child helper, and asserts busy/exit 5,
  request ID, unchanged repository/worktree snapshot and elapsed retry time
  of at least 700 ms and below the helper's ten-second deadline.
- `c3_confirmed_flags_match_prepare` enumerates delivered available C3
  mutations via JSON help's `--request-id` option, not a nonexistent kind
  field. Each of the six gets a fresh isolated fixture and valid input;
  save/move observations come from actual reads. All answer not_confirmable,
  exit 2, without canonical/worktree changes.
- `c3_noop_inventory_accounts_for_part_a` compares the same help inventory
  against all six entries, checks each capability against its audit row,
  verifies four existing no-op test references, and supplies binding-source
  reasons for index refresh/rebuild. Fresh explicit index work is ok, not noop.

The new tests characterize existing approved behavior; no production behavior
was written. The focused first run passed **10 / 0 failed / 228 filtered**,
2.65s. No intentional failing production regression is claimed for this
test-only evidence task. Formatting touched only the new module via
`devenv shell -- rustfmt --edition 2024 tests/cli/authoring_exits.rs`.

### Checkpoint gate

All six prescribed commands ran in this worktree:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed, 0.36s. |
| `devenv shell -- cargo fmt --check` | Passed across the tree. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed, 1.09s; no old-test lint or other outside-C3 failure. |
| `devenv shell -- cargo check --locked --bin manyhands-cli` | Passed, 18.52s; no features. |
| `devenv shell -- sh scripts/check-cli-no-gpui.sh` | Passed; locked no-default-feature normal dependency tree contains no GPUI package. |
| `devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract` | Completed retry passed **182 CLI unit + 238 CLI integration + 71 read-contract + 44 mutation-contract = 535**, zero failed/ignored/measured/filtered in every target. Target durations: 0.23s / 58.49s / 9.85s / 8.62s respectively. |

The first combined gate tool call hit the shell's **120-second timeout** during
mutation-contract tests, after the first five gates and both CLI targets had
passed (182 and 238). No assertion failure was reported, and a process probe
confirmed no remaining Cargo/test process. Only the incomplete exact test-gate
command was retried, with a 600-second tool budget; that completed as above.
The five completed gates were not repeated. The 535 total counts the completed
run once, not the interrupted run or focused run again. No full Cargo suite or
native CI ran. Task 2's previously unexecuted adjacent unit case passed in the
182-test CLI-unit target.

### Self-review and delivery for controller review

- Reviewed the complete new module, all referenced test assertions, source
  reasons, audit equality, catalog filtering, valid prepare fixtures, child
  lease lifetime, retry bounds, isolation and effect assertions. Exit-4
  actor evidence is distinguished from a binary exit; pending Part B cases
  remain explicit. No unresolved implementation issue found in self-review.
- Checkpoint `git fetch origin` and `git merge origin/main` again left main
  unchanged and already current. `git merge-tree --write-tree HEAD origin/main`
  succeeded at the task base with tree
  `b78fb7a25e93aa1f768186dd09843e83323fcef8`. A final check on the actual
  checkpoint commit is reported with its returned SHA.
- Intended changed files are the new test module, module registration, this
  ledger and canonical Part A comment `01M5RNFS864VQHTB7HHC81ES4P`. The comment's
  ULID came from `devenv shell -- ./target/debug/manyhands-cli id new`; the
  timestamp is from `date -u`. Status, diff, whitespace and last ten commits
  inspected before staging only these paths.
- Task base: `90c8e3ae15f9085c74b283c95bc557d60a7abc00`. Task head: the commit
  carrying this section, `test(cli): C3 Part A exit table and inventories`,
  whose full SHA is returned to the controller. **Independent Part A spec and
  quality review is pending**, over the actual range
  `7368902a677a8228813f21da8f74d3cafff68ec3..Task4HEAD` (Tasks 1–4).
  The controller owns review and post-review bookkeeping after return.

Commit/push authorization is restricted to the ticket branch. Only the new
tests/registration and checkpoint records changed; no library, Cargo, output
schema, existing-test semantics or approved-document changes. Task 5 was not
started. No subagents, PR, main integration, closure, cleanup, dogfooding or
native CI. This is a locally verified Part A checkpoint for independent review,
not a review approval or whole-Cycle completion claim.

## Task 4 review follow-up: SPEC P2, C3 index-unavailable provenance

Recorded **2026-10-25T04:44:19Z**, base
`329a1436f3671586382ca4c1f41b2c9a8ab8a699`. The controller reported SPEC P2:
the `index_unavailable` row referenced
`index_rebuild_restores_a_database_replaced_by_other_bytes`, but that test
asserts the refusal on **C1 `document list`** (`tests/cli/index.rs:211–216`)
and subsequently succeeds with C3 rebuild. Verified the finding directly.
The original 535-test gate was green, but did not establish C3 provenance
for this row; the prior coverage claim is corrected by this follow-up.

### Resolution and red/green evidence

- Added only the new Task 4 test
  `index_refresh_on_a_corrupt_database_is_index_unavailable` in
  `tests/cli/authoring_exits.rs`, and repointed its existing exit-table row.
  Task 2's `tests/cli/index.rs` is unchanged.
- The fixture closes its service, removes SQLite WAL/SHM first, then replaces
  the database with exactly 4096 bytes of `0xA5`. The binary invokes C3
  `index refresh` with a valid request ID. It asserts code index_unavailable,
  exit 3, exact command identity, retained request ID, index.rebuild recovery,
  identical canonical/worktree snapshot and identical `refs/` output.
- **Red:** initially exercised the existing row's C1 `document list`
  invocation in the new fixture, retaining the required `command ==
  "index refresh"` assertion. Ran
  `devenv shell -- cargo test --all-features --locked --test cli authoring_exits::index_refresh_on_a_corrupt_database_is_index_unavailable`.
  Result: **0 passed / 1 failed / 238 filtered**, 0.23s, exactly at command
  identity: actual `document list`, expected `index refresh`. This is an
  evidence-attribution regression demonstration, not a production defect or
  library gap; code/exit alone would have passed the misleading old invocation.
- **Green:** changed only the new fixture's invocation to `index refresh`
  with `--request-id`; retained its assertions. After formatting the new
  module, `devenv shell -- cargo test --all-features --locked --test cli authoring_exits`
  passed **11 / 0 failed / 228 filtered**, 2.79s.

### Revised-tree checkpoint gate

The test artifact changed, so the plan's checkpoint gate was rerun **once** on
the revised tree, with a 600-second tool timeout. All six commands passed:

| Command | Result |
| --- | --- |
| `devenv shell -- cargo check --all-features --locked` | Passed, 0.38s. |
| `devenv shell -- cargo fmt --check` | Passed. |
| `devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings` | Passed, 1.14s. |
| `devenv shell -- cargo check --locked --bin manyhands-cli` | Passed, 0.27s. |
| `devenv shell -- sh scripts/check-cli-no-gpui.sh` | Passed. |
| `devenv shell -- cargo test --all-features --locked --bin manyhands-cli --test cli --test read_contract --test mutation_contract` | **182 CLI unit + 239 CLI integration + 71 read-contract + 44 mutation-contract = 536 passed**; zero failed/ignored/measured/filtered. Durations respectively 0.25s / 51.19s / 11.22s / 8.62s. |

The prior checkpoint had **535** tests; this adds exactly **one**, now **536**.
The focused run is not added again to that total. No further broad run, full
suite or native CI was performed.

### Self-review and return boundary

Self-review checked the precise command/result attribution, valid request ID,
whole-file corruption boundary, recovery assertion, snapshot/ref invariance,
row reference and additive test-only scope. Initial and checkpoint fetch/merge
left main at `e49f978b499ee34d6b77061caaa3a6fb9d37a88b`, already current.
Precommit merge-tree succeeded with tree
`40154382afcb645ef95b13ccacfaec64a2460664`; status/diff/log and whitespace checks
are inspected before the separate follow-up commit. Canonical comment
`01M5RNZCK3GCXM70AN3DZXGTG7` uses the built CLI's ID and actual UTC clock.

The follow-up commit contains this ledger, that comment and only
`tests/cli/authoring_exits.rs`. SPEC P2 is implemented and locally verified;
**independent re-review/acceptance remains pending**. The controller reviews
the follow-up from `329a143` and the complete Part A range from `7368902` to
the returned follow-up SHA. Ticket-only commit/push with the required co-author
trailer; no Task 5, subagents, library/Cargo/schema/approved-document changes,
completed Task 2 test edits, PR, main integration, closure, cleanup or dogfooding.
