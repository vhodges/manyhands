---
id: 01M70RA45PXWFVYGXZR2EX147M
item_id: 01M70PYS3B3YWEFTY0QBJGCF6N
created_at: 2026-11-09T18:15:06.776273901Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Cause found and fixed (2026-11-09). Fix commit `50fe563e` on this ticket's branch.

**Cause: the test fixtures depend on a global Git identity.** These four fixtures call `RepositoryService::enable` with `identity: None`, without setting an identity in the fixture repository:

| Failing group | Fixture |
| --- | --- |
| execution_tests (59) | `execution_tests.rs` `Fixture::new` |
| record (25) | `authority_tests.rs` `Fixture::with_request` (`record::authority::tests`) |
| instance (8) | `instance_tests.rs` `Fixture::new` |
| run (1) | `run.rs` `tests::fixture` |

With `identity: None`, `enable_recorded` (`src/repository.rs`, about line 5310) resolves the identity from Git's levels in order: local, global, XDG, system. If none of them holds both `user.name` and `user.email`, it returns `Ok(EnableRepositoryOutcome::IdentityRequired)` and writes no `.manyhands/config.toml`. The fixtures only `.unwrap()` the `Result`, so they carry on with an unenabled repository:

- Most then panic with `NotFound` when they read the absent configuration, as at `mutation_tests.rs:45`. Not every failure was sampled, but the probe below turns all 93 green.
- `run::tests::retirement_live_inventory_invalid_fresh_authenticated_request_replays_refusal` fails at `run.rs:1211`: retirement on a repository that was never enabled does not give the expected `Refused` outcome. The probe shows this is the same cause, not a separate one.

This machine has no global identity: `~/.gitconfig`, `~/.config/git/config` and `/etc/gitconfig` are all absent, and the devenv shell does not add one. The main checkout's identity is only in its local `.git/config`, which the tempdir fixtures do not see. The author of 01M6KHVD6D0DB8KYT6VGQP10Y0 presumably had a global identity, which explains why they saw these tests green. Other repository fixtures, such as `context_release_tests.rs`, already set a local identity before `enable`.

**Evidence, on an unchanged tree (b02d1261 = origin/main f4442618 plus the ticket checkpoint):**
- `mutation_tests::retirement_batch_public_live_pass_retires_real_context_and_replays_without_transport`, run alone with `--exact --test-threads=1`, panics at `mutation_tests.rs:45:49` in 0.13 s. With `XDG_CONFIG_HOME` pointed at a file holding only `user.name`/`user.email`, it passes in 5.55 s.
- `run::tests::retirement_live_inventory_invalid_fresh_authenticated_request_replays_refusal`, run the same way, panics at `run.rs:1211:9`. With the probe identity it passes in 0.30 s.
- `--lib -- repository::context_retirement::` with the probe identity: 142 passed, 0 failed (192 s). Without it: 49 passed, 93 failed, as reported.

Hypotheses 1 (config path moved) and 3 (a later merge broke them) are ruled out, so no bisect was needed. Hypothesis 2 (environment) is the cause.

**Fix:** each of the four fixtures now sets `user.name`/`user.email` in the fixture repository's local config before enabling, like the other fixtures. Only test code changed, and no behavior did, so no new test was added: the 93 tests are the regression check on any machine without a global identity. No `src/desktop/` change, and the Wave 04 Cycle 01 branch was not touched.

**Verification so far, on 50fe563e with no global identity:**
- `--lib -- repository::context_retirement::`: 142 passed, 0 failed (189 s).
- `cargo check --all-features --locked`, `cargo fmt --check`, and `cargo clippy --all-targets --all-features --locked -- -D warnings`: all clean.
- The fast tier with `--no-fail-fast` is running. Results will follow in another comment. No SSH tier applies: `src/repository/context_retirement/` is in no SSH row.

**Follow-up worth considering, not done here:** the fixtures' `.unwrap()` on `enable` hides a non-`Enabled` outcome. Asserting the outcome would have reported this at the right line.
