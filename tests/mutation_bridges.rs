//! The bridges: the mutations the baseline lacked, bound to the boundary.
//! `repo identity-set` is the first. Of `host approve` and `host replace`
//! this target holds what needs no server; the rest of them is in
//! `tests/mutation_remote`.
//!
//! Every test of this target begins with `git_home::isolated` or
//! `git_home::own`, so that none reads or writes the Git configuration
//! of whoever runs it: where libgit2 looks outside a repository is
//! process-wide; see `support::git_home`.

mod support;

#[path = "mutation_bridges/conformance.rs"]
mod conformance;

#[path = "mutation_bridges/host.rs"]
mod host;

#[path = "mutation_bridges/identity.rs"]
mod identity;
