//! Whose an operation's row is, as far as this machine can tell: the host a
//! row records is this one's name, read here, and a row is live when its
//! holder is a process that still exists on this host.
//!
//! A row is live when its indexing heartbeat is fresh, or when it names this
//! host and a process id that the system still knows. Anything else, another
//! host, an unknown host or pid, or a process that is gone, is stale. The
//! caveat is a reused pid: after the holder died, the system may give its pid
//! to an unrelated process, and the stale row then looks live. That only ever
//! lengthens a wait, which ends in `busy`; liveness never leads to a takeover
//! of a row that is not provably stale. A process must not judge its own row
//! by it: its own pid always exists. `holder_liveness` is the judgment every
//! reader makes: it judges a row this very process holds by its heartbeat
//! alone, a remote row of another process on this host by that process, and
//! a local row of one by whether its call still works under it.

/// What liveness needs to know of an operation's row. It is plain data; the
/// caller reads the row and computes the heartbeat, so nothing here touches
/// the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributionRow {
    /// The host that last claimed the row, when it recorded one.
    pub host: Option<String>,
    /// The process id that last claimed the row, as SQLite stores it. A
    /// value that is not a positive `u32` is unknown.
    pub pid: Option<i64>,
    /// The row is in `indexing` and its heartbeat is fresh by the existing
    /// rule (`INDEX_OWNER_STALE_AFTER`). The caller applies that rule.
    pub index_heartbeat_fresh: bool,
}

/// Whether a row's holder is still working.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    Live,
    Stale,
}

/// Asks the system whether a process exists.
pub trait ProcessProbe: Send + Sync {
    /// True when a process with this id exists on this host, whatever its
    /// owner.
    fn exists(&self, pid: u32) -> bool;
}

/// Judges a row: live on a fresh indexing heartbeat, else live only when the
/// row names `this_host` and a positive pid that `probe` finds. Anything
/// unknown is stale.
pub fn row_liveness(row: &AttributionRow, this_host: &str, probe: &dyn ProcessProbe) -> Liveness {
    if row.index_heartbeat_fresh {
        return Liveness::Live;
    }
    if row.host.as_deref() != Some(this_host) {
        return Liveness::Stale;
    }
    let Some(pid) = row
        .pid
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid > 0)
    else {
        return Liveness::Stale;
    };
    if probe.exists(pid) {
        Liveness::Live
    } else {
        Liveness::Stale
    }
}

/// How the call that holds a row works under it, for `holder_liveness`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Worked {
    /// A local row: its call works under the repository's lease, or, once
    /// it has let the lease go, under a fresh heartbeat. `lease_held` is
    /// whether a call holds the lease now, as the reader looked without
    /// waiting.
    UnderLease { lease_held: bool },
    /// A remote row: its call holds no lease and keeps no heartbeat, and
    /// only its process says whether it still works.
    ByProcess,
}

/// Judges a row as a reader in this process judges it. A row that names
/// this host and a known pid is judged by that process:
///
/// - this very process, `this_pid`, is live only on a fresh indexing
///   heartbeat. It always exists, so its pid proves nothing: such a row is
///   one an earlier call of this process left, or one another call of it
///   is at work under, and only the heartbeat tells the two apart. So a
///   row this process left after its call ended reads as stale, as it
///   would after a restart;
/// - another process that is gone makes the row stale, whatever the
///   heartbeat says: a heartbeat is only ever beaten by the process that
///   claimed the row, and one that is gone beats no more, though its last
///   beat can still be fresh;
/// - another process that exists makes a remote row live. A local row it
///   is live only while its call works under it: on a fresh heartbeat, or
///   while a call holds the repository's lease. A long-lived process, a
///   desktop app, whose call ended and left the row is not at work under
///   it, and its row reads as stale.
///
/// Any other row, of another host or naming no host or pid, is judged by
/// `row_liveness`: live on a fresh heartbeat and stale otherwise.
pub fn holder_liveness(
    row: &AttributionRow,
    this_host: &str,
    this_pid: u32,
    probe: &dyn ProcessProbe,
    worked: Worked,
) -> Liveness {
    let pid = row
        .pid
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| *pid > 0);
    match pid {
        Some(pid) if row.host.as_deref() == Some(this_host) => {
            let live = if pid == this_pid {
                row.index_heartbeat_fresh
            } else if !probe.exists(pid) {
                false
            } else {
                match worked {
                    Worked::ByProcess => true,
                    Worked::UnderLease { lease_held } => row.index_heartbeat_fresh || lease_held,
                }
            };
            if live {
                Liveness::Live
            } else {
                Liveness::Stale
            }
        }
        Some(_) | None => row_liveness(row, this_host, probe),
    }
}

