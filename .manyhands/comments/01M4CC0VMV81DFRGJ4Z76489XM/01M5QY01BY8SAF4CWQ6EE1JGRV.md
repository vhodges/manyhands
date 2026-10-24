---
id: 01M5QY01BY8SAF4CWQ6EE1JGRV
item_id: 01M4CC0VMV81DFRGJ4Z76489XM
created_at: 2026-10-24T21:59:02Z
created_by: Claude Opus 5.5 <noreply@anthropic.com>
manyhands_managed: true
manyhands_kind: comment
---

## Planning checkpoint: drafts written, review pending

**Base.** The ticket branch merged `origin/main` at `e49f978` as merge
commit `9ed4a04`. It was merged, not rebased. No build or test was run for
planning.

**Drafts** (status `draft`):

- `docs/Cycles/wave-03-cli-03-authoring-and-discovery.md`
- `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-design.md`
- `docs/plans/2026-10-24-wave-03-cli-03-authoring-and-discovery-implementation.md`

**Owner decisions, 2026-10-24.**

1. No defect first. `01M4PB82`, `01M4PB80` and `01M4PB7Y` are known limits.
2. The save patch is filled by the CLI from one `show_item` read, with its
   replay limit accepted.
3. Dogfooding happens in the final task, with the owner's go-ahead at that
   step. It records the Wave 03 Cycle deps on C4, C5, D4, D5, D6 and G1, and
   switches AGENTS.md and the `file-ticket` skill to the CLI.
4. One Cycle in two Parts, each with a gate that includes `--all-features`
   and an independent review.

Stated before the questions, and not objected to:

- no `--context` on mutation verbs. The multi-item ticket's later text
  governs over its design's earlier lines;
- `path observe` reports what is at a path in the named context;
- bodies travel as JSON strings only;
- identity recovery is a pass-through.

**Found in planning.** The G1 worktree held an uncommitted title edit
(" External Edit") and an untracked `ticket.md~`. The owner cleaned it on
2026-10-24, and it was re-checked clean at `7b306d1`. The dogfooding preflight
stays.

**Review round 1, at `6c25c43`.**

- **Contract and consistency:** 38 findings, 2 blocking, 21 important and
  15 minor.
- **Source and feasibility:** 22 findings, 2 blocking, 7 important and
  13 minor.

What the findings changed:

- the fill read's refusals;
- `null` on create;
- outcomes stated by value, because every write re-emits the front matter;
- privacy checked in the request stores;
- the dogfooding preflight, value checks, stop and resume rules, and the
  scope and review of the switch;
- replay and cache-loss cases;
- corrected fixtures and codes.

Not applied, because the source showed otherwise: contract findings 10 and
33, and source finding 20(b).

**Owner decisions after review, 2026-10-24.**

5. The fill read's answer passes through under the mutation's name, and the
   CLI RFC's refusal rule is amended.
6. Exit 4 is a reason entry backed by the library-actor test. Exit 5 is
   reached through the binary.

**Open questions.** None. The scoped re-review of the revisions is next.

**Proposed execution.** Subagent-driven, as C2 was.
