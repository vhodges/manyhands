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
directory, then normalize through the domain service. For reads, a linked
worktree maps to its repository's common Git identity, while item operations still resolve
their one canonical context. File paths inside input objects are documented
as repository-relative or invocation-relative by their command.

Amended 2026-10-20 by C2: every mutation refuses a linked worktree's root with
`not_repository_root`. Mutation roots are joined to the invocation directory,
not canonicalized by the CLI: create can name an absent root, and remove can
name the exact stored root of a missing registration directory.

All item/comment identifiers are full canonical ULIDs. Titles and display
abbreviations are never mutation selectors. Nonconforming resources without a
valid ID are inspected/repaired by exact path plus observation token. Short IDs
remain separate research; no lossy alias is introduced by this RFC.

Amended 2026-10-18 by C1: reads take their inputs as flags; `--input`,
`--request-id`, `--prepare` and `--confirm` are mutation-only and are refused
by C1 reads. `--non-interactive` is accepted but has no effect in C1.

Mutation `--input` contains one UTF-8 JSON object, including Markdown bodies as strings.
`-` consumes stdin once. Mutations do not accept bodies/passphrases in command
arguments and do not launch an implicit `$EDITOR`. A caller can edit files
with its own tools, then submit them. Unknown input fields and invalid enum
values are rejected to catch mistakes. Missing and explicit `null` differ:
missing optional update fields leave values unchanged; `null` clears only a
documented nullable field. Empty strings do not mean clear.

Amended 2026-10-18 by C1: `MANYHANDS_DATA_DIR` is an advanced/testing override
for the application data directory. Unset uses the platform default; a nonempty
absolute path is used directly, and a relative path resolves against the
invocation directory. An empty value is `invalid_path`, exit 2. Opening services
may create/migrate this directory; help, version, usage errors and unavailable
commands do not open it or initialize the Git transport.

## Command taxonomy

`--repo` is required except for `repo list`, `key` commands, `host list`, `host inspect`,
`request show`, `request cancel`, `id new` and help/version operations.
Amended 2026-10-20 by C2: the request verbs refuse `--repo` and select the
application-level record with required `--request-id`. `--id` selects an item,
`--key-id` selects a key, and `--operation-id` selects an operation; their names
are not interchangeable. Every verb has human and JSON output. Each row lists command-specific inputs;
IDs, request IDs, observation and consent rules below apply where relevant.

Amended 2026-10-18 by C1: `host inspect --authority <host[:port]>` reads
application-global pins and refuses `--repo`; bracketed IPv6 is accepted and
the port defaults to 22. `folder list` joins the reads. `document show` and
`ticket show` require exactly one of `--id` or repository-relative `--path`;
only `--path` accepts an optional `--context` worktree path. `path observe`
and `ticket check` move to C3. F2's `request show`, `request cancel` and
`host observe` join C2, and `operation abandon` joins C4.
Amended 2026-10-20 by C2: all eighteen C2 verbs are available with the rules
below; C3–C5 verbs remain cataloged as planned until their delivering Cycles.

