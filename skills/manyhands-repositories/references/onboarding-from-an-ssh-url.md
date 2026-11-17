# Onboarding from an SSH URL: why the order, and where it fails

The sequence in `../SKILL.md` was run end to end here against a throwaway OpenSSH `sshd` on `127.0.0.1` port 2299 (an ed25519 host key, public-key login only, a bare repository holding a Manyhands-enabled `main`). It was not run against a real Git forge (GitHub, GitLab, Gitea) or on Windows or macOS; check those before relying on details such as how a forge presents several host key algorithms.

## Why this order

- Manyhands generates keys under `~/.ssh/manyhands/` and never uses ssh-agent or default keys, so plain `git clone` does not use the selected key. The clone names the key file with `GIT_SSH_COMMAND` and `-o IdentitiesOnly=yes`, so that `ssh` offers that key and no other.
- `ssh` does its own host check against `known_hosts`. With no terminal it cannot ask, and answers `Host key verification failed` (run: both with and without `BatchMode`). The tempting fix, `StrictHostKeyChecking=no` or `accept-new`, trusts whatever answers, so the person's confirmation of the fingerprint would mean nothing. The sequence reads the fingerprint with `ssh-keyscan`, has the person check it, then trusts exactly that key, in a file used for this one clone (`UserKnownHostsFile`). Your own `known_hosts` is not modified.
- `host observe` and `host approve` cannot come before the clone: they need a registered repository with a selected remote. Run against the unregistered clone, `host observe` answered `repository_not_registered`, and so did `remote list` and `remote select`. So the order is clone, `repo enable`, remote, then Manyhands' host pin.
- Git's trust and Manyhands' pin are separate. After `host approve` there was no `~/.ssh/known_hosts`; Manyhands compares the presented key with its own pin, and a `known_hosts` entry does not stand in for it.

## What each step answered (real runs)

| Step | Result |
| --- | --- |
| Clone before the public key was on the host | `<user>@127.0.0.1: Permission denied (publickey).` exit 128 |
| Clone with no terminal and no `known_hosts` entry | `Host key verification failed.` exit 128 |
| `ssh-keyscan -p 2299 127.0.0.1` then `ssh-keygen -lf` | `256 SHA256:... [127.0.0.1]:2299 (ED25519)`, the same fingerprint `host observe` later reported |
| Clone with the key authorized, the scanned key trusted | success; `.manyhands/config.toml` came with it, and `origin` carried the SSH URL |
| `repo enable --prepare` with no `identity` | `identity_required`, exit 3 (a global Git identity was present and ignored) |
| `repo enable` confirmed, with `identity` | success, but `repo identity` still `Source: none`; `repo identity-set` then wrote it to the clone's local Git config |
| `remote select` (config had no selection) | success, and a local commit "Configure Manyhands publication remote" on `main` |
| `host observe` (host not yet pinned) | `presented_fingerprint` equal to the scanned one, `pinned_fingerprint` null; it did not authenticate, so it worked with a passphrase key and with a key the host rejects |
| `host approve --prepare` and `--confirm` | success; `host list` showed `127.0.0.1:2299  ssh-ed25519  SHA256:...` |
| `repo sync` afterwards | success, `publication: published`: Manyhands' own transport used the key and the pin |

## Where it stops

| Where | What happens | Hand-off |
| --- | --- | --- |
| Passphrase-protected key | In a terminal `ssh` prints `Enter passphrase for key '...'`, unless the command has `-o BatchMode=yes`, which suppresses the prompt (the clone then fails with `Permission denied (publickey)`; verified in a pty both ways). With no terminal it fails the same way, which looks like a missing public key. `host observe` of a pinned host, `host approve --confirm`, `host replace --confirm`, sync and `ticket close` answer `unlock_required`; `host observe` of a host not yet pinned does not authenticate and works. | The person runs the clone (without `BatchMode`), and later the blocked Manyhands command with the same confirmation ID and request ID, at their own terminal. You never type or fetch the passphrase. |
| Host fingerprint (Git) | No terminal, no trusted key: `Host key verification failed`. | Show the scanned fingerprint and wait for the person's OK before trusting it. |
| Host fingerprint (Manyhands) | `host observe` is read-only and trusts nothing; `host approve` is show-then-wait. A wrong fingerprint answers `host_mismatch`. | Show the exact fingerprint, wait for the OK, never approve a fingerprint you only observed. |
| The public key is not on the host | `Permission denied (publickey)` from Git; `key_rejected` from `host approve --confirm`. | The person adds the key by hand. |
| The host key changed after it was pinned | `host observe` shows the pinned and the presented key side by side; sync answers `host_replacement_required`. | The person decides; `host replace` is show-then-wait. |

## Notes

- `scp`-style URLs (`git@host:org/repo.git`) were accepted by `remote add` and `remote select`. They have no port, so they use port 22.
- `host observe --authority <host:port>` accepted the selected remote's own `127.0.0.1:2299` and answered as without it; `other.example.com:22` answered `invalid_input`. Use it only to name the selected remote's host.
- A changed pin and an unpinned host both stop sync with a `host.approve` or `host.replace` action in `recovery`. Neither has a no-prompt override; that is the design.
