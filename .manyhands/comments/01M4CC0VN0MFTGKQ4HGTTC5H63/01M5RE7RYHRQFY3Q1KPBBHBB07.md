---
manyhands_managed: true
manyhands_kind: comment
id: "01M5RE7RYHRQFY3Q1KPBBHBB07"
item_id: "01M4CC0VN0MFTGKQ4HGTTC5H63"
created_at: "2026-10-25T02:48:18Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Planning checkpoint

**Base.** This ticket branch merged `origin/main` at `e49f978` as `98f49b7`,
with no conflicts and no rebase.

**Drafts.** The three planning documents are written as drafts:

- `docs/Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md`;
- `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md`;
- `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-implementation.md`.

**Product-owner decisions (2026-10-24):**

1. **Load fidelity.** No fallback editor. The pinned editor's whole load
   normalization is accepted, including inside code. The host converts CR
   line endings to LF before loading.
2. **Context provisioning.** Edit and Create provision the context through
   a new F2 `context prepare` binding. It is built inside this ticket as
   prerequisite Part 0, library-only, reviewed at its own checkpoint. This
   departs from track rule 5.
3. **Create.** Create provisions the context; the first Save creates the
   item.
4. **Stale-edit review.** Per field, then save.
5. **Shape.** One Cycle: Part 0, then Parts A–D.
6. **Read view.** The read-only view renders with `zorite-markdown`.
7. **Outline defaults, not changed by the owner:**
   - known library defects are not prerequisites;
   - the pin stays at 0.10.0;
   - Save is offered only for a real change;
   - front matter is preserved by value;
   - Windows and macOS evidence is G1's.

**Findings that shaped the plan:**

- **The editor's load rewrite is wider than the desktop RFC records.** It
  has two `$$` rewrites, and they apply inside inline code, `~~~` fences,
  indented code and HTML. The design records it as N-1 to N-4.
- **No F2 command provisions a context on its own.** Hence decision 2.
- **A library document save always rewrites the file**, so the desktop
  offers Save only when the item is dirty.

**Not done yet.** Independent review has not run. Nothing was built or
tested.

**Next.** The two independent reviews (source and feasibility; contract and
consistency), then the owner's approval.
