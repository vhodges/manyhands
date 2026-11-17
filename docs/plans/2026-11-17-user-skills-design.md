# Distributable user skills design

Status: approved by the owner, 2026-11-17, with the version check changed to a
minimum version. Reviewed by a builder check; the owner answered its two open
questions (autonomy policy, key guidance).

## Intent

Manyhands has agent skills for developing Manyhands (`.agents/skills/`), but
none for *using* it. An agent given a Manyhands repository has to learn the CLI
from `--help` and the CLI contract RFC, and the CLI has conventions an agent
cannot guess: one JSON object on `--input`, a `--request-id` on every
noninteractive write, `--prepare` then `--confirm` on the confirmed verbs,
`observation` tokens from a prior read, and passphrase-protected keys that
cannot unlock noninteractively.

This work adds a set of skills, written for agents that drive `manyhands-cli`
on behalf of a person, that ship in this repository and in every release
archive. Their first job is to get a new user from nothing to a working setup
(key, host trust, repository). Their second is to cover the item lifecycle
(documents, tickets, comments, sharing) and the recovery paths.

## Goals

- An agent that has the skills installed can create, edit, save, share,
  promote and close items, comment, enroll keys, onboard a repository, poll and
  rebuild the index, using only the skills and the CLI.
- A new user's agent can run one setup skill that walks the keys, hosts and
  repository skills in order.
- A user can find out what skills exist, what each does and when to use it,
  and how to install them for their agent runtime.
- The skills match the shipped CLI. Drift is caught by a test, not by a user.

## Non-goals

- No `manyhands-cli skills install` verb. Installation is copying a directory,
  documented.
- No `repo clone` verb. The skills document the working sequence for an SSH
  URL today (see Repositories). A clone verb is a separate ticket.
- No desktop how-to. The skills name where the desktop app does something the
  CLI cannot, and stop there.
- No changes to the developer skills in `.agents/skills/`.
- No uploading keys to Git forges (a PRD non-goal).

## Anti-goals

A skill set that restates `--help`, tells an agent to edit `.manyhands/` files
by hand or skip `--prepare`/`--confirm`, asks an agent to handle a passphrase,
or documents a command shape no one ran is a failure even if it reads well.

## Constraints

- **Format:** the open Agent Skills format: a directory per skill holding
  `SKILL.md` with `name` (equal to the directory name) and `description`
  frontmatter, and optional `references/`.
- **Layout:** `skills/` at the repository root, one directory per skill, plus
  `skills/README.md`. The set installs as one whole directory, because task
  skills link to the foundation skill by relative path
  (`../manyhands-overview/references/…`).
- **Release:** `scripts/release/package.py` stages the payload for four
  targets. `skills/` must reach every archive, and the macOS app's
  `Resources`. The Windows installer keeps a managed-file list and
  `validate_payload` checks payload contents, so both need the new tree.
- **Testing budget (AGENTS.md):** one non-SSH check, extending an existing
  target, and a release-payload check in `scripts/release/tests`. No SSH case.
  No sleeps.
- **Content:** Manyhands content (tickets, documents, comments) is one line per
  paragraph, never hard-wrapped, so the skills say so.
- **Dogfooding:** the work gets one Manyhands ticket before its implementation
  plan, per AGENTS.md.

## Approaches considered

- **A. Many focused skills plus a shared foundation skill. Chosen.** Narrow
  descriptions trigger precisely; the shared concepts live once. Cost: the set
  installs as a unit.
- **B. A few broad, self-contained skills.** Installable one at a time, but
  long, with the shared concepts copied and drifting, and unlike the list the
  owner gave.
- **C. One router skill with a large references tree.** One install unit, but
  one vague description triggers worse than specific ones and gives no focused
  entry points.

The owner chose A on 2026-11-17, with the format, clone-verb and accuracy
calls below.

## The skills

Eleven skills. Names are the directory names.