/// The real probe: the operating system's process table.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemProbe;

impl ProcessProbe for SystemProbe {
    #[cfg(unix)]
    fn exists(&self, pid: u32) -> bool {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return false;
        };
        // SAFETY: signal 0 sends nothing; `kill` only checks that the
        // process exists and may be signalled.
        if unsafe { libc::kill(pid, 0) } == 0 {
            return true;
        }
        // EPERM: the process exists but belongs to someone else.
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    #[cfg(windows)]
    fn exists(&self, pid: u32) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, GetLastError};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        // The exit code a running process reports (`STILL_ACTIVE`).
        const STILL_ACTIVE_CODE: u32 = 259;

        // SAFETY: `OpenProcess` takes plain values and returns a handle or
        // null; a handle it returns is closed below on every path.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            // Access denied: the process exists but is protected from us.
            // Invalid parameter, and anything else: no such process.
            // SAFETY: reads the calling thread's last error.
            return unsafe { GetLastError() } == ERROR_ACCESS_DENIED;
        }
        let mut exit_code = 0_u32;
        // SAFETY: `handle` is open, and `exit_code` lives for the call.
        let read = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
        // SAFETY: `handle` was opened above and is not used again.
        unsafe { CloseHandle(handle) };
        read != 0 && exit_code == STILL_ACTIVE_CODE
    }
}

