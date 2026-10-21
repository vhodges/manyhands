---
manyhands_managed: true
manyhands_kind: comment
id: "01M5F9J0ABY8D78M0HHPJ45JT7"
item_id: "01M5CS4MJ0G7B0MPJD0KN0QMYA"
created_at: "2026-10-21T13:14:08Z"
---

## Task 5 complete; final verification begins

Split ItemPane into the 150-line root state/constructor/composition and private
details (310), reads (157), status (121), source (85), discussion (97) modules.
The relationship view keeps its existing location and `item::text` helper path.
Read ordering/stamps, textarea entity/selection, retained title, rendering order,
copy actions and refresh completion handling remain unchanged.

The first compile identified missing `StatefulInteractiveElement` imports in
three extracted rendering modules. Restored the original trait in those scopes;
no policy, behavior or test edits were needed. All planned commands then passed,
using `devenv shell -- cargo test --locked --features desktop-test-support --bin manyhands <filter>`:

- `desktop::view::item`: 15 passed.
- `desktop::view::relationships`: 8 passed.
- `desktop::view::tabs`: 17 passed.
- `desktop::view::scroll`: 2 passed.
- `desktop::view::keyboard`: 12 passed.

All 54 passed with no failures/ignored tests. Formatting and diff checks passed.
Re-listing the whole desktop gives 362 tests and unchanged inventory hash
`a20c47450f3a8613a7c0b90ff1fca93bddf53ab8e3bdbb39e65cc0bcdd291343`. Test diffs
are still exactly the five authorized one-line accessor substitutions.

Fetched main before final gates: still `0b23664`, already merged. The required
`git merge-tree --write-tree HEAD origin/main` passed, tree
`36b698142c3890d2986363b957bff2423d8680d3`. HEAD is still `8fd42e7`, so this
check proves the committed base is current, not that the uncommitted source was
merged; repeat after an authorized checkpoint commit. No shared-library, CLI,
integration-test, manifest, lockfile or CI change is present.

Proceeding to final all-feature gates and independent code review. The work is
uncommitted and the ticket remains open.
