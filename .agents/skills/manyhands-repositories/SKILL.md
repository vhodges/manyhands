---
name: manyhands-repositories
description: "Use when creating a Manyhands repository, enabling Manyhands in an existing or cloned repository, onboarding an SSH URL or clone the person gives you, setting the publication remote, commit identity or ticket slug prefix, inspecting or removing a repository, putting a repository setup on a branch for a pull request, changing its mode or remote that way, cancelling a pending setup, or a command answers repository_not_registered, repository_not_enabled, identity_required or setup_pending."
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
| A repository whose setup must reach the main branch through a pull request (and the person asked for that) | `repo enable` with `integration` `pull_request`: "Enable in pull-request mode" |
| A repository that shows a `data.setup` in `repo inspect`, or a write answering `setup_pending` | "A pending setup" and "Cancel a setup" |
| A repository they believe is enabled | `repo inspect`, then `repo list` |

## Look first

```sh
manyhands-cli --json repo inspect --repo <repo-root>
manyhands-cli --json repo list
manyhands-cli --json repo identity --repo <repo-root>
manyhands-cli --json remote list --repo <repo-root>
```

`repo inspect` works on a repository that is not registered: `data.registered` is false, `data.configuration.publication_remote` is the selected remote or null, `data.configuration.integration` is `direct` or `pull_request` (null unless the configuration is valid) and `data.configuration.forge` is `{kind, web_url, pull_request_url}` or null when there is no `[forge]` table (on a build with the pull-request setting, Wave 06 Cycle 01; `repo list` items carry the same two fields, and human `repo inspect` prints `Integration:` and, with a forge, `Forge:`), `data.identity_state` is `available` or `required`, `data.remotes` lists the remotes, and on a build with the concurrent-writers change `data.operations` lists the unfinished operations (see [recovery](../manyhands-recovery/SKILL.md)). `repo list` lists only registered repositories (`data.items[].root`, `accessibility`, `configuration`, `problem_count`). On a build with onboarding on a setup branch (Wave 06 Cycle 02) both also carry `setup`: null, or the pending setup `{id, kind, state, request_id, branch, remote, integration, pull_request}`, where `kind` is `enable` or `change`, `state` is `recorded` until the remote holds the setup's branch and then `published`, and `pull_request` is `{source, target, forge, url}` (`repo list` reads the index only, so its `url` can be null where `repo inspect`'s is not, and a setup that has merged can still show there until `index refresh`). A setup record that cannot be read gives `setup` null with the problem `setup_unreadable` in `data.configuration.problems`, as `operations_unreadable` does for `operations`: tell the person, and `repo remove` clears it. Human `repo inspect` adds `Setup:` and the link's line. Neither shows an SSH key: the selected key is global, see [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

## `repo create` and `repo enable`

Both are confirmed verbs: prepare, show the person the effects, wait, then confirm.

- `repo create` makes the directory if it is missing, runs `git init` in it, writes the Manyhands configuration, commits "Initialize Manyhands", and registers it. It refuses a path that is already a Git repository (`occupied_path`, exit 2).
- `repo enable` takes a Git repository that exists (`not_repository` otherwise) and does the same without the init. The repository must be on `primary_branch`, else `wrong_branch` (exit 3). If it already has `.manyhands/config.toml` (a clone), no "Initialize" commit is made: the effects are only registration and the worktree exclusion.
- Enabling an already enabled repository with the same input is `already_applied` (exit 0). With a different `ticket_slug_prefix` it answers `invalid_input` (exit 2). Check `repo inspect` first rather than guessing.

| Verb | Required | Optional |
| --- | --- | --- |
| `repo create` | `primary_branch` (string, the branch name, usually `main`) | `identity` (object with required `name` and `email`, both strings, nothing else), `ticket_slug_prefix` (string) |
| `repo enable` | `primary_branch` | the `repo create` fields, and on a build with onboarding on a setup branch `publication_remote` (string), `integration` (`direct` or `pull_request`) and `forge` (object of optional `kind`, `web_url`, `pull_request_url`); see "Enable in pull-request mode" |

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
| `remote select` | `name` (string, or null to clear the selection) | `integration` (`direct` or `pull_request`), `forge` (object, or null to remove it); see "Changing the setting through a setup branch" |
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

Not usable end to end yet. This describes the setting only, and pull-request mode is not usable end to end until close and promote support it: until then `ticket close` and `document promote` in a pull-request repository still merge into the primary and push it, as in direct mode, and the pull-request workflow that replaces that arrives in later Cycles of Wave 06. Do not put a repository people use into this mode yet, and do not suggest it. Only a build with the pull-request setting (Wave 06 Cycle 01) knows these keys; `v0.2.1` and earlier refuse a version 2 configuration, which is deliberate. A build with onboarding on a setup branch (Wave 06 Cycle 02) can set them up, through a branch for a pull request: the sections below.

