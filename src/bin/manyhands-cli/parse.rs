use std::ffi::OsString;

use clap::{Arg, ArgAction, ArgMatches, Command};

use super::catalog::{self, Availability, CommandSpec, Kind};

#[derive(Debug)]
pub struct Invocation {
    pub json: bool,
    pub action: Action,
}

#[derive(Debug)]
pub enum HelpTarget {
    Root,
    Resource(&'static str),
    Command(&'static CommandSpec),
}

#[derive(Debug)]
pub enum Action {
    Help(HelpTarget),
    Version,
    Usage {
        command: Option<&'static CommandSpec>,
        detail: String,
    },
    Unavailable {
        command: &'static CommandSpec,
    },
    Run {
        command: &'static CommandSpec,
        args: clap::ArgMatches,
    },
}

/// Build the shared parser/help tree from the catalog.
pub fn command() -> Command {
    let mut root = base_command("manyhands-cli")
        .bin_name("manyhands-cli")
        .args_override_self(true)
        .subcommand_required(false);
    for (name, summary) in [
        ("json", "Write a JSON result."),
        ("non-interactive", "Run without prompts (no effect in C1)."),
        ("help", "Show help."),
        ("version", "Show the version."),
    ] {
        root = root.arg(
            Arg::new(name)
                .long(name)
                .action(ArgAction::SetTrue)
                .global(true)
                .help(summary),
        );
    }
    let mut resources = Vec::new();
    for spec in catalog::COMMANDS {
        if resources.contains(&spec.resource) {
            continue;
        }
        resources.push(spec.resource);
        let mut resource = base_command(spec.resource).subcommand_required(false);
        for spec in catalog::resource_commands(spec.resource) {
            let mut verb = base_command(spec.verb).about(spec.summary);
            match spec.availability {
                Availability::Available(_) => {
                    for option in spec.options {
                        let name = option.name.strip_prefix("--").expect("catalog long option");
                        let mut arg = Arg::new(name).long(name).help(option.summary);
                        if let Some(value) = option.value {
                            arg = arg.value_name(value).action(ArgAction::Set);
                        } else {
                            arg = arg.action(ArgAction::SetTrue);
                        }
                        arg = match name {
                            "closure" => arg
                                .value_parser(["open", "closed", "all"])
                                .default_value("all"),
                            "direction" => arg
                                .value_parser(["down", "up", "both"])
                                .default_value("down"),
                            "depth" => arg.value_parser(clap::value_parser!(u32)),
                            _ => arg,
                        };
                        // Required inputs are validated only after help/version.
                        verb = verb.arg(arg);
                    }
                }
                Availability::Planned(_) => {
                    verb = verb.arg(
                        Arg::new("rest")
                            .num_args(0..)
                            .trailing_var_arg(true)
                            .allow_hyphen_values(true),
                    );
                }
            }
            resource = resource.subcommand(verb);
        }
        root = root.subcommand(resource);
    }
    root
}

fn base_command(name: &'static str) -> Command {
    Command::new(name)
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
}

pub fn parse(args: Vec<OsString>) -> Invocation {
    let raw_json = json_requested(&args);
    let raw_help = help_requested(&args);
    let matches = match command().try_get_matches_from(&args) {
        Ok(matches) => matches,
        Err(error) => {
            let target = raw_target(&args);
            let action = if raw_help {
                Action::Help(target)
            } else {
                Action::Usage {
                    command: match target {
                        HelpTarget::Command(command) => Some(command),
                        _ => None,
                    },
                    detail: error.render().to_string(),
                }
            };
            return Invocation {
                json: raw_json,
                action,
            };
        }
    };
    let json = matches.get_flag("json") || raw_json;
    let resource = matches.subcommand();
    let selected = resource.and_then(|(resource, matches)| {
        matches.subcommand().map(|(verb, args)| {
            (
                catalog::find(resource, verb).expect("command built from catalog"),
                args,
            )
        })
    });
    let action = if matches.get_flag("help") || raw_help {
        Action::Help(match selected {
            Some((command, _)) => HelpTarget::Command(command),
            None => match resource {
                Some((resource, _)) => {
                    HelpTarget::Resource(catalog::resource_commands(resource)[0].resource)
                }
                None => HelpTarget::Root,
            },
        })
    } else if matches.get_flag("version") {
        Action::Version
    } else if let Some((resource, _)) = resource {
        if let Some((command, args)) = selected {
            match command.availability {
                Availability::Planned(_) => Action::Unavailable { command },
                Availability::Available(_) => match validate(command, args) {
                    Ok(()) => Action::Run {
                        command,
                        args: args.clone(),
                    },
                    Err(detail) => Action::Usage {
                        command: Some(command),
                        detail,
                    },
                },
            }
        } else {
            Action::Help(HelpTarget::Resource(
                catalog::resource_commands(resource)[0].resource,
            ))
        }
    } else {
        Action::Help(HelpTarget::Root)
    };
    Invocation { json, action }
}

fn validate(command: &CommandSpec, args: &ArgMatches) -> Result<(), String> {
    for option in command.options.iter().filter(|option| option.required) {
        let name = option.name.strip_prefix("--").expect("catalog long option");
        if !args.contains_id(name) {
            return Err(format!("{} is required.", option.name));
        }
    }
    match (command.resource, command.verb) {
        ("document" | "ticket", "show") => {
            let id = args.contains_id("id");
            let path = args.contains_id("path");
            if id == path {
                return Err("Exactly one of --id or --path is required.".to_owned());
            }
            if args.contains_id("context") && !path {
                return Err("--context requires --path.".to_owned());
            }
        }
        ("ticket", "list") if args.get_flag("ready") && args.get_flag("blocked") => {
            return Err("--ready conflicts with --blocked.".to_owned());
        }
        ("host", "inspect") => {
            authority(
                args.get_one::<String>("authority")
                    .expect("required authority"),
            )?;
        }
        _ => {}
    }
    Ok(())
}

/// Rules checked by startup after the mode is known. Errors hold usage detail
/// for human stderr.
// Startup wires in the mode decision in Task 2.
#[cfg_attr(not(test), allow(dead_code))]
pub fn mode_rules(spec: &CommandSpec, args: &ArgMatches, interactive: bool) -> Result<(), String> {
    if !matches!(spec.kind, Kind::Mutation { .. }) {
        return Ok(());
    }
    let prepare = args.get_flag("prepare");
    let request_id = args.contains_id("request-id");
    if prepare && request_id {
        return Err("--prepare conflicts with --request-id.".to_owned());
    }
    if prepare && args.contains_id("confirm") {
        return Err("--prepare conflicts with --confirm.".to_owned());
    }
    if !interactive && !prepare && !request_id {
        return Err("--request-id is required for a non-interactive mutation.".to_owned());
    }
    Ok(())
}

fn raw_target(args: &[OsString]) -> HelpTarget {
    let mut words = args
        .iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .filter(|arg| !arg.as_encoded_bytes().starts_with(b"-"));
    let Some(resource) = words.next().and_then(|word| word.to_str()) else {
        return HelpTarget::Root;
    };
    if let Some(command) = words
        .next()
        .and_then(|word| word.to_str())
        .and_then(|verb| catalog::find(resource, verb))
    {
        return HelpTarget::Command(command);
    }
    match catalog::resource_commands(resource).first() {
        Some(command) => HelpTarget::Resource(command.resource),
        None => HelpTarget::Root,
    }
}

pub fn json_requested(args: &[OsString]) -> bool {
    flag_requested(args, "--json")
}

pub fn help_requested(args: &[OsString]) -> bool {
    flag_requested(args, "--help")
}

fn flag_requested(args: &[OsString], flag: &str) -> bool {
    args.iter()
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == flag)
}

pub fn authority(text: &str) -> Result<(String, u16), String> {
    let invalid = || {
        "--authority must be host[:port] or [IPv6][:port], with a port from 1 to 65535.".to_owned()
    };
    let (host, port) = if let Some(bracketed) = text.strip_prefix('[') {
        let (host, suffix) = bracketed.split_once(']').ok_or_else(invalid)?;
        host.parse::<std::net::Ipv6Addr>().map_err(|_| invalid())?;
        let port = if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':').ok_or_else(invalid)?)
        };
        (host, port)
    } else {
        let (host, port) = match text.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (text, None),
        };
        if host.is_empty()
            || host.contains(['[', ']', '@', '/'])
            || host.chars().any(char::is_whitespace)
        {
            return Err(invalid());
        }
        (host, port)
    };
    let port = match port {
        Some(text) => {
            if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(invalid());
            }
            let port = text.parse::<u16>().map_err(|_| invalid())?;
            if port == 0 {
                return Err(invalid());
            }
            port
        }
        None => 22,
    };
    Ok((host.to_owned(), port))
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
