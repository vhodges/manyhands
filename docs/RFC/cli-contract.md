---
title: "CLI Contract RFC"
date: 2026-10-05
status: approved
author: "Vince Hodges <vhodges@gmail.com> && Codex"
manyhands_managed: true
manyhands_kind: document
id: "01M46S07YFTWMB56ZFSS00BKMN"
---

# CLI Contract RFC

## Status and goals

This approved RFC specifies `manyhands-cli` for people, agents and CI. It implements
`MH-CLI-001` and the equivalent repository, credential, content, discussion,
index and collaboration operations in [PRD v0.5](../PRD/mvp.md), subject to the
[architecture RFC](mvp-rfc.md). The product owner approved it on 2026-10-05;
Wave/Cycle planning and implementation gates still apply.
The [runtime RFC](application-runtime-and-polling.md) owns polling and session
lifetime; the [desktop RFC](desktop-information-architecture-and-editor.md)
defines the graphical counterpart. Remaining feasibility and verification gates
are in the [review register](wave-03-rfc-review.md).

The CLI is a headless adapter over shared domain services. It runs without a
display or the `desktop` feature, never launches system Git as its backend,
and provides no daemon mode or resident background polling/indexing. A
no-argument call prints concise help and exits zero, preserving the skeleton
smoke-test intent.

## Invocation and target resolution

```text
manyhands-cli [--json] [--non-interactive] <resource> <verb>
    [--repo <path>] [--id <ULID>] [--key-id <opaque-id>]
    [--operation-id <ULID>] [--request-id <ULID>]
    [--input <file|->] [--prepare | --confirm <confirmation-id>]
```

`--help` and `--version` exit zero without starting services. Global options
are accepted before or after the subcommand. Unknown commands/options are usage
errors. Repository-scoped commands require `--repo`; there is no implicit
"most recently selected" repository. Paths resolve relative to the invocation
directory, then normalize through the domain service. A linked worktree maps
to its repository's common Git identity, while item operations still resolve
their one canonical context. File paths inside input objects are documented
as repository-relative or invocation-relative by their command.

All item/comment identifiers are full canonical ULIDs. Titles and display
abbreviations are never mutation selectors. Nonconforming resources without a
valid ID are inspected/repaired by exact path plus observation token. Short IDs
remain separate research; no lossy alias is introduced by this RFC.

`--input` contains one UTF-8 JSON object, including Markdown bodies as strings.
`-` consumes stdin once. Mutations do not accept bodies/passphrases in command
arguments and do not launch an implicit `$EDITOR`. A caller can edit files
with its own tools, then submit them. Unknown input fields and invalid enum
values are rejected to catch mistakes. Missing and explicit `null` differ:
missing optional update fields leave values unchanged; `null` clears only a
documented nullable field. Empty strings do not mean clear.

## Command taxonomy

`--repo` is required except for `repo list`, `key` commands, `host list`,
`id new` and help/version operations. `--id` selects an item,
`--key-id` selects a key, and `--operation-id` selects an operation; their names
are not interchangeable. Every verb has human and JSON output. Each row lists command-specific inputs;
IDs, request IDs, observation and consent rules below apply where relevant.

