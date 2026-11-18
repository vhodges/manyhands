---
id: 01M7PB1K6NQPM9EJWTEN6JWVH6
item_id: 01M67Y754SJ1FTYQJFR37Z4ZTY
created_at: 2026-11-18T03:26:13.341923828Z
created_by: Vince Hodges <vince@imbas.ca>
manyhands_managed: true
manyhands_kind: comment
---
Code review 2026-11-17 (`/code-review high`), decision: the owner reviewed the findings and approved closing without follow-up work. Nothing is blocking; if someone hits one of these, open a ticket then.

Known and accepted:
- A failed agent authentication (the agent holds the key but the server rejects it, or the agent refuses) costs a second connection before the passphrase flow.
- `UnixStream::connect` to the agent has no timeout, and the read timeout is per read, not a total deadline, so a wedged agent socket could stall an operation.
- An agent that can sign proceeds before the `blocked_unlock` check, as an unprotected key already does. Cancelling an unlock prompt does not stop automatic polls the agent can sign for.
- RSA is out of scope (an RSA selected key uses the key file flow), no test pins that row, and ECDSA parsing and signing are not exercised by any test. Only ed25519 is proven end to end.
- The one `unsafe` shim (transmuting a raw libgit2 credential into `git2::Cred`) rests on size and alignment assertions, the exact `git2` pin and a unit test. Upstreaming `Cred::ssh_custom` to git2-rs would remove it.
- The agent socket is read from `SSH_AUTH_SOCK` once when the service opens; the CLI does so per run.
- Still unverified: a real `ssh-agent` and macOS.
