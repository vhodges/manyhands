# Cross-Platform Build Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build both Manyhands executables for each supported native platform and architecture with GitHub Actions, and upload the release binaries as workflow artifacts.

**Architecture:** One GitHub Actions job uses a five-entry matrix of native hosted runner and Rust target pairs. A Linux-only setup step installs GPUI's native development libraries; every row installs Rust, restores an architecture-specific Cargo cache, builds both binaries with the desktop feature, and uploads the appropriate executable paths.

**Tech Stack:** GitHub Actions, `actions/checkout`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `actions/upload-artifact`, Cargo, GPUI Kit.

---

### Task 1: Add The Native Build Matrix

**Files:**
- Create: `.github/workflows/build.yml`

**Step 1: Create the GitHub Actions workflow**

Create `.github/workflows/build.yml` with this content:

```yaml
name: Build

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read

jobs:
  build:
    name: ${{ matrix.target }}
    runs-on: ${{ matrix.runner }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - runner: ubuntu-24.04
            target: x86_64-unknown-linux-gnu
            platform: linux
            executable-suffix: ""
          - runner: ubuntu-24.04-arm
            target: aarch64-unknown-linux-gnu
            platform: linux
            executable-suffix: ""
          - runner: windows-2022
            target: x86_64-pc-windows-msvc
            platform: windows
            executable-suffix: .exe
          - runner: windows-11-arm
            target: aarch64-pc-windows-msvc
            platform: windows
            executable-suffix: .exe
          - runner: macos-14
            target: aarch64-apple-darwin
            platform: macos
            executable-suffix: ""

    steps:
      - uses: actions/checkout@v4

      - name: Install Linux build dependencies
        if: matrix.platform == 'linux'
        shell: bash
        run: |
          sudo apt-get update
          sudo apt-get install --yes --no-install-recommends \
            libasound2-dev \
            libfontconfig-dev \
            libglib2.0-dev \
            libgtk-3-dev \
            libssl-dev \
            libvulkan-dev \
            libwayland-dev \
            libx11-xcb-dev \
            libxkbcommon-x11-dev \
            pkg-config

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: ${{ matrix.target }}

      - name: Build release binaries
        run: cargo build --locked --release --all-features --bins --target ${{ matrix.target }}

      - uses: actions/upload-artifact@v4
        with:
          name: manyhands-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/manyhands${{ matrix.executable-suffix }}
            target/${{ matrix.target }}/release/manyhands-cli${{ matrix.executable-suffix }}
          if-no-files-found: error
          retention-days: 14
```

The target triple is included in each output path because Cargo places an
explicit target build in `target/<target>/release`. `--all-features --bins`
builds both the feature-gated desktop binary and the headless CLI.

**Step 2: Validate the workflow syntax**

Run:

```sh
nix shell nixpkgs#actionlint -c actionlint .github/workflows/build.yml
```

Expected: exit status 0 with no lint findings.

**Step 3: Validate the desktop build locally**

Run:

```sh
devenv shell -- cargo check --all-features --locked
```

Expected: Cargo exits successfully after compiling the desktop binary and CLI.

**Step 4: Commit the workflow**

```sh
git add .github/workflows/build.yml
git commit -m "ci: add cross-platform build matrix"
```

### Task 2: Verify The Hosted Workflow

**Files:**
- Verify: `.github/workflows/build.yml`

**Step 1: Push the branch or open a pull request**

Push the branch containing the workflow so GitHub Actions can execute it.

Expected: GitHub schedules five `Build` jobs, one for each target triple.

**Step 2: Confirm every matrix row builds natively**

Verify that the following jobs pass:

```text
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-pc-windows-msvc
```

Expected: Every job completes the Cargo release build without needing a display server.

**Step 3: Inspect uploaded artifacts**

Open the workflow run's artifacts section and confirm each
`manyhands-<target>` artifact contains both `manyhands` and `manyhands-cli`,
with `.exe` suffixes only in the Windows artifacts.

Expected: Five downloadable artifacts, retained for 14 days. They are unsigned
raw executables; signing and installer packaging remain out of scope.
