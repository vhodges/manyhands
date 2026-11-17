---
name: manyhands-keys-and-hosts
description: "Use when enrolling, choosing or inspecting an SSH key for Manyhands, giving its public key to a Git host, approving or replacing a host fingerprint, or when a command answers no_selected_key, unlock_required, host_approval_required or host_replacement_required."
---

# Keys and hosts

Read the [overview](../manyhands-overview/SKILL.md) first. Who runs what is in [autonomy](../manyhands-overview/references/autonomy.md); input files, `--request-id`, prepare and confirm are in [CLI conventions](../manyhands-overview/references/cli-conventions.md); codes are in [errors and exit codes](../manyhands-overview/references/errors-and-exit-codes.md). This skill says only what is specific to keys and host trust.

Manyhands talks to a Git host over SSH with one key and checks the host against its own pin. The order is always: `key generate` (or `key import`), then `key select`, then the key on the host (by the person), then a repository enabled and with a selected remote ([repositories](../manyhands-repositories/SKILL.md)), then host trust. Only after those can `host observe` and `host approve` run. Keep your `--input` files in a temporary directory outside any repository, and delete them when done.

## How keys work

- A generated key is two files at `~/.ssh/manyhands/<key-id>` (private, mode 600) and `<key-id>.pub`. `key show` prints the real paths in `data.private_key_path` and `data.public_key_path`; use those, never a guess. An imported key stays where the person keeps it, and Manyhands does not copy it. Never read a private key file.
- Manyhands never uses ssh-agent, `~/.ssh/id_*` or any key it was not given. Only a registered key works.
- **Selection is global for now.** One key is selected for the whole application, and Manyhands' own SSH connections for every enabled repository use it. Plain `git` and `ssh` do not: they ignore the selection. `key select` and `key clear` take no `--repo` and say nothing about a repository (verified: run from outside any repository, from two different enabled repositories, the same key is selected). It may become per repository later, so do not assume: read the current selection with `key list` or `key show` before you rely on or change it. Selecting another key replaces the selection for every repository, so tell the person when you switch a key that was already selected.
- The selection is in `key list`: each entry in `data.items` has `selected` (true or false). `key show --key-id <KEY_ID>` shows the same in `data.selected`. Without `--json`: `key list` prints `selected` or `-` on each line, and `key show` prints `Selected: true`. `repo inspect` and `repo list` do not show a key.

## The passphrase choice: the person decides

Explain both to the person and wait for their choice. Do not choose silently.

- `none`: no passphrase. You can sync, close tickets and observe hosts unattended. It is weaker if the key file ever leaks, because anyone holding the file can use it.
- `passphrase`: stronger if the file leaks. But Manyhands cannot unlock it for you. These answer `unlock_required` (exit 3) in a non-interactive run: `host observe` of a host that is already pinned, `host approve --confirm`, `host replace --confirm`, sync and `ticket close`; and the clone in onboarding prompts for it. The person must run those steps at their own terminal and type the passphrase. You never see or handle it.

Recommend `none` for a key used only with Manyhands, a key that exists for no other purpose and can be revoked on the host in seconds, and say that reason. A key the person also uses for other things deserves the passphrase. Say that it is their choice.

## Generate or import a key

If the person says a key is already set up but `key list` is empty, do not guess and do not generate one. Ask which key it is and where its files are; offer `key import` for an existing key, or generate a new one if they prefer.

`key generate` and `key import` are not confirmable (`--prepare` answers `not_confirmable`). Describe the action in words, wait for the person's OK, then run it.

| Verb | Required | Optional |
| --- | --- | --- |
| `key generate` | `label` (string), `protection` (`none` or `passphrase`) | none |
| `key import` | `label`, `private_key_path` (strings; paths are relative to where you run the command) | `public_key_path` (string) |

No passphrase field exists: a `passphrase` field is rejected as unknown, and human output says `Unknown input field: passphrase.` Keep the file free of secrets.

```json schema=key_generate
{
  "label": "Manyhands key for <person>",
  "protection": "none"
}
```

```sh
manyhands-cli --json --non-interactive key generate --input key.json --request-id <REQUEST_ID>
```

