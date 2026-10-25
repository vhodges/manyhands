---
title: "Wave 03 C3: CLI Authoring And Discovery Maintenance"
date: 2026-10-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M5QY018FX9D7N05BE3CSYXTC"
---

# Wave 03 C3: CLI Authoring And Discovery Maintenance

## Status

**Draft, for product-owner approval.**

- The owner answered four planning questions on 2026-10-24 (decisions 1–4),
  two after the first review round (decisions 5–6), and one after the
  second (decision 7).
- Both review rounds' findings are applied (see the Review Record).
- Nothing is approved and no implementation is authorized. No build or test
  was run for these drafts.

The other planning documents:

- [Design](../plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-design.md)
- [Implementation plan](../plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-implementation.md)

Ticket: `01M4CC0VMV81DFRGJ4Z76489XM`. The ticket branch merged `origin/main`
at `e49f978` on 2026-10-24 as merge commit `9ed4a04`; it was merged, not
rebased.

## Purpose And Authority

Make local document and ticket authoring, and explicit discovery repair,
available to agents and to this project's own dogfooding
([Wave 03](../Waves/wave-03-dogfooding.md#c3-cli-authoring-and-discovery-maintenance)).
An agent with JSON, or a person at a terminal, can do all of the following
through `manyhands-cli`, without editing canonical files by hand:

- create, save and move items;
- create folders;
- repair and adopt nonconforming files;
- refresh and rebuild the index;
- set ticket relationships.

C3 adds verbs only. It reuses C2's mutation layer unchanged: structured
input, request IDs, two-phase confirmation, terminal consent and the bounded
retry.

Authority, in order:

1. The Wave 03 document: C3's scope, exclusions and exit evidence, and track
   rules 1–7.
2. The [CLI contract RFC](../RFC/cli-contract.md), as amended by C1 and C2.
3. The [ticket relationships and short codes RFC](../RFC/ticket-relationships-and-short-codes.md)
   for `deps`, `parent`, `initials` and `ticket slug-assign`.
4. The F2 Cycle and design: the bindings, previews, result codes, recovery
   actions and schemas C3 consumes unchanged. That includes the split that
   leaves the save patch to the CLI, and the departures from the RFCs that
   F2 lists.
5. The C1 and C2 Cycles and designs: parsing, envelopes, exit statuses,
   isolation, goldens and the mutation driver, which C3 extends.
6. The note on this ticket about multi-item work contexts
   (`01M5NVMGZ1BXK1P15RZNNYX06B`, ticket `01M5NKA3MY7CW4D90M866YS946`).

## Entry Evidence And Dependency Boundary

- **Wave 02.** Cycles 01–04 are closed on main.
- **Wave 03.** F1, F2, C1 and C2 are closed and merged to main. D1 and D2 are
  merged too, and C3 depends on neither (track rule 2).
- **Branch.** As under Status. The merge is pushed with the drafts.
- **Library boundary.** Every C3 verb has an F2 binding or an F1 read:
  - the eleven mutations are variants of `Mutation`
    (`src/repository/mutation/mod.rs:398`);
  - `path observe` is `observe_path`, and `ticket check` is
    `check_ticket_relationships` (`src/repository/read/items.rs:1963`,
    `:2033`).

  C3 changes nothing under `src/repository.rs`, `src/repository/`,
  `src/lib.rs`, `src/results.rs` or `src/runtime.rs`. A gap found during C3
  becomes its own ticket under track rule 5.
- **Known library limits (decision 1).** Three open defects touch C3's verbs.
  None is a prerequisite; each is recorded as a known limit:
  - `01M4PB82G07RZSQMNJZN672S3W`: a database damaged in pages that opening it
    never reads cannot be rebuilt;
  - `01M4PB80T7SVB23TQ6466RDHCB`: the domain refuses a repeat of a committed
    document move. F2's binding works around it, outside the lease;
  - `01M4PB7YZMBFAFFM94AZY08HWS`: an interrupted save whose row recorded no
    step is closed as `rejected` although its file was written.
- **Multi-item work contexts.** Ticket `01M5NKA3MY7CW4D90M866YS946` lands after
  C3. The documents disagree on whether C3 waits:
  - its ticket text (2026-10-23 19:30) says C3 does not wait;
  - its design (17:05) says "Land before C3 plans its authoring verbs" and
    that C3's mutation verbs "should take an optional `--context`".

  C3 follows the later ticket text and the note on this ticket. See Explicit
  Exclusions.
- **Entry-gate item 3.** The API audit has no C3 refresh section yet; Task 0
  adds one.
- **Entry-gate item 7.** Task 0 runs the required Rust checks at the
  implementation baseline.
- **Gate ticket.** `01M57SKVF6TQ3QM6AXJ35JGGSY` gates C4, C5 and D5, not C3.

## Scope

- **Thirteen verbs.**
  - documents: `document create`, `document save`, `document move`,
    `document repair`;
  - folders: `folder create`;
  - tickets: `ticket create`, `ticket save`, `ticket repair`,
    `ticket slug-assign`;
  - index: `index refresh`, `index rebuild`;
  - reads: `path observe`, `ticket check`.
- **Confirmation.** `document repair` and `ticket repair` are confirmed, and
  their previews state the adoption of an ID. The other nine mutations
  answer `not_confirmable` to `--prepare`, as F2 bound them.
- **Mutation input.** As C2 decision 2: selectors are flags, and everything
  else is one JSON object from `--input`. Bodies and sources are JSON
  strings. Each input-taking verb publishes a closed input schema and an
  example input.
- **Save patch (decisions 2, 5 and 7).** `document save` and `ticket save`
  treat an omitted field as unchanged, and `null` as clearing a nullable
  field. The CLI reads the item once and fills the omitted fields, passing
  the caller's observation through unchanged. When that read fails, its
  answer is passed through, even where it differs from the library's own
  save path (decision 7).
- **Relationships.**
  - `ticket create` and `ticket save` take `deps` and `parent`, with `null`
    accepted on both, as the relationships RFC says.
  - `ticket create` and `ticket slug-assign` take optional `initials`.
  - `ticket check` validates a proposed set of relationships before a
    write.
- **Identity recovery.** A save without a usable identity answers
  `identity_required` with the `repo.identity_set` recovery action. After
  `repo identity-set`, the same request ID succeeds. C3 adds no identity
  prompt to saves.
- **Discovery maintenance.**
  - `index refresh` and `index rebuild` each run one explicit local
    operation and exit.
  - `index rebuild` recovers a lost database, or one whose file was replaced
    by other bytes.
  - `path observe` returns the observation that creation, move and repair
    need, including the absent token for an empty path.
- **Dogfooding (decision 3).** In the final task, with the owner's go-ahead at
  that step, the branch-built CLI records the Wave 03 Cycle dependencies on
  this project's open tickets. AGENTS.md and the `file-ticket` skill then
  direct ticket create and update to the CLI.
- **Output.** Library envelopes pass through unchanged. Every verb has human
  output, with progress on stderr as in C2.

## Explicit Exclusions And Downstream Obligations

- **Out of scope:** no comment, synchronization, conflict, promotion,
  closure, polling or draft verb. `comment add` and `operation abandon` stay
  with C4, and `ticket close` with C5. Until then, comments and closure stay
  canonical-file edits.
- **No `--context` on mutation verbs.** The note on this ticket asks C3 to
  "Keep the target context an explicit part of each request, so that an
  optional `--context` can be added later without changing the meaning of an
  existing input. Do not add it in C3."
  - Each C3 request names its target by item ID.
  - The library derives that item's context from the ID alone
    (`context_names`, `src/repository/mutation/context.rs:26`).
  - So an optional `--context` added later, defaulting to that context,
    leaves every existing input meaning what it meant.
  - C3 does not build on the usage error "--context requires --path." either.
- **No other prompts.** No interactive prompt beyond C2's consent prompt,
  which the two repairs use. No identity prompt on saves, and no `$EDITOR`.
  No body through argv, the environment or a file named in the input.
- **No background or network work.** No background work, network contact or
  publication. `index refresh` and `index rebuild` never fetch and never
  rewrite canonical state.
- **No library change.** The defects C3 exposes keep their own tickets
  (decision 1). The partial-damage form of index corruption is not recovered
  by `index rebuild` until `01M4PB82` lands.
- **The save patch's replay limit (decision 2).** A retry recomputes omitted
  fields from the file:
  - after a successful save, a third party's change to an omitted field makes
    the retry answer `request_mismatch`, not the stored result.
    `request show` still returns the stored result, and no effect is
    repeated;
  - a retry of an interrupted save reads what the interrupted attempt
    wrote, so its digest matches, as long as the file still holds those
    bytes;
  - the fill read comes before the replay. So a retry of a finished request
    whose read now fails gets the read's answer, while `request show` still
    gives the stored result.
- **An interrupted save that is then edited cannot be finished in C3.**
  While a save's row is pending, every other request on the repository
  answers `recovery_required`. If the file is edited before the request is
  retried, no retry completes it: with fields omitted it answers
  `request_mismatch`, and with every field `external_change`. The only way
  out in C3 is to restore the bytes the request committed. Otherwise it waits
  for C4's `operation abandon`. The design's interrupted-saves table is the
  contract, and its test is written first.
- **The fill read's answer can differ from the library's (decision 7).** On
  a repository the index does not hold, a save that omits a field answers
  `repository_not_registered` with no recovery action. A save that sends
  every field answers `repository_not_enabled` for a root that was never
  enabled. After a lost cache, it answers `repository_not_registered` with
  `index.rebuild`.
- **Front matter is re-emitted.** Every write re-serializes the whole front
  matter in the library's canonical order and style. Values, unknown keys,
  ULIDs and the body are preserved, but formatting is not. Saving a
  hand-written document commits that normalization even when no value
  changes. A ticket save compares parsed values, and is a no-op when nothing
  changed.
- **Status text `closed` is ordinary metadata.** Neither save nor create can
  write closure fields.
- **Empty folders.** They are local and invisible to Git until they hold
  tracked content. No placeholder commit.
- **Native evidence.** C3 is proven on Linux. Its tests are in the existing
  `cli` target, which is already in the native workflow's list. Windows
  repair consent is a native manual check. Native CI runs on main after
  merge, with its own approval.

## Required Contract

1. **Grammar.** C2's grammar, with the selectors in the design:
   - `--repo` on every C3 verb;
   - `--id` on `document save`, `document move`, `ticket save`,
     `ticket slug-assign` and `ticket check`;
   - `--path`, and an optional `--context`, on `path observe`;
   - `--deps` and `--parent` on `ticket check`.
2. **Input.** C2's rules: one UTF-8 JSON object, read once.
   - Unknown, duplicate and mistyped fields, and missing required ones, are
     `invalid_input`. So are `closed_at`, `closed_by` and `slug`.
   - Missing and `null` differ where the design says so.
   - No diagnostic the CLI writes contains an input value, a body or a
     source.
3. **The save patch (decisions 2, 5 and 7).**
   - An omitted draft field takes the item's current value from one
     `show_item` read, which happens only when a field is omitted.
   - The caller's observation goes to the library unchanged. The library
     alone decides staleness, replay and mismatch.
   - When the read fails, its answer passes through under the mutation's
     name, with its code, scope and recovery actions, and with the request ID
     kept. This holds where that answer differs from a full-field save's
     (decision 7).
   - An item of the other kind answers `item_not_found` with the item scope,
     as C1's `show` builds it.
   - `deps` and `parent` are never filled: omitted means unchanged in the
     library.
4. **Request identity, consent, prompts, retry and refusals.** These follow
   C2's Required Contract items 3, 4, 6, 8 and 10. Item 8 is amended by
   decision 5 for the fill read's answer. Prompts never read stdin, so a
   repair's source can come from `--input -` while consent comes from the
   terminal.
5. **Never printed.** No body, source, secret, backend text, SQL or `Debug`
   rendering appears, in either mode. Library envelopes carry no body.
6. **Reads stay reads.**
   - `path observe` and `ticket check` take flags only. They answer from the
     index and the file system, write nothing and contact no network.
   - `path observe` reports a managed file of another item in the named
     context as what is there, never as outside the context.
   - `ticket check` checks exactly the relationships it is given.
7. **Index commands.** `index refresh` and `index rebuild` take no input.
   They are F2 mutations, so JSON and non-interactive calls need
   `--request-id`.

## Acceptance And Exit Evidence

| Wave exit evidence | Required proof |
| --- | --- |
| Offline checkpoints and no-ops | Through the binary, in a repository with no remote: `document create`, `document save`, `document move`, `ticket create`, `ticket save` and `ticket slug-assign` each commit one checkpoint on the item's branch. Effects say `publication: not_requested`, and the exit is 0. On an item the library wrote, a fresh request that writes what it already holds answers `noop` and makes no commit. A save that omits every draft field changes no value, unknown key or body byte. |
| Stale writes | A stale observation on `document save`, `document move`, `ticket save`, `ticket slug-assign` and on repair's `--confirm` answers `external_change`, exit 3. The file's bytes, the branch tip and the worktree are unchanged. |
| Invalid and malformed repair | A corrected source that is not conforming is refused at prepare with the binding's code, and nothing is written. A path that holds no nonconforming entry is refused. A corrected ID that another item holds is `identity_ambiguous` and creates no context. A repair keeps a closed ticket's closure fields. |
| Adoption retry | A marker-only document is adopted under the ID its corrected source names. A library-level actor stops a repair after its write. The CLI completes it under the same request ID and confirmation, with one commit and the same ID. A replay of the finished repair writes nothing. |
| Move collision | A file at the destination is `occupied_path`, exit 2. Two kinds of destination token are `external_change`: one from another context, and one taken before a file arrived there. Source and destination are unchanged in both cases. A document with no editing context moves with the root's destination token. |
| Index loss and corruption | Two cases, each with the WAL files removed first. Deleting the database file makes reads answer `repository_not_registered`, with no recovery action. Overwriting the file with bytes that are not a database makes them answer `index_unavailable` with `index.rebuild`. In both, `index rebuild --repo` restores the index, and `document list` and `ticket list` list every item again. Canonical files, Git refs and the worktree are byte-identical before and after. The partial-damage form is a known limit (decision 1). |
| Commit success, refresh failure | A library-level actor saves with a failure at `BeforeIndexTransactionCommit`, and gets a partial result with the checkpoint committed. The CLI then retries the same request with a field omitted, and completes discovery with no second commit. |
| Schemas and documented examples | Every input-taking C3 verb has a closed input schema under `schemas/input/v1/`, and an example input under `tests/fixtures/cli_input_v1/`. Each example validates against its schema and runs through the binary to the outcome its test's table gives. Every C3 verb has JSON goldens checked against the published output schemas. Both repairs have a preview golden. |
| Dogfooding switch | With the owner's go-ahead in the final task, the branch-built CLI refreshes this repository's index. It records the Wave 03 Cycle dependencies with `ticket save`, checked by value with `ticket show`, whose `deps` then hold exactly the recorded IDs (design, Dogfooding). It pushes the six branches, which creates them on origin. AGENTS.md and the `file-ticket` skill direct ticket create and update to the CLI. While this row is owed, the Cycle may be review-ready but is not closed or merged, unless the owner amends this evidence. |

Also required:

- **Exit classes (decision 6).**
  - Exit 0, 2, 3 and 5 are reached through the binary; exit 5 uses a
    lease held by a child process.
  - Exit 130 is reached through the pseudo-terminal.
  - Exit 4 is listed in the exit table's reasons, backed by the
    library-actor test.
  - The codes no fixture can reach are listed with their reasons. The rows
    plus that list equal, by set equality, the code set that Task 0's audit
    refresh records for C3's verbs.
- **Interrupted saves.** Every row of the design's interrupted-saves table
  holds. Its test is written first, before any `document save` code; a row
  that answers otherwise stops for a ruling at a design checkpoint.
- **Replay and cache loss.** Every C3 mutation has a lost-output replay case
  with no second effect. A committed save retried after the database is
  rebuilt makes no second commit. A confirmation presented after cache loss
  is `confirmation_not_found`.
- **Human mode, input and no-ops.**
  - Every C3 verb runs through the binary in human mode as well as JSON.
  - Every input-taking verb refuses malformed input as `invalid_input`.
  - Every verb that can be a no-op has a no-op case. A no-op is a fresh
    request that writes nothing, not a replay of a finished one.
- **Identity.** `identity_required` on a save offers `repo.identity_set`.
  After `repo identity-set`, a retry with a fresh observation succeeds under
  the same request ID, because the refused save left a new context and
  deleted its record. `initials_required` on
  create is recovered the same way. These cases need a test machine whose
  system Git configuration sets no identity, as C2's ruling R-17 has it.
- **Relationships.**
  - A dependency cycle and a parent cycle are each refused with
    `relationship_cycle` on create, before any context exists, and on save.
  - An unknown target is accepted and reported as unresolved.
  - For the same complete proposal, `ticket check` reports the same
    rejection without writing.
  - A closed ticket's save and slug assignment answer `ticket_closed`.
  - A ticket with a different code answers `slug_already_assigned`.
- **Mutating-Cycle evidence.** The Wave's evidence row for mutating Cycles
  (`MH-COLLAB-006` and the NFRs) maps as in C2:
  - **progress:** a test asserts human `progress:` lines, and empty JSON
    stderr;
  - **consent:** prepare, confirm and interactive cases for the two repairs,
    with the source piped through `--input -`;
  - **safe-point cancellation:** C3's local commands have no safe point
    after acceptance (F2). Cancellation is shown by `cancelled` at the
    repair's consent prompt;
  - **redaction:** a sentinel body and source never reach stdout, stderr or
    the request stores, and the other sentinels never reach a diagnostic;
  - **stale writes:** as in the table;
  - **retry:** replay, `request_mismatch` and the completion of a stopped
    request under its own ID.
- **Never fetch.** The index verbs, run with a remote whose refs differ,
  leave `refs/remotes` unchanged.
- **Unchanged behavior.**
  - Help and version still start no transport and open no data directory.
  - C4 and C5 verbs still answer `command_unavailable`.
  - Test children never inherit the developer's terminal, keys, Git
    configuration or application data.
  - The CLI still has no display or desktop dependency, spawns no thread and
    leaves no process behind.
- **Documents.** The CLI RFC, the relationships RFC, the Wave, the API audit,
  AGENTS.md and the `file-ticket` skill carry the amendments the design
  lists.

## Product-Owner Decisions (2026-10-24)

Answered before review:

1. **No defect first.** `01M4PB82`, `01M4PB80` and `01M4PB7Y` are known
   limits, not prerequisites. C3 proves index loss and whole-file
   corruption, which the library recovers today.
2. **The save patch is filled by the CLI from a read.** This follows the CLI
   RFC and F2's split. Its cost is the replay limit under Explicit Exclusions.
3. **Dogfooding in the final task, with the owner's go-ahead at that step.**
   The branch-built CLI records these dependencies. Each is a commit on that
   ticket's own branch, which is then pushed:
   - C4 on C3 and on `01M57SKVF6TQ3QM6AXJ35JGGSY`;
   - C5 on C4 and on `01M57SKVF6TQ3QM6AXJ35JGGSY`;
   - D4 on D3;
   - D5 on D4 and on `01M57SKVF6TQ3QM6AXJ35JGGSY`;
   - D6 on D5;
   - G1 on C5 and D6.

   AGENTS.md and the `file-ticket` skill switch ticket create and update to
   the CLI.
4. **Shape.** One Cycle in two Parts, each with a gate that includes
   `--all-features` and an independent review:
   - **(A)** the shared CLI changes, `path observe`, `ticket check`,
     `index refresh`, `index rebuild`, `folder create`, and the document
     create, save and move verbs;
   - **(B)** the ticket create, save and slug-assign verbs, both repairs, the
     exit evidence and the dogfooding switch.

Answered after the first review round:

5. **The fill read's answer passes through.** When the save patch's read
   fails, its own answer passes through under the mutation's name: its code,
   item scope and recovery actions, with the request ID kept. An item of the
   other kind answers `item_not_found`, as C1's `show` builds it. The CLI
   RFC's refusal rule is amended for this.
6. **Exit 4 evidence.** Exit 4 is listed in the exit table's reasons, backed
   by the test in which a library actor leaves a save partial and the CLI
   completes it. Exit 5 is reached through the binary.

Answered after the second review round:

7. **Accept the fill read's different answers, and document them.** On a
   repository the index does not hold, a save that omits a field answers the
   read's `repository_not_registered` with no recovery action. A full-field
   save answers `repository_not_enabled` for a never-enabled root, or
   `repository_not_registered` with `index.rebuild` after a lost cache. Both
   cases are tested. The difference is recorded in the known limits and in
   the CLI RFC amendment. A small library ticket, so that reads offer
   `index.rebuild` for a lost registration, was offered as optional and is
   not filed.

The following were stated to the owner before the questions on 2026-10-24,
and the owner did not object. They are submitted for approval with these
documents:

- no `--context` on mutation verbs;
- `path observe` reports what is at a path in the named context;
- bodies travel only as JSON strings;
- identity recovery is a pass-through, with no prompt on saves;
- the CLI fixes C3 needs: array input, usage-error privacy for C3 verbs, and
  each verb made available only in the task that dispatches it;
- the lessons from the C1 and C2 ledgers, listed in the plan's Global
  Constraints;
- library-level actors for the interrupted cases;
- Task 0's audit refresh.

The owner cleaned the G1 worktree on 2026-10-24, after planning found an
uncommitted edit there.

The design proposes engineering rulings R-1 to R-18. Several narrow or
condition the contract or its evidence, or act on the real repository:

- R-5: field optionality;
- R-6: `ticket check`'s semantics and exit status;
- R-11: library-level actors;
- R-12: corruption evidence;
- R-13: what counts as documented examples;
- R-16: the dogfooding procedure, and leaving `working-on-a-ticket`
  unchanged;
- R-18: the identity machine.

Named edit E1 changes pre-existing CLI goldens. All of these are submitted
for approval with these documents.

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning, tickets | Three open F2 defects touch C3's exit evidence: partial database damage, a repeated committed move, and an interrupted save's rejected row. | Decision | Decision 1. |
| Planning, RFC and source | The CLI RFC makes save a patch. The library takes a complete draft, and F2's design leaves the patch to the CLI. | Decision | Decision 2. Design rulings R-4 and R-5. |
| Planning, Wave | The Wave asks C3 to switch dogfooding and to record this project's Wave 03 dependencies, but does not say when or how. | Decision | Decision 3. |
| Planning, size | Thirteen verbs, two of them confirmed. | Decision | Decision 4. |
| Planning, tickets | The multi-item design says to land before C3, and to give C3's mutations an optional `--context`. Its later ticket text and the note on this ticket say the opposite. | Settled | The later text governs. Stated to the owner on 2026-10-24, with no objection. |
| Planning, source | C2 hides clap's usage text only for verbs delivered by C2 (`parse.rs:120`). | Ruling | Design ruling R-2. |
| Planning, source | `InputObject` has no array accessor. | Ruling | Design ruling R-3. |
| Planning, source | The interrupted cases cannot be produced through the binary, because the failure hook is in-process. | Ruling | Library-level actors (design ruling R-11). |
| Planning, source | C2's exit table is pinned to its own audit section and counts. | Ruling | A separate C3 table (design ruling R-10). |
| Planning, worktree | The G1 worktree held an uncommitted title edit. | Settled | The owner cleaned it on 2026-10-24. Dogfooding keeps a preflight. |
| Independent review, contract lens (2026-10-24, `6c25c43`) | 38 findings: 2 blocking, 21 important, 15 minor. Blocking: the fill's refusals broke the CLI RFC's closed refusal rule, and `ticket create` refused `null`, which the relationships RFC accepts. Important findings changed the dogfooding procedure (preflight before asking, value checks, stop and resume rules, closure gating, the switch's scope and its own review). They also added replay cases for every mutation, cache-loss retries, both cycle kinds, a piped repair source, a never-fetch check, a progress check, and the note's wording. | Decision, ruling, settled | Decisions 5 and 6. `null` on create accepted (R-5). The other findings were applied to the three documents. Two were not applied, because the source showed otherwise. No. 33: a move to the document's own path is a no-op when it has a context (`document.rs:550`). No. 10: a malformed `project` makes the ticket nonconforming, so the fill never clears it (`canonical.rs:1106`). |
| Independent review, source lens (2026-10-24, `6c25c43`) | 22 findings: 2 blocking, 7 important, 13 minor. Blocking: every write re-emits the front matter, so the byte and no-op claims were false for hand-written files and for the dogfooding check; and the index stores titles, so a title sentinel cannot be absent from the data directory. Important: the R-2 test could not fail; slug assignment's repeat is a no-op; a move to its own path needs a context; `docs/notes` exists; deletion answers `repository_not_registered` and WAL files must go; and unfiltered tests extend to C3. | Ruling, settled | Outcomes are stated by value (design fact 9). Privacy is checked in the request stores (R-17). Tests and fixtures are corrected. One finding was not applied: No. 20(b), the full suite stays in Task 9, because AGENTS.md's 2026-10-24 exception covers `working-on-a-ticket` tickets, not Cycles. The suspected finding No. 9 was applied: the clause "a plain refresh also completes it" was dropped. |
| Scoped re-review with an adversarial round (2026-10-24, `6c25c43..521b91b`) | 22 findings: 1 blocking, 6 important, 15 minor. Blocking: a read never offers `index.rebuild` for `repository_not_registered`, so six places were wrong, and decision 5 gives a save with an omitted field a different answer from a full-field save. Important: the six target branches are not on origin, so the pushes create remote branches and publish existing commits; dogfooding could commit an uncommitted edit; the resume rule lost pushes and could not settle a stopped save; R-17 contradicted mutation data that echoes input; a partial save plus a later edit never completes until C4; the identity retry needs a fresh observation. | Decision, ruling, settled | Decision 7. Design facts 10, 11, 17, 18 and 21 corrected; facts 36 and 37 added. Dogfooding has a build-then-preflight order, a clean check before each save, value checks on the `deps` IDs, a ledger written before sending, a resume that resends unsettled requests under their own IDs, and pushes for every unpublished tip. R-17 exempts the echoing data fields. The limits are qualified, golden names fixed, and the fixtures corrected. Fact 17 is left unconfirmed, as a Task 0 check. The third round reviewed this revision. |
| Third scoped review with an adversarial round (2026-10-24, `521b91b..5dc1921`) | 16 findings: 1 blocking, 3 important, 12 minor. Blocking: the documented remedy for an interrupted save whose omitted field was then edited (resend every field) does not work, because a re-entered save answers `external_change` for any file content but its own bytes or its original observation. Important: a pending save blocks every other request on the repository, not only refreshes; the resume rule could loop or misreport a finished save; the value check could not run after a resume. | Ruling, settled | The third blocker in the same mechanism, so it was not rewritten again. The design states it as an interrupted-saves table of required outcomes, and Task 3 writes that test first and stops at a design checkpoint if the library answers otherwise. Fact 21 extended. Resume starts from `request show`. The ledger records the compared fields with a body digest, and step 5 reads before its clean check. Minors applied: citations, golden names, Task 0's grep, `CLI_ONLY` in Task 4, the "Equal" list, and the resolved repository in `fill::item_not_found`. These edits had no independent review. |

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing
beyond the ticket branch, merging, closing and worktree cleanup each need
their own authorization.
