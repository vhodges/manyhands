//! What every built command owes the boundary, whatever it does: one
//! table over the bound commands, with a row for each.
//!
//! The rules are the ones no binding can opt out of:
//!
//! 1. a finished request is answered again with its stored result, byte
//!    for byte, and nothing of its record changes;
//! 2. the same request ID with a changed input is `request_mismatch`,
//!    and the finished record stays as it is;
//! 3. a request that is refused leaves no record;
//! 4. a command that needs confirmation is refused without one, as
//!    `confirmation_required`, and any other command is refused with
//!    one, as `not_confirmable`; either way no record is left;
//! 5. a command that needs confirmation is refused as `external_change`
//!    when what its preview showed has changed by the time its
//!    confirmation is presented, and no record is left. The confirmation
//!    is a real one of the same request, so the refusal is the binding's
//!    own comparison with what was previewed and not a check of the
//!    confirmation. The rule is run for every row that says its command
//!    needs confirmation, and fails for a row that gives no way to make
//!    its preview stale: rule 4 fails a row that says its command needs
//!    none when it does, so no confirmed command is without this rule.
//!
//! A row is a fixture and the requests the rules make of it. The
//! rows are spread over four test targets, because the fixtures are:
//! `HOMES` says which target holds the row of each command, and `run`,
//! which each of those targets calls with its rows, fails for a bound
//! command that has no home, for a command of the target that has no
//! row, and for a row whose rule does not hold. A rule a command cannot
//! be driven through is an `Expect::Exempt` in its row, with the reason,
//! and is printed when the table runs.
//!
//! Nothing here names a type of the library: one of the four targets
//! compiles the library as a module of its own, so its types are not
//! those of the others. A row gives its envelopes as the JSON they
//! serialize to.
#![allow(dead_code)]

use std::panic::{AssertUnwindSafe, catch_unwind};

/// The test target that holds a command's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Home {
    /// `tests/mutation_replay`: a command that needs no confirmation
    /// and no server.
    Replay,
    /// `tests/mutation_confirmation`: a command that needs confirmation.
    Confirmation,
    /// `tests/mutation_remote`: a command that needs a server.
    Remote,
    /// `tests/mutation_bridges`: a bridge, whose tests keep to a Git
    /// home of their own.
    Bridges,
}

/// Where the row of every bound command is. `run` fails when this is
/// not exactly the bound commands, so a command that is built and has
/// no home here fails all four targets.
pub const HOMES: [(&str, Home); 30] = [
    ("document create", Home::Replay),
    ("document save", Home::Replay),
    ("document move", Home::Replay),
    ("ticket create", Home::Replay),
    ("ticket save", Home::Replay),
    ("ticket slug-assign", Home::Replay),
    ("remote add", Home::Replay),
    ("remote select", Home::Replay),
    ("index refresh", Home::Replay),
    ("index rebuild", Home::Replay),
    ("key generate", Home::Replay),
    ("key import", Home::Replay),
    ("key select", Home::Replay),
    ("key clear", Home::Replay),
    ("key remove", Home::Replay),
    ("operation resume", Home::Replay),
    ("repo create", Home::Confirmation),
    ("repo enable", Home::Confirmation),
    ("remote remove", Home::Confirmation),
    ("repo remove", Home::Confirmation),
    ("key delete", Home::Confirmation),
    ("operation abandon", Home::Confirmation),
    ("item sync", Home::Remote),
    ("repo sync", Home::Remote),
    ("host approve", Home::Remote),
    ("host replace", Home::Remote),
    ("repo identity-set", Home::Bridges),
    ("folder create", Home::Bridges),
    ("document repair", Home::Bridges),
    ("ticket repair", Home::Bridges),
];

/// The request IDs the rules use. No fixture uses them.
pub const REQUEST: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY1";

/// A confirmation ID no preview gave.
pub const NO_SUCH_CONFIRMATION: &str = "01ARZ3NDEKTSV4RRFFQ69G5FY9";

/// Which request of a row to make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The request. For a command that needs confirmation: with the
    /// confirmation of a preview of it, the same one on every call.
    Same,
    /// The same request ID with a changed input: another value of an
    /// input, or, for a command with no input but its repository,
    /// another command.
    Changed,
    /// A request the command refuses; see `Row::refused_with`.
    Refused,
    /// The request with the wrong consent: with no confirmation, for a
    /// command that needs one, and with `NO_SUCH_CONFIRMATION` for any
    /// other.
    Unconsented,
}

/// A request's record, as far as the rules read it.
#[derive(Debug, PartialEq, Eq)]
pub struct Recorded {
    pub finished: bool,
    /// Everything the record holds, as its `Debug` text.
    pub image: String,
}

