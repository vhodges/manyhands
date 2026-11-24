---
title: "Wave 06 Cycle 01: Pull-Request Setting, Link Builder And Pull-Only Repo Sync"
date: 2026-11-24
status: "approved"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M878QWAET79VP1S0EXVCEMAC"
---

# Wave 06 Cycle 01: Pull-Request Setting, Link Builder And Pull-Only Repo Sync

Ticket: vh-c2dtj (`01M87683441PRAYQJQBQBG96YY`), branch `manyhands/ticket/01M87683441PRAYQJQBQBG96YY`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 01 design](../plans/2026-11-24-wave-06-cycle-01-pull-request-setting-design.md). Plan: [Cycle 01 implementation plan](../plans/2026-11-24-wave-06-cycle-01-pull-request-setting-implementation.md).

Status: **approved 2026-11-24** (revision 4).

Revision history:

- Revision 1 (`21b811d5`): first draft.
- Revision 2 (`754a42a1`): two independent reviews of revision 1 (source and feasibility: 3 Blocking, 8 Important, 7 Minor; contract and consistency: 2 Blocking, 16 Important, 15 Minor) and the owner's decisions 4 to 10.
- Revision 3 (`45a6ba9f`): a scoped re-review of revision 2 with an adversarial round (3 Blocking, 4 Important, 6 Minor, all in pull-only `repo sync`) and the owner's decision 11. The design's Part D is restated as required outcomes, built test-first with a design checkpoint.
- Revision 4: a third scoped re-review (0 Blocking, 5 Important, 5 Minor). The synchronization, not the binding, decides a new operation's mode. Revision 4 has not been independently reviewed.

## Purpose And Authority

This is the first Cycle of Wave 06. It makes a repository's mode (direct or pull request) and its forge a setting in `.manyhands/config.toml`, shows them in `repo inspect` and `repo list`, adds the library functions that recognise a forge and build a pull request link, and makes `repo sync` pull-only in a pull-request repository. Later Cycles build on it: onboarding (vh-c97se), close (vh-gy8gz), promote (vh-rphcs), after the merge (vh-ssepw) and the desktop (vh-p7st2).

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24 and merged as `0db9124d`: sections "Repository setting", "The pull request link" and "Repo sync in pull-request mode"; decisions 3, 9, 15, 17, 18 and 21.
2. This Cycle's Product-Owner Decisions, below.
3. AGENTS.md, including its Testing Strategy and local test gate.
4. The [canonical content schema RFC](../RFC/canonical-content-and-comment-schema.md), the [index RFC](../RFC/repository-index-persistence-and-refresh.md), the [Git workflow RFC](../RFC/git-workflow-and-conflict-recovery.md) and the [CLI contract](../RFC/cli-contract.md), each amended by this Cycle (design D-1 to D-7).

## Entry Evidence And Dependency Boundary

| Gate | State |
| --- | --- |
| RFC approved and on main | `0db9124d`, 2026-11-24 |
| Ticket branch holds current main | `origin/main` `6cfe0ab0` merged at `07e5fb9c` |
| Dependencies | none: vh-c2dtj has no `deps` |
| Dependents | vh-c97se, vh-gy8gz and vh-rphcs depend on this Cycle; vh-ssepw and vh-p7st2 depend on it through them |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work (owner, 2026-11-24) |

## Scope

1. The configuration: `integration` (`"direct"` or `"pull_request"`) and an optional `[forge]` table (`kind`, `web_url`, `pull_request_url`), validated on read and on write. A pull-request configuration is `format_version = 2`, and this build reads versions 1 and 2.
2. The index and the reads: the index's configuration rows store the mode and the forge, and `repo inspect` and `repo list` report them as `integration` and `forge`; human `repo inspect` prints them.
3. A forge module in the library: forge recognition, the default mode of RFC decision 9, the pull request link builder and its human line.
4. Pull-only `repo sync` in a pull-request repository: fetch and fast-forward only, never a merge or a push of the primary. A local primary with commits the remote lacks is refused with the new code `primary_has_local_commits`, which names them and holds nothing.
5. The amendments to the CLI contract and the three RFCs, and the shipped skills that describe configuration keys, `repo sync` and recovery.

