//! Forge recognition and pull request links.
//!
//! The remote parser here is for links only: it reads a remote URL to name a
//! repository on a forge's web site. It never connects, and never decides host
//! trust; the transport has its own parser for that.

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::canonical::{ForgeKind, ForgeSetting, Integration};

/// Every byte is encoded except `A-Z a-z 0-9 - . _ ~ /`.
const ENCODED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'/');

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteLocation {
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
}

/// Reads `ssh://[user@]host[:port]/path` and scp-like `[user@]host:path`.
/// Anything else, including `https://`, `file://` and local paths, is `None`.
pub fn parse(url: &str) -> Option<RemoteLocation> {
    let (authority, path, scp_like) = match url.split_once("://") {
        Some((scheme, rest)) => {
            if !scheme.eq_ignore_ascii_case("ssh") {
                return None;
            }
            let (authority, path) = rest.split_once('/')?;
            (authority, path, false)
        }
        None => {
            let (authority, path) = url.split_once(':')?;
            if authority.contains('/') {
                return None;
            }
            (authority, path, true)
        }
    };
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, rest)| rest);
    let (host, port) = match host_port.rsplit_once(':') {
        Some((_, _)) if scp_like => return None,
        Some((host, port)) => (host, Some(port.parse::<u16>().ok()?)),
        None => (host_port, None),
    };
    if host.is_empty() || host.contains(|c: char| c.is_whitespace()) {
        return None;
    }
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path).trim_matches('/');
    if path.is_empty() {
        return None;
    }
    Some(RemoteLocation {
        host: host.to_ascii_lowercase(),
        port,
        path: path.to_string(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Forge {
    pub kind: Option<ForgeKind>,
    pub location: Option<RemoteLocation>,
    pub web_url: Option<String>,
    pub template: Option<String>,
}

/// A setting names the forge (its kind may be none); otherwise a remote on
/// one of the three cloud hosts does.
pub fn recognise(remote_url: Option<&str>, setting: Option<&ForgeSetting>) -> Option<Forge> {
    let location = remote_url.and_then(parse);
    if let Some(setting) = setting {
        return Some(Forge {
            kind: setting.kind,
            location,
            web_url: setting.web_url.clone(),
            template: setting.pull_request_url.clone(),
        });
    }
    let kind = match location.as_ref()?.host.as_str() {
        "bitbucket.org" => ForgeKind::BitbucketCloud,
        "github.com" => ForgeKind::Github,
        "gitlab.com" => ForgeKind::Gitlab,
        _ => return None,
    };
    Some(Forge {
        kind: Some(kind),
        location,
        web_url: None,
        template: None,
    })
}

pub fn default_integration(
    remote_url: Option<&str>,
    setting: Option<&ForgeSetting>,
) -> Integration {
    if remote_url.is_some() && recognise(remote_url, setting).is_some() {
        Integration::PullRequest
    } else {
        Integration::Direct
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequestLink {
    pub source: String,
    pub target: String,
    pub forge: Option<ForgeKind>,
    pub url: Option<String>,
}

impl PullRequestLink {
    pub fn human_line(&self) -> String {
        match &self.url {
            Some(url) => format!("pull request: {url}"),
            None => format!(
                "open a pull request from {} into {}",
                self.source, self.target
            ),
        }
    }
}

fn encode(text: &str) -> String {
    utf8_percent_encode(text, ENCODED).to_string()
}

fn cloud_address(kind: ForgeKind) -> Option<&'static str> {
    match kind {
        ForgeKind::BitbucketCloud => Some("https://bitbucket.org"),
        ForgeKind::Github => Some("https://github.com"),
        ForgeKind::Gitlab => Some("https://gitlab.com"),
        ForgeKind::BitbucketDataCenter => None,
    }
}

pub fn pull_request_link(forge: Option<&Forge>, source: &str, target: &str) -> PullRequestLink {
    PullRequestLink {
        source: source.to_string(),
        target: target.to_string(),
        forge: forge.and_then(|forge| forge.kind),
        url: forge.and_then(|forge| url(forge, &encode(source), &encode(target))),
    }
}

/// `source` and `target` arrive already encoded.
fn url(forge: &Forge, source: &str, target: &str) -> Option<String> {
    if let Some(template) = &forge.template {
        return Some(
            template
                .replace("{source}", source)
                .replace("{target}", target),
        );
    }
    let kind = forge.kind?;
    let web_url = forge
        .web_url
        .as_deref()
        .map(|web_url| web_url.trim_end_matches('/'));
    let base = web_url.or_else(|| cloud_address(kind))?;
    let path = &forge.location.as_ref()?.path;
    let segments: Vec<&str> = path.split('/').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    let path = encode(path);
    match (kind, segments.as_slice()) {
        (ForgeKind::BitbucketCloud, [_, _]) => Some(format!(
            "{base}/{path}/pull-requests/new?source={source}&dest={target}"
        )),
        (ForgeKind::BitbucketDataCenter, [key, repository]) if !key.starts_with('~') => {
            Some(format!(
                "{base}/projects/{}/repos/{}/pull-requests?create&sourceBranch=refs/heads/{source}&targetBranch=refs/heads/{target}",
                encode(&key.to_uppercase()),
                encode(repository)
            ))
        }
        (ForgeKind::Github, [_, _]) => Some(format!(
            "{base}/{path}/compare/{target}...{source}?expand=1"
        )),
        (ForgeKind::Gitlab, [_, _, ..]) => Some(format!(
            "{base}/{path}/-/merge_requests/new?merge_request[source_branch]={source}&merge_request[target_branch]={target}"
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "manyhands/ticket/01M87683441PRAYQJQBQBG96YY";

    fn location(url: &str) -> Option<(String, Option<u16>, String)> {
        parse(url).map(|l| (l.host, l.port, l.path))
    }

    fn loc(host: &str, port: Option<u16>, path: &str) -> Option<(String, Option<u16>, String)> {
        Some((host.to_string(), port, path.to_string()))
    }

    fn setting(
        kind: Option<ForgeKind>,
        web_url: Option<&str>,
        template: Option<&str>,
    ) -> ForgeSetting {
        ForgeSetting {
            kind,
            web_url: web_url.map(str::to_string),
            pull_request_url: template.map(str::to_string),
        }
    }

    fn link(remote: &str, setting: Option<&ForgeSetting>, source: &str) -> PullRequestLink {
        let forge = recognise(Some(remote), setting);
        pull_request_link(forge.as_ref(), source, "main")
    }

    #[test]
    fn remote_locations() {
        assert_eq!(
            location("git@bitbucket.org:ws/repo.git"),
            loc("bitbucket.org", None, "ws/repo")
        );
        assert_eq!(
            location("ssh://git@bitbucket.org:22/ws/repo.git"),
            loc("bitbucket.org", Some(22), "ws/repo")
        );
        assert_eq!(
            location("ssh://git@Bitbucket.Example.com:7999/key/repo.git"),
            loc("bitbucket.example.com", Some(7999), "key/repo")
        );
        assert_eq!(
            location("git@github.com:/owner/repo/"),
            loc("github.com", None, "owner/repo")
        );
        for refused in [
            "https://github.com/o/r.git",
            "/srv/repo.git",
            "file:///srv/repo.git",
            "",
            "ssh://git@host",
            "ssh://git@host:notaport/a/b",
            "git@host:",
        ] {
            assert_eq!(parse(refused), None, "{refused}");
        }
    }

    #[test]
    fn recognition_and_default_mode() {
        let cases = [
            ("git@bitbucket.org:a/b.git", ForgeKind::BitbucketCloud),
            ("git@GitHub.com:a/b.git", ForgeKind::Github),
            ("ssh://git@GITLAB.COM:22/a/b.git", ForgeKind::Gitlab),
        ];
        for (remote, kind) in cases {
            let forge = recognise(Some(remote), None).expect(remote);
            assert_eq!(forge.kind, Some(kind), "{remote}");
            assert_eq!(
                default_integration(Some(remote), None),
                Integration::PullRequest,
                "{remote}"
            );
        }

        assert_eq!(recognise(Some("git@example.com:a/b.git"), None), None);
        assert_eq!(
            default_integration(Some("git@example.com:a/b.git"), None),
            Integration::Direct
        );

        let data_center = setting(Some(ForgeKind::BitbucketDataCenter), None, None);
        let forge = recognise(Some("git@example.com:a/b.git"), Some(&data_center)).unwrap();
        assert_eq!(forge.kind, Some(ForgeKind::BitbucketDataCenter));
        assert_eq!(
            default_integration(Some("git@example.com:a/b.git"), Some(&data_center)),
            Integration::PullRequest
        );

        let template = setting(None, None, Some("https://f.example/{source}"));
        let forge = recognise(Some("git@example.com:a/b.git"), Some(&template)).unwrap();
        assert_eq!(forge.kind, None);
        assert_eq!(
            forge.template.as_deref(),
            Some("https://f.example/{source}")
        );
        assert_eq!(
            default_integration(Some("git@example.com:a/b.git"), Some(&template)),
            Integration::PullRequest
        );

        assert_eq!(default_integration(None, None), Integration::Direct);
        assert_eq!(
            default_integration(None, Some(&data_center)),
            Integration::Direct
        );
    }

    #[test]
    fn pull_request_urls() {
        let url = |l: PullRequestLink| l.url;
        let some = |s: &str| Some(s.to_string());

        let cloud = link("git@bitbucket.org:ws/repo.git", None, SOURCE);
        assert_eq!(cloud.forge, Some(ForgeKind::BitbucketCloud));
        assert_eq!(cloud.source, SOURCE);
        assert_eq!(cloud.target, "main");
        assert_eq!(
            url(cloud),
            some(&format!(
                "https://bitbucket.org/ws/repo/pull-requests/new?source={SOURCE}&dest=main"
            ))
        );
        assert_eq!(url(link("git@bitbucket.org:a/b/c.git", None, SOURCE)), None);

        let data_center = setting(
            Some(ForgeKind::BitbucketDataCenter),
            Some("https://bitbucket.example.com/bitbucket/"),
            None,
        );
        assert_eq!(
            url(link(
                "ssh://git@bitbucket.example.com:7999/key/repo.git",
                Some(&data_center),
                SOURCE
            )),
            some(&format!(
                "https://bitbucket.example.com/bitbucket/projects/KEY/repos/repo/pull-requests?create&sourceBranch=refs/heads/{SOURCE}&targetBranch=refs/heads/main"
            ))
        );
        let no_web_url = setting(Some(ForgeKind::BitbucketDataCenter), None, None);
        assert_eq!(
            url(link(
                "ssh://git@bitbucket.example.com:7999/key/repo.git",
                Some(&no_web_url),
                SOURCE
            )),
            None
        );
        assert_eq!(
            url(link(
                "ssh://git@bitbucket.example.com:7999/~me/repo.git",
                Some(&data_center),
                SOURCE
            )),
            None
        );

        assert_eq!(
            url(link("git@github.com:o/r.git", None, SOURCE)),
            some(&format!(
                "https://github.com/o/r/compare/main...{SOURCE}?expand=1"
            ))
        );
        let enterprise = setting(
            Some(ForgeKind::Github),
            Some("https://ghe.example.com"),
            None,
        );
        assert_eq!(
            url(link(
                "git@ghe.example.com:o/r.git",
                Some(&enterprise),
                SOURCE
            )),
            some(&format!(
                "https://ghe.example.com/o/r/compare/main...{SOURCE}?expand=1"
            ))
        );
        let github_only = setting(Some(ForgeKind::Github), None, None);
        assert_eq!(
            url(link(
                "git@ghe.example.com:o/r.git",
                Some(&github_only),
                SOURCE
            )),
            some(&format!(
                "https://github.com/o/r/compare/main...{SOURCE}?expand=1"
            ))
        );

        assert_eq!(
            url(link("git@gitlab.com:group/sub/r.git", None, SOURCE)),
            some(&format!(
                "https://gitlab.com/group/sub/r/-/merge_requests/new?merge_request[source_branch]={SOURCE}&merge_request[target_branch]=main"
            ))
        );
        assert_eq!(url(link("git@gitlab.com:r.git", None, SOURCE)), None);

        let templated = setting(
            Some(ForgeKind::Github),
            None,
            Some("https://forge.example/new?from={source}&to={target}"),
        );
        let templated = link("git@example.com:a/b.git", Some(&templated), SOURCE);
        assert_eq!(templated.forge, Some(ForgeKind::Github));
        assert_eq!(
            templated.url,
            some(&format!("https://forge.example/new?from={SOURCE}&to=main"))
        );

        let three = link("git@github.com:a/b/c.git", None, SOURCE);
        assert_eq!(three.url, None);
        assert_eq!(three.source, SOURCE);
        assert_eq!(three.target, "main");
        let none = pull_request_link(None, SOURCE, "main");
        assert_eq!(none.url, None);
        assert_eq!(none.forge, None);
        assert_eq!(none.source, SOURCE);
        assert_eq!(none.target, "main");

        let awkward = "a&b #c?d é";
        let encoded = "a%26b%20%23c%3Fd%20%C3%A9";
        let cloud = link("git@bitbucket.org:ws/repo.git", None, awkward);
        assert_eq!(
            cloud.url,
            some(&format!(
                "https://bitbucket.org/ws/repo/pull-requests/new?source={encoded}&dest=main"
            ))
        );
        assert_eq!(cloud.source, awkward);
        let template = setting(None, None, Some("https://f.example/?s={source}"));
        assert_eq!(
            url(link("git@example.com:a/b.git", Some(&template), awkward)),
            some(&format!("https://f.example/?s={encoded}"))
        );
    }

    #[test]
    fn human_lines() {
        let with_url = PullRequestLink {
            source: SOURCE.to_string(),
            target: "main".to_string(),
            forge: None,
            url: Some("https://x.example/pr".to_string()),
        };
        assert_eq!(with_url.human_line(), "pull request: https://x.example/pr");
        let without = PullRequestLink {
            url: None,
            ..with_url
        };
        assert_eq!(
            without.human_line(),
            format!("open a pull request from {SOURCE} into main")
        );
    }
}
