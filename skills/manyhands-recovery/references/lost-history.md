# Lost history: `history_unknown` and its repair verbs

Run in scratch clones of one remote, after overwriting the data directory's database file and rebuilding. A real loss of the database may behave differently.

## Why it happens

Manyhands records, per item, whether its branch was ever published. When that record is lost it cannot tell "never published" from "published, then lost track". It stops anything that could publish the wrong thing: `item sync` of that item, and it sets a repository-wide flag (a marker file `remote-history-recovery-required` in the data directory).

What was observed after the rebuild: `poll once` reported the never-published item as `history_unknown` (and an item whose remote branch Git still tracked was fine, `diverged`); `item sync` on the unknown item answered `blocked` / `recovery_required` (exit 3, nothing written, `publication` `pending`) with `recovery` `remote.history`; `repo sync` of main still answered `ok`. The rebuild also lost the selected key and the host pin: restore them first ([polling and the index](../../manyhands-polling-and-index/SKILL.md), [keys and hosts](../../manyhands-keys-and-hosts/SKILL.md)).

## 1. Look: `remote history` (you run it alone)

```sh
manyhands-cli --json remote history --repo <repo-root>
```

It connects to the publication remote and runs one poll first, so it has `poll once`'s side effects (it may bring clean branches forward and materialize new items; it never pushes), and it needs the key and the pinned host. It takes no `--request-id`. It lists `data.entries[]`: `kind`, `item_id`, `branch`, `local_oid`, `tracking_ref_exists`, `remote_oid`, `relation` (to the remote), `ahead`, `behind`, `resolved`, `disposition` and `choices` (`publish`, `release`, `republish`), and `data.cache_loss`, the repository-wide flag. Without `--json` each entry is one line and a `next:` line gives the repair command. A passphrase key answers `unlock_required` (not run for this verb): the person runs it.

Two states were seen:

- An item with no remote branch and no tracking ref: `relation` null, `resolved` false, `disposition` `history_unknown`, `choices.publish` true. This needs a decision.
- Items whose remote branch exists: `relation` `same`, `resolved` true. The poll settled them; no decision is needed, though `cache_loss` stayed true until the flag is cleared.

## 2. Decide: the person

For an unresolved entry the person chooses; `publish` is the only choice the CLI offers (`release`, giving up the local copy, belongs to the desktop app: `item resolve-history --as release` answered `invalid_input`). Show them the entry: the item, the branch, `local_oid`, whether the remote has it, and what `publish` means: it records the item as never published, so the next `item sync` publishes it (verified, and the sync then answered `published`).

`item resolve-history` is a hand-over verb ([autonomy](../../manyhands-overview/references/autonomy.md)). You may run `--prepare`; the preview has effect `history_resolved` and `details` (branch, `local_oid`, `tracking_ref_exists`, `cache_loss`) and expires in about ten minutes. You never run `--confirm`.

```sh
manyhands-cli --json --non-interactive item resolve-history --repo <repo-root> --id <ITEM_ID> --as publish --prepare
```

Give the person the same command with `--confirm <CONFIRMATION_ID> --request-id <REQUEST_ID>` instead of `--prepare`. On an item that is not unknown, `--prepare` answered `already_applied`. A missing `--as` is a `usage_error`.

After the person confirms, the answer was `ok` with `data` `cache_loss_cleared` true and `marker` `removed` when that was the last unknown item. Then run `item sync` for it (alone) and `poll once`.

## 3. The repository-wide flag: `remote resolve-history`

If `remote history` still shows `cache_loss` true with nothing unknown, the flag is cleared with `remote resolve-history`. It is a hand-over verb, with a preview:

```sh
manyhands-cli --json --non-interactive remote resolve-history --repo <repo-root> --prepare
```

Verified: the preview had effect `history_flag_cleared` and `marker_present`; the person's `--confirm` answered `ok` with `cache_loss_cleared` true and `marker` `removed`, and `remote history` then showed `cache_loss` false. While an item still reads unknown it answers `recovery_required` (`recovery` `repo.inspect`), and with nothing to clear it answers `already_applied`.

## What stayed

After the repairs the file `ssh-host-trust-reapproval-required` was still in the data directory although the host had been approved again, and `poll status` still said `recovery_suspended` true. Neither has a CLI verb. Tell the person; do not delete files in the data directory.

`history_unresolved`, the other blocker named in some answers, and `republish` were not run.
