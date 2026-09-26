# Manyhands Initial Scaffold Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build and launch a minimal `manyhands` GPUI Kit desktop window through the Rust-enabled Devenv environment.

**Architecture:** The repository root becomes one Cargo binary package. `src/main.rs` uses the GPUI API re-exported by GPUI Kit to initialize its component system and open a root-wrapped Hello World view. Devenv supplies the native Linux libraries required to compile and run GPUI Kit.

**Tech Stack:** Rust 2024, Cargo, Devenv/Nix, GPUI Kit 0.6, GPUI Kit components.

**Commit Policy:** Do not create commits unless the user explicitly requests them.

---

### Task 1: Establish Cargo Package Metadata

**Files:**
- Create: `Cargo.toml`
- Modify: `.gitignore:1-10`

**Step 1: Run the initial compile probe**

Run: `devenv shell -- cargo check`

Expected: FAIL because no `Cargo.toml` exists in the repository.

**Step 2: Add the Cargo manifest and build-output ignore rule**

Create `Cargo.toml`:

```toml
[package]
name = "manyhands"
version = "0.1.0"
edition = "2024"

[dependencies]
gpui-kit = "0.6"
```

Append the following to `.gitignore`:

```gitignore

# Rust
/target/
```

Use GPUI Kit as the only direct GUI dependency. It resolves and re-exports its compatible GPUI version, avoiding an incompatible direct `gpui` dependency.

**Step 3: Verify Cargo recognizes the new package**

Run: `devenv shell -- cargo check`

Expected: FAIL with a message that the package has no targets, because `src/main.rs` does not exist yet.

**Step 4: Inspect the pending package changes**

Run: `git status --short`

Expected: `Cargo.toml` is untracked and `.gitignore` is modified.

### Task 2: Supply Linux GPUI Libraries Through Devenv

**Files:**
- Modify: `devenv.nix:8`

**Step 1: Confirm the shell lacks a required native discovery tool**

Run: `devenv shell -- pkg-config --modversion fontconfig`

Expected: FAIL because `pkg-config` is not yet included in the development environment.

**Step 2: Expand the Devenv package list**

Replace the current `packages` assignment with:

```nix
  packages = with pkgs; [
    git
    pkg-config
    clang
    wayland
    vulkan-headers
    vulkan-loader
    libxcb
    libxkbcommon
    atk
    fontconfig
    gio-sharp
    glib
    gtk3
  ];
```

Keep the existing `languages.rust.enable = true;` line unchanged. These packages mirror GPUI Kit's Linux development shell: compiler and package discovery, display/input libraries, GPU loader, fonts, and GTK/GIO libraries.

**Step 3: Verify native library discovery**

Run: `devenv shell -- pkg-config --modversion fontconfig`

Expected: PASS and print the installed Fontconfig version.

**Step 4: Inspect the pending Devenv change**

Run: `git diff -- devenv.nix`

Expected: the existing Rust-enabled configuration remains intact and only the package list has expanded.

### Task 3: Add the Native Hello World View

**Files:**
- Create: `src/main.rs`
- Create: `Cargo.lock`

**Step 1: Confirm Cargo still has no executable target**

Run: `devenv shell -- cargo check`

Expected: FAIL with the no-target error from Task 1.

**Step 2: Implement the minimal GPUI Kit application**

Create `src/main.rs`:

```rust
use gpui_kit::component::button::*;
use gpui_kit::component::*;
use gpui_kit::*;

struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("hello")
                    .primary()
                    .label("Say hello")
                    .on_click(|_, _, _| println!("Hello from Manyhands!")),
            )
    }
}

fn main() {
    let app = gpui_kit::application().with_assets(gpui_kit::assets::Assets);

    app.run(move |cx| {
        gpui_kit::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open Manyhands window");
        })
        .detach();
    });
}
```

`gpui_kit::init` must run before using components, and `Root` must be the first view in a GPUI Kit window. Cargo will generate `Cargo.lock` when it resolves the crates.io dependency graph.

**Step 3: Compile the executable**

Run: `devenv shell -- cargo check`

Expected: PASS and compile `manyhands` plus its locked GPUI Kit dependency graph.

**Step 4: Format and lint the scaffold**

Run: `devenv shell -- cargo fmt --check`

Expected: PASS with no formatting differences.

Run: `devenv shell -- cargo clippy --all-targets -- -D warnings`

Expected: PASS with no warnings.

**Step 5: Inspect the generated source and lockfile**

Run: `git status --short`

Expected: `src/main.rs` and `Cargo.lock` appear alongside the planned configuration changes.

### Task 4: Validate the Native Runtime

**Files:**
- Verify only: `Cargo.toml`, `Cargo.lock`, `src/main.rs`, `devenv.nix`

**Step 1: Launch the application through Devenv**

Run: `devenv shell -- cargo run`

Expected: PASS, open a native Manyhands window showing “Hello, World!” and a “Say hello” button. Clicking the button prints `Hello from Manyhands!` in the terminal.

**Step 2: Handle missing display support explicitly**

If the command fails only because `DISPLAY`/`WAYLAND_DISPLAY` is unavailable, retain the successful compile, format, and lint evidence and report that the graphical runtime needs a desktop session. Do not change application code to hide that environmental limitation.

**Step 3: Inspect the final working tree**

Run: `git status --short`

Expected: only the planned source, configuration, lockfile, and documentation changes are present.
