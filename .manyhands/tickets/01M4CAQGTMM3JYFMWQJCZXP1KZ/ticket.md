---
manyhands_managed: true
manyhands_kind: ticket
id: "01M4CAQGTMM3JYFMWQJCZXP1KZ"
title: "Replan Wave 03 into a foundation and parallel CLI and desktop tracks"
type: "task"
status: "open"
project: "manyhands"
team: "core"
wave: "03"
---

Wave 03 was approved on 2026-10-05 as thirteen Cycles in one sequence, CLI
first. On 2026-10-07 the product owner asked to replan it into two tracks that
can run in parallel, accepting one or two serial prerequisite Cycles that both
tracks depend on. This ticket records that replan. It changes planning
documents only.

## Decisions

The product owner selected all three recommendations on 2026-10-07:

1. **Foundation scope.** Two serial, library-only foundation Cycles (F1 read
   boundary and result model; F2 request replay, confirmation and shared
   mutation bridges), proven by library integration tests. No CLI verb or
   desktop screen is delivered by the foundation.
2. **Wave 02 entry gate.** Each Wave 03 Cycle is gated by the Wave 02 Cycles it
   consumes. The complete Wave 02 integration gate moves from "before the
   first Wave 03 Cycle" to "before G1 exits".
3. **Naming.** Cycles are identified as F1–F2, C1–C5, D1–D6 and G1, with
   documents named `wave-03-foundation-NN`, `wave-03-cli-NN`,
   `wave-03-desktop-NN` and `wave-03-gate-NN`.

## Resulting shape

Fourteen Cycles; the longest serial path is nine (F1, F2, D1–D6, G1) instead
of thirteen. The mapping from the 2026-10-05 numbering is in
[Wave 03, Structure And Identifiers](../../../docs/Waves/wave-03-dogfooding.md#structure-and-identifiers).

## Changed documents

- [Wave 03](../../../docs/Waves/wave-03-dogfooding.md): entry gate and per-Cycle
  Wave 02 dependency table, delivery approach, track rules, all Cycle
  sections, capability ownership, traceability, risks and approval record.
  Front matter `status` is `approved`.
- [RFC approval register](../../../docs/RFC/wave-03-rfc-review.md): the
  thirteen-Cycle sentence.
- [MVP architecture RFC](../../../docs/RFC/mvp-rfc.md): it required every Cycle
  document at `wave-NN-cycle-NN-<slug>.md`. It now permits a Wave to organize
  Cycles into tracks and name their documents `wave-NN-<track>-NN-<slug>.md`.
  This amendment to an approved RFC was approved with the replan.
- [API audit](../../../docs/research/wave-03-api-audit.md),
  [readiness exploration](../../../docs/research/wave-03-readiness-exploration.md)
  and [editor feasibility](../../../docs/research/wave-03-editor-feasibility.md):
  a replan note with the number mapping. Their evidence rows are not rewritten.

## Approval (2026-10-07)

The product owner approved the revised documents, including the consequences
below and the MVP architecture RFC amendment, and asked for the status flip
and a commit. The product owner also approved `zorite-editor` as the editor;
the Wave document and RFC register record that selection and leave its
source-preservation gap open for resolution before D3 planning. Merge to main
and ticket closure are not yet authorized.

## Editor byte-change ruling (2026-10-07)

After the commit above, the product owner accepted the byte changes made by
`zorite-editor`'s load normalization and asked for the RFC to be amended.
The [desktop/editor RFC](../../../docs/RFC/desktop-information-architecture-and-editor.md#editor-load-normalization-amended-2026-10-07)
gains an "Editor load normalization" subsection and an updated acceptance
bullet; the canonical RFC, RFC register row W3-01, the Wave document and the
feasibility record are aligned. The amendment's bounds were written by the
agent from that ruling: normalization is not a user edit, so opening and a
no-change save still write nothing; only layout-only rewrites are accepted;
metadata, the CLI and the headless library are unaffected.

## Consequences approved with the replan

These follow from the decisions and were settled while writing:

- F2 cannot prove promotion/closure confirmation against real fixtures, because
  Wave 02 Cycles 09/10 do not exist at its baseline. Whichever of C5 or D5 is
  planned first lands that binding as a shared-library change (track rule 6).
- Library bridges formerly owned by CLI Cycles 03/04 (identity configuration,
  host approval, folder creation, repair/adoption) move into F2, and the
  closure filter into F1, so D1 and D3 do not depend on the CLI track.
- Scenarios needing both front ends (a CLI edit against an open desktop draft;
  a CLI operation concurrent with the desktop poller) move to G1. Tracks prove
  them with a second library-level actor.
- After F2, a headless-library change needed by a track gets its own ticket and
  merges to main first; front-end branches leave `src/repository*` unchanged
  (track rule 5).
- D4 requires Wave 02 Cycles 06 and 07, not 07 alone: a divergent sync must
  surface as a preserved conflict state.

## Infrastructure

- Branch: `manyhands/ticket/01M4CAQGTMM3JYFMWQJCZXP1KZ`.
- Worktree: `.manyhands/worktrees/01M4CAQGTMM3JYFMWQJCZXP1KZ`.
- Created from freshly fetched `origin/main` / local `main`
  `ae3d69035d901d352cb709eacaee5fc2b51084de`.
- CLI ticket operations are not implemented, so canonical files are used.

## Closure conditions

- The product owner approves the revised Wave document; its `status` returns
  to `approved` and the approval is recorded there and here.
- The change is merged to main. No Cycle ticket, Cycle document or code is
  created by this ticket.
