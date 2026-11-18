use std::io;

use manyhands::repository::{
    SharedKeyId,
    keys::{KeySourceToken, UnlockReason},
};
use zeroize::Zeroizing;

use super::*;
use crate::{render, terminal::FakeTerminal};

const SENTINEL: &str = "pass-sentinel-ÆØ";
const NAME: &str = "Name for commits in this repository: ";
const EMAIL: &str = "Email: ";
const NEW: &str = "New passphrase for key \"work\": ";
const REPEAT: &str = "Repeat it: ";
const UNLOCK: &str = "Passphrase for key \"work\": ";

fn terminal(answers: &[Option<&str>]) -> FakeTerminal {
    FakeTerminal {
        answers: answers.iter().map(|line| line.map(str::to_owned)).collect(),
        ..Default::default()
    }
}

fn request(label: &str, reason: UnlockReason) -> UnlockRequest {
    let source = tempfile::NamedTempFile::new().unwrap();
    UnlockRequest {
        key_id: SharedKeyId::parse("01M4CC0VMTTNKVYS6XWQG4CK1N").unwrap(),
        label: label.to_owned(),
        source: KeySourceToken::observe(source.path()).unwrap(),
        reason,
    }
}

fn assert_private(terminal: &FakeTerminal) {
    assert!(!terminal.written.contains(SENTINEL));
    assert!(
        terminal
            .asks
            .iter()
            .all(|(text, _)| !text.contains(SENTINEL))
    );
}

#[test]
fn consent_accepts_y_and_yes_in_any_case_and_nothing_else() {
    for answer in ["y", "Y", "YES", "yEs", " yes "] {
        assert!(matches!(
            consent(&mut terminal(&[Some(answer)]), "preview\n"),
            Consent::Yes
        ));
    }
    for answer in [Some("n"), Some(""), Some("ye"), None] {
        assert!(matches!(
            consent(&mut terminal(&[answer]), "preview\n"),
            Consent::No
        ));
    }
}

#[test]
fn consent_writes_the_preview_then_asks() {
    let mut terminal = terminal(&[Some("y")]);
    assert!(matches!(consent(&mut terminal, "preview\n"), Consent::Yes));
    assert_eq!(terminal.written, "preview\nProceed? [y/N] ");
    assert_eq!(terminal.asks, [("Proceed? [y/N] ".into(), true)]);
}

#[test]
fn identity_asks_name_then_email_echoed() {
    let mut terminal = terminal(&[Some(" Ada "), Some("ada@example.test")]);
    let Asked::Answer(identity) = identity(&mut terminal) else {
        panic!("expected identity")
    };
    assert_eq!(identity.name, " Ada ");
    assert_eq!(identity.email, "ada@example.test");
    assert_eq!(terminal.asks, [(NAME.into(), true), (EMAIL.into(), true)]);
    assert_eq!(terminal.written, format!("{NAME}{EMAIL}"));
}

#[test]
fn identity_asks_again_after_an_empty_answer_and_gives_up_after_three() {
    for empty in ["", " \t "] {
        for pair in [[Some(empty), Some("email")], [Some("name"), Some(empty)]] {
            let mut retry = terminal(&[pair[0], pair[1], Some("name"), Some("email")]);
            assert!(matches!(identity(&mut retry), Asked::Answer(_)));
            assert_eq!(retry.asks.len(), 4);
            let mut exhausted = terminal(&[
                pair[0],
                pair[1],
                pair[0],
                pair[1],
                pair[0],
                pair[1],
                Some("unused"),
            ]);
            assert!(matches!(identity(&mut exhausted), Asked::Exhausted));
            assert_eq!(exhausted.asks.len(), 6);
            assert_eq!(exhausted.answers.len(), 1);
        }
    }
}

#[test]
fn identity_cancel_is_cancelled() {
    for answers in [vec![None], vec![Some("name"), None]] {
        let mut terminal = terminal(&answers);
        assert!(matches!(identity(&mut terminal), Asked::Cancelled));
        assert_eq!(terminal.asks.len(), answers.len());
    }
}

#[test]
fn new_passphrase_is_masked_and_asked_twice() {
    let mut terminal = terminal(&[Some(SENTINEL), Some(SENTINEL)]);
    assert!(matches!(
        new_passphrase(&mut terminal, "work"),
        Asked::Answer(_)
    ));
    assert_eq!(terminal.asks, [(NEW.into(), false), (REPEAT.into(), false)]);
    assert_eq!(terminal.written, format!("{NEW}{REPEAT}"));
    assert_private(&terminal);
}

#[test]
fn a_mismatch_or_an_empty_passphrase_asks_again_and_says_why() {
    for (first, second, explanation) in [
        (SENTINEL, "different", "Passphrases do not match.\n"),
        ("", "", "Passphrase must not be empty.\n"),
        ("", SENTINEL, "Passphrase must not be empty.\n"),
        (SENTINEL, "", "Passphrase must not be empty.\n"),
    ] {
        let mut terminal = terminal(&[Some(first), Some(second), Some(SENTINEL), Some(SENTINEL)]);
        assert!(matches!(
            new_passphrase(&mut terminal, "work"),
            Asked::Answer(_)
        ));
        assert_eq!(
            terminal.written,
            format!("{NEW}{REPEAT}{explanation}{NEW}{REPEAT}")
        );
        assert!(terminal.asks.iter().all(|(_, echo)| !echo));
        assert_private(&terminal);
    }
}

