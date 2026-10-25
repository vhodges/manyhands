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

**Drafts.** The Cycle document, design and implementation plan are drafts:

- `docs/Cycles/wave-03-desktop-03-rich-text-and-source-authoring.md`;
- `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-design.md`;
- `docs/plans/2026-10-24-wave-03-desktop-03-rich-text-and-source-authoring-implementation.md`.

**Product-owner decisions (2026-10-24):**

1. **Load fidelity.** No fallback editor. The pinned editor's whole load
   normalization is accepted, including inside code. The host converts CR
   line endings to LF before loading.
2. **Context provisioning.** Edit and Create provision the context through
   a new F2 `context prepare` binding. It is built inside this ticket as
   prerequisite Part 0, departing from track rule 5.
3. **Create.** Create provisions the context; the first Save creates the
   item.
4. **Stale-edit review.** Per field, then save.
5. **Shape.** One Cycle: Part 0, then Parts A–D.
6. **Read view.** The read-only view renders with `zorite-markdown`.
7. **Outline defaults, not changed by the owner:**
   - known library defects are not prerequisites;
   - the pin stays at 0.10.0;
   - Save is offered only for a real change;
   - front matter is preserved by value, and the unedited body by bytes;
   - Tab indents inside the editor;
   - keyboard formatting uses the editor's own actions;
   - Windows and macOS evidence is G1's;
   - no separate ticket for the binding.
8. **Unsupported syntax.** Shown literally and edited in place, unlabeled.
   The RFC is amended.
9. **Images.** Local raster images inside the repository only: PNG, JPEG,
   GIF, WebP and BMP. No remote images and no SVG.
10. **Cross-document links.** An exact relative path, in the item's own
    context first and then on primary. Failing that, the nearest document
    with the same file name. `[[Title]]` matches by title, closest first.
11. **Context release.** Part 0 also adds `context release`, used when an
    Edit or Create is left without saved work.

**Review round 1, at `b32c737`.**
- **Source and feasibility:** 2 blocking, 13 important, 11 minor.
- **Contract and consistency:** 5 blocking, 21 important, 14 minor.
- Each finding was checked against source.
- The contract-level findings became decisions 8–11, asked and answered.
  The rest were applied in the revision.

**Not done yet.** Round 2, a scoped re-review with the adversarial brief,
is next. Nothing has been built or tested.
