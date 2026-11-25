---
name: manyhands-repositories
description: "Use when creating a Manyhands repository, enabling Manyhands in an existing or cloned repository, onboarding an SSH URL or clone the person gives you, setting the publication remote, commit identity or ticket slug prefix, inspecting or removing a repository, or a command answers repository_not_registered, repository_not_enabled or identity_required."
---

# Repositories

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id`, prepare and confirm, and the main checkout root are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). Keys and host trust are in [keys and hosts](../manyhands-keys-and-hosts/SKILL.md); this skill says when to use them.

Every verb here except `repo list` takes `--repo <repo-root>`, the repository's main checkout root, the path on the first line of `git worktree list` (a linked worktree is not it). There is no "current repository": the CLI never guesses from where you stand. Keep your `--input` files in a temporary directory outside the repository, so they are not committed, and delete them when done.

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

`repo inspect` works on a repository that is not registered: `data.registered` is false, `data.configuration.publication_remote` is the selected remote or null, `data.configuration.integration` is `direct` or `pull_request` (null unless the configuration is valid) and `data.configuration.forge` is `{kind, web_url, pull_request_url}` or null when there is no `[forge]` table (on a build with the pull-request setting, Wave 06 Cycle 01; `repo list` items carry the same two fields, and human `repo inspect` prints `Integration:` and, with a forge, `Forge:`), `data.identity_state` is `available` or `required`, `data.remotes` lists the remotes, and on a build with the concurrent-writers change `data.operations` lists the unfinished operations (see [recovery](../manyhands-recovery/SKILL.md)). `repo list` lists only registered repositories (`data.items[].root`, `accessibility`, `configuration`, `problem_count`). Neither shows an SSH key: the selected key is global, see [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

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

The prepare's `data.effects` lists what will happen (`directory_created`, `repository_initialized`, `identity_written`, `worktree_exclusion_written`, `configuration_written`, `initialization_committed`, `repository_registered`) and `data.details.identity.written_locally` says whether the identity will be stored. Show it in words.

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

Who runs it is in [autonomy](../manyhands-overview/references/autonomy.md).

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

`remote remove` is confirmed ([autonomy](../manyhands-overview/references/autonomy.md)). A selected remote cannot be removed (`publication_remote_in_use`): select null first, which stops publication, so ask the person.

```json schema=remote_remove
{
  "name": "origin"
}
```

```sh
manyhands-cli --json --non-interactive remote remove --repo <repo-root> --input remove.json --prepare
```

After the person's OK, confirm with `--confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>` as for the other confirmed verbs.

## Pull-request mode

Not usable end to end yet. This describes the setting only, and pull-request mode is not usable end to end until close and promote support it: until then `ticket close` and `document promote` in a pull-request repository still merge into the primary and push it, as in direct mode, and the pull-request workflow that replaces that arrives in later Cycles of Wave 06. Do not put a repository people use into this mode yet, and do not suggest it. Only a build with the pull-request setting (Wave 06 Cycle 01) knows these keys; `v0.2.1` and earlier refuse a version 2 configuration, which is deliberate.

`.manyhands/config.toml` holds two more keys. `integration` is `"direct"` or `"pull_request"`, spelled exactly; a missing key means direct, and any other value makes the configuration invalid (`invalid_configuration`). An optional `[forge]` table names the Git host: `kind` is one of `bitbucket-cloud`, `bitbucket-data-center`, `github` or `gitlab`, `web_url` and `pull_request_url` are absolute `https://` or `http://` URLs, and `pull_request_url` is a template that contains `{source}` and may contain `{target}`. A `[forge]` table needs a `kind` or a `pull_request_url`.

`integration = "pull_request"` needs `format_version = 2` and a `publication_remote`. Version 2 is what makes a build without the setting refuse the repository instead of pushing its primary. A version 2 file in direct mode is accepted and written back as version 1, and any other version is invalid. `repo inspect` shows the configured setting, not a forge recognised from the remote.

In a pull-request repository `repo sync` only pulls: it fast-forwards the primary and never merges or pushes it ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)). That decides how a repository gets there. The configuration that switches the mode must already be on the remote's primary: committed and pushed while the repository was still direct, or merged through the forge. A local commit of the switch is refused by the next `repo sync` as `primary_has_local_commits` ([recovery](../manyhands-recovery/SKILL.md)), so never switch a checkout by committing the change on its primary and then syncing. No Manyhands command writes these keys yet, and you never edit `.manyhands/config.toml` yourself: the switch is the person's decision and their Git work, and your part is to say what is needed (from the contract, not run here).