The answer's `data.id` is the key ID and `data.fingerprint` its fingerprint. With `"protection": "passphrase"` a non-interactive run answers `unlock_required` and writes nothing, because the passphrase can be typed only at a terminal. Hand that command to the person: give them the file and the command line without `--json --non-interactive`, and ask them to run it and tell you when it is done. Then find the new key with `key list` (it never shows a passphrase).

```json schema=key_import
{
  "label": "Existing key",
  "private_key_path": "<path-to-private-key>",
  "public_key_path": "<path-to-private-key>.pub"
}
```

```sh
manyhands-cli --json --non-interactive key import --input import.json --request-id <REQUEST_ID>
```

Import records the path and reads only the public companion. Without `public_key_path` the key has no fingerprint in `key list` and `key public` answers `public_key_unavailable` (exit 3); then ask the person for the `.pub` text or have them import with `public_key_path`. An imported passphrase-protected key imports fine; it fails later with `unlock_required` when used.

## Read keys

```sh
manyhands-cli --json key list
manyhands-cli --json key show --key-id <KEY_ID>
manyhands-cli key public --key-id <KEY_ID>
```

`key public` prints the public key line (for `--json`, `data.public_key` and `data.fingerprint`). That line is what the person adds to the Git host.

## Select and clear (alone)

```sh
manyhands-cli --json --non-interactive key select --key-id <KEY_ID> --request-id <REQUEST_ID>
manyhands-cli --json --non-interactive key clear --request-id <REQUEST_ID>
```

Neither takes `--repo` or `--input`. `key select` of an unknown ID answers `key_not_found` (exit 2). Clearing when nothing is selected answers `already_applied`.

## The public key goes to the Git host by hand

Manyhands does not upload keys to a Git host. Show the person the `key public` line and ask them to add it to their account or deploy keys on the host, then wait. Until they have, any SSH step fails with `Permission denied (publickey)` (verified: a clone before the key was authorized failed this way). Do not try to automate adding it.

## Host trust

Manyhands pins a host's key itself, per `host:port`. This is separate from Git's and OpenSSH's `known_hosts`: approving a host in Manyhands does not touch `~/.ssh/known_hosts`, and a trusted `known_hosts` entry does not satisfy Manyhands (verified: after `host approve` no `known_hosts` file existed).

Prerequisites, all checked: a registered, enabled repository (`repository_not_registered` otherwise), a selected publication remote (`publication_remote_required`, exit 3), and a selected key (`no_selected_key`, exit 3). 
`host observe` makes a real network connection to the remote's host to read its current key. It writes nothing and trusts nothing. Whether it authenticates depends on the pin: observing a host that is not pinned yet does not authenticate, so it works with no key accepted by the host and with a passphrase-protected key (verified); observing a host that is already pinned does, and then answers `unlock_required` for a passphrase-protected key and `key_rejected` for a key the host does not accept. Run it only when the person has asked you to set up or check that host, and say that it connects to the host. It is the way to learn the fingerprint to show them.

```sh
manyhands-cli --json --non-interactive host observe --repo <repo-root>
```

Read `data.host`, `data.port`, `data.presented_algorithm` and `data.presented_fingerprint`. `pinned_fingerprint` is null when the host is not yet trusted.

