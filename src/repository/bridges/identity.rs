//! Local identity: the name, email and initials a repository's own Git
//! configuration holds, and the one operation that sets them.
//!
//! Only the repository's own configuration file is ever written: never
//! the global, the XDG or the system one. The operation works on any
//! repository that has a working directory, whether Manyhands was enabled
//! on it or not: an identity is what enabling, and every save after it,
//! needs first.
//!
//! **One write, under Git's own lock.** Other programs write the same
//! file: `git config`, `git remote add`, an editor's Git integration.
//! They keep each other apart with `config.lock`, a file made beside the
//! configuration file that only one of them can make, and so does this:
//!
//! 1. `config.lock` is made, and the call fails as `RepositoryBusy` if
//!    it is there already. It is given the configuration file's mode.
//!    A `config` that is a symbolic link is followed first, as Git
//!    follows it: the lock is beside the file the link names, and that
//!    file is what is replaced.
//! 2. The configuration file is read, now that no program that keeps to
//!    the lock can change it, and copied to a scratch file. Git sets
//!    the keys in the scratch file. The scratch file and not
//!    `config.lock` itself, because Git edits a configuration file by
//!    making `<file>.lock` beside it and renaming that over the file:
//!    done to `config.lock`, that would replace the lock this call
//!    holds with a file of Git's making and mode. The scratch file is
//!    in a directory of this call's own, in the Git directory, that
//!    only this user can enter: Git gives what it writes the process's
//!    default mode, and the copy holds all the configuration does.
//! 3. What the scratch file then holds is written to `config.lock`,
//!    which is renamed over the configuration file.
//!
//! So every key is set by one rename, or none is: an identity is never
//! half set, in this process or after it is killed. Nothing is ever put
//! back: a call that fails removes its lock and its scratch directory, and
//! the configuration file is as it was because it was never touched. A
//! lock left by a process that died is not removed here; it is its
//! owner's, and Git itself asks the user to remove one.
//!
//! **No journal row.** The operation is short and holds the repository
//! lease throughout; with one rename there is nothing a row could
//! record as begun and not ended.
//!
//! **A pending operation does not refuse it.** An operation that stopped
//! for want of an identity is taken up again once there is one, so
//! nothing here asks the journal anything. The lease is what keeps it
//! apart from an operation that is running.

use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use git2::{Config, ConfigLevel, Repository};

use super::super::{
    FailurePoint, RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
    canonical, canonical_repository_root, repository_lease,
};

const NAME_KEY: &str = "user.name";
const EMAIL_KEY: &str = "user.email";

/// The key of the initials a user set for this repository, in its own
/// Git configuration.
const INITIALS_KEY: &str = "manyhands.initials";

const OPERATION: RepositoryOperation = RepositoryOperation::SetLocalIdentity;

/// What a request does to the repository's initials.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InitialsChange {
    /// The request says nothing of them: they stay as they are.
    Unchanged,
    /// The request clears them: the repository holds none afterwards, and
    /// a short code is composed from the name.
    Clear,
    /// The request sets them: two or three ASCII letters or digits, kept
    /// in lower case.
    Set(String),
}

/// What the repository's own Git configuration holds of an identity. A
/// value held at any other level is not here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalIdentity {
    pub name: Option<String>,
    pub email: Option<String>,
    /// `manyhands.initials`, as it is written: it need not be valid.
    pub initials: Option<String>,
}

/// A request to set a repository's local identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetLocalIdentityRequest {
    /// The repository's root: the working directory of a repository that
    /// is not bare.
    pub root: PathBuf,
    pub name: String,
    pub email: String,
    pub initials: InitialsChange,
    /// What the caller saw the repository's own configuration hold. The
    /// identity is written only while it still holds that; `None` writes
    /// it whatever is there.
    pub expected: Option<LocalIdentity>,
}

/// What `set_local_identity` did. Either way it names what the
/// repository's own configuration holds now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetLocalIdentityOutcome {
    Changed(LocalIdentity),
    /// The configuration already held what the request asks: nothing was
    /// written.
    Unchanged(LocalIdentity),
}

/// The name, the email and the change to the initials of a request, once
/// they are known to be writable: the initials in lower case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValidIdentity {
    pub(crate) name: String,
    pub(crate) email: String,
    pub(crate) initials: InitialsChange,
}

