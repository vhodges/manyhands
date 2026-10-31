---
title: "Future Ideas"
date: 2026-10-15
draft: false
tags:
  - ideas
  - not scheduled
author: "Vince Hodges <vince@imbas.ca>"
manyhands_managed: true
manyhands_kind: document
id: "01M5MHD4JADFCMEC8DHJRJBXNJ"
---
# Future Ideas

Ideas worth keeping that are not near-term work. Tickets are for work that
will happen soon; an idea moves out of here into a ticket when it becomes
that.

## Child processes inherit libgit2's network socket (macOS, Windows)

*Noted 2026-10-15, from Wave 02 Cycle 08 native CI.*

**What.** libgit2 creates its TCP socket with `socket(... | SOCK_CLOEXEC)`
(`src/libgit2/streams/socket.c`) and defines `SOCK_CLOEXEC` as `0` where the
platform lacks it (`src/util/posix.h`). macOS has no `SOCK_CLOEXEC`, and Windows
sockets are inheritable by default, so on both platforms any child process
started while a libgit2 connection is open receives a copy of the socket.
Linux is unaffected.

**Impact today: none in production.** Manyhands' production code starts no
child processes, and the CLI is short-lived. The Cycle 08 SSH test fixture,
which runs `git upload-pack` in the client's own process, did hit it: an
interrupted fetch's connection never closed. The fixture now bounds and abandons
a stalled relay (`tests/support/ssh_server.rs`).

**Impact if triggered.** Manyhands itself keeps working: operations complete
or interrupt normally. But a connection Manyhands closes stays open while the
child holds its copy, so the server's `upload-pack`/`receive-pack` lingers until
the child exits or the server times out. Git applies ref updates only after a
complete exchange, so the remote cannot be left half-updated. The child sees
only encrypted SSH traffic, no keys or session state, and runs as the same user;
at worst it could break the connection.

**When to act.** Before Manyhands starts child processes while network work may
be running: an external editor, hooks, credential helpers, the `git` CLI, or
anything launched alongside background polling in the desktop app.

**Options.**

1. *Fix libgit2 upstream* (the real fix): draft issue below. Then a libgit2 and
   `libgit2-sys` release, or a patched `libgit2-sys` carried meanwhile.
2. *Spawn children that inherit nothing beyond stdio* (mitigation, our own
   spawns only): on macOS `posix_spawn` with `POSIX_SPAWN_CLOEXEC_DEFAULT`, or
   close descriptors above 2 in `pre_exec`; on Windows `CreateProcessW` with
   `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` (Rust std has no stable API for it, so a
   small wrapper).
3. *Not practical:* setting the flag from outside libgit2. `git2` does not
   expose the socket; scanning descriptors in a callback is racy and has no
   reasonable Windows equivalent.

### Draft libgit2 issue

> **Title:** Network socket is inheritable by child processes on macOS and
> Windows (`SOCK_CLOEXEC` fallback is a no-op)
>
> **Version:** libgit2 1.9.7 (via `libgit2-sys` 0.18.8). Not yet checked
> against current `main`.
>
> **Description**
>
> `git_socket_stream` creates its socket with:
>
> ```c
> s = socket(p->ai_family, p->ai_socktype | SOCK_CLOEXEC, p->ai_protocol);
> ```
>
> (`src/libgit2/streams/socket.c`), and `src/util/posix.h` defines
> `SOCK_CLOEXEC` as `0` where the platform does not provide it. On macOS, which
> has no `SOCK_CLOEXEC`, the socket is therefore created without close-on-exec,
> and nothing sets `FD_CLOEXEC` afterwards. On Windows, sockets created by
> `socket()` are inheritable handles, and `HANDLE_FLAG_INHERIT` is not cleared.
>
> As a result, any child process the application starts while a connection is
> open (from any thread) inherits the connection. When the application then
> frees the remote, the TCP connection stays open while the child holds its
> copy, and the server-side `upload-pack`/`receive-pack` keeps running until the
> child exits or the server times out. Linux is unaffected because
> `SOCK_CLOEXEC` is applied atomically there.
>
> This affects the SSH transport, which runs libssh2 over this socket stream.
> (We believe the TLS streams built on it, such as Secure Transport on macOS, are
> affected the same way; we have not verified that.)
>
> libgit2 already guards its own process pipes this way
> (`util/unix/process.c` sets `FD_CLOEXEC`; `util/win32/process.c` clears
> `HANDLE_FLAG_INHERIT`), and sets `FD_CLOEXEC` on pack files (`pack.c`).
>
> **How we found it**
>
> An application test fixture runs an SSH server in the same process as the
> libgit2 client and starts `git upload-pack` for each connection. After the
> client interrupted a fetch (transfer-progress callback returned non-zero) and
> freed the remote, the connection never closed on macOS or Windows: the
> spawned `upload-pack` held an inherited copy of the client socket, and the
> server blocked writing into it. On Linux the same test closed within
> milliseconds.
>
> **Reproduction outline**
>
> 1. On macOS, start a fetch over SSH.
> 2. From the transfer-progress callback, start a long-lived child, e.g.
>    `posix_spawn` of `sleep 60`.
> 3. `lsof -a -p <child pid> -i` lists the remote's TCP connection.
> 4. Return non-zero from the callback and free the remote: the connection
>    remains `ESTABLISHED` until the child exits.
>
> On Windows the equivalent is visible as the socket handle in the child (e.g.
> with Process Explorer).
>
> **Suggested fix**
>
> After creating the socket, mark it non-inheritable on every platform, matching
> what `pack.c` and `process.c` already do. On Unix the flag is set
> unconditionally rather than only when `SOCK_CLOEXEC` is the `0` fallback,
> since glibc defines `SOCK_CLOEXEC` as an enum-valued macro that evaluates to
> `0` in `#if`:
>
> ```c
> s = socket(p->ai_family, p->ai_socktype | SOCK_CLOEXEC, p->ai_protocol);
> if (s != INVALID_SOCKET) {
> #ifdef GIT_WIN32
>     SetHandleInformation((HANDLE)s, HANDLE_FLAG_INHERIT, 0);
> #else
>     int flags = fcntl(s, F_GETFD);
>     if (flags >= 0)
>         fcntl(s, F_SETFD, flags | FD_CLOEXEC);
> #endif
> }
> ```
>
> On Windows, `WSASocketW(..., WSA_FLAG_OVERLAPPED | WSA_FLAG_NO_HANDLE_INHERIT)`
> would make it atomic. On macOS a small window remains between `socket()` and
> `fcntl()` if another thread spawns a process at that moment; no atomic
> alternative exists there.
>
> Happy to open a PR with this change if it is welcome.

## Dockable Drafts panel

*Noted 2026-10-22, from Wave 03 D2 planning.*

**What.** The Drafts surface becomes a panel whose visibility can be
toggled on and off and whose dock position can be set. The product owner
expects it to become a ticket.

**Where it starts.** D2 shows Drafts as a content page in place of the
tabs (D2 design, Drafts page, ruling R23). D2 keeps the page's contents
independent of where it is hosted, so the change is to the host and to the
routing in `src/desktop/view/workspace/content.rs`, not to the page.
