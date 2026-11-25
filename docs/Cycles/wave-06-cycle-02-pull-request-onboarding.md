---
id: 01M89CHP8JBKETDQSZGBPVRTET
title: 'Wave 06 Cycle 02: Onboarding And Setting Changes On A Setup Branch'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 02: Onboarding And Setting Changes On A Setup Branch

Ticket: vh-c97se (`01M84H84WQYE8J2WJWEDGJ45W4`), branch `manyhands/ticket/01M84H84WQYE8J2WJWEDGJ45W4`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 02 design](../plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-design.md). Plan: [Cycle 02 implementation plan](../plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-implementation.md).

Status: **draft**, 2026-11-25, revision 1. Author: Claude. These three documents are Manyhands managed documents on their own branches (decision 9).

Revision history:

- Revision 1: first draft.

## Purpose And Authority

This is the second Cycle of Wave 06. It lets `repo enable` put a repository's Manyhands setup on a branch for a pull request instead of committing it to the primary, and lets `remote select` do the same for a change of mode, forge or remote selection in an enabled repository. While the setup branch is unmerged the repository is setup pending: writes are refused, `repo list` and `repo inspect` show it waiting with its link, and a poll that sees the setup on the remote's primary brings the repository to the state a direct enable would. A pending setup can be cancelled. This is how the owner's work repository, already enabled in direct mode, reaches pull-request mode.

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24: section "Onboarding and setting changes"; decisions 8, 9, 15, 19 and 21.
2. This Cycle's Product-Owner Decisions, below.
3. AGENTS.md, including its Testing Strategy and local test gate.
4. [Wave 06 Cycle 01](wave-06-cycle-01-pull-request-setting.md) and its design: the configuration keys, `forge::default_integration`, the link builder and `primary_has_local_commits`.
5. The [CLI contract](../RFC/cli-contract.md), the [Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md) and the [index RFC](../RFC/repository-index-persistence-and-refresh.md), each amended by this Cycle (design D-1 to D-3).

## Entry Evidence And Dependency Boundary

| Gate | State |
| --- | --- |
| RFC approved and on main | `0db9124d`, 2026-11-24 |
| Cycle 01 (vh-c2dtj) closed and merged | `14fe8524`, 2026-11-25 |
| Ticket branch holds current main | `origin/main` `3df52dcb` merged at `da032ffb` (merged, not rebased) |
| Dependencies | vh-c2dtj, closed |
| Parallel Cycles | Cycle 03 (vh-gy8gz) and Cycle 04 (vh-rphcs) depend only on Cycle 01. Cycle 03 reuses this Cycle's `pull_request` object (decision 6); whichever lands second merges with the other |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work (owner, 2026-11-24) |

## Scope

1. A setup record per repository in the registry, and the setup-pending state it defines.
2. `repo enable` in pull-request mode: the input gains `publication_remote`, `integration` and `forge`; the default mode follows RFC decision 9; the setup is committed on `manyhands/setup/<ULID>` from the remote's primary, published, and answered with a pull request link.
3. Setting changes: `remote select` gains `integration` and `forge`, and a change that is or becomes pull-request mode goes through a setup branch.
4. The setup gate: while a setup is pending, the commands of design Part B answer the new code `setup_pending`.
5. `repo list` and `repo inspect` show the pending setup and its link; the JSON `pull_request` object.
6. `poll once` activates a first-time setup once the remote's primary holds it, and completes a change once the primary's configuration moves.
7. `repo setup-cancel`: abandon a first-time setup, or cancel a change, deleting the remote setup branch only while it is at its published tip.
8. Host trust for a first-time setup: `host observe` and `host approve` use its pending remote.
9. The desktop's minimum (decision 5): a setup-pending repository is polled, and its row says it is waiting.
10. The amendments to the CLI contract and RFCs, and the shipped skills.

## Explicit Exclusions And Downstream Obligations

