---
slug: vh-z2d7f
id: 01M67Y754SJ1FTYQJFR37Z4ZTY
title: Let the SSH transport authenticate through a running ssh-agent
type: task
status: open
project: manyhands
team: core
manyhands_managed: true
manyhands_kind: ticket
---
Let Manyhands' SSH transport authenticate through a running ssh-agent, so a
passphrase-protected key that the agent already holds can publish without a
prompt, including in non-interactive runs.

Found by Wave 03 C4's dogfooding (`01M4CC0VMWEQQ9G4W0DAG7ADPY`) on
2026-10-30. The owner asked for this ticket, and expects agent support to be
useful beyond comments.

## Today

- Manyhands registers a key by reference to its file. Each connection
  unlocks the key itself, so a protected key asks for its passphrase at a
  terminal.
- In `--non-interactive` mode, a protected key answers `unlock_required`.
  For a publishing verb (`comment add`, `item sync`, `repo sync`,
  `item republish`), that stop holds the repository's remote reservation.
  Other syncs and comments then answer `busy` until the same request is sent
  again at a terminal, or cancelled and sent once more.
- The owner's key is protected and is already loaded in ssh-agent, which
  Git uses. Manyhands does not use the agent.
- So agents cannot publish through Manyhands. C4 deferred switching agent
  comments to `comment add` for this reason (C4 design, Dogfooding step 4).

## Expected behaviour

- When an ssh-agent is reachable (`SSH_AUTH_SOCK` on Unix; the OpenSSH agent
  or Pageant on Windows, as the plan decides) and it holds the **selected**
  key, the transport authenticates through the agent, with no passphrase
  prompt and no `unlock_required`.
- It matches the selected key by its public key or fingerprint. It never
  offers the agent's other keys.
- With no agent, or an agent that lacks the key, it behaves as today: a
  prompt at a terminal, and `unlock_required` without one.
- No key material or agent detail reaches output, logs or stored results.

## Acceptance criteria

- Library and SSH-fixture tests with a test agent cover:
  - agent authentication of a protected selected key, non-interactive;
  - an agent that lacks the key, falling back as today;
  - the agent's other keys never offered.
- A non-interactive `comment add` and `item sync` publish with an
  agent-held protected key.
- The credentials and transport RFC text is amended, and the desktop
  benefits through the shared library.
- Follow-on: revisit C4's deferred comment switch, so that AGENTS.md
  directs agent comments to `comment add`.
- Shared-library change under the Wave track rule: reviewed and merged to
  main on its own.
