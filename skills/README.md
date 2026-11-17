# Manyhands skills

Agent Skills for using Manyhands through its command line, `manyhands-cli`. Each directory here is one skill: a `SKILL.md` an agent loads when its description matches the task, plus the references it links to. The skills teach how to use Manyhands, not how to build it.

| Skill | What it does | When to use it |
| --- | --- | --- |
| `manyhands-overview` | Explains what Manyhands is, how `manyhands-cli` takes JSON input, request IDs, confirmations and observation tokens, what its errors mean, and which actions an agent takes alone and which it hands to the person. | First, before any other Manyhands skill, and whenever you are unsure how a command behaves. |
| `manyhands-getting-started` | Walks a new person through setup in order: the CLI version, an SSH key, a repository (new, existing, cloned or from an SSH URL), the remote, the host fingerprint and a first poll, with a check after each step and a stop wherever the person must choose, confirm or type a passphrase. | New to Manyhands, or asked to "set up Manyhands" for a repository. |
| `manyhands-keys-and-hosts` | Generates, imports, selects and shows SSH keys, explains the passphrase choice, and observes, approves and replaces host fingerprints. | Setting up or choosing a key, giving its public key to a Git host, or when a command stops at a host fingerprint or an unlock. |
| `manyhands-repositories` | Creates or enables a repository, onboards a clone or an SSH URL, sets the remote, commit identity and ticket short-code prefix, and inspects or removes a repository. | Starting with a new, existing or cloned repository, or when a repository is not enabled. |
| `manyhands-documents` | Creates folders and documents, reads and saves them with observation tokens, recovers from a stale save, moves, repairs and promotes documents. | Writing, editing or promoting a document, or when a document command answers `external_change` or `occupied_path`. |
| `manyhands-finding-work` | Reads before acting: lists and filters tickets and documents, what is ready or blocked, dependencies, short codes, folders, comments, and whether the index is current. | Finding out what exists or what to do next, before creating or changing anything. |
| `manyhands-tickets` | Creates, reads and saves tickets with observation tokens, sets status, type, dependencies and parent, assigns short codes, repairs a ticket file, and closes a ticket after showing the person what it will do. | Creating, updating or closing a ticket, or when a ticket command answers `external_change`, `ticket_closed`, `relationship_cycle`, `recovery_required` or `unlock_required`. |
| `manyhands-comments` | Reads an item's comments, adds a root comment or a reply, sets who it is by, and explains that adding a comment saves it on the item's branch and then publishes that branch. | Discussing a ticket or a document, or when a comment command answers `ticket_closed`, `item_not_found`, `recovery_required` or `unlock_required`. |
| `manyhands-sharing-and-sync` | Shares an item with `item sync`, main with `repo sync`, and receives others' work. Shows both sides of a conflict and applies the side the person picks. Republishes a deleted branch. Brings main into one item locally. | Making work visible to collaborators or receiving theirs, or when a sync answers `recovery_required`, `remote_branch_deleted`, `publication_remote_required`, `unlock_required` or `busy`. |
| `manyhands-polling-and-index` | Runs `poll once` to fetch what collaborators published, explains what a poll changes and leaves alone, retires completed local contexts after showing the person, sets the polling interval and pause, and refreshes or rebuilds the index. | A list or status looks out of date, you need others' work, or an index or polling answer reports a problem. |
| `manyhands-recovery` | Finds why a command answered `recovery_required`, `busy` or an incomplete result with `operation list`, `operation show` and `request show`, says which unfinished operation holds which work, and which steps are the agent's and which go to the person (resume, abandon, lost-history repair). | A command says recovery is required, an operation is listed as unfinished, writes are refused for no visible reason, or `history_unknown` appears. |

Minimum CLI version: 0.2.0

## Install

Install instructions follow once the set is complete.
