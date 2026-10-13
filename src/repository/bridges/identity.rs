//! Local identity: the name, email and initials a repository's own Git
//! configuration holds, and the one operation that sets them.
//!
//! Only the repository's own configuration file is ever written: never
//! the global, the XDG or the system one. The operation works on any
//! repository that has a working directory, whether Manyhands was enabled
//! on it or not: an identity is what enabling, and every save after it,
//! needs first.
//!
//! **No journal row.** The operation is short, holds the repository lease
//! throughout and leaves nothing half done that it can still undo: if a
//! key cannot be written after another was, the configuration file is put
//! back as it was, byte for byte. A process that is killed between two
//! keys leaves the keys it wrote; nothing records that, and the next
//! caller sees the configuration as it is.
//!
//! **A pending operation does not refuse it.** An operation that stopped
//! for want of an identity is taken up again once there is one, so
//! nothing here asks the journal anything. The lease is what keeps it
//! apart from an operation that is running.

use std::path::{Path, PathBuf};

use git2::{Config, ConfigLevel, Repository};

use super::super::{
    FailurePoint, RepositoryError, RepositoryErrorKind, RepositoryOperation, RepositoryService,
    canonical, canonical_repository_root, read_bytes_if_exists, repository_lease, restore_file,
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
    /// from it again as it was written. The file is a scratch one of
    /// this call's own, outside any repository, and is removed.
    fn round_trips(&self) -> Result<bool, RepositoryError> {
        let io = |error| RepositoryError::io(OPERATION, None, error);
        let git = |error| RepositoryError::git(OPERATION, None, error);
        // The handle is let go before Git replaces the file by name.
        let scratch = tempfile::NamedTempFile::new().map_err(io)?.into_temp_path();
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
        let git = |error| RepositoryError::git(OPERATION, Some(root.to_owned()), error);
        let config = local_config(repository).map_err(git)?;
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
        Self::of(&config).map_err(git)
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

impl RepositoryService {
    /// Writes a name and an email, and optionally initials, to the local
    /// Git configuration of the repository at `request.root`, under the
    /// repository lease. No other configuration is written.
    ///
    /// It is idempotent: a configuration that already holds what is
    /// asked is left as it is. When a key cannot be written after
    /// another was, the configuration file is restored to its bytes as
    /// they were, and the error is the one that stopped the write.
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
        if expected.is_some_and(|expected| expected != current) {
            return Err(RepositoryError::new(
                OPERATION,
                Some(root),
                RepositoryErrorKind::ExternalChange,
                "the repository's local identity is not the one that was observed",
            ));
        }
        let path = repository.commondir().join("config");
        let before = read_bytes_if_exists(&path, OPERATION, &root)?;
        let written = (|| {
            let mut config = local_config(&repository).map_err(git)?;
            identity.write(&mut config, &current, || {
                self.check_failure(FailurePoint::AfterLocalIdentityName, OPERATION, &root)
            })
        })();
        match written {
            Ok(()) => Ok(SetLocalIdentityOutcome::Changed(
                identity.written_over(&current),
            )),
            Err(original) => match restore_file(&path, before.as_deref()) {
                Ok(()) => Err(original),
                Err(restoration) => Err(RepositoryError::with_source(
                    OPERATION,
                    Some(root),
                    RepositoryErrorKind::RollbackIncomplete,
                    restoration,
                )),
            },
        }
    }
}