| Commands | Inputs and behavior |
| --- | --- |
| `repo list` | Registered repositories and their latest discovery/recovery state; no network. |
| `repo inspect` | Selected path, configuration, primary, identity and observed problems. Does not enable it. |
| `repo create`, `repo enable` | Input `primary_branch`; create requires an unused/empty target. Missing identity returns recovery before the initialization commit. |
| `repo remove` | Remove registration only; preserve repository and drafts. Confirmation required. |
| `repo identity` | Inspect effective name/email and source. |
| `repo identity-set` | Input `name`, `email`; explicitly writes repository-local Git config, never global config. Confirmation required. |
| `remote list` | Names, redacted locations and publication selection; never echo embedded credentials. |
| `remote add` | Input `name`, `url`; add the named configuration, without implicit publication selection. |
| `remote remove` | Input `name`; show publication/polling effects and require confirmation. |
| `remote select` | Input `name` (SSH only) or `null` to clear publication selection. |
| `key list`, `key show` | Non-secret registrations, selection, ownership and fingerprint; show takes an opaque key ID. |
| `key generate` | Input `label`, `protection` (`passphrase` or `none`); passphrase comes only from masked terminal interaction. |
| `key import` | Input `label`, invocation-relative `private_key_path`, optional `public_key_path`; register by reference. |
| `key select`, `key clear` | Explicit shared selection or clearing; never changes Git commit identity. |
| `key remove`, `key delete` | Unregister only, or separately delete owned generated files. Delete requires exact effect confirmation and cleared selection. Imported deletion is rejected. |
| `key public` | Return public-key text/fingerprint if available; never private contents. |
| `host list`, `host inspect` | Non-secret pins; inspect input `authority`; `--repo` identifies the relevant repository for an observation. |
| `host approve`, `host replace` | Input exact authority and presented fingerprint; replacement also requires old fingerprint. Explicit confirmation and transport re-observation required. |
| `document list`, `document show` | Tree/list or complete item DTO, including source, metadata, observation and context. |
| `folder create` | Input repository-relative `path` under `docs/`; local empty folder until content is added. |
| `document create` | Input `item_id`, repository-relative `path`, `title`, `body`. Explicit IDs support safe automation retries. |
| `document save` | Input `observation`, optional `title`, `body`; preserve unknown metadata. No implicit publication. |
| `document move` | Input source `observation`, `destination_path`, `destination_observation`; preserve ID and checkpoint both paths. |
| `document repair`, `ticket repair` | Input exact `path`, `observation`, corrected complete `source`; preview and explicit confirmation. Validate identity/schema and use an owned authoring context, never patch primary as a shortcut. |
| `ticket list`, `ticket show` | Canonical metadata/body, context, conformity and observation. List includes closed tickets unless filtered. |
| `ticket create` | Input `item_id`, `title`, `type`, `status`, optional `project`, `team`, `body`. Closure metadata cannot be supplied; free-form status may be `closed` without making the ticket lifecycle-closed. |
| `ticket save` | Input `observation` and changed metadata/body. `project`/`team` can be cleared with null. Project status does not control closure; `ticket close` owns `closed_at`/`closed_by`. Reopening is outside MVP. |
| `comment list` | `--id` selects the parent item; return ordered roots/replies with parent IDs and visible malformed entries. |
| `comment add` | `--id` selects item; input `comment_id`, `body`, optional `parent_id`. One checkpoint then immediate configured sync; no-remote means local pending. |
| `item sync`, `repo sync` | Deliberate item-context or primary synchronization. Unsaved caller files are not implicitly submitted. |
| `document promote`, `ticket close` | Exact item effect preview and confirmed integration/publication/cleanup. CLI callers save drafts first; final domain checkpoint still handles required lifecycle metadata. |
| `index status`, `index refresh`, `index rebuild` | Inspect cache freshness, or local scan/rebuild. Never fetch or rewrite canonical state. Rebuild supports explicit root after cache loss. |
| `poll status`, `poll configure`, `poll pause`, `poll resume`, `poll once` | Durable policy and observed results; configure input `interval_seconds` in 60–3600. Once is explicit remote polling, not synchronization. No process discovery or worker control. |
| `operation list`, `operation show`, `operation resume` | Inspect/reconcile or deliberately resume one operation ID. Resume never infers permission for a different target/action. |
| `conflict show`, `conflict resolve` | Select operation ID; inspect base/local/remote and expected path observations, then submit explicit resolutions. |
| `id new` | Generate and return a canonical ULID for caller-owned requests/items/comments; no repository mutation. |

List ordering is deterministic: repositories by normalized root, documents by
relative path then ID, tickets by latest content-change time descending then ID,
and operations by start time then ID. Comment ordering remains schema-defined.
Ticket list supports exact `--status`, `--type` and `--project` filters. Lists
return all matching results in v1; silent truncation is forbidden. Paging is a
future additive contract, not an undocumented default limit.

