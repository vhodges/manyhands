# Manyhands skills

Agent Skills that let an AI coding agent drive `manyhands-cli`, the Manyhands command line, on behalf of a person: set up keys and repositories, find, write and close tickets and documents, comment, share work with collaborators, and recover when something goes wrong. Each directory here is one skill: a `SKILL.md` the agent loads when its description matches the task, plus the references it links to.

The skills are about using Manyhands. They contain no how-to for the desktop app and no guidance for developing Manyhands itself.

| Skill | What it does | When to use it |
| --- | --- | --- |
| `manyhands-overview` | Explains what Manyhands is and how `manyhands-cli` behaves: JSON input, request IDs, confirmations, observation tokens, error codes, and which actions the agent takes alone and which it hands to the person. | First, before any other Manyhands skill, and whenever you are unsure how a command behaves. |
| `manyhands-getting-started` | Takes a new person through setup in order: the CLI version, an SSH key, a repository, the remote, the host fingerprint and a first poll, checking each step and stopping wherever the person must choose or type a passphrase. | A person is new to Manyhands or asks you to set it up for a repository. |
| `manyhands-keys-and-hosts` | Creates, imports, selects and shows SSH keys, explains the passphrase choice, and observes, approves and replaces host fingerprints. | Setting up or choosing a key, giving its public key to a Git host, or when a command stops at a host fingerprint or an unlock. |
| `manyhands-repositories` | Creates or enables a repository, onboards a clone or an SSH URL, sets the remote, commit identity and ticket short-code prefix, and inspects or removes a repository. | Starting with a new, existing or cloned repository, or when a repository is not registered or not enabled. |
| `manyhands-finding-work` | Reads before acting: lists and filters tickets and documents, shows what is ready or blocked and what depends on what, finds a ticket by short code, and checks whether the index is current. | Finding out what exists or what to do next, before creating or changing anything. |
| `manyhands-documents` | Creates folders and documents, reads and saves them with observation tokens, recovers from a stale save, and moves, repairs and promotes documents. | Writing, editing or promoting a document, or when a document command answers `external_change` or `occupied_path`. |
| `manyhands-tickets` | Creates, reads and saves tickets, sets status, type, dependencies, parent and short code, repairs a ticket file, and closes a ticket after showing the person what the close will do. | Creating, updating or closing a ticket, or when a ticket command answers `external_change`, `ticket_closed` or `relationship_cycle`. |
| `manyhands-comments` | Reads an item's comments, adds a root comment or a reply, sets who it is by, and explains that adding a comment saves it on the item's branch and publishes that branch. | Discussing a ticket or a document, or when a comment command answers `ticket_closed` or `item_not_found`. |
| `manyhands-sharing-and-sync` | Shares an item with `item sync` and main with `repo sync`, receives others' work, shows both sides of a conflict and applies the one the person picks, and republishes a deleted branch. | Making work visible to collaborators or receiving theirs, or when a sync answers `recovery_required`, `remote_branch_deleted` or `busy`. |
| `manyhands-polling-and-index` | Fetches what collaborators published with `poll once`, explains what a poll changes and leaves alone, retires completed local contexts after showing the person, sets the polling interval, and refreshes or rebuilds the index. | A list or status looks out of date, you need others' work, or a poll or index answer reports a problem. |
| `manyhands-recovery` | Finds why a command answered `recovery_required` or `busy`, names the unfinished operation holding the work, and says which steps are the agent's and which go to the person. | A command says recovery is required, an operation is listed as unfinished, writes are refused for no visible reason, or `history_unknown` appears. |

Minimum CLI version: 0.2.0

Install `manyhands-cli` itself first: take it from the GitHub Releases page, https://github.com/vhodges/manyhands/releases, and follow "Installation and startup" in the `README.md` inside the release archive. This README covers installing the skills only.

The table is ordered by use: the foundation, setup, working with items, sharing and upkeep, and what to do when something goes wrong. The version is a minimum, not an exact match: a newer `manyhands-cli` is fine. Check yours with `manyhands-cli --version`.

### Which skill when

- New to Manyhands, or setting it up: `manyhands-getting-started`.
- Any other task: `manyhands-overview` first (every task skill says so), then the skill for the task.
- Something is stuck or refused: `manyhands-recovery`.