| Commands | Inputs and behavior |
| --- | --- |
| `repo list` | Registered repositories and their latest discovery/recovery state; no network. |
| `repo inspect` | Selected path, configuration, primary, identity and observed problems. Does not enable it. |
| `repo create`, `repo enable` | Input `primary_branch`, optional `identity: {name, email}` and `ticket_slug_prefix`. Supplied identity is included in the explicit confirmation preview and persisted only in repository-local config. Create requires an unused/empty target. Missing effective identity returns recovery before the initialization commit; never invent or write a global identity. |
| `repo remove` | Remove registration only; preserve repository and drafts. Confirmation required. An unavailable repository can be selected by its exact stored root; unrelated unfinished work remains blocked. |
| `repo identity` | Inspect effective name/email and source. |
| `repo identity-set` | Input `name`, `email`, optional nullable `initials`; missing leaves initials unchanged, null clears, a string sets. Explicitly writes repository-local Git config, never global config. Confirmation required. |
| `remote list` | Names, redacted locations and publication selection; never echo embedded credentials. |
| `remote add` | Input `name`, `url`; add the named configuration, without implicit publication selection. |
| `remote remove` | Input `name`; preview tracking-reference/upstream effects and require confirmation. The publication remote is refused with `publication_remote_in_use`, so preview `publication_affected` is always false. |
| `remote select` | Input `name` (SSH only) or `null` to clear publication selection. |
| `key list`, `key show` | Non-secret registrations, selection, ownership and fingerprint; show takes an opaque key ID. |
| `key generate` | Input `label`, `protection` (`passphrase` or `none`); passphrase comes only from masked terminal interaction. |
| `key import` | Input `label`, invocation-relative `private_key_path`, optional `public_key_path`; register by reference. |
| `key select`, `key clear` | Explicit shared selection or clearing; never changes Git commit identity. |
| `key remove`, `key delete` | Unregister only, or separately delete owned generated files. Delete requires exact effect confirmation and cleared selection. Imported deletion is rejected. |
| `key public` | Return public-key text/fingerprint if available; never private contents. |
| `host list`, `host inspect` | Non-secret application-global pins; inspect takes `--authority`, never `--repo`. No network observation. |
| `host observe` | Required `--repo`, optional `--authority <host[:port]>` (bracketed IPv6, default port 22); observes configured SSH authorities without writing a pin. May unlock for authentication. Human output supplies the next approve/replace input object. |
| `host approve`, `host replace` | Input exact `authority`, `algorithm`, `fingerprint`; replacement also requires `old_fingerprint`. Explicit confirmation, transport re-observation and authentication with the selected key are required before storing trust. |
| `document list`, `document show` | Tree/list or complete item DTO, including source, metadata, observation and context. Show takes `--id` or `--path` with optional `--context`. |
| `folder list` | List indexed document folders under `docs/`; no scan or refresh. |
| `folder create` | Input repository-relative `path` under `docs/`; local empty folder until content is added. |
| `document create` | Input `item_id`, repository-relative `path`, `title`, `body`. Explicit IDs support safe automation retries. |
| `document save` | Input `observation`, optional `title`, `body`; preserve unknown metadata. No implicit publication. |
| `document move` | Input source `observation`, `destination_path`, `destination_observation`; preserve ID and checkpoint both paths. |
| `document repair`, `ticket repair` | Input exact `path`, `observation`, corrected complete `source`; preview and explicit confirmation. Validate identity/schema and use an owned authoring context, never patch primary as a shortcut. |
| `ticket list`, `ticket show` | Canonical metadata/body, context, conformity and observation. List includes closed tickets unless filtered. Show takes `--id` or `--path` with optional `--context`. |
| `path observe`, `ticket check` | Planned for C3: observations and relationship validation for repair/authoring flows. |
| `ticket create` | Input `item_id`, `title`, `type`, `status`, optional `project`, `team`, `body`. Closure metadata cannot be supplied; free-form status may be `closed` without making the ticket lifecycle-closed. |
| `ticket save` | Input `observation` and changed metadata/body. `project`/`team` can be cleared with null. Project status does not control closure; `ticket close` owns `closed_at`/`closed_by`. Reopening is outside MVP. |
| `comment list` | `--id` selects the parent item; return one flat, ordered list of roots and replies with parent IDs and depth, then visible malformed entries. Replies are not nested. |
| `comment add` | `--id` selects item; input `comment_id`, `body`, optional `parent_id`. One checkpoint then immediate configured sync; no-remote means local pending. |
| `item sync`, `repo sync` | Deliberate item-context or primary synchronization. Unsaved caller files are not implicitly submitted. |
| `document promote`, `ticket close` | Exact item effect preview and confirmed integration/publication/cleanup. CLI callers save drafts first; final domain checkpoint still handles required lifecycle metadata. |
| `index status`, `index refresh`, `index rebuild` | Inspect cache freshness, or local scan/rebuild. Never fetch or rewrite canonical state. Rebuild supports explicit root after cache loss. |
| `poll status`, `poll configure`, `poll pause`, `poll resume`, `poll once` | Durable policy and observed results; configure input `interval_seconds` in 60–3600. Once is explicit remote polling, not synchronization. No process discovery or worker control. |
| `operation list`, `operation show`, `operation resume` | Inspect/reconcile or deliberately resume one operation ID. List returns every stored operation that resume could act on or that still has work outstanding; show finds any stored operation, finished or not. Resume never infers permission for a different target/action. |
| `operation abandon` | Planned for C4: F2's confirmed abandonment of a pending local operation; closes its journal record without changing files. |
| `request show`, `request cancel` | Required `--request-id`, refuse `--repo`; inspect the request's state/result or request safe-point cancellation. Accepted local C2 requests report `requested: false`; this does not interrupt their process. |
| `conflict show`, `conflict resolve` | Select operation ID; inspect base/local/remote and expected path observations, then submit explicit resolutions. |
| `id new` | Generate and return a canonical ULID for caller-owned requests/items/comments; no repository mutation. |