impl ValidIdentity {
    /// Checks the input alone, and writes nothing to any repository.
    ///
    /// A name and an email are each nonempty, hold no control character
    /// and neither begin nor end with white space; the email holds an
    /// `@`; and initials that are set are two or three ASCII letters or
    /// digits. Beyond that, each value is one Git gives back as it was
    /// given and a commit can carry:
    ///
    /// - written to a Git configuration file and read from it again, it
    ///   is equal to the input. The file is a scratch one in the system's
    ///   temporary directory;
    /// - the signature Git makes of the name and the email holds both
    ///   unchanged. Git refuses angle brackets in either, and drops
    ///   white space, commas, colons, semicolons, quotes and backslashes
    ///   from their ends: a value it would change is refused here, since
    ///   the commits made as it would not carry it.
    pub(crate) fn of(
        name: &str,
        email: &str,
        initials: &InitialsChange,
    ) -> Result<Self, RepositoryError> {
        let invalid = |message: &'static str| {
            RepositoryError::new(
                OPERATION,
                None,
                RepositoryErrorKind::InvalidIdentity,
                message,
            )
        };
        for value in [name, email] {
            if value.is_empty() || value.chars().any(char::is_control) || value.trim() != value {
                return Err(invalid(
                    "an identity's name and email must be nonempty, hold no control character \
                     and neither begin nor end with white space",
                ));
            }
        }
        if !email.contains('@') {
            return Err(invalid("an identity's email must hold an @"));
        }
        match git2::Signature::now(name, email) {
            Ok(signature) if signature.name() == Some(name) && signature.email() == Some(email) => {
            }
            Ok(_) | Err(_) => {
                return Err(invalid(
                    "an identity's name and email must be ones a commit carries unchanged",
                ));
            }
        }
        let initials = match initials {
            InitialsChange::Unchanged => InitialsChange::Unchanged,
            InitialsChange::Clear => InitialsChange::Clear,
            InitialsChange::Set(text) => match canonical::normalize_initials(text) {
                Ok(initials) => InitialsChange::Set(initials),
                Err(canonical::InitialsProblem::Invalid) => {
                    return Err(invalid(
                        "initials must be two or three ASCII letters or digits",
                    ));
                }
            },
        };
        let identity = Self {
            name: name.to_owned(),
            email: email.to_owned(),
            initials,
        };
        if !identity.round_trips()? {
            return Err(invalid(
                "an identity must be one Git configuration gives back as it was given",
            ));
        }
        Ok(identity)
    }

    /// Whether the identity, written to a Git configuration file, reads
    /// from it again as it was written. The file is a scratch one in a
    /// directory of this call's own, outside any repository, that only
    /// this user can enter, and both are removed.
    fn round_trips(&self) -> Result<bool, RepositoryError> {
        let io = |error| RepositoryError::io(OPERATION, None, error);
        let git = |error| RepositoryError::git(OPERATION, None, error);
        // In a directory only this user can enter: Git makes the file
        // with the process's default mode, and it holds a name and an
        // email. The directory goes, with the file, when this returns.
        let private = private_directory(&std::env::temp_dir()).map_err(io)?;
        let scratch = private.path().join("config");
        let mut config = Config::open(&scratch).map_err(git)?;
        let nothing = LocalIdentity::default();
        // A value Git cannot write at all is one it does not give back.
        if self.write(&mut config, &nothing, || Ok(())).is_err() {
            return Ok(false);
        }
        let written = LocalIdentity::of(&Config::open(&scratch).map_err(git)?).map_err(git)?;
        Ok(written == self.written_over(&nothing))
    }

    /// Sets the identity's keys in `config`, which holds `current`.
    /// `after_name` is run once the name is set and before the email is.
    fn write(
        &self,
        config: &mut Config,
        current: &LocalIdentity,
        after_name: impl FnOnce() -> Result<(), RepositoryError>,
    ) -> Result<(), RepositoryError> {
        let git = |error| RepositoryError::git(OPERATION, None, error);
        config.set_str(NAME_KEY, &self.name).map_err(git)?;
        after_name()?;
        config.set_str(EMAIL_KEY, &self.email).map_err(git)?;
        match &self.initials {
            InitialsChange::Unchanged => {}
            InitialsChange::Clear => {
                if current.initials.is_some() {
                    config.remove(INITIALS_KEY).map_err(git)?;
                }
            }
            InitialsChange::Set(initials) => {
                config.set_str(INITIALS_KEY, initials).map_err(git)?;
            }
        }
        Ok(())
    }

    /// What the repository's own configuration holds once this is
    /// written over `current`.
    pub(crate) fn written_over(&self, current: &LocalIdentity) -> LocalIdentity {
        LocalIdentity {
            name: Some(self.name.clone()),
            email: Some(self.email.clone()),
            initials: match &self.initials {
                InitialsChange::Unchanged => current.initials.clone(),
                InitialsChange::Clear => None,
                InitialsChange::Set(initials) => Some(initials.clone()),
            },
        }
    }

    /// Whether `current` already holds this: writing it would change
    /// nothing.
    pub(crate) fn held_by(&self, current: &LocalIdentity) -> bool {
        self.written_over(current) == *current
    }
}