Show the person the exact `presented_fingerprint` and algorithm, and ask them to confirm it against a source they trust (the Git host's published fingerprints, or their administrator). **Do not run `host approve` on a fingerprint you only observed, without their OK.** A fingerprint you saw on the wire is exactly what an interception would also show you.

| Verb | Required | Optional |
| --- | --- | --- |
| `host approve` | `authority`, `algorithm`, `fingerprint` (strings) | none |
| `host replace` | the three above plus `old_fingerprint` | none |

`authority` is `host:port`, with the port from `data.port` of `host observe`. It must be the host and port of the selected remote: any other authority answers `invalid_input` and, even without `--json`, names no field (verified), so check it first. Human `host observe` (no `--json`) prints a ready-made `Input for host approve:` JSON line with the right `authority`, `algorithm` and `fingerprint`; you may use it as the input file, after the person has confirmed the fingerprint. A bare host means port 22: for a host on another port, omitting the port is refused with `invalid_input`. IPv6 is `[address]:port`. `algorithm` and `fingerprint` are copied from `presented_algorithm` and `presented_fingerprint`, character for character.

```json schema=host_approve
{
  "authority": "<host:port>",
  "algorithm": "<presented_algorithm>",
  "fingerprint": "<presented_fingerprint>"
}
```

```sh
manyhands-cli --json --non-interactive host approve --repo <repo-root> --input approve.json --prepare
manyhands-cli --json --non-interactive host approve --repo <repo-root> --input approve.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

The prepare names `details.host`, `details.port`, `details.algorithm` and `details.fingerprint`; show them with the OK request. Confirming connects again, requires that the host still presents that fingerprint, and authenticates with the selected key before it stores the pin. So a wrong or stale fingerprint answers `host_mismatch` (exit 3) with the presented one in `data`, and a key the host does not accept answers `key_rejected` (exit 3, verified). Never fix a `host_mismatch` by pasting the other fingerprint without showing the person.

With a passphrase-protected selected key, `--confirm` answers `unlock_required` and nothing is stored. Hand the same command to the person for their terminal, with the same confirmation ID and request ID (verified: it then succeeds within the confirmation's expiry, after the passphrase prompt). `host replace --confirm` behaves the same.

```sh
manyhands-cli host approve --repo <repo-root> --input approve.json --confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>
```

Check the pin afterwards:

```sh
manyhands-cli --json host list
manyhands-cli --json host inspect --authority <host:port>
```

`host list` has no `--repo`: pins are application-wide.

### A changed host key

If a host that was pinned now presents a different key, `host observe` shows both `pinned_fingerprint` and `presented_fingerprint`, and sync answers `host_replacement_required` with a `host.replace` action; `host approve` answers the same. A changed host key is either a legitimate rotation or someone intercepting the connection. Tell the person plainly, show both fingerprints, and let them decide. `host replace` is a show-then-wait verb like `host approve`; its input is the table above, with `old_fingerprint` the pinned one.

```json schema=host_replace
{
  "authority": "<host:port>",
  "algorithm": "<presented_algorithm>",
  "fingerprint": "<presented_fingerprint>",
  "old_fingerprint": "<pinned_fingerprint>"
}
```

```sh
manyhands-cli --json --non-interactive host replace --repo <repo-root> --input replace.json --prepare
```

Confirm as for `host approve`.

## Hand to the person

- `key remove` (unregister) and `key delete` (delete the generated files). Never run them, even if asked to "clean up". You may run `key delete --prepare` to show what it would remove (the private and public key files and the registration); `key remove` has no preview (`not_confirmable`). See [autonomy](../manyhands-overview/references/autonomy.md). `key delete` refuses a selected key (`selected_key_in_use`) and an imported key (`key_not_deletable`; imported files are the person's). Tell the person which key and why.
- Any passphrase: `key generate` with `passphrase`, and any step that answers `unlock_required`.
- Adding the public key to the Git host.

## Codes you will meet

| Code (exit) | Meaning and next step |
| --- | --- |
| `no_selected_key` (3) | No key is selected. `key list`, then `key select`. |
| `unlock_required` (3) | The selected key has a passphrase and the step authenticates. Hand the command, with its confirmation ID and request ID, to the person's terminal. |
| `publication_remote_required` (3) | The repository has no selected remote. [Repositories](../manyhands-repositories/SKILL.md). |
| `repository_not_registered` (3) | This data directory does not know the repository. [Recovery](../manyhands-recovery/SKILL.md), "Not registered": `repo enable --prepare`, not `index rebuild`. |
| `host_approval_required` (3) | A step needs the host pinned. `host observe`, show the person, `host approve`. |
| `host_replacement_required` (3) | The pinned key changed. See "A changed host key". |
| `host_mismatch` (3) | The fingerprint you gave is not what the host presents. Observe again and show the person. |
| `key_rejected` (3) | The host did not accept the selected key. The public key is not on the host, or is on the wrong account. Tell the person. (`host observe` of a host that is not pinned yet still works; of a pinned host it also answers `key_rejected`.) |
| `key_not_found` (2) | The key ID is not registered. `key list`. |

Git itself reports a rejected key as `Permission denied (publickey)`.