Repair cannot bypass identity uniqueness, a closed ticket's lifecycle fields,
or the prohibition on reopening. Assigning an ID to marker-only content is an
explicit adoption effect in the confirmation preview. Existing valid IDs remain
unchanged. A document folder target resolves in the selected item's context
when organizing an existing item, or in the documentation tree for an empty
folder; creation never implies publication or a placeholder-file commit.

## Observations, request identity and retries

Show/inspect returns an opaque observation token representing the exact source,
destination and relevant context state. A token is an optimistic concurrency
precondition, not authentication. Writes require it for existing resources;
creation explicitly expects absence. Stale observations produce
`external_change`, preserving input and canonical state. There is no `--force`
override. Caller-provided source/body never enters operation diagnostics.

Every mutating invocation has a request ULID. JSON/noninteractive mutation
requires caller-supplied `--request-id`; human interactive mode may allocate one
and prints it before work begins. Reusing a request ID requires the same
repository, command, target and semantic input. The adapter records non-secret
intent identity and completed effects, not raw body/credentials. For body-bearing
requests it records a content digest sufficient to reject altered replay.
Key-generation replay compares non-secret intent and owned-key creation state;
it MUST NOT persist a passphrase or a passphrase-derived digest.

The same request after partial completion returns existing effects and resumes
only eligible remaining work. Reusing it with different input is a conflict,
never a new operation. Comment and item IDs also remain fixed across retries.
After cache loss, if canonical/Git evidence cannot prove safe replay, return
`recovery_required`; do not promise indefinite exactly-once execution from a
rebuildable database. `operation resume` may need original input again and says
so explicitly. It cannot silently reconstruct an unsaved body from an operation
record. These are required adapter/domain additions, not guarantees of current
Wave 01 request structs.

## Consent and noninteractive input

Terminal mode may prompt for missing identity, passphrase, host approval and
lifecycle consent. `--json`, `--non-interactive`, or absence of a controlling
terminal disables all prompts. A JSON command that needs a secret returns
`unlock_required`; it does not read stdin again or fall back to a key agent.
For protected keys, interactive one-shot commands unlock for that invocation.
No pipe/argument/environment secret channel or desktop unlock sharing is
included. There is no resident CLI credential session.

Two-phase confirmation makes automation explicit:

1. Run the exact action with `--prepare` to get a non-secret confirmation ID and
   effect summary. It performs only prerequisite observation and permitted
   application-local bookkeeping; it does not save, push, merge or delete.
2. Resubmit with `--confirm <id>`, the same action/target/input, and a mutation
   request ID. The service rechecks observations before the first effect.

The confirmation binds item/key/repository, branch/ref OIDs, remote selection,
affected paths, intended cleanup and action-specific fields. It expires after
ten minutes if unused. Changed external observations require a new preview.
After acceptance, durable operation consent covers only unchanged remaining
effects; the operation's own completed transitions do not invalidate its retry.
An expired or missing confirmation after cache loss requires a new preview.
No global `--yes` can approve host trust, key deletion or a changed target.

`host approve/replace` bind exact fingerprints separately from lifecycle
consent, and persist trust only after the observed host presents them again.
Republishing a remotely deleted previously published item branch also requires
an explicit prepare/confirm effect summary; ordinary sync must not recreate it
silently. Previewing arbitrary raw input does not make it valid or authorized.

## JSON v1 and human output

One-shot `--json` writes exactly one UTF-8 JSON object plus newline to stdout
on every handled outcome, including usage/validation errors when `--json` can
be recognized. No progress, ANSI escapes, prompts or logs go to stdout. Fatal
process termination/broken output streams may prevent a final object; clients
must use their request ID to inspect recovery. Human mode is readable prose
plus stable IDs and actionable next steps; its layout is not a parsing API.

All envelope fields below are required; nullable fields use JSON null. Version 1
uses strings for IDs/OIDs/timestamps and never encodes an ID as a JSON number.

