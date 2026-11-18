//! Failure reporting for the desktop executable, independent of GPUI startup.

use std::ffi::CStr;

type Dialog = fn(&'static CStr, &'static CStr);

const TITLE: &CStr = c"Manyhands";
const STARTUP_MESSAGE: &CStr = c"Manyhands could not start because Git transport initialization failed. Please restart the application.";
const PANIC_MESSAGE: &CStr =
    c"Manyhands encountered an unexpected error. Please close and restart the application.";

#[derive(Clone, Copy)]
struct Reporter {
    dialog: Option<Dialog>,
}

const REPORTER: Reporter = Reporter {
    #[cfg(all(windows, not(debug_assertions)))]
    dialog: Some(native_dialog),
    #[cfg(not(all(windows, not(debug_assertions))))]
    dialog: None,
};

pub(crate) fn report_startup_failure() {
    REPORTER.report_startup_failure();
}

pub(crate) fn install_panic_hook() {
    REPORTER.install_panic_hook();
}

impl Reporter {
    fn report_startup_failure(self) {
        eprintln!("Git transport initialization failed");
        if let Some(dialog) = self.dialog {
            dialog(TITLE, STARTUP_MESSAGE);
        }
    }

    fn install_panic_hook(self) {
        let Some(dialog) = self.dialog else {
            return;
        };
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            // Preserve diagnostics and unwinding/abort behavior. The native
            // dialog never receives the panic payload or its source location.
            previous(info);
            dialog(TITLE, PANIC_MESSAGE);
        }));
    }
}

#[cfg(all(windows, not(debug_assertions)))]
fn native_dialog(title: &'static CStr, message: &'static CStr) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_TASKMODAL, MessageBoxA,
    };

    // These fixed ASCII C strings need no allocation or encoding conversion,
    // including when called from the panic hook. Do not add payloads or logs.
    // SAFETY: both strings are static and NUL-terminated. A null owner allows
    // reporting before GPUI has created a window, including startup panics.
    // Failure to display the dialog must not cause another panic or alter exit.
    unsafe {
        MessageBoxA(
            std::ptr::null_mut(),
            message.as_ptr().cast(),
            title.as_ptr().cast(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND | MB_TASKMODAL,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Output};

    const CHILD_SCENARIO: &str = "MANYHANDS_STARTUP_REPORTING_TEST";
    const SECRET: &str = "test-secret-token C:\\private\\repository";
    const STARTUP_MESSAGE: &str = "Manyhands could not start because Git transport initialization failed. Please restart the application.";
    const PANIC_MESSAGE: &str =
        "Manyhands encountered an unexpected error. Please close and restart the application.";

    fn child(scenario: &str) -> Output {
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "startup_reporting::tests::reporting_child",
                "--nocapture",
            ])
            .env(CHILD_SCENARIO, scenario)
            .output()
            .unwrap()
    }

    fn record_dialog(title: &'static CStr, message: &'static CStr) {
        println!(
            "DIALOG_TITLE={}\nDIALOG_MESSAGE={}",
            title.to_str().unwrap(),
            message.to_str().unwrap()
        );
    }

    fn assert_dialog(output: &Output, message: &str) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(&format!(
                "DIALOG_TITLE=Manyhands\nDIALOG_MESSAGE={message}\n"
            )),
            "missing fixed-text dialog: {stdout}"
        );
        assert!(!stdout.contains(SECRET));
        assert!(!stdout.contains("private"));
    }

    #[test]
    fn startup_failure_reports_fixed_dialog_and_preserves_stderr() {
        let output = child("startup-dialog");
        assert!(output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Git transport initialization failed")
        );
        assert_dialog(&output, STARTUP_MESSAGE);
    }

    #[test]
    fn panic_reports_fixed_dialog_chains_previous_hook_and_still_unwinds() {
        let output = child("panic-dialog");
        assert_eq!(output.status.code(), Some(101));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("PREVIOUS_HOOK"));
        assert!(
            stderr.contains(SECRET),
            "original panic hook must still run"
        );
        assert_dialog(&output, PANIC_MESSAGE);
    }

    #[test]
    fn console_reporting_preserves_stderr_and_panic_without_dialogs() {
        let output = child("console");
        assert_eq!(output.status.code(), Some(101));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Git transport initialization failed"));
        assert!(stderr.contains("PREVIOUS_HOOK"));
        assert!(stderr.contains(SECRET));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("DIALOG_"));
    }

    #[cfg(not(all(windows, not(debug_assertions))))]
    #[test]
    fn selected_non_windows_or_debug_reporter_does_not_add_dialogs() {
        let output = child("selected-console");
        assert_eq!(output.status.code(), Some(101));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Git transport initialization failed"));
        assert!(stderr.contains("PREVIOUS_HOOK"));
        assert!(stderr.contains(SECRET));
        assert!(!String::from_utf8_lossy(&output.stdout).contains("DIALOG_"));
    }

    // Each scenario runs in a subprocess so replacing the process-wide panic
    // hook cannot affect other desktop tests or open a real native dialog.
    #[test]
    fn reporting_child() {
        let Ok(scenario) = std::env::var(CHILD_SCENARIO) else {
            return;
        };
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            eprintln!("PREVIOUS_HOOK");
            previous(info);
        }));

        match scenario.as_str() {
            "startup-dialog" => Reporter {
                dialog: Some(record_dialog),
            }
            .report_startup_failure(),
            "panic-dialog" => {
                Reporter {
                    dialog: Some(record_dialog),
                }
                .install_panic_hook();
                panic!("{SECRET}");
            }
            "console" => {
                let reporter = Reporter { dialog: None };
                reporter.install_panic_hook();
                reporter.report_startup_failure();
                panic!("{SECRET}");
            }
            "selected-console" => {
                install_panic_hook();
                report_startup_failure();
                panic!("{SECRET}");
            }
            _ => panic!("unknown reporting test scenario"),
        }
    }
}