/// The repository whose working directory is exactly `root`, and its
/// canonical root. A bare repository, a directory that is no
/// repository's root and a linked worktree are each refused: a linked
/// worktree shares the configuration of the repository that owns it,
/// and that one is named by its own root.
pub(crate) fn identity_repository(root: &Path) -> Result<(Repository, PathBuf), RepositoryError> {
    let (repository, root) = canonical_repository_root(root, OPERATION)?;
    if repository.is_worktree() {
        return Err(RepositoryError::new(
            OPERATION,
            Some(root),
            RepositoryErrorKind::InvalidPath,
            "the selected path is a linked worktree, not the repository's own root",
        ));
    }
    Ok((repository, root))
}

/// The repository's own configuration, and nothing of any other level.
fn local_config(repository: &Repository) -> Result<Config, git2::Error> {
    repository.config()?.open_level(ConfigLevel::Local)
}

/// One value of the repository's own configuration, or `None` when it
/// holds none under `key`.
fn local_value(config: &Config, key: &str) -> Result<Option<String>, git2::Error> {
    match config.get_string(key) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// `manyhands.initials` in the repository's own configuration, and at no
/// other level: a value a user set for another repository, or for every
/// one, is not this repository's.
pub(crate) fn local_initials(repository: &Repository) -> Result<Option<String>, git2::Error> {
    local_value(&local_config(repository)?, INITIALS_KEY)
}

impl LocalIdentity {
    /// Reads the repository's own configuration. Nothing is written and
    /// no lease is taken.
    ///
    /// Each of the three keys must be said at most once, and in the
    /// configuration file itself. One that is there twice, or that a
    /// file the configuration includes holds, is refused as
    /// `RecoveryRequired`: Git will not set such a key, what it "holds"
    /// is not one value, and the file is the user's to put right.
    pub(crate) fn own(repository: &Repository, root: &Path) -> Result<Self, RepositoryError> {
        let config = local_config(repository)
            .map_err(|error| RepositoryError::git(OPERATION, Some(root.to_owned()), error))?;
        Self::said_once(&config, root)
    }

    /// What `config`, a repository's own configuration or a copy of it,
    /// holds of an identity, each key of which it must say at most once
    /// and itself; see `own`.
    fn said_once(config: &Config, root: &Path) -> Result<Self, RepositoryError> {
        let git = |error| RepositoryError::git(OPERATION, Some(root.to_owned()), error);
        for key in [NAME_KEY, EMAIL_KEY, INITIALS_KEY] {
            let mut said = 0;
            let mut included = false;
            let mut entries = config.multivar(key, None).map_err(git)?;
            while let Some(entry) = entries.next() {
                let entry = entry.map_err(git)?;
                said += 1;
                included |= entry.include_depth() > 0;
            }
            if said > 1 || included {
                return Err(RepositoryError::new(
                    OPERATION,
                    Some(root.to_owned()),
                    RepositoryErrorKind::RecoveryRequired,
                    "the repository's own Git configuration holds a key of the identity more \
                     than once, or in an included file",
                ));
            }
        }
        Self::of(config).map_err(git)
    }

    /// What `config` holds of an identity.
    fn of(config: &Config) -> Result<Self, git2::Error> {
        Ok(Self {
            name: local_value(config, NAME_KEY)?,
            email: local_value(config, EMAIL_KEY)?,
            initials: local_value(config, INITIALS_KEY)?,
        })
    }
}

/// A new directory in `parent` that only this user can enter, removed
/// with what it holds when the value is dropped. Git gives the files it
/// writes the process's default mode, whatever mode the file they copy
/// has: what is made in here is closed to others all the same.
fn private_directory(parent: &Path) -> std::io::Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("manyhands-identity-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir_in(parent)
}

/// The bytes of the file at `path`, or none for a file that is not
/// there: Git reads a configuration file that is missing as an empty one.
fn bytes_or_none(path: &Path) -> std::io::Result<Vec<u8>> {
    match std::fs::read(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        read => read,
    }
}

/// The file the repository's own configuration is in, under `directory`,
/// the repository's common Git directory.
///
/// A `config` that is a symbolic link is followed to what it names, as
/// Git follows it: the lock is taken beside that file and that file is
/// replaced, so the link stays a link and locks out the programs that
/// write through it. A link that names nothing is refused as
/// `RecoveryRequired`: there is no file to lock, and making one where a
/// broken link points is the user's to decide. A `config` that is not
/// there at all is where one will be made.
fn config_file(directory: &Path, root: &Path) -> Result<PathBuf, RepositoryError> {
    let io = |error| RepositoryError::io(OPERATION, Some(root.to_owned()), error);
    let path = directory.join("config");
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(path),
        Err(error) => return Err(io(error)),
        Ok(_) => {}
    }
    match std::fs::canonicalize(&path) {
        Ok(resolved) => Ok(resolved),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(RepositoryError::new(
            OPERATION,
            Some(root.to_owned()),
            RepositoryErrorKind::RecoveryRequired,
            "the repository's Git configuration is a link to a file that is not there",
        )),
        Err(error) => Err(io(error)),
    }
}

