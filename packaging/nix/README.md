# Manyhands Nix packages

The root flake builds from source for `x86_64-linux` and `aarch64-linux`.
`default` / `manyhands` contains the desktop and a separately built featureless
CLI; `manyhands-cli` is the CLI alone, without desktop fonts or graphics
dependencies. These are not repackaged Ubuntu release binaries.

The desktop requires a Wayland session and working host graphics drivers.
Its wrapper supplies GPUI's dynamically loaded libraries, including Wayland,
Vulkan, EGL, and GL. It does not install drivers or replace the system graphics
configuration. The CLI is installed directly, without a wrapper or graphics
loader override. Git operations and SSH transport use the linked libraries;
no client-side Git executable is required or added to PATH.

Installed disclosures are under `share/doc/manyhands/`: Manyhands' MIT
`LICENSE`, complete Rust dependency notices, original nested notices, and
checksum-verified MPL/libgit2 source archives. The full package also preserves
separately licensed Plasma protocol XML, its original crate archive and LGPL
texts; its wrapper's MIT metadata does not override file-level licences.
The full package also contains
IBM Plex Sans provenance and SIL OFL text; fonts remain embedded, not installed
globally. Nix runtime libraries are separate store packages, not copied into
Manyhands. Their source recipes are pinned by `flake.lock`'s Nixpkgs input.

The release toolchain and notice generator versions are checked against
`packaging/tool-versions.env`. Crates and original source archives are pinned
inputs; notice generation uses frozen/offline resolution inside the sandbox.
Original licence files missing from published crates are supplied as reviewed,
checksum-pinned supplemental inputs. Canonical SPDX placeholders without an
original copyright attribution fail generation instead of passing silently.
Development still uses `devenv shell`, with its existing independent inputs.

From a checkout containing this flake:

```sh
nix run .#manyhands-cli -- --help
nix run .
nix build .#manyhands-cli
nix build .#default
```

For a released version, set `TAG` to a published tag **containing the flake**:

```sh
nix run "github:vhodges/manyhands/${TAG}#manyhands-cli" -- --help
nix profile install "github:vhodges/manyhands/${TAG}#default"
```

`v0.1.6` predates this flake. Publication and tag-pinned consumer acceptance
are separate release steps; do not assume an older tag contains these files.

## NixOS and Home Manager

Add this input to your own configuration flake, replacing the placeholder
with a published tag containing the flake, then update your configuration's
lock file. Manyhands keeps its own pinned Nixpkgs/toolchain: do **not** make
its `nixpkgs` input follow a different revision unless you intend to review
and rebuild the toolchain change.

```nix
inputs.manyhands.url = "github:vhodges/manyhands/REPLACE_WITH_PUBLISHED_TAG";
```

Inside your flake's `outputs = { nixpkgs, manyhands, ... }: ...`, a NixOS
system can select the full package in an inline module:

```nix
nixosConfigurations.myHost = nixpkgs.lib.nixosSystem {
  system = "x86_64-linux"; # or aarch64-linux
  modules = [
    ./configuration.nix
    ({ pkgs, ... }: {
      environment.systemPackages = [
        manyhands.packages.${pkgs.stdenv.hostPlatform.system}.default
      ];
    })
  ];
};
```

For Home Manager, add an inline module to your existing `modules` list
within the same outputs scope. This example selects **only the CLI**:

```nix
({ pkgs, ... }: {
  home.packages = [
    manyhands.packages.${pkgs.stdenv.hostPlatform.system}.manyhands-cli
  ];
})
```

Select `.default` instead for desktop plus CLI; do not install both variants
in one profile because both provide `bin/manyhands-cli`. Neither package
changes your user data, Git identity, SSH keys, or system graphics setup.
On NixOS, enable the appropriate host graphics drivers/Wayland compositor
in your system configuration. Other Linux distributions may need additional
host-driver integration; providing loader libraries alone does not establish
desktop compatibility there. No NixGL or driver configuration is automated.

## Checks and acceptance

```sh
nix flake check --all-systems --no-build # evaluates both architectures only
nix build .#manyhands-cli
nix build .#default
nix flake check                         # builds this machine's checks
nix path-info -r .#manyhands-cli         # inspect runtime closure
```

Checks exercise source filtering, isolated packaged CLI help and repository
creation with an empty PATH, the CLI's Git-/graphics-free runtime closure,
installed desktop disclosures/entry/icon, and
loading GPUI's required shared libraries in a sandbox. They do not open a
desktop window or prove Vulkan driver/device access. Package builds also
generate complete disclosures offline. Source-archive/protocol checks derive
selected package versions and checksums from Cargo.lock and the installed
inventory, rather than duplicating version literals. Project Rust tests remain a separate
Devenv/native gate, not a repeated package-build dependency.

For desktop acceptance, use NixOS/Wayland with working host drivers, unset
inherited `LD_LIBRARY_PATH`/`DYLD_LIBRARY_PATH`, and launch the installed
`bin/manyhands` wrapper (not its private wrapped executable) outside Devenv.
Use an empty temporary `HOME` and `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/
`XDG_CACHE_HOME`, confirm a window opens and text renders, then close it.
Check both binaries and readable disclosures on each supported architecture.
ARM64 output evaluation is not an ARM64 build or runtime test. Tag-pinned
remote commands require a published tag and are a post-publication check.

Desktop disclosure/distribution review is still open at this implementation
checkpoint. Unreviewed declaration-only upstream cases remain fail-closed, and no
exception for generated KDE bindings or resolution of the preserved objc2
Apple SDK caveat is asserted. See the release execution record for actual
acceptance and unresolved decisions. A local source build is not, by itself,
clearance to redistribute a binary or operate a binary cache.
