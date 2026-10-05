use std::net::{IpAddr, Ipv6Addr};

use git2::Repository;

use super::{SshAuthority, SshDirection, SshTransportErrorKind};

#[derive(Clone, PartialEq, Eq)]
pub(in super::super) struct SshEndpoint {
    pub(in super::super) authority: SshAuthority,
    pub(in super::super) username: Option<String>,
    pub(in super::super) connection_url: String,
}

pub(in super::super) fn configured_remote_endpoint(
    repository: &Repository,
    remote_name: &str,
    direction: SshDirection,
) -> Result<SshEndpoint, SshTransportErrorKind> {
    let config = repository
        .config()
        .and_then(|mut config| config.snapshot())
        .map_err(|_| SshTransportErrorKind::ConfigurationInvalid)?;
    let fetch_key = format!("remote.{remote_name}.url");
    let fetch_url = config
        .get_string(&fetch_key)
        .map_err(|_| SshTransportErrorKind::ConfigurationInvalid)?;
    let configured_url = match direction {
        SshDirection::Fetch => fetch_url,
        SshDirection::Push => {
            let push_key = format!("remote.{remote_name}.pushurl");
            match config.get_string(&push_key) {
                Ok(push_url) => push_url,
                Err(error) if error.code() == git2::ErrorCode::NotFound => fetch_url,
                Err(_) => return Err(SshTransportErrorKind::ConfigurationInvalid),
            }
        }
    };
    let configured = parse_ssh_endpoint(&configured_url)?;
    let effective = repository
        .remote_anonymous(&configured_url)
        .map_err(|_| SshTransportErrorKind::ConfigurationInvalid)?;
    let effective_url = match direction {
        SshDirection::Fetch => effective.url(),
        SshDirection::Push => effective.pushurl().or_else(|| effective.url()),
    }
    .ok_or(SshTransportErrorKind::ConfigurationInvalid)?;
    let effective = parse_ssh_endpoint(effective_url)?;
    if configured != effective {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    Ok(configured)
}

pub(in super::super) fn parse_ssh_endpoint(
    url: &str,
) -> Result<SshEndpoint, SshTransportErrorKind> {
    if url.is_empty()
        || url
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        parse_url_endpoint(rest)
    } else {
        parse_scp_endpoint(url)
    }
}