/// The name of the host this process runs on, as an operation's row
/// records it: the system's host name, else `COMPUTERNAME`, else
/// `unknown`. It reads nothing else and never fails, so recording it never
/// stops an operation.
pub fn this_host() -> String {
    system_host_name()
        .or_else(|| {
            std::env::var("COMPUTERNAME")
                .ok()
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// The system's host name, when it has one that is UTF-8.
#[cfg(unix)]
fn system_host_name() -> Option<String> {
    // Longer than any host name POSIX or Linux allows.
    let mut buffer = [0_u8; 256];
    // SAFETY: the pointer and the length describe `buffer`, which lives for
    // the call; `gethostname` writes no more than the length it is given.
    let written = unsafe { libc::gethostname(buffer.as_mut_ptr().cast(), buffer.len()) };
    if written != 0 {
        return None;
    }
    // A name that filled the buffer may have no terminator: it is taken as
    // it is, so every call on this host reads the same one.
    let end = buffer
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(buffer.len());
    std::str::from_utf8(&buffer[..end])
        .ok()
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// No system call is made for it here: `COMPUTERNAME` names the host.
#[cfg(not(unix))]
fn system_host_name() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct FakeProbe(HashSet<u32>);

    impl ProcessProbe for FakeProbe {
        fn exists(&self, pid: u32) -> bool {
            self.0.contains(&pid)
        }
    }

    #[test]
    fn liveness_is_live_only_for_a_known_host_and_existing_pid() {
        let probe = FakeProbe(HashSet::from([4242]));
        let here = "here";
        let row = |host: Option<&str>, pid: Option<i64>, fresh: bool| AttributionRow {
            host: host.map(str::to_owned),
            pid,
            index_heartbeat_fresh: fresh,
        };
        let cases = [
            (
                "same host, existing pid",
                row(Some(here), Some(4242), false),
                Liveness::Live,
            ),
            (
                "same host, missing pid",
                row(Some(here), Some(4243), false),
                Liveness::Stale,
            ),
            (
                "other host, existing pid",
                row(Some("there"), Some(4242), false),
                Liveness::Stale,
            ),
            (
                "host unknown",
                row(None, Some(4242), false),
                Liveness::Stale,
            ),
            ("pid unknown", row(Some(here), None, false), Liveness::Stale),
            ("pid zero", row(Some(here), Some(0), false), Liveness::Stale),
            (
                "pid negative",
                row(Some(here), Some(-4242), false),
                Liveness::Stale,
            ),
            (
                "pid above u32::MAX",
                row(Some(here), Some(i64::from(u32::MAX) + 1), false),
                Liveness::Stale,
            ),
            (
                "fresh indexing heartbeat, missing pid, other host",
                row(Some("there"), Some(4243), true),
                Liveness::Live,
            ),
            (
                "fresh indexing heartbeat, nothing known",
                row(None, None, true),
                Liveness::Live,
            ),
        ];
        for (name, row, expected) in cases {
            assert_eq!(row_liveness(&row, here, &probe), expected, "{name}");
        }
    }

    #[test]
    fn a_row_of_this_host_is_judged_by_its_process_and_of_this_process_by_its_heartbeat() {
        // The probe finds this process, as the system's always does.
        let probe = FakeProbe(HashSet::from([4242, 7]));
        let here = "here";
        let row = |host: &str, pid: i64, fresh: bool| AttributionRow {
            host: Some(host.to_owned()),
            pid: Some(pid),
            index_heartbeat_fresh: fresh,
        };
        let free = Worked::UnderLease { lease_held: false };
        let held = Worked::UnderLease { lease_held: true };
        let remote = Worked::ByProcess;
        let cases = [
            (
                "this process, no heartbeat",
                row(here, 7, false),
                held,
                Liveness::Stale,
            ),
            (
                "this process, fresh heartbeat",
                row(here, 7, true),
                free,
                Liveness::Live,
            ),
            (
                "this process, remote",
                row(here, 7, false),
                remote,
                Liveness::Stale,
            ),
            (
                "another process, remote",
                row(here, 4242, false),
                remote,
                Liveness::Live,
            ),
            (
                "another process, local, lease free, no heartbeat",
                row(here, 4242, false),
                free,
                Liveness::Stale,
            ),
            (
                "another process, local, lease held",
                row(here, 4242, false),
                held,
                Liveness::Live,
            ),
            (
                "another process, local, fresh heartbeat",
                row(here, 4242, true),
                free,
                Liveness::Live,
            ),
            (
                "a process that is gone",
                row(here, 4243, false),
                held,
                Liveness::Stale,
            ),
            (
                "a process that is gone, its last beat fresh",
                row(here, 4243, true),
                free,
                Liveness::Stale,
            ),
            (
                "this pid on another host",
                row("there", 7, false),
                held,
                Liveness::Stale,
            ),
            (
                "another host, fresh heartbeat",
                row("there", 4243, true),
                free,
                Liveness::Live,
            ),
        ];
        for (name, row, worked, expected) in cases {
            assert_eq!(
                holder_liveness(&row, here, 7, &probe, worked),
                expected,
                "{name}"
            );
        }
    }

    #[test]
    fn the_system_probe_sees_this_process_and_not_a_free_pid() {
        let probe = SystemProbe;
        assert!(probe.exists(std::process::id()), "this process exists");

        // A reaped child's pid is free; the system may hand it out again
        // within milliseconds, so one hit is retried with a new child.
        let mut free_seen = false;
        for _ in 0..3 {
            let mut child = trivial_child();
            let pid = child.id();
            child.wait().expect("child exits");
            if !probe.exists(pid) {
                free_seen = true;
                break;
            }
        }
        assert!(
            free_seen,
            "a reaped child's pid was reported as existing three times"
        );
    }

    #[cfg(unix)]
    fn trivial_child() -> std::process::Child {
        std::process::Command::new("true")
            .spawn()
            .expect("spawn true")
    }

    #[cfg(windows)]
    fn trivial_child() -> std::process::Child {
        std::process::Command::new("cmd")
            .args(["/C", "exit", "0"])
            .spawn()
            .expect("spawn cmd")
    }
}
