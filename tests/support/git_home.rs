//! Where libgit2 looks for configuration outside a repository, for a test
//! target whose every test must neither read nor write the developer's
//! own: the system, global, XDG and program-data levels are each sent to
//! a directory of the test process's own.
//!
//! Where libgit2 looks is process-wide, and changing it while another
//! thread uses libgit2 is not safe. So a target that uses this has every
//! one of its tests call `isolated` before it does anything with Git:
//! the first call makes the change, and every other waits for it.
//!
//! The home all the tests of a process share holds no configuration
//! file, and no test may write one there: tests run side by side. A test
//! that needs configuration outside its repository runs in a process of
//! its own, through `own`.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use git2::ConfigLevel;

/// The variable that tells a test it is the child of itself, and names
/// the directory that child's Git is to look in.
const OWN_HOME: &str = "MANYHANDS_TEST_OWN_GIT_HOME";

/// The directories libgit2 is sent to, one for each level it reads
/// outside a repository.
#[derive(Debug)]
pub struct GitHome {
    directory: PathBuf,
}

impl GitHome {
    fn level(&self, name: &str) -> PathBuf {
        self.directory.join(name)
    }

    /// The global configuration file, `~/.gitconfig` as Git names it.
    pub fn global_file(&self) -> PathBuf {
        self.level("global").join(".gitconfig")
    }

    /// The XDG configuration file, `$XDG_CONFIG_HOME/git/config`.
    pub fn xdg_file(&self) -> PathBuf {
        self.level("xdg").join("config")
    }

    /// The system configuration file, `/etc/gitconfig`.
    pub fn system_file(&self) -> PathBuf {
        self.level("system").join("gitconfig")
    }

    /// The three files, as they are on disk: `None` for one that is not
    /// there.
    pub fn files(&self) -> [Option<Vec<u8>>; 3] {
        [self.global_file(), self.xdg_file(), self.system_file()].map(|path| fs::read(path).ok())
    }
}

static HOME: OnceLock<GitHome> = OnceLock::new();

/// Removes the home this process made for itself, when it ends.
extern "C" fn remove_home() {
    if let Some(home) = HOME.get() {
        let _ = fs::remove_dir_all(&home.directory);
    }
}

fn enter(directory: &Path) -> GitHome {
    let home = GitHome {
        directory: directory.to_owned(),
    };
    for (level, name) in [
        (ConfigLevel::System, "system"),
        (ConfigLevel::Global, "global"),
        (ConfigLevel::XDG, "xdg"),
        (ConfigLevel::ProgramData, "program_data"),
    ] {
        let directory = home.level(name);
        fs::create_dir_all(&directory).unwrap();
        // Every test of the target calls `isolated` before it uses
        // libgit2, and all but the first wait here for it: no other use
        // of libgit2 overlaps the change.
        unsafe { git2::opts::set_search_path(level, &directory).unwrap() };
    }
    home
}

/// Sends this process's libgit2 to a home of its own, once, and gives
/// that home. Every test of a target that uses this calls it first.
pub fn isolated() -> &'static GitHome {
    HOME.get_or_init(|| match std::env::var_os(OWN_HOME) {
        // The child of a test: its parent made the directory, and
        // removes it.
        Some(directory) => enter(Path::new(&directory)),
        None => {
            let directory = tempfile::tempdir().unwrap().keep();
            // SAFETY: the function is `extern "C"`, takes nothing and
            // does not unwind.
            unsafe { libc::atexit(remove_home) };
            enter(&directory)
        }
    })
}

/// Runs the calling test, whose path in its test target is `name`, in a
/// process of its own, whose Git home no other test shares: the test may
/// write configuration there. `None` in the parent, once the child has
/// passed; the home in the child, which goes on with the test.
pub fn own(name: &str) -> Option<&'static GitHome> {
    if std::env::var_os(OWN_HOME).is_some() {
        return Some(isolated());
    }
    let home = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(OWN_HOME, home.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "the child ran no test"
    );
    None
}
