# CLI third-party licences and source

This bundle covers the featureless `manyhands-cli` package only. It does not
include the desktop dependency graph or IBM Plex Sans fonts. Manyhands' own
MIT licence is installed alongside `THIRD-PARTY-NOTICES.html`.

The HTML contains complete licence texts and attributions for the normal CLI
dependency graph. `dependencies/` preserves original licence, copyright,
copying, and notice files from dependency sources, including the vendored
libgit2 source tree. Unused nested upstream notices may be included
conservatively; Rust build-only and test-only dependencies are omitted.
`inventory.json` records package versions and relative disclosure paths.

`sources/` contains the original published `.crate` archives for MPL-covered
dependencies and libgit2-sys, including its vendored libgit2 source and
licence/exception. They are verified against `Cargo.lock` SHA-256 checksums.
No dependency source is modified by this notice generator. A `.crate` file
is a gzip-compressed tar archive, for example:

```sh
tar -xzf sources/option-ext-0.2.0.crate
```

The Nix build supplies the notice tool, vendored crates, and original source
archives as pinned inputs. Notice generation runs offline in the sandbox.
Nix runtime libraries remain separate store packages with their own terms;
see the Nix installation guide and pinned Nixpkgs expressions for their sources.