`.manyhands/config.toml` holds two more keys. `integration` is `"direct"` or `"pull_request"`, spelled exactly; a missing key means direct, and any other value makes the configuration invalid (`invalid_configuration`). An optional `[forge]` table names the Git host: `kind` is one of `bitbucket-cloud`, `bitbucket-data-center`, `github` or `gitlab`, `web_url` and `pull_request_url` are absolute `https://` or `http://` URLs, and `pull_request_url` is a template that contains `{source}` and may contain `{target}`. A `[forge]` table needs a `kind` or a `pull_request_url`.

`integration = "pull_request"` needs `format_version = 2` and a `publication_remote`. Version 2 is what makes a build without the setting refuse the repository instead of pushing its primary. A version 2 file in direct mode is accepted and written back as version 1, and any other version is invalid. `repo inspect` shows the configured setting, not a forge recognised from the remote.

In a pull-request repository `repo sync` only pulls: it fast-forwards the primary and never merges or pushes it ([sharing and sync](../manyhands-sharing-and-sync/SKILL.md)). That decides how a repository gets there. The configuration that switches the mode must already be on the remote's primary: committed and pushed while the repository was still direct, or merged through the forge. A local commit of the switch is refused by the next `repo sync` as `primary_has_local_commits` ([recovery](../manyhands-recovery/SKILL.md)), so never switch a checkout by committing the change on its primary and then syncing. On a build with onboarding on a setup branch, Manyhands writes the keys for you, on a branch for a pull request, and never on the main branch: the sections below. You never edit `.manyhands/config.toml` yourself, and the switch is the person's decision.

### Enable in pull-request mode

Only on a build with onboarding on a setup branch (Wave 06 Cycle 02), and only when the person asks for a pull-request repository: ask first, because it changes how the whole team shares work, and say that pull-request mode is not usable end to end until close and promote support it (above). Then `repo enable` puts the repository's setup on a branch for a pull request, instead of committing it to the main branch. It is show-then-wait ([autonomy](../manyhands-overview/references/autonomy.md)). Have the remote added (`remote add`; a remote that is not SSH answers `invalid_remote`), a key selected and its public key on the host ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)), the main branch checked out and clean, and an identity.

```json schema=repo_enable
{
  "primary_branch": "main",
  "identity": { "name": "<person name>", "email": "<person email>" },
  "publication_remote": "origin",
  "integration": "pull_request"
}
```

`integration` is `direct` or `pull_request`. Unless the person asked for pull-request mode, pass `"integration": "direct"` with a `publication_remote`: left out, the mode the host suggests can be pull request. The prepare names the mode, so check it before you show it. Left out, a given `publication_remote` makes the mode the one its host suggests (pull request for a host Manyhands recognises, or for one you describe with `forge`; otherwise direct), and no remote makes it direct. `forge` is `{kind, web_url, pull_request_url}`, each optional, for a host Manyhands does not recognise. A pull-request configuration with no remote is `invalid_input`. A repository that already has a configuration (a clone) is only registered, and a field that differs from the configuration is `invalid_input`: the shared configuration decides the mode.

