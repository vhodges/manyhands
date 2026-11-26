//! What a first-time setup intends (Wave 06 Cycle 02, design Part E, "The
//! mode, for a repository without configuration"): the mode and the file
//! the pull-request or direct enable writes, or the refusal.

use super::*;
use crate::canonical::{self, ForgeKind, ForgeSetting, Integration};

const CLOUD: &str = "ssh://git@github.com/team/repo.git";
const OTHER: &str = "ssh://git@example.invalid/team/repo.git";

fn forge(kind: ForgeKind) -> ForgeSetting {
    ForgeSetting {
        kind: Some(kind),
        web_url: None,
        pull_request_url: None,
    }
}

/// What `for_enable` intends, read back from the file it would write.
#[derive(Debug, PartialEq, Eq)]
struct Written {
    format_version: i64,
    integration: Integration,
    publication_remote: Option<String>,
    forge_kind: Option<ForgeKind>,
}

/// One row: the arguments, and the intent or the refusal.
struct Row {
    what: &'static str,
    remote: Option<&'static str>,
    integration: Option<Integration>,
    forge: Option<ForgeSetting>,
    fetch_url: Option<&'static str>,
    expect: Result<Written, IntentError>,
}

fn written(
    format_version: i64,
    integration: Integration,
    remote: Option<&str>,
    forge_kind: Option<ForgeKind>,
) -> Result<Written, IntentError> {
    Ok(Written {
        format_version,
        integration,
        publication_remote: remote.map(str::to_owned),
        forge_kind,
    })
}