## The order for a repository that will share

1. A key exists (`key generate` or `key import`) and is selected (`key select`), and its public key is on the Git host ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)).
2. The repository is created or enabled (`repo create` or `repo enable`).
3. The remote is added and selected.
4. Host trust: `host observe`, the person checks the fingerprint, `host approve`. These need steps 1 to 3: `host observe` answers `no_selected_key` without a key, `publication_remote_required` without a selected remote and `repository_not_registered` for a repository that is not enabled.
5. `poll once` (you run it alone; it answers `publication_remote_required` only when no remote is selected). Until it has run, the repository has no record of the remote's primary branch, and `ticket close --prepare` and `document promote --prepare` answer `recovery_required` with `data.blockers` `["primary_missing"]` (verified). A passphrase key makes the poll the person's: [polling and the index](../manyhands-polling-and-index/SKILL.md).

A repository with no remote works fully on its own; sharing waits (`publication_remote_required`).

## Onboarding a clone

A clone of a repository that already uses Manyhands has `.manyhands/config.toml`, but is not registered on this machine and has no identity in its local Git configuration. After `repo enable` the identity is still `required`: enabling a repository that already has a Manyhands configuration does not write the identity you supply (verified, even though `repo enable` accepted it). So:

1. `repo enable` (prepare, show, wait, confirm). Include `identity` in the input, as the prepare refuses without it.
2. `repo identity` shows `Source: none`. Run `repo identity-set` (prepare, show, wait, confirm) with the same name and email the person already gave for `repo enable`; ask again only if you never had them.
3. `remote list`. If the config already selects a remote, `repo inspect` shows `Publication remote: <name>`. If not, `remote select`.
4. Host trust, if it will share: [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

## Onboarding from an SSH URL

Use when the person gives only a URL, such as `ssh://git@git.example.com/org/repo.git`. Run these in order and stop where it says. The reasons and the failures are in [onboarding from an SSH URL](references/onboarding-from-an-ssh-url.md); the sequence was run end to end against a throwaway `sshd` on a high port.

Git's host check is not Manyhands' host pin, and you never skip either. Steps 3 and 4 always come before any hand-off of the clone to the person, whether or not the key has a passphrase.

1. Ask the person where to clone. If that directory already exists and is not empty, `git clone` refuses it (`destination path ... already exists and is not an empty directory`): ask the person for another path, and never empty or delete their directory yourself. An existing empty directory is fine, and `git clone` creates the destination folder (and any missing parents) itself. Ask which key to use if more than one is registered (`key list`). If none exists, or the person says one exists and `key list` is empty, go to [keys and hosts](../manyhands-keys-and-hosts/SKILL.md) first. Select it: `manyhands-cli --json --non-interactive key select --key-id <KEY_ID> --request-id <REQUEST_ID>`. Selecting changes the key for every repository: say so if another key was selected. Get `<private_key_path>` from `data.private_key_path` of `key show`.
2. Ask whether that key has a passphrase if you did not generate it yourself with `none` (`key show` does not say). Note it: a passphrase key changes only who runs step 5 and step 8, not the order.
3. **The public key must already be on the Git host, by the person's hand.** Show `manyhands-cli key public --key-id <KEY_ID>` and wait. Without it the clone fails with `Permission denied (publickey)`.
4. **Stop at the host fingerprint.** Git must trust the host before it can clone, but plain `git clone` with no terminal fails with `Host key verification failed`. Never use `-o StrictHostKeyChecking=no` or `accept-new`: that trusts whatever answers. Instead read the host's key without trusting it (a network connection to the real host) into a temporary directory outside any repository, and show the person the fingerprint:

   ```sh
   mkdir <temp-dir>
   ssh-keyscan -p <port> <host> > <temp-dir>/scanned.txt
   ssh-keygen -lf <temp-dir>/scanned.txt
   ```

   Omit `-p <port>` for port 22. `ssh-keyscan` is read-only and trusts nothing, so it may run as soon as the key exists, even before the person has added the key to the host (the order above still holds: the person has the key on the host before you scan). If the host does not resolve, `ssh-keyscan` prints `getaddrinfo <host>: Name or service not known`, exits 1 and writes nothing (verified): tell the person, there is nothing to show. Ask the person to confirm each fingerprint against a source they trust (the Git host's published list), and wait. Keep the scanned fingerprint for step 8.
5. Only after their OK, clone with that exact key, trusting only the scanned host key. If the key has no passphrase, run:

   ```sh
   GIT_SSH_COMMAND='ssh -i <private_key_path> -o IdentitiesOnly=yes -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=<temp-dir>/scanned.txt' git clone <url> <destination>
   ```

   **If the key has a passphrase, hand the clone to the person's own terminal** and give them this command, which differs only in leaving out `-o BatchMode=yes`. `BatchMode` forbids the passphrase prompt even at a terminal (verified: the clone fails with `Permission denied (publickey)` and no prompt; without it, it prompts and clones):

   ```sh
   GIT_SSH_COMMAND='ssh -i <private_key_path> -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=<temp-dir>/scanned.txt' git clone <url> <destination>
   ```

   Wait until they say it is done, then continue at step 6. Delete `<temp-dir>` afterwards. The environment variable form is for a POSIX shell. In PowerShell set `$env:GIT_SSH_COMMAND` first; that form was not run here.
6. `repo enable` on `<destination>` (include the `identity` the person gives), then `repo identity-set` if `repo identity` shows `Source: none`, as in "Onboarding a clone".
7. Make sure a remote is selected (`repo inspect`, then `remote select` if `Publication remote` is empty).
8. **Stop at Manyhands' host pin.** `host observe --repo <destination>` (a network call; a host that is not pinned yet is not authenticated, so it works even with a passphrase key). Compare `presented_fingerprint` with what the person accepted at step 4, show it, and wait for their OK. Then `host approve` with `--prepare`, show the preview, and `--confirm`. With a passphrase key `--confirm` answers `unlock_required`: give the person the same command at their terminal, with the same confirmation ID and request ID. Details and the input are in [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

   ```sh
   manyhands-cli host approve --repo <destination> --input approve.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
   ```
9. Check: `host inspect --authority <host:port>` shows the pin, and `repo inspect --repo <destination>` shows `registered: true`, a publication remote and `identity_state: available`. Sync and sharing: [sharing and sync](../manyhands-sharing-and-sync/SKILL.md).

If you cannot do a step, say which and why, and give the person the exact command. Never edit `.git/config`, `known_hosts` or `.manyhands/config.toml` by hand to get past a step.

## Remove a repository: hand-over

`repo remove` unregisters a repository from Manyhands: it drops the registration, the index and the operation records, and leaves the repository's files and Git history alone. It is a hand-over verb ([autonomy](../manyhands-overview/references/autonomy.md)): you may run `--prepare` to tell the person exactly what it will do (`data.effects`: `registration_removed`, `index_dropped`, `operation_records_dropped`, `remote_state_dropped`; and `details.files_changed` and `details.git_state_changed`, both false), and you never confirm it. Give them the command to run.

```sh
manyhands-cli --json --non-interactive repo remove --repo <repo-root> --prepare
```

## Codes you will meet

| Code (exit) | Meaning and next step |
| --- | --- |
| `identity_required` (3) | The repository has no commit identity. Ask the person; supply `identity` on `repo enable` or `repo create`, or run `repo identity-set` once registered. |
| `repository_not_registered` (3) | This data directory does not know the repository (never enabled, or enabled under another data directory). `repo enable --prepare`, show, wait. [Recovery](../manyhands-recovery/SKILL.md), "Not registered"; `index rebuild`, which `recovery` may suggest, is not the first answer. |
| `repository_not_enabled` (2) | A write to a repository that was never enabled. `repo enable`. |
| `not_repository` (2) | No Git repository at the path. `repo create` for a new one. |
| `occupied_path` (2) | `repo create` where a Git repository already is. Use `repo enable`. |
| `wrong_branch` (3) | The repository is not on `primary_branch`. Ask the person which branch is primary. |
| `invalid_remote` (2) | A remote that cannot be selected (not SSH). |
| `publication_remote_in_use` (3) | The remote to remove is selected. Select null first, after asking. |
| `publication_remote_required` (3) | Sharing or host trust needs a selected remote. |
