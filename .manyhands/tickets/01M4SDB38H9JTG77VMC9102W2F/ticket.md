---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4SDB38H9JTG77VMC9102W2F"
title: "Enabling a repository can erase another process's Git configuration change"
type: "defect"
status: "open"
project: "manyhands"
team: "core"
---

When a repository is enabled with an identity supplied by the caller, Manyhands
copies the whole of the repository's Git configuration file (`.git/config`)
into memory, writes the identity, and, if any later step of enablement fails,
puts the copy back over the file. The copy is put back without using the lock
that Git programs use to keep their configuration writes apart. A change that
another program made to the file in the meantime, for example `git remote add`
or `git config` run in a terminal or by an editor, is erased. Nothing reports
the loss: the caller is told only that enablement failed and changed nothing.

Terms used below:

- **Git's configuration lock.** Every Git program that changes `.git/config`
  first creates a file named `config.lock` beside it, which only one program
  can create at a time. It writes the new configuration into that file and
  renames it over `config`. A program that finds `config.lock` already there
  does not write.
- **libgit2.** The Git library Manyhands uses. Its `Config::set_str` call sets
  one key and follows the lock protocol for that one key.
- **Snapshot and restore.** Reading a whole file into memory before a change,
  and writing those bytes back if the change is abandoned.
- **Repository lease.** Manyhands' own lock file for a repository. It keeps
  Manyhands operations apart from each other. No other program knows of it.

## Evidence

Found on 2026-10-12 by the reviews of Wave 03 Cycle F2 Task 15 (ticket
`01M4CC0VMR8HSPZXQ1WX41GWVK`, branch
`manyhands/ticket/01M4CC0VMR8HSPZXQ1WX41GWVK`). The same weakness was found in
the new `repo identity-set` command during that Task and removed from it there.
The enablement code is older than that Cycle and is the same on `main`.

Everything in this section was traced by reading the code on that branch, the
code on `main`, and the vendored libgit2 source (`libgit2-sys-0.18.8+1.9.7`:
`config.c`, `config_file.c`, `repository.c`, `util/filebuf.c`) and `tempfile`
3.27. Nothing was compiled or run, and no test reproduces the loss.

### Where the snapshot and restore are

There is one place in `src/` that restores `.git/config` from a snapshot:
enablement, in `RepositoryService::enable_recorded` (`src/repository.rs`).

- **Snapshot.** After all its checks have passed and before it writes
  anything, `enable_recorded` reads `<git directory>/config` into
  `local_config_before` with a plain file read. It reads it on every call that
  reaches this point, whether or not it will be used.
- **Writes.** If the request supplies an identity, `write_local_identity` sets
  `user.name` and then `user.email` with two `Config::set_str` calls. libgit2
  directs both to the repository's own configuration file
  (`git_repository_config` sets the write order to the local level). Each call
  takes `config.lock`, re-reads the file, writes, and renames, so each key by
  itself is safe against other programs. The two together are not one step.
  Enablement then, in order: points `HEAD` at the primary branch if the
  repository has no commit yet; writes `.git/info/exclude`; creates the
  `.manyhands` directory; writes `.manyhands/config.toml`; records the step in
  its journal; builds a tree; and makes the initialization commit. None of
  those later steps is a call that sets a configuration key. That libgit2
  writes no Git configuration inside its own `set_head` and commit calls is
  assumed from reading enablement's calls; libgit2's code for those calls was
  not read, and this must be checked in planning.
- **Restore.** If any of those steps returns an error, `restore_enablement`
  runs. When the request supplied an identity it calls `restore_file` on
  `.git/config` with the snapshot. `restore_file` calls
  `replace_bytes_atomically`, which creates a temporary file in `.git`, writes
  the snapshot into it and renames it over `config`. It does not create or
  check `config.lock`. If there was no `config` file at the snapshot, the
  restore deletes the file instead.
- **When the restore runs.** Only on a failure inside enablement, and only
  when the request supplied an identity. It also runs when the failure was the
  very first `set_str`, that is, when Manyhands had written nothing. It does
  not run on success, and it does not run if the process is killed (the
  identity then stays written and the journal row stays pending).

Enablement holds the repository lease throughout. That keeps other Manyhands
operations out. It does not keep out `git`, an editor or any other program.

Two neighbouring cases use the same helper and are not this defect:

- `set_publication_remote_recorded` (the `remote select` command) snapshots
  and restores `.manyhands/config.toml`. That is Manyhands' own file in the
  working directory, not Git configuration, and Git has no lock for it.
