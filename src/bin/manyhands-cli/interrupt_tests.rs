//! `sigaction` and the run's latch are process-wide. Every case that raises
//! a signal, sets the latch or changes a disposition runs the test binary
//! again as a child, filtered to that one case, and asserts on the child's
//! exit. No in-process test sees a set latch (R-36).
#![cfg(unix)]

use super::*;
use std::{
    io::Read,
    os::unix::process::ExitStatusExt,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

/// The variable that tells a child which case it runs.
const CASE: &str = "MANYHANDS_INTERRUPT_CASE";
/// Written by a child once it is inside its case, so that a child that
/// died is known to have died there.
const INSIDE: &str = "interrupt case started";
/// How long a child may run before it is killed. Generous, for a loaded
/// machine: a child that hangs fails its case instead of the binary.
const CHILD_DEADLINE: Duration = Duration::from_secs(60);

/// `None` in the child that runs the case `name`: run its body. In the
/// parent, the child's output once it has exited.
fn child(name: &str) -> Option<Output> {
    if std::env::var(CASE).as_deref() == Ok(name) {
        eprintln!("{INSIDE}");
        return None;
    }
    let module = module_path!()
        .split_once("::")
        .map_or(module_path!(), |(_, path)| path);
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            &format!("{module}::{name}"),
            "--exact",
            "--test-threads=1",
            "--nocapture",
        ])
        .env(CASE, name)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Drain both pipes while the child runs, so that it never blocks on them.
    let drain = |mut pipe: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            bytes
        })
    };
    let stdout = drain(Box::new(child.stdout.take().unwrap()));
    let stderr = drain(Box::new(child.stderr.take().unwrap()));
    let deadline = Instant::now() + CHILD_DEADLINE;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the child running {name} exceeded {CHILD_DEADLINE:?} and was killed");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Some(Output {
        status,
        stdout: stdout.join().unwrap(),
        stderr: stderr.join().unwrap(),
    })
}

/// The child ran its case to the end, and the case passed.
fn passed(output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stderr.contains(INSIDE), "{stdout}\n{stderr}");
    assert!(stdout.contains("1 passed"), "{stdout}\n{stderr}");
}

/// What `SIGINT` is set to now: the handler and the flags a caller sets.
/// The C library may add flags of its own, such as glibc's restorer flag
/// on every action it installs, so only these are compared.
fn disposition() -> (libc::sighandler_t, libc::c_int) {
    const FLAGS: libc::c_int = libc::SA_RESTART
        | libc::SA_SIGINFO
        | libc::SA_NODEFER
        | libc::SA_RESETHAND
        | libc::SA_ONSTACK;
    // SAFETY: a null new action only reads the current one into valid storage.
    unsafe {
        let mut current: libc::sigaction = std::mem::zeroed();
        assert_eq!(
            libc::sigaction(libc::SIGINT, std::ptr::null(), &mut current),
            0
        );
        (current.sa_sigaction, current.sa_flags & FLAGS)
    }
}

fn set(handler: libc::sighandler_t) {
    // SAFETY: SIG_DFL and SIG_IGN are valid dispositions for SIGINT.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = handler;
        libc::sigemptyset(&mut action.sa_mask);
        assert_eq!(
            libc::sigaction(libc::SIGINT, &action, std::ptr::null_mut()),
            0
        );
    }
}

fn press() {
    // SAFETY: raise only sends SIGINT to this thread.
    assert_eq!(unsafe { libc::raise(libc::SIGINT) }, 0);
}

#[test]
fn the_window_restores_a_default_and_keeps_an_inherited_ignore() {
    let Some(output) = child("the_window_restores_a_default_and_keeps_an_inherited_ignore") else {
        // From the default: the handler, with SA_RESTART, then the default.
        set(libc::SIG_DFL);
        let before = disposition();
        assert_eq!(before.0, libc::SIG_DFL);
        during(|token| {
            let (handler, flags) = disposition();
            assert!(handler != libc::SIG_DFL && handler != libc::SIG_IGN);
            assert_ne!(flags & libc::SA_RESTART, 0, "SA_RESTART");
            assert!(!token.is_cancelled());
        });
        assert_eq!(disposition(), before);
        // From an inherited ignore: nothing is installed, so a press neither
        // cancels the token nor ends the process, and the ignore stays.
        reset();
        set(libc::SIG_IGN);
        let before = disposition();
        assert_eq!(before.0, libc::SIG_IGN);
        during(|token| {
            assert_eq!(disposition(), before);
            press();
            assert!(!token.is_cancelled());
        });
        assert!(!latched());
        assert_eq!(disposition(), before);
        return;
    };
    passed(&output);
}

#[test]
fn a_press_cancels_the_token_and_the_process_goes_on() {
    let Some(output) = child("a_press_cancels_the_token_and_the_process_goes_on") else {
        reset();
        let before = disposition();
        let answer = during(|token| {
            assert!(!token.is_cancelled());
            press();
            assert!(token.is_cancelled());
            7
        });
        assert_eq!(answer, 7);
        assert_eq!(disposition(), before);
        return;
    };
    passed(&output);
}

#[test]
fn the_run_s_latch_is_set_by_a_press_and_stays_set_in_a_later_window() {
    let Some(output) = child("the_run_s_latch_is_set_by_a_press_and_stays_set_in_a_later_window")
    else {
        reset();
        assert!(!latched());
        during(|_| press());
        assert!(latched());
        // A second window of the same run starts cancelled: the press
        // count and the cancellation persist across C2's retry.
        during(|token| {
            assert!(token.is_cancelled());
            assert!(latched());
        });
        assert!(latched());
        // A new run starts with no press.
        reset();
        assert!(!latched());
        during(|token| assert!(!token.is_cancelled()));
        return;
    };
    passed(&output);
}

