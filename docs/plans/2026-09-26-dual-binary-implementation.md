# Dual-Binary Package Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Split Manyhands into a headless CLI and optional-feature desktop binary that share a GPUI-independent library boundary.

**Architecture:** Keep one root Cargo package with explicitly declared binary targets. `src/lib.rs` is the future shared domain layer; the desktop entry point is gated by an optional `desktop` feature that enables GPUI Kit, while `manyhands-cli` builds with default features and never imports graphical code.

**Tech Stack:** Rust 2024, Cargo features and binary targets, Devenv/Nix, GPUI Kit 0.6.

**Commit Policy:** Do not create commits unless the user explicitly requests them.

---

### Task 1: Define Explicit Targets And Feature Boundaries

**Files:**
- Modify: `Cargo.toml:1-9`
- Modify: `Cargo.lock` only if Cargo changes it

**Step 1: Prove the headless binary does not exist yet**

Run:

```sh
devenv shell -- cargo check --locked --bin manyhands-cli
```

Expected: FAIL with an error that there is no binary target named
`manyhands-cli`.

**Step 2: Declare the package's binaries and optional desktop dependency**

Replace the relevant sections of `Cargo.toml` with:

```toml
[package]
name = "manyhands"
version = "0.1.0"
edition = "2024"
authors = ["Vince Hodges <vhodges@gmail.com>"]
license = "MIT"
autobins = false

[features]
default = []
desktop = ["dep:gpui-kit"]

[dependencies]
gpui-kit = { version = "0.6", optional = true }

[[bin]]
name = "manyhands"
path = "src/main.rs"
required-features = ["desktop"]

[[bin]]
name = "manyhands-cli"
path = "src/bin/manyhands-cli.rs"
```

`autobins = false` prevents Cargo from implicitly declaring `src/main.rs` a
second time. The `desktop` feature must stay out of `default` so a normal CLI
build does not select GPUI Kit.

**Step 3: Confirm Cargo recognizes the new target declaration**

Run:

```sh
devenv shell -- cargo check --locked --bin manyhands-cli
```

Expected: FAIL because `src/bin/manyhands-cli.rs` has not been created. The
failure must not mention a missing binary target or a GPUI-related library.

**Step 4: Inspect the manifest change**

Run:

```sh
git diff -- Cargo.toml Cargo.lock
```

Expected: GPUI Kit is optional, the default feature set is empty, and exactly
two binaries are declared.

**Step 5: Commit if requested**

If the user requests a commit, stage only `Cargo.toml` and an updated
`Cargo.lock`, then use:

```sh
git commit -m "build: separate desktop feature from cli"
```

### Task 2: Add Headless And Shared-Layer Entry Points

**Files:**
- Create: `src/lib.rs`
- Create: `src/bin/manyhands-cli.rs`
- Verify: `src/main.rs`

**Step 1: Confirm the declared CLI source is missing**

Run:

```sh
devenv shell -- cargo check --locked --bin manyhands-cli
```

Expected: FAIL with an error that `src/bin/manyhands-cli.rs` cannot be read.

**Step 2: Create the future shared domain boundary**

Create `src/lib.rs`:

```rust
//! Headless domain logic shared by the Manyhands front ends.
```

Do not move the existing `HelloWorld` view into the library. There is no
domain operation to share yet; this empty, headless crate boundary prevents
future shared logic from acquiring a GUI dependency by accident.

**Step 3: Add the minimal headless executable**

Create `src/bin/manyhands-cli.rs`:

```rust
fn main() {}
```

This is intentionally only a target skeleton. Do not invent CLI commands,
flags, or a command-line parsing dependency before the first CLI operation is
designed.

**Step 4: Verify the headless path**

Run:

```sh
devenv shell -- cargo check --locked --bin manyhands-cli
```

Expected: PASS without enabling `desktop`.

Run:

```sh
devenv shell -- cargo run --locked --bin manyhands-cli
```

Expected: PASS, exit successfully, and open no native window.

**Step 5: Verify the desktop path remains opt-in**

Run:

```sh
devenv shell -- cargo check --locked --features desktop --bin manyhands
```

Expected: PASS and compile the existing GPUI Kit entry point.

**Step 6: Commit if requested**

If the user requests a commit, stage only the two new source files and use:

```sh
git commit -m "feat: add headless cli target"
```

### Task 3: Update Repository Guidance For Both Execution Modes

**Files:**
- Modify: `AGENTS.md:3-23`

**Step 1: Update the structure guidance**

Replace the first structure bullet with guidance equivalent to:

```markdown
- This is one root Cargo package, not a workspace. It declares two binaries:
  `manyhands` is the desktop entry point in `src/main.rs` and requires the
  `desktop` feature; `manyhands-cli` is the headless entry point in
  `src/bin/manyhands-cli.rs`.
- Put domain logic shared by both front ends in `src/lib.rs`. It must not
  depend on GPUI or GPUI Kit; those dependencies belong only to the desktop
  binary and desktop-only modules.
```

Retain the requirement to depend directly on `gpui-kit` only and to initialize
it before creating components.

**Step 2: Replace the Rust verification commands**

Replace the current pre-submission commands with:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

`--all-features` ensures the optional desktop target remains compiled, linted,
and tested; the CLI is included because its target has no required feature.

**Step 3: Add separate smoke-test instructions**

Replace the existing desktop launch command with:

```sh
devenv shell -- cargo run --locked --features desktop --bin manyhands
```

Keep the active-display caveat. Add this headless smoke-test command for the
initial target skeleton:

```sh
devenv shell -- cargo run --locked --bin manyhands-cli
```

When the first real CLI command has a help interface, change the headless
smoke test to `manyhands-cli -- --help` and state its expected output.

**Step 4: Inspect the documentation change**

Run:

```sh
git diff --check
git diff -- AGENTS.md
```

Expected: PASS with guidance covering both explicit binary names, the desktop
feature, and feature-complete verification.

**Step 5: Commit if requested**

If the user requests a commit, stage only `AGENTS.md` and use:

```sh
git commit -m "docs: document dual-binary development"
```

### Task 4: Verify The Complete Package

**Files:**
- Verify only: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/main.rs`, `src/bin/manyhands-cli.rs`, `AGENTS.md`

**Step 1: Run all feature-complete automated checks**

Run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Expected: Every command exits successfully without formatting differences or
Clippy warnings.

**Step 2: Smoke-test the headless binary**

Run:

```sh
devenv shell -- cargo run --locked --bin manyhands-cli
```

Expected: PASS without an active desktop display or graphical runtime.

**Step 3: Smoke-test the desktop binary when a display is available**

Run:

```sh
devenv shell -- cargo run --locked --features desktop --bin manyhands
```

Expected: PASS and open the existing Hello World window. If the only failure
is a missing `DISPLAY` or `WAYLAND_DISPLAY`, preserve successful automated
verification and report the environmental limitation rather than changing
application code.

**Step 4: Review the final worktree**

Run:

```sh
git status --short
```

Expected: only files intentionally changed for the dual-binary structure,
plus any pre-existing user files that were left untouched.
