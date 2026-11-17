---
name: manyhands-repositories
description: "Use when creating a Manyhands repository, enabling Manyhands in an existing or cloned repository, onboarding an SSH URL or clone the person gives you, setting the publication remote, commit identity or ticket slug prefix, or inspecting or removing a repository."
---

# Repositories

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id`, prepare and confirm, and the main checkout root are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). Keys and host trust are in `manyhands-keys-and-hosts`; this skill says when to use them.

Every verb here takes `--repo <repo-root>`, the repository's main checkout root, the directory that holds `.git`. There is no "current repository": the CLI never guesses from where you stand.

## Which situation is it

| The person has | Do |
| --- | --- |
| Nothing yet, or a new folder | `repo create` |
| A Git repository that is not Manyhands-enabled | `repo enable` |
| A clone of a repository that already uses Manyhands (it has `.manyhands/config.toml`) | `repo enable`, then check the identity: see "Onboarding a clone" |
| Only an SSH URL | "Onboarding from an SSH URL" |
| A repository they believe is enabled | `repo inspect`, then `repo list` |

## Look first

```sh
manyhands-cli --json repo inspect --repo <repo-root>
manyhands-cli --json repo list
manyhands-cli --json repo identity --repo <repo-root>
manyhands-cli --json remote list --repo <repo-root>
```

`repo inspect` works on a repository that is not registered: `data.registered` is false, `data.configuration.publication_remote` is the selected remote or null, `data.identity_state` is `available` or `required`, `data.remotes` lists the remotes. `repo list` lists only registered repositories (`data.items[].root`, `accessibility`, `configuration`, `problem_count`). Neither shows an SSH key: the selected key is global, see `manyhands-keys-and-hosts`.

## `repo create` and `repo enable`

Both are confirmed verbs: prepare, show the person the effects, wait, then confirm.

- `repo create` makes the directory if it is missing, runs `git init` in it, writes the Manyhands configuration, commits "Initialize Manyhands", and registers it. It refuses a path that is already a Git repository (`occupied_path`, exit 2).
- `repo enable` takes a Git repository that exists (`not_repository` otherwise) and does the same without the init. The repository must be on `primary_branch`, else `wrong_branch` (exit 3). If it already has `.manyhands/config.toml` (a clone), no "Initialize" commit is made: the effects are only registration and the worktree exclusion.
- Enabling an already enabled repository with the same input is `already_applied` (exit 0). With a different `ticket_slug_prefix` it answers `invalid_input` (exit 2). Check `repo inspect` first rather than guessing.

| Verb | Required | Optional |
| --- | --- | --- |
| `repo create`, `repo enable` | `primary_branch` (string, the branch name, usually `main`) | `identity` (object with required `name` and `email`, both strings, nothing else), `ticket_slug_prefix` (string) |

Ask the person for the name and email: they become the commit author of every Manyhands commit in this repository. Do not invent them and do not copy them from a global Git configuration; Manyhands ignores the global Git identity (verified: with a global `user.name`, `repo enable` without `identity` answered `identity_required`, exit 3, and its recovery action `repo.identity_set` cannot run before the repository is registered).

```json schema=repo_enable
{
  "primary_branch": "main",
  "identity": { "name": "<person name>", "email": "<person email>" },
  "ticket_slug_prefix": "<short-prefix>"
}
```

```sh
manyhands-cli --json --non-interactive repo enable --repo <repo-root> --input enable.json --prepare
manyhands-cli --json --non-interactive repo enable --repo <repo-root> --input enable.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

`repo create` takes the same input; the example is the same shape:

```json schema=repo_create
{
  "primary_branch": "main",
  "identity": { "name": "<person name>", "email": "<person email>" },
  "ticket_slug_prefix": "<short-prefix>"
}
```

