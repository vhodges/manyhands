//! Whose an operation's row is, as far as this machine can tell: the host a
//! row records is this one's name, read here.

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
