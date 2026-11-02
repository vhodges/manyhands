---
id: 01M5MJMV6XWME37VA45S9BNS6H
item_id: 01M5MAVV5CQE98Q9P2GG9T4FP9
created_at: 2026-10-23T14:29:10Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Review-ready on local gates

The fix is commit `b62d9e9`. Review follow-up `33485cb` changes only
documentation and a test comment. Main moved during the work, from
`90adb72` to `25b9fcf`, and was merged in as `abf66a0` without conflicts.
Its three commits change only Markdown front matter (documents, comments and
closed tickets), and no test reads those files. `origin/main` is now an
ancestor of the branch, and `git merge-tree --write-tree HEAD origin/main` is
clean. These two comments use main's new canonical comment form, including
`created_by`.

**Independent review.** One read-only reviewer covered `a3994ee..b62d9e9`.
It found nothing Critical or Important; its verdict was ready to merge once
the full suite passed. It traced every `resolve_identity` caller, checked the
rule against libgit2 1.9.7's `signature.c`, and confirmed that the
partial-create test really takes up the pending row. Its Minor findings were
handled as follows:

- **Fixed:** the contract amendment now says that the resolver rule also
  covers item and comment saves, repair, the publication-remote change and
  `repo identity`.
- **Fixed:** a test helper's comment.
- **Not changed:** `ValidIdentity::of` repeats the signature round-trip. The
  owner kept `repo identity-set` as a separate, stricter rule, and changing it
  is out of scope.
- **Not changed:** two small test gaps. A domain test covers a local unusable
  identity. A root that does not exist cannot be checked with
  `recovery_inspection`, because that call canonicalizes the root.
- **Raised for the owner, not changed:** the desktop copy `IDENTITY_REQUIRED`
  ("Git has no name and email for this repository …") and the
  publication-remote message ("configure user.name and user.email …") are
  now loose in one rare case. That case is a configured identity that exists
  but cannot be committed. Either message could be reworded, or a distinct
  "unusable" state could be added in follow-up work.

**Verification on `b62d9e9`:**

- `devenv shell -- cargo test --all-features --locked --no-fail-fast` exited
  0: 3281 passed, 0 failed, 1 ignored.
- `cargo check`, `cargo fmt --check` and `cargo clippy` (with `--all-features`
  and with `--features desktop`) exited 0, as recorded in comment
  `01M5MCXTQA3JE7JKS52PPNFNFC`.
- The CLI smoke test (`cargo run --locked --bin manyhands-cli`) exited 0, and
  so did `scripts/check-cli-no-gpui.sh`.
- The desktop smoke test (`cargo run --locked --features desktop --bin
  manyhands`) launched on the active display and ran until a 15-second timeout
  stopped it (exit 124), with no crash.
- After the merge, `cargo fmt --check` exited 0, and
  `repository_enablement unusable_identity` passed 4 of 4.

**For C2 owner verification.** The owner's earlier partial request
(`01M5MA6DJRYF77RVB9E01EX1DX`) stays as it was recorded. The following comes
from the reviewer reading the code; it has not been run. Sending the request
again with the same ID and the bracketed input should now end in a definite
`invalid_input`. The request would stay accepted, with its pending row and
recorded effects kept, but it could not complete with that input. To clear it, run `operation abandon`, then
start a fresh `repo create`. This follows the ticket's rule to preserve
recovery evidence.

**Follow-up proposed, not filed.** Sync and promotion still choose their
identity in a different way (`select_committing_identity` and
`effective_identity` in `src/repository/remote/sync.rs`). They use only the old
empty/NUL check. A confirmed bracketed identity there would be written to
local configuration, and the commit would then fail at `Signature::now`.

Native CI was not dispatched. Push, PR, merge and closure each need owner
authorization.