```sh
manyhands-cli --json --non-interactive repo create --repo <new-repo-root> --input create.json --prepare
manyhands-cli --json --non-interactive repo create --repo <new-repo-root> --input create.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

The prepare's `data.effects` lists what will happen (`directory_created`, `repository_initialized`, `identity_written`, `configuration_written`, `initialization_committed`, `repository_registered`) and `data.details.identity.written_locally` says whether the identity will be stored. Show it in words.

### Commit identity

The identity is stored in the repository's local Git configuration (`user.name`, `user.email`); it never changes the person's global Git configuration or their SSH key. Read it with `repo identity` (`Source: repository` when set, `none` when missing). `ticket_slug_prefix` is written to `.manyhands/config.toml`; ticket short codes are then `<prefix>-<initials>-<random>`, for example `mh-rp-as7dk` (the tickets skill covers short codes). Initials come from the name unless set.

`repo identity-set` changes it later. It is confirmed, so prepare, show the person (`details.current` and `details.effective` against the new name and email), wait, confirm.

| Verb | Required | Optional |
| --- | --- | --- |
| `repo identity-set` | `name`, `email` (strings) | `initials` (string sets them; null clears them back to derived; omit to leave as they are) |

```json schema=repo_identity_set
{
  "name": "<person name>",
  "email": "<person email>",
  "initials": "<initials>"
}
```

```sh
manyhands-cli --json --non-interactive repo identity-set --repo <repo-root> --input identity.json --prepare
manyhands-cli --json --non-interactive repo identity-set --repo <repo-root> --input identity.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

`repo identity-set` is not in the autonomy table's list; it is confirmed like the other repository verbs here, so treat it like `repo enable`: show the preview, wait for the OK.

## Remotes

A remote is where shared work goes. Manyhands uses one selected SSH remote, the publication remote. Adding a remote does not select it. Selecting writes `publication_remote` into `.manyhands/config.toml` and commits "Configure Manyhands publication remote" on the primary branch, so a clone of the repository later inherits the selection. Only SSH remotes (`ssh://` and `git@host:path`) can be selected: an `https://` remote answers `invalid_remote` (exit 2).

| Verb | Required | Optional |
| --- | --- | --- |
| `remote add` | `name`, `url` (strings) | none |
| `remote select` | `name` (string, or null to clear the selection) | none |
| `remote remove` | `name` (string) | none |

```json schema=remote_add
{
  "name": "origin",
  "url": "<ssh-url>"
}
```

```sh
manyhands-cli --json --non-interactive remote add --repo <repo-root> --input remote.json --request-id <REQUEST_ID>
```

```json schema=remote_select
{
  "name": "origin"
}
```

```sh
manyhands-cli --json --non-interactive remote select --repo <repo-root> --input select.json --request-id <REQUEST_ID>
```

`remote add` and `remote select` you run alone. `remote list` shows `selected_for_publication` and `publication_eligible`; the URL shown has no credentials. Adding the same remote again is `already_applied`.

`remote remove` is confirmed and not in the autonomy table. Prepare, show the person, wait. A selected remote cannot be removed (`publication_remote_in_use`): select null first, which stops publication, so ask the person.

```json schema=remote_remove
{
  "name": "origin"
}
```

```sh
manyhands-cli --json --non-interactive remote remove --repo <repo-root> --input remove.json --prepare
```

Confirm with `--confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>` as for the other confirmed verbs.

## The order for a repository that will share

1. A key exists and is selected, and its public key is on the Git host (`manyhands-keys-and-hosts`).
2. The repository is created or enabled.
3. The remote is added and selected.
4. Host trust: `host observe`, the person checks the fingerprint, `host approve`. These need steps 1 to 3: `host observe` answers `no_selected_key` without a key, `publication_remote_required` without a selected remote and `repository_not_registered` for a repository that is not enabled.

A repository with no remote works fully on its own; sharing waits (`publication_remote_required`).

## Onboarding a clone

A clone of a repository that already uses Manyhands has `.manyhands/config.toml`, but is not registered on this machine and has no identity in its local Git configuration. After `repo enable` the identity is still `required`: enabling a repository that already has a Manyhands configuration does not write the identity you supply (verified, even though `repo enable` accepted it). So:

1. `repo enable` (prepare, show, wait, confirm). Include `identity` in the input, as the prepare refuses without it.
2. `repo identity` shows `Source: none`. Ask the person for the commit name and email, then `repo identity-set` as above.
3. `remote list`. If the config already selects a remote, `repo inspect` shows `Publication remote: <name>`. If not, `remote select`.
4. Host trust, if it will share: `manyhands-keys-and-hosts`.

## Onboarding from an SSH URL

Use when the person gives only a URL, such as `ssh://git@git.example.com/org/repo.git`. Run these in order and stop where it says. The reasons and the failures are in [onboarding from an SSH URL](references/onboarding-from-an-ssh-url.md); the sequence was run end to end against a throwaway `sshd` on a high port.

