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

**Draft, for product-owner approval.** The owner answered the four planning
questions on 2026-10-24; they are decisions 1–4 below. Nothing is approved and
no implementation is authorized. No build or test was run for these drafts.

- [Design](../plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-design.md)
- [Implementation plan](../plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-implementation.md)

Ticket: `01M4CC0VMV81DFRGJ4Z76489XM`. The ticket branch merged `origin/main`
at `e49f978` on 2026-10-24 as merge commit `9ed4a04`. It was merged, not
rebased.

## Purpose And Authority

Make local document and ticket authoring, and explicit discovery repair,
available to agents and to this project's own dogfooding
([Wave 03](../Waves/wave-03-dogfooding.md#c3-cli-authoring-and-discovery-maintenance)).
An agent with JSON, or a person at a terminal, can create, save and move items,
create folders, repair and adopt nonconforming files, refresh and rebuild the
index and set ticket relationships, all through `manyhands-cli` and without
editing canonical files by hand.

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
   actions and schemas C3 consumes unchanged, and the design's split that
   leaves the save patch to the CLI ("the CLI RFC's optional title is the
   CLI's concern").
5. The C1 and C2 Cycles and designs: parsing, envelopes, exit statuses,
   isolation, goldens and the mutation driver, which C3 extends.
6. The note on this ticket about multi-item work contexts
   (`01M5NVMGZ1BXK1P15RZNNYX06B`, ticket `01M5NKA3MY7CW4D90M866YS946`).

## Entry Evidence And Dependency Boundary

- **Wave 02.** Cycles 01–04 are closed on main.
- **Wave 03.** F1, F2, C1 and C2 are closed and merged to main. D1 and D2 are
  merged too; C3 depends on neither (track rule 2).
- **Branch.** As under Status. The merge is not yet pushed.
- **Library boundary.** Every C3 verb has an F2 binding or an F1 read:
  - the eleven mutations are variants of `Mutation`
    (`src/repository/mutation/mod.rs:398`);
  - `path observe` is `observe_path` and `ticket check` is
    `check_ticket_relationships` (`src/repository/read/items.rs:1963`, `:2033`).

  C3 changes nothing under `src/repository.rs`, `src/repository/`,
  `src/lib.rs`, `src/results.rs` or `src/runtime.rs`. A gap found during C3
  becomes its own ticket under track rule 5.
- **Known library limits (decision 1).** Three open defects touch C3's
  verbs. None is a prerequisite; each is recorded as a known limit:
  - `01M4PB82G07RZSQMNJZN672S3W`: a database damaged in pages that opening it
    never reads cannot be rebuilt;
  - `01M4PB80T7SVB23TQ6466RDHCB`: the domain refuses a repeat of a committed
    document move. F2's binding works around it, outside the lease;
  - `01M4PB7YZMBFAFFM94AZY08HWS`: an interrupted save whose row recorded no
    step is closed as `rejected` although its file was written.
- **Multi-item work contexts.** Ticket `01M5NKA3MY7CW4D90M866YS946` lands
  after C3. Its ticket text (2026-10-23 19:30) says that C3 does not wait for
  it. Its design (17:05) says "Land before C3 plans its authoring verbs" and
  that C3's mutation verbs "should take an optional `--context`". C3 follows
  the later ticket text and the note on this ticket; see Explicit Exclusions.
- **Entry-gate item 3.** The API audit has no C3 refresh section. Task 0 adds
  one.
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
- **Confirmation.** `document repair` and `ticket repair` are confirmed. Their
  previews state the adoption of an ID. The other nine mutations are
  unconfirmed and answer `not_confirmable` to `--prepare`, as F2 bound them.
- **Mutation input.** As C2 decision 2: selectors are flags; everything else is
  one JSON object from `--input`. Bodies and sources are JSON strings. Each
  input-taking verb publishes a closed input schema and an example input.
- **Save patch (decision 2).** `document save` and `ticket save` treat an
  omitted field as unchanged and `null` as clearing a nullable field. The CLI
  reads the item and fills the omitted fields before it calls the library,
  passing the caller's observation through unchanged.
- **Relationships.** `ticket create` and `ticket save` take `deps` and
  `parent`. `ticket create` and `ticket slug-assign` take optional `initials`.
  `ticket check` validates proposed relationships before a write.
- **Identity recovery.** A save without a usable identity answers
  `identity_required` with the `repo.identity_set` recovery action. After
  `repo identity-set`, the same request ID succeeds. C3 adds no identity
  prompt to saves.
- **Discovery maintenance.** `index refresh` and `index rebuild` run one
  explicit local operation and exit. `index rebuild` recovers a lost or
  replaced database. `path observe` returns the observation that creation,
  move and repair need, including the absent token for an empty path.
- **Dogfooding (decision 3).** In the final task, with the owner's go-ahead at
  that step, the branch-built CLI records the Wave 03 Cycle dependencies on
  this project's open tickets. AGENTS.md and the `file-ticket` skill then
  direct ticket create and update to the CLI.
- **Output.** Library envelopes pass through unchanged. Human output for every
  verb, with progress on stderr as in C2.

## Explicit Exclusions And Downstream Obligations

- No comment, synchronization, conflict, promotion, closure, polling or draft
  verb. `comment add` and `operation abandon` stay with C4; ticket comments
  stay canonical files until C4.
- **No `--context` on mutation verbs.** Each request's target context stays
  implied by its item ID, as the library derives it. An optional `--context`
  can then be added later without changing an existing input's meaning
  (multi-item ticket `01M5NKA3`, Level 2). C3 does not build on the usage error
  "--context requires --path." either.
- No interactive prompt beyond C2's consent prompt, which the two repairs use.
  No identity prompt on saves; no `$EDITOR`; no body through argv, the
  environment or a file named in the input.
- No background work, network contact or publication. `index refresh` and
  `index rebuild` never fetch and never rewrite canonical state.
- No library change. The defects C3 exposes keep their own tickets (decision
  1). The partial-damage form of index corruption is not recovered by
  `index rebuild` until `01M4PB82` lands.
- **The save patch's replay limit (decision 2).** Suppose a third party
  changes an omitted field after a save succeeded. A retry of that request
  then computes a different input and answers `request_mismatch`, not the
  stored result. `request show` still returns the stored result, and no
  effect is repeated.
- Status text `closed` is ordinary metadata. Neither save nor create can
  write closure fields; `ticket close` belongs to C5.
- Empty folders are local and unlisted by Git until they hold tracked content.
  No placeholder commit.
- Native evidence. C3 is proven on Linux. Its tests are in the existing
  `cli` target, which is already in the native workflow's list. Native CI runs
  on main after merge, with its own approval.

## Required Contract

1. **Grammar.** C2's grammar, with the selectors in the design: `--repo` on
   every C3 verb; `--id` on `document save`, `document move`, `ticket save`,
   `ticket slug-assign` and `ticket check`; `--path` and an optional
   `--context` on `path observe`; `--deps` and `--parent` on `ticket check`.
2. **Input.** C2's rules: one UTF-8 JSON object, read once; unknown,
   duplicate and mistyped fields, and missing required ones, are
   `invalid_input`. Missing and `null` differ where the design says so. No
   diagnostic the CLI writes contains an input value, a body or a source.
3. **The save patch.** An omitted draft field takes the item's current value
   from one `show_item` read. The read happens only when a field is omitted.
   The caller's observation goes to the library unchanged, and the library
   alone decides staleness, replay and mismatch. A read of an item of the
   other kind, or one without the field, answers `item_not_found`, as the
   binding's own read does. `deps` and `parent` are never filled: omitted
   means unchanged in the library.
4. **Request identity, consent, retry and refusals.** As C2's Required
   Contract items 3, 4, 8 and 10, unchanged.
5. **Never printed.** No body, source, secret, backend text, SQL or `Debug`
   rendering, in either mode. Library envelopes carry no body.
6. **Reads stay reads.** `path observe` and `ticket check` take flags only.
   They answer from the index and the file system, write nothing and contact
   no network. `path observe` reports a managed file of another item in the
   named context as what is there, never as outside the context.
7. **Index commands.** `index refresh` and `index rebuild` take no input. They
   are mutations under F2, so JSON and non-interactive calls need
   `--request-id`.

## Acceptance And Exit Evidence

| Wave exit evidence | Required proof |
| --- | --- |
| Offline checkpoints and no-ops | Through the binary, in a repository with no remote: `document create`, `document save`, `document move`, `ticket create`, `ticket save` and `ticket slug-assign` each commit one checkpoint on the item's branch. Effects say `publication: not_requested` and the exit is 0. A fresh request that writes what the file already holds answers `noop` and makes no commit. A save that omits every draft field changes no byte of the file. |
| Stale writes | A stale observation on `document save`, `document move`, `ticket save`, `ticket slug-assign` and on repair's `--confirm` answers `external_change`, exit 3. The file's bytes, the branch tip and the worktree are unchanged. |
| Invalid and malformed repair | A corrected source that is not conforming is refused at prepare with the binding's code, and nothing is written. A path that holds no nonconforming entry is refused. A corrected ID that another item holds is `identity_ambiguous` and creates no context. A repair keeps a closed ticket's closure fields. |
| Adoption retry | A marker-only document is adopted under the ID its corrected source names. A library-level actor stops a repair after its write; the CLI completes it under the same request ID and confirmation, with one commit and the same ID. A replay of the finished repair writes nothing. |
| Move collision | A file at the destination is `occupied_path`, exit 2. A destination token from another context, or from before a file arrived, is `external_change`. Source and destination are unchanged in both cases. |
| Index loss and corruption | After the database file is deleted, and again after it is overwritten with bytes that are not a database, `index rebuild --repo` restores it: `document list` and `ticket list` list every item again, and canonical files, Git refs and the worktree are byte-identical before and after. The partial-damage form is a known limit (decision 1). |
| Commit success, refresh failure | A library-level actor saves with a failure at `BeforeIndexTransactionCommit`, and gets a partial result with the checkpoint committed. The CLI then completes discovery under the same request ID with no second commit. A plain `index refresh` also completes it. |
| Schemas and documented examples | Every input-taking C3 verb has a closed input schema under `schemas/input/v1/` and an example input under `tests/fixtures/cli_input_v1/`. Each example validates against its schema and runs through the binary to its documented outcome. Every C3 verb has JSON goldens checked against the published output schemas, and both repairs have a preview golden. |
| Dogfooding switch | With the owner's go-ahead in the final task, the branch-built CLI refreshes this repository's index and records the Wave 03 Cycle dependencies with `ticket save` (design, Dogfooding). `ticket blocked` then lists each recorded blocker. AGENTS.md and the `file-ticket` skill direct ticket create and update to the CLI. |

Also required:

- Every exit class a C3 verb can produce is reached through the binary. The
  codes no fixture can reach are listed with their reasons. The table plus
  that list equals, by set equality, the code set Task 0's audit refresh
  records for C3's verbs.
- Every C3 verb runs through the binary in human mode as well as JSON. Every
  verb that can be a no-op has a no-op case. A no-op is a fresh request that
  writes nothing, not a replay of a finished one.
- `identity_required` on a save offers `repo.identity_set`, and the same
  request succeeds after `repo identity-set`. These cases need a test machine
  whose system Git configuration sets no identity, as C2's ruling R-17 has it.
- Relationships: a dependency cycle and a parent cycle are refused with
  `relationship_cycle` before any context exists; an unknown target is
  accepted and reported as unresolved; `ticket check` reports the same
  rejection without writing; a closed ticket's save and slug assignment
  answer `ticket_closed`; a ticket with a code answers
  `slug_already_assigned`.
- The Wave's mutating-Cycle evidence row (`MH-COLLAB-006` and the NFRs) maps
  as in C2:
  - **progress:** human `progress:` lines, and empty JSON stderr;
  - **consent:** prepare, confirm and interactive cases for the two repairs;
  - **safe-point cancellation:** C3's local commands have no safe point after
    acceptance (F2). Cancellation is shown by `cancelled` at the repair's
    consent prompt;
  - **redaction:** a sentinel body and source never reach stdout, stderr or
    the data directory;
  - **stale writes:** as in the table;
  - **retry:** replay, `request_mismatch` and the completion of a stopped
    request under its own ID.
- Help and version still start no transport and open no data directory. C4
  and C5 verbs still answer `command_unavailable`.
- Test children never inherit the developer's terminal, keys, Git
  configuration or application data.
- The CLI still has no display or desktop dependency, spawns no thread and
  leaves no process behind.
- The CLI RFC, the Wave, the API audit, AGENTS.md and the `file-ticket` skill
  carry the amendments the design lists.

## Product-Owner Decisions (2026-10-24)

1. **No defect first.** `01M4PB82`, `01M4PB80` and `01M4PB7Y` are known
   limits, not prerequisites. C3 proves index loss and whole-file corruption,
   which the library recovers today.
2. **The save patch is filled by the CLI from a read.** This follows the CLI
   RFC and F2's split. Its cost is the replay limit under Explicit Exclusions.
3. **Dogfooding in the final task, with the owner's go-ahead at that step.**
   The branch-built CLI records these dependencies, each a commit on that
   ticket's own branch, which is then pushed:
   - C4 on C3 and on `01M57SKVF6TQ3QM6AXJ35JGGSY`;
   - C5 on C4 and on `01M57SKVF6TQ3QM6AXJ35JGGSY`;
   - D4 on D3;
   - D5 on D4 and on `01M57SKVF6TQ3QM6AXJ35JGGSY`;
   - D6 on D5;
   - G1 on C5 and D6.

   AGENTS.md and the `file-ticket` skill switch ticket create and update to
   the CLI.
4. **Shape.** One Cycle in two Parts, with a gate that includes
   `--all-features` and an independent review after each:
   - **(A)** the shared CLI changes, `path observe`, `ticket check`,
     `index refresh`, `index rebuild`, `folder create`, and the document
     create, save and move verbs;
   - **(B)** the ticket create, save and slug-assign verbs, both repairs,
     the exit evidence and the dogfooding switch.

Stated to the owner before the questions on 2026-10-24, and not objected to.
They are submitted for approval with these documents:

- no `--context` on mutation verbs;
- `path observe` reports what is at a path in the named context;
- bodies travel only as JSON strings;
- identity recovery is a pass-through, with no prompt on saves;
- the CLI fixes C3 needs: array input, usage-error privacy for C3 verbs, and
  availability only in the task that dispatches a verb;
- the lessons from the C1/C2 ledgers, listed in the plan's Global
  Constraints;
- library-level actors for the interrupted cases;
- Task 0's audit refresh.

The design proposes engineering rulings R-1 to R-18. Rulings R-5, R-6, R-12
and R-18 narrow or condition the contract or its evidence. Named edit E1
changes pre-existing CLI goldens. All three are submitted for approval with
these documents.

## Review Record

| Source | Concern | Classification | Resolution |
| --- | --- | --- | --- |
| Planning, tickets | Three open F2 defects touch C3's exit evidence: partial database damage, a repeated committed move, an interrupted save's rejected row. | Decision | Decision 1. |
| Planning, RFC and source | The CLI RFC makes save a patch. The library takes a complete draft (`TicketDraft`, `DocumentDraft`), and F2's design leaves the patch to the CLI. | Decision | Decision 2. Design rulings R-4 and R-5. |
| Planning, Wave | The Wave asks C3 to switch dogfooding and record this project's Wave 03 dependencies, but does not say when or how. | Decision | Decision 3. |
| Planning, size | Thirteen verbs, two confirmed. | Decision | Decision 4. |
| Planning, tickets | The multi-item design says to land before C3 and give C3's mutations an optional `--context`; its later ticket text and the note on this ticket say the opposite. | Settled | The later text governs. Stated to the owner on 2026-10-24, not objected to. |
| Planning, source | C2 hides clap's usage text only for verbs delivered by C2 (`parse.rs:120`), so a C3 verb's usage error could echo an input value. | Ruling | Design ruling R-2. |
| Planning, source | `InputObject` has no array accessor, so `deps` cannot be read. | Ruling | Design ruling R-3. |
| Planning, source | The interrupted cases cannot be produced through the binary: the failure hook is in-process only (`open_at_with_failure_point_for_testing`, `src/repository.rs:4490`). | Ruling | A library-level actor sets them up over the CLI's data directory. Design ruling R-11. |
| Planning, source | C2's exit table is pinned to its own audit section and count (`tests/cli/exits.rs:601`). | Ruling | C3's table is separate. Design ruling R-10. |
| Planning, ticket | The ticket's Added Scope still names initials and the prefix, which C2 delivered. | Settled | The ticket text is corrected in Task 9. |

Record planning, per-task progress, decisions, baseline and final
verification, review and review-ready status as ticket comments. Publishing
beyond the ticket branch, merging, closing and worktree cleanup each need
their own authorization.
