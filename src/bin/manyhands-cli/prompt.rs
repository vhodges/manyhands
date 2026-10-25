use std::cell::Cell;

use manyhands::repository::{
    CommitIdentity,
    keys::{
        PassphraseResponse, SecretPassphrase, SessionCredentialProvider, UnlockReason,
        UnlockRequest,
    },
};

use crate::{render, terminal::Terminal};

pub enum Consent {
    Yes,
    No,
}

pub enum Asked<T> {
    Answer(T),
    Cancelled,
    Exhausted,
}

pub fn consent(terminal: &mut dyn Terminal, preview_text: &str) -> Consent {
    if terminal.write(preview_text).is_err() {
        return Consent::No;
    }
    let Ok(Some(line)) = terminal.ask("Proceed? [y/N] ", true) else {
        return Consent::No;
    };
    if line.trim().eq_ignore_ascii_case("y") || line.trim().eq_ignore_ascii_case("yes") {
        Consent::Yes
    } else {
        Consent::No
    }
}

pub fn identity(terminal: &mut dyn Terminal) -> Asked<CommitIdentity> {
    for _ in 0..3 {
        let Ok(Some(mut name)) = terminal.ask("Name for commits in this repository: ", true) else {
            return Asked::Cancelled;
        };
        let Ok(Some(mut email)) = terminal.ask("Email: ", true) else {
            return Asked::Cancelled;
        };
        if !name.trim().is_empty() && !email.trim().is_empty() {
            // Preserve the supplied values; the library owns content validation.
            return Asked::Answer(CommitIdentity {
                name: std::mem::take(&mut *name),
                email: std::mem::take(&mut *email),
            });
        }
        if terminal
            .write("Name and email must not be empty.\n")
            .is_err()
        {
            return Asked::Cancelled;
        }
    }
    Asked::Exhausted
}

pub fn new_passphrase(terminal: &mut dyn Terminal, label: &str) -> Asked<SecretPassphrase> {
    let prompt = format!("New passphrase for key \"{}\": ", render::field(label));
    for _ in 0..3 {
        let Ok(Some(mut line)) = terminal.ask(&prompt, false) else {
            return Asked::Cancelled;
        };
        let Ok(Some(repeated)) = terminal.ask("Repeat it: ", false) else {
            return Asked::Cancelled;
        };
        let explanation = if line.is_empty() || repeated.is_empty() {
            "Passphrase must not be empty.\n"
        } else if line != repeated {
            "Passphrases do not match.\n"
        } else {
            match SecretPassphrase::new(std::mem::take(&mut *line)) {
                Ok(passphrase) => return Asked::Answer(passphrase),
                Err(_) => "Passphrase must not contain NUL.\n",
            }
        };
        if terminal.write(explanation).is_err() {
            return Asked::Cancelled;
        }
    }
    Asked::Exhausted
}

pub struct TerminalProvider<'a> {
    terminal: Option<&'a mut dyn Terminal>,
    /// Invocation-owned evidence for R-18; holds no secret and is never cleared.
    supplied: &'a Cell<bool>,
}

impl<'a> TerminalProvider<'a> {
    pub fn new(terminal: Option<&'a mut dyn Terminal>, supplied: &'a Cell<bool>) -> Self {
        Self { terminal, supplied }
    }
}

impl SessionCredentialProvider for TerminalProvider<'_> {
    fn request_passphrase(&mut self, request: &UnlockRequest) -> PassphraseResponse {
        let Some(terminal) = self.terminal.as_deref_mut() else {
            return PassphraseResponse::Unavailable;
        };
        if request.reason == UnlockReason::AuthenticationAmbiguous
            && terminal
                .write(&format!("{}\n", request.reason.guidance()))
                .is_err()
        {
            return PassphraseResponse::Cancelled;
        }
        let prompt = format!("Passphrase for key \"{}\": ", render::field(&request.label));
        let Ok(Some(mut line)) = terminal.ask(&prompt, false) else {
            return PassphraseResponse::Cancelled;
        };
        match SecretPassphrase::new(std::mem::take(&mut *line)) {
            Ok(passphrase) => {
                self.supplied.set(true);
                PassphraseResponse::Supplied(passphrase)
            }
            Err(_) => PassphraseResponse::Cancelled,
        }
    }
}

#[cfg(test)]
#[path = "prompt_tests.rs"]
mod tests;
