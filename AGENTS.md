# Manyhands

## Structure

- This is one root Cargo binary, not a workspace. The desktop entry point is `src/main.rs`.
- Depend on `gpui-kit` only. It supplies the compatible GPUI API through `gpui_kit::*`; do not add a separate direct `gpui` dependency.
- Initialize GPUI Kit with `gpui_kit::init(cx)` inside `app.run` before using components, and make `Root` the first view created for each window.

## Development

- Run all Rust commands through Devenv: `devenv shell -- cargo <command>`.
- Before submitting Rust changes, run:
  ```sh
  devenv shell -- cargo check --locked
  devenv shell -- cargo fmt --check
  devenv shell -- cargo clippy --all-targets --locked -- -D warnings
  devenv shell -- cargo test --locked
  ```
- Smoke-test the native window with `devenv shell -- cargo run --locked`. It requires an active desktop display.

## Linux Toolchain

- Keep `devenv.nix`'s GPUI runtime library list and `env.LD_LIBRARY_PATH` in sync. GPUI dynamically loads Wayland; removing the loader path causes `NoWaylandLib` at startup even when the Nix packages are installed.
- Commit `Cargo.lock` after Cargo dependency changes and `devenv.lock` after Devenv input changes.