#[test]
fn three_failed_passphrase_attempts_are_exhausted() {
    for pair in [[Some(""), Some("")], [Some(SENTINEL), Some("different")]] {
        let mut terminal = terminal(&[
            pair[0],
            pair[1],
            pair[0],
            pair[1],
            pair[0],
            pair[1],
            Some("unused"),
        ]);
        assert!(matches!(
            new_passphrase(&mut terminal, "work"),
            Asked::Exhausted
        ));
        assert_eq!(terminal.asks.len(), 6);
        assert_eq!(terminal.answers.len(), 1);
        assert_private(&terminal);
    }
}

#[test]
fn a_cancel_at_either_passphrase_prompt_is_cancelled() {
    for answers in [vec![None], vec![Some(SENTINEL), None]] {
        let mut terminal = terminal(&answers);
        assert!(matches!(
            new_passphrase(&mut terminal, "work"),
            Asked::Cancelled
        ));
        assert_eq!(terminal.asks.len(), answers.len());
        assert_private(&terminal);
    }
}

#[test]
fn passphrase_whitespace_is_not_trimmed_and_nul_is_refused() {
    let mut whitespace = terminal(&[Some(" "), Some(" ")]);
    assert!(matches!(
        new_passphrase(&mut whitespace, "work"),
        Asked::Answer(_)
    ));
    let mut mismatch = terminal(&[Some(" pass "), Some("pass"), None]);
    assert!(matches!(
        new_passphrase(&mut mismatch, "work"),
        Asked::Cancelled
    ));
    assert!(mismatch.written.contains("Passphrases do not match.\n"));
    let mut nul = terminal(&[Some("a\0b"), Some("a\0b"), Some(SENTINEL), Some(SENTINEL)]);
    assert!(matches!(new_passphrase(&mut nul, "work"), Asked::Answer(_)));
    assert_eq!(nul.asks.len(), 4);
    assert!(!nul.written.contains("a\0b"));
    assert_private(&nul);
}

#[test]
fn the_provider_asks_once_masked_and_supplies_and_records_that_it_did() {
    let mut terminal = terminal(&[Some(SENTINEL), None]);
    let supplied = Cell::new(false);
    let request = request("work", UnlockReason::ProtectedKey);
    let mut provider = TerminalProvider::new(Some(&mut terminal), &supplied);
    assert!(matches!(
        provider.request_passphrase(&request),
        PassphraseResponse::Supplied(_)
    ));
    assert!(supplied.get());
    // A later dismissal must not erase the invocation's supplied evidence.
    assert!(matches!(
        provider.request_passphrase(&request),
        PassphraseResponse::Cancelled
    ));
    assert!(supplied.get());
    assert_eq!(
        terminal.asks,
        [(UNLOCK.into(), false), (UNLOCK.into(), false)]
    );
    assert_eq!(terminal.written, format!("{UNLOCK}{UNLOCK}"));
    assert_private(&terminal);
}

#[test]
fn the_provider_cancels_on_ctrl_c_and_on_an_empty_line() {
    for answer in [None, Some(""), Some("a\0b")] {
        let mut terminal = terminal(&[answer, Some("unused")]);
        let supplied = Cell::new(false);
        let mut provider = TerminalProvider::new(Some(&mut terminal), &supplied);
        assert!(matches!(
            provider.request_passphrase(&request("work", UnlockReason::ProtectedKey)),
            PassphraseResponse::Cancelled
        ));
        assert!(!supplied.get());
        assert_eq!(terminal.asks, [(UNLOCK.into(), false)]);
        assert_eq!(terminal.answers.len(), 1);
    }
}

#[test]
fn the_provider_shows_the_guidance_for_ambiguous_authentication() {
    let mut terminal = terminal(&[Some(SENTINEL)]);
    let supplied = Cell::new(false);
    let mut provider = TerminalProvider::new(Some(&mut terminal), &supplied);
    assert!(matches!(
        provider.request_passphrase(&request("work", UnlockReason::AuthenticationAmbiguous)),
        PassphraseResponse::Supplied(_)
    ));
    assert_eq!(
        terminal.written,
        format!(
            "{}\n{UNLOCK}",
            UnlockReason::AuthenticationAmbiguous.guidance()
        )
    );
    assert_eq!(terminal.asks, [(UNLOCK.into(), false)]);
    assert_private(&terminal);
}

#[test]
fn the_provider_without_a_terminal_is_unavailable() {
    for previously_supplied in [false, true] {
        let supplied = Cell::new(previously_supplied);
        let mut provider = TerminalProvider::new(None, &supplied);
        assert!(matches!(
            provider.request_passphrase(&request("work", UnlockReason::ProtectedKey)),
            PassphraseResponse::Unavailable
        ));
        assert_eq!(supplied.get(), previously_supplied);
    }
}