/// Git's lock on a configuration file: `<file>.lock` beside it, which
/// only one program can make. Whoever holds it is the one writer of the
/// file, and writes by renaming the lock over it.
///
/// Dropped without that, the lock is removed and the file is as it was.
struct ConfigLock {
    config: PathBuf,
    lock: PathBuf,
    /// The lock file, open, until it is renamed or removed.
    file: Option<File>,
    root: PathBuf,
}

impl ConfigLock {
    /// Makes the lock of `config`, the file itself and no link to it,
    /// with the mode `config` has, or the process's default mode for a
    /// `config` that is not there.
    /// `RepositoryBusy` when the lock is there already: another program
    /// is writing the file, or one died and left it.
    fn take(config: &Path, root: &Path) -> Result<Self, RepositoryError> {
        let io = |error| RepositoryError::io(OPERATION, Some(root.to_owned()), error);
        let mut name = config.as_os_str().to_owned();
        name.push(".lock");
        let lock = PathBuf::from(name);
        let permissions = match std::fs::metadata(config) {
            Ok(metadata) => Some(metadata.permissions()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(io(error)),
        };
        let file = match File::options().write(true).create_new(true).open(&lock) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(RepositoryError::new(
                    OPERATION,
                    Some(root.to_owned()),
                    RepositoryErrorKind::RepositoryBusy,
                    "the repository's Git configuration is locked by another program",
                ));
            }
            Err(error) => return Err(io(error)),
        };
        // From here on the lock is this call's, and is removed with it.
        let held = Self {
            config: config.to_owned(),
            lock,
            file: Some(file),
            root: root.to_owned(),
        };
        // The file was made with the process's default mode; the one it
        // replaces may be more open or more closed than that. Only Unix
        // has a mode to keep.
        #[cfg(unix)]
        if let Some(permissions) = permissions {
            std::fs::set_permissions(&held.lock, permissions).map_err(io)?;
        }
        #[cfg(not(unix))]
        let _ = permissions;
        Ok(held)
    }

    /// Replaces the configuration file with `source`, by one rename of
    /// the lock over it. On an `Err` the file is as it was.
    fn replace(mut self, source: &[u8]) -> Result<(), RepositoryError> {
        let root = self.root.clone();
        let io = |error| RepositoryError::io(OPERATION, Some(root.clone()), error);
        if let Some(mut file) = self.file.take() {
            file.write_all(source).map_err(io)?;
            file.sync_all().map_err(io)?;
            // Closed before it is renamed, which Windows requires.
        }
        std::fs::rename(&self.lock, &self.config).map_err(io)?;
        // The lock is gone with the rename. A lock that is there from
        // now on is another program's, and is not this call's to remove.
        self.lock = PathBuf::new();
        Ok(())
    }
}

impl Drop for ConfigLock {
    fn drop(&mut self) {
        drop(self.file.take());
        if !self.lock.as_os_str().is_empty() {
            let _ = std::fs::remove_file(&self.lock);
        }
    }
}

