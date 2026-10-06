//! A real main must bootstrap before this runner creates any threads.
#![allow(dead_code)]
#[path = "ssh_signals.rs"]
pub(crate) mod signals;
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
    run_mode(cases, &[], false);
}

/// Controls are child-only negative probes, never ordinary passing cases.
pub fn run_with_output_privacy(cases: &[Case], controls: &[Case]) {
    run_mode(cases, controls, true);
}

fn run_mode(cases: &[Case], controls: &[Case], output_privacy: bool) {
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
            .chain(controls)
            .find(|(name, _)| Some(*name) == args.get(1).map(String::as_str))
        else {
            std::process::exit(2)
        };
        let ok = std::panic::catch_unwind(|| signals::run_case(*case))
            .is_ok_and(|result| result.is_ok());
        if !ok {
            eprintln!("SSH fixture case failed");
            std::process::exit(1);
        }
        return;
    }
    let options = match parse_options(&args) {
        Ok(options) => options,
        Err(_) => {
            eprintln!("unsupported SSH runner options");
            std::process::exit(2);
        }
    };
    let filter = options.filter.as_ref();
    let mut count = 0;
    for (name, _) in cases {
        if filter.is_some_and(|filter| {
            !(if options.exact {
                *name == filter
            } else {
                name.contains(filter)
            })
        }) {
            continue;
        }
        if args.iter().any(|arg| arg == "--list") {
            println!("{name}: test");
            continue;
        }
        let result = if output_privacy {
            run_isolated_with_output_privacy(name).map_err(|_| FixtureError)
        } else {
            run_isolated(name)
        };
        if result.is_err() {
            eprintln!("test {name} ... FAILED (isolated child or watchdog failure)");
            std::process::exit(1);
        }
        println!("test {name} ... ok");
        count += 1;
    }
    println!("{count} SSH cases passed");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsolationFailure {
    Fixture,
    Child,
    ProbeInventory,
    OutputPrivacy,
}
impl From<FixtureError> for IsolationFailure {
    fn from(_: FixtureError) -> Self {
        Self::Fixture
    }
}
pub fn run_isolated(case: &str) -> Result<(), FixtureError> {
    capture_isolated(case, false).map_err(|_| FixtureError)
}
pub fn run_isolated_with_output_privacy(case: &str) -> Result<(), IsolationFailure> {
    capture_isolated(case, true)
}
fn capture_isolated(case: &str, output_privacy: bool) -> Result<(), IsolationFailure> {
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
    let probes = isolation.path().join("private probes");
    fixed(std::fs::create_dir(&probes))?;
    let mut command = Command::new(fixed(std::env::current_exe())?);
    command.env(super::ssh_privacy::PROBES, &probes);
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
    // Drain all raw output into bounded buffers; overflow is a failure. Wait for
    // child completion before loading its probes, then scan before filtering.
    // Neither raw diagnostics nor probes are ever copied into parent failures.
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
    let output = out.join();
    let errors = err.join();
    let output = fixed(output)??;
    let errors = fixed(errors)??;
    if output_privacy || case == "transport_privacy" {
        let probes =
            super::ssh_privacy::load(&probes).map_err(|_| IsolationFailure::ProbeInventory)?;
        let output_scan = super::ssh_privacy::clean(&output, &probes);
        let error_scan = super::ssh_privacy::clean(&errors, &probes);
        output_scan.map_err(|_| IsolationFailure::OutputPrivacy)?;
        error_scan.map_err(|_| IsolationFailure::OutputPrivacy)?;
    }
    {
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
    result.map_err(|_| IsolationFailure::Child)
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
const CAPTURE_LIMIT: usize = 8 * 1024 * 1024;
fn drain(mut stream: impl Read) -> Result<Vec<u8>, FixtureError> {
    let mut buffer = [0; 4096];
    let mut retained = Vec::new();
    let mut overflow = false;
    loop {
        let count = fixed(stream.read(&mut buffer))?;
        if count == 0 {
            break;
        }
        if retained.len() + count <= CAPTURE_LIMIT && !overflow {
            retained.extend_from_slice(&buffer[..count]);
        } else {
            overflow = true;
        }
    }
    if overflow {
        Err(FixtureError)
    } else {
        Ok(retained)
    }
}
struct Options {
    filter: Option<String>,
    exact: bool,
}
fn parse_options(args: &[String]) -> Result<Options, FixtureError> {
    let mut options = Options {
        filter: None,
        exact: false,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--test-threads" => {
                let value = args.next().ok_or(FixtureError)?;
                if value.parse::<usize>().ok().filter(|n| *n > 0).is_none() {
                    return Err(FixtureError);
                }
            }
            "--exact" => options.exact = true,
            "--list" | "--nocapture" | "--show-output" | "--quiet" => {}
            _ if arg.starts_with("--test-threads=") => {
                if arg[15..].parse::<usize>().ok().filter(|n| *n > 0).is_none() {
                    return Err(FixtureError);
                }
            }
            _ if !arg.starts_with('-') && options.filter.is_none() => {
                options.filter = Some(arg.clone())
            }
            _ => return Err(FixtureError),
        }
    }
    Ok(options)
}
pub fn runner_options_regression() -> Result<(), FixtureError> {
    let output = fixed(
        Command::new(fixed(std::env::current_exe())?)
            .args(["--list", "--test-threads", "1"])
            .output(),
    )?;
    assert!(
        output.status.success(),
        "supported runner options must succeed"
    );
    assert!(
        output.stdout.windows(6).any(|w| w == b": test"),
        "runner options must not become a filter"
    );
    assert!(parse_options(&["--test-threads".into(), "0".into()]).is_err());
    assert!(parse_options(&["--unsupported".into()]).is_err());
    Ok(())
}
pub fn raw_output_regression() -> Result<(), FixtureError> {
    let mut bytes = vec![b'x'; 20_478];
    bytes.extend_from_slice(b"private-stream-probe");
    let captured = drain(&bytes[..])?;
    let probes = vec![b"private-stream-probe".to_vec()];
    assert!(
        super::ssh_privacy::clean(&captured, &probes).is_err(),
        "raw output scanner must detect beyond-prefix and read-boundary leaks"
    );
    assert!(
        super::ssh_privacy::clean(b"clean", &[]).is_err(),
        "missing probes must fail closed"
    );
    assert!(
        super::ssh_privacy::clean(b"clean", &[vec![]]).is_err(),
        "empty probes must fail closed"
    );
    super::ssh_privacy::clean(b"clean", &probes)?;
    Ok(())
}

pub fn capture_limit_regression() -> Result<(), FixtureError> {
    assert!(
        drain(std::io::repeat(b'x').take((CAPTURE_LIMIT + 1) as u64)).is_err(),
        "capture overflow must fail closed"
    );
    Ok(())
}
