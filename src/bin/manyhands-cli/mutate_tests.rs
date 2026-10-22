use super::*;
use crate::{
    catalog::{Availability, InputRule, Kind, RepoRule},
    terminal::FakeTerminal,
};
use manyhands::results::{Outcome, RecoveryAction, RecoveryActionKind, Scope};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

static SPEC: CommandSpec = CommandSpec {
    resource: "repo",
    verb: "identity-set",
    summary: "",
    availability: Availability::Available("C2"),
    repo: Some(RepoRule::Required),
    options: &[],
    kind: Kind::Mutation {
        confirmed: true,
        input: InputRule::Required,
    },
};
fn id() -> RequestId {
    RequestId::parse("01M4CC0VMTTNKVYS6XWQG4CK1N").unwrap()
}
fn confirmation() -> ConfirmationId {
    ConfirmationId::parse("01M4CC0VMTTNKVYS6XWQG4CK1P").unwrap()
}
fn answer(
    code: ResultCode,
    outcome: Outcome,
    retry: Option<RequestId>,
) -> Envelope<MutationDataDto> {
    let mut e = Envelope::mutation("repo identity-set", Scope::default(), outcome, code)
        .with_request_id(id().to_string());
    if let Some(id) = retry {
        e.recovery.push(RecoveryAction::new(
            RecoveryActionKind::RequestRetry,
            [("request_id", id.to_string().into())],
        ));
    }
    e
}
#[test]
fn flow_table_and_priorities() {
    for confirmed in [false, true] {
        for interactive in [false, true] {
            for exists in [false, true] {
                assert_eq!(
                    flow(true, Some(confirmation()), confirmed, interactive, exists),
                    Flow::Prepare
                );
                assert_eq!(
                    flow(false, Some(confirmation()), confirmed, interactive, exists),
                    Flow::Execute {
                        confirmation: Some(confirmation())
                    }
                );
                assert_eq!(
                    flow(false, None, confirmed, interactive, exists),
                    if confirmed && interactive && !exists {
                        Flow::Interactive
                    } else {
                        Flow::Execute { confirmation: None }
                    }
                );
            }
        }
    }
}
#[test]
fn retry_requires_code_or_partial_and_the_first_action_for_this_request() {
    for (code, outcome, retry, expected) in [
        (ResultCode::Busy, Outcome::Error, Some(id()), true),
        (
            ResultCode::InternalError,
            Outcome::Partial,
            Some(id()),
            true,
        ),
        (
            ResultCode::Busy,
            Outcome::Error,
            Some(RequestId::new()),
            false,
        ),
        (
            ResultCode::TransportUnavailable,
            Outcome::Error,
            Some(id()),
            false,
        ),
        (ResultCode::RequestMismatch, Outcome::Partial, None, false),
        (ResultCode::Ok, Outcome::Success, Some(id()), false),
    ] {
        assert_eq!(should_retry(&answer(code, outcome, retry), id()), expected);
    }
    let mut e = answer(ResultCode::Busy, Outcome::Error, Some(id()));
    e.recovery
        .insert(0, RecoveryAction::new(RecoveryActionKind::IndexRebuild, []));
    assert!(!should_retry(&e, id()));
}

