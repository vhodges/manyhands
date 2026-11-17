# Manyhands skills

Agent Skills for using Manyhands through its command line, `manyhands-cli`. Each directory here is one skill: a `SKILL.md` an agent loads when its description matches the task, plus the references it links to. The skills teach how to use Manyhands, not how to build it.

| Skill | What it does | When to use it |
| --- | --- | --- |
| `manyhands-overview` | Explains what Manyhands is, how `manyhands-cli` takes JSON input, request IDs, confirmations and observation tokens, what its errors mean, and which actions an agent takes alone and which it hands to the person. | First, before any other Manyhands skill, and whenever you are unsure how a command behaves. |
| `manyhands-keys-and-hosts` | Generates, imports, selects and shows SSH keys, explains the passphrase choice, and observes, approves and replaces host fingerprints. | Setting up or choosing a key, giving its public key to a Git host, or when a command stops at a host fingerprint or an unlock. |
| `manyhands-repositories` | Creates or enables a repository, onboards a clone or an SSH URL, sets the remote, commit identity and ticket short-code prefix, and inspects or removes a repository. | Starting with a new, existing or cloned repository, or when a repository is not enabled. |
| `manyhands-documents` | Creates folders and documents, reads and saves them with observation tokens, recovers from a stale save, moves, repairs and promotes documents. | Writing, editing or promoting a document, or when a document command answers `external_change` or `occupied_path`. |
| `manyhands-finding-work` | Reads before acting: lists and filters tickets and documents, what is ready or blocked, dependencies, short codes, folders, comments, and whether the index is current. | Finding out what exists or what to do next, before creating or changing anything. |

Minimum CLI version: 0.2.0

## Install

Install instructions follow once the set is complete.