impl RepositoryService {
    /// Has `hook` run once in the next `set_local_identity` on this
    /// service that writes, while it holds the lock on the repository's
    /// configuration file: after the name is set in what it will write,
    /// and before anything is put in place.
    #[doc(hidden)]
    pub fn set_local_identity_lock_hook_for_testing(&self, hook: impl FnOnce() + Send + 'static) {
        *self
            .identity_lock_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(Box::new(hook));
    }

    fn run_identity_lock_hook(&self) {
        let hook = self
            .identity_lock_hook
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }

    /// Writes a name and an email, and optionally initials, to the local
    /// Git configuration of the repository at `request.root`, under the
    /// repository lease. No other configuration is written.
    ///
    /// It is idempotent: a configuration that already holds what is
    /// asked is left as it is. Otherwise every key is set by one
    /// replacement of the configuration file under Git's own lock on
    /// it, or none is; see the module's description. A call that fails
    /// has not touched the file.
    ///
    /// `RepositoryBusy` when the configuration file is locked;
    /// `ExternalChange` when it does not hold what `request.expected`
    /// says, compared under the lock; `RecoveryRequired` when it says a
    /// key of the identity twice or in an included file.
    pub fn set_local_identity(
        &self,
        request: SetLocalIdentityRequest,
    ) -> Result<SetLocalIdentityOutcome, RepositoryError> {
        let SetLocalIdentityRequest {
            root,
            name,
            email,
            initials,
            expected,
        } = request;
        let identity = ValidIdentity::of(&name, &email, &initials)?;
        let (repository, root) = identity_repository(&root)?;
        let git = |error| RepositoryError::git(OPERATION, Some(root.clone()), error);
        let _lease = repository_lease(&repository, &root, OPERATION)?;
        let current = LocalIdentity::own(&repository, &root)?;
        if identity.held_by(&current) {
            return Ok(SetLocalIdentityOutcome::Unchanged(current));
        }
        let changed = || {
            RepositoryError::new(
                OPERATION,
                Some(root.clone()),
                RepositoryErrorKind::ExternalChange,
                "the repository's local identity is not the one that was observed",
            )
        };
        let io = |error| RepositoryError::io(OPERATION, Some(root.clone()), error);
        let directory = repository.commondir();
        let path = config_file(directory, &root)?;
        let lock = ConfigLock::take(&path, &root)?;
        // No program that keeps to the lock changes the file from here
        // on. What it holds now is what is edited: a change to any other
        // key that landed before the lock is kept.
        let source = bytes_or_none(&path).map_err(io)?;
        // The file itself is asked what it holds and whether it says
        // each key once: what it includes is found from where it is.
        let locked = LocalIdentity::said_once(&Config::open(&path).map_err(git)?, &root)?;
        if identity.held_by(&locked) {
            return Ok(SetLocalIdentityOutcome::Unchanged(locked));
        }
        // The one compare with what the caller saw, made where the file
        // can no longer change.
        if expected.is_some_and(|expected| expected != locked) {
            return Err(changed());
        }
        // Git sets the keys in a copy, in a directory of this call's own
        // that only this user can enter: the copy holds everything the
        // file does, and Git gives it the process's default mode. The
        // directory is in the Git directory, and goes with the copy on
        // every way out of here.
        let private = private_directory(directory).map_err(io)?;
        let scratch = private.path().join("config");
        std::fs::write(&scratch, &source).map_err(io)?;
        let mut config = Config::open(&scratch).map_err(git)?;
        identity.write(&mut config, &locked, || {
            self.run_identity_lock_hook();
            self.check_failure(FailurePoint::AfterLocalIdentityName, OPERATION, &root)
        })?;
        // What Git gives back of what it was given, which is what the
        // repository will hold.
        let written = LocalIdentity::of(&Config::open(&scratch).map_err(git)?).map_err(git)?;
        if written != identity.written_over(&locked) {
            return Err(RepositoryError::new(
                OPERATION,
                Some(root),
                RepositoryErrorKind::InvalidIdentity,
                "the repository's Git configuration does not give the identity back as given",
            ));
        }
        let edited = std::fs::read(&scratch).map_err(io)?;
        // Immediately before the rename: a program that does not keep to
        // the lock has not written the file meanwhile.
        if bytes_or_none(&path).map_err(io)? != source {
            return Err(changed());
        }
        lock.replace(&edited)?;
        Ok(SetLocalIdentityOutcome::Changed(written))
    }
}
