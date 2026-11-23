---
id: 01M7R2ME33ZVHX4NF34XCVVA24
item_id: 01M5F3GVDYTH1Q5V5FCS9YF7V3
created_at: 2026-11-18T19:43:00.755975836Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Superseded by `01M7R2GPPF7F7JRKHX8C5ZZ4PF` (vh-4y6na, user-visible desktop gaps and correctness holes) and `01M7R2GPPF135ZT7M4TZCR09XT` (vh-fdp7z, hygiene, test support and tidy-up). Open items were checked against main at `5d80bc15` on 2026-11-18; the code was read, no tests were run.

**Done by Wave 04** (see the three earlier comments here). Cycle 04 (`01M7AMAPNHHTNECXBF4CA5WPKV`): the shared scroll offset, reopening Settings with a newer selection, the double list read, the key-delete focus fallback and the identity prefill. Cycle 05a (`01M7D5Z7FZTGTC1HE25KQ1MYQY`): the Browse reason, the chooser text, Retry for a busy inspection, non-UTF-8 roots, and form clearing (decisions 12 and 16, which also answer the product question about clearing forms).

**Found already fixed or moot in the code:** the Escape-on-sidebar-Remove test (the sidebar Remove is now the More menu, and Escape is covered in `repositories_tests/toolbar.rs`); busy, failed and double-Add inspection tests; remove-then-register-again (`repositories_tests.rs:409`); a second `observe_host` while connecting (`settings/hosts.rs:455`); `open_a_document`'s assertions (`keyboard_tests.rs:307`); the key-delete typed-failure tests.

**Moved to `01M7R2GPPF7F7JRKHX8C5ZZ4PF`:** a refused host approval offers no Replace (`view/confirm.rs:978-1010`), the hidden "From:" line, the close buttons inside `Role::TabList`, and a check of what a recovery for another repository does while a host check is connecting.

**Moved to `01M7R2GPPF135ZT7M4TZCR09XT`:** the remaining test gaps (absent-path Create preview, Partial effects rows, the non-protected Retry via `retry(id, None)`, key-source lines and busy or failed Copy, the non-busy key generate failure).

**Dropped, with reasons:** the `RemoteAdd` raw URL in two records (hardening with no known defect); "No SSH publication remote" next to "Connecting" (a rare edge case); the `NOT_AVAILABLE_YET` reword (the symbol no longer exists); the Kit's `defer_close_dialog` focus restore (the symbol is not in this repo, so it is a Kit-side concern); the `on_handle` visibility assertion and focus-after-removal (test-only, no known defect); the UI-changes product question (Wave 04 is that rework).