- Enablement also snapshots and restores `.git/info/exclude` as a whole file.
  Git has no lock protocol for that file either. A hand edit of it made during
  a failing enablement would be lost in the same way.

Other code that writes Git configuration does so key by key through libgit2
and restores nothing: `add_remote_recorded` (`Repository::remote`),
`remove_remote_recorded` (`Repository::remote_delete`), and
`committing_identity` in `src/repository/remote/sync.rs` (two `set_str`
calls). These cannot erase another program's change. Their weaknesses are
different and are listed under "Open points for planning".

### The losing sequence

Any failure of enablement after the snapshot will do. One that an existing
test already produces is a file standing where the `.manyhands` directory
should be (`enable_unborn_obstacle_restores_supplied_local_identity` in
`tests/repository_enablement.rs`).

1. `enable_recorded` reads `.git/config` into `local_config_before`.
2. `write_local_identity` sets `user.name` and `user.email`.
3. Another program runs `git remote add origin <url>`. It takes
   `config.lock`, writes, and renames. The file now holds the identity and
   the new remote.
4. A later step of enablement fails, for example creating `.manyhands`.
5. `restore_enablement` renames the snapshot over `.git/config`. The remote
   added at step 3 is gone. So is anything else written since step 1.
6. The caller gets the error of step 4. Through the bound `repo enable`
   command that is `internal_error`. The restore succeeded, so the call is
   marked as having left nothing behind, its journal row is discarded, and the
   result says nothing changed.

The other program's write can land anywhere between step 1 and step 5,
including before step 2.

A second sequence needs no successful Manyhands write at all. It is the one
the Task 15 review gave for the identity command (its finding I1):

1. The snapshot is read.
2. Another program takes `config.lock`.
3. The first `set_str` fails at once. libgit2 does not wait for a held lock.
4. The other program renames its lock over `config`.
5. The restore renames the snapshot over `config`. The other program's change
   is gone. The caller is told `internal_error`.

This one needs step 4 to fall in the short time between steps 3 and 5.

### What else the restore changes

- **File mode (Unix).** The temporary file is created by
  `tempfile::NamedTempFile`, which gives it mode `0600`, and the rename keeps
  that. A restored `.git/config` is therefore readable and writable by its
  owner only, whatever it was before. A shared repository whose configuration
  was group-writable loses that.
- **A linked configuration file.** If `.git/config` is a symbolic link (a
  link to a file elsewhere), the rename replaces the link with a regular
  file. The repository stops following the shared file. libgit2's own writes
  follow the link, so only the restore does this.
- **Missing or partial file.** The restore is one rename, so the file is
  never missing or half-written while the process lives. The temporary file
  is not flushed to disk before the rename, so what survives a power loss at
  that moment was not established. A process killed before the rename leaves
  a stray temporary file in `.git` and the configuration as enablement had
  written it.

Separately from the restore, each libgit2 write creates its lock file with
the process's default mode (`0666` less the umask) and does not copy the
mode of the file it replaces. A successful enablement with a supplied
identity therefore also changes the mode of `.git/config`, in the other
direction: a file kept at `0600` because it holds a credential in a URL
becomes readable by others under the usual umask. This is by reading
`config_file.c` and `filebuf.c`; it was not run.

## Who is affected today

- `RepositoryService::enable`, when `EnableRepositoryRequest::identity` is
  given. Without an identity nothing is written to Git configuration and
  nothing is restored.
- `RepositoryService::create_and_enable`, which always passes an identity to
  the same code. It runs on a repository created a moment earlier, so
  another program writing its configuration at that moment is very unlikely;
  the mode effects still apply.
- On the Cycle F2 branch, the bound commands `repo enable` (when the request
  carries an identity) and `repo create`. These bindings are not on `main`
  yet.