| Field | Type and allowed values |
| --- | --- |
| `schema_version` | Integer `1`. |
| `command` | Canonical two-word command string. |
| `request_id` | ULID string or null for read-only/generated-help results. |
| `operation_id` | Domain operation ID string or null when no operation began. |
| `outcome` | `success`, `noop`, `partial`, `blocked`, `cancelled`, `error`. |
| `code` | Stable lower_snake_case result/recovery code; never raw backend error text. |
| `message` | Redacted human explanation, not a machine discriminator. |
| `scope` | Object with nullable `repository`, `item_id`, `branch`, `worktree`, `remote` strings. |
| `effects` | Object defined below; separates completed work from pending work. |
| `data` | Command-specific object, or null when no requested data is available. |
| `recovery` | Array of objects with `action`, nullable `operation_id`, and non-secret structured `arguments`. No shell command strings. |

`effects` always contains: `write` (`not_requested`, `unchanged`, `written`),
`checkpoint` (`not_requested`, `unchanged`, `committed`, `pending`),
`discovery` (`not_requested`, `current`, `pending`), `publication`
(`not_requested`, `published`, `current`, `pending`), `integration`
(`not_requested`, `complete`, `pending`), `cleanup`
(`not_requested`, `complete`, `pending`), and nullable string `commit_oid`.
It describes the operation including effects observed from an earlier attempt.

Item DTOs include ID, kind, repository-relative path, required and optional
canonical metadata, unknown metadata, body, observation, context/provenance,
conformity problems and content-change timestamp. `show` additionally includes
complete canonical `source`; list omits bodies/source. Malformed entries use
nullable IDs/metadata plus exact path and problem code. Comment DTOs include
ID, item ID, parent ID, author, created timestamp, body and conformity problems.
List `data` is `{ "items": [...], "complete": true }`; it is never an unversioned
top-level array. Read operations intentionally return requested source; mutation
outcomes, errors, polling logs and progress do not echo it.

Other DTOs expose the table's named non-secret fields, plus problem codes.
Command-specific JSON Schemas and golden fixtures MUST be published alongside
the CLI implementation before its Cycle can exit; they must follow this
envelope and specify required/nullable fields for every command. This RFC does
not authorize directly serializing Rust enums as the public API.

Example: a comment checkpoint succeeded but configured publication failed:

```json
{
  "schema_version": 1,
  "command": "comment add",
  "request_id": "01K7F6H9J2N4Q6S8V0X2Z4B6DB",
  "operation_id": "01K7F6H9J2N4Q6S8V0X2Z4B6DC",
  "outcome": "partial",
  "code": "publication_pending",
  "message": "Comment saved locally; publication needs retry.",
  "scope": {
    "repository": "/projects/example",
    "item_id": "01K7F6H9J2N4Q6S8V0X2Z4B6DA",
    "branch": "manyhands/ticket/01K7F6H9J2N4Q6S8V0X2Z4B6DA",
    "worktree": null,
    "remote": "origin"
  },
  "effects": {
    "write": "written",
    "checkpoint": "committed",
    "discovery": "current",
    "publication": "pending",
    "integration": "not_requested",
    "cleanup": "not_requested",
    "commit_oid": "0123456789abcdef0123456789abcdef01234567"
  },
  "data": { "comment_id": "01K7F6H9J2N4Q6S8V0X2Z4B6DD" },
  "recovery": [{
    "action": "operation.resume",
    "operation_id": "01K7F6H9J2N4Q6S8V0X2Z4B6DC",
    "arguments": {}
  }]
}
```

Envelope breaking changes require a new schema version. Additive object fields
are allowed; consumers ignore unknown fields. Outcome/effect enum changes and
command input meaning changes are breaking. Recovery codes are extensible:
clients handle an unknown code through outcome and exit class, retaining the
structured result rather than retrying blindly.

## Exit statuses and partial completion