/// Decision 9: `integration` when given; otherwise the forge's default for
/// the remote's fetch URL when a remote is given; otherwise direct. The
/// file is the serializer's, so a pull-request mode without a remote and an
/// invalid forge are refused before anything is written, and a remote with
/// no URL is `invalid_remote`.
#[test]
fn enable_intent_follows_decision_nine() {
    use Integration::{Direct, PullRequest};
    let rows = vec![
        Row {
            what: "no remote: direct, and the file is today's",
            remote: None,
            integration: None,
            forge: None,
            fetch_url: None,
            expect: written(1, Direct, None, None),
        },
        Row {
            what: "a direct mode named with no remote",
            remote: None,
            integration: Some(Direct),
            forge: None,
            fetch_url: None,
            expect: written(1, Direct, None, None),
        },
        Row {
            what: "a cloud remote with no mode: the forge's default, pull request",
            remote: Some("origin"),
            integration: None,
            forge: None,
            fetch_url: Some(CLOUD),
            expect: written(2, PullRequest, Some("origin"), None),
        },
        Row {
            what: "a cloud remote with direct named: direct wins",
            remote: Some("origin"),
            integration: Some(Direct),
            forge: None,
            fetch_url: Some(CLOUD),
            expect: written(1, Direct, Some("origin"), None),
        },
        Row {
            what: "a cloud remote with pull request named",
            remote: Some("origin"),
            integration: Some(PullRequest),
            forge: None,
            fetch_url: Some(CLOUD),
            expect: written(2, PullRequest, Some("origin"), None),
        },
        Row {
            what: "an unrecognised remote with no mode and no forge: direct",
            remote: Some("origin"),
            integration: None,
            forge: None,
            fetch_url: Some(OTHER),
            expect: written(1, Direct, Some("origin"), None),
        },
        Row {
            what: "an unrecognised remote with a forge given: the forge makes it pull request",
            remote: Some("origin"),
            integration: None,
            forge: Some(forge(ForgeKind::Gitlab)),
            fetch_url: Some(OTHER),
            expect: written(2, PullRequest, Some("origin"), Some(ForgeKind::Gitlab)),
        },
        Row {
            what: "an unrecognised remote with direct named and a forge: direct, the forge kept",
            remote: Some("origin"),
            integration: Some(Direct),
            forge: Some(forge(ForgeKind::Gitlab)),
            fetch_url: Some(OTHER),
            expect: written(1, Direct, Some("origin"), Some(ForgeKind::Gitlab)),
        },
        Row {
            what: "an unrecognised remote with pull request named and no forge",
            remote: Some("origin"),
            integration: Some(PullRequest),
            forge: None,
            fetch_url: Some(OTHER),
            expect: written(2, PullRequest, Some("origin"), None),
        },
        Row {
            what: "a forge with no remote: direct, written with the forge (R-9)",
            remote: None,
            integration: None,
            forge: Some(forge(ForgeKind::Github)),
            fetch_url: None,
            expect: written(1, Direct, None, Some(ForgeKind::Github)),
        },
        Row {
            what: "pull request without a remote",
            remote: None,
            integration: Some(PullRequest),
            forge: None,
            fetch_url: None,
            expect: Err(IntentError::InvalidInput),
        },
        Row {
            what: "an invalid forge: a template with no {source}",
            remote: Some("origin"),
            integration: None,
            forge: Some(ForgeSetting {
                kind: Some(ForgeKind::Github),
                web_url: None,
                pull_request_url: Some("https://forge.example.invalid/pr?to={target}".to_owned()),
            }),
            fetch_url: Some(OTHER),
            expect: Err(IntentError::InvalidInput),
        },
        Row {
            what: "an invalid forge: neither a kind nor a template",
            remote: Some("origin"),
            integration: None,
            forge: Some(ForgeSetting {
                kind: None,
                web_url: Some("https://forge.example.invalid".to_owned()),
                pull_request_url: None,
            }),
            fetch_url: Some(OTHER),
            expect: Err(IntentError::InvalidInput),
        },
        Row {
            what: "a named remote that has no fetch URL is not a remote there",
            remote: Some("origin"),
            integration: None,
            forge: None,
            fetch_url: None,
            expect: Err(IntentError::InvalidRemote),
        },
    ];

    for row in rows {
        let found = for_enable(
            "main",
            row.remote,
            row.integration,
            row.forge.as_ref(),
            row.fetch_url,
        );
        match (found, row.expect) {
            (Ok(intended), Ok(expect)) => {
                let text = canonical::serialize_repository_config(&intended.configuration)
                    .unwrap_or_else(|problem| panic!("{}: {}", row.what, problem.message));
                // What a reader of the file makes of it.
                let parsed = canonical::parse_repository_config(&text).unwrap();
                let version = text
                    .parse::<toml::Table>()
                    .unwrap()
                    .get("format_version")
                    .and_then(toml::Value::as_integer)
                    .unwrap();
                let actual = Written {
                    format_version: version,
                    integration: parsed.integration(),
                    publication_remote: parsed.publication_remote.clone(),
                    forge_kind: parsed.forge().and_then(|setting| setting.kind),
                };
                assert_eq!(actual, expect, "{}", row.what);
                assert_eq!(intended.integration, expect.integration, "{}", row.what);
                assert_eq!(parsed.primary_branch, "main", "{}", row.what);
                // A pull-request intent is a version 2 file, which an older
                // build does not read; a direct one is today's version 1.
                assert!(
                    text.contains(&format!("format_version = {}", expect.format_version)),
                    "{}: {text}",
                    row.what
                );
            }
            (Err(found), Err(expect)) => assert_eq!(found, expect, "{}", row.what),
            (Ok(_), Err(expect)) => panic!("{}: intended a file, wanted {expect:?}", row.what),
            (Err(found), Ok(_)) => panic!("{}: refused as {found:?}", row.what),
        }
    }

    // The refusals are `invalid_input` and `invalid_remote`.
    assert_eq!(
        IntentError::InvalidInput.code(),
        crate::results::ResultCode::InvalidInput
    );
    assert_eq!(
        IntentError::InvalidRemote.code(),
        crate::results::ResultCode::InvalidRemote
    );
}