#[test]
fn a_label_with_control_characters_is_escaped_in_every_prompt() {
    let label = "key\x1b[2J";
    let escaped = render::field(label);
    let mut terminal = terminal(&[Some(SENTINEL), Some(SENTINEL), Some(SENTINEL)]);
    assert!(matches!(
        new_passphrase(&mut terminal, label),
        Asked::Answer(_)
    ));
    let supplied = Cell::new(false);
    let mut provider = TerminalProvider::new(Some(&mut terminal), &supplied);
    assert!(matches!(
        provider.request_passphrase(&request(label, UnlockReason::ProtectedKey)),
        PassphraseResponse::Supplied(_)
    ));
    assert_eq!(
        terminal.asks,
        [
            (format!("New passphrase for key \"{escaped}\": "), false),
            (REPEAT.into(), false),
            (format!("Passphrase for key \"{escaped}\": "), false),
        ]
    );
    assert!(!terminal.written.contains(label));
    assert_eq!(terminal.written.matches(&escaped).count(), 2);
    assert_private(&terminal);
}

#[test]
fn no_secret_appears_in_what_a_prompt_writes() {
    let mut terminal = terminal(&[
        Some(SENTINEL),
        Some(SENTINEL),
        Some(SENTINEL),
        Some("different"),
        Some(SENTINEL),
        Some(SENTINEL),
        Some(SENTINEL),
        Some(SENTINEL),
        None,
    ]);
    assert!(matches!(consent(&mut terminal, "preview\n"), Consent::No));
    assert!(matches!(identity(&mut terminal), Asked::Answer(_)));
    assert!(matches!(
        new_passphrase(&mut terminal, "work"),
        Asked::Answer(_)
    ));
    let supplied = Cell::new(false);
    let mut provider = TerminalProvider::new(Some(&mut terminal), &supplied);
    assert!(matches!(
        provider.request_passphrase(&request("work", UnlockReason::ProtectedKey)),
        PassphraseResponse::Supplied(_)
    ));
    assert!(matches!(
        provider.request_passphrase(&request("work", UnlockReason::AuthenticationAmbiguous)),
        PassphraseResponse::Cancelled
    ));
    assert_private(&terminal);
}

// Inject an I/O error at a specific write/ask without touching a real terminal.
struct FailingTerminal {
    inner: FakeTerminal,
    fail_at: usize,
    calls: usize,
}

impl FailingTerminal {
    fn new(answers: &[Option<&str>], fail_at: usize) -> Self {
        Self {
            inner: terminal(answers),
            fail_at,
            calls: 0,
        }
    }

    fn next(&mut self) -> io::Result<()> {
        self.calls += 1;
        if self.calls == self.fail_at {
            Err(io::Error::other("injected terminal failure"))
        } else {
            Ok(())
        }
    }
}

impl Terminal for FailingTerminal {
    fn write(&mut self, text: &str) -> io::Result<()> {
        self.next()?;
        self.inner.write(text)
    }

    fn ask(&mut self, prompt: &str, echo: bool) -> io::Result<Option<Zeroizing<String>>> {
        self.next()?;
        self.inner.ask(prompt, echo)
    }
}

#[test]
fn consent_write_or_ask_failure_is_no() {
    for fail_at in 1..=2 {
        let mut terminal = FailingTerminal::new(&[Some("y")], fail_at);
        assert!(matches!(consent(&mut terminal, "preview\n"), Consent::No));
        assert_eq!(terminal.calls, fail_at);
    }
}

#[test]
fn identity_ask_or_explanation_write_failure_cancels() {
    for fail_at in 1..=3 {
        let mut terminal = FailingTerminal::new(&[Some(""), Some("email")], fail_at);
        assert!(matches!(identity(&mut terminal), Asked::Cancelled));
        assert_eq!(terminal.calls, fail_at);
    }
}

#[test]
fn new_passphrase_ask_or_explanation_write_failure_cancels() {
    for fail_at in 1..=3 {
        let mut terminal = FailingTerminal::new(&[Some(SENTINEL), Some("different")], fail_at);
        assert!(matches!(
            new_passphrase(&mut terminal, "work"),
            Asked::Cancelled
        ));
        assert_eq!(terminal.calls, fail_at);
        assert_private(&terminal.inner);
    }
}

#[test]
fn provider_guidance_write_or_ask_failure_cancels_without_supplying() {
    for fail_at in 1..=2 {
        let mut terminal = FailingTerminal::new(&[Some(SENTINEL)], fail_at);
        let supplied = Cell::new(false);
        let mut provider = TerminalProvider::new(Some(&mut terminal), &supplied);
        assert!(matches!(
            provider.request_passphrase(&request("work", UnlockReason::AuthenticationAmbiguous)),
            PassphraseResponse::Cancelled
        ));
        assert!(!supplied.get());
        assert_eq!(terminal.calls, fail_at);
        assert_private(&terminal.inner);
    }
}
