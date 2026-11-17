---
name: manyhands-getting-started
description: "Use when a person is new to Manyhands or asks you to set it up, enable it, onboard a repository or an SSH URL, or get to a first working repository with a key, a remote and a first poll."
---

# Getting started

Read the [overview](../manyhands-overview/SKILL.md) first. This skill is only the order of the setup and the check after each step; the how, the inputs and the failures are in the skills it sends you to, and it repeats none of them. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md).

Setup is where you stop most often, because the key, the host fingerprint and any passphrase are the person's. Do the checks, show what they ask for, and wait at every "Stop" below. Never choose for them, never skip a stop to save time, and never handle a passphrase.

## Step 0: is it installed

- `manyhands-cli --version` prints `manyhands-cli <version>`. It must be at least the line `Minimum CLI version` in the [skills README](../README.md) (a newer one is fine and never a warning). If the command is not found or is older, stop and tell the person: installing the CLI is theirs, and the README has the install steps.
- You are reading this skill, so the skills are installed. They are meant to be installed as the whole `skills/` directory, because they link to each other; if a link here does not open, say so.
- Find the repository's main checkout root (the first line of `git worktree list`). It is the `--repo` of every command below. Keep your `--input` files in a temporary directory outside the repository.

## Pick the starting point

Ask the person which one is theirs, or look: `manyhands-cli --json repo inspect --repo <repo-root>` tells you whether the path is registered, has a Manyhands configuration and has remotes.

| The person has | Visit, in this order |
| --- | --- |
| A. A Git repository with no remote | repository (enable), then first read. The key and the remote wait until they want to share. |
| B. A clone, or a repository, that already has an SSH remote | key, repository (enable), remote (select), host pin, first poll |
| C. Only an SSH URL | key, then the [SSH URL onboarding](../manyhands-repositories/references/onboarding-from-an-ssh-url.md) in full (fingerprint, clone, enable, identity, remote, host pin), first poll |
| D. Nothing yet | repository (create), then first read. A remote later, as in B. |

The order is key, then repository, then remote, then host pin. A host pin cannot come before a registered repository with a selected remote and a selected key. If the person already said they want a key now, with or without a passphrase, do the key step first in A and D too.

## Step 1: look at what exists

```sh
manyhands-cli --json key list
manyhands-cli --json host list
manyhands-cli --json repo list
```

Keys and host pins are application-wide, not per repository: another repository's key may already be selected, and selecting a different one replaces it for all of them. If a key is already selected, ask the person whether to use it; do not generate a second one. If they say a key exists but `key list` is empty, do not guess: that is the question in [keys and hosts](../manyhands-keys-and-hosts/SKILL.md).

## Step 2: a key (B and C always; A and D when they want to share)

Go to [keys and hosts](../manyhands-keys-and-hosts/SKILL.md) for how: explain the passphrase choice and recommend none for a key used only for Manyhands, then `key generate` or `key import`, then `key select`.

- **Stop: the key choice.** Say which of generate or import, and passphrase or none, and wait. Generating is show-then-wait. In A and D there is no remote yet, so no key is needed until one exists: the person decides whether to generate one now. If they already gave the protection choice (none or passphrase), that is the choice; still describe the action first, because `key generate` has no preview.
- **Stop: any passphrase.** A passphrase key cannot be generated or unlocked by you. Hand the command to the person's terminal.
- **Stop: the public key.** For B and C, show the `key public` line and ask the person to add it to their Git host. Wait until they say it is done.
- Check: in `manyhands-cli --json key list` the selected key has `data.items[].selected` true. Without `--json` each line is `<id> <label> generated selected <fingerprint>`: the fourth column reads `selected`, and `-` for a key that is not selected (right after a generate and before the select). Run no host command yet.

## Step 3: the repository

Go to [repositories](../manyhands-repositories/SKILL.md): `repo enable` for A and B, `repo create` for D (a path that is already a Git repository answers `occupied_path`: use `enable`). For C the clone and the enable are inside the SSH URL onboarding.

- **Stop: the commit identity and the preview.** Ask for the name and email (do not take them from global Git settings), run `--prepare`, show the effects in words and wait for the OK before `--confirm`.
- A clone that already has `.manyhands/config.toml` needs `repo identity-set` after the enable; the repositories skill says why.
- Check:

```sh
manyhands-cli repo inspect --repo <repo-root>
manyhands-cli repo identity --repo <repo-root>
```

