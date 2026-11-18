---
id: 01M7KNH6X4EDRBVDKWS1EP95YB
item_id: 01M7JZTKVFBBQ7MDAXEYE03X5W
created_at: 2026-11-17T02:31:47.078325094Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Review-ready, 2026-11-16. Branch head is the merge of origin/main (b16a20ab) into the ticket branch.

**Owner confirmation of the cause.** On the released v0.2.0 build (without these fixes), the owner removed the unreachable manyhands demo repo from the app registry and re-authenticated the key. The automatic poll five minutes later did not report the key locked. That is consistent with one failing repo evicting the shared passphrase. It does not test the fixes, which no Mac build carries yet.

**Verification, on the final tree (commit 5605fe3b, then main merged in).**
- `cargo fmt --check` and `cargo clippy --all-targets --all-features --locked -- -D warnings`: clean, also after the merge.
- Fast tier before the merge: 6,067 passed, 0 failed, 79 binaries (exit 0).
- SSH tier, all 11 harness targets, before the merge: cli_remote 166, comment_publication 41, document_promotion 79, mutation_remote 808, remote_merge_recovery 29, remote_observation 15, remote_polling 86, remote_synchronization 69, ssh_fixture 35, ssh_transport 123, ticket_closure 78 cases, all passed (exit 0). Main's merged changes touch only `src/desktop/view`, so the SSH tier was not rerun.
- After the merge: `--lib --bins` rerun with a real exit status: lib 1,564 passed (160 filtered by the `local_sync` skip), desktop bin 1,977, cli bin 317, 0 failed (exit 0). A first rerun was truncated by a `| head` in the command and proved nothing; this one is the real result.
- `git merge-tree` against origin/main: clean.

**Not covered.** No Mac build with the fixes exists. The UI still probably shows a poll failing because the server rejected the key as a locked key: follow-up. Per-repository keys: ticket 01M7K13QGQ7CH3C9AG1AAF3MJB.

Not closed: `ticket close` waits for review approval.
