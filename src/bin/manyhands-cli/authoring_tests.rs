use crate::{
    catalog,
    parse::{self, Action},
};

// The binary cases exercise execution. This pins the common confirmation grammar
// for the three unconfirmed mutations, including prepare's request-ID exclusion.
#[test]
fn authoring_maintenance_verbs_share_the_confirmation_flag_rules() {
    for (resource, verb) in [
        ("index", "refresh"),
        ("index", "rebuild"),
        ("folder", "create"),
    ] {
        let spec = catalog::find(resource, verb).unwrap();
        for (flags, valid) in [
            (vec!["--prepare"], true),
            (vec!["--request-id", "01M4CC0VMTTNKVYS6XWQG4CK1N"], true),
            (
                vec!["--prepare", "--request-id", "01M4CC0VMTTNKVYS6XWQG4CK1N"],
                false,
            ),
            (
                vec!["--prepare", "--confirm", "01M4CC0VMTTNKVYS6XWQG4CK1N"],
                false,
            ),
        ] {
            let mut argv = vec!["manyhands-cli", resource, verb, "--repo", "fixture"];
            if resource == "folder" {
                argv.extend(["--input", "-"]);
            }
            argv.extend(flags);
            let parsed = parse::parse(argv.into_iter().map(Into::into).collect());
            let Action::Run { args, .. } = parsed.action else {
                panic!("maintenance verb must parse");
            };
            assert_eq!(parse::mode_rules(spec, &args, false).is_ok(), valid);
        }
    }
}
