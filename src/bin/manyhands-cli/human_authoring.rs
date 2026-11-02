use super::{
    human,
    render::{Human, field},
};
use manyhands::repository::{PathObservationDto, RelationshipCheckDto};

pub fn path_observation(data: &PathObservationDto) -> Human {
    Human {
        stdout: format!(
            "path: {}\nexists: {}\nobservation: {}\ncontext: {} ({})\n",
            field(&data.path),
            if data.exists { "yes" } else { "no" },
            field(&data.observation),
            field(&data.context.worktree),
            field(data.context.branch.as_deref().unwrap_or("-")),
        ),
        notes: Vec::new(),
    }
}

fn ids(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_owned()
    } else {
        values
            .iter()
            .map(|value| field(value))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn relationship_check(data: &RelationshipCheckDto) -> Human {
    let mut output = Human {
        stdout: format!(
            "deps: {}\nparent: {}\nunresolved: {}\n",
            ids(&data.deps),
            field(data.parent.as_deref().unwrap_or("-")),
            ids(&data.unresolved)
        ),
        notes: Vec::new(),
    };
    if let Some(rejection) = &data.rejection {
        output.stdout.push_str(&format!(
            "rejected: {}: {}\n",
            field(rejection.code.as_str()),
            ids(&rejection.ids)
        ));
    }
    human::incomplete(&mut output, data.complete);
    human::index_note(&mut output, data.index.state.as_str());
    output
}

#[cfg(test)]
#[path = "human_authoring_tests.rs"]
mod tests;
