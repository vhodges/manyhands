//! A real main must bootstrap before this runner creates any threads.
#![allow(dead_code)]
use super::ssh_remote::{FixtureError, fixed};
use std::{
    io::Read,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Call only from the real main before creating threads, including test workers.
///
/// # Safety
/// The caller owns the same pre-thread contract as the production initializer.
pub unsafe fn initialize() -> Result<(), FixtureError> {
    // SAFETY: delegated by this function's caller, before any thread exists.
    unsafe {
        fixed(crate::runtime::initialize_git_transport_before_threads())?;
        assert!(crate::runtime::git_transport_initialized());
        assert_eq!(
            fixed(git2::opts::get_server_connect_timeout_in_milliseconds())?,
            10_000
        );
        assert_eq!(
            fixed(git2::opts::get_server_timeout_in_milliseconds())?,
            30_000
        );
        fixed(crate::runtime::initialize_git_transport_before_threads())?;
        if let Some(home) = std::env::var_os("MANYHANDS_SSH_TEST_HOME") {
            for level in [
                git2::ConfigLevel::System,
                git2::ConfigLevel::Global,
                git2::ConfigLevel::XDG,
                git2::ConfigLevel::ProgramData,
            ] {
                fixed(git2::opts::set_search_path(
                    level,
                    std::path::Path::new(&home),
                ))?;
            }
        }
    }
    Ok(())
}

pub type Case = (&'static str, fn() -> Result<(), FixtureError>);

pub fn run(cases: &[Case]) {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("SSH fixture assertion failed");
        if let Some(location) = info.location() {
            observation(&[location.line() as u128]);
        }
    }));
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--ssh-case") {
        let Some((_, case)) = cases
            .iter()
            .find(|(name, _)| Some(*name) == args.get(1).map(String::as_str))
        else {
            std::process::exit(2)
        };
        let ok = std::panic::catch_unwind(*case).is_ok_and(|result| result.is_ok());
        if !ok {
            eprintln!("SSH fixture case failed");
            std::process::exit(1);
        }
        return;
    }
    let filter = args.iter().find(|arg| !arg.starts_with('-'));
    let mut count = 0;
    for (name, _) in cases {
        if filter.is_some_and(|filter| !name.contains(filter)) {
            continue;
        }
        if args.iter().any(|arg| arg == "--list") {
            println!("{name}: test");
            continue;
        }
        if run_isolated(name).is_err() {
            eprintln!("test {name} ... FAILED (isolated child or watchdog failure)");
            std::process::exit(1);
        }
        println!("test {name} ... ok");
        count += 1;
    }
    println!("{count} SSH cases passed");
}

pub fn run_isolated(case: &str) -> Result<(), FixtureError> {
    let isolation = fixed(
        tempfile::Builder::new()
            .prefix("manyhands isolated SSH ")
            .tempdir(),
    )?;
    let home = isolation.path().join("home");
    let data = isolation.path().join("application data");
    let temporary = isolation.path().join("fixtures");
    for directory in [&home, &data, &temporary] {
        fixed(std::fs::create_dir(directory))?;
    }
    fixed(std::fs::write(
        home.join(".gitconfig"),
        "[credential]\n\thelper = !exit 97\n",
    ))?;
    fixed(std::fs::create_dir(home.join(".ssh")))?;
    fixed(std::fs::write(
        home.join(".ssh/config"),
        "Host *\n  ProxyCommand exit 98\n",
    ))?;
    fixed(std::fs::write(home.join(".ssh/known_hosts"), b""))?;
    let default_key = super::ssh_remote::generate_key()?;
    fixed(std::fs::write(
        home.join(".ssh/id_ed25519"),
        fixed(default_key.to_openssh(russh::keys::ssh_key::LineEnding::LF))?.as_bytes(),
    ))?;
    let mut command = Command::new(fixed(std::env::current_exe())?);
    command
        .args(["--ssh-case", case])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if name.starts_with("GIT_")
            || name.starts_with("SSH_")
            || name.starts_with("GCM_")
            || name == "MANYHANDS_SSH_TEST_HOME"
        {
            command.env_remove(key);
        }
    }
    for key in [
        "HOME",
        "USERPROFILE",
        "XDG_CONFIG_HOME",
        "MANYHANDS_SSH_TEST_HOME",
    ] {
        command.env(key, &home);
    }
    for key in ["XDG_DATA_HOME", "XDG_STATE_HOME", "APPDATA", "LOCALAPPDATA"] {
        command.env(key, &data);
    }
    // Parent ownership also removes fixture keys/repositories if the watchdog
    // must terminate a child before its TempDir destructors can run.
    for key in ["TMPDIR", "TMP", "TEMP"] {
        command.env(key, &temporary);
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", home.join(".gitconfig"))
        .env("SSH_AUTH_SOCK", home.join("unavailable-agent"))
        .env("GIT_TERMINAL_PROMPT", "0");
    let mut child = fixed(command.spawn())?;
    let stdout = child.stdout.take().ok_or(FixtureError)?;
    let stderr = child.stderr.take().ok_or(FixtureError)?;
    // Drain continuously with bounded buffers. Child diagnostics may contain
    // backend strings or keys, so they are never copied into parent failures.
    let out = std::thread::spawn(move || drain(stdout));
    let err = std::thread::spawn(move || drain(stderr));
    let deadline = Instant::now() + Duration::from_secs(100);
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(FixtureError)
                };
            }
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break Err(FixtureError);
            }
        }
    };
    if let Ok(output) = out.join() {
        for line in String::from_utf8_lossy(&output).lines() {
            if let Some(numbers) = line.strip_prefix("SSH_OBSERVATION ")
                && numbers
                    .split_whitespace()
                    .all(|part| part.parse::<u128>().is_ok())
            {
                println!("{case} observation: {numbers}");
            }
        }
    }
    let _ = err.join();
    result
}
pub fn observation(values: &[u128]) {
    println!(
        "SSH_OBSERVATION {}",
        values
            .iter()
            .map(u128::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    );
}
fn drain(mut stream: impl Read) -> Vec<u8> {
    let mut buffer = [0; 4096];
    let mut retained = Vec::new();
    while let Ok(count) = stream.read(&mut buffer) {
        if count == 0 {
            break;
        }
        let room = 16_384usize.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..count.min(room)]);
    }
    retained
}
