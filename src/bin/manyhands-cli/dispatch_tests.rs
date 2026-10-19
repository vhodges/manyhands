use super::*;
use crate::{
    catalog::{self, Availability},
    render::UNAVAILABLE_CODE,
};
use manyhands::results::{Effects, Scope};
use serde_json::Value;

#[test]
fn placeholders_are_exactly_the_27_remaining_available_verbs() {
    let expected: Vec<_> = catalog::COMMANDS
        .iter()
        .filter(|spec| {
            spec.availability == Availability::Available
                && (spec.resource, spec.verb) != ("id", "new")
        })
        .map(|spec| format!("{} {}", spec.resource, spec.verb))
        .collect();
    let actual = placeholder_verbs();
    assert_eq!(actual.len(), 27);
    assert_eq!(actual, expected);
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
fn each_other_available_verb_is_an_explicit_placeholder_failure() {
    let directory = tempfile::tempdir().unwrap();
    let service = RepositoryService::open_at(directory.path()).unwrap();
    for spec in catalog::COMMANDS.iter().filter(|spec| {
        spec.availability == Availability::Available && (spec.resource, spec.verb) != ("id", "new")
    }) {
        for json in [false, true] {
            let output = dispatch(spec, &ArgMatches::default(), json, &service);
            assert_eq!(output.status, 2);
            if json {
                assert!(output.stderr.is_empty());
                let value: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(value["command"], format!("{} {}", spec.resource, spec.verb));
                assert_eq!(value["code"], UNAVAILABLE_CODE.as_str());
                assert!(value["data"].is_null());
            } else {
                assert!(output.stdout.is_empty());
                assert_eq!(
                    output.stderr,
                    format!(
                        "error: {}\ncode: {}\n",
                        UNAVAILABLE_CODE.message(),
                        UNAVAILABLE_CODE.as_str()
                    )
                    .as_bytes()
                );
            }
        }
    }
}