#[derive(Default)]
struct FakeBoundary {
    preparations: RefCell<VecDeque<Envelope<PrepareDataDto>>>,
    answers: RefCell<VecDeque<Envelope<MutationDataDto>>>,
    exists: RefCell<Option<Result<bool, Envelope<RequestDto>>>>,
    calls: RefCell<Vec<&'static str>>,
    confirmations: RefCell<Vec<Option<ConfirmationId>>>,
    identities: RefCell<Vec<Option<String>>>,
}
impl Boundary for FakeBoundary {
    fn prepare(&self, mutation: &Mutation) -> Envelope<PrepareDataDto> {
        self.calls.borrow_mut().push("prepare");
        if let Mutation::RepoCreate(input) = mutation {
            self.identities
                .borrow_mut()
                .push(input.identity.as_ref().map(|i| i.name.clone()));
        }
        self.preparations
            .borrow_mut()
            .pop_front()
            .expect("unexpected prepare")
    }
    fn execute(
        &self,
        call: MutationCall,
        _: &mut SessionCredentials<TerminalProvider<'_>>,
        _: &mut dyn ProgressSink,
    ) -> Envelope<MutationDataDto> {
        self.calls.borrow_mut().push("execute");
        assert_eq!(call.request_id, id());
        self.confirmations.borrow_mut().push(call.confirmation);
        self.answers
            .borrow_mut()
            .pop_front()
            .expect("unexpected execute")
    }
    fn request_exists(&self, request_id: RequestId) -> Result<bool, Envelope<RequestDto>> {
        assert_eq!(request_id, id());
        self.calls.borrow_mut().push("exists");
        self.exists.borrow_mut().take().expect("unexpected lookup")
    }
}
fn preview() -> Envelope<PrepareDataDto> {
    Envelope::mutation(
        "repo identity-set",
        Scope::default(),
        Outcome::Success,
        ResultCode::Ok,
    )
    .with_data(PrepareDataDto::Preview(Box::new(PreviewDto {
        confirmation_id: confirmation().to_string(),
        expires_at: "tomorrow".into(),
        command: "repo identity-set".into(),
        scope: Scope::default(),
        effects: vec![],
        details: PreviewDetailsDto::RepositoryRemoval(RepositoryRemovalPreviewDto {
            root: "/fixture".into(),
            enabled_at: None,
            files_changed: false,
            git_state_changed: false,
        }),
    })))
}
fn flags() -> MutationFlags {
    MutationFlags {
        request_id: Some(id()),
        prepare: false,
        confirm: None,
    }
}
fn session(interactive: bool) -> Session {
    Session {
        json: !interactive,
        interactive,
        terminal: None,
    }
}
fn build(_: &Extras) -> Mutation {
    Mutation::RemoteAdd(RemoteAddInput {
        root: "/fixture".into(),
        name: "origin".into(),
        url: "ssh://example.test/repo".into(),
    })
}
fn invoke(
    b: &FakeBoundary,
    s: &mut Session,
    f: MutationFlags,
    retry: bool,
    sleeps: &mut Vec<Duration>,
) -> Output {
    run(
        b,
        s,
        &SPEC,
        f,
        Plan {
            identity_prompt: false,
            retry,
        },
        &mut build,
        &mut |d| sleeps.push(d),
    )
}
#[test]
fn a_busy_answer_is_retried_three_times_at_most_with_the_set_delays() {
    let b = FakeBoundary::default();
    let mut last = answer(ResultCode::Busy, Outcome::Error, Some(id()));
    last.operation_id = Some("last".into());
    b.answers.borrow_mut().extend([
        answer(ResultCode::Busy, Outcome::Error, Some(id())),
        answer(ResultCode::Busy, Outcome::Error, Some(id())),
        answer(ResultCode::Busy, Outcome::Error, Some(id())),
        last.clone(),
    ]);
    let mut sleeps = vec![];
    let out = invoke(&b, &mut session(false), flags(), true, &mut sleeps);
    assert_eq!(sleeps, RETRY_DELAYS_MS.map(Duration::from_millis));
    assert_eq!(b.calls.borrow().as_slice(), ["execute"; 4]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap(),
        serde_json::to_value(last).unwrap()
    );
    assert!(out.stderr.is_empty());
}
#[test]
fn a_retry_presents_the_same_confirmation() {
    let b = FakeBoundary::default();
    b.answers.borrow_mut().extend([
        answer(ResultCode::Busy, Outcome::Error, Some(id())),
        answer(ResultCode::Ok, Outcome::Success, None),
    ]);
    let mut f = flags();
    f.confirm = Some(confirmation());
    assert_eq!(
        invoke(&b, &mut session(false), f, true, &mut vec![]).status,
        0
    );
    assert_eq!(
        b.confirmations.borrow().as_slice(),
        [Some(confirmation()); 2]
    );
}
#[test]
fn a_retry_stops_at_the_first_answer_that_does_not_qualify() {
    let b = FakeBoundary::default();
    b.answers.borrow_mut().extend([
        answer(ResultCode::Busy, Outcome::Error, Some(id())),
        answer(ResultCode::TransportUnavailable, Outcome::Error, Some(id())),
    ]);
    let mut sleeps = vec![];
    assert_eq!(
        invoke(&b, &mut session(false), flags(), true, &mut sleeps).status,
        5
    );
    assert_eq!(sleeps, [Duration::from_millis(100)]);
    assert_eq!(b.calls.borrow().len(), 2);
}
#[test]
fn no_retry_when_the_plan_says_so() {
    let b = FakeBoundary::default();
    b.answers
        .borrow_mut()
        .push_back(answer(ResultCode::Busy, Outcome::Error, Some(id())));
    let mut sleeps = vec![];
    invoke(&b, &mut session(false), flags(), false, &mut sleeps);
    assert!(sleeps.is_empty());
    assert_eq!(b.calls.borrow().len(), 1);
}
#[test]
fn a_show_request_failure_other_than_not_found_is_rendered_and_starts_nothing() {
    let b = FakeBoundary::default();
    *b.exists.borrow_mut() = Some(Err(Envelope::failure(
        "request show",
        Scope::default(),
        ResultCode::Busy,
        vec![],
    )));
    let out = invoke(&b, &mut session(true), flags(), true, &mut vec![]);
    assert_eq!(out.status, 5);
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains("code: busy")
    );
    assert_eq!(b.calls.borrow().as_slice(), ["exists"]);
}
struct SharedTerminal(Rc<RefCell<FakeTerminal>>);
impl Terminal for SharedTerminal {
    fn write(&mut self, s: &str) -> std::io::Result<()> {
        self.0.borrow_mut().write(s)
    }
    fn ask(&mut self, s: &str, e: bool) -> std::io::Result<Option<zeroize::Zeroizing<String>>> {
        self.0.borrow_mut().ask(s, e)
    }
}
fn interactive(answers: &[&str]) -> (Session, Rc<RefCell<FakeTerminal>>) {
    let t = Rc::new(RefCell::new(FakeTerminal::default()));
    t.borrow_mut()
        .answers
        .extend(answers.iter().map(|s| Some((*s).into())));
    (
        Session {
            json: false,
            interactive: true,
            terminal: Some(Box::new(SharedTerminal(t.clone()))),
        },
        t,
    )
}
#[test]
fn interactive_prepare_then_consent_yes_executes_with_the_previews_confirmation() {
    let b = FakeBoundary::default();
    *b.exists.borrow_mut() = Some(Ok(false));
    b.preparations.borrow_mut().push_back(preview());
    b.answers
        .borrow_mut()
        .push_back(answer(ResultCode::Ok, Outcome::Success, None));
    let (mut s, t) = interactive(&["yes"]);
    assert_eq!(invoke(&b, &mut s, flags(), true, &mut vec![]).status, 0);
    assert_eq!(
        b.calls.borrow().as_slice(),
        ["exists", "prepare", "execute"]
    );
    assert_eq!(b.confirmations.borrow().as_slice(), [Some(confirmation())]);
    assert_eq!(t.borrow().asks.len(), 1);
    assert!(!t.borrow().written.contains("Apply with"));
}
#[test]
fn consent_no_is_a_cancelled_refusal_echoing_the_request_id() {
    let b = FakeBoundary::default();
    *b.exists.borrow_mut() = Some(Ok(false));
    b.preparations.borrow_mut().push_back(preview());
    let (mut s, _) = interactive(&["no"]);
    let out = invoke(&b, &mut s, flags(), true, &mut vec![]);
    assert_eq!(out.status, 130);
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(&id().to_string())
    );
    assert_eq!(b.calls.borrow().as_slice(), ["exists", "prepare"]);
}
#[test]
fn identity_required_then_asked_then_prepared_again() {
    let b = FakeBoundary::default();
    *b.exists.borrow_mut() = Some(Ok(false));
    b.preparations.borrow_mut().extend([
        Envelope::failure(
            "repo create",
            Scope::default(),
            ResultCode::IdentityRequired,
            vec![],
        ),
        preview(),
    ]);
    b.answers
        .borrow_mut()
        .push_back(answer(ResultCode::Ok, Outcome::Success, None));
    let (mut s, t) = interactive(&["Name sentinel", "email-sentinel", "yes"]);
    let out = run(
        &b,
        &mut s,
        &SPEC,
        flags(),
        Plan {
            identity_prompt: true,
            retry: true,
        },
        &mut |e| {
            Mutation::RepoCreate(RepoCreateInput {
                root: "/fixture".into(),
                primary_branch: "main".into(),
                identity: e.identity.clone(),
                slug_prefix: None,
            })
        },
        &mut |_| panic!("sleep"),
    );
    assert_eq!(out.status, 0);
    assert_eq!(
        b.identities.borrow().as_slice(),
        [None, Some("Name sentinel".into())]
    );
    assert_eq!(t.borrow().asks.len(), 3);
    assert!(!String::from_utf8(out.stderr).unwrap().contains("sentinel"));
    let note = identity_note(id());
    assert!(note.contains(&id().to_string()));
    assert!(note.contains("\"identity\""));
    assert!(!note.contains("sentinel"));
}
#[test]
fn an_existing_record_skips_prepare_and_consent() {
    let b = FakeBoundary::default();
    *b.exists.borrow_mut() = Some(Ok(true));
    b.answers
        .borrow_mut()
        .push_back(answer(ResultCode::Ok, Outcome::Success, None));
    let (mut s, t) = interactive(&[]);
    assert_eq!(invoke(&b, &mut s, flags(), true, &mut vec![]).status, 0);
    assert!(t.borrow().asks.is_empty());
    assert_eq!(b.calls.borrow().as_slice(), ["exists", "execute"]);
}
#[test]
fn prepare_flag_calls_prepare_only() {
    let b = FakeBoundary::default();
    b.preparations.borrow_mut().push_back(preview());
    let (mut s, t) = interactive(&[]);
    let out = invoke(
        &b,
        &mut s,
        MutationFlags {
            request_id: None,
            prepare: true,
            confirm: None,
        },
        true,
        &mut vec![],
    );
    assert_eq!(out.status, 0);
    assert!(t.borrow().asks.is_empty());
    assert_eq!(b.calls.borrow().as_slice(), ["prepare"]);
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("Apply with")
    );
}
#[test]
fn refusals_have_null_operation_and_data_empty_scope_no_recovery() {
    for request_id in [None, Some(id())] {
        for code in [
            ResultCode::InvalidId,
            ResultCode::InvalidInput,
            ResultCode::Cancelled,
            ResultCode::UnlockRequired,
        ] {
            let out = refusal(
                &session(false),
                "repo identity-set",
                request_id,
                code,
                Some("fixed detail".into()),
            );
            let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(
                v["request_id"],
                serde_json::json!(request_id.map(|id| id.to_string()))
            );
            assert!(v["operation_id"].is_null() && v["data"].is_null());
            assert_eq!(v["scope"], serde_json::to_value(Scope::default()).unwrap());
            assert_eq!(
                v["effects"],
                serde_json::to_value(manyhands::results::Effects::not_requested()).unwrap()
            );
            assert_eq!(v["recovery"], serde_json::json!([]));
            assert_eq!(v["message"], code.message());
            assert!(out.stderr.is_empty());
        }
    }
}
