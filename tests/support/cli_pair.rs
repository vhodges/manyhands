//! Two clones of one SSH remote, each a Manyhands repository of its own,
//! for the CLI's collaboration journeys (R-24).
//!
//! The server's primary is seeded before either clone is made (R-33), with
//! the tickets, so both clones index them, and with a configuration that
//! already selects `origin` as the publication remote: a repository enabled
//! elsewhere. Neither clone's `remote select` then commits. A committing
//! `remote select` leaves the worktree's Git index stale, which refuses the
//! next synchronization as `worktree_not_clean`: ticket
//! `01M669FJR27FRHCNPWAMD6VH4V`, "Remote selection must leave the Git index
//! current for the configuration it commits".
//!
//! Everything that makes a clone a Manyhands repository goes through the
//! binary, one `checked` run per step: a selected key the server allows,
//! an identity, enablement, the publication remote and the approved host.
//! Git builds only history: the seed on the remote, and the clones
//! themselves. Neither clone has an editing context yet: each case makes
//! its own with a first write.
//!
//! Each clone selects its key before it is enabled. Selecting a key fences
//! every repository whose publication remote is configured already, and a
//! fenced repository cannot tell a context it never published from one
//! deleted on the server (`history_unknown`).
//!
//! Mounted only from `tests/cli_remote.rs` (fact 59).

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::{
    checked, confirm, host_input, prepare, repository,
    ssh_remote::{FixtureError, SshRemoteFixture, fixed},
    support::cli::CliEnv,
};

/// The ticket the clones work on.
pub const ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAW";
/// Another ticket on primary, for a request sent again with another target.
pub const OTHER_ITEM: &str = "01ARZ3NDEKTSV4RRFFQ69G5FAX";
/// The editing branch of `ITEM`.
pub const CONTEXT: &str = "refs/heads/manyhands/ticket/01ARZ3NDEKTSV4RRFFQ69G5FAW";
pub const PRIMARY: &str = "refs/heads/main";
/// The configuration of a repository enabled elsewhere, `origin` its
/// publication remote, as enablement and `remote select` write it.
const CONFIGURATION: &str =
    "format_version = 1\nprimary_branch = \"main\"\npublication_remote = \"origin\"\n";

/// One clone: its CLI environment, with a data directory and a home of its
/// own, and its root.
pub struct CliClone {
    pub env: CliEnv,
    pub root: PathBuf,
}

pub struct CliPair {
    pub server: SshRemoteFixture,
    pub a: CliClone,
    pub b: CliClone,
    /// Holds both clones and their data directories while the pair lives.
    _directory: tempfile::TempDir,
}

impl CliPair {
    pub fn new() -> Result<Self, FixtureError> {
        let server = SshRemoteFixture::start()?;
        crate::private_fixture(&server);
        seed(&server)?;
        let directory = fixed(tempfile::tempdir())?;
        let a = CliClone::new(&server, directory.path(), "a", "Clone A")?;
        let b = CliClone::new(&server, directory.path(), "b", "Clone B")?;
        Ok(Self {
            server,
            a,
            b,
            _directory: directory,
        })
    }

    /// The remote's repository.
    pub fn bare(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open_bare(self.server.repository_path()))
    }

    /// The commit `reference` names on the remote, when it exists there.
    pub fn served(&self, reference: &str) -> Result<Option<git2::Oid>, FixtureError> {
        Ok(self.bare()?.refname_to_id(reference).ok())
    }
}

impl CliClone {
    /// Clones the remote into `directory`, beside a data directory of its
    /// own, and makes the clone a Manyhands repository through the binary.
    /// `name` is the clone's identity, and the label of its key.
    fn new(
        server: &SshRemoteFixture,
        directory: &Path,
        letter: &str,
        name: &str,
    ) -> Result<Self, FixtureError> {
        let clone = Self::cloned(server, directory, letter)?;
        clone.select_a_key(server, &format!("{name} key"))?;
        clone.confirmed(
            &[
                "repo",
                "identity-set",
                "--repo",
                clone.root(),
                "--input",
                "-",
            ],
            &json!({"name": name, "email": format!("clone-{letter}@example.invalid")}),
        );
        // The primary is enabled already: enablement registers it.
        clone.confirmed(
            &["repo", "enable", "--repo", clone.root(), "--input", "-"],
            &json!({"primary_branch": "main"}),
        );
        // The configuration the clone was made with selects `origin`.
        clone.json(
            &[
                "remote",
                "select",
                "--repo",
                clone.root(),
                "--input",
                "-",
                "--request-id",
                &new_request(),
            ],
            &fixed(serde_json::to_vec(&json!({"name": "origin"})))?,
            "already_applied",
            0,
        );
        let host = host_input(server);
        let preview = prepare(&clone.env, &clone.root, "approve", &host);
        confirm(&clone.env, &clone.root, "approve", &host, &preview, "ok", 0);
        Ok(clone)
    }

