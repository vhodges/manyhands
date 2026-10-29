use crate::render::{Human, field};
use manyhands::{
    repository::*,
    results::{Effects, Envelope, Outcome, ResultCode},
};

// Only published, content-free DTOs reach this formatter. Inputs never do.
// Nested field names retain their context, and every dynamic scalar is escaped.
pub(super) fn fields(dto: &impl serde::Serialize) -> String {
    fn walk(prefix: &str, value: &serde_json::Value, out: &mut String) {
        match value {
            serde_json::Value::Object(values) => {
                for (key, value) in values {
                    let key = key.replace('_', " ");
                    let label = if prefix.is_empty() {
                        let mut c = key.chars();
                        c.next()
                            .map(|ch| ch.to_uppercase().collect::<String>() + c.as_str())
                            .unwrap_or_default()
                    } else {
                        format!("{prefix} {key}")
                    };
                    walk(&label, value, out);
                }
            }
            serde_json::Value::Array(values) => {
                if values.is_empty() {
                    out.push_str(&format!("{}: none\n", field(prefix)));
                }
                for value in values {
                    walk(prefix, value, out);
                }
            }
            value => {
                let text = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => "none".into(),
                    _ => value.to_string(),
                };
                out.push_str(&format!("{}: {}\n", field(prefix), field(&text)));
            }
        }
    }
    let mut out = String::new();
    // Public DTOs contain only serializable scalars, arrays and objects.
    if let Ok(mut value) = serde_json::to_value(dto) {
        // Keep human field order stable even when dependencies enable preserve_order.
        value.sort_all_objects();
        walk("", &value, &mut out);
    }
    out
}
pub fn preview(preview: &PreviewDto) -> String {
    let details = match &preview.details {
        PreviewDetailsDto::Repository(d) => fields(d),
        PreviewDetailsDto::RemoteRemoval(d) => fields(d),
        PreviewDetailsDto::RepositoryRemoval(d) => fields(d),
        PreviewDetailsDto::KeyDeletion(d) => fields(d),
        PreviewDetailsDto::OperationAbandon(d) => fields(d),
        PreviewDetailsDto::Identity(d) => fields(d),
        PreviewDetailsDto::Host(d) => fields(d),
        PreviewDetailsDto::Repair(d) => fields(d),
        PreviewDetailsDto::Republish(d) => fields(d),
    };
    let mut out = format!("Preview of {}\n{details}", field(&preview.command));
    for effect in &preview.effects {
        out.push_str(&format!("Will {}.\n", effect_phrase(effect.effect)));
    }
    out.push_str(&format!(
        "Confirmation: {} (expires {})\n",
        field(&preview.confirmation_id),
        field(&preview.expires_at)
    ));
    out
}
pub fn effect_phrase(effect: PreviewEffect) -> &'static str {
    match effect {
        PreviewEffect::DirectoryCreated => "create the directory",
        PreviewEffect::RepositoryInitialized => "initialize the Git repository",
        PreviewEffect::IdentityWritten => "write the repository identity",
        PreviewEffect::HeadMoved => "move HEAD to the primary branch",
        PreviewEffect::WorktreeExclusionWritten => "exclude the managed worktrees",
        PreviewEffect::ConfigurationWritten => "write the Manyhands configuration",
        PreviewEffect::InitializationCommitted => "commit the initialization",
        PreviewEffect::RepositoryRegistered => "register the repository",
        PreviewEffect::RemoteRemoved => "remove the remote",
        PreviewEffect::RemoteTrackingRefsRemoved => "remove remote tracking references",
        PreviewEffect::BranchUpstreamsRemoved => "remove branch upstreams",
        PreviewEffect::RegistrationRemoved => "remove the repository registration",
        PreviewEffect::IndexDropped => "drop the repository index",
        PreviewEffect::OperationRecordsDropped => "drop operation records",
        PreviewEffect::RemoteStateDropped => "drop remote state",
        PreviewEffect::PrivateKeyFileDeleted => "delete the private key file",
        PreviewEffect::PublicKeyFileDeleted => "delete the public key file",
        PreviewEffect::KeyUnregistered => "unregister the key",
        PreviewEffect::OperationClosed => "close the operation",
        PreviewEffect::RequestReleased => "release the request",
        PreviewEffect::HostPinned => "pin the host key",
        PreviewEffect::HostPinReplaced => "replace the host pin",
        PreviewEffect::ContextCreated => "create the editing context",
        PreviewEffect::IdAdopted => "adopt the item ID",
        PreviewEffect::SourceWritten => "write the corrected source",
        PreviewEffect::CheckpointCommitted => "commit the checkpoint",
        PreviewEffect::BranchRepublished => "push the deleted branch again at the local commit",
    }
}
fn effects(effects: &Effects) -> String {
    let entries = [
        ("write", effects.write.as_str()),
        ("checkpoint", effects.checkpoint.as_str()),
        ("discovery", effects.discovery.as_str()),
        ("publication", effects.publication.as_str()),
        ("integration", effects.integration.as_str()),
        ("cleanup", effects.cleanup.as_str()),
    ];
    let mut values: Vec<String> = entries
        .into_iter()
        .filter(|(_, v)| *v != "not_requested")
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if let Some(oid) = &effects.commit_oid {
        values.push(format!("commit={}", field(oid)));
    }
    if values.is_empty() {
        String::new()
    } else {
        format!("effects: {}\n", values.join(" "))
    }
}
fn metadata<T>(envelope: &Envelope<T>) -> String {
    let mut out = String::new();
    if let Some(id) = &envelope.request_id {
        out.push_str(&format!("request: {}\n", field(id)));
    }
    if let Some(id) = &envelope.operation_id {
        out.push_str(&format!("operation: {}\n", field(id)));
    }
    out.push_str(&effects(&envelope.effects));
    out
}
pub fn mutation_success(envelope: &Envelope<MutationDataDto>) -> Human {
    let mut stdout = if envelope.outcome == Outcome::Noop {
        "Already applied; nothing changed.\n".into()
    } else {
        format!("{}\n", field(&envelope.message))
    };
    stdout.push_str(&metadata(envelope));
    if let Some(data) = &envelope.data {
        stdout.push_str(&fields(data));
    }
    Human {
        stdout,
        notes: vec![],
    }
}
pub fn failure_extra(envelope: &Envelope<MutationDataDto>, supplied: bool) -> String {
    let mut out = metadata(envelope);
    if let Some(data) = &envelope.data {
        out.push_str(&fields(data));
    }
    if supplied && envelope.code == ResultCode::TransportUnavailable {
        out.push_str("hint: if the passphrase was wrong, run the command again\n");
    }
    out
}
pub fn observation(dto: &HostObservationDto) -> Human {
    let mut stdout = fields(dto);
    if dto.pinned_algorithm.as_deref() != Some(dto.presented_algorithm.as_str())
        || dto.pinned_fingerprint.as_deref() != Some(dto.presented_fingerprint.as_str())
    {
        let authority = if dto.host.contains(':') {
            format!("[{}]:{}", dto.host, dto.port)
        } else {
            format!("{}:{}", dto.host, dto.port)
        };
        // Reconstruct public next-command fields only from the published DTO.
        let mut input = serde_json::json!({
            "authority": authority,
            "algorithm": dto.presented_algorithm,
            "fingerprint": dto.presented_fingerprint,
        });
        let verb = if let Some(old) = &dto.pinned_fingerprint {
            input["old_fingerprint"] = old.clone().into();
            "replace"
        } else {
            "approve"
        };
        // This compact JSON line is part of the human output's stable layout too.
        input.sort_all_objects();
        stdout.push_str(&format!("Input for host {verb}:\n{input}\n"));
    }
    Human {
        stdout,
        notes: vec![],
    }
}

pub fn request(dto: &RequestDto) -> Human {
    Human {
        stdout: fields(dto),
        notes: vec![],
    }
}
pub fn cancel(dto: &CancelDto) -> Human {
    let mut stdout = fields(dto);
    if dto.state == RequestState::Accepted && !dto.requested {
        stdout.push_str("This local command cannot be stopped after acceptance.\n");
    }
    Human {
        stdout,
        notes: vec![],
    }
}
#[cfg(test)]
#[path = "human_admin_tests.rs"]
mod tests;
