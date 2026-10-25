use std::io::{self, Write};

use crate::human_admin;
use manyhands::results::{Envelope, FailureClass, ResultCode};
use manyhands::{
    repository::{MutationDataDto, PrepareDataDto},
    results::Outcome,
};
use serde::Serialize;

pub fn mutation_output(
    json: bool,
    envelope: &Envelope<MutationDataDto>,
    passphrase_supplied: bool,
) -> Output {
    if json {
        return json_output(envelope);
    }
    if envelope.failure_class().is_some() {
        return human_failure(
            envelope,
            Some(&human_admin::failure_extra(envelope, passphrase_supplied)),
        );
    }
    let human = human_admin::mutation_success(envelope);
    Output {
        stdout: human.stdout.into_bytes(),
        stderr: Vec::new(),
        status: exit_status(envelope.failure_class()),
    }
}
pub fn prepare_output(json: bool, envelope: &Envelope<PrepareDataDto>) -> Output {
    if json {
        return json_output(envelope);
    }
    if envelope.failure_class().is_some() {
        let extra = envelope.data.as_ref().map(human_admin::fields);
        return human_failure(envelope, extra.as_deref());
    }
    let stdout = match envelope.data.as_ref() {
        Some(PrepareDataDto::Preview(preview)) => format!(
            "{}Apply with the same command and --confirm {}; add --request-id unless you run it at a terminal.\n",
            human_admin::preview(preview),
            field(&preview.confirmation_id)
        ),
        _ if envelope.outcome == Outcome::Noop => "Already applied; nothing changed.\n".into(),
        _ => format!("{}\n", field(&envelope.message)),
    };
    Output {
        stdout: stdout.into_bytes(),
        stderr: Vec::new(),
        status: exit_status(envelope.failure_class()),
    }
}

pub fn observation_output(
    json: bool,
    envelope: &Envelope<manyhands::repository::HostObservationDto>,
    supplied: bool,
) -> Output {
    if !json && envelope.failure_class().is_some() {
        return human_failure(
            envelope,
            (supplied && envelope.code == ResultCode::TransportUnavailable)
                .then_some("hint: if the passphrase was wrong, run the command again"),
        );
    }
    envelope_output(json, envelope, human_admin::observation)
}

pub const USAGE_CODE: ResultCode = ResultCode::UsageError;
pub const UNAVAILABLE_CODE: ResultCode = ResultCode::CommandUnavailable;

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

/// Escape only field-owned controls, preserving ordinary Unicode and paths.
/// Never apply this to layout, canonical public-key text or requested bodies/sources.
pub(super) fn field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
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
                Some(text) => stderr.push_str(&field(text)),
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