    /// A Git clone of the remote, whose `origin` is the remote.
    fn cloned(
        server: &SshRemoteFixture,
        directory: &Path,
        letter: &str,
    ) -> Result<Self, FixtureError> {
        let root = directory.join(format!("clone-{letter}"));
        let mut fetch = git2::FetchOptions::new();
        fetch.remote_callbacks(callbacks(server)?);
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch);
        fixed(builder.clone(&server.url(), &root))?;
        let env = CliEnv::new(directory.join(format!("data-{letter}")));
        // Line endings as the clones write them, on every platform.
        fixed(std::fs::write(
            env.home.path().join(".gitconfig"),
            "[core]\n\tautocrlf = false\n",
        ))?;
        Ok(Self { env, root })
    }

    /// Imports the key the server allows into the clone's own key store,
    /// and selects it.
    fn select_a_key(&self, server: &SshRemoteFixture, label: &str) -> Result<(), FixtureError> {
        let key = self.json(
            &[
                "key",
                "import",
                "--input",
                "-",
                "--request-id",
                &new_request(),
            ],
            &fixed(serde_json::to_vec(&json!({
                "label": label,
                "private_key_path": server.client_key_path(),
            })))?,
            "ok",
            0,
        );
        self.json(
            &[
                "key",
                "select",
                "--key-id",
                key["data"]["id"].as_str().ok_or(FixtureError)?,
                "--request-id",
                &new_request(),
            ],
            &[],
            "ok",
            0,
        );
        Ok(())
    }

    pub fn root(&self) -> &str {
        self.root.to_str().unwrap()
    }

    /// One JSON-mode run, its code and exit checked.
    pub fn json(&self, args: &[&str], input: &[u8], code: &str, status: i32) -> Value {
        checked(&self.env, args, input, code, status, None)
    }

    /// A confirmed verb: its preview, then its confirmation.
    fn confirmed(&self, args: &[&str], input: &Value) -> Value {
        let bytes = serde_json::to_vec(input).unwrap();
        let mut prepared = args.to_vec();
        prepared.push("--prepare");
        let preview = self.json(&prepared, &bytes, "ok", 0);
        let request = new_request();
        let mut confirmed = args.to_vec();
        confirmed.extend([
            "--request-id",
            &request,
            "--confirm",
            preview["data"]["confirmation_id"].as_str().unwrap(),
        ]);
        self.json(&confirmed, &bytes, "ok", 0)
    }

    pub fn repo(&self) -> Result<git2::Repository, FixtureError> {
        fixed(git2::Repository::open(&self.root))
    }

    /// The commit `reference` names in the clone, when it exists.
    pub fn tip(&self, reference: &str) -> Result<Option<git2::Oid>, FixtureError> {
        Ok(self.repo()?.refname_to_id(reference).ok())
    }

    /// The editing worktree of `ITEM`.
    pub fn context(&self) -> PathBuf {
        self.root.join(".manyhands/worktrees").join(ITEM)
    }
}

/// Git's callbacks for the server as a clone's own Git reaches it: the key
/// the server allows, which is not protected, and the server's host key.
pub fn callbacks(server: &SshRemoteFixture) -> Result<git2::RemoteCallbacks<'_>, FixtureError> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(|_, user, _| {
        git2::Cred::ssh_key(
            user.unwrap_or("fixture"),
            None,
            server.client_key_path(),
            None,
        )
    });
    let host = fixed(server.host_public_key().to_bytes())?;
    callbacks.certificate_check(move |certificate, _| {
        if certificate.as_hostkey().and_then(|key| key.hostkey()) == Some(host.as_slice()) {
            Ok(git2::CertificateCheckStatus::CertificateOk)
        } else {
            Err(git2::Error::from_str("fixture host mismatch"))
        }
    });
    Ok(callbacks)
}

fn new_request() -> String {
    repository::RequestId::new().to_string()
}

fn ticket_path(id: &str) -> String {
    format!(".manyhands/tickets/{id}/ticket.md")
}

/// A ticket's canonical source.
fn ticket_source(id: &str, title: &str) -> String {
    format!(
        "---\nmanyhands_managed: true\nmanyhands_kind: ticket\nid: {id}\ntitle: {title}\n\
         type: task\nstatus: open\n---\nBody of {title}.\n"
    )
}

/// R-33: the configuration and both tickets on the remote's primary, in
/// one commit, before any clone is made.
fn seed(server: &SshRemoteFixture) -> Result<(), FixtureError> {
    let bare = fixed(git2::Repository::open_bare(server.repository_path()))?;
    let parent = fixed(bare.find_commit(fixed(bare.refname_to_id(PRIMARY))?))?;
    let mut index = fixed(git2::Index::new())?;
    fixed(index.read_tree(&fixed(parent.tree())?))?;
    for (path, bytes) in [
        (
            ".manyhands/config.toml".to_owned(),
            CONFIGURATION.to_owned(),
        ),
        (ticket_path(ITEM), ticket_source(ITEM, "Shared")),
        (ticket_path(OTHER_ITEM), ticket_source(OTHER_ITEM, "Other")),
    ] {
        fixed(index.add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: fixed(bare.blob(bytes.as_bytes()))?,
            flags: 0,
            flags_extended: 0,
            path: path.into_bytes(),
        }))?;
    }
    let tree = fixed(bare.find_tree(fixed(index.write_tree_to(&bare))?))?;
    let signature = fixed(git2::Signature::now("Fixture", "fixture@example.invalid"))?;
    fixed(bare.commit(
        Some(PRIMARY),
        &signature,
        &signature,
        "Seed a Manyhands repository with two tickets",
        &tree,
        &[&parent],
    ))?;
    Ok(())
}
