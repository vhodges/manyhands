use crate::{
    catalog::{self, Availability, InputRule, Kind, RepoRule},
    parse::{self, Action},
};
#[test]
fn first_four_verbs_have_their_c2_contracts_and_required_options() {
    for (resource, verb, kind, repo, valid) in [
        (
            "repo",
            "identity-set",
            Kind::Mutation {
                confirmed: true,
                input: InputRule::Required,
            },
            RepoRule::Required,
            vec!["--repo", "fixture", "--input", "input.json"],
        ),
        (
            "remote",
            "add",
            Kind::Mutation {
                confirmed: false,
                input: InputRule::Required,
            },
            RepoRule::Required,
            vec!["--repo", "fixture", "--input", "input.json"],
        ),
        (
            "request",
            "show",
            Kind::Read,
            RepoRule::Refused,
            vec!["--request-id", "01M4CC0VMTTNKVYS6XWQG4CK1N"],
        ),
        (
            "request",
            "cancel",
            Kind::Read,
            RepoRule::Refused,
            vec!["--request-id", "01M4CC0VMTTNKVYS6XWQG4CK1N"],
        ),
    ] {
        let spec = catalog::find(resource, verb).unwrap();
        assert_eq!(spec.availability, Availability::Available("C2"));
        assert_eq!(spec.kind, kind);
        assert_eq!(spec.repo, Some(repo));
        let args = |options: &[&str]| {
            parse::parse(
                [vec!["manyhands-cli", resource, verb], options.to_vec()]
                    .concat()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            )
            .action
        };
        assert!(matches!(args(&valid), Action::Run { .. }));
        assert!(matches!(args(&[]), Action::Usage { .. }));
        for option in spec.options.iter().filter(|o| o.required) {
            let mut missing = valid.clone();
            let index = missing.iter().position(|s| *s == option.name).unwrap();
            missing.drain(index..index + 2);
            assert!(matches!(args(&missing), Action::Usage { .. }));
        }
        if resource == "request" {
            for option in ["--repo", "--input", "--prepare", "--confirm"] {
                let mut invalid = valid.clone();
                invalid.push(option);
                if option != "--prepare" {
                    invalid.push("sentinel");
                }
                assert!(matches!(args(&invalid), Action::Usage { .. }));
            }
        }
    }
}
