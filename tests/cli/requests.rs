use super::mutation::{Fixture, REQUEST, checked, golden};
use crate::support::mutation::NoPassphrase;
use manyhands::repository::{keys::SessionCredentials, *};

fn accepted(f: &Fixture) {
    let service = RepositoryService::open_at_with_failure_point_for_testing(
        f.data.path(),
        FailurePoint::AfterRemoteMutation,
    )
    .unwrap();
    let result = service.execute(
        MutationCall {
            request_id: RequestId::parse(REQUEST).unwrap(),
            confirmation: None,
            mutation: Mutation::RemoteAdd(RemoteAddInput {
                root: f.repo.root.clone(),
                name: "origin".into(),
                url: "ssh://git@example.invalid/team/repo.git".into(),
            }),
        },
        &mut SessionCredentials::new(NoPassphrase),
        &mut NoProgress,
        &CancellationToken::new(),
    );
    assert_eq!(result.outcome, manyhands::results::Outcome::Partial);
}
#[test]
fn request_show_of_a_finished_request() {
    let f = Fixture::new();
    let p = f.prepare();
    let result = f.execute(p["data"]["confirmation_id"].as_str().unwrap());
    let v = checked(
        &["request", "show", "--request-id", REQUEST, "--json"],
        &f.env,
        &[],
        0,
        "ok",
        Some("request.schema.json"),
    );
    assert_eq!(v["data"]["state"], "finished");
    assert_eq!(v["data"]["result"]["data"], result["data"]);
    golden("request_show", &v, &f.repo.root);
}
#[test]
fn request_show_of_an_accepted_request() {
    let f = Fixture::new();
    accepted(&f);
    let v = checked(
        &["request", "show", "--request-id", REQUEST, "--json"],
        &f.env,
        &[],
        0,
        "ok",
        Some("request.schema.json"),
    );
    assert_eq!(v["data"]["state"], "accepted");
    assert!(v["data"]["result"].is_null());
    golden("request_show_accepted", &v, &f.repo.root);
}
#[test]
fn unknown_requests_are_not_found() {
    let f = Fixture::new();
    for verb in ["show", "cancel"] {
        checked(
            &["request", verb, "--request-id", REQUEST, "--json"],
            &f.env,
            &[],
            2,
            "request_not_found",
            None,
        );
    }
}
#[test]
fn request_cancel_of_a_finished_local_request() {
    let f = Fixture::new();
    let p = f.prepare();
    f.execute(p["data"]["confirmation_id"].as_str().unwrap());
    let before = f.config();
    let v = checked(
        &["request", "cancel", "--request-id", REQUEST, "--json"],
        &f.env,
        &[],
        0,
        "ok",
        Some("cancel.schema.json"),
    );
    assert_eq!(v["data"]["state"], "finished");
    assert_eq!(v["data"]["requested"], false);
    assert_eq!(v["data"]["acknowledged"], false);
    assert_eq!(f.config(), before);
    golden("request_cancel", &v, &f.repo.root);
}
#[test]
fn request_cancel_of_an_accepted_local_request_is_not_requested() {
    let f = Fixture::new();
    accepted(&f);
    let before = f.config();
    let v = checked(
        &["request", "cancel", "--request-id", REQUEST, "--json"],
        &f.env,
        &[],
        0,
        "ok",
        Some("cancel.schema.json"),
    );
    assert_eq!(v["data"]["state"], "accepted");
    assert_eq!(v["data"]["requested"], false);
    assert_eq!(v["data"]["acknowledged"], false);
    assert_eq!(f.config(), before);
}
#[test]
fn request_verbs_refuse_repo() {
    let f = Fixture::new();
    for verb in ["show", "cancel"] {
        checked(
            &[
                "request",
                verb,
                "--request-id",
                REQUEST,
                "--repo",
                f.repo.root.to_str().unwrap(),
                "--json",
            ],
            &f.env,
            &[],
            2,
            "usage_error",
            None,
        );
    }
}
