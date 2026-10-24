use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use manyhands::{
    repository::{ReadError, RepositoryService},
    results::{Envelope, ResultCode, Scope},
};
use serde::Serialize;

use super::{
    catalog::{self, Availability, CommandSpec, RepoRule},
    dispatch,
    parse::{self, Action, HelpTarget, Invocation},
    render::{self, Human, Output},
    terminal::Terminal,
};

pub trait Startup {
    fn open_terminal(&mut self) -> Option<Box<dyn Terminal>>;
    fn start_transport(&mut self) -> Result<(), ()>;
    fn open_service(&mut self) -> Result<RepositoryService, ReadError>;
}

pub fn data_directory(var: Option<OsString>, cwd: &Path) -> Result<Option<PathBuf>, ReadError> {
    let Some(var) = var else { return Ok(None) };
    if var.is_empty() {
        return Err(ReadError::invalid_path());
    }
    let path = PathBuf::from(var);
    Ok(Some(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }))
}

pub fn run(invocation: Invocation, startup: &mut dyn Startup) -> Output {
    let json = invocation.json;
    match invocation.action {
        Action::Help(target) => help_output(json, target),
        Action::Version => {
            let envelope = Envelope::read_success(
                "cli version",
                Scope::default(),
                VersionDto {
                    name: "manyhands-cli",
                    version: env!("CARGO_PKG_VERSION"),
                },
            );
            render::envelope_output(json, &envelope, |data| Human {
                stdout: format!("{} {}\n", data.name, data.version),
                notes: Vec::new(),
            })
        }
        Action::Usage { command, detail } => render::failure_output(
            json,
            &Envelope::failure(
                command
                    .map(command_name)
                    .unwrap_or_else(|| "cli usage".into()),
                Scope::default(),
                render::USAGE_CODE,
                Vec::new(),
            ),
            Some(detail),
        ),
        Action::Unavailable { command } => {
            let name = command_name(command);
            let extra = match command.availability {
                Availability::Planned(cycle) => Some(format!("{name} arrives with {cycle}.")),
                Availability::Available(_) => None,
            };
            render::failure_output(
                json,
                &Envelope::failure(name, Scope::default(), render::UNAVAILABLE_CODE, Vec::new()),
                extra,
            )
        }
        Action::Run { command, args } => {
            // Read commands may have no clap arguments at all. Check their kind
            // before looking up the prompting-only flag.
            let terminal = if command.prompts() && !json && !args.get_flag("non-interactive") {
                startup.open_terminal()
            } else {
                None
            };
            let interactive = terminal.is_some();
            if let Err(detail) = parse::mode_rules(command, &args, interactive) {
                // IDs are parsed only after service opening, in dispatch.
                // Every early refusal retains C1's null request ID.
                return render::failure_output(
                    json,
                    &Envelope::failure(
                        command_name(command),
                        Scope::default(),
                        render::USAGE_CODE,
                        Vec::new(),
                    ),
                    Some(detail),
                );
            }
            if startup.start_transport().is_err() {
                return render::failure_output(
                    json,
                    &ReadError::new(ResultCode::InternalError).to_envelope(command_name(command)),
                    Some("Git transport initialization failed".into()),
                );
            }
            match startup.open_service() {
                Ok(service) => dispatch::dispatch(
                    command,
                    &args,
                    &mut crate::mutate::Session {
                        json,
                        interactive,
                        terminal,
                    },
                    &service,
                ),
                Err(error) => {
                    render::failure_output(json, &error.to_envelope(command_name(command)), None)
                }
            }
        }
    }
}

fn command_name(command: &CommandSpec) -> String {
    format!("{} {}", command.resource, command.verb)
}

#[derive(Serialize)]
struct HelpDto {
    commands: Vec<HelpCommandDto>,
}

#[derive(Serialize)]
struct HelpCommandDto {
    command: String,
    available: bool,
    delivered_by: &'static str,
    summary: &'static str,
    repository: Option<&'static str>,
    options: Vec<HelpOptionDto>,
}

#[derive(Serialize)]
struct HelpOptionDto {
    name: &'static str,
    value: Option<&'static str>,
    required: bool,
    summary: &'static str,
}

#[derive(Serialize)]
struct VersionDto {
    name: &'static str,
    version: &'static str,
}

fn help_output(json: bool, target: HelpTarget) -> Output {
    let commands = match &target {
        HelpTarget::Root => catalog::COMMANDS.iter().collect(),
        HelpTarget::Resource(resource) => catalog::resource_commands(resource),
        HelpTarget::Command(command) => vec![*command],
    };
    let data = HelpDto {
        commands: commands
            .iter()
            .map(|spec| HelpCommandDto {
                command: command_name(spec),
                available: matches!(spec.availability, Availability::Available(_)),
                delivered_by: spec.delivered_by(),
                summary: spec.summary,
                repository: spec.repo.map(|rule| match rule {
                    RepoRule::Required => catalog::REPOSITORY_VALUES[0],
                    RepoRule::Refused => catalog::REPOSITORY_VALUES[1],
                }),
                options: spec
                    .options
                    .iter()
                    .map(|option| HelpOptionDto {
                        name: option.name,
                        value: option.value,
                        required: option.required,
                        summary: option.summary,
                    })
                    .collect(),
            })
            .collect(),
    };
    let envelope = Envelope::read_success("cli help", Scope::default(), data);
    render::envelope_output(json, &envelope, |_| Human {
        stdout: text_help(&target, &commands),
        notes: Vec::new(),
    })
}

fn text_help(target: &HelpTarget, commands: &[&CommandSpec]) -> String {
    if let HelpTarget::Command(command) = target
        && let Availability::Planned(cycle) = command.availability
    {
        return format!(
            "{}\n{}\nArrives with {cycle}.\n",
            command_name(command),
            command.summary
        );
    }
    let mut tree = parse::command();
    // Build first so subcommand help inherits global options and full bin names.
    tree.build();
    let selected = match target {
        HelpTarget::Root => &mut tree,
        HelpTarget::Resource(resource) => tree
            .find_subcommand_mut(resource)
            .expect("catalog resource"),
        HelpTarget::Command(command) => tree
            .find_subcommand_mut(command.resource)
            .expect("catalog resource")
            .find_subcommand_mut(command.verb)
            .expect("catalog verb"),
    };
    let mut selected = selected.clone();
    if !matches!(target, HelpTarget::Command(_)) {
        // Clap's default root help stops at resources. Supply its catalog-derived
        // epilogue to expose every verb and delivering Cycle at this level.
        let mut catalog_help = String::from("Command catalog:\n");
        for command in commands {
            catalog_help.push_str(&format!("  {}  {}", command_name(command), command.summary));
            match command.availability {
                Availability::Available(_) => catalog_help.push_str(" Available now."),
                Availability::Planned(cycle) => {
                    catalog_help.push_str(&format!(" Arrives with {cycle}."))
                }
            }
            catalog_help.push('\n');
        }
        catalog_help.push_str("\nMANYHANDS_DATA_DIR: advanced/testing data-directory override; relative paths use the invocation directory.");
        selected = selected.after_help(catalog_help);
    }
    let text = selected.render_help().to_string();
    format!("{}\n", text.trim_end_matches('\n'))
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
