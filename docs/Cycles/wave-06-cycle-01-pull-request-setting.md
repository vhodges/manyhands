---
title: "Wave 06 Cycle 01: Pull-Request Setting, Link Builder And Pull-Only Repo Sync"
date: 2026-11-24
status: "draft"
author: "Claude"
manyhands_managed: true
manyhands_kind: document
id: "01M878QWAET79VP1S0EXVCEMAC"
---

# Wave 06 Cycle 01: Pull-Request Setting, Link Builder And Pull-Only Repo Sync

Ticket: vh-c2dtj (`01M87683441PRAYQJQBQBG96YY`), branch `manyhands/ticket/01M87683441PRAYQJQBQBG96YY`. Wave: [Wave 06](../Waves/wave-06-pull-request-workflow.md). Design: [Cycle 01 design](../plans/2026-11-24-wave-06-cycle-01-pull-request-setting-design.md). Plan: [Cycle 01 implementation plan](../plans/2026-11-24-wave-06-cycle-01-pull-request-setting-implementation.md).

Status: **draft** (revision 1), not yet independently reviewed.

## Purpose And Authority

This is the first Cycle of Wave 06. It makes a repository's mode (direct or pull request) and its forge a setting in `.manyhands/config.toml`, shows them in `repo inspect` and `repo list`, adds the library functions that recognise a forge and build a pull request link, and makes `repo sync` pull-only in a pull-request repository. Later Cycles build on it: onboarding (vh-c97se), close (vh-gy8gz), promote (vh-rphcs), after the merge (vh-ssepw) and the desktop (vh-p7st2).

Authorities, in order:

1. The [Pull-Request Mode RFC](../RFC/pull-request-mode.md), approved 2026-11-24 and merged as `0db9124d`: sections "Repository setting", "The pull request link" and "Repo sync in pull-request mode"; decisions 3, 9, 15, 17, 18 and 21.
2. This Cycle's Product-Owner Decisions, below.
3. AGENTS.md, including its Testing Strategy and local test gate.
4. The [canonical content schema RFC](../RFC/canonical-content-and-comment-schema.md), amended by this Cycle for `format_version = 2`, and the [CLI contract](../RFC/cli-contract.md), amended for the new code, the configuration fields and pull-only `repo sync`.

## Entry Evidence And Dependency Boundary

| Gate | State |
| --- | --- |
| RFC approved and on main | `0db9124d`, 2026-11-24 |
| Ticket branch holds current main | `origin/main` `6cfe0ab0` merged at `07e5fb9c`; branch head `adeae732` |
| Dependencies | none: vh-c2dtj has no `deps` |
| Dependents | vh-c97se, vh-gy8gz and vh-rphcs depend on this Cycle; vh-ssepw and vh-p7st2 depend on it through them |
| Deployments in pull-request mode | none: this repository is direct mode and Manyhands is not yet used at work (owner, 2026-11-24) |

## Scope

1. The configuration: `integration` (`"direct"` or `"pull_request"`) and an optional `[forge]` table (`kind`, `web_url`, `pull_request_url`), validated on read and on write. A pull-request configuration is `format_version = 2`, and this build reads versions 1 and 2.
2. The index and the reads: the index's configuration rows store the mode and the forge, and `repo inspect` and `repo list` report them as `integration` and `forge` in the configuration object; human `repo inspect` prints them.
3. A forge module in the library: forge recognition from the publication remote's URL or the forge setting, the default mode of RFC decision 9, the pull request link builder, and its human line.
4. Pull-only `repo sync` in a pull-request repository: fetch and fast-forward only, never a merge or a push of the primary. A local primary with commits the remote lacks is refused with the new code `primary_has_local_commits`, which names them.
5. The desktop's classification of a `repo sync` answer, so that a pull-only result reads as updated or current, never as shared.
6. The amendments to the CLI contract, the canonical content schema RFC and the index RFC, and the shipped skills that describe configuration keys, `repo sync` and recovery.

## Explicit Exclusions And Downstream Obligations

- **No interim guard (decision 1).** Close, promote, remote selection and conflict resolution keep their direct behaviour in a pull-request repository until their own Cycles. The RFC's evidence that no command commits to or pushes the primary in pull-request mode is delivered command by command in Cycles 02 to 04.
- **Link storage and the JSON `pull_request` object (decision 2)** arrive with the first command that gives a link: Cycle 03 (vh-gy8gz) owns the per-item record of given links, its registry migration and the JSON schema of the `pull_request` object, and Cycle 04 reuses them.
- Enabling in pull-request mode and every setting change through a setup branch: Cycle 02 (vh-c97se). Nothing in this Cycle writes `integration` or `[forge]`; a repository is switched by editing its configuration by hand.
- The after-merge work, `merged_at` and the checkout warning: Cycle 05 (vh-ssepw).
- Desktop labels and copy ("Update development", the Add repository choice): Cycle 06 (vh-p7st2) and vh-fk279.
- `item sync` is unchanged.

## Required Contract