| Skill | Use when | Covers |
| --- | --- | --- |
| `manyhands-overview` | Always first; any Manyhands task | What Manyhands is: local-first, Git-backed, items (documents and tickets), per-item branches and worktrees, checkpoint on save, the registry and data directory. CLI conventions: `--json`, `--non-interactive`, `--repo`, JSON `--input`, `--request-id`, `--prepare`/`--confirm`, observation tokens, exit codes, error envelope and recovery codes, the passphrase limit, when to stop and ask the person. Shared references that the others link to. |
| `manyhands-getting-started` | A new user, or "set up Manyhands" | The ordered setup path wrapping the keys, repositories and polling skills, with checks (`key list`, `repo inspect`, `poll status`) after each step. |
| `manyhands-keys-and-hosts` | Enrolling or choosing an SSH key; host trust prompts | `key generate`, `import`, `select`, `clear`, `remove`, `delete`, `public`; passphrase versus none; giving the public key to the Git host (by hand; not automated); `host observe`, `approve`, `replace`; shared key registry; the selected key (global for now, not per repository; owner, 2026-11-17). |
| `manyhands-repositories` | Creating a repository, onboarding a clone, setting remotes | `repo create`, `enable`, `remove`, `inspect`, `identity-set`; `remote add`, `select`, `remove`; commit identity; ticket slug prefix. Onboarding from an SSH URL: clone with Git using the selected key, `host observe` and `approve`, `repo enable`. |
| `manyhands-finding-work` | Looking before acting | `ticket list`, `show`, `check`, dependency and ready/blocked views, `document list`, `show`, `folder list`, `comment` reads, filters, JSON shapes, reading `status`. |
| `manyhands-documents` | Creating, editing, saving, sharing or promoting a document | `folder create`, `document create`, `save`, `move`, `repair`; observation and save patches; sharing (points to sync); `document promote` and its confirmation; one-line paragraphs. |
| `manyhands-tickets` | Creating, updating status, sharing or closing a ticket | `ticket create`, `save`, `slug-assign`, `repair`; free-form `type` and `status`; `deps` and `parent`; `ticket check`; `ticket close` and its confirmation, and why saving status `closed` is not closure. |
| `manyhands-comments` | Discussing an item | `comment add`, threading with `parent_id`, `created_by`, that it publishes the item's branch, reading comments. |
| `manyhands-sharing-and-sync` | Making work visible to collaborators, or receiving theirs | `item sync`, `repo sync`, `item republish`, `conflict show` and `resolve`, local-context sync, what is published when, per-item operation. |
| `manyhands-polling-and-index` | Stale lists, "is it up to date", scheduled refresh | `poll once`, `configure`, `pause`, `resume`, `poll status`, `index status`, `refresh`, `rebuild`, `--retire-completed`, what polling will and will not touch. |
| `manyhands-recovery` | A command reported recovery, a blocked operation or a stuck item | `operation list`, `show`, `resume`, `abandon`, `request show`, `cancel`, `remote history` and the history repair verbs, `context retire`, closure cleanup stopped by ignored files, reading `recovery_required`. |

Each task skill follows one shape: when to use, preconditions, the command
sequence with a worked JSON example, how to read the result, the failure codes
that matter and what to do, and when to hand back to the person.

## Documentation

`skills/README.md` is the catalog: what the set is, a table of every skill
with its purpose and when to use it (the table above, in plain words), the
recommended starting points ("new here: getting-started; doing work:
overview, then the task skill"), and install instructions.

Install instructions cover copying or symlinking the whole `skills/`
directory into a project (`.claude/skills/` for Claude Code, `.agents/skills/`
for Codex-style runtimes) or a user directory (`~/.claude/skills/`), plus
updating on a new release and what is shipped where in the archive. The main
`README.md` and `packaging/README-release.md` each get a short pointer.

## Accuracy and checks

- **Written from runs.** Each skill's commands are run against a scratch
  repository with `MANYHANDS_DATA_DIR` set to a temp directory, so no real
  registry is touched. Examples are paste-ready JSON checked against
  `schemas/input/v1/` and `tests/fixtures/cli_input_v1/`.
- **Drift test (non-SSH).** One table-driven test, in an existing CLI target,
  checks for every skill: `name` equals its directory, `description` is
  present and starts with "Use when", every relative link resolves, every
  `manyhands-cli <resource> <verb>` in a code block is in the command
  catalog, and every skill appears in `skills/README.md`.
- **Release check.** A test in `scripts/release/tests` that a staged payload
  contains `skills/` for each target, including the macOS `Resources` copy.
- **Behavior check.** Per `superpowers:writing-skills`, each skill is tested by
  giving a fresh agent a realistic task with and without the skill and fixing
  what it gets wrong. This is the evidence the content works; it is not part
  of the suite.

## Known gotchas the skills must cover

Taken from the project's own findings, to be re-verified while writing, not
copied: `comment add` pushes the target item's branch and merges main in first
if main moved; a ticket worktree holding ignored files (`target/`, `.devenv/`)
leaves `ticket close` cleanup pending; `history_unknown` has repair verbs; a
passphrase-protected key makes `ticket close` and sync answer
`unlock_required` for a noninteractive agent, who must hand the step to the
person at their terminal; repository operations are per item (a branch and a
worktree each); one open investigation reports `recovery_required` while an
extra ad hoc worktree exists.

## Left to the builder

Skill wording and descriptions; how many reference files each skill gets;
which worked examples to include; exact install paths per runtime (verify
Codex's and Claude Code's current skill directories); whether the Windows
installer's managed-file list needs a policy change or only an added path;
where in the macOS bundle the copy lives; the name and placement of the drift
test.

## Agent autonomy (owner decision, 2026-11-17)

`manyhands-overview` carries this policy and the other skills defer to it.

| Agent acts alone | Agent shows the person what it will do, then waits for their OK | Agent hands to the person |
| --- | --- | --- |
| Reads. Creating and saving documents and tickets, `ticket slug-assign`, comments, `item sync`, `repo sync`, `context sync-local`, `poll once`, `index refresh` and `rebuild`, `key select` and `clear`, `remote add` and `select`, `request cancel` (only for your own blocked request: cancelling is final, and a new request redoes the work). | `host approve` and `replace` (showing the fingerprint), `key generate` and `import`, `repo create`, `enable` and `identity-set`, `remote remove`, `document promote`, `document repair`, `ticket repair`, `ticket close`, `item republish`. Where a verb has `--prepare`, the agent shows the preview before `--confirm`. | `key delete`, `key remove`, `repo remove`. `operation abandon`, `item resolve-history`. `conflict resolve` and `context resolve-local`, unless the person has said which side wins. Any passphrase step. |

## Key guidance (owner decision, 2026-11-17)

Setup explains both options and lets the person choose. A passphrase-less key
lets the agent work unattended but is weaker if the key file leaks. A
passphrase key is stronger, but sync and `ticket close` answer
`unlock_required` to a noninteractive agent, so the person runs those steps at
their terminal. The skill recommends a passphrase-less key for a key used only
with Manyhands, and says why.

## Onboarding from an SSH URL

Manyhands generates keys at `~/.ssh/manyhands/<key-id>`
([authentication RFC](../RFC/authentication-and-credential-handling.md)) and
never uses ssh-agent or default keys, and `git` does not use the selected key
on its own. The documented sequence (corrected 2026-11-17 by the Task 5 run
against a throwaway `sshd`; ruling R10) is:

1. `key select`: the key (generated or imported) is selected, and the person
   adds its public key to the Git host by hand.
2. The agent reads the host's fingerprint with `ssh-keyscan` (a network read
   that trusts nothing), shows it, and the person accepts it.