```sh
manyhands-cli --json --non-interactive repo enable --repo <repo-root> --input enable.json --prepare
manyhands-cli --json --non-interactive repo enable --repo <repo-root> --input enable.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

The prepare names the mode, the remote and the forge, and its effects are `repository_registered`, `setup_recorded`, `setup_branch_committed` and `setup_branch_published`; it contacts nothing. Show it and wait. Confirming registers the repository at once as setup pending, with no configuration and nothing committed to the main branch, then fetches the remote's main branch, commits the configuration on top of it on a branch `manyhands/setup/<setup id>` and pushes that branch alone, never the main branch. The answer is `ok` with `data.type` `setup`: `data.state` `published`, `data.branch`, and `data.pull_request` (`source`, `target`, `forge`, and a `url` that is null when the forge needs the remote's URL). Tell the person to open the pull request from `data.branch` into the main branch, with the link when there is one, and that until it merges the repository is registered but not yet set up ("A pending setup" below).

A stop after the setup is recorded is exit 4, `outcome` `partial`, with the stop's code and the same `data` with `state` `recorded`: `no_selected_key`, `key_unavailable`, `key_rejected`, `unlock_required`, `unlock_failed`, `host_approval_required`, `host_replacement_required`, `transport_unavailable`, `remote_unavailable`, `busy`, `identity_required` or `recovery_required`. Nothing is lost. Deal with the cause (a key or the host: [keys and hosts](../manyhands-keys-and-hosts/SKILL.md), where `host observe` and `host approve` work on the pending setup's remote; a passphrase is the person's at their terminal), then send the same command with the same `--confirm` and `--request-id` again. It continues from the record, registers nothing twice and pushes the branch once. Never start a second `repo enable` with a new request ID to get past a stop: while the first is pending it answers `setup_pending` (or `busy` first, while the first still runs). Ctrl-C ends the process and leaves the request accepted, the same way.

Three answers end the setup, and the same request sent again only replays them: `remote_primary_missing` (exit 3: the remote has no main branch to set up from; the person pushes one first), `remote_already_set_up` (exit 3: the remote's main branch already holds a Manyhands configuration; update the checkout from the remote, then `repo enable` again with no pull-request fields, which registers it as a clone) and `primary_has_local_commits` (exit 3: the local main branch has commits the remote's lacks, listed in `data.local_commits`; the way out is the person's Git work, [recovery](../manyhands-recovery/SKILL.md), "Local commits on the primary of a pull-request repository"). A first-time setup that ended this way leaves the repository unregistered; once the cause is fixed, start again with a new request ID.

### Changing the setting through a setup branch

`remote select` also takes `integration` and `forge`. Plain, with only `name`, it is unchanged and yours to run alone: a direct repository that stays direct commits the selection on the main branch as before (a `forge` change goes in the same commit). With `integration` or `forge` it can publish a branch and it blocks close and promote, so describe the change in words and wait for the person's OK first (it is unconfirmed, so `--prepare` answers `not_confirmable` and there is no preview). `forge` is an object as above, null to remove the `[forge]` table, and left out to keep it.

```json schema=remote_select
{
  "name": "origin",
  "integration": "pull_request",
  "forge": { "kind": "github" }
}
```

A direct repository that turns pull-request mode on, naming any SSH remote (a first one or a different one), and a pull-request repository that keeps its remote and changes `integration` or `forge`, go through a setup branch, as enable does, with `data.kind` `change`: the answer is `ok` with `data.type` `setup`, the main branch, its configuration and the checkout do not change, and the new setting takes effect when the pull request merges and a poll brings it in. A selection that changes nothing is `already_applied`. Clearing the remote of a pull-request configuration is `invalid_configuration`. A pull-request repository naming another remote is `invalid_input`: "Routes by hand" below. The stops and the refusals are enable's, with these differences: `remote_already_set_up` does not apply, a refused change leaves the repository enabled as it was, and `external_change` means the remote's main branch no longer holds the configuration the change started from (for the same remote, `poll once` first and try again; for another remote, "Routes by hand"). A close or promotion that is unfinished or accepted answers `recovery_required` and starts no change: finish it first.

### A pending setup

While `repo inspect` shows a `data.setup`, the setup is pending: `recorded` means it is not on the remote yet (a stop above: send that request again), `published` means it waits for the merge. The person merges the pull request (a merge commit, a squash and a fast-forward all work), and then you run `poll once` alone ([polling and the index](../manyhands-polling-and-index/SKILL.md), "A pending setup's poll"), which brings the main branch forward and finishes the setup. Do not run Git to move the main branch yourself. Meanwhile a first-time setup refuses every write that needs the configuration with `setup_pending` (exit 3, nothing written), and a pending change refuses only `ticket close`, `document promote`, `remote select`, `remote remove` and `repo enable`, while every other write carries on in the current mode on the current remote: [recovery](../manyhands-recovery/SKILL.md), "A pending setup". `host observe` and `host approve` use the pending setup's remote when the configuration has none, so the host can be trusted before the merge.

Do not run an older build on a repository with a pending setup. An older build sharing the data directory sees it as registered without a configuration, its `repo enable` would commit "Initialize Manyhands" on the main branch, its `repo remove` refuses the setup's recorded step as unrecognized evidence, and its re-entry of the setup's request deletes the request. Every teammate also needs a build with the pull-request setting (Wave 06 Cycle 01) before the setup merges, because the configuration is version 2 and an older build refuses it.

Two clones of the repository can each open a setup, so two pull requests can exist. Whichever merges first completes both clones' setups, and the other pull request and its remote branch are left: tell the person to decline it. Any move of the main branch's configuration completes a pending change, another clone's merged change or an edit made outside Manyhands included, and `poll once` does not say so: `repo inspect` shows `setup` null.

### Cancel a setup

`repo setup-cancel` ends a pending setup of either kind, and is show-then-wait: run `--prepare`, show the person the setup (`details`), its branch and that the branch will be deleted from the remote (the effects `setup_branch_deleted_remotely`, `setup_branch_deleted`, `setup_record_removed` and, for a first-time setup, `repository_unregistered`), and wait. It takes `--repo` only, no input.

```sh
manyhands-cli --json --non-interactive repo setup-cancel --repo <repo-root> --prepare
manyhands-cli --json --non-interactive repo setup-cancel --repo <repo-root> --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