- **Close and promote in pull-request mode** are Cycles 03 and 04. While a setup is pending they refuse; otherwise they keep Cycle 01's behaviour.
- **The desktop's Add repository** mode choice, forge fields and remote, the "Open pull request page" and "Copy link" actions, and the waiting row's action are Cycle 06's (vh-p7st2) (decision 5).
- **The per-item record of given links** is Cycle 03's. The setup's link lives in its record.
- **Moving a pull-request repository to another remote** is refused (decision 8, K-1).
- **Converting a repository enabled the old way whose setup never reached the remote** stays a non-goal (RFC): its local primary has commits the remote lacks, and the setting change refuses with `primary_has_local_commits`.
- `item sync`, close's and promote's direct paths, and every direct-mode answer not named here are unchanged.

**Notes owed to other tickets at approval** (the planning skill's step 9):

- vh-gy8gz (Cycle 03): the `pull_request` schema and `PullRequestDto` come from this Cycle; close and promote refuse with `setup_pending` while a setup is pending.
- vh-rphcs (Cycle 04): the same.
- vh-p7st2 (Cycle 06): Add repository and the waiting row's action stay its work; this Cycle adds the row's state and the scheduler's setup poll.
- vh-ssepw (Cycle 05): the checkout warning is its own; this Cycle's setup poll answers `waiting_for_checkout`.
- The Wave 06 document: Cycle 02's line "and Add repository" moves to Cycle 06 (design D-6).

## Required Contract

1. **(owner's eye)** `repo enable` takes optional `publication_remote` (string), `integration` (`"direct"` or `"pull_request"`) and `forge` (`kind`, `web_url`, `pull_request_url`, each optional). `repo create`'s input is unchanged.
2. **(owner's eye)** For a repository without configuration, the mode is `integration` when given; otherwise pull-request mode when `publication_remote` is given and its forge is recognised or `forge` is given (RFC decision 9); otherwise direct. A pull-request configuration without a remote, or an invalid forge, is `invalid_input`; a missing or non-SSH remote is `invalid_remote`; both before any write. A direct enable writes a given remote and forge in its "Initialize Manyhands" commit.
3. For a repository with a valid configuration, enable registers only, as today; a supplied `publication_remote`, `integration` or `forge` that differs from the configuration is `invalid_input`.
4. **(owner's eye)** A pull-request enable checks the branch, the worktree and the identity as today (an unborn HEAD is `wrong_branch`), then registers the repository as setup pending before any network call, then fetches the remote's primary and:
   - answers `remote_primary_missing` (blocked, exit 3, no recovery action) when the remote has no primary; `primary_has_local_commits` (Cycle 01's code and data, decision 7) when the local primary has commits the remote's primary lacks; `external_change` when the remote's primary already holds a configuration. Each ends the setup: no registration, record or branch is left;
   - otherwise commits the configuration and the remote selection in one commit, "Set up Manyhands", on `manyhands/setup/<ULID>` made from the remote's primary, publishes that branch, and answers `ok` with the setup and its `pull_request` object. The local primary and the root checkout do not change, and the remote's primary is not pushed.
5. **(owner's eye)** A key, unlock, host or network stop after registration keeps the setup recorded, answers that stop, and leaves the request accepted: the same request continues it. `host observe` and `host approve` work on the pending repository, with its pending remote (decision 2).
6. **(owner's eye)** While a first-time setup is pending, the commands of design Part B answer `setup_pending` (blocked, exit 3, recovery `repo.inspect`) before writing anything, on a first call and on a prepare. While a change is pending, `ticket close`, `document promote`, `remote select` and `repo enable` answer `setup_pending`, and other writes carry on. Reads answer as for a registered repository without configuration.
7. `repo list` and `repo inspect` report `setup`: null, or `{ id, kind, state, branch, remote, integration, pull_request }`, where `pull_request` is `{ source, target, forge, url }` once published and null before. Human `repo inspect` names the setup branch and prints the link.
8. **(owner's eye)** `remote select` takes optional `integration` and `forge` (an object, or null to remove it). A direct repository that stays direct keeps today's commit on the primary. Any other change goes through a setup branch, as in 4, with the commit "Change Manyhands setup": it must keep the current publication remote (`invalid_input` otherwise, decision 8); it is refused while a close or promote is unfinished (`recovery_required`); the remote's primary must hold the local primary's configuration (`external_change`) and every local primary commit (`primary_has_local_commits`). It stays unconfirmed.
9. **(owner's eye)** `poll once` on a repository with a published first-time setup fetches the remote's primary and, when it holds a valid configuration with the same primary branch and the root checkout is on the primary, clean and not ahead, fast-forwards the local primary, registers the repository as a direct enable would, deletes the local setup branch and the record, and answers `activated`, saying whether the merged configuration differs from the intended one (the merged one wins). Otherwise it answers why it is still waiting, and changes nothing. Its data is a setup poll object; the poll data of an active repository does not change.
10. A pending change completes when the local primary's configuration differs from the one the change began from, seen by `poll once` or `repo sync`: the local setup branch and the record are deleted.
11. **(owner's eye)** `repo setup-cancel` is a new confirmed verb with no input. It deletes the remote setup branch only while it is at its published tip (kept, and said so, if someone pushed to it), deletes the local setup branch and the record, and for a first-time setup unregisters the repository. It refuses with `recovery_required` when the setup has already reached the remote's primary. With no pending setup it is `already_applied`. `repo remove` keeps its contract: registry only, so it drops the record and leaves both setup branches.
12. **(owner's eye)** No pull-request path commits to the local primary or pushes the primary. The only refs a setup pushes or deletes are its own `manyhands/setup/<ULID>`.
13. Direct mode is unchanged: every existing answer, golden and test keeps its result, except the `setup` field and the named edits.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1-3 | A table-driven library test of the intended configuration and mode, the input refusals and the clone rule; the input schema tests | planned |
| A2 | 4, 12 | Design Part D's required outcomes S1 to S10, each with its test; the commit builder's test that only the configuration changes | planned |
| A3 | 5 | A non-SSH test: a pull-request enable that stops at `no_selected_key` leaves a recorded setup that `repo inspect` and `repo list` show, and the same request continues; a `bind/host.rs` test for the pending remote | planned |
| A4 | 6 | The exhaustive gate test over every command; a test that a first call and a prepare are gated and a re-entry is not; the `setup_pending` golden | planned |
| A5 | 7 | Read tests and the four repository goldens with `setup`; a human `repo inspect` test | planned |
| A6 | 8 | A table-driven library test of `remote select`'s paths and refusals; today's remote selection tests unchanged | planned |
| A7 | 9 | Design Part G's G1 to G7 in a library test with local repositories and a stubbed channel, including a crash after the fast-forward | planned |
| A8 | 10 | A library test: complete after a poll's and a sync's fast-forward, not otherwise | planned |
| A9 | 11 | A library test of cancel's outcomes; the non-SSH cancel of a recorded setup; the goldens | planned |
| A10 | 4, 9, 12 | SSH case 1 (design P14): `remote_primary_missing` on an empty server, a published setup with the server's primary unchanged, a gated write, a squash on the server, activation by `poll once` | planned |
| A11 | 8, 11, 12 | SSH case 2 (design P15): a change through a setup branch, `setup_pending` for close, cancel keeping a moved branch, cancel deleting an unmoved one | planned |
| A12 | 13 | The fast tier and every SSH-harness target, once at review-ready | planned |
| A13 | Scope 9 | Desktop model and scheduler tests (design P13) | planned |
| A14 | Scope 10 | The amended RFC sections and skills; `tests/cli/recovery_tables.rs` and `tests/cli/skills.rs` pass | planned |

## Product-Owner Decisions (2026-11-25)

1. **One Cycle** for first-time enable and setting changes: they share the setup record, the channel and the poll.
2. **Register first, then resume.** A pull-request enable registers the repository as setup pending before any network call, then fetches, commits and publishes. A stop at unlock, host trust or the network leaves the setup recorded, `host observe` and `host approve` work on it, and the same request finishes it.
3. **`remote select` gains `integration` and `forge`.** A direct repository that stays direct keeps today's commit on the primary; any other change goes through a setup branch. It stays unconfirmed.
4. **A new confirmed `repo setup-cancel`** for both kinds of setup. It deletes the remote setup branch only at its published tip, and unregisters a first-time setup. `repo remove` keeps its registry-only contract and leaves the branches.
5. **The desktop's minimum:** the scheduler polls a setup-pending repository and its row does not say "Needs fixing". Add repository's mode choice, the forge fields, the waiting row's action and the link actions stay Cycle 06's.
6. **This Cycle defines the JSON `pull_request` object** (`source`, `target`, `forge`, `url`) and its schema; Cycle 03 reuses them and keeps the per-item record of given links.
7. **`primary_has_local_commits`** answers a local primary with commits the remote lacks at enable, in place of the RFC's `primary_ahead_of_remote`; no new code.
8. **Moving a pull-request repository to another remote is refused** (`invalid_input`), with a route by hand in the skill.
9. **The three planning documents are Manyhands managed documents**, each on its own branch and published with `item sync`, so the owner can review them in the desktop app. This departs from the planning skill's rule that Cycle documents live on the ticket branch. The execution ledger stays on the ticket branch, and each document is promoted on its own after approval.

## Known Limits

- K-1: A setting change keeps the current publication remote. A pull-request repository cannot move to another remote through Manyhands, and a direct repository with no shared remote cannot switch to pull-request mode through it; the skill gives the route by hand (decision 8, design R-12).
- K-2: A corrupt registry replaced by an empty one loses a pending setup's record; the local setup branch stays. The skill says to delete the branch, and the remote one, by hand and enable again.
- K-3: `poll once` does not report that a change completed; `repo inspect` shows `setup` null (design R-16).
- K-4: Until Cycle 06, the desktop shows a waiting repository without a link action, and Add repository enables in direct mode only.
- K-5: Until Cycles 03 and 04, pull-request mode is not usable end to end: close and promote still push the primary when no setup is pending.
- K-6: An unborn HEAD cannot be enabled in pull-request mode (`wrong_branch`, design R-11).
- K-7: A first-time setup refused at its fetch is ended, so a retry of the same request fetches and decides again (design R-7).
- K-8: Every teammate needs a build with Cycle 01 before a pull-request setup merges, because the configuration is version 2 (RFC decision 21).

## Open Questions

1. **Proposed decision 10: mechanical test edits found by Task 0's sweep are pre-authorized**, as Cycle 01's decisions 9 and 12 were: an edit that keeps every assertion's intent is applied and logged as E-14 onwards with its reason; an edit that removes or weakens an assertion stops for the owner. The plan is written to this answer.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | Scope: first-time enable and setting changes | decision | One Cycle (decision 1) | Owner, 2026-11-25 |
| Planning | Host trust needs a registered repository whose file names the remote | decision | Register first, resume (decision 2) | `bind/host.rs` `Host::read`; design Baseline Fact 8 |
| Planning | No verb writes `integration` or `[forge]` | decision | `remote select` gains them (decision 3) | Cycle 01's design Part A, "Unchanged writers" |
| Planning | Abandon and cancel | decision | `repo setup-cancel` (decision 4) | — |
| Planning | Add repository listed in Cycles 02 and 06 | decision | The desktop's minimum (decision 5) | Wave 06 document |
| Planning | The `pull_request` object was given to Cycle 03, but enable gives the first link | decision | Defined here (decision 6) | Cycle 01, decision 2 |
| Planning | The RFC's `primary_ahead_of_remote` is a blocker name, not a code | decision | `primary_has_local_commits` (decision 7) | `promotion/observe.rs` `remote_facts` |
| Planning | A remote change in pull-request mode needs two remotes in play | decision | Refused (decision 8) | — |
| Planning | Planning documents on the ticket branch are not visible in the desktop | decision | Managed documents (decision 9) | Owner, 2026-11-25 |
| Planning | The transport and about thirty remote sites read the configuration file | ruling | A setup channel (design R-3) | design Baseline Facts 2 and 5 |
| Planning | A new remote operation action needs a table rebuild with nine children | ruling | No remote reservation; a setup lock (design R-4, R-5) | design Baseline Fact 4 |
| Planning | Poll data is closed and replayed with exactly five keys | ruling | A setup poll data shape of its own (design R-16) | design Baseline Fact 6 |