Stop and hand over at each marked step. Git's host check is not Manyhands' host pin, and you never skip either.

1. Ask the person where to clone, and which key to use if more than one is registered (`key list`). If none exists, `manyhands-keys-and-hosts` first. Select it: `manyhands-cli --json --non-interactive key select --key-id <KEY_ID> --request-id <REQUEST_ID>`. Selecting changes the key for every repository: say so if another key was selected.
2. **Stop if the key is passphrase-protected.** `key show` does not say, so ask the person if you did not generate the key yourself with `none`. `ssh` will prompt for the passphrase and you cannot answer; without a terminal it fails as `Permission denied (publickey)`. Give the person the clone command (step 4) to run in their own terminal, then continue from step 5 when they say it is done.
3. **The public key must already be on the Git host, by the person's hand.** Show `manyhands-cli key public --key-id <KEY_ID>` and wait. Without it the clone fails with `Permission denied (publickey)`.
4. **Stop at the host fingerprint.** Git must trust the host before it can clone, but plain `git clone` with no terminal fails with `Host key verification failed`. Never use `-o StrictHostKeyChecking=no` or `accept-new`: that trusts whatever answers. Instead read the host's key without trusting it (this is a network connection to the real host) and show the person the fingerprint:

   ```sh
   ssh-keyscan -p <port> <host> > scanned.txt
   ssh-keygen -lf scanned.txt
   ```

   Omit `-p <port>` for port 22. Ask the person to confirm each fingerprint against a source they trust (the Git host's published list), and wait.
5. Only after their OK, clone with that exact key and only the scanned host key trusted. `<private_key_path>` is `data.private_key_path` from `key show`:

   ```sh
   GIT_SSH_COMMAND='ssh -i <private_key_path> -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=<path>/scanned.txt' git clone <url> <destination>
   ```

   The environment variable form is for a POSIX shell. In PowerShell set `$env:GIT_SSH_COMMAND` first; that form was not run here.
6. `repo enable` on `<destination>`, then the identity check, as in "Onboarding a clone".
7. Make sure a remote is selected (`repo inspect`, then `remote select` if `Publication remote` is empty).
8. **Stop at Manyhands' host pin.** `host observe --repo <destination>` (a network call), compare `presented_fingerprint` with what the person approved at step 4, show it, wait for their OK, then `host approve` with `--prepare`, show the preview, `--confirm`. Details are in `manyhands-keys-and-hosts`.
9. Check: `host inspect --authority <host:port>` shows the pin, and `repo inspect --repo <destination>` shows `registered: true`, a publication remote and `identity_state: available`. Sync and sharing: `manyhands-sharing-and-sync`.

If you cannot do a step, say which and why, and give the person the exact command. Never edit `.git/config`, `known_hosts` or `.manyhands/config.toml` by hand to get past a step.

## Remove a repository: hand-over

`repo remove` unregisters a repository from Manyhands: it drops the registration, the index and the operation records, and leaves the repository's files and Git history alone. It is a hand-over verb. You may run `--prepare`, which changes nothing, to tell the person exactly what it will do (`data.effects`, and `details.files_changed` and `details.git_state_changed`, both false). Then give them the command and let them run it.

```sh
manyhands-cli --json --non-interactive repo remove --repo <repo-root> --prepare
```

## Codes you will meet

| Code (exit) | Meaning and next step |
| --- | --- |
| `identity_required` (3) | The repository has no commit identity. Ask the person; supply `identity` on `repo enable` or `repo create`, or run `repo identity-set` once registered. |
| `repository_not_registered` (3) | Not enabled here. `repo enable`. Its `recovery` suggests `index.rebuild`; that is not the fix. |
| `repository_not_enabled` (2) | A write to a repository that was never enabled. `repo enable`. |
| `not_repository` (2) | No Git repository at the path. `repo create` for a new one. |
| `occupied_path` (2) | `repo create` where a Git repository already is. Use `repo enable`. |
| `wrong_branch` (3) | The repository is not on `primary_branch`. Ask the person which branch is primary. |
| `invalid_remote` (2) | A remote that cannot be selected (not SSH). |
| `publication_remote_in_use` (3) | The remote to remove is selected. Select null first, after asking. |
| `publication_remote_required` (3) | Sharing or host trust needs a selected remote. |
