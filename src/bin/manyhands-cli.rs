// `ReadError` carries its whole scope by value, as the result contract has it.
#![allow(clippy::result_large_err)]

use std::ffi::OsString;

use manyhands::{
    repository::{ReadError, RepositoryService},
    results::ResultCode,
};

#[path = "manyhands-cli/catalog.rs"]
mod catalog;
#[path = "manyhands-cli/dispatch.rs"]
mod dispatch;
#[path = "manyhands-cli/human.rs"]
mod human;
#[path = "manyhands-cli/parse.rs"]
mod parse;
#[path = "manyhands-cli/render.rs"]
mod render;
#[path = "manyhands-cli/startup.rs"]
mod startup;

fn main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    let invocation = parse::parse(args);
    render::write_and_exit(startup::run(invocation, &mut RealStartup));
}

struct RealStartup;

impl startup::Startup for RealStartup {
    fn start_transport(&mut self) -> Result<(), ()> {
        // SAFETY: only argument collection and parsing have run; they start no
        // threads or native Git activity. Service opening follows this call.
        unsafe { manyhands::runtime::initialize_git_transport_before_threads() }.map_err(|_| ())
    }

    fn open_service(&mut self) -> Result<RepositoryService, ReadError> {
        let cwd = std::env::current_dir().map_err(|_| ReadError::new(ResultCode::InternalError))?;
        match startup::data_directory(std::env::var_os("MANYHANDS_DATA_DIR"), &cwd)? {
            Some(path) => RepositoryService::open_at(&path),
            None => RepositoryService::open_default(),
        }
        .map_err(ReadError::from)
    }
}