### Registration removal targeting

Amended 2026-10-19 by defect ticket `01M4R19P943QZKHHKRXV2D2TP2`.
Amended 2026-10-20 by C2: `repo remove` now exposes this library mutation contract.

`repo remove` selects a registration before requiring a live Git repository.
When the root is missing or its directory no longer contains `.git`, callers
use the exact stored absolute root returned by `repo list` or a prior preview.
This includes the stored Windows canonical prefix. Live relative paths and
resolvable aliases retain normal canonicalization; a missing alias's former
target is not guessed. An interior directory or linked worktree does not select
its owner's registration for removal. Unknown missing paths retain
`invalid_path`; permissions, broken Git indirection and corrupt metadata are
errors, not evidence that a repository is absent.

The same explicit confirmation is required for an unmounted and a deleted
root. The preview identifies the stored root and registration time and lists
registration, index, operation-record and remote-state removal. Repository
files, Git state, recoverable drafts, shared keys and host trust are preserved.
Cached accessibility from `repo list` is not used to authorize removal.

A matching persisted request can replay after both the directory and the
registration have gone. An accepted request whose removal completed but whose
result was lost reconciles to `already_applied`; its own pending removal can
retry under the same operation ID. Command, scope, target and digest checks
still reject reuse of an ID for another request. Fresh execution compares the
previewed registration again under the deletion guard. Registration stamps
retain their existing ID/time semantics, including the separately documented
same-second replacement limitation in the
[implementation plan](../plans/2026-10-19-missing-repository-registration-removal-implementation.md).

The registry-only path preserves unrelated unfinished local/remote operations,
promotion/closure recovery and accepted requests. It returns `recovery_required`
for those blockers, or the existing retryable busy result while an active poll
is asked to yield. It never implicitly abandons that work. A live repository
continues to require its Git lease. A root or lease replacement detected while
acquiring guards produces a retryable busy result.

### List ordering

List ordering is deterministic: repositories by normalized root, documents by
relative path then ID, tickets by latest content-change time descending then ID,
and operations by start time then ID. Comment ordering remains schema-defined.
Ticket list supports exact `--status`, `--type` and `--project` filters, plus
`--closure open|closed|all` (default `all`). Closure filtering uses lifecycle
metadata, independently of free-form status text. Lists
return all matching results in v1; silent truncation is forbidden. Paging is a
future additive contract, not an undocumented default limit.

Repair cannot bypass identity uniqueness, a closed ticket's lifecycle fields,
or the prohibition on reopening. Assigning an ID to marker-only content is an
explicit adoption effect in the confirmation preview. Existing valid IDs remain
unchanged. A document folder target resolves in the selected item's context
when organizing an existing item, or in the documentation tree for an empty
folder; creation never implies publication or a placeholder-file commit.

