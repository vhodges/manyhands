---
manyhands_managed: true
manyhands_kind: comment
id: "01M46Q3F4TRNX5MHKPJJ2H1AD2"
item_id: "01K7F6H9J2N4Q6S8V0X2Z4B6DB"
created_at: "2026-10-05T19:02:00Z"
---

Task 2 implemented in e68561b..0a184b4 and independently approved for spec
and quality. Real SSH fixture passed20 cases, including selected-key identity,
encrypted Ed25519, fetch/push OIDs, restricted commands/auth, and helper cleanup.
Fresh check, fmt, all-target/all-feature clippy, runtime regression, and CLI
smoke passed through Devenv. Both Windows feature trees include the required
OpenSSL flags; native Windows/macOS/ARM execution remains pending authorized CI.

Pre-thread startup reads back10,000/30,000ms defaults. Stalled handshake/auth/
advertisement/transfer reads failed at approximately30s; a progressing fetch
succeeded at42s. Delayed exec acknowledgement returned error after45s with
internal cleanup, consistent with accepted extra-call budgets, not an overall
deadline. No watchdog expiry was treated as timeout proof.

Review minor retained for Task5/final review: custom harness must parse or
reject unsupported value-taking options such as --test-threads1, rather than
accidentally filtering every case. Task3 now builds host pins, recovery trust
marker, and selected-key callbacks on the reviewed fixture.
