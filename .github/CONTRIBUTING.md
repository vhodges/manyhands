# Contributing to Manyhands

Contributions are welcome. Manyhands is developed through requirements, RFCs,
Waves, and Cycles, so shared understanding before implementation matters.

## Discuss First

Discuss a non-trivial feature, behavior change, architecture change, or broad
refactor before opening a pull request. Start or join either a GitHub Issue or
GitHub Discussion so the proposed scope and approach are understood.

Small, self-contained documentation corrections may be opened directly as a
pull request.

## Development Expectations

Read and follow [`AGENTS.md`](../AGENTS.md). In particular:

- Keep shared domain logic in `src/lib.rs`, independent of GPUI and GPUI Kit.
- Use `gpui-kit` rather than a direct `gpui` dependency for desktop code.
- Run Rust commands through Devenv.

Before submitting Rust changes, run:

```sh
devenv shell -- cargo check --all-features --locked
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy --all-targets --all-features --locked -- -D warnings
devenv shell -- cargo test --all-features --locked
```

Smoke-test the affected front end when practical:

```sh
# Headless CLI
devenv shell -- cargo run --locked --bin manyhands-cli

# Desktop application; requires an active desktop display.
devenv shell -- cargo run --locked --features desktop --bin manyhands
```

## Pull Requests

Keep pull requests focused. Describe the intent of the change, link the prior
Issue or Discussion when one exists, and include the verification you ran.
Avoid unrelated refactors and do not commit credentials, private keys, or other
secrets.

## Agentic Work

Agentic work is welcome when it meets the same quality bar as any other
contribution. Disclosure is optional. The contributor who opens the pull
request remains responsible for the change's correctness, provenance, tests,
security, and maintainability.

## License

By contributing, you agree that your contribution is licensed under the
repository's [MIT License](../LICENSE).
