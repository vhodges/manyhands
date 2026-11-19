---
id: 01M7R2ME33PNVE2BQ4KZCZNRKX
item_id: 01M5CS4MH25QQ769HV61ZRKHXG
created_at: 2026-11-18T19:41:02.101919965Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Superseded by `01M7R2GPPF7F7JRKHX8C5ZZ4PF` (vh-4y6na, user-visible desktop gaps and correctness holes) and `01M7R2GPPF135ZT7M4TZCR09XT` (vh-fdp7z, hygiene, test support and tidy-up). Open items were checked against main at `5d80bc15` on 2026-11-18; the code was read, no tests were run.

**Done by Wave 04** (see the two earlier comments here). Cycle 03a (`01M784Y8C73B39GR4JWENXE5D0`): the two `aria_selected` signals and `RepositoriesPane::set` notifying during render. Cycle 03b (`01M784Y94Q3AXT6XJX38F5VK1V`): "Finishing" hiding the running stage (`view/operations.rs:56-62`).

**Found already fixed or moot in the code:** `#[must_use]` on the `DelayingBackend` hold handles; the item view's Repair action (now a live "Repair…" banner button, so there is no disabled state to assert); a parent read after a failed item read (`view/item/reads.rs:51-53`); `row_focus` pruning (`view/relationships.rs:176`); a parent that turns Unresolved keeping its name (`model/relationships.rs:106`); a failed read of blocker names (there is one `list_tickets` read now, with ULID fallback); Find matches on refresh (Find became an ID-only lookup in `1568da1b`).

**Moved to `01M7R2GPPF7F7JRKHX8C5ZZ4PF`:** a conforming comment's own problems are never shown, two Refresh buttons on `item_not_found`, no failure text on the Dependencies group, tab element IDs keyed by strip index, the `local_time` hardening, and the quadratic `build_tree`.

**Dropped, with reasons:** the document list key collision (`document:{worktree}:{path}`; the risk is low and nothing reaches it today); the single remaining raw `git2::Repository::init` in `services/reads_tests.rs:265`; and the "busy, failed and double inspection paths" bullet (too vague to check; the Add sheet covers them in `add_repository_tests/checks.rs`).