The loss needs three things together: an identity supplied by the caller, a
failure of enablement after the snapshot, and another program writing
`.git/config` between the snapshot and the restore. That span covers a few
file writes, a journal write and a commit, so it is short. Enablement is
normally run once for a repository. The likelihood is low and was not
measured. What makes it a defect is the kind of loss: unrelated settings
(a remote, a branch's upstream, a credential helper), with no message.

## Options

### A. One write under Git's lock, and nothing restored

Do what `repo identity-set` now does (`set_local_identity` and `ConfigLock`
in `src/repository/bridges/identity.rs`, on the Cycle F2 branch only until
it merges):

1. Resolve `.git/config` if it is a link, and create `config.lock` beside
   the real file. If the lock is already there, answer `busy`.
2. Read the file, now that no Git program can change it. Copy it to a
   scratch file in a new directory inside `.git` that only the current user
   can enter. Let libgit2 set the keys in the scratch copy.
3. Write the result into `config.lock`, give the lock the configuration
   file's mode, and rename it over the file.

Every key is set by one rename or none is. A call that fails before the
rename removes its lock and has not touched the file.

For enablement the question is where the rename goes among the other steps,
because enablement also writes files, a commit and journal rows. The commit
does not read the identity from configuration: `enable_recorded` passes the
name and email to the commit directly. So, provided libgit2 itself writes no
Git configuration while it moves `HEAD` and makes the commit (assumed, not
confirmed; see "Open points for planning"), the lock can be taken before the
first write and renamed after the commit succeeds. With that order a failed
enablement has never touched `.git/config`, and nothing needs restoring.

- **Fixes.** The loss of another program's change. The two keys being set
  in two steps. The mode change on restore and on success. The replacement
  of a linked configuration file.
- **Leaves.** A program that ignores `config.lock` can still be overwritten;
  the identity writer narrows this with a byte comparison just before the
  rename and cannot close it. If the process dies, or the rename fails,
  after the commit and before the rename, the repository is enabled and its
  commit carries the right author, and the identity is not stored. A later
  save would then answer `identity_required`. A killed process can leave
  `config.lock` behind; every Git configuration write, by Git or by
  Manyhands, then fails until someone removes the file. Manyhands does not
  remove it, since it cannot tell a dead owner from a live one.
- **Behaviour change.** A held or stale lock makes enablement answer `busy`
  rather than `internal_error`. While enablement holds the lock, another
  program's `git config` fails with Git's usual "could not lock config
  file" message; that is how Git programs already treat each other. An
  identity key that appears twice in the file or comes from an included
  file is refused by the identity writer as `recovery_required`; enablement
  today fails on those as `internal_error`.
- **Cost and risk.** Medium. The writer has to be taken out of the identity
  module into a shared place, and the write phase of `enable_recorded`
  reordered. The writer itself went through three review rounds in Task 15.
  It depends on that branch merging first.
- **Tests and recovery steps.** The existing rollback tests compare
  `.git/config` byte for byte after a failure and should pass unchanged,
  since the file is never touched. New tests are needed for a held lock,
  for a change made by another writer during enablement, and for the mode.
  The journal steps (`ConfigurationWritten`, `InitializationCommitted`) keep
  their meaning. The binding relies on the identity being written before
  the `InitializationCommitted` step is recorded; the rename has to stay
  before that step, and planning has to decide what a retry does when it
  finds the commit made and the identity absent.
- **Platforms.** On Unix the mode is kept. On Windows nothing is done about
  file attributes, a read-only `config` or one held open by another program
  makes the rename fail (the file is left untouched), and none of the
  identity writer's code has yet run on Windows.

### B. Keep write-then-undo, but undo only the keys this call set

Keep `write_local_identity` as it is. On failure, do not restore the file.
Set `user.name` and `user.email` back to the values they had, or remove
them if they were absent, through libgit2, so each undo goes through Git's
lock.

- **Fixes.** The loss of another program's change. The mode change and link
  replacement caused by the restore.
- **Leaves.** The undo is two separate writes; if the second fails, or the
  lock is held at that moment, one key is undone and the other is not, and
  the call has to answer `recovery_required` (the existing
  `RollbackIncomplete` path). The forward write stays two steps. The mode
  change on a successful write stays. A held lock is still `internal_error`
  unless that is mapped separately. libgit2 refuses to set or remove a key
  that appears twice or comes from an included file, so on such a file the
  undo cannot work; enablement's forward write would already have failed on
  the same file. If the user or another program changes `user.name` during
  the window, the undo overwrites that one value.
- **Cost and risk.** Small: one function and its caller, no dependence on
  the Cycle F2 branch.
- **Tests and recovery steps.** Tests that compare `.git/config` byte for
  byte after a failed enablement may need to compare values instead:
  removing the last key of a section through libgit2 was not checked to
  leave the file byte-identical. Journal steps are unchanged.
- **Platforms.** Nothing new; all writes are libgit2's.

### C. Keep the snapshot, but restore only when it is safe

Take `config.lock` before restoring. Under the lock, compare the file with
what this call last wrote. If it is equal, rename the snapshot back with the
original mode. If it differs, leave the file alone and answer
`recovery_required`.

- **Fixes.** The loss of another program's change. The mode change on
  restore, if the mode is copied.
- **Leaves.** In the conflict case the repository keeps the identity that a
  failed enablement wrote, so the result must say that a change remains and
  must offer a way out (for example `repo identity-set`, or enabling
  again). The link replacement stays unless the path is resolved first. The
  forward write stays two steps, and "what this call last wrote" has to be
  read back after them, which has its own gap. The second sequence above
  (lock held at the first write) is covered only because the file then
  differs or the lock is busy.
- **Cost and risk.** Small in lines, but it adds a new outcome (failed, and
  not fully undone, through no fault of the restore) that the result,
  journal and bound command must carry truthfully.
- **Tests and recovery steps.** Existing tests should pass. New tests are needed
  for the conflict case, and the binding needs a recovery action for it.
- **Platforms.** Needs the same lock creation and rename as A, so the same
  Windows notes apply.

### D. Do nothing and document the limit

State in the design that a failing enablement with a supplied identity can
overwrite a concurrent Git configuration change.

- **Fixes.** Nothing.
- **Leaves.** Silent loss of unrelated settings, the mode changes, the link
  replacement. The identity command would behave differently from
  enablement for the same file, and the design sentence that the identity
  command "restores the configuration file's bytes, as enablement does" is
  already out of date for that command.
- **Cost and risk.** None now. The cost falls on a user who loses a remote
  or an upstream setting and has no message to explain it.

## Recommendation

**Option A**, with the lock taken before enablement's first write and
renamed over the configuration file after the commit succeeds.

Reasons:

- It is the only option that removes the restore. B and C keep an undo, and
  each undo has a failure of its own that must be reported and recovered
  from.
- It also corrects what B and C leave: the identity set in two steps, the
  mode change on a successful write, and a foreign lock answered as
  `internal_error`.
- The writer exists and has been reviewed three times. Enablement and
  `repo identity-set` would then treat the same file the same way, and
  answer `busy` for the same condition.
- The existing rollback tests keep their byte-for-byte assertions.

What would change this:

- If the Cycle F2 branch will not merge soon, or the fix is wanted on
  `main` before it does, choose **B** as an interim fix. It is small, it
  needs nothing from that branch, and it ends the whole-file loss.
- If planning finds that holding `config.lock` across the commit is not
  acceptable (a longer span in which other programs' configuration writes
  fail, and a longer span in which a kill leaves a stale lock), the
  alternative within A is to rename before the commit and undo by key if
  the commit fails. That brings back an undo for one step, and B's
  limitations for that step.
- If the identity writer fails on Windows when it is first run there, A
  waits on that.

**One configuration writer.** The lock, link and private scratch copy
handling should be taken out of the identity module into one Git
configuration writer, used first by `repo identity-set`, enablement and
`committing_identity`. `add_remote` and `remove_remote` call libgit2's
remote functions, which do more than set configuration keys (removal also
deletes references; see ticket `01M4PB7WZN9WNAQ3KXZ19J28KM`), so moving
them is a larger change and should be planned separately. `remote select`
does not write Git configuration and is not a candidate. Independently of
the writer, `add_remote` and `remove_remote` should answer `busy`, not
`internal_error`, when `config.lock` is held: today libgit2's "locked"
error reaches the caller as a Git failure. That is a small change and
could be done with either A or B.

## Open points for planning

- Where exactly the rename sits relative to the commit and the
  `InitializationCommitted` journal step, and what a retry does when it
  finds the commit made and the identity not stored.
- Whether libgit2 writes Git configuration inside `Repository::set_head` or
  `Repository::commit`. Option A's order (lock held across the commit)
  depends on it not doing so. This was assumed from reading enablement's own
  calls and was not confirmed in libgit2's source; if either call does write
  configuration, it would fail against the lock enablement holds.
- Whether a failed enablement must still leave `.git/config` byte-identical
  (true under A; not established under B).
- Whether enablement should refuse, before writing anything, a
  configuration file in which `user.name` or `user.email` appears twice or
  comes from an included file, as `repo identity-set` does.
- The `busy` message does not name `config.lock`, so a user with a stale
  lock is not told which file to remove (Task 15 re-review, note R3).
- `.git/info/exclude` is restored whole in the same function. Git has no
  lock for it. Decide whether to compare before restoring or to accept it.
- `add_remote`, `remove_remote` and `committing_identity` write several
  keys in separate steps and can be left part-done by a failure or a kill;
  the first two answer `internal_error` on a held lock and the third
  answers `recovery_required`.
- Nothing here has been run. Before the fix: a test that changes
  `.git/config` from another writer during a failing enablement and shows
  the loss. After it: the identity writer's tests on both Windows targets.
- The design text that describes enablement's restore, and the sentence
  that the identity command restores "as enablement does", need updating
  with whichever option is chosen.
