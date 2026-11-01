# Manyhands

Manyhands is a local-first, Git-backed project management and documentation
tool. It keeps Markdown-backed tickets, managed documents, and their
collaboration history in repositories that users already control.

The project provides a Rust desktop application and a headless CLI. Development
is guided by the approved product and architecture documents linked below.

## Development

Manyhands uses [Devenv](https://devenv.sh/) to provide its Rust toolchain and
native desktop dependencies. Run Rust commands through Devenv:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

The package exposes two binaries:

```sh
# Headless CLI smoke test
devenv shell -- cargo run --locked --bin manyhands-cli

# Desktop smoke test; requires an active desktop display.
devenv shell -- cargo run --locked --features desktop --bin manyhands
```

The desktop application targets Windows, macOS, and Linux. Linux desktop use
requires a Wayland session.

## Releases

The [Release workflow](.github/workflows/release.yml) is invoked manually
from `main` with an existing, merged version tag. It builds the CLI and
desktop for Linux x86_64/ARM64, Windows x86_64, and macOS Apple Silicon,
packages their documentation and licences, and assembles a draft GitHub
Release with checksums. Publication is manual. See the
[release guide](packaging/README.md) for tag/version requirements, signing
setup, runtime support, and recovery.

### Linux Nix packages

The root flake builds from source for Linux x86_64 and ARM64. From a checkout
containing the flake:

```sh
nix run .#manyhands-cli -- --help # featureless, headless CLI
nix build .#default             # desktop plus a separately built CLI
nix run .                      # desktop; needs Wayland and host graphics drivers
```

See the [Nix guide](nix/README.md) for tag-pinned installation, NixOS/Home
Manager selection, installed licences/source disclosures, and runtime checks.
`v0.1.6` predates the flake; remote installation needs a later published tag
containing it. Devenv remains the development environment.

## Repository Layout

- `src/lib.rs`: domain logic shared by the desktop and CLI front ends.
- `src/main.rs`: desktop application entry point.
- `src/bin/manyhands-cli.rs`: headless CLI entry point.
- `docs/`: charter, product requirements, RFCs, Waves, and Cycles.
- `.github/`: CI workflow and contributor guidance.

## Project Documents

- [Project charter](docs/charter.md)
- [MVP/Dogfood PRD](docs/PRD/mvp.md)
- [MVP architecture RFC](docs/RFC/mvp-rfc.md)

## Contributing

Contributions are welcome. Please read the
[contribution guide](.github/CONTRIBUTING.md) before starting work.

## License

Manyhands is licensed under the [MIT License](LICENSE).
Bundled [IBM Plex Sans fonts](assets/fonts/README.md) retain their SIL Open
Font License. Release packages include dependency and asset disclosures;
see [release packaging](packaging/README.md) for the release process and
licence-generation tooling.
