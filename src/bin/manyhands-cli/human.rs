use super::render::Human;
use manyhands::repository::NewIdDto;

pub fn new_id(data: &NewIdDto) -> Human {
    Human {
        stdout: format!("{}\n", data.id),
        notes: Vec::new(),
    }
}

#[cfg(test)]
#[path = "human_tests.rs"]
mod tests;