### The safety model every skill shares

The agent works alone on reads and on routine work: creating and saving tickets, documents and comments, and sharing saved work (`item sync`, `repo sync`, and a comment, which saves and publishes its branch). It shows the person first and waits for their OK before it closes a ticket, promotes a document, republishes a branch, trusts a host (`host approve`, `host replace`), creates or imports a key, or creates or enables a repository. It hands over to the person any passphrase step, removing a key or a repository, and the choices about a conflict, an abandoned operation or lost history. The single statement of which action is which is [autonomy.md](manyhands-overview/references/autonomy.md); the other skills link to it rather than repeat it.

## Install

A skill is a directory. Install the whole set, never single skills: the task skills link to shared material in `manyhands-overview` with relative paths such as `../manyhands-overview/references/autonomy.md`, and those links work only when all eleven `manyhands-*` directories sit side by side in the same parent directory. A plain copy keeps them that way. Claude Code does not document symlinked skill directories; one may work, but verify it before relying on it.

Pick the target for your agent runtime. Each runtime reads skills from the places in this table (names current as of this release; check the runtime's own documentation if they have changed):

| Runtime | For one project | For one person, in every project |
| --- | --- | --- |
| Claude Code | `<project>/.claude/skills/` | `~/.claude/skills/` |
| Codex | `<project>/.agents/skills/` | `~/.agents/skills/` |

Run the copy from the directory that contains `skills/` (the repository root, or the root of the unpacked release archive's folder). On Linux and macOS, for example for one person in Claude Code:

```sh
mkdir -p ~/.claude/skills
cp -R skills/manyhands-* ~/.claude/skills/
```

On Windows, in PowerShell, from the same place (the target here is for one person in Claude Code):

```powershell
New-Item -ItemType Directory -Force "$HOME\.claude\skills"
Copy-Item -Recurse skills\manyhands-* "$HOME\.claude\skills\"
```

For a project, use `<project>/.claude/skills/` (or `.agents/skills/`) as the target instead. If the project is itself a Manyhands repository, do one of two things after the copy, or prefer the per-user install: commit the copied directory, or list it in the repository's `.git/info/exclude` (for example the line `.claude/`). Otherwise the main checkout has an untracked `.claude/`, and `ticket close --prepare` and `document promote --prepare` answer `recovery_required` with the blocker `primary_dirty` until it is committed or excluded (verified). Claude Code picks up new or changed skills during a session, and `/reload-skills` forces it; for other runtimes, check their documentation or start a new session.

### Check that it worked

List the installed directories and expect eleven, each holding a `SKILL.md` (use `~/.agents/skills/` or the project paths for the other targets; other skills may also live there, so list only ours):

```sh
ls -d ~/.claude/skills/manyhands-*
```

Then ask the agent which Manyhands skills it has, or ask it to set up Manyhands for a repository and watch it open `manyhands-overview` first.

### Update

Replace the whole set with the one from the new release. Remove the old `manyhands-*` directories first so a skill that was renamed or dropped does not linger, then copy again:

```sh
rm -rf ~/.claude/skills/manyhands-*
cp -R skills/manyhands-* ~/.claude/skills/
```

Do not mix versions: skills from one release describe the CLI of that release. Edits made to an installed copy are lost on update; keep your own changes elsewhere.

### Where the skills are in a release

- The Linux archive, the Windows zip and the macOS zip each hold the set in `skills/`, at the root of the unpacked archive's folder (the archive wraps everything in one top-level `manyhands-v<version>-<target>/` folder).
- On macOS the same directory is inside the app, at `Manyhands.app/Contents/Resources/skills`.
- The Windows installer puts it in `<install dir>\skills`. The installer owns that tree: it removes or replaces files there on upgrade, so copy the set elsewhere (as above) before editing anything.
- A Homebrew cask installs the app and links the CLI. It does not link the skills anywhere; take them from the release archive, from the app's `Contents/Resources/skills`, or from the repository.
- The Nix packages install the binaries and the licence documents only, not the skills; take them from the release archive or from a checkout of the repository.

A Claude Code plugin is a different way to package skills and is out of scope here.
