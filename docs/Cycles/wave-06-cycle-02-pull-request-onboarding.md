---
id: 01M89CHP8JBKETDQSZGBPVRTET
title: 'Wave 06 Cycle 02: Onboarding And Setting Changes On A Setup Branch'
manyhands_managed: true
manyhands_kind: document
---
# Wave 06 Cycle 02: Onboarding And Setting Changes On A Setup Branch

Ticket: vh-c97se (`01M84H84WQYE8J2WJWEDGJ45W4`), branch `manyhands/ticket/01M84H84WQYE8J2WJWEDGJ45W4`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 02 design](../plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-design.md). Plan: [Cycle 02 implementation plan](../plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-implementation.md).

Status: **approved**, 2026-11-25, revision 4 (owner approval 2026-11-25, decisions 17 and 18). Implemented, reviewed and closed 2026-11-26: the owner approved the review, and `ticket close` merged vh-c97se into main at `c56b6ef1`. Author: Claude. These three documents are Manyhands managed documents (decision 9), promoted to main at closure.

Revision history:

- Revision 1 (Cycle document `8dce7517`, design `97443833`, plan `c014237b`): first draft.
- Revision 2 (Cycle document `e9ad0848`, design `98a0c019`, plan `efb04672`): two independent reviews of revision 1 (source and feasibility: 2 Blocking, 10 Important, 14 Minor; contract and consistency: 3 Blocking, 13 Important, 8 Minor) and the owner's decisions 10 to 14.
- Revision 3 (Cycle document `71594c02`, design `4a1d899d`, plan `5882ae02`): a scoped re-review of revision 2 (1 Blocking, 7 Important, 8 Minor) and an adversarial round (3 Blocking, 8 Important, 12 Minor), and decision 15. The request lifecycle of a setup is restated as required outcomes (design Part K), built test-first with design checkpoints. 
- Revision 4: a third round, a scoped re-review of revision 3 (0 Blocking, 5 Important, 11 Minor) and an adversarial round (2 Blocking, 8 Important, 7 Minor), all in the request lifecycle; decision 16. As the planning skill says after a third round in one mechanism, the lifecycle is not redesigned again: its rules K1 to K7 and outcomes L1 to L14 are the contract, built test-first with design checkpoints. Revision 4 has not been independently reviewed.
- Approval, 2026-11-25: the owner approved revision 4 as written, with no further review round (decisions 17 and 18). Revision 4 stays without an independent review; its lifecycle is built test-first with the design checkpoints of Tasks 6 to 10.
- Implementation, 2026-11-25: Tasks 0 to 13 subagent-driven, each reviewed, then a whole-branch review, two fix waves and a short third round; review-ready with the code at `0e2a5e77`. The design is amended where the build departed from it (published OID check, the digest's refactor, L10 and L11, Baseline Fact 9, activation's attempt clearing, the scheduler's read, what stops the finish and G5). Rulings, named test edits E-16 to E-32 and every test run are in the execution ledger, `docs/plans/2026-11-25-wave-06-cycle-02-pull-request-onboarding-execution.md`.
- Closure, 2026-11-26: the owner approved the review; `ticket close` merged the ticket's branch into main at `c56b6ef1` and published it, and the Cycle document, the design and the plan were promoted with the Wave 06 document. Follow-on: the recovery skill's kept-branch wording (ruling R-AF), K-24, and the final review's deferred Minors.

## Purpose And Authority

This is the second Cycle of Wave 06. It lets `repo enable` put a repository's Manyhands setup on a branch for a pull request instead of committing it to the primary, and lets `remote select` do the same for a change of mode, forge or remote in an enabled repository. While the setup branch is unmerged the setup is pending: a first-time setup refuses writes, a change refuses close and promote, `repo list` and `repo inspect` show it waiting with its link, and once the setup is on the primary the repository reaches the state a direct enable would. A pending setup can be cancelled. This is how the owner's work repository, already enabled in direct mode, reaches pull-request mode.

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24: section "Onboarding and setting changes"; decisions 8, 9, 15, 17, 19, 21 and 22.
2. This Cycle's Product-Owner Decisions, below, and the departures they make from the RFC (listed below).
3. AGENTS.md, including its Testing Strategy and local test gate, as decision 11 narrows the SSH tier for this Cycle.
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
2. `repo enable` in pull-request mode: the input gains `publication_remote`, `integration` and `forge`; the default mode follows RFC decision 9; the setup is committed on `manyhands/setup/<ULID>` from the remote's primary, published, and answered with a pull request link. A direct enable may name its remote and forge too.
3. Setting changes: `remote select` gains `integration` and `forge`, and a change that is or becomes pull-request mode goes through a setup branch.
4. The setup gate: while a setup is pending, the commands of design Part B answer the new code `setup_pending`.
5. `repo list` and `repo inspect` show the pending setup and its link; the JSON `pull_request` object.
6. Activation and completion: `poll once` activates a setup on another remote once its primary holds it, and any setup completes once the local primary holds it.
7. The request lifecycle of a setup: the same request continues it, and a request whose setup ended elsewhere never starts a second one (design Part K).
8. `repo setup-cancel`: abandon a first-time setup, or cancel a change, deleting the remote setup branch only while it is at its published tip.
9. Host trust for a setup on another remote: `host observe` and `host approve` use its remote.
10. The desktop's minimum (decision 5): a setup-pending repository is polled, and its row says it is waiting.
11. The amendments to the CLI contract and RFCs, the Wave document and the shipped skills.

## Explicit Exclusions And Downstream Obligations

- **Close and promote in pull-request mode** are Cycles 03 and 04. While a setup is pending they refuse; otherwise they keep Cycle 01's behaviour.
- **The desktop's Add repository** mode choice, forge fields and remote, the "Open pull request page" and "Copy link" actions, and the waiting row's action are Cycle 06's (vh-p7st2) (decision 5).
- **The per-item record of given links** is Cycle 03's. The setup's link is built from its record.
- **Deleting a remote setup branch that survives the merge** is Cycle 05's guarded deletion (RFC decision 22; K-8).
- **Moving a pull-request repository to another remote** is refused (decision 8, K-1).
- **Converting a repository enabled the old way whose setup never reached the remote** stays a non-goal (RFC): its local primary has commits the remote lacks, and a change refuses with `primary_has_local_commits`.
- `item sync`, close's and promote's direct paths, and every direct-mode answer not named here are unchanged.

**Notes owed to other tickets at approval** (the planning skill's step 9):

- vh-gy8gz (Cycle 03): the `pull_request` schema and `PullRequestDto` come from this Cycle; close and promote refuse with `setup_pending` while a setup is pending; the Wave line for its link record changes (design D-6).
- vh-rphcs (Cycle 04): the `pull_request` object and the `setup_pending` refusal.
- vh-p7st2 (Cycle 06): Add repository and the waiting row's action stay its work; this Cycle adds the row's state and the scheduler's setup poll.
- vh-ssepw (Cycle 05): a surviving remote setup branch is its guarded deletion's to remove; the checkout warning is its own, and this Cycle's setup poll answers `waiting_for_checkout`.
- The Wave 06 document: design D-6.

## Departures From The RFC

Each is the owner's decision or recorded as a ruling, and amended into the RFCs through design D-1 to D-3:

1. Enabling registers the repository first, before any network call, and then fetches, commits and publishes (decision 2); the RFC lists registration last.
2. A local primary ahead of the remote answers `primary_has_local_commits` (decision 7), and a remote already set up answers the new `remote_already_set_up` (decision 13); the RFC names `primary_ahead_of_remote`.
3. While a first-time setup is pending, reads answer as for a registered repository without configuration: Markdown files on the primary stay listed, unmanaged, as today. The RFC says "as they would for a repository with no items"; no managed item exists, so the two agree on items.
4. Tests: the RFC's "CLI contract tests against a local remote. No new SSH case, because the transport does not change", and the Wave's and ticket's "local or `file://` remote" cannot be met, because no non-SSH publication remote can be selected (Cycle 01). This Cycle uses library tests, non-SSH integration tests against a never-contacted SSH URL, and one SSH case for the new transport path (decision 11).
5. A same-remote change's completion is not reported by `poll once` (K-3); the RFC's "the poll says so" is met for a setup on another remote.

## Required Contract

1. **(owner's eye)** `repo enable` takes optional `publication_remote` (string), `integration` (`"direct"` or `"pull_request"`) and `forge` (`kind`, `web_url`, `pull_request_url`, each optional). `repo create`'s input is unchanged.
2. **(owner's eye)** For a repository without configuration, the mode is `integration` when given; otherwise pull-request mode when `publication_remote` is given and its forge is recognised or `forge` is given (RFC decision 9); otherwise direct. A pull-request configuration without a remote, or an invalid forge, is `invalid_input`; a missing or non-SSH remote is `invalid_remote`; both before any write. A direct enable writes a given remote and forge in its "Initialize Manyhands" commit.
3. For a repository with a valid configuration, enable registers only, as today; a supplied `publication_remote`, `integration` or `forge` that differs from the configuration is `invalid_input`.
4. **(owner's eye)** A pull-request enable checks the branch, the worktree and the identity as today (an unborn HEAD is `wrong_branch`), then registers the repository as setup pending before any network call, then fetches the remote's primary and:
   - answers `remote_primary_missing` (blocked, exit 3, no recovery action) when the remote has no primary; `primary_has_local_commits` (Cycle 01's code and data, decision 7) when the local primary has commits the remote's primary lacks; `remote_already_set_up` (blocked, exit 3, no recovery action, decision 13) when the remote's primary already holds a configuration. Each ends the setup and finishes the request: no registration, record, setup branch or private ref is left (the request's own completed operation row, a supplied identity and the worktree exclusion stay);
   - otherwise commits the configuration and the remote selection in one commit, "Set up Manyhands", on `manyhands/setup/<ULID>` made from the remote's primary, publishes that branch, and answers `ok` with the setup and its `pull_request` object. The local primary and the root checkout do not change, and the remote's primary is not pushed.
5. **(owner's eye)** A key, unlock, host or network stop after registration keeps the setup recorded, answers that stop, and leaves the request accepted: the same request continues it. `host observe` and `host approve` work on the pending repository, with its pending remote (decision 2).
6. **(owner's eye)** While a first-time setup is pending, the commands of design Part B answer `setup_pending` (blocked, exit 3, recovery `repo.inspect`) before writing anything, on a first call, at acceptance and on a prepare. While any setup is pending, `ticket close`, `document promote`, `remote select`, `remote remove` and `repo enable` answer `setup_pending`; while a change is pending, other writes carry on in the current mode and on the current remote. Reads answer as for a registered repository without configuration.
7. `repo list` and `repo inspect` report `setup`: null, or `{ id, kind, state, request_id, branch, remote, integration, pull_request }`, where `pull_request` is `{ source, target, forge, url }`. Human `repo inspect` names the setup and its state and prints the link; human `repo list` marks the repository.
8. **(owner's eye)** `remote select` takes optional `integration` and `forge` (an object, or null to remove it). A direct repository that stays direct keeps today's commit on the primary. From direct mode, a change to pull-request mode may name any SSH remote (decision 12). A pull-request repository's change keeps its remote (`invalid_input` otherwise, decision 8). A setup-branch change, with the commit "Change Manyhands setup", is refused while a close or promote is unfinished or accepted (`recovery_required`), and when the remote's primary lacks a local primary commit (`primary_has_local_commits`) or no longer holds the configuration the change starts from (`external_change`). A change equal to the current configuration is `already_applied`. It stays unconfirmed.
9. **(owner's eye)** `poll once` on a repository whose setup is on another remote (every first-time setup, and a change to a first or new remote; for such a change, after today's poll of the current remote) fetches that remote's primary and, when it holds the setup and the root checkout is on the primary, clean and not ahead, fast-forwards the local primary and completes the setup: the repository is registered as a direct enable plus remote selection would leave it, the local setup branch and the record are deleted, and the answer is `activated`, saying whether the merged configuration differs from the intended one (the merged one wins). Otherwise it answers why it is still waiting and changes nothing. A merge commit, a squash and a fast-forward each activate. Its data is a setup poll object; the poll data of an active repository does not change.
10. Any pending setup completes when the local primary (`refs/heads/<primary>`, not HEAD) already holds it: a valid configuration naming that primary, and for a change one other than the configuration it began from. The gate, the end of any `poll once` and the setup poll finish it, only while HEAD names the primary: the local setup branch and the record are deleted, and a setup on another remote is registered on its remote.
11. **(owner's eye)** `repo setup-cancel` is a new confirmed verb with no input. It deletes the remote setup branch only while it is at the setup commit (kept, and said so, if someone pushed to it), deletes the local setup branch and the record, unregisters a first-time setup, and finishes the setup's own request as `cancelled`. A remote setup branch at another OID is kept, with no delete sent; a refused or lost delete answers `push_rejected`, retryable. It refuses with `recovery_required` when the setup has already reached the remote's primary. With no pending setup it is `already_applied`. `repo remove` keeps its registry-only contract and always works (decision 16): it drops the record, finishes the setup's request as `cancelled` if it is still accepted, and leaves both setup branches, with the root present or gone.
12. **(owner's eye)** A setup request is continued by sending the same request again while its record exists; every answer after recording that does not end the setup leaves it accepted. Every removal of the record finishes the request in the same transaction (a refusal with that refusal; a cancel or `repo remove` with `cancelled`; completion with `already_applied`, or `external_change` when the merged configuration is not the intended one), so it replays that answer and never starts a second setup. A published setup's answer replays without a reachability check. `request cancel` does not reach a setup request; `repo setup-cancel` stops a setup.
13. **(owner's eye)** No pull-request path commits to the local primary or pushes the primary. The only refs a setup pushes or deletes are its own `manyhands/setup/<ULID>`.
14. Direct mode is unchanged: every existing answer, golden and test keeps its result, except the `setup` field and the named edits.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1-3 | A table-driven library test of the intended configuration and mode, the input refusals, the clone rule and the direct path with a remote; the input schema tests, with `repo create` refusing the new fields | met |
| A2 | 4, 13 | Design Part D's required outcomes S1 to S12, each with its test; the commit builder's test that only the configuration changes | met |
| A3 | 5 | A non-SSH test: a pull-request enable that stops at `no_selected_key` leaves a recorded setup that `repo inspect` and `repo list` show, `host observe` answers `no_selected_key`, and the same request continues; the SSH case's host stop and approval | met (`no_selected_key` is pinned in `tests/cli/repository.rs`: the `repository_enablement` harness cannot start Git's transport, design Baseline Fact 9's implementation note) |
| A4 | 6 | The exhaustive gate test over every command; first call, acceptance, prepare and re-entry; the race with a change; the `setup_pending` golden | met |
| A5 | 7 | Read tests and the four repository goldens with `setup`; human `repo inspect` and `repo list` tests | met |
| A6 | 8 | A table-driven library test of `remote select`'s paths and refusals, each pull-request row leaving the local primary unchanged; today's remote selection tests unchanged | met |
| A7 | 9 | Design Part G's G1 to G7 in a library test with local repositories and a stubbed channel, for a merge commit, a squash and a fast-forward, including a crash after the fast-forward, and a comparison with a direct enable plus remote selection | met (G5 also waits while the setup branch is checked out in a worktree) |
| A8 | 10 | A library test of completion for a merge commit, a squash and a fast-forward, by the gate and by a poll, and not before | met |
| A9 | 11 | A library test of cancel's outcomes and its crash; the non-SSH cancel of a recorded setup; the goldens | met |
| A10 | 4, 5, 8, 9, 11, 13 | The SSH case (design P15), decision 11: host stop and approval on the pending repository, `remote_primary_missing`, a published setup with the server's primary unchanged, a gated write, a squash on the server, activation, a change, `setup_pending` for close, cancel keeping a moved branch and deleting an unmoved one. Cycle 01's decision 6 (no push shown, in place of a refusing remote) meets the RFC's push evidence | met: `a_setup_branch_is_published_activated_changed_and_cancelled`, 28 s |
| A11 | 14 | The fast tier, the SSH case, and the `ssh_transport` and `remote_polling` targets, once at review-ready (decision 11) | met: fast tier 4914 s on `9ce90e25` with three scheduler load flakes that pass singly, the targets reading the last round's text rerun on `0e2a5e77` (296 s); SSH case 28 s, `ssh_transport` 141 s, `remote_polling` 240 s, and `document_promotion`'s four delete cases singly (R-G), 6 to 9 s each |
| A12 | Scope 10 | Desktop model and scheduler tests (design P14) | met |
| A13 | 12 | Design Part K's L1 to L14, each with its test (design P12) | met (L10 and L11 as amended in the design, ruling R-X) |
| A14 | Scope 11 | The amended RFC sections and skills; `tests/cli/recovery_tables.rs`, `tests/cli/skills.rs`, `tests/cli/inventory.rs` and `tests/cli/collaboration_exits.rs` pass | met |

## Product-Owner Decisions (2026-11-25)

1. **One Cycle** for first-time enable and setting changes: they share the setup record, the channel and the poll.
2. **Register first, then resume.** A pull-request enable registers the repository as setup pending before any network call, then fetches, commits and publishes. A stop at unlock, host trust or the network leaves the setup recorded, `host observe` and `host approve` work on it, and the same request finishes it.
3. **`remote select` gains `integration` and `forge`.** A direct repository that stays direct keeps today's commit on the primary; any other change goes through a setup branch. It stays unconfirmed.
4. **A new confirmed `repo setup-cancel`** for both kinds of setup. It deletes the remote setup branch only at its published tip, and unregisters a first-time setup. `repo remove` keeps its registry-only contract and leaves the branches.
5. **The desktop's minimum:** the scheduler polls a setup-pending repository and its row does not say "Needs fixing". Add repository's mode choice, the forge fields, the waiting row's action and the link actions stay Cycle 06's.
6. **This Cycle defines the JSON `pull_request` object** (`source`, `target`, `forge`, `url`) and its schema; Cycle 03 reuses them and keeps the per-item record of given links.
7. **`primary_has_local_commits`** answers a local primary with commits the remote lacks at enable, in place of the RFC's `primary_ahead_of_remote`; no new code.
8. **Moving a pull-request repository to another remote is refused** (`invalid_input`), with a route by hand in the skill.
9. **The three planning documents are Manyhands managed documents**, each on its own branch and published with `item sync`, so the owner can review them in the desktop app. This departs from the planning skill's rule that Cycle documents live on the ticket branch. The execution ledger stays on the ticket branch, and each document is promoted on its own after the ticket closes.
10. **Extra tests the plan does not list are approved as needed.**
11. **SSH coverage.** The setup is another path through the existing transport, so one SSH case covers it, and at review-ready the SSH tier is that case, `ssh_transport` (the shared connection code) and `remote_polling` (the poll), not every SSH-harness target. This departs from AGENTS.md's table for this Cycle; native CI runs every target after the merge. The owner asked to be told if something fundamental changes. Three shared paths change: the shared connection preparation gains a second source for the remote's name, with today's source unchanged (design Part C); the mutation boundary runs the setup gate on every command's first call, prepare and acceptance, which with no record is one registry read (design Part B; the fast tier runs every command through it); and the end of every `poll once` checks for a setup to complete, a no-op with no record (`remote_polling` runs at review-ready). AGENTS.md maps the last two to `mutation_remote` and six remote targets; this Cycle keeps decision 11's tier.
12. **From direct mode, a change to pull-request mode may name any SSH remote**: a first remote, or a different one. The setup uses that remote, host trust and the setup poll read it from the record, and activation switches the repository to it, as today's direct remote change does. Decision 8's refusal stays for repositories already in pull-request mode.
13. **A new code, `remote_already_set_up`**, answers a pull-request enable whose remote's primary already holds a configuration: blocked, no recovery action, and the skill says to update the checkout and enable again, which registers it as a clone.
14. **Autonomy:** `repo setup-cancel` is show-then-wait; `remote select` with `integration` or `forge` is show-then-wait, because it publishes a branch and blocks close and promote; a plain `remote select` stays act-alone.
15. **Edits to existing tests are approved**: the named edits and those Task 0's sweep finds, each logged with its reason; an edit that removes or weakens an assertion still stops for the owner.
16. **`repo remove` always works on a setup-pending repository.** It keeps its registry-only contract (decision 4): it finishes the setup's unfinished request as `cancelled`, so the request can never start a second setup, drops the registration and the record, and leaves both setup branches for the person to delete by hand, even when the root folder is gone. For a pending change the person then runs `repo enable` again, which registers the repository as a clone. This is the way out of a setup whose remote can no longer be reached.
17. **Approval (2026-11-25).** The owner approved the Cycle document, the design and the plan at revision 4 ("The three docs are approved"), and confirmed the tests ("Tests are confirmed"): decision 11's tier stands with the fuller disclosure, which settles the open question, and the named edits E-1 to E-15 are covered by decision 15. No fourth review round.
18. **Execution (2026-11-25).** Implementation is authorized. It is subagent-driven: one implementer per task, on Sonnet 5.5 at high effort, then a fresh reviewer, with the orchestrator checking the design checkpoints against the source. Implementation approval does not cover merging, closing the ticket, promoting the documents or dispatching native CI; each needs its own go-ahead.

## Known Limits

- K-1: A pull-request repository cannot move to another remote through Manyhands (decision 8); the skill gives the route by hand.
- K-2: A corrupt registry replaced by an empty one loses a pending setup's record; the local setup branch stays. The skill says to delete the branch, and the remote one, by hand and enable again.
- K-3: `poll once` does not report that a same-remote change completed; `repo inspect` shows `setup` null (design R-16).
- K-4: Until Cycle 06, the desktop shows a waiting repository without a link action, and Add repository enables in direct mode only.
- K-5: A setup refused at its fetch is ended and its request finished: a retry of the same request replays the refusal, and a new request tries afresh (design R-7). After a crash between the ending and the request's settle, the retry answers `cancelled` instead (design L5).
- K-6: One setup per clone, not per repository: two clones can each open a setup pull request. Whichever merges first completes both clones' setups; the other pull request and its remote branch are left, and the skill says to decline it.
- K-7: Any move of the primary's configuration completes a pending change (design R-17): another clone's merged change, or an edit made outside Manyhands.
- K-8: A remote setup branch that survives the merge is left in place; deleting it is Cycle 05's guarded step (RFC decision 22). The skill gives the route by hand meanwhile.
- K-9: Until Cycle 06, the desktop shows a `setup_pending` refusal through its generic failure message.
- K-10: Until Cycles 03 and 04, pull-request mode is not usable end to end: close and promote still push the primary when no setup is pending.
- K-11: Every teammate needs a build with Cycle 01 before a pull-request setup merges, because the configuration is version 2 (RFC decision 21).
- K-12: An unborn HEAD cannot be enabled in pull-request mode (`wrong_branch`, design R-11).
- K-13: A change on another remote needs that remote's primary to hold this repository's current configuration (a mirror of the current remote, or the remote the configuration was pushed to by hand); otherwise it answers `external_change`. Commits that reach the local primary from the current remote while the change is pending must also reach the new remote, or the setup poll answers `diverged` until they do; cancelling, or `repo remove` and enabling again, is the way out.
- K-14: An older build sharing the data directory sees a setup-pending repository as registered without a configuration, and its `repo enable` would commit "Initialize Manyhands" on the primary. An older build's registry-only `repo remove` refuses the `setup_recorded` step as unrecognized evidence, and its re-entry of a setup request deletes the request. The skill says not to run an older build on such a repository; no release ships before Cycle 04.
- K-15: A local setup branch kept because it moved holds a configuration, so after a first-time setup is cancelled the unregistered root answers `repository_not_registered` instead of `repository_not_enabled` until the branch is deleted.
- K-16: `repo list`, which reads the index only, can show a complete setup as pending until the next refresh; `repo inspect` reads the local primary.
- K-17: An `index rebuild` running at the moment a first-time setup is ended or cancelled can register the root again with no configuration; `repo remove` clears it.
- K-18: `poll once --retire-completed` on a repository whose first-time setup is pending polls the setup and retires nothing: it begins no retirement pass, since the setup's poll reports no items.
- K-19: A setup record that vanishes (a cancel or `repo remove`) between a `--retire-completed` poll's admission and its run answers `recovery_required`, owed. The poll may leave its reserved remote row behind (not verified); `operation list` shows it, and its abandon clears it.
- K-20: A crash inside activation's fast-forward, with the tree checked out and the primary's ref not yet moved, leaves the setup poll answering `waiting_for_checkout` until the person cleans the tree: the same hazard `repo sync` has today.
- K-21: A crash in the window where a request's row is settled by deletion can leave a setup record naming a request that no longer exists; `repo setup-cancel` or `repo remove` clears it.
- K-22: A local setup branch checked out in a worktree holds completion: the setup poll answers `waiting_for_checkout` until the person switches away from it. When the worktree inventory cannot be read, completion goes on and keeps the local setup branch, which the person deletes by hand.
- K-23: A second crash right after an abandoned finish can leave one more pending row; `operation list` names it with its blocker, and its abandon clears it.
- K-24: The desktop reads a repository's setup with a whole `repo inspect` read on every scheduler read and every automatic poll. A lighter read is follow-on work.

## Open Questions

None. The one open question, decision 11's tier with the fuller disclosure, was settled by decision 17: the tier stands.

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
| Planning | The transport and about thirty remote sites read the configuration file | ruling, then decision | A setup channel (design R-3), confirmed as "another path through existing code" (decision 11) | design Baseline Facts 2 and 5 |
| Planning | A new remote operation action needs a table rebuild with nine children | ruling | No remote reservation; a setup lock (design R-4, R-5) | design Baseline Fact 4 |
| Planning | Poll data is closed and replayed with exactly five keys | ruling | A setup poll data shape of its own (design R-16) | design Baseline Fact 6 |
| Review round 1, source and feasibility | 2 Blocking, 10 Important, 14 Minor | — | Resolved in revision 2; see the design's Review Record | revision 2 |
| Review round 1, contract and consistency | 3 Blocking, 13 Important, 8 Minor | — | Resolved in revision 2, with decisions 10 to 14; see the design's Review Record | revision 2 |
| Review round 1 | Extra tests | decision | Approved as needed (decision 10) | Owner, 2026-11-25 |
| Review round 1 | Two SSH cases and the whole SSH tier against the RFC's and the Wave's test lines | decision | One SSH case; `ssh_transport` and `remote_polling` at review-ready (decision 11) | Owner, 2026-11-25 |
| Review round 1 | R-12 refused more than decision 8 | decision | Any SSH remote from direct mode (decision 12); R-12 retired | Owner, 2026-11-25 |
| Review round 1 | `external_change` for a remote already set up | decision | `remote_already_set_up` (decision 13) | Owner, 2026-11-25 |
| Review round 1 | Autonomy of the new and changed verbs | decision | Show then wait (decision 14) | Owner, 2026-11-25 |
| Review round 1 | Re-entry of the setup request compares a digest that registration changes | ruling | Re-entry through the record and `LocalStep::SetupRecorded` (design Part K) | `bind/repo.rs` `Confirmed::row`; the digest binds `registered` |
| Review round 1 | Completion missed by the desktop's direct poll and by a crash during activation | ruling | Completion is level-triggered (design R-17) | `desktop/services/mod.rs` calls `poll_publication_remote` |
| Review round 1 | Mechanical edits to existing tests | decision | Approved (decision 15) | Owner, 2026-11-25 |
| Review round 2, scoped re-review | 1 Blocking, 7 Important, 8 Minor | — | Resolved in revision 3; see the design's Review Record | revision 3 |
| Review round 2, adversarial | 3 Blocking, 8 Important, 12 Minor | — | Resolved in revision 3; see the design's Review Record | revision 3 |
| Review round 2 | Decision 11 named only the transport change | decision | The gate and the poll's end disclosed in decision 11; the tier is kept unless the owner says otherwise (open question 1) | scoped re-review finding 7 |
| Review round 2 | A setup request's re-entry, replay and ending | ruling | Restated as required outcomes, L1 to L12 in revision 3 and L1 to L14 in revision 4 (design Part K, R-19), built test-first | both round 2 reviews |
| Review round 2 | `repo remove` while a setup request is accepted | ruling, then decision | Revision 3 refused it (R-18); revision 4 has it finish the request and always work (decision 16) | `registration_removal.rs`: the accepted-request check runs only on the registry-only path |
| Review round 3, scoped re-review | 0 Blocking, 5 Important, 11 Minor | — | Resolved in revision 4, not independently reviewed; see the design's Review Record | revision 4 |
| Review round 3, adversarial | 2 Blocking, 8 Important, 7 Minor | — | Resolved in revision 4, not independently reviewed; see the design's Review Record | revision 4 |
| Review round 3 | No way out of a setup whose remote cannot be reached | decision | `repo remove` always works (decision 16) | Owner, 2026-11-25 |
| Review round 3 | The standing of a setup request's answers after recording | ruling | Every non-final answer `Owed` (design K2, R-20) | `settle.rs` deletes a request over a completed row |
| Approval | The three documents at revision 4, and decision 11's tier with the fuller disclosure | decision | Approved as written; the tier stands (decision 17) | Owner, 2026-11-25 |
| Approval | Execution method and authorization | decision | Subagent-driven, Sonnet 5.5 high implementers; implementation authorized (decision 18) | Owner, 2026-11-25 |
