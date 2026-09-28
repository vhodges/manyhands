# Rust Git Research

> Status: unverified external research. Validate crate versions and APIs before implementation; the examples use direct `gpui` imports while this repository uses `gpui-kit` re-exports.

## 1. Git Library Options

### Core Libraries

- [`git2`](https://crates.io/crates/git2) provides Rust bindings to libgit2. The research identifies it as mature, feature-complete, and heavily tested, while noting its C dependency can lengthen builds and requires a C toolchain for cross-compilation. Sources: [Reddit discussion](https://www.reddit.com/r/rust/comments/1gshlja/how_to_use_git_in_a_rust_application/), [crates.io Git keyword](https://crates.io/keywords/git).
- [`gix`](https://crates.io/crates/gix), from [gitoxide](https://github.com/gitoxidelabs/gitoxide), is a pure-Rust Git implementation focused on performance, memory safety, and multithreading. The research describes it as rapidly maturing and suitable for many everyday programmatic operations. Sources: [gitoxide crate](https://crates.io/crates/gitoxide), [Reddit discussion](https://www.reddit.com/r/rust/comments/1gshlja/how_to_use_git_in_a_rust_application/).

### Comparison

| Feature | `git2` | `gix` / gitoxide |
| --- | --- | --- |
| Implementation | C bindings to libgit2 | Pure Rust |
| Maturity | Long-standing and production-ready | Rapidly maturing; production-viable for many tasks |
| Performance | Standard libgit2 performance | Research claims very high performance |
| Compilation | Requires C build tools and system libraries | Native Cargo compilation |
| Safety | Constrained by a C boundary | Rust memory-safety guarantees |

### Specialized Crates

- [`git-version`](https://crates.io/crates/git-version) embeds the current Git commit hash or version string at build time, for example in a `--version` flag.
- [`git-config`](https://crates.io/crates/git-config) is a gitoxide ecosystem crate for parsing and editing `.gitconfig` files.

### Research Recommendation

- Choose `git2` for immediate feature completeness and established stability.
- Choose `gix` for a pure-Rust, cross-compilation-friendly stack where performance is a priority.
- For basic operations, consider invoking the installed Git CLI with `std::process::Command` to minimize integration work. Sources: [Reddit discussion](https://www.reddit.com/r/rust/comments/1gshlja/how_to_use_git_in_a_rust_application/), [crates.io Git keyword](https://crates.io/keywords/git), [gitoxide crate](https://crates.io/crates/gitoxide).

## 2. `git2` Backend Architecture

The research recommends `git2` for a GPUI desktop application that needs Git worktrees plus merge, fetch, and push operations. It cites GPUI Kit and GPUI as the surrounding UI stack. Sources: [GPUI Kit](https://github.com/longbridge/gpui-kit), [GPUI crate](https://crates.io/crates/gpui).

### Why `git2` Over `gix` for This Scope

The research describes `gix` as fast and native to Rust, but incomplete for some write and porcelain operations needed by this feature set.

- **Worktrees:** `gix` has read-oriented worktree infrastructure, while the research identifies libgit2's `git2::Worktree` as the more complete API for creating, pruning, and modifying worktrees.
- **Merging:** The research identifies `git2` as robust and well tested for programmatic multi-file conflict resolution. Sources: [jj issue](https://github.com/jj-vcs/jj/issues/2316), [gitoxide releases](https://github.com/gitoxidelabs/gitoxide/releases), [worktree article](https://medium.com/@pererikbergman/the-ultimate-guide-to-git-worktrees-from-daily-dev-to-ai-agents-2b39e63a359d).

### Mapping Operations to `git2`

- **Clone:** `git2::Repository::clone()`.
- **Fetch, pull, and push:** `git2::Remote`, with runtime authentication callbacks for SSH agents and credential helpers. Source: [Hacker News discussion](https://news.ycombinator.com/item?id=44588584).
- **Branches:** `repo.branch()` for creation and deletion; `repo.checkout_tree()` for switching.
- **Commits:** stage through `repo.index()`, build a `git2::Tree`, then call `repo.commit()`.
- **Worktrees:** `git2::Worktree` and `repo.worktree()` to create a detached checkout at a chosen path.
- **Working-tree changes:** `repo.diff_tree_to_workdir_with_index()` for modified, untracked, and deleted paths relative to `HEAD`.

### GPUI Background Execution

GPUI work must avoid blocking its main UI thread. Because `git2` operations are synchronous, run them on the GPUI background executor and return a task.

```rust
use gpui::*;
use git2::Repository;
use std::path::PathBuf;

fn async_git_commit(
    cx: &mut AppContext,
    repo_path: PathBuf,
    message: String,
) -> Task<Result<git2::Oid, git2::Error>> {
    cx.background_executor().spawn(async move {
        // This runs isolated on a background thread pool.
        let repo = Repository::open(repo_path)?;

        // Build the index, tree, and signature here.
        let commit_id = repo.commit(
            Some("HEAD"),
            &author,
            &committer,
            &message,
            &tree,
            &[&parent_commit],
        )?;

        Ok(commit_id)
    })
}
```

### Thread Boundaries and UI Updates

The research states that `git2::Repository`, `git2::Commit`, and `git2::Tree` are not `Send` or `Sync`. Do not retain Git handles in GPUI model state or transfer them between threads. Store a repository-root `PathBuf`, open the repository inside each background task, and return only plain data such as strings, paths, branch lists, and object IDs to the UI.

When the task completes, update a GPUI model on the main UI context to trigger the rerender:

```rust
cx.spawn(|mut cx| async move {
    match async_git_commit(&mut cx, path, msg).await {
        Ok(oid) => {
            cx.update(|cx| {
                // Notify the UI that the document or issue status was saved to Git.
            })
            .unwrap();
        }
        Err(e) => println!("Git error: {:?}", e),
    }
})
.detach();
```

## 3. Multi-Platform CI/CD

The research recommends GitHub Actions native Linux, macOS, and Windows runners rather than cross-compiling on a local Linux host. It notes that `git2` links native libraries including libgit2, OpenSSL, and libssh2, so CI must install platform prerequisites before building. Sources: [cargo-cross action](https://github.com/marketplace/actions/cargo-cross-action), [Rust build action](https://github.com/marketplace/actions/build-rust-projects-with-cross).

### Proposed GitHub Actions Matrix

```yaml
name: Multi-Platform Release Builds

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    name: Build on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]

    steps:
      - name: Checkout Code
        uses: actions/checkout@v4

      - name: Set up Rust Toolchain
        uses: dtolnay/rust-toolchain@stable

      # Install native platform prerequisites for libgit2 and GPUI.
      - name: Install Dependencies (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          # libgit2 requires cmake/pkg-config. OpenSSL is needed for fetch/push protocols.
          # GPUI also requires X11 and Vulkan dependencies on Linux.
          sudo apt-get install -y cmake pkg-config libssl-dev libx11-dev libxkbcommon-dev libvulkan-dev

      - name: Install Dependencies (macOS)
        if: matrix.os == 'macos-latest'
        run: |
          # macOS includes modern security/SSH tools, but cmake might be missing.
          brew install cmake

      - name: Install Dependencies (Windows)
        if: matrix.os == 'windows-latest'
        run: |
          # Git2-rs uses Windows' Schannel crypto backend.
          choco install cmake --installargs 'ADD_CMAKE_TO_PATH=System'

      - name: Cargo Cache
        uses: Swatinem/rust-cache@v2

      - name: Build Release Binaries
        run: cargo build --release
```

### `git2` Compilation Notes

The research proposes this dependency declaration to use vendored OpenSSL:

```toml
[dependencies]
git2 = { version = "0.19", features = ["vendored-openssl"] }
```

It claims vendored OpenSSL compiles an internal copy on Linux while macOS and Windows use their native encryption facilities. Validate this behavior against the selected `git2` version and its transitive `libgit2-sys` feature set before adopting it.

### GPUI Build Dependencies

The Linux CI runner requires graphics headers, such as X11 and Vulkan development packages, to compile GPUI even when the workflow never opens a native window.

## 4. SSH Authentication with `git2`

The research recommends local SSH-agent authentication for a Git-backed, local-first application aimed at technical users. It proposes explicitly configuring `git2::RemoteCallbacks` so fetch, pull, and push operations can respond to Git authentication requests without blocking the UI.

### SSH Callback Pattern

```rust
use git2::{Credential, CredentialType, RemoteCallbacks};

/// Sets up remote callbacks wired to use the system's local ssh-agent.
fn setup_ssh_callbacks<'a>() -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();

    callbacks.credentials(|_url, username_from_url, allowed_types| {
        // Verify that the remote host is asking for SSH credentials.
        if allowed_types.contains(CredentialType::SSH_KEY) {
            // Attempt to connect to the running ssh-agent on the machine.
            let username = username_from_url.unwrap_or("git");
            Credential::ssh_key_from_agent(username)
        } else {
            Err(git2::Error::from_str(
                "Remote repository requested non-SSH credentials (for example, HTTPS).",
            ))
        }
    });

    callbacks
}
```

### Background Fetch Operation

Network operations can block indefinitely, so the research requires moving them to the GPUI background executor. Open the repository within the task to avoid Git-handle thread-safety issues.

```rust
use gpui::{AppContext, Task};
use git2::{FetchOptions, Repository};
use std::path::PathBuf;

pub fn async_git_fetch(
    cx: &mut AppContext,
    repo_path: PathBuf,
    remote_name: String,
) -> Task<Result<(), String>> {
    cx.background_executor().spawn(async move {
        let repo = Repository::open(&repo_path)
            .map_err(|e| format!("Failed to open repository: {}", e))?;

        let mut remote = repo
            .find_remote(&remote_name)
            .map_err(|e| format!("Remote '{}' not found: {}", remote_name, e))?;

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(setup_ssh_callbacks());

        // Download progress can also be exposed through callbacks.
        remote
            .fetch(
                &["refs/heads/*:refs/remotes/origin/*"],
                Some(&mut fetch_options),
                None,
            )
            .map_err(|e| format!("Git fetch failed: {}", e))?;

        Ok(())
    })
}
```

### Desktop SSH Edge Cases

1. **Agent differences:** The research states that Linux and macOS use the standard `SSH_AUTH_SOCK` environment variable. It describes Windows as using the OpenSSH Authentication Agent service and named pipes, with the user responsible for enabling that service. Validate the libgit2 behavior on current Windows targets.
2. **Fallback key files:** If the agent fails, the research suggests trying `~/.ssh/id_ed25519` after resolving either `HOME` or `USERPROFILE`:

   ```rust
   Credential::ssh_key_from_agent(username).or_else(|_| {
       let home = std::env::var("HOME")
           .map(PathBuf::from)
           .or_else(|_| std::env::var("USERPROFILE").map(PathBuf::from))
           .map_err(|_| git2::Error::from_str("Could not find home directory"))?;

       let private_key = home.join(".ssh").join("id_ed25519");
       Credential::ssh_key(username, None, &private_key, None)
   })
   ```

3. **Passphrases:** If an encrypted key is not cached by the agent, return credential failures to the GPUI main thread, present a passphrase modal, and retry with `Credential::ssh_key`. Treat passphrases as sensitive data: do not log, persist, or retain them beyond the retry.

## 5. Tracking Uncommitted Changes

The research treats change tracking as the basis of the UI sidebar: users should see modified, untracked, deleted, and conflicted files before saving or committing. Extract Git data into plain Rust types so it can cross from a background Git task to the UI model.

### Domain Types

```rust
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitFileStatus {
    Modified,
    Added,
    Deleted,
    Untracked,
    Conflicted,
}

#[derive(Clone, Debug)]
pub struct UncommittedChange {
    pub path: PathBuf,
    pub status: GitFileStatus,
}
```

### Background Status Scan

The research proposes using `Repository::statuses` with untracked files included, and running that scan on the GPUI background executor.

```rust
use git2::{Repository, StatusOptions};
use gpui::{AppContext, Task};
use std::path::PathBuf;

pub fn scan_workspace_changes(
    cx: &mut AppContext,
    repo_path: PathBuf,
) -> Task<Result<Vec<UncommittedChange>, String>> {
    cx.background_executor().spawn(async move {
        let repo = Repository::open(&repo_path)
            .map_err(|e| format!("Failed to open repo: {}", e))?;

        let mut changes = Vec::new();

        // Configure Git status options to include untracked files.
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);

        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| format!("Failed to get repository status: {}", e))?;

        for entry in statuses.iter() {
            let path = PathBuf::from(entry.path().unwrap_or(""));
            let status_flags = entry.status();

            let status = if status_flags.is_conflicted() {
                GitFileStatus::Conflicted
            } else if status_flags.is_wt_new() || status_flags.is_index_new() {
                GitFileStatus::Untracked
            } else if status_flags.is_wt_modified() || status_flags.is_index_modified() {
                GitFileStatus::Modified
            } else if status_flags.is_wt_deleted() || status_flags.is_index_deleted() {
                GitFileStatus::Deleted
            } else {
                continue; // Skip unmodified and ignored records.
            };

            changes.push(UncommittedChange { path, status });
        }

        Ok(changes)
    })
}
```

### GPUI Model Integration

The research proposes storing the root path, change list, and in-progress flag in a model. `refresh` avoids duplicate scans, sets a loading state, starts the background task, and updates model data once the task completes.

```rust
use gpui::{AppContext, Context, Model, ModelContext};

pub struct WorkspaceGitState {
    repo_path: PathBuf,
    pub uncommitted_changes: Vec<UncommittedChange>,
    pub is_scanning: bool,
}

impl WorkspaceGitState {
    pub fn new(repo_path: PathBuf, cx: &mut AppContext) -> Model<Self> {
        cx.new_model(|_cx| Self {
            repo_path,
            uncommitted_changes: Vec::new(),
            is_scanning: false,
        })
    }

    /// Triggers a background scan and updates the model's state safely.
    pub fn refresh(&mut self, cx: &mut ModelContext<Self>) {
        if self.is_scanning {
            return; // Avoid duplicate concurrent scans.
        }

        self.is_scanning = true;
        cx.notify(); // Let the UI show a loading spinner.

        let repo_path = self.repo_path.clone();

        cx.spawn(|model, mut cx| async move {
            let result = scan_workspace_changes(&mut cx, repo_path).await;

            // Return to the main UI thread to mutate state.
            model
                .update(&mut cx, |this, cx| {
                    this.is_scanning = false;
                    match result {
                        Ok(changes) => {
                            this.uncommitted_changes = changes;
                        }
                        Err(err) => {
                            eprintln!("Git Scan Error: {}", err);
                        }
                    }
                    cx.notify();
                })
                .ok();
        })
        .detach();
    }
}
```

### Sidebar View

The research uses the model's change list to render a sidebar with a status label and colored badge for each changed path.

```rust
use gpui::{prelude::*, Render, ViewContext, VisualContext};

pub struct GitSidebarView {
    state: Model<WorkspaceGitState>,
}

impl Render for GitSidebarView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state.read(cx);

        div()
            .flex()
            .flex_col()
            .w_64()
            .h_full()
            .bg(rgb(0x1e1e1e))
            .child(
                div()
                    .p_2()
                    .text_sm()
                    .text_color(rgb(0xcccccc))
                    .child(if state.is_scanning {
                        "Scanning..."
                    } else {
                        "Changes"
                    }),
            )
            .child(
                v_flex().gap_1().children(state.uncommitted_changes.iter().map(
                    |change| {
                        let badge_color = match change.status {
                            GitFileStatus::Modified => rgb(0xe2b440),
                            GitFileStatus::Untracked => rgb(0x4caf50),
                            GitFileStatus::Deleted => rgb(0xf44336),
                            GitFileStatus::Conflicted => rgb(0x9c27b0),
                            _ => rgb(0xffffff),
                        };

                        div()
                            .flex()
                            .justify_between()
                            .p_1()
                            .hover(|style| style.bg(rgb(0x2a2a2a)))
                            .child(span(change.path.to_string_lossy().to_string()).text_sm())
                            .child(div().w_2().h_2().bg(badge_color))
                    },
                )),
            )
    }
}
```

### File-System Triggers

Pair the model with `notify` so no manual refresh is required:

1. `notify` receives a local file-system event.
2. Route the event to `state.update(cx, |state, cx| state.refresh(cx))`.
3. The sidebar rerenders with the updated change list.

## 6. Filesystem Watcher Architecture

Integrating the `notify` crate with a GPUI background architecture makes a desktop application feel native, snappy, and reactive.

File-system events happen rapidly and in bursts. For example, switching branches can change hundreds of files at once. Debounce events so the background Git thread does not spend redundant work rescanning the repository.

This research describes how to bridge a cross-platform file watcher with a GPUI model while establishing an extensible pattern for future polling and indexing.

### 6.1 Channel Architecture

The `notify` event loop runs on background OS threads. Bridge that foreign thread safely into GPUI through a standard crossbeam or asynchronous channel such as `smol::channel` or `tokio::sync::mpsc`. The proposed approach uses an asynchronous channel.

Add this dependency:

```toml
notify = "8.0" # Check the latest version and required features.
```

### 6.2 Spawn the Watcher in GPUI

Expand a `WorkspaceGitState` model so initialization creates a channel, spawns a native file watcher, and forwards events to a background task loop. Retain the watcher handle on the model so the watcher lives as long as the model.

```rust
use gpui::{AppContext, Context, Model, ModelContext, Task};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::time::Duration;

pub struct WorkspaceGitState {
    repo_path: PathBuf,
    pub uncommitted_changes: Vec<crate::UncommittedChange>,
    pub is_scanning: bool,
    _watcher: RecommendedWatcher,
}

impl WorkspaceGitState {
    pub fn new(repo_path: PathBuf, cx: &mut AppContext) -> Model<Self> {
        // Create a channel to forward file-system events.
        let (tx, rx) = smol::channel::unbounded::<notify::Result<Event>>();

        // Initialize the cross-platform native watcher.
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send_blocking(res);
            },
            Config::default(),
        )
        .expect("Failed to initialize system file watcher");

        watcher
            .watch(&repo_path, RecursiveMode::Recursive)
            .expect("Failed to watch repository directory");

        // Construct the model.
        let model = cx.new_model(|_cx| Self {
            repo_path,
            uncommitted_changes: Vec::new(),
            is_scanning: false,
            _watcher: watcher,
        });

        // Start the asynchronous event processing loop.
        Self::spawn_event_listener(model.clone(), rx, cx);

        model
    }
}
```

### 6.3 Debounced Event Listener

The listener runs continuously on a GPUI background executor. On a file mutation, it waits for a quiet window, such as 200 ms, before scanning. Each subsequent event resets the timer.

This queue can also receive future network or remote-polling updates.

```rust
impl WorkspaceGitState {
    fn spawn_event_listener(
        model: Model<Self>,
        rx: smol::channel::Receiver<notify::Result<Event>>,
        cx: &mut AppContext,
    ) {
        cx.spawn(|mut cx| async move {
            let mut debounce_timer: Option<Task<()>> = None;

            while let Ok(event_result) = rx.recv().await {
                match event_result {
                    Ok(event) => {
                        // Filter noise in .git while retaining ref updates.
                        if Self::should_ignore_event(&event) {
                            continue;
                        }

                        // Cancel the previous timer if it is still ticking down.
                        debounce_timer.take();

                        let model_clone = model.clone();
                        let mut cx_clone = cx.clone();

                        debounce_timer = Some(cx.background_executor().spawn(async move {
                            // Settle window: wait for file activity to calm down.
                            cx_clone
                                .background_executor()
                                .timer(Duration::from_millis(200))
                                .await;

                            // Return to the main UI thread to refresh model state.
                            model_clone
                                .update(&mut cx_clone, |this, cx| {
                                    this.refresh(cx);
                                })
                                .ok();
                        }));
                    }
                    Err(e) => eprintln!("File watcher error: {:?}", e),
                }
            }
        })
        .detach();
    }

    /// Determines if a file event should be ignored to protect performance.
    fn should_ignore_event(event: &Event) -> bool {
        for path in &event.paths {
            // Ignore .git objects and index locks, but retain HEAD and refs updates.
            if let Some(oss_str) = path.to_str() {
                if oss_str.contains(".git/objects") || oss_str.contains(".git/index.lock") {
                    return true;
                }
            }
        }
        false
    }
}
```

## 7. Future Cache, Index, and Polling Work

- Spawn a secondary background GPUI task that waits, for example, 60 seconds, then performs a lightweight `git2` fetch to detect divergence in `refs/remotes/origin`.
- Route polling updates through the same invalidation pipeline rather than updating the UI directly. A poller can emit a custom event or write a state file that triggers `WorkspaceGitState::refresh`.
- Extend the workspace scan to process changed Markdown pages, extract front-matter metadata such as task tags, priorities, and titles, and store it in a local UI cache or search index.

## 8. Suggested Follow-up Research

- Native GPUI conflict-resolution views with side-by-side diffs for conflicted planning pages.
- Safe worktree generation and isolation with `git2`.
