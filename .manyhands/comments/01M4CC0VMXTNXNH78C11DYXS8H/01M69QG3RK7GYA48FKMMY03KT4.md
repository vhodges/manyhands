---
manyhands_managed: true
manyhands_kind: comment
id: "01M69QG3RK7GYA48FKMMY03KT4"
item_id: "01M4CC0VMXTNXNH78C11DYXS8H"
parent_id: "01M69GTSPKC3SRDCEQVNMQBSRM"
created_at: "2026-10-31T19:38:01Z"
created_by: "Claude Opus 5.5 <noreply@anthropic.com>"
---

## Task 2a: lifecycle binding, prepare and first run

- **Base.** `05b8d4f0`, with `origin/main` `25a8174d` merged first as `961ae44a`.
- **Delivered.** `document promote` and `ticket close` at the boundary: every predicate arm, `lifecycle_parent`, the `Session` methods, the preview, blocked and mutation data with schemas and 16 goldens, and conformance rows.
- **Verified.** `c5_lifecycle` 32 cases, `mutation_contract` 44, `read_contract` 72, `mutation_replay` 383, `mutation_confirmation` 183, `mutation_bridges` 166, `--lib mutation` 126; fmt, clippy and the desktop commands. The ledger has the times.
- **Not yet working.** A resend of an unfinished request answers `recovery_required` without calling the domain until Task 2b, and `request cancel` reaches nothing in a run until Task 2c.
- **Outside the task's file list.** Compile-required arms in three test support files and `host.rs`, and enumeration rows in `tests/read_contract.rs`. None changes behaviour; the ledger lists them.
