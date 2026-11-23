---
id: "01M6MKMPHJBFGKFGP4QGD9B4R4"
item_id: "01M6KHVD6D0DB8KYT6VGQP10Y0"
parent_id: "01M6MJDR01K71MT2TKBP560Z8W"
created_at: "2026-11-05T01:02:16Z"
created_by: "GPT-6.1 Sol <noreply@openai.com>"
manyhands_managed: true
manyhands_kind: comment
---

## Checkpoint correction: Task 2 adversarial mechanism findings

Clean correction entry `85bc17d7e9f5ae28c82ecd1f85698a0a030431f8`; full Task 2
base remains `7b1912d152a1ab7440bd17c48a0a544e40e19258`. Same ticket branch/worktree;
`origin/main` unchanged at `c69789d97abd4c514ef4d4ac62f3a4fa47ca30f4`.

Controller accepted **2 Important internal mechanism findings** in the first
adversarial checkpoint. Verified both against actual source; revised only the
execution ledger and this canonical record, not approved Cycle/design policy:

1. Original P's advertisement precedes target processing, final frozen image and
   original-handoff eligibility. The revised seven-table proposal separates
   immutable exact **advertisement provenance** captured under owned UNPLANNED P
   from immutable **consumption binding** created with the full manifest in owned
   pre-terminal finalization. Eligibility gates consumption/child begin, not
   initial capture or binding insertion. No placeholder mutation, second ad,
   current-snapshot guess or public replay supplies authority. Unbound/ineligible
   source holds have explicit fenced/terminal release lifetimes.
2. Final unpin after a G→G+1 config fence otherwise strands a historical batch
   that startup refuses. Exact pin release now includes an immutable release
   receipt and conditional obsolete-**noncurrent** batch collection in the same
   transaction; any other pin or integration window retains the batch. Only old
   observation children cascade. Current projection, D14 materialization, tracking,
   provenance/binding/effect/result history are not deleted or rebound. Released
   scalar batch IDs cannot be re-resolved on replay, including SQLite ID reuse.

The ledger adds the verified counterexamples/resolutions and six new proposed
test groups (39–44), for **44 proposed groups** total. These include the full
UNPLANNED→one-ad→manifest/binding→eligible→consumption ordering, failure/crash and
unbound-pin lifetimes, **successful reopen after final release**, shared-pin/window
retention, FK/history/D14 preservation and scalar-ID-reuse negatives.

**Evidence limits:** no production code, tests, schemas or Cargo changes; tests
written 0, RED 0, GREEN 0; no Cargo/SQL/runtime probe. No agent, push, other-worktree
edit, real-context action, main integration or native CI. Required coauthor applies
to the documentation-only correction commit.

**Still stopped:** checkpoint is not approved. Controller sends the corrected
proposal to the same adversarial reviewer before this implementer begins TDD or
proceeds past step 9. No review pass, Task 2 completion or new owner-policy decision
is claimed.