Good answer: `Registered: true`, `Configuration: valid`, `Identity: available`, and in `repo identity` `Source: repository`. `Publication remote: -` is right for A and D. If a command answers `repository_not_registered` or `repository_not_enabled`, this data directory does not know the repository yet: [recovery](../manyhands-recovery/SKILL.md), "Not registered", then `repo enable`, never `index rebuild`.

## Step 4: the remote (B; for C it is inside the onboarding, so only run the check; skip for A and D)

`remote select` is in [repositories](../manyhands-repositories/SKILL.md), and `remote add` first if the repository has no remote yet. Both you run alone, and both need a registered repository (`remote list` answers `repository_not_registered` before the enable; `repo inspect` still lists the remotes).

- Check: `manyhands-cli remote list --repo <repo-root>` ends `selected=true` on the remote, and `repo inspect` shows `Publication remote: origin` (or its name). With no remote `remote list` prints nothing.

## Step 5: the host pin (B; for C it is inside the onboarding, so only run the check)

Go to [keys and hosts](../manyhands-keys-and-hosts/SKILL.md), "Host trust": `host observe`, show the fingerprint, then `host approve`. For C this is step 8 of the onboarding.

- **Stop: the fingerprint.** In C the person has already seen the scanned fingerprint; in B this is the first time. Show `presented_fingerprint` and the algorithm exactly, ask them to confirm it against the Git host's published list, and wait. `host approve` is show-then-wait, with its own preview.
- **Stop: a passphrase at `host approve --confirm`.** It answers `unlock_required`: give the person the same command for their terminal.
- Check: `manyhands-cli host list` has a line `<host>:<port>  <algorithm>  <fingerprint>` (host pins are not per repository), and `manyhands-cli host inspect --authority <host:port>` shows the pinned fingerprint. If `host_approval_required` came earlier, it is gone.

## Step 6: the first poll (B and C)

```sh
manyhands-cli --json --non-interactive poll once --repo <repo-root> --request-id <REQUEST_ID>
manyhands-cli poll status --repo <repo-root>
```

[Polling and the index](../manyhands-polling-and-index/SKILL.md) has the answers. A fresh `<REQUEST_ID>` comes from `manyhands-cli id new`. Good: `Completion: completed`, every target `current` (or `fast_forwarded`, `materialized`), and `poll status` showing `Latest outcome: completed` and a `Latest observed at`.

- A and D: `poll once` answers `publication_remote_required` (exit 3). That is expected, not a failure: a repository with no remote works fully by itself. `poll status` still answers, with `Latest outcome: -`.
- **A remote with no `main` yet.** A brand-new empty remote has no main branch. The first `poll once` then reports `refs/heads/main` as `history_unknown`, and `repo sync` against it answers `invalid_configuration` and leaves an unfinished operation that makes later syncs `busy` (all of this seen once, with an empty remote; not reproduced). So read the targets, and do not run `repo sync` yet. Putting main on the remote is the person's step: tell them, wait, and they push it with their own Git and credentials. Then poll again: `refs/heads/main` should read `current` (not verified; expected). If you already ran a sync, [recovery](../manyhands-recovery/SKILL.md) covers the leftover operation.
- `host_approval_required`, `no_selected_key`, `key_rejected` and `unlock_required` send you back to step 5 or 2; none of them is yours to fix alone.

## Step 7: a first read and, if they want, a first ticket

```sh
manyhands-cli ticket list --repo <repo-root>
manyhands-cli document list --repo <repo-root>
```

An empty repository prints nothing and exits 0; that is a working repository with nothing in it yet. A list that answers `repository_not_registered` goes back to step 3.

The setup is done when these are all true: `key list` shows a selected key (when the person is sharing), `repo inspect` shows the repository registered with an available identity, and (with a remote) `host list` has the host and `poll once` completed. Say so to the person in those terms, and offer a first ticket rather than creating one unasked. Then hand on:

- Looking around: [finding work](../manyhands-finding-work/SKILL.md).
- Tickets: [tickets](../manyhands-tickets/SKILL.md). Documents: [documents](../manyhands-documents/SKILL.md). Comments: [comments](../manyhands-comments/SKILL.md).
- Sharing work with collaborators later: [sharing and sync](../manyhands-sharing-and-sync/SKILL.md).

## Not run end to end

Walked in scratch repositories for A, B and D, with B against a throwaway local `sshd` on a high port and a bare repository. C was run end to end earlier and is in the onboarding reference; this skill only sequences it. Not run: a real Git forge, a passphrase key through the whole setup (the individual hand-offs are in [keys and hosts](../manyhands-keys-and-hosts/SKILL.md)), Windows and macOS.