#[test]
fn a_second_press_in_one_window_ends_the_process_by_sigint() {
    let Some(output) = child("a_second_press_in_one_window_ends_the_process_by_sigint") else {
        reset();
        during(|token| {
            press();
            assert!(token.is_cancelled());
            eprintln!("{SURVIVED}");
            press();
        });
        unreachable!("the second press ends the process");
    };
    died_after_one_press(&output);
}

/// Written by a child after its first press, which it survives.
const SURVIVED: &str = "first press survived";

/// The child survived its first press and then died by `SIGINT`.
fn died_after_one_press(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(INSIDE), "{stderr}");
    assert!(stderr.contains(SURVIVED), "{stderr}");
    assert_eq!(output.status.signal(), Some(libc::SIGINT), "{stderr}");
}

#[test]
fn a_press_outside_a_window_after_one_inside_it_ends_the_process() {
    let Some(output) = child("a_press_outside_a_window_after_one_inside_it_ends_the_process")
    else {
        reset();
        during(|_| press());
        assert!(latched());
        eprintln!("{SURVIVED}");
        // Between execute calls no handler is installed (R-21).
        press();
        unreachable!("the default disposition ends the process");
    };
    died_after_one_press(&output);
}

#[test]
fn an_unwind_out_of_the_window_restores_the_disposition() {
    let Some(output) = child("an_unwind_out_of_the_window_restores_the_disposition") else {
        let before = disposition();
        let unwound = std::panic::catch_unwind(|| during(|_| panic!("unwind")));
        assert!(unwound.is_err());
        assert_eq!(disposition(), before);
        return;
    };
    passed(&output);
}

/// A boundary whose every `execute` answers a partial result that offers
/// the same request again, which C2's bounded retry re-runs (fact 66).
/// With `press`, each call is pressed inside its window after its last
/// safe point.
struct Retried {
    press: bool,
    calls: std::cell::Cell<usize>,
}

impl crate::mutate::Boundary for Retried {
    fn prepare(
        &self,
        _: &manyhands::repository::Mutation,
    ) -> manyhands::results::Envelope<manyhands::repository::PrepareDataDto> {
        unreachable!("an unconfirmed execute prepares nothing")
    }

    fn execute(
        &self,
        call: manyhands::repository::MutationCall,
        _: &mut manyhands::repository::keys::SessionCredentials<
            crate::prompt::TerminalProvider<'_>,
        >,
        _: &mut dyn manyhands::repository::ProgressSink,
    ) -> manyhands::results::Envelope<manyhands::repository::MutationDataDto> {
        use manyhands::results::{
            Effects, Envelope, Outcome, RecoveryAction, RecoveryActionKind, ResultCode, Scope,
        };
        self.calls.set(self.calls.get() + 1);
        if self.press {
            during(|_| press());
        }
        let request = call.request_id.to_string();
        Envelope {
            schema_version: 1,
            command: "repo sync".into(),
            request_id: Some(request.clone()),
            operation_id: None,
            outcome: Outcome::Partial,
            code: ResultCode::PushRejected,
            message: ResultCode::PushRejected.message().into(),
            scope: Scope::default(),
            effects: Effects::not_requested(),
            data: None,
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::RequestRetry,
                [("request_id", request.into())],
            )],
        }
    }

    fn request_exists(
        &self,
        _: manyhands::repository::RequestId,
    ) -> Result<bool, manyhands::results::Envelope<manyhands::repository::RequestDto>> {
        unreachable!("an unconfirmed execute looks for no record")
    }
}

/// Runs `boundary` through `mutate::run` once, and gives the number of
/// sleeps C2's bounded retry took.
fn drive(boundary: &Retried) -> usize {
    let spec = crate::catalog::find("repo", "sync").unwrap();
    let mut sleeps = 0;
    let output = crate::mutate::run(
        boundary,
        &mut crate::mutate::Session {
            json: true,
            interactive: false,
            terminal: None,
        },
        spec,
        crate::mutate::MutationFlags {
            request_id: Some(manyhands::repository::RequestId::new()),
            allocated: false,
            prepare: false,
            confirm: None,
        },
        crate::mutate::Plan {
            identity_prompt: false,
            retry: true,
        },
        &mut |_| {
            manyhands::repository::Mutation::RepoSync(manyhands::repository::RepoSyncInput {
                root: "/r".into(),
            })
        },
        &mut |_| sleeps += 1,
    );
    assert_eq!(output.status, 4);
    sleeps
}

#[test]
fn a_latched_press_stops_c2_s_retry_and_the_next_run_starts_unlatched() {
    let Some(output) = child("a_latched_press_stops_c2_s_retry_and_the_next_run_starts_unlatched")
    else {
        let pressed = Retried {
            press: true,
            calls: 0.into(),
        };
        assert_eq!(drive(&pressed), 0);
        assert_eq!(pressed.calls.get(), 1, "no retry follows a press (R-36)");
        assert!(latched());
        // A new run resets the latch on entry, and its answer is retried.
        let unpressed = Retried {
            press: false,
            calls: 0.into(),
        };
        assert_eq!(drive(&unpressed), crate::mutate::RETRY_DELAYS_MS.len());
        assert_eq!(
            unpressed.calls.get(),
            1 + crate::mutate::RETRY_DELAYS_MS.len()
        );
        assert!(!latched());
        return;
    };
    passed(&output);
}