It deletes the remote setup branch only while it is still at the commit Manyhands pushed. If a reviewer pushed to it, it is kept and the answer says so (`data.remote_branch` `kept`): tell the person, the branch is theirs. A local setup branch that is checked out or has moved is kept too (`data.local_branch` `kept`). `data.remote_branch` `not_published` means nothing was ever pushed and nothing was connected to. A first-time setup is unregistered (`data.unregistered` true), and a cancelled change leaves the repository as it was. The `repo enable` or `remote select` request that made the setup is finished as `cancelled`: sent again it answers `cancelled` and starts nothing, so start a new request when the person wants to try again. A key, unlock, host or network stop leaves the cancel's request accepted, so send the same request again after the cause is dealt with; `push_rejected` (exit 4) means the server refused or lost the delete, and the same request again retries it. A setup that has already merged answers `recovery_required` (exit 3, `data.type` `setup`) and changes nothing, because cancelling it would leave the main branch configured and this clone unregistered: run `poll once`, which finishes a merged setup. A repository with no setup, or not registered, answers `already_applied`.

`repo remove` is the way out when the remote can no longer be reached or the root is gone, and it is the person's ("Remove a repository: hand-over" below). It always works on a pending setup: it finishes the setup's request as `cancelled`, drops the registration and the record, and leaves both setup branches, the local one and the remote's, for the person to delete by hand. After a pending change, the person then runs `repo enable` again, which registers the repository as a clone.

### Routes by hand

Manyhands has no verb for these. They are the person's Git work, and you say what is needed and never run it (from the contract, not run here).

- **Another remote for a pull-request repository.** `remote select` refuses it (`invalid_input`), because the change would have two remotes in play. Moving the repository's branches between hosts is Git work outside these skills (a push of the main branch with its `.manyhands/config.toml`, and of each item branch, to the new remote), and the person does it. Then they point the selected remote's name at the new URL (`git remote set-url <name> <new-url>`) and run `repo remove` and `repo enable` again, which registers the checkout as a clone of what the new remote holds, and approve the new host ([keys and hosts](../manyhands-keys-and-hosts/SKILL.md)). `repo remove` drops the root's operation records, so unfinished operations (`operation list`) should be finished first.
- **A change of remote from a direct repository.** Naming a different remote is allowed, but that remote's main branch must hold this repository's current configuration, a mirror of the current remote's main branch or the remote the configuration was pushed to by hand; otherwise the change answers `external_change`. Commits that reach the local main branch from the current remote while the change is pending must reach the new remote too, or the setup poll answers `diverged` until they do. Cancelling, or `repo remove` and enabling again, is the way out.
- **A lost record.** A corrupt registry that was set aside and replaced by an empty one loses a pending setup's record. The local setup branch and the remote's stay, and Manyhands no longer knows of the setup: `repo inspect` shows `setup` null (and the repository may be unregistered). The setup id is the local branch's: `git branch --list 'manyhands/setup/*'`, or `local_branches` in `repo inspect`. The person deletes that local branch (`git branch -D manyhands/setup/<setup id>`) and only the remote branch with the same id (`git push <remote> --delete manyhands/setup/<setup id>`), because another clone's setup branch on the remote is a teammate's pending pull request. They decline their own pull request, and run `repo enable` again. [Recovery](../manyhands-recovery/SKILL.md), "A pending setup", has the signs.

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

`repo remove` unregisters a repository from Manyhands: it drops the registration, the index and the operation records, and leaves the repository's files and Git history alone. A pending setup never blocks it: it also drops the setup's record, finishes the setup's request as `cancelled`, and leaves the setup's branches, with the root present or gone ("Cancel a setup" above). It is a hand-over verb ([autonomy](../manyhands-overview/references/autonomy.md)): you may run `--prepare` to tell the person exactly what it will do (`data.effects`: `registration_removed`, `index_dropped`, `operation_records_dropped`, `remote_state_dropped`; and `details.files_changed` and `details.git_state_changed`, both false), and you never confirm it. Give them the command to run.

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
| `setup_pending` (3) | The repository has a setup that has not merged yet, and this verb waits for it. Nothing was written. "A pending setup" above; [recovery](../manyhands-recovery/SKILL.md), "A pending setup". |
| `remote_primary_missing` (3) | A pull-request `repo enable` or `remote select` found no main branch on the remote to set up from. The setup ended. The person pushes one first. |
| `remote_already_set_up` (3) | The remote's main branch already holds a Manyhands configuration. The setup ended. Update the checkout and `repo enable` again, which registers a clone. |