| Exit | Meaning |
| --- | --- |
| `0` | Requested work complete or documented no-op. Local-only comment/promotion/closure can succeed with publication pending when no remote exists. |
| `2` | Command/input/validation error; requested mutation not started. |
| `3` | Action blocked before a requested durable effect: identity, unlock, host/consent requirement, stale observation, conflict or other recovery prerequisite. |
| `4` | Incomplete operation after any durable effect, including checkpoint/index, configured publication or cleanup failure. |
| `5` | Retryable busy/yield/transport failure before any durable effect. |
| `1` | Unexpected internal/I/O failure not otherwise classified. Preserve typed completed effects if known. |
| `130` | User cancellation of a one-shot operation; effects identify any completed work. |

Classification order is cancellation, known partial effects, input error,
required recovery, transient inability, unexpected failure, then success/no-op.
Input validation after an earlier replay's completed effects returns `4`, not
`2`. Explicit sync without a publication remote returns `3`; it did not fulfill
the requested publication action. A normal local save with no sync requested
returns `0`. A successful list may include nonconforming entries; stale or failed
requested refresh is incomplete and must be represented separately.

## Conflict and cancellation behavior

`conflict show` returns operation scope, merge observations and each affected
path's base/local/remote source and conflict kind. `conflict resolve` takes an
array of `{path, observation, source}` resolutions for every conflicted owned
canonical path. It validates all resolutions before any write, checkpoints
through the domain boundary and resumes only the recorded operation's permitted
remaining work. It does not run external merge tools or accept a blanket
"ours/theirs" policy. Changed observations require a fresh resolution.
Noncanonical and unsupported structural conflicts remain visible and preserved;
the selected scope is guided external-tool recovery, recorded as W3-04 in the
review register. After external repair, `operation show` re-observes actual Git
state and `operation resume` deliberately continues eligible remaining work.
Neither command stages arbitrary code or treats merely deleting text markers
as proof that a Git conflict is resolved.

`poll once`, `index refresh` and `index rebuild` complete one explicit operation
and exit. They do not install a service, launch a child poller or leave a thread
running after the command completes. `poll status` reports persisted policy and
observed outcomes; it does not discover or control a desktop process. There is
no streaming daemon JSON format; every command follows the single-result
envelope contract. External server/CI scheduling may invoke one-shot commands,
but resident CLI scheduling is outside the MVP. A possible future daemon is
intended for server-side change watching and automation triggers, not ordinary
user-machine polling; it requires a separate PRD/enhancement design.

Ctrl-C requests safe-point cancellation and exits `130` after the final result
when possible. Another forced termination is interruption, never evidence of
rollback. Platform-native stop handling follows the runtime RFC.

## Alternatives, risks and required evidence

Structured file/stdin input is selected over large flag sets or an implicit
editor because it preserves multiline input and explicit concurrency tokens.
One final JSON object is selected over streaming all commands because callers
can inspect a single authoritative outcome.
Noninteractive secret ingestion and credential IPC are excluded by the selected
session model; this limits protected-key automation and is deliberate.

Acceptance requires real-process tests of every command's human/JSON result,
input failure, no-op and applicable recovery/exit class, with golden v1 schemas.
Pipe multiline/unicode bodies, invalid input, stdin EOF and terminal absence;
prove no prompt consumes body input. Run retries after lost stdout, partial
comment publication, indexing failure and cleanup failure, including cache loss
and altered-request rejection. Verify exact consent bindings, secret redaction,
unknown/changed hosts, explicit identity, missing keys and non-deletion of
imported key material.

Run the explicit CLI counterparts of all six PRD journeys with real repositories
and SSH transport; assert Git objects and canonical content, not only output
strings. The background
journey uses explicit `poll once` to prove the CLI counterpart; automatic
scheduling is desktop-only. Verify safe cancellation and overlap with a desktop
operation through the existing domain protocol, no resident worker after exit,
and no display or desktop dependency. These are future acceptance obligations;
the current `manyhands-cli` entry point is still an empty scaffold at the draft base.
