# Third-party licences and source

Manyhands' own MIT licence is in the package's top-level `LICENSE`.
`THIRD-PARTY-NOTICES.html` contains complete harvested licence texts and
attributions for the target's featureless CLI and desktop dependency
graphs, plus IBM Plex Sans attribution and its SIL Open Font License.

`dependencies/` preserves original licence, copyright, copying, and notice
files found throughout the dependency sources, including vendored native
libraries and embedded assets. `inventory.json` records the package
versions and paths. Some unused, nested upstream notices are included
conservatively; their presence does not mean all upstream code is linked
into Manyhands. Rust build-only and test-only graphs are otherwise omitted.
Windows also includes the vendored OpenSSL source package's notices.

`sources/` contains the original published `.crate` source archives for
MPL-2.0-covered dependencies and for the libgit2 binding package, including
its vendored libgit2 source and licence/exception. These archives are
verified against `Cargo.lock` SHA-256 checksums. No upstream source has
been modified by the packaging helper. A `.crate` file is a gzip-compressed
tar archive; extract it with a tar-capable archive manager or:

```sh
tar -xzf sources/option-ext-0.2.0.crate
```

The HTML notices also provide version-specific original source download
links. These source components retain their respective licences. The
font source/provenance and full licence are under `fonts/`.

## Maintainer notes

Generate notices using the pinned tool; its CLI feature must be enabled:

```sh
devenv shell -- cargo install cargo-about --version 0.9.3 --locked --features cli
devenv shell -- python3 -B scripts/release/notices.py --target x86_64-unknown-linux-gnu --output /tmp/opencode/manyhands-notices
```

The output must be a new directory. The generator uses `--locked --fail`
and target-specific feature graphs; it does not silently accept unknown
licences. The template escapes licence text and metadata. `about.toml`
prefers MIT for dual-licensed crates and accepts the reviewed licence
families in the locked dependency inventory. No crate-specific licence
overrides or synthetic clarifications are currently needed on Linux.

Both JSON discovery and HTML rendering use cargo-about's `--output-file`
option and explicit UTF-8 reads. Version 0.9.3 rejects stdout output when
PowerShell occurs anywhere in its process ancestry, including Python
capturing stdout. File output keeps the generator compatible with Windows
PowerShell and preserves Unicode notices independently of shell encoding.

Normal-graph metadata is not a runtime-linkage audit. The native packaging
steps must additionally account for any external dylib/DLL copied from a
runner, and the corresponding notices/source obligations. System runtime
libraries are documented separately rather than copied indiscriminately.
