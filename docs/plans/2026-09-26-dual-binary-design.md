# Manyhands Dual-Binary Design

## Goal

Provide a graphical desktop application and a genuinely headless command-line
application while sharing one domain implementation. The CLI must build and
run on systems without graphical libraries or a display server.

## Package Structure

Keep Manyhands as one root Cargo package, not a workspace. Make its binaries
explicit and disable automatic binary discovery in `Cargo.toml`:

```toml
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

The resulting source layout is:

```text
src/
  lib.rs
  main.rs
  bin/
    manyhands-cli.rs
```

`src/lib.rs` owns domain logic, data access, and operations shared by both
front ends. It must not import GPUI, GPUI Kit, or other desktop-only code.

`src/main.rs` remains the `manyhands` desktop entry point. It alone imports
and initializes GPUI Kit, creates the native window, and adapts the shared
library to graphical views.

`src/bin/manyhands-cli.rs` is the `manyhands-cli` entry point. It calls the
shared library and maps its results and errors to command-line output and exit
codes. It has no dependency on GPUI.

## Build And Distribution Behavior

The `desktop` feature is disabled by default, so a headless build remains
independent of graphical native dependencies:

```sh
devenv shell -- cargo run --locked --bin manyhands-cli -- --help
```

The desktop application is built or run explicitly with its feature enabled:

```sh
devenv shell -- cargo run --locked --features desktop --bin manyhands
```

A release containing both commands must enable the desktop feature. Cargo
commands must specify `--bin` because the package has two executables.

## Error Handling And Tests

Shared operations return typed results and errors rather than printing output
or interacting with the window system. The CLI translates them into standard
output, standard error, and exit codes; the desktop app translates them into
view state and user-visible feedback.

Unit and integration tests for shared functionality run with default features
and therefore remain headless. Feature-complete checks include the desktop
feature to compile and lint the graphical entry point.

## AGENTS.md Changes

Update the repository guidance when this design is implemented:

- Change the structure note from "one root Cargo binary" to "one root Cargo
  package with two explicit binaries"; retain the prohibition on a workspace.
- State that `src/lib.rs` is the headless shared layer and that GPUI Kit may be
  imported only by the desktop binary and desktop-only modules.
- Replace generic Rust verification commands with feature-complete commands:

  ```sh
  devenv shell -- cargo check --all-features --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
  devenv shell -- cargo test --all-features --locked
  ```

- Replace the desktop smoke-test command with:

  ```sh
  devenv shell -- cargo run --locked --features desktop --bin manyhands
  ```

- Add a headless CLI smoke-test command once the CLI exposes its initial
  command, for example:

  ```sh
  devenv shell -- cargo run --locked --bin manyhands-cli -- --help
  ```

## Non-Goals

- Do not introduce a Cargo workspace or separate `core`, `desktop`, and `cli`
  crates.
- Do not share presentation-layer code between the CLI and desktop UI.
- Do not make the GUI feature part of the default feature set.