The [ticket relationships and short codes RFC](ticket-relationships-and-short-codes.md#cli)
extends this taxonomy, as amended on 2026-10-07. It adds `ticket ready`,
`ticket blocked`, `ticket deps`, `ticket children`, `ticket cycles`,
`ticket plan`, `ticket critical-path`, `ticket find` and
`ticket slug-assign`, and optional relationship, initials and prefix inputs
on existing ticket, identity and repository verbs. `--id` still takes only a
canonical ULID; a short code is a search key and is rejected as an item ID.

## C2 mutation options and input schemas

Amended 2026-10-20 by C2. Selectors remain flags; all other mutation values
arrive in one `--input <file|->` JSON object. Every mutation declares
`--prepare` and `--confirm <ULID>`; unconfirmed verbs answer `not_confirmable`.
Request and confirmation IDs must be canonical ULIDs, as must `--key-id`.

| Verbs | Required | Refused |
| --- | --- | --- |
| `repo create`, `repo enable`, `repo identity-set`, `remote add`, `remote remove`, `remote select`, `host approve`, `host replace` | `--repo`, `--input` | — |
| `repo remove` | `--repo` | `--input` |
| `key generate`, `key import` | `--input` | `--repo` |
| `key select`, `key remove`, `key delete` | `--key-id` | `--repo`, `--input` |
| `key clear` | — | `--repo`, `--input` |
| `host observe` | `--repo` | `--input`, `--request-id`, `--prepare`, `--confirm` |
| `request show`, `request cancel` | `--request-id` | `--repo`, `--input`, `--prepare`, `--confirm` |
| Noninteractive/JSON mutation execution | `--request-id` | — |
| Every mutation | — | `--prepare` with either `--request-id` or `--confirm` |

Confirmed C2 mutations are repo create/enable/remove/identity-set, remote
remove, key delete and host approve/replace. All other C2 mutations are
unconfirmed. `--prepare` takes no request ID and allocates none.

Each input-taking verb publishes a closed schema under `schemas/input/v1/`,
named `<resource>_<verb>.schema.json` with hyphens replaced by underscores.
The shared `commit_identity.schema.json` is reached by `$ref`. Required fields
are listed below; optional does not mean nullable unless explicitly stated.

| Verb | Input fields |
| --- | --- |
| `repo create`, `repo enable` | Required string `primary_branch`; optional object `identity` with required strings `name`, `email`; optional string `ticket_slug_prefix`. |
| `repo identity-set` | Required strings `name`, `email`; optional string-or-null `initials` (unchanged/clear/set as above). |
| `remote add` | Required strings `name`, `url`. |
| `remote remove` | Required string `name`. |
| `remote select` | Required string-or-null `name`; null clears the selection. |
| `key generate` | Required string `label`; required `protection`: `passphrase` or `none`. |
| `key import` | Required strings `label`, `private_key_path`; optional string `public_key_path`. Both paths are invocation-relative. |
| `host approve` | Required strings `authority`, `algorithm`, `fingerprint`; authority uses the host selector's syntax. |
| `host replace` | Approve fields plus required string `old_fingerprint`. |

Input is read after service opening, once to EOF for stdin. A leading UTF-8
BOM is skipped. Unreadable/empty/non-UTF-8 input, malformed JSON, non-object
values, duplicate keys at any depth, unknown fields, missing required fields,
wrong types and invalid enum values are `invalid_input`, exit 2. Human input
diagnostics name the escaped field and problem, never its value. `passphrase`
is an unknown field; input provides no secret channel. Content validation is
the library's, not a second CLI policy.

Input compatibility is distinct from output compatibility: adding an optional
field or accepting a new enum value is non-breaking. Adding a required field,
removing/renaming a field, narrowing a type/enum or changing its meaning requires
a new input version. Input objects are closed, but only required fields occur
in `required`; the output all-properties rule does not apply to optional input.
Input `ticket_slug_prefix` maps to published preview field `slug_prefix`.

## Observations, request identity and retries

Show/inspect returns an opaque observation token representing the exact source,
destination and relevant context state. A token is an optimistic concurrency
precondition, not authentication. Writes require it for existing resources;
creation explicitly expects absence. Stale observations produce
`external_change`, preserving input and canonical state. There is no `--force`
override. Caller-provided source/body never enters operation diagnostics.

Every mutating execution has a request ULID. JSON/noninteractive mutation
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

Amended 2026-10-20 by C2: only `execute` is automatically retried, and only
when **(code is `busy` OR outcome is `partial`) AND the first recovery action
is `request.retry` naming this request**. At most three further attempts follow
100, 200 and 400 ms delays. Each uses the same command, target, input, request
ID and confirmation as the preceding attempt, with no repeated consent or
identity prompt. Stop at the first nonqualifying answer; return the last
envelope unchanged. Human retries emit `progress: retrying` to stderr.
Prepare, host observe and request show/cancel are never automatically retried;
neither is `transport_unavailable` merely because it offers a retry action.
One invocation keeps one credential session across its bounded retries.

Protected key generation is excluded from automatic retry because its secret
is moved into the first call. On a later invocation (R-19), an interactive
retry asks for the passphrase again even for a finished request; if files were
already written, the library registers them without using the new answer.
A noninteractive retry is CLI-built `unlock_required`, without a library
execute call. `request show` exposes the stored result of a finished request.

## Consent and noninteractive input

Terminal mode may prompt for missing identity, passphrase, host approval and
lifecycle consent. `--json`, `--non-interactive`, or absence of a controlling
terminal disables all prompts. A JSON command that needs a secret returns
`unlock_required`; it does not read stdin again or fall back to a key agent.
For protected keys, interactive one-shot commands unlock for that invocation.
No pipe/argument/environment secret channel or desktop unlock sharing is
included. There is no resident CLI credential session.

Amended 2026-10-20 by C2: interactive mode means human output, no
`--non-interactive`, and an available controlling terminal. All prompt I/O is
on that terminal device, never stdin/stdout/stderr, so piped `--input -` and
terminal prompts coexist. A new passphrase is masked and entered twice, with
at most three attempts; exhaustion is `invalid_input`.

For an interactive confirmed command without explicit `--prepare`/`--confirm`:
prepare, display the preview on the terminal, ask `Proceed? [y/N]`, then execute
with that confirmation only for `y`/`yes` (case-insensitive). A supplied request
ID whose record already exists goes directly to execute without re-preview or
consent; any lookup failure except `request_not_found` is returned unchanged.
Unconfirmed commands execute directly. Explicit prepare/confirm never prompt
for consent or identity; confirmed execution can still prompt to unlock.

Create/enable with no input identity may ask for name/email only after prepare
answers `identity_required`, then prepare again using the supplied identity.
**Owner clarification, 2026-10-22:** every later invocation of that request,
including an interactive retry, must provide the same identity in `--input`.
Existing-record retries never reconstruct or prompt for it; the library decides
replay or mismatch. Bounded retries within the original invocation reuse it.
The CLI's note explains this without printing identity values. Library preview
and result data retain their non-secret fields unchanged; CLI-written diagnostics
never echo input values (except the escaped non-secret label in a new-passphrase
prompt). Interactive execution without an ID allocates one and writes
`request: <ULID>` immediately to stderr before library work.

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
consent. Amended 2026-10-20 by C2 (F2 decision 6): trust is persisted only after
the connection re-observes the expected algorithm/fingerprint **and authenticates
with the selected key**. Merely seeing a host key does not store a pin.
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

Amended 2026-10-18 by C1: human successes write results to stdout; stale,
never-refreshed or incomplete-list notes go to stderr. Human failures write
their message, code and recovery actions to stderr, leaving stdout empty.
JSON always writes one compact envelope and newline to stdout, with empty
stderr. An output write/flush failure, including a broken pipe, exits 1
without a panic or further diagnostic.

Amended 2026-10-20 by C2: human progress stages go immediately to stderr once
per stage change; host observe writes `progress: connecting` before its call.
Failed writes of these early lines or the early request-ID note are ignored
so they cannot stop in-flight work (R-9). Prompt write failure cancels the
prompt. The final output write retains C1's exit-1 rule. JSON stderr stays empty.

Amended 2026-10-18 by C1: JSON help and version are success envelopes named
`cli help` and `cli version`, with empty scope and no requested effects.
Help data lists commands, availability, delivering Cycle, summary, repository
rule and options; planned commands have null repository and empty options.
Version data contains `name` and package `version`; `schema_version` remains
in the envelope. Usage errors name the recognized two-word command, or
`cli usage` when no catalog command can be named. The bare invocation is root
help and exits zero.

All envelope fields below are required; nullable fields use JSON null. Version 1
uses strings for IDs/OIDs/timestamps and never encodes an ID as a JSON number.

| Field | Type and allowed values |
| --- | --- |
| `schema_version` | Integer `1`. |
| `command` | Canonical two-word command string. |
| `request_id` | ULID string, or null when none applies, including prepare, ordinary reads/help and pre-dispatch failures; request selectors and post-open refusal retention follow C2 below. |
| `operation_id` | Domain operation ID string or null when no operation began; may be set on a refusal after acceptance (F2, amended 2026-10-20 by C2). |
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

A recovery `action` is a dotted name from a closed, published registry. An
action carries exactly its registered argument keys, or none when the result
names no repository. Reads use `index.rebuild`, `index.refresh` and
`repo.inspect`, each with the argument `root`. Mutation commands add theirs,
such as `operation.resume`, with the Cycle that delivers them.

Item DTOs include ID, kind, repository-relative path, required and optional
canonical metadata, unknown metadata, body, observation, context/provenance,
conformity problems and content-change timestamp. `show` additionally includes
complete canonical `source`; list omits bodies/source. Malformed entries use
nullable IDs/metadata plus exact path and problem code. Comment DTOs include
ID, item ID, parent ID, depth, author, created timestamp, body and conformity
problems. List `data` is an object with `items` and a boolean `complete`, as in
`{ "items": [...], "complete": true }`; it is never an unversioned top-level
array. `complete` is false when a directory holding the listed kind could not
be fully read, so entries may be missing. Lists read from the index also carry
its state. Read operations intentionally return requested source; mutation
outcomes, errors, polling logs and progress do not echo it.

`comment list` returns one flat list; replies are not nested (product owner,
2026-10-08). `items` is the depth-first flattening of the threads: each root
comment in `created_at` then ID order, immediately followed by its replies in
the same order, each followed by its own; then nonconforming entries in path
order. `depth` is 0 for a root comment and for a nonconforming entry. For
every entry `parent_id` is null exactly when `depth` is 0; otherwise it is the
`id` of the nearest earlier entry one level up. The consumer builds and renders
the tree. A nested chain's JSON depth would exceed default parser limits, and
changing the shape after the CLI ships would be breaking.

Other DTOs expose the table's named non-secret fields, plus problem codes. An
operation's failure code comes from its own closed registry, separate from the
envelope's result codes.
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

Amended 2026-10-20 by C2: the version 1 **output** JSON Schemas in `schemas/v1/`
are closed: every object they define sets `additionalProperties: false` and
requires all its properties, except four free-form objects: the envelope's
`data`, `unknown_metadata`, a recovery action's `arguments`, and request show's
stored `result.data` (F2's fourth object). Input schemas live separately and
allow the optional fields described above. The output compatibility rule is
(product owner, 2026-10-08):

- A schema describes exactly what the build that ships it produces.
- Adding a field, an enumeration value or a result or problem code is a
  non-breaking change made in place within version 1. The envelope's outcome
  and effect values are the exception: a client decides what happened from
  them, so any change to those two sets is breaking.
- Consumers MUST ignore fields they do not know and MUST tolerate codes and
  enumeration values they do not know. A client handles an unknown code
  through outcome and exit class, retaining the structured result rather than
  retrying blindly.
- Output from a newer build is therefore not guaranteed to validate against an
  older build's schema. Validate output only against the schemas of the build
  that produced it.
- Removing, renaming or retyping a field, or changing the meaning of an
  existing value, requires a new schema version. So does changing the outcome
  or effect values, or the meaning of a command input.

## Exit statuses and partial completion

Amended 2026-10-18 by C1: `usage_error` covers unknown commands/options,
missing or conflicting inputs and malformed option values. An approved verb
delivered by a later Cycle returns `command_unavailable`; human stderr names
that Cycle. Both are input failures (exit 2), with empty scope, null data,
no recovery actions and their fixed code messages in JSON.

### CLI-built refusals (R-6)

Amended 2026-10-20 by C2. For C2 the CLI builds refusals before execute or in
place of a library call; library envelopes pass through unchanged. The codes
are `usage_error`, `command_unavailable`, `invalid_path`, `invalid_input`,
`invalid_id`, `cancelled`, `unlock_required` and `internal_error`. They name
the parsed command (or `cli usage`), have null `operation_id` and `data`, empty
scope, no recovery actions, all effects `not_requested` and null `commit_oid`.
JSON `message` is the fixed result-code message, never input or backend text.

**Owner clarification, 2026-10-22:** every pre-dispatch error, including
usage/mode, transport startup and service-opening failures, keeps
`request_id: null` even if argv has a valid ID. IDs are parsed only after the
service opens. A post-open CLI refusal retains a valid parsed or
allocated-and-printed request ID; absent/malformed request IDs remain null.
Checks are request ID, confirmation ID, key ID, input, key-store lookup, then
prompts. Invalid IDs are `invalid_id`; malformed input or three exhausted
identity/new-passphrase attempts are `invalid_input`; protected generation
without a terminal is `unlock_required`; unavailable current-user key-store
location is `internal_error`. No new output code/schema is introduced.

Amended 2026-10-20 by C2: a wrong passphrase currently passes through as
`transport_unavailable`, exit 5, pending defect `01M5E6DYJ6GN7DMW4424TH3V78`
(decision 9). When this invocation supplied a passphrase, human stderr adds
`hint: if the passphrase was wrong, run the command again`. JSON is unchanged,
and no automatic retry sends the wrong answer again. `unlock_failed`, exit 3,
is demonstrated by a server rejecting the key after the right passphrase.

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

Amended 2026-10-20 by C2 (decision 6): before execute, declining consent,
Ctrl-C or EOF at a prompt is CLI-built `cancelled`, exit 130. Only the unused
prepare confirmation can remain, to expire. During a connection's unlock
prompt, Ctrl-C/EOF dismisses the prompt; the library returns `unlock_required`,
exit 3. C2 installs no process-wide Ctrl-C handler and has no safe point after
acceptance. Outside prompts, termination is interruption, not rollback. C4
owns safe-point Ctrl-C cancellation for synchronization.

The accepted local-write interruption window (decision 8) remains: create,
enable or remote select can stop between writing `.manyhands/config.toml` and
committing it, leaving a dirty-worktree refusal on retry. Remove the uncommitted
configuration for create/enable, or restore its committed content for remote
select, then retry the same request. C4 owns later operation abandonment.

## Alternatives, risks and required evidence

Amended 2026-10-18 by C1: the structured-input rationale below applies to
mutations; read inputs are flags.

Structured file/stdin mutation input is selected over large flag sets or an implicit
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
and no display or desktop dependency. Amended 2026-10-20 by C2: C1 reads and
C2's eighteen administration verbs now run through the executable. C2's empty,
existing-history and Unix generated-key onboarding journeys are in
`tests/cli_remote/journeys.rs`; exit, stdin/PTY and privacy evidence is in
`tests/cli/` and `tests/cli_remote/`. Authoring/synchronization and the remaining
PRD journeys remain obligations of later Cycles. Native CI runs after merge;
Windows prompting/key-file checks remain manual, not implied by Linux evidence.
