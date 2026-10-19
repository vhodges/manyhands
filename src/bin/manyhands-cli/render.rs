use std::io::{self, Write};

use manyhands::results::{Envelope, FailureClass, ResultCode};
use serde::Serialize;

pub const USAGE_CODE: ResultCode = ResultCode::InvalidInput; // Task 3 changes it.
pub const UNAVAILABLE_CODE: ResultCode = ResultCode::InvalidInput; // Task 3 changes it.

pub struct Human {
    pub stdout: String,
    pub notes: Vec<String>,
}

pub struct Output {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub status: i32,
}

pub fn exit_status(class: Option<FailureClass>) -> i32 {
    match class {
        None => 0,
        Some(FailureClass::Input) => 2,
        Some(FailureClass::Blocked) => 3,
        Some(FailureClass::Incomplete) => 4,
        Some(FailureClass::Transient) => 5,
        Some(FailureClass::Internal) => 1,
        Some(FailureClass::Cancelled) => 130,
    }
}

pub fn envelope_output<T: Serialize>(
    json: bool,
    envelope: &Envelope<T>,
    human: impl FnOnce(&T) -> Human,
) -> Output {
    if json {
        return json_output(envelope);
    }
    if envelope.failure_class().is_some() {
        return human_failure(envelope, None);
    }
    let rendered = envelope.data.as_ref().map(human).unwrap_or(Human {
        stdout: String::new(),
        notes: Vec::new(),
    });
    Output {
        stdout: rendered.stdout.into_bytes(),
        stderr: rendered
            .notes
            .iter()
            .map(|note| format!("note: {note}\n"))
            .collect::<String>()
            .into_bytes(),
        status: exit_status(envelope.failure_class()),
    }
}

pub fn failure_output(
    json: bool,
    envelope: &Envelope<serde_json::Value>,
    extra: Option<String>,
) -> Output {
    if json {
        json_output(envelope)
    } else {
        human_failure(envelope, extra.as_deref())
    }
}

fn json_output<T: Serialize>(envelope: &Envelope<T>) -> Output {
    match serde_json::to_string(envelope) {
        Ok(text) => Output {
            stdout: format!("{text}\n").into_bytes(),
            stderr: Vec::new(),
            status: exit_status(envelope.failure_class()),
        },
        Err(_) => {
            // Serialize only the fixed public failure, never the backend error.
            let failure = Envelope::<serde_json::Value>::failure(
                &envelope.command,
                envelope.scope.clone(),
                ResultCode::InternalError,
                Vec::new(),
            );
            json_output(&failure)
        }
    }
}

fn human_failure<T>(envelope: &Envelope<T>, extra: Option<&str>) -> Output {
    let mut stderr = format!(
        "error: {}\ncode: {}\n",
        envelope.message,
        envelope.code.as_str()
    );
    if let Some(extra) = extra {
        stderr.push_str(extra.trim_end_matches('\n'));
        stderr.push('\n');
    }
    for recovery in &envelope.recovery {
        stderr.push_str("next: ");
        stderr.push_str(recovery.action.as_str());
        for (key, value) in &recovery.arguments {
            stderr.push_str(&format!(" {key}="));
            match value.as_str() {
                Some(text) => stderr.push_str(text),
                None => stderr.push_str(&value.to_string()),
            }
        }
        stderr.push('\n');
    }
    Output {
        stdout: Vec::new(),
        stderr: stderr.into_bytes(),
        status: exit_status(envelope.failure_class()),
    }
}

fn write_output(
    output: &Output,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<()> {
    stdout.write_all(&output.stdout)?;
    stdout.flush()?;
    stderr.write_all(&output.stderr)?;
    stderr.flush()
}

pub fn write_and_exit(output: Output) -> ! {
    let status = match write_output(&output, &mut io::stdout().lock(), &mut io::stderr().lock()) {
        Ok(()) => output.status,
        Err(_) => 1,
    };
    std::process::exit(status)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
