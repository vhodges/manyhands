---
manyhands_managed: true
manyhands_kind: comment
id: "01M5JYBTSGT87AWGD9CSWAWS8N"
item_id: "01M4CC0VMTTNKVYS6XWQG4CK1N"
created_at: "2026-10-22T23:15:29Z"
---

## Task 9 — Key Generation And Masked Passphrase Entry

Implemented from `c40ffbc223579508ab65d9f965be2d89116389d9`, inline without
subagents. Controller reports Task 8 spec/quality approval in
`ses_e9a259bf1ffeSai2cfbB7LmiBt` / `ses_e9a23eb6bffeNga7SWg3sMfCgI` and its push.

Generation now accepts the closed label/protection input and uses the existing
F2 mutation. Protected execution prompts twice, moves the zeroizing secret once
and disables automatic retries; noninteractive execution/replay is unlock_required.
Prepare asks nothing and delegates not_confirmable to F2. Shared flag handling
allocates and prints the interactive request before input/store/prompts, retaining
it on CLI refusals and preserving explicit-ID replay lookup semantics.

Added 15 tests (14 CLI, one bin), two registered JSON goldens and the schema/example.
Unix PTY cases prove masking/decryption, mismatch exhaustion, Ctrl-C, live early
request-ID output, prepare and R-19 replay/request-show. UTF-8/UTF-16LE scans cover
all data/home files and all output/transcript bytes. A separate output privacy
table covers exactly all 15 available C2 verbs in both modes using C1 sentinels.

RED before production: keys 17 pass/4 fail, prompts 9/6, privacy 0/2, schemas 5/1.
Final targeted GREEN through Devenv, Cargo locked except fmt: keys **21**, prompts
**15**, privacy **2**, input_schemas **6**, exits **5**, key_generate human/prepare
**2**, help **9**; bin mutate **14**, bin key_generate **1**. Counts overlap.
Goldens regenerated with the explicit update variable outside CI, then compared
without it and reviewed. Formatting and diff whitespace checks passed.

Two new fixture assumptions corrected against existing library semantics:
0700 directory, 0600 private key, 0644 public companion; privacy enable uses a
not-yet-enabled repository so the requested prefix can be set. No library or
pre-existing assertion change. Exact 49/52 audit union retained, staged
unlock_required replaced with real binary evidence. Full details in the ledger.

Self-review found no unresolved blocker. This comment's containing commit is the
Task 9 delivery, `feat: add key generate with masked passphrase entry`, with the
required coauthor. **Post-commit Part B gate and independent controller reviews
pending.** Part B review base is `d9496b9`. Ticket remains open.

ID allocated through built CLI `id new` with fresh temporary HOME/XDG/app-data,
empty inherited environment, disabled Git configuration and detached session.
