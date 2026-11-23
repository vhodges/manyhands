---
slug: vh-fk279
id: 01M84H83Q65MCH42FYBS1WR5WP
title: Share main and other desktop copy say "main" instead of the repository's primary branch name
type: defect
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
The desktop calls every repository's primary branch "main", whatever it is named. A repository whose primary branch is `development` offers "Share main" in its menu and in Activity, and its row says "Not shared yet: main.", though what Share main pushes is `development`. Found 2026-11-23 by the owner, testing 0.2.1 on macOS with two newly added repositories, one on `main` and one on `development`.

Hard-coded strings, as of main `66a115c9`:

- `SHARE_MAIN` "Share main" (`src/desktop/model/activity/copy.rs:83`), used by the repository menu (`src/desktop/model/repository_state/menu.rs`).
- `SHARE_MAIN_SET_UP` "Share main…" (`src/desktop/view/sidebar.rs:13`).
- The `RepoSync` request label "Share main" (`src/desktop/services/requests.rs:457`).
- `SHARE_MAIN_SHARES` "Share main shares main." and `MAIN_NAME` "main" in the not-shared-yet sentence (`src/desktop/model/repository_state/words.rs:39` and `:41`).
- The `"main"` fallback for an unknown primary branch (`src/desktop/model/activity/requests.rs:149`).

Some copy already names the real branch: the lifecycle prepare's sharing line (`sharing_main(&primary)`, `src/desktop/view/lifecycle/prepare.rs:438`) and the local review sheet (R-35, `src/desktop/view/item/review.rs`). The repository's configuration carries the name (`entry.summary.configuration.primary_branch`), so the menu, the row sentence and Activity can use it too.

Expected: every label and sentence that names the primary branch uses its configured name ("Share development", "Not shared yet: development."). Planning decides the fallback when the name is not known yet (today "main"), and whether "main" stays anywhere as a word for the role: the Add repository form's "Main branch" field names the role, not a branch, and can stay.

Acceptance: desktop behavior tests of the strings the repository menu, the row sentence and Activity produce for a repository whose primary branch is not `main`, with no appearance tests (AGENTS.md Testing Strategy). Desktop-only, so the fast tier only.