use crate::{
    catalog::{self, Availability, InputRule, Kind, RepoRule},
    parse::{self, Action},
};

#[test]
fn remote_remove_and_select_have_exact_catalog_and_parser_contracts() {
    for (verb, confirmed) in [("remove", true), ("select", false)] {
        let spec = catalog::find("remote", verb).unwrap();
        assert_eq!(spec.availability, Availability::Available("C2"));
        assert_eq!(spec.repo, Some(RepoRule::Required));
        assert_eq!(
            spec.kind,
            Kind::Mutation {
                confirmed,
                input: InputRule::Required
            }
        );
        assert_eq!(
            spec.options
                .iter()
                .map(|o| (o.name, o.required))
                .collect::<Vec<_>>(),
            [
                ("--repo", true),
                ("--input", true),
                ("--request-id", false),
                ("--prepare", false),
                ("--confirm", false)
            ]
        );
        for extra in [
            vec!["--repo", "fixture", "--input", "-", "--prepare"],
            vec![
                "--repo",
                "fixture",
                "--input",
                "-",
                "--request-id",
                "01M4CC0VMTTNKVYS6XWQG4CK1N",
            ],
        ] {
            let parsed = parse::parse(
                [vec!["manyhands-cli", "remote", verb], extra]
                    .concat()
                    .into_iter()
                    .map(Into::into)
                    .collect(),
            );
            let Action::Run { args, .. } = parsed.action else {
                panic!("remote mutation must parse")
            };
            assert!(parse::mode_rules(spec, &args, false).is_ok());
        }
    }
}

#[test]
fn repository_verbs_are_confirmed_c2_mutations_with_exact_options() {
    for verb in ["create", "enable", "remove"] {
        let spec = catalog::find("repo", verb).unwrap();
        assert_eq!(spec.availability, Availability::Available("C2"));
        assert_eq!(spec.repo, Some(RepoRule::Required));
        assert_eq!(
            spec.kind,
            Kind::Mutation {
                confirmed: true,
                input: if verb == "remove" {
                    InputRule::Refused
                } else {
                    InputRule::Required
                },
            }
        );
        let mut expected = vec![("--repo", true)];
        if verb != "remove" {
            expected.push(("--input", true));
        }
        expected.extend([
            ("--request-id", false),
            ("--prepare", false),
            ("--confirm", false),
        ]);
        assert_eq!(
            spec.options
                .iter()
                .map(|option| (option.name, option.required))
                .collect::<Vec<_>>(),
            expected
        );
    }
}

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