/// A fixture, as the rules drive it.
pub struct Subject<'w> {
    /// Makes the request under the request ID, through `execute`, and
    /// gives its envelope as the JSON it serializes to.
    pub call: &'w dyn Fn(&str, Ask) -> Vec<u8>,
    /// The record of the request, if there is one.
    pub record: &'w dyn Fn(&str) -> Option<Recorded>,
    /// How many request records there are.
    pub request_rows: &'w dyn Fn() -> i64,
    /// For a command that needs confirmation: takes the preview whose
    /// confirmation `Ask::Same` presents, and then changes what that
    /// preview showed, as another actor would. `None` for a command that
    /// needs none.
    pub outdate: Option<&'w dyn Fn()>,
}

/// What a rule is expected to do for a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expect {
    /// The rule holds.
    Holds,
    /// The rule cannot be driven for the command, for the reason given.
    /// It is not run.
    Exempt(&'static str),
    /// The command breaks the rule, and this is what was observed: a
    /// part of the failure's message. The rule is run, and the table
    /// fails when it holds or fails in another way, so the entry cannot
    /// outlive the defect.
    KnownFailing(&'static str),
}

/// Builds a new fixture and gives it to the rule it is called with. An
/// `Err` is a fixture that could not be built.
pub type With = fn(&mut dyn FnMut(&Subject<'_>)) -> Result<(), String>;

/// One command's row.
pub struct Row {
    pub command: &'static str,
    /// Whether the command needs confirmation.
    pub confirmed: bool,
    pub with: With,
    /// The code `Ask::Refused` is refused with.
    pub refused_with: &'static str,
    pub replay: Expect,
    pub mismatch: Expect,
    pub refusal: Expect,
    pub consent: Expect,
}

impl Row {
    /// A row all four rules hold for.
    pub const fn new(
        command: &'static str,
        confirmed: bool,
        refused_with: &'static str,
        with: With,
    ) -> Self {
        Self {
            command,
            confirmed,
            with,
            refused_with,
            replay: Expect::Holds,
            mismatch: Expect::Holds,
            refusal: Expect::Holds,
            consent: Expect::Holds,
        }
    }
}

fn parsed(envelope: &[u8]) -> serde_json::Value {
    serde_json::from_slice(envelope).expect("an envelope is JSON")
}

fn code(envelope: &[u8]) -> String {
    parsed(envelope)["code"]
        .as_str()
        .expect("an envelope has a code")
        .to_owned()
}

fn text(envelope: &[u8]) -> String {
    String::from_utf8_lossy(envelope).into_owned()
}

/// Makes the request and requires that it finished: the first step of
/// the two rules about a finished request.
fn finished(command: &str, subject: &Subject<'_>) -> (Vec<u8>, Recorded, i64) {
    let first = (subject.call)(REQUEST, Ask::Same);
    assert_eq!(
        parsed(&first)["command"].as_str(),
        Some(command),
        "{command}: the row drives another command"
    );
    let record = (subject.record)(REQUEST)
        .unwrap_or_else(|| panic!("{command}: the request left no record: {}", text(&first)));
    assert!(
        record.finished,
        "{command}: the request did not finish: {}",
        text(&first)
    );
    (first, record, (subject.request_rows)())
}

fn replay(row: &Row, subject: &Subject<'_>) {
    let command = row.command;
    let (first, record, rows) = finished(command, subject);
    let again = (subject.call)(REQUEST, Ask::Same);
    assert!(
        again == first,
        "{command}: the replay differs from the stored result:\n{}\n{}",
        text(&first),
        text(&again)
    );
    assert_eq!(
        (subject.record)(REQUEST).as_ref(),
        Some(&record),
        "{command}: the replay changed the record"
    );
    assert_eq!((subject.request_rows)(), rows, "{command}: request records");
}

fn mismatch(row: &Row, subject: &Subject<'_>) {
    let command = row.command;
    let (_, record, rows) = finished(command, subject);
    let changed = (subject.call)(REQUEST, Ask::Changed);
    assert_eq!(
        code(&changed),
        "request_mismatch",
        "{command}: a changed input under a finished request's ID: {}",
        text(&changed)
    );
    assert_eq!(
        parsed(&changed)["outcome"].as_str(),
        Some("error"),
        "{command}"
    );
    assert_eq!(
        (subject.record)(REQUEST).as_ref(),
        Some(&record),
        "{command}: the changed request changed the record"
    );
    assert_eq!((subject.request_rows)(), rows, "{command}: request records");
}

/// Asserts that `ask` is refused as `expected` and leaves no record.
fn refused(command: &str, subject: &Subject<'_>, ask: Ask, expected: &str) {
    let rows = (subject.request_rows)();
    let refused = (subject.call)(REQUEST, ask);
    assert_eq!(
        code(&refused),
        expected,
        "{command}, {ask:?}: {}",
        text(&refused)
    );
    assert_eq!(
        (subject.record)(REQUEST),
        None,
        "{command}, {ask:?}: a refused request left a record"
    );
    assert_eq!(
        (subject.request_rows)(),
        rows,
        "{command}, {ask:?}: request records"
    );
}

fn refusal(row: &Row, subject: &Subject<'_>) {
    refused(row.command, subject, Ask::Refused, row.refused_with);
}

fn consent(row: &Row, subject: &Subject<'_>) {
    let expected = if row.confirmed {
        "confirmation_required"
    } else {
        "not_confirmable"
    };
    refused(row.command, subject, Ask::Unconsented, expected);
}

fn stale(row: &Row, subject: &Subject<'_>) {
    let command = row.command;
    let outdate = subject.outdate.unwrap_or_else(|| {
        panic!(
            "{command}: a command that needs confirmation has no stale-preview row: its \
             fixture gives no way to change what its preview showed"
        )
    });
    outdate();
    refused(command, subject, Ask::Same, "external_change");
}

type Rule = fn(&Row, &Subject<'_>);

fn rules(row: &Row) -> Vec<(&'static str, Rule, Expect)> {
    let mut rules: Vec<(&'static str, Rule, Expect)> = vec![
        ("replay", replay, row.replay),
        ("mismatch", mismatch, row.mismatch),
        ("refusal", refusal, row.refusal),
        ("consent", consent, row.consent),
    ];
    if row.confirmed {
        rules.push(("stale preview", stale, Expect::Holds));
    }
    rules
}

/// The message a panic carried.
fn message(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|text| (*text).to_owned()))
        .unwrap_or_default()
}

/// Runs the table of the target `home` over `rows`. `bound` is the name
/// of every command the boundary binds.
///
/// Panics when a bound command has no home, when the rows are not
/// exactly the commands of this home, and when a rule does other than
/// its row expects.
pub fn run(home: Home, bound: &[&str], rows: &[Row]) {
    run_part(home, bound, rows, &[]);
}

/// `run`, for a target whose fixtures are too slow to drive every row
/// in one test: `rows` are some of the target's rows, and `elsewhere`
/// names the commands whose rows another test of the target drives.
/// Together they are to be exactly the commands of `home`.
pub fn run_part(home: Home, bound: &[&str], rows: &[Row], elsewhere: &[&str]) {
    let mut homed: Vec<&str> = HOMES.iter().map(|(command, _)| *command).collect();
    homed.sort_unstable();
    let mut bound = bound.to_vec();
    bound.sort_unstable();
    assert_eq!(
        homed, bound,
        "every bound command has one home in `HOMES`, and a row in that target's table"
    );
    let mut expected: Vec<&str> = HOMES
        .iter()
        .filter(|(_, at)| *at == home)
        .map(|(command, _)| *command)
        .collect();
    expected.sort_unstable();
    let mut given: Vec<&str> = rows
        .iter()
        .map(|row| row.command)
        .chain(elsewhere.iter().copied())
        .collect();
    given.sort_unstable();
    assert_eq!(
        given, expected,
        "{home:?}: one row for each command of this target, and no other"
    );

    for row in rows {
        for (name, rule, expect) in rules(row) {
            let command = row.command;
            let ran = |row: &Row| {
                catch_unwind(AssertUnwindSafe(|| {
                    let mut driven = 0;
                    (row.with)(&mut |subject| {
                        driven += 1;
                        rule(row, subject);
                    })
                    .unwrap_or_else(|problem| panic!("{command}: the fixture: {problem}"));
                    assert_eq!(driven, 1, "{command}: the row gave the rule no fixture");
                }))
            };
            match expect {
                Expect::Holds => {
                    if let Err(panic) = ran(row) {
                        panic!("{command}, {name}: {}", message(panic.as_ref()));
                    }
                }
                Expect::Exempt(reason) => {
                    assert!(!reason.is_empty(), "{command}, {name}: a reason");
                    println!("conformance: {command}, {name}: exempt: {reason}");
                }
                Expect::KnownFailing(observed) => {
                    assert!(!observed.is_empty(), "{command}, {name}: what was observed");
                    match ran(row) {
                        Ok(()) => panic!(
                            "{command}, {name}: the rule holds now; its known-failing entry \
                             is to be removed"
                        ),
                        Err(panic) => {
                            let message = message(panic.as_ref());
                            assert!(
                                message.contains(observed),
                                "{command}, {name}: fails in another way than recorded: \
                                 {message}"
                            );
                            println!("conformance: {command}, {name}: known failing: {observed}");
                        }
                    }
                }
            }
        }
    }
}