3. Clone with Git pointing at the key and trusting only the scanned key, in a
   file used for this one clone:
   `GIT_SSH_COMMAND='ssh -i <key-file> -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=<scanned>' git clone <url>`.
   Plain `git clone` with no terminal fails with `Host key verification failed`,
   and `accept-new` would trust whatever answers.
4. `repo enable` on the clone, then `repo identity-set` (enabling an already
   initialized clone does not write the supplied identity).
5. `remote add` and `remote select` if the clone's configuration selects no
   remote.
6. `host observe` and the person's OK on the fingerprint, then `host approve`
   for Manyhands' own host pin (Git's `known_hosts` is separate).

The order differs from the first draft (key, host pin, repository) because on
a clone that is not yet registered, `host observe`, `remote list` and `remote
select` answer `repository_not_registered`: Manyhands' host pin needs an
enabled repository with a selected remote. A passphrase-protected key makes
`ssh` prompt (and `host observe` of a pinned host, `host approve --confirm`,
sync and `ticket close` answer `unlock_required`), which an agent cannot
answer; the skill hands those steps to the person. An imported key lives where
the person put it. The sequence was run end to end against a local `sshd` and
documents the gaps (no real forge, no Windows or macOS run).

## Compatibility with the installed CLI

The skills ship in the same archive as the CLI they were written against, and
`skills/README.md` and `manyhands-overview` record the minimum CLI version they
need (owner decision, 2026-11-17: a minimum, not an exact match; the owner
expects little disruptive CLI change and will update the skills if there is
some). The overview tells the agent to run `manyhands-cli --version` first and
to say so when it is older than that minimum, then trust `--help` and the JSON
schemas over the skill. A newer CLI is not a warning. The drift test also validates each documented example JSON
against `schemas/input/v1/`.

## Calls made without the owner

- Skill set and names above (eleven; `finding-work` and `recovery` added to
  the owner's list).
- `skills/` as the directory, `skills/README.md` as the catalog.
- The drift test's checks and the release test.
- Whether to write `skills/` content for desktop users: no.
- The fresh-agent behavior check is required before review-ready: at least two
  realistic tasks per skill, run by subagents of the model family in use, with
  a no-skill baseline only for the tricky skills (keys-and-hosts, tickets
  close, recovery). Results are recorded in the ticket, not in the suite.
- The version stamp, the `--version` check and the schema validation of
  examples (Compatibility section).
- The SSH-URL onboarding sequence (previous section).