## Explicit Exclusions And Downstream Obligations

- **No interim guard (decision 1).** Close, promote and remote selection keep their direct behaviour in a pull-request repository until their own Cycles. The RFC's evidence that no command commits to or pushes the primary in pull-request mode is delivered command by command: `repo sync` here, remote selection and enabling in Cycle 02, close in Cycle 03, promote in Cycle 04. A primary conflict cannot arise in a pull-only operation, because it never merges; a direct-mode operation left pending when the repository switched finishes in direct mode (decision 11).
- **Link storage and the JSON `pull_request` object (decision 2)** arrive with the first command that gives a link. Cycle 03 (vh-gy8gz) owns the per-item record of given links, its registry migration and the JSON schema of the `pull_request` object; Cycle 04 reuses them.
- Enabling in pull-request mode and every setting change through a setup branch: Cycle 02 (vh-c97se). Nothing in this Cycle writes `integration` or `[forge]`.
- The after-merge work, `merged_at` and the checkout warning: Cycle 05 (vh-ssepw).
- The desktop: no change in this Cycle beyond the named edits (decision 4). Labels and copy are Cycle 06's (vh-p7st2) and vh-fk279's.
- `item sync` is unchanged.

**Notes owed to other tickets at approval** (the planning skill's step 9):

- vh-c2dtj: its Scope's guard item is dropped (decision 1), and the link record and JSON object move (decision 2).
- vh-gy8gz: gains the per-item link record, its migration and the `pull_request` JSON schema.
- vh-rphcs: "the link is offered again" depends on Cycle 03's record.
- vh-c97se: no non-SSH publication remote can be selected, so its "local or `file://` remote" tests need the SSH harness; remote selection in a pull-request repository must commit byte-canonical configuration (K-5); clearing the remote of a pull-request repository is refused by validation (design Part A).
- vh-gy8gz, vh-rphcs and vh-c97se: the RFC's "CLI contract tests against a local remote" is not possible (design Baseline Fact 8).
- The Wave 06 document: Cycle 01's checklist drops the link record and the guard.

## Required Contract

1. **(owner's eye)** `integration` is `"direct"` or `"pull_request"`; a missing key means direct. Any other value, or a non-string, makes the configuration invalid.
2. **(owner's eye)** `integration = "pull_request"` requires `format_version = 2` and a `publication_remote`. A version 1 file that asks for pull requests is invalid. Amended during execution, 2026-11-24: the guarantee is not that an older build never acts on such a file in direct mode (a pre-mode build treats `integration` as an unknown key). It is that every configuration this build accepts as a pull-request configuration is version 2, every writer writes pull-request mode as version 2, and builds before this one refuse version 2 (the 0.2.1 parser refuses any version other than 1).
3. A version 2 file in direct mode is valid, and a rewrite writes it as version 1 (decision 8). Only pull-request mode is written as version 2. A direct-mode file without `[forge]` is written byte for byte as today. Versions other than 1 and 2 stay invalid.
4. `[forge]` is optional, and when present holds a `kind` or a `pull_request_url`. `kind` is one of `bitbucket-cloud`, `bitbucket-data-center`, `github`, `gitlab`. `web_url` and `pull_request_url` are absolute `https://` or `http://` URLs, and `pull_request_url` contains `{source}`. Unknown keys are kept on rewrite, inside `[forge]` as everywhere else.
5. **(owner's eye)** `repo inspect` and `repo list` report `integration` (`"direct"` or `"pull_request"` for a valid configuration, otherwise null) and `forge` (the configured `kind`, `web_url` and `pull_request_url`, or null when the configuration has no `[forge]`). It is the configured setting, not the forge recognised from the remote.
6. Forge recognition: a `[forge]` setting counts as recognised. Otherwise an SSH remote whose host is `bitbucket.org`, `github.com` or `gitlab.com` (any case) is that forge's cloud service. Any other host is unrecognised.
7. The default mode (RFC decision 9) is pull-request mode when there is a publication remote and its forge is recognised, and direct otherwise. This Cycle provides it as a library function; Cycle 02 calls it.
8. The link builder gives a source, a target, the forge and a URL or null, using the RFC's URL shapes and percent-encoded branch names, and never fails. Its human line is `pull request: <url>`, or `open a pull request from <source> into <target>` with no URL.
9. **(owner's eye)** In a pull-request repository, `repo sync`:
   - fast-forwards a local primary the remote is ahead of, pushes nothing, and answers as direct mode answers a fast-forward-only sync today: `already_applied`, `effects.publication` `current` (decision 4);
   - answers `already_applied`, `current`, when the two are equal, as today;
   - refuses a local primary with commits the remote lacks (ahead or diverged) with `primary_has_local_commits` (blocked, exit 3, no recovery action, every effect `not_requested`), before any merge, local update or push; only a push endpoint that does not hold the fetched primary is refused after the fast-forward, with nothing pushed. Amended during execution, 2026-11-24: that refusal comes in `push_call`, after the local fast-forward, in three cases: the push endpoint advertises no primary (the list is the fetched primary's 20 newest commits, `complete` false if there are more); it is strictly behind the fetched primary (the list is the commits it lacks, taken against its tip); or it is strictly ahead of the fetched primary (a teammate's merge landing mid-sync, or a separate push URL that is ahead; the list is empty and `complete` is true). In all three the person has no local-only commits, and running `repo sync` again with a new request ID fetches again (K-9). `data` is tagged `"type": "primary_local_commits"`; `data.local_commits` names up to 20 of them and `data.complete` says whether that is all. The operation ends with the refusal and holds nothing (decision 5), and the same request retried answers the same;
   - never merges or pushes the primary in an operation that began in pull-request mode, on any call of it, and never answers `published` for one; an operation that began in direct mode finishes in direct mode (decision 11); `item sync`, polls and close's and promote's primary children stay direct (decision 1);
   - keeps today's refusals when the root is not on the primary or not clean.
10. Direct mode is unchanged. Every existing answer, golden and test keeps its result, except the configuration object's two new fields and the named edits.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1-4 | A table-driven library test of parse and serialize over every rule, including byte-for-byte direct-mode output and unknown keys inside `[forge]` | planned |
| A2 | 2 | `git show v0.2.1:src/canonical.rs` holds "unsupported format_version; expected 1", recorded in the ledger at Task 0 | planned |
| A3 | 3 | Remote selection rewrites a pull-request configuration with its mode, forge, version and unknown keys | planned |
| A4 | 5 | Read tests and the read-contract and CLI goldens for `repo inspect` and `repo list` with the new fields; a test of human `repo inspect` | planned |
| A5 | 5 | The migration adds the columns once, and the first refresh after it fills them | planned |
| A6 | 6-8 | A table-driven test from remote URLs and forge settings to forge, default mode and link URL; a human-line test | planned |
| A7 | 8 | The owner opens a Bitbucket Cloud link built for a real work repository and it reaches the create-pull-request page (decision 7). Amended during execution, 2026-11-24: the owner confirmed by eye the Bitbucket Cloud link built for a placeholder repository (the owner's choice); not opened against a live forge | planned |
| A8 | 9 | The design's required outcomes O1 to O15 of Part D, each with its proof: library tests of the local-commit listing, the decision after each `graph_plan`, `push_call`'s refusal, the stored setting, `stopped` and the replay of the refusal's data | planned |
| A9 | 9 | One SSH case: a pull-only `repo sync` fast-forwards with the server unchanged, then refuses a local commit with the server's primary still unchanged and nothing held, and the same request retried answers the same; it writes the code's mutation golden. This meets the RFC's push evidence by showing no push is made (decision 6) | planned |
| A10 | 9 | `primary_has_local_commits` registered everywhere a result code is registered | planned |
| A11 | 10 | The fast tier and the SSH tier of design Testing, once at review-ready | planned |
| A12 | Scope 5 | The amended RFC sections and skills; `tests/cli/recovery_tables.rs` and `tests/cli/skills.rs` pass | planned |

## Product-Owner Decisions (2026-11-24)

1. **No interim guard.** The owner chose not to make close, promote and remote selection refuse in a pull-request repository before Cycles 02 to 04 deliver their pull-request paths, because this repository is direct mode, Manyhands is not yet used at work, and there are no other deployments. The ticket's scope item "a guard that refuses any commit to or push of the primary" is dropped, and no push-level guard is built.
2. **Links: the builder only.** The per-item record of given links and the JSON `pull_request` object move to Cycle 03, where the first link is given.
3. **Show the mode and the forge** in `repo inspect` and `repo list`.
4. **A pull-only fast-forward keeps today's answer** (`already_applied`, publication `current`), which the desktop already reads as current, never as shared. This replaces the first-round choice of a new "updated" answer: a replay could not reproduce it, because the journal records only "published" and "already current". The desktop needs no change in this Cycle.
5. **The `primary_has_local_commits` refusal ends its operation and holds nothing**, with no recovery action. A later `repo sync` checks again.
6. **The SSH case proves that no push is made** (the server's primary unchanged after a refusal where direct mode would have pushed), in place of a server that refuses pushes: a refusing hook is not portable to the Windows native runner.
7. **The link check is Bitbucket Cloud**, done by the owner right after the link builder is built (Task 2), in place of the RFC's "checked in planning".
8. **A version 2 direct-mode file is accepted** and written back as version 1.
9. **Mechanical test edits found by Task 0's sweep are pre-authorized**: an edit that keeps every assertion's intent is applied and logged as E-n with its reason. An edit that removes or weakens an assertion stops for the owner.
10. **No release until Cycle 04 lands.** The skills describe the new keys and pull-only `repo sync`, and say that pull-request mode is not usable end to end until close and promote support it.
11. **An operation finishes in the mode it began.** Each remote operation records whether it is pull-only when it starts, and every later call of it (resume, conflict continuation, cancellation, rerun) keeps that mode. A direct-mode `repo sync` interrupted and resumed after the repository switched finishes as direct, and may push the primary once more, as it intended.

12. **Test edits are approved.** At approval the owner approved the named edits and any other test edit this Cycle needs, which widens decision 9 to every test edit, each logged in the ledger with its reason.

At approval (2026-11-24): the owner approved the three documents at revision 4 (`6c45a2b5`) without a further review round, approved the test edits (decision 12), chose subagent-driven execution, and authorized implementation. Not authorized: any push other than the ticket branch, the close and its merge, a release (decision 10), and native CI.

## Known Limits

- K-1: Recognition covers the three cloud hosts. Bitbucket Data Center, or any self-hosted forge, needs `[forge]`, and Data Center gets no URL without `web_url`.
- K-2: Until Cycles 03 and 04, close and promote in a pull-request repository still push the primary (decision 1). A repository reaches pull-request mode only when its configuration is on the remote's primary: committed and pushed in direct mode, or merged through the forge. A local commit of the switch is refused by the next pull-only `repo sync` as a local commit.
- K-3: The forge module parses remote URLs for links only. It is not the SSH transport's parser, and is never used to connect or for host trust.
- K-4: No URL shape was checked against a live forge. Amended during execution, 2026-11-24: decision 7's check was the owner confirming the Bitbucket Cloud shape by eye on a link built for a placeholder repository; none was opened against a live forge. The others follow the forges' documented forms.
- K-5: Remote selection accepts only a byte-canonical configuration blob (`canonical_configuration_blob`). A hand-written version 2 file must be the serializer's output for remote selection to rewrite it.
- K-6: An operation begun in direct mode finishes in direct mode after a switch (decision 11), so it can push the primary once more.
- K-7: A crash after a `primary_has_local_commits` refusal has ended its operation, and before its request record is final, leaves a retry of that request answering `cancelled`. Nothing changed; a new `repo sync` refuses again (design R-16).
- K-8: A build without this Cycle that refreshes a migrated registry writes the configuration row without the new columns and clears the dirty mark, so `repo list` shows the mode as direct and no forge until the primary next changes; `repo inspect` reads the file and is not affected. Such a build also ignores `pull_only`: it refuses a version 2 configuration, so it cannot act in a pull-request repository, but after a switch back to version 1 it would resume a pull-only operation as direct.
- K-9: Amended during execution, 2026-11-24: When the push endpoint is ahead of the fetched primary (a merge landing between the fetch and the push check, or a separate push URL ahead), a pull-only `repo sync` refuses with `primary_has_local_commits` and an empty list, after its fast-forward; the code's message is not true in this case. Running `repo sync` again fetches again.

## Open Questions

None.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | Interim answer of close, promote and remote selection in a pull-request repository | decision | No interim guard (decision 1) | Owner, 2026-11-24 |
| Planning | Link storage and JSON object with no producer in this Cycle | decision | Moved to Cycle 03 (decision 2) | Owner, 2026-11-24 |
| Planning | Callers cannot see the mode | decision | `integration` and `forge` in the configuration object (decision 3) | Owner, 2026-11-24 |
| Planning | New struct fields on `RepositoryConfig` break 16 construction sites, 4 in SSH-harness test files | ruling | Typed accessors over the existing table (design R-1) | Design, Baseline Fact 2 |
| Planning | Changing the SSH transport's URL parser forces every SSH target | ruling | A separate parser for links (design R-2) | Design, Baseline Fact 7 |
| Review round 1, source and feasibility | 3 Blocking, 8 Important, 7 Minor | — | All resolved in revision 2; see the design's Review Record | This revision |
| Review round 1, contract and consistency | 2 Blocking, 16 Important, 15 Minor | — | All resolved in revision 2, with decisions 4 to 10 | This revision |
| Review round 1 | A refusal under the existing divergence path keeps the reservation | decision | The refusal ends its operation (decision 5) | `ended_if_unresolved`; `tests/mutation_remote.rs` divergence case |
| Review round 1 | A new "updated" answer cannot be replayed | decision | Today's answer kept (decision 4) | `remote/state.rs` `authoritative_kind` |
| Review round 1 | The RFC's refusing-remote evidence | decision | No push made, shown by the SSH case (decision 6) | — |
| Review round 1 | The link shapes are not checked against a forge | decision | Bitbucket Cloud, after Task 2 (decision 7) | — |
| Review round 1 | Version 2 in direct mode needs an explicit compatibility decision | decision | Accepted, written back as 1 (decision 8) | Canonical schema RFC, Migration |
| Review round 1 | Unplanned premise edits | decision | Mechanical edits pre-authorized (decision 9) | Fast-writes ledger: 11 planned edits, 85 made |
| Review round 1 | Half-delivered pull-request mode in a release | decision | No release until Cycle 04 (decision 10) | — |
| Review round 2, scoped re-review with adversarial round | 3 Blocking, 4 Important, 6 Minor | — | All resolved in revision 3; see the design's Review Record | This revision |
| Review round 2 | A setting read afresh per call strands an operation begun in the other mode | decision | The mode belongs to the operation (decision 11) | `sync.rs` merge-child and push-prepared refusals under `refuse_divergence` |
| Review round 3, scoped re-review with adversarial round | 0 Blocking, 5 Important, 5 Minor | — | Resolved in revision 4, not independently reviewed; see the design's Review Record | This revision |
| Approval | Documents, test edits, execution method, implementation | decision | Approved 2026-11-24 (decision 12 and the approval line) | Owner, in the planning chat |
