# C4 Owner Checklist

This list is for the owner's decisions and checks on Wave 03 C4 (CLI discussion, synchronization and conflict recovery) before the Cycle is called review-ready. C4 is a CLI Cycle, so there is no on-screen check: section 1 is a read of the human output, and section 2 is the dogfooding go-ahead. Tick each box as you go, and write each answer in the table in section 5.

## Before you start

- The worktree is `/home/vhodges/work/src/manyhands/.manyhands/worktrees/01M4CC0VMWEQQ9G4W0DAG7ADPY`, and the branch is `manyhands/ticket/01M4CC0VMWEQQ9G4W0DAG7ADPY`.
- The human-output goldens are plain text under `tests/fixtures/cli_v1/`. Each one is the exact text a verb prints. Reading them needs no build.
- To try a verb yourself, build the branch's CLI once with `devenv shell -- cargo build --locked --bin manyhands-cli`, then run `target/debug/manyhands-cli <verb> --help`.

## 1. Human output

Read each golden, and judge whether the text reads well to a person at a terminal.

- [ ] **Read:** `comment_add_text.txt`, `item_sync_text.txt`, `repo_sync_text.txt`, `conflict_show_text.txt`, `conflict_resolve_text.txt`, `item_republish_text.txt`, `operation_resume_text.txt`, `operation_abandon_text.txt` and `help_root_text.txt`. **Expect:** each `next:` line is a command you could paste and run, and each `note:` reads as plain advice.
- [ ] **Decide: label case.** The `conflict show` lines and the local-sync data lines are lowercase (`operation:`, `state:`, `next action:`, `path:`), as C2's metadata lines are. The generic field list used by `comment add`, `operation abandon` and the other results is Title Case (`Comment id:`, `Pending reason:`, `Action:`). A single `comment add` result can show both. Choose one: keep both as they are; make all of them lowercase; or make all of them Title Case. (Part B review M5; final CLI review, recommendation 4.)
- [ ] **Decide: conflict structure labels.** For a path that is no item's, `structure` comes from the conflict gate's classifier. A text code file edited on both sides reads `binary`, and a file both sides added reads `delete`. The documents now say what `structure` means there (Task 12b; the RFCs' conflict notes). Choose one: accept this, documented as it is; or ask for a follow-up ticket for clearer labels.
- [ ] **Decide: nested conflict fields.** A `comment add` whose publication stopped on a conflict prints the nested `Conflict …` fields and also the `conflict:` line, so the conflict shows twice (final CLI review, minor 8). Choose one: accept it; or ask for a follow-up ticket to hide the nested fields.

## 2. Dogfooding go-ahead (Task 15 Step 3; design "Dogfooding", R-26)

With your go-ahead, the controller runs the design's Dogfooding steps 1 to 3 against the registered main checkout:

1. **Preflight.** Any failed check stops for you:
   - the C4 worktree is clean and pushed, and every edit in it is committed;
   - local `main` equals `origin/main`;
   - the ticket's context is registered in the main checkout's index;
   - the selected key can be used. A protected key makes non-interactive mode answer `unlock_required`, and then you run this step at a terminal;
   - C4 adds no migration of the shared application database.

   Whether a publication remote is configured is recorded.
2. **One verification comment** on the C4 ticket, through the branch-built CLI's `comment add`, with `created_by` `Claude Opus 5.5 <noreply@anthropic.com>`. Its request ID is recorded before it is sent. With a publication remote, publishing merges local main into the ticket context and pushes the ticket branch. The go-ahead covers that. A refusal or a partial result stops for you.
3. **A check by value** with `comment list`.

Then, under the same go-ahead:
- **Step 4, the switch.** AGENTS.md and the other files of fact 48 direct comments to `comment add`. AGENTS.md states the extended exception for `working-on-a-ticket` and `scripts/ticket-loop` (decision 15). The switch gets its own review.
- **Step 5, review-ready.** The Cycle and design are marked review-ready, `merge-tree` is checked, and the review-ready comment is added through `comment add`. The ticket stays open.

- [ ] **Decide:** go ahead with dogfooding, steps 1 to 5? Or say if you want to run step 2 at a terminal yourself.

## 3. Follow-up tickets

Each candidate is a Known Limit already written into the documents. Creating a ticket needs your go-ahead.

- [ ] **A comment stopped for an untrusted host** offers no `host.approve`, only `request.retry`, and its request keeps the reservation. The documents name the way out, which the final fix checked through the binary:
  1. `host observe --repo` names the host and the key it presents.
  2. `host approve` or `host replace` works while the reservation is held.
  3. The same `comment add` request, sent again, then publishes.

  The candidate follow-up: offer `host.approve` from the comment's own answer. (Final library review, issue 2; Tasks 2b, 4a and 4b.)
- [ ] **A filesystem refusal during `conflict resolve`**, after a path was written, answers `internal_error` (exit 4) with no `next:` line. `conflict show` names `retry_resolution`. The candidate follow-up: print that `conflict show` line. (Final CLI review, recommendation 3.)
- [ ] **Move the three exit-evidence cases** out of `tests/cli_remote/interruption.rs` into `tests/cli_remote/exits.rs`, as `c4_exits_*`. (Final CLI review, recommendation 2.)

## 4. Still gated on you

- [ ] The two defect tickets C4 opened on main's behalf, each on its own pushed branch:
  - `01M669FJR27FRHCNPWAMD6VH4V`, remote selection and the Git index;
  - `01M66HFV44YXNQ7ANJVA16TQ1H`, a key selected after a publication remote blocks first publication.

  Each needs a session, or your scheduling.
- [ ] Notes on the C5, D3, D4 and D5 tickets about what C4 delivered and changed for them.
- [ ] `ticket save` of the C4 ticket's planning boxes.
- [ ] Native CI after merge. `interrupt/windows.rs` is compiled for the first time on the Windows runner. The margin notes are in the execution ledger: the 500 ms press window, the 5 s latch stall, and the poll case opening the registry beside the CLI child.
- [ ] Code review approval, then closure of the ticket in the final push before merge, then the merge.

## 5. Results

| Item | Answer |
| --- | --- |
| 1. Goldens read well | |
| 1. Label case | |
| 1. Structure labels | |
| 1. Nested conflict fields | |
| 2. Dogfooding go-ahead | |
| 3. Untrusted-host follow-up | |
| 3. Filesystem-refusal follow-up | |
| 3. Exit-evidence move | |
| 4. Main-defect tickets | |
| 4. C5, D3, D4 and D5 notes | |
| 4. `ticket save` | |

## 6. Rulings made during execution

While executing the C4 plan, the controller ruled on about 100 questions without stopping, as the execution method requires. The owner can overturn any of them. Each line says what was decided and what it costs if wrong. Section 6.1 lists rulings a user or an RFC reader would notice. Section 6.2 lists rulings on process and scope. Section 6.3 lists rulings the controller later reversed or corrected. The full record is the execution ledger, `docs/plans/2026-10-26-wave-03-cli-04-discussion-and-sync-execution.md`.

### 6.1 Rulings that change behaviour or documents

- **Task 1, busy sync.** A new item sync on the same item, or while the primary's merge is pending, answers `busy`, not `item_not_found` or `worktree_conflicted`, through a read-only check in `bind/sync.rs`. A merge pending under promote or close keeps the domain's answer (C5's). Cost if wrong: one mapping and its cases to revert.
- **Task 1, what a sync reports.** A lost push that a peer then built on answers `already_applied` and does not credit the earlier push. A sync that made a merge commit names it in effects (`integration: complete`) whether it succeeds, stops or is cancelled. `conflict_next` says `resolve` only when every path is `EligibleCanonical`. Cost if wrong: an agent is not told the earlier attempt pushed; one extra effect field.
- **P7, abandoning a comment.** Fact 10's stop does not trigger. A comment whose local row is still pending can be abandoned; the binding is spent and the commit stays unpublished until item sync (existing F2 behaviour, `abandon.rs:1273`). Cost if wrong: Known Limits lacks one line.
- **Tasks 2a and 2b, comment add input checks.** On the binding path a bad parent is refused before the binding row is written. `CommentAuthor::parse` applies `ValidIdentity::of`'s rules except the Git-configuration round trip (R-7). Cost if wrong: the public path keeps a binding row for a bad parent; `parse` would need a scratch file.
- **Task 2b, refusals that wrote nothing.** A refusal after the binding that wrote nothing releases the binding and rejects the row in one transaction. An Input-class refusal (`occupied_path`; `item_not_found` from nonconforming canonical content) also finishes the record with recovery none, and a re-send replays it (exit 2). Cost if wrong: one standing arm each.
- **Tasks 2b and 6 (M9), `remote_branch_deleted`.** A comment add stopped this way stays accepted (`Owed`) and offers `request.cancel`; cancel, then the same request again, ends it `cancelled` and frees the reservation. A conflict-resolve continuation offers `request.cancel` naming the holder's request. R-34's "final" record is replaced; Task 14 amends R-34 and row 931. Cost if wrong: one standing arm and one case.
- **Tasks 2b, 6 and 12a, human output after `remote_branch_deleted`.** Three `next:` lines (request cancel, the same request again, item republish), not the design's two, because cancel alone leaves republish `busy`. `operation resume` after a deletion gets a human-mode note, since its JSON offers no recovery action. Cost if wrong: one extra line or note.
- **Tasks 9 and 10, other human notes.** The `busy` note for item republish names the step, not a request ID, because no public read gives the holder's ID (Task 14 amends row 13). `conflict resolve` answering `original_request_required` notes that written paths were edited, not Task 7's supersede note. Cost if wrong: the user looks up their own request ID; one note's wording.
- **Task 3, `conflict show` answers.** A synchronization with no recorded conflict is `invalid_input` in any phase. A `ResolutionPrepared` whose inspection fails answers the inspection's code. A Git lock reads `recovery_required`; `repair_refused` is for decided refusals only. A missing `MERGE_HEAD` still reads `repair_refused` where the restart says `recovery_required` (accepted). Cost if wrong: Task 14's two new rows; transient errors read as refusals.
- **Task 4a, superseded resolution attempts.** Their rows (paths, index and ref-log artifacts, attempt, owner) are deleted in one transaction; the staging directory stays. Task 14 amends the design sentence at line 1196. Cost if wrong: superseded attempts leave no registry trace.
- **Task 4a, cancelling a resolve.** The continuation also reads the holder's cancel flag (and a comment add's), so the design's step 6 holds. A cancel after push intent stops before the push and resolve answers `cancelled`; the holder's next call answers `recovery_required`. Cost if wrong: one extra flag read; `recovery_required` where `cancelled` would do.
- **Tasks 4b and 6 (M3), `operation resume` rows.** A cancelled comment publication answers `recovery_required` (row 8); other cancelled operations keep F2's `already_applied`. A fenced comment publication resumed by either ID answers `external_change`. Resume of a comment checkpointed with no remote is pinned (design line 938). Cost if wrong: about five lines in `found_comment`; one call.
- **Task 5, how `item republish` ends.** `push_rejected` finishes the record, so a held reservation cannot block the `item sync` the user needs. It offers no recovery action (`outcome.rs`, checked in Task 14). A refused republish ends its operation `cancelled`, so operation reads show `cancelled`. A restart that finds its operation ended answers `RecoveryRequired`, as item sync does. Cost if wrong: one standing arm; for `cancelled`, a schema change C4 may not make (fact 75).
- **Task 5, `Deleted` push evidence.** Evidence classified `Deleted` cannot tell present from deleted, so a preview is offered and execution refuses `external_change` with nothing pushed (a Known Limit). Row 15's host stop is `host_replacement_required`. Cost if wrong: a misleading preview, no wrong push.
- **Task 6 I1, hand-edited written path.** The wedge stays a Known Limit, since the library's refusal to overwrite a hand edit is outside C4's files. The user restores each written path to the request's resolution, then the same request continues or a new one supersedes. A refusal that provably wrote nothing closes its own attempt. Cost if wrong: a user who hand-edits must restore the file.
- **Task 6 M2, reused request ID.** `own_attempt` looks for the live step's attempt first, so resending an earlier stage's input while a later stage is live answers `external_change`. It is fixed by code and untested. Cost if wrong: a reused ID answers `request_mismatch`, F2's literal reading.
- **Tasks 7, 11 and 13, Ctrl-C.** Ctrl-C during the held fetch answers `cancelled` (130), 4 runs of 4; accepted, no owner stop. A second press, then a NEW request ID, answers `busy` until the killed request is sent again (C2 Known Limit 01M4R19PC0T6ZPKM39X9DF26HX). The latch case answering `push_rejected` is R-36 behaviour. An inherited `SIG_IGN` is kept on Unix. Cost if wrong: one doc sentence, one expected answer, one window branch.
- **Task 12b, `structure` of a noncanonical path.** A text code file edited on both sides reads `structure: binary`, and an add/add reads `delete`. That is the gate's classifier, outside C4, so it is not a stop. Task 14 adds an RFC sentence. Cost if wrong: reader confusion only.
- **Task 14, document amendments.** Four amendments where delivered behaviour contradicted RFC or design text; a dated C4 amendment to Cycle item 9 and its Ctrl-C acceptance row; the fenced-comment resume row; the D24 sweep. Two Importants were fixed: Known Limits wrongly said `request cancel` alone frees a reservation, and the RFC claimed sides for every item path. Cost if wrong: dated doc sentences.
- **Task 15 I1, refused republish push.** Fixed now, not left as a Known Limit as the reviewer suggested. `acknowledge_refused_push` sets outcome `protocol_rejected` and `push_absence` ignores that cancelled row, so `operation show` reports `failure_code protocol_rejected`. Only a branch published outside this registry looped. Cost if wrong: an evidence regression the re-review and full suite must catch.
- **Task 15 I2 and other Known Limits.** A comment publication stopped for an untrusted host offers no `host.approve`; the way out is `host observe`, then approve or replace while held, then the same comment add. The other new Known Limits:
  - two requests resolving one conflict at once can leave an attempt with no owner (Task 6 M8; no data is lost);
  - a filesystem refusal on a written path answers `internal_error` with no `next:` line (12b).

  The nested `Conflict` fields in `comment add`'s human output went to the owner checklist instead. The follow-ups are the owner's to ticket, since creating a ticket is gated.

### 6.2 Rulings on process and scope

- **Test runs (P1, Tasks 6, 11, 15).** Task 0 skips the full suite; each task runs targeted tests. The full suite runs once, at Task 15 on the final tree after the final fix. A gate cites an earlier full run when only docs, comments, `src/bin/manyhands-cli` or `tests/cli*` changed since. Cost if wrong: a pre-existing failure appears only at Task 15; one more run of about an hour.
- **Files outside a Files list (P2, P3, Tasks 1, 2a, 9, 12b).** Tasks 4a and 5 may edit `conflict_read.rs`, `bind/resume.rs` and their test files. A doc line in `bind/resume.rs` and `tests/support/repair.rs` is in scope. Task 12b's shared `save` and one Task 7 test literal are C4 files. `CommentAuthor` moved from 2a to 2b, since clippy denies dead code. Cost if wrong: none.
- **Owner stops (P4, P5, Tasks 4b, 7).** Implementers write each task's completion comment and push; fix rounds append to the execution ledger. The plan's owner stops stay with the owner; "stop for a ruling" is a controller ruling, as for Task 4b's row 8 and Task 7's spike. Cost if wrong: an avoidable pause.
- **Defects in C2-era code (Task 12a).** `remote select` leaves the Git index stale; the owner chose "ticket it, C4 carries on" (01M669FJR27FRHCNPWAMD6VH4V), so `CliPair` seeds the primary with the remote selected. The history-unknown defect also became a ticket (01M66HFV44YXNQ7ANJVA16TQ1H). Cost if wrong: a fixture hides the index defect.
- **Desktop-alone clippy (P6).** The gate runs the `--bin manyhands` forms and compares `--all-targets` with main's 58 errors, a main failure outside C4. After the D3 merge it showed 77; the 19 new were D3's, by inference, not a main build. It passes after `f487cc74`. Cost if wrong: a C4 test-only dead-code error could hide; one cold build.
- **Conformance split (owner, 2026-10-28; Tasks 4a, 5).** The remote conformance case is split: republish's row is its own case, `every_command_of_this_target_keeps_the_rules_of_the_boundary` is split per row, and Task 5's world is publish-then-delete. Task 14 notes it in Named Edits. Cost if wrong: more cases; one rule's coverage narrows.
- **Merges of main.** Task 13 was committed on its base and main merged separately, keeping its review range pure. Merged: `bda0a2d1` as `2f2b1f77` (six additive conflicts), `f487cc74` as `cec0ce42`, `48ffc058` as `a58aef28`; the gate reran on the merged tree. Cost if wrong: none.
- **Review handling.** Minors are folded into fix rounds. Task 1's Minor 4 was reclassified Important. Approved tasks with only folded minors were checked inline, not re-reviewed (Tasks 9, 10, 11, 12a, 12b). Shared code is preferred to copies (Tasks 3, 5, 7). Cost if wrong: the final review sees it.
- **Test pins and harness choices.** `outcome_tests.rs` hits stay unedited (P8). R-38's "checked before busy" cannot be co-staged. Task 4a adds the R-14 collision case. A Drop guard restores the hand-edit directory's mode (12b). `confirmation_expired` stays a reason (13). Cost if wrong: one case or its expected answer.
- **Task 8 attribution.** The commit names Opus 5.5 though Sonnet implemented it; it stays, since fixing it needs a force push. From Task 9, implementers use their harness's own attribution line. Cost if wrong: one line per commit.
- **Final review (Task 15).** Two independent reviewers by side, because the net diff is 42k lines. The final fix takes CLI M1 to M7, L3 and L4, and leaves M9, M10 and library minors 5, 6 and 8. The label-case mix, M8 and Task 11's M5 go to the owner checklist. Cost if wrong: a cross-side interaction neither sees.
- **Native CI owed.** Windows-only code is not compiled locally (Task 4a's sentinel release, Task 7's `interrupt/windows.rs`), and Windows watchdog margins are noted (12a, 13). Cost if wrong: found after merge.

### 6.3 Mistakes the controller corrected

- **Task 3, concern 8.** The first ruling gave `sides` only for paths whose structure reads `regular`, dropping the sides an external repair of `incompatible_closures` most needs. It was refined: sides for any item path, none for a non-item path.
- **Task 6, fix-round concern 1.** The controller's I1.3 ruling (a message naming the changed path) broke the Global Constraint that the JSON message is always `ResultCode::message()`. `Answer::message` was removed; the escape moved to Task 9's human output and Task 14's Known Limit.
- **Task 9, minor 1.** The controller's Task 6 wording said to restore a path to the resolution "or its pre-write content". The library requires a written path to hold the request's resolution; only an unwritten one may keep conflicted content. The notes and the Known Limit carry were corrected.
- **Task 12a, I-1.** The controller's carry "republish preview not busy" proved nothing, because a preview takes no reservation. The case now confirms a real republish and asserts `ok`, the server ref and one accepted push.
- **Task 2b, concern 2.** The ruling that a refused, unwritten comment add leaves "the record stays" was superseded for the Input class by I1, where the record finishes. Ticket comment 01M62KA2F519YGJ71XADTDX9N7 said `remote_branch_deleted` finishes; Task 6's comment corrects it.
- **Task 6, M1.** New evidence on the 4a concern-2 ruling: deleting a superseded attempt's rows while the step stayed `resolution_prepared` let a refused superseding request block outside repair, against decision 18. The close now also resets the step to `conflict_pending`.
- **Task 7, minor 4.** The controller's ruling said C2 answers `confirmation_required` before the library. A test comment copied it; the library's consent gate (`confirm.rs:402`) answers. The final fix list includes a consent-gate comment.
