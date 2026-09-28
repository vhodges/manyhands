# Cross-Platform Build Design

## Goal

Use GitHub Actions to build both Manyhands binaries for the supported native
platform and architecture combinations, and make the resulting release
executables downloadable from each workflow run.

## Workflow Structure

Add one GitHub Actions workflow triggered by pushes and pull requests to
`main`, plus `workflow_dispatch` for manual runs. A single job uses a matrix
of native GitHub-hosted runner and Rust target pairs:

| Runner | Rust target |
| --- | --- |
| `ubuntu-24.04` | `x86_64-unknown-linux-gnu` |
| `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` |
| `windows-2022` | `x86_64-pc-windows-msvc` |
| `windows-11-arm` | `aarch64-pc-windows-msvc` |
| `macos-14` | `aarch64-apple-darwin` |

Each row builds on hardware matching its target architecture. This is more
reliable for GPUI's native desktop dependencies than cross-compiling ARM
targets from x86 runners.

## Build Steps

Each matrix job checks out the source, installs stable Rust for the target,
and restores a Cargo cache. It builds both explicitly declared binaries in
release mode with all features enabled:

```sh
cargo build --locked --release --all-features --bins --target <target>
```

Enabling all features compiles the GPUI desktop binary and the headless CLI
together. The release output is uploaded as a uniquely named artifact for its
Rust target triple. Windows paths include the `.exe` extension; other paths do
not.

## Platform Requirements

Linux jobs install the development packages required to compile GPUI's Wayland,
Vulkan, X11/XKB, GTK, and Fontconfig integrations. This setup runs on both
x86_64 and ARM64 Ubuntu runners. Windows uses the MSVC toolchain supplied by
the runner. macOS uses the Xcode SDK supplied by the ARM64 `macos-14` runner.

## Non-Goals

- Do not cross-compile desktop targets.
- Do not run the desktop application in CI; it needs a graphical display.
- Do not code-sign binaries or create installers or release assets. Those are
  future distribution steps.
