---
id: 01M4VX5NM6TMJ35B0048FWBBN7
item_id: 01K7F6H9J2N4Q6S8V0X2Z4B6DG
created_at: 2026-10-14T00:32:06Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---

Task 5 RED before production under owner-approved step-5-only exception.
Sequential Devenv commands: cargo test --locked --lib remote::poll => 29 passed,
7 failed, 525 filtered at missing materialization todo; cargo test --locked --lib
remote::state => 46 passed, 2 failed, 513 filtered (missing table columns and
dependent missing loss-test trigger); cargo test --locked --test
repository_enablement => 77 passed, 1 failed (exact inventory missing table).
Each exit 101. All exact commands and trial details recorded in ledger.

Initial compile RED required new API types/signature/FailurePoint variants;
only behavior-free types and todo added. Discovery fixture signature corrected.
First runtime trial was fixture setup RED: reservation policy plan unconfigured.
Compared working fixture and configured synthetic plan; corrected runtime now
reaches placeholder in all seven new groups. No behavior/helper-body edits yet.
Partial schema loss is not exercised until the absent envelope exists.

Seven groups cover real repositories/ODB trees, once-only/reopen/later-call
ownership, two-service barrier, obstruction/redirect preservation, all four
public interruption boundaries including real Linux Git checkout failure,
conservative D2 retained partial state and reobservation. GREEN will execute all
matrix variants. No fake transport, sleeps, developer keys/global config changes
or cfg(test) production failure hook. Task 5 production begins after this RED.
Task 6 unstarted; ticket open; Task 2 SQL kind-immutability minor deferred final.
