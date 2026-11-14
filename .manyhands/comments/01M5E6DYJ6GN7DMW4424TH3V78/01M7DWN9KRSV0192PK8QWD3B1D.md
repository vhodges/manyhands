---
id: 01M7DWN9KRSV0192PK8QWD3B1D
item_id: 01M5E6DYJ6GN7DMW4424TH3V78
created_at: 2026-11-14T20:46:33.260677265Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Note from Wave 04 Cycle 05a (Common Frame, Add Repository And Unlock Key, ticket `01M7D5Z7FZTGTC1HE25KQ1MYQY`), approved 2026-11-14 and not yet implemented.

Cycle 05a restyles the desktop's Unlock key dialog. The design handoff (`docs/design/05-sheets-and-dialogs.md`, Unlock key dialog) asks that "a wrong passphrase shows an inline error under the field and keeps the focus there". The owner chose to leave that out for now (Cycle 05a decision 4), because the library reports a wrong passphrase as `transport_unavailable`, which this ticket fixes.

So, in Cycle 05a:

- Unlock closes the dialog as before, and a wrong passphrase fails the job as it does today.
- The inline error covers only what the dialog refuses itself (an empty passphrase, or one containing a NUL).
- `05` gets an "As built" note saying so, pointing here.

**What this changes for this ticket:** once the library answers `unlock_failed` for a wrong passphrase, the desktop's inline error is follow-on desktop work. It is not part of Cycle 05a. Plan it with this fix, or as its own ticket.
