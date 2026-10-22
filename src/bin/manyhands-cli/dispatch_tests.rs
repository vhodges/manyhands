use super::*;
use crate::catalog::{self, Availability};
use manyhands::results::{Effects, Scope};
use serde_json::Value;

#[test]
fn no_available_verb_is_a_placeholder() {
    assert!(placeholder_verbs().is_empty());
}

#[test]
fn id_new_dispatches_a_canonical_id_in_both_modes() {
    let directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(directory.path()).unwrap();
    let spec = catalog::find("id", "new").unwrap();
    for json in [false, true] {
        let output = dispatch(spec, &ArgMatches::default(), json, &service);
        assert_eq!(output.status, 0);
        assert!(output.stderr.is_empty());
        let id = if json {
            let value: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["command"], "id new");
            assert_eq!(value["code"], "ok");
            assert_eq!(
                value["scope"],
                serde_json::to_value(Scope::default()).unwrap()
            );
            assert_eq!(
                value["effects"],
                serde_json::to_value(Effects::not_requested()).unwrap()
            );
            value["data"]["id"].as_str().unwrap().to_owned()
        } else {
            let text = String::from_utf8(output.stdout).unwrap();
            assert_eq!(text.len(), 27);
            assert!(text.ends_with('\n'));
            text.trim_end().to_owned()
        };
        assert!(id.parse::<manyhands::canonical::ItemId>().is_ok());
    }
}

#[test]
fn all_28_available_verbs_reach_a_real_dispatch_arm() {
    let directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(directory.path()).unwrap();
    let missing = directory.path().join("missing-repository");
    let mut count = 0;
    for spec in catalog::COMMANDS
        .iter()
        .filter(|spec| matches!(spec.availability, Availability::Available(_)))
    {
        let mut argv = vec!["manyhands-cli", spec.resource, spec.verb];
        for option in spec.options.iter().filter(|option| option.required) {
            argv.extend([
                option.name,
                match option.name {
                    "--repo" => missing.to_str().unwrap(),
                    "--authority" => "example.invalid",
                    "--slug" => "mh-vh-k9x2b",
                    _ => "01ARZ3NDEKTSV4RRFFQ69G5FAV",
                },
            ]);
        }
        if matches!((spec.resource, spec.verb), ("document" | "ticket", "show")) {
            argv.extend(["--id", "01ARZ3NDEKTSV4RRFFQ69G5FAV"]);
        }
        let invocation = parse::parse(argv.into_iter().map(Into::into).collect());
        let parse::Action::Run { args, .. } = invocation.action else {
            panic!("expected Run for {spec:?}")
        };
        let output = dispatch(spec, &args, true, &service);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["command"], format!("{} {}", spec.resource, spec.verb));
        // An empty placeholder list cannot hide a missing dispatch arm behind
        // the internal-routing fallback. Each arm must reach its actual read.
        assert_ne!(value["code"], "command_unavailable", "{spec:?}");
        assert_ne!(value["code"], "internal_error", "{spec:?}");
        count += 1;
    }
    assert_eq!(count, 28);
}

#[test]
fn item_id_helper_uses_canonical_ids_and_repository_scope() {
    let directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(directory.path()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let repository = git2::Repository::init(root.path()).unwrap();
    let mut config = repository.config().unwrap();
    config.set_str("user.name", "Test").unwrap();
    config
        .set_str("user.email", "test@example.invalid")
        .unwrap();
    // Resolve an actual registered repository, with the minimum enable fixture.
    let signature = git2::Signature::now("Test", "test@example.invalid").unwrap();
    let tree_id = repository.index().unwrap().write_tree().unwrap();
    let tree = repository.find_tree(tree_id).unwrap();
    repository
        .commit(Some("HEAD"), &signature, &signature, "Initial", &tree, &[])
        .unwrap();
    let branch = repository.head().unwrap().shorthand().unwrap().to_owned();
    service
        .enable(manyhands::repository::EnableRepositoryRequest {
            root: root.path().to_owned(),
            primary_branch: branch,
            identity: None,
            operation_id: manyhands::repository::OperationId::new(),
        })
        .unwrap();
    let repo = service.resolve_repository(root.path()).unwrap();
    for id in [
        "01ARZ3NDEKTSV4RRFFQ69G5FAV",
        "01arz3ndektsv4rrffq69g5fav",
        "mh-vh-k9x2b",
    ] {
        let invocation = crate::parse::parse(
            [
                "manyhands-cli",
                "ticket",
                "show",
                "--repo",
                root.path().to_str().unwrap(),
                "--id",
                id,
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
        );
        let crate::parse::Action::Run { args, .. } = invocation.action else {
            panic!("expected Run")
        };
        let result = item_id(&args, &repo);
        if id == "01ARZ3NDEKTSV4RRFFQ69G5FAV" {
            assert_eq!(result.unwrap().to_string(), id);
        } else {
            let envelope =
                serde_json::to_value(result.unwrap_err().to_envelope::<Value>("ticket show"))
                    .unwrap();
            assert_eq!(envelope["code"], "invalid_id");
            assert_eq!(
                envelope["scope"],
                serde_json::to_value(repo.scope()).unwrap()
            );
        }
    }
}

#[test]
fn ticket_filter_helper_accepts_the_narrower_future_relationship_options() {
    for verb in ["ready", "blocked", "plan"] {
        let invocation = crate::parse::parse(
            [
                "manyhands-cli",
                "ticket",
                verb,
                "--repo",
                "/repository",
                "--status",
                "open",
                "--type",
                "task",
                "--project",
                "manyhands",
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
        );
        let crate::parse::Action::Run { args, .. } = invocation.action else {
            panic!("expected Run")
        };
        assert_eq!(
            ticket_filter(&args),
            manyhands::repository::TicketFilter {
                status: Some("open".into()),
                ticket_type: Some("task".into()),
                project: Some("manyhands".into()),
                ..Default::default()
            }
        );
    }
}