fn parse_url_endpoint(rest: &str) -> Result<SshEndpoint, SshTransportErrorKind> {
    // libgit2 excludes query/fragment suffixes from the SSH service target.
    // They are unsupported in URLs, while SCP path bytes remain literal.
    if rest.contains(['?', '#']) {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    let (authority, path) = rest
        .split_once('/')
        .ok_or(SshTransportErrorKind::ConfigurationInvalid)?;
    if authority.is_empty() || path.is_empty() || authority.contains('%') {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    let (username, host_port) = split_userinfo(authority)?;
    let (host, port) = parse_host_port(host_port)?;
    endpoint(username, host, port, path, PathSyntax::Url)
}

fn parse_scp_endpoint(url: &str) -> Result<SshEndpoint, SshTransportErrorKind> {
    if url.contains("://") || url.contains('\\') {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    let (authority, path) = split_scp_authority(url)?;
    if authority.is_empty() || path.is_empty() || authority.contains('%') {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    let (username, host) = split_userinfo(authority)?;
    if matches!(host, "file" | "http" | "https")
        || (host.len() == 1 && host.as_bytes()[0].is_ascii_alphabetic())
    {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    let host = parse_host(host, true)?;
    endpoint(username, host, 22, path, PathSyntax::Scp)
}

fn split_scp_authority(url: &str) -> Result<(&str, &str), SshTransportErrorKind> {
    if let Some(open) = url.find('[') {
        let close = url[open + 1..]
            .find(']')
            .map(|offset| open + 1 + offset)
            .ok_or(SshTransportErrorKind::ConfigurationInvalid)?;
        if url.as_bytes().get(close + 1) != Some(&b':') {
            return Err(SshTransportErrorKind::ConfigurationInvalid);
        }
        Ok((&url[..=close], &url[close + 2..]))
    } else {
        url.split_once(':')
            .ok_or(SshTransportErrorKind::ConfigurationInvalid)
    }
}

fn split_userinfo(authority: &str) -> Result<(Option<String>, &str), SshTransportErrorKind> {
    match authority.split_once('@') {
        Some((username, host))
            if !username.is_empty()
                && !host.contains('@')
                && username.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')
                }) =>
        {
            Ok((Some(username.to_owned()), host))
        }
        Some(_) => Err(SshTransportErrorKind::ConfigurationInvalid),
        None => Ok((None, authority)),
    }
}

fn parse_host_port(host_port: &str) -> Result<(String, u16), SshTransportErrorKind> {
    if let Some(rest) = host_port.strip_prefix('[') {
        let (literal, suffix) = rest
            .split_once(']')
            .ok_or(SshTransportErrorKind::ConfigurationInvalid)?;
        let host = literal
            .parse::<Ipv6Addr>()
            .map_err(|_| SshTransportErrorKind::ConfigurationInvalid)?
            .to_string();
        let port = match suffix {
            "" => 22,
            value if value.starts_with(':') => parse_port(&value[1..])?,
            _ => return Err(SshTransportErrorKind::ConfigurationInvalid),
        };
        return Ok((host, port));
    }
    let (host, port) = match host_port.split_once(':') {
        Some((host, port)) if !port.contains(':') => (host, parse_port(port)?),
        Some(_) => return Err(SshTransportErrorKind::ConfigurationInvalid),
        None => (host_port, 22),
    };
    Ok((parse_host(host, false)?, port))
}

fn parse_port(port: &str) -> Result<u16, SshTransportErrorKind> {
    port.parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(SshTransportErrorKind::ConfigurationInvalid)
}

fn parse_host(host: &str, brackets_allowed: bool) -> Result<String, SshTransportErrorKind> {
    if brackets_allowed
        && let Some(literal) = host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
    {
        return literal
            .parse::<Ipv6Addr>()
            .map(|address| address.to_string())
            .map_err(|_| SshTransportErrorKind::ConfigurationInvalid);
    }
    if host.is_empty() || host.contains(['[', ']']) || !host.is_ascii() {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    if let Ok(address) = host.parse::<IpAddr>() {
        return Ok(address.to_string());
    }
    let valid = host.split('.').all(|label| {
        !label.is_empty()
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            && label
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && label
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
    });
    valid
        .then(|| host.to_ascii_lowercase())
        .ok_or(SshTransportErrorKind::ConfigurationInvalid)
}

enum PathSyntax {
    Url,
    Scp,
}

fn endpoint(
    username: Option<String>,
    host: String,
    port: u16,
    path: &str,
    syntax: PathSyntax,
) -> Result<SshEndpoint, SshTransportErrorKind> {
    if path.is_empty() {
        return Err(SshTransportErrorKind::ConfigurationInvalid);
    }
    let display_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.clone()
    };
    let username_prefix = username
        .as_deref()
        .map(|username| format!("{username}@"))
        .unwrap_or_default();
    let port_suffix = if port != 22 {
        format!(":{port}")
    } else {
        String::new()
    };
    Ok(SshEndpoint {
        authority: SshAuthority { host, port },
        username,
        // Preserve path form and bytes: converting relative SCP to an SSH URL
        // adds a leading slash and selects a different server repository.
        // Equality deliberately rejects cross-form rewrites conservatively.
        connection_url: match syntax {
            PathSyntax::Url => format!("ssh://{username_prefix}{display_host}{port_suffix}/{path}"),
            PathSyntax::Scp => format!("{username_prefix}{display_host}:{path}"),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::transport::SshAuthority;

    #[test]
    fn url_and_scp_endpoints_normalize_host_and_default_port() {
        for url in ["ssh://git@EXAMPLE.com/repo", "git@example.com:repo"] {
            let endpoint = parse_ssh_endpoint(url).unwrap();
            assert_eq!(
                endpoint.authority,
                SshAuthority {
                    host: "example.com".to_owned(),
                    port: 22,
                }
            );
            assert_eq!(endpoint.username.as_deref(), Some("git"));
        }
    }

    #[test]
    fn explicit_port_is_preserved() {
        let endpoint = parse_ssh_endpoint("ssh://git@example.com:2222/repo").unwrap();

        assert_eq!(
            endpoint.authority,
            SshAuthority {
                host: "example.com".to_owned(),
                port: 2222,
            }
        );
    }

    #[test]
    fn equivalent_bracketed_ipv6_literals_have_one_authority() {
        let compressed = parse_ssh_endpoint("ssh://git@[2001:db8::1]/repo").unwrap();
        let expanded =
            parse_ssh_endpoint("git@[2001:0db8:0000:0000:0000:0000:0000:0001]:repo").unwrap();

        assert_eq!(compressed.authority, expanded.authority);
        assert_eq!(expanded.connection_url, "git@[2001:db8::1]:repo");
        assert!(compressed != expanded);
    }

    #[test]
    fn connection_spelling_preserves_repository_path_form() {
        for (input, expected) in [
            ("git@EXAMPLE.com:repo.git", "git@example.com:repo.git"),
            ("git@example.com:/repo.git", "git@example.com:/repo.git"),
            ("git@example.com:~/repo.git", "git@example.com:~/repo.git"),
            ("git@example.com:/~/repo.git", "git@example.com:/~/repo.git"),
            (
                "git@example.com:~user/repo.git",
                "git@example.com:~user/repo.git",
            ),
            (
                "ssh://git@example.com/repo.git",
                "ssh://git@example.com/repo.git",
            ),
            (
                "ssh://git@example.com/~/repo.git",
                "ssh://git@example.com/~/repo.git",
            ),
            (
                "ssh://git@example.com/~user/repo.git",
                "ssh://git@example.com/~user/repo.git",
            ),
        ] {
            assert_eq!(parse_ssh_endpoint(input).unwrap().connection_url, expected);
        }
        let relative = parse_ssh_endpoint("git@example.com:repo.git").unwrap();
        let absolute = parse_ssh_endpoint("ssh://git@example.com/repo.git").unwrap();
        assert!(relative != absolute);
    }

    #[test]
    fn url_delimiters_are_rejected_but_scp_delimiters_are_literal() {
        for suffix in ["?other", "#other", "?", "#"] {
            assert!(matches!(
                parse_ssh_endpoint(&format!("ssh://git@example.com/repo.git{suffix}")),
                Err(SshTransportErrorKind::ConfigurationInvalid)
            ));
            let scp = format!("git@example.com:repo.git{suffix}");
            assert_eq!(parse_ssh_endpoint(&scp).unwrap().connection_url, scp);
        }
    }

    #[test]
    fn path_interpretation_rewrites_are_rejected_in_both_directions() {
        for direction in [SshDirection::Fetch, SshDirection::Push] {
            for (from, to) in [
                ("git@example.com:repo.git", "ssh://git@example.com/repo.git"),
                ("ssh://git@example.com/repo.git", "git@example.com:repo.git"),
                ("git@example.com:~/repo.git", "git@example.com:/repo.git"),
                ("git@example.com:/repo.git", "git@example.com:~/repo.git"),
                // Cross-form rewrites remain conservatively unsupported even
                // when this backend would produce the same service target.
                (
                    "git@example.com:/repo.git",
                    "ssh://git@example.com/repo.git",
                ),
                (
                    "git@example.com:~/repo.git",
                    "ssh://git@example.com/~/repo.git",
                ),
            ] {
                let temp = tempfile::tempdir().unwrap();
                let repository = Repository::init(temp.path()).unwrap();
                repository.remote("origin", from).unwrap();
                let rule = if direction == SshDirection::Fetch {
                    "insteadOf"
                } else {
                    "pushInsteadOf"
                };
                repository
                    .config()
                    .unwrap()
                    .set_str(&format!("url.{to}.{rule}"), from)
                    .unwrap();
                assert!(matches!(
                    configured_remote_endpoint(&repository, "origin", direction),
                    Err(SshTransportErrorKind::ConfigurationInvalid)
                ));
            }
        }
    }

    #[test]
    fn username_less_ssh_url_remains_structurally_valid() {
        let endpoint = parse_ssh_endpoint("ssh://example.com/repo").unwrap();

        assert_eq!(endpoint.username, None);
    }

    #[test]
    fn ambiguous_or_non_ssh_endpoints_are_rejected() {
        for url in [
            "http://example.com/repo",
            "https://example.com/repo",
            r"C:\\repo",
            "file:/repo",
            "ssh://git:secret@example.com/repo",
            "ssh://git@example.com:0/repo",
            "ssh://git@example.com:65536/repo",
            "ssh://git@example.com/",
            "git@example.com:",
            "ssh://git@[2001:db8::1/repo",
            "ssh://git@%65xample.com/repo",
            "ssh://git@example.com/re po",
            "ssh://git@example.com/re\0po",
            "[git@example.com:2222]:repo.git",
        ] {
            assert!(
                matches!(
                    parse_ssh_endpoint(url),
                    Err(SshTransportErrorKind::ConfigurationInvalid)
                ),
                "{url:?}"
            );
        }
    }
}
