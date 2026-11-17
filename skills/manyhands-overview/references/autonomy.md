# What an agent does alone, and what it hands to the person

The owner set this policy on 2026-11-17. Every other skill defers to it and does not restate it.

| Agent acts alone | Agent shows the person what it will do, then waits for their OK | Agent hands to the person |
| --- | --- | --- |
| Reads. Creating and saving documents and tickets, `ticket slug-assign`, comments, `item sync`, `repo sync`, `context sync-local`, `poll once`, `index refresh` and `rebuild`, `key select` and `clear`, `remote add` and `select`. | `host approve` and `replace` (showing the fingerprint), `key generate` and `import`, `repo create`, `enable` and `identity-set`, `remote remove`, `document promote`, `document repair`, `ticket repair`, `ticket close`, `item republish`. Where a verb has `--prepare`, the agent shows the preview before `--confirm`. | `key delete`, `key remove`, `repo remove`. `operation abandon`, `item resolve-history`. `conflict resolve` and `context resolve-local`, unless the person has said which side wins. Any passphrase step. |

## Showing the person a prepared preview

Run the verb with `--prepare`. It changes nothing and answers with a confirmation ID, an expiry (about ten minutes) and the effects it will have. Show the person the preview in plain words: what will be created, merged, removed or trusted, and for `host approve` and `replace` the fingerprint itself, copied from the answer and never summarized. Ask for their OK and wait. Only then run the same command with `--confirm <confirmation-id>` and a fresh `--request-id`. If the person says no, or the expiry passes, drop the confirmation and prepare again if they change their mind.

Only some verbs are confirmable. Any other verb answers `--prepare` with `not_confirmable` (exit 2) and has no preview. Of the verbs in the middle column, `key generate` and `key import` are such verbs: describe the action in your own words and wait for the person's OK before you run it. Every other verb follows its own column of the table, whether or not it has a preview: `key select` and `remote add` you run alone, and `key remove`, `key delete` and `repo remove` you hand to the person and never run to completion yourself, even after an OK.

For a hand-over verb that has a preview (`key delete`, `repo remove`, and the others in the third column that accept `--prepare`), you may run `--prepare` so you can show the person what the command would do, and you never run `--confirm` or the verb itself. A prepare changes no key, repository, file or Git state; it only records a pending confirmation in Manyhands' data directory (verified by comparing the key list, the repository list, the key files and the repository's Git state before and after). Give the person the exact command to run. `--prepare` never executes, so trying it on an unfamiliar verb is safe. The mechanics of prepare and confirm are in [CLI conventions](cli-conventions.md).

When a step is handed to the person, say which command, why you cannot run it (a passphrase prompt needs their terminal, or the action is destructive), and what you will check afterwards.