1. **(owner's eye)** `integration` is `"direct"` or `"pull_request"`; a missing key means direct. Any other value, or a non-string, makes the configuration invalid.
2. **(owner's eye)** `integration = "pull_request"` requires `format_version = 2` and a `publication_remote`. A version 1 file that asks for pull requests is invalid, so a build without this Cycle can never act on one in direct mode.
3. A version 2 file in direct mode is valid. A rewrite writes version 2 only for pull-request mode, so a direct-mode file is written as version 1, byte for byte as today. Versions other than 1 and 2 stay invalid.
4. `[forge]` is optional. `kind` is one of `bitbucket-cloud`, `bitbucket-data-center`, `github`, `gitlab`. `web_url` and `pull_request_url` are absolute `https://` or `http://` URLs, and `pull_request_url` contains `{source}`. Unknown keys are kept on rewrite, inside `[forge]` as everywhere else.
5. **(owner's eye)** `repo inspect` and `repo list` report `integration` (`"direct"`, `"pull_request"`, or null unless the configuration is valid) and `forge` (the configured `kind`, `web_url` and `pull_request_url`, or null) in the configuration object.
6. Forge recognition: a configured `[forge]` wins. Otherwise an SSH remote whose host is `bitbucket.org`, `github.com` or `gitlab.com` (any case) is that forge's cloud service. Any other host is unrecognised.
7. The default mode (RFC decision 9) is pull-request mode when the publication remote's forge is recognised or configured, and direct otherwise. This Cycle provides it as a library function; Cycle 02 calls it.
8. The link builder gives a source, a target, the forge and a URL or null, using the RFC's URL shapes and percent-encoded branch names, and never fails. Its human line is `pull request: <url>`, or `open a pull request from <source> into <target>` with no URL.
9. **(owner's eye)** In a pull-request repository, `repo sync`:
   - fast-forwards a local primary the remote is ahead of, pushes nothing, and answers `ok` with `effects.publication` `not_requested` and `effects.commit_oid` the new head;
   - answers `already_applied` with `effects.publication` `current` when the two are equal, as today;
   - refuses a local primary that has commits the remote lacks (ahead or diverged) with `primary_has_local_commits` (blocked, exit 3), before any merge, local update or push, with `data.local_commits` naming up to 20 of them and `data.complete` saying whether that is all.
10. **(owner's eye)** The desktop classifies a pull-only fast-forward as updated and an equal result as current. Neither reads as shared.
11. Direct mode is unchanged. Every existing answer, golden and test keeps its result, except the configuration object's two new fields and the named edits.

## Acceptance And Exit Evidence

| # | Contract | Proof | Status |
| --- | --- | --- | --- |
| A1 | 1-4 | A table-driven library test of parse and serialize over every rule, including byte-for-byte direct-mode output and unknown keys inside `[forge]` | planned |
| A2 | 2 | The base parser (`6cfe0ab0`) refuses `format_version = 2`, recorded as a baseline fact read from the source | planned |
| A3 | 5 | Read contract and CLI goldens for `repo inspect` and `repo list` with the new fields; a read test of a pull-request configuration | planned |
| A4 | 6-8 | A table-driven test from remote URLs and forge settings to forge, default mode and link URL; a human-line test | planned |
| A5 | 9 | Library tests of the pull-only decision through the sync tests' fetch and push stand-in: remote ahead, equal, local ahead, diverged, and the 20-commit cap | planned |
| A6 | 9 | One SSH case: a pull-only `repo sync` fast-forwards and the server's primary is unchanged; a second assertion that a local commit is refused with nothing pushed | planned |
| A7 | 9 | `primary_has_local_commits` registered: the result-code design table, the envelope and request schemas, the recovery tables and the golden-or-exemption list | planned |
| A8 | 10 | Desktop classification rows for a pull-only fast-forward and an equal result | planned |
| A9 | 11 | The fast tier and the SSH tier that AGENTS.md selects, once at review-ready | planned |
| A10 | Scope 6 | The amended RFC sections and skills, checked by `tests/cli/skills.rs` where it covers them | planned |

## Product-Owner Decisions (2026-11-24)

1. **No interim guard.** The owner chose not to make close, promote and remote selection refuse in a pull-request repository before Cycles 02 to 04 deliver their pull-request paths, because this repository is direct mode, Manyhands is not yet used at work, and there are no other deployments. The ticket's scope item "a guard that refuses any commit to or push of the primary" is dropped from this Cycle, and no push-level guard is built.
2. **Links: the builder only.** The per-item record of given links and the JSON `pull_request` object move to Cycle 03, where the first link is given.
3. **Show the mode and the forge** in `repo inspect` and `repo list` from this Cycle.
4. **The desktop classifies a pull-only result as updated** (or current), never as shared. Labels and copy stay for Cycle 06.

At approval: to be recorded.

## Known Limits

- K-1: Recognition covers the three cloud hosts. Bitbucket Data Center, or any self-hosted forge, needs `[forge]`, and Data Center gets no URL without `web_url`.
- K-2: A repository switched to pull-request mode by hand before Cycles 03 and 04 land can still jam on a close or promote (decision 1).
- K-3: The forge module parses remote URLs for links only. It is not the SSH transport's parser, and is never used to connect or for host trust.
- K-4: The URL shapes follow the forges' documented "create pull request" pages. They are not checked against live forges in this Cycle (design U-1).

## Open Questions

None.

## Review Record

| Source | Concern | Classification | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| Planning | Interim answer of close, promote and remote selection in a pull-request repository | decision | No interim guard (decision 1) | Owner, 2026-11-24 |
| Planning | Link storage and JSON object with no producer in this Cycle | decision | Moved to Cycle 03 (decision 2) | Owner, 2026-11-24 |
| Planning | Callers cannot see the mode | decision | `integration` and `forge` in the configuration object (decision 3) | Owner, 2026-11-24 |
| Planning | The desktop would read a pull-only result as a share | decision | Classified as updated or current (decision 4) | Owner, 2026-11-24 |
| Planning | New struct fields on `RepositoryConfig` break 16 construction sites, 4 in SSH-harness test files | ruling | Typed accessors over the existing table (design R-1) | Design, Baseline Fact 2 |
| Planning | Changing the SSH transport's URL parser forces every SSH target | ruling | A separate parser for links (design R-2) | Design, Baseline Fact 7 |
