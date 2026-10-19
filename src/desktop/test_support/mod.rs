//! Test isolation and real-repository builders for desktop tests.
//!
//! Every desktop test that touches Git calls `git_home::isolated()` first,
//! builds its repositories through the library and keeps its data in
//! temporary directories.

pub(crate) mod fixtures;

// The library tests' own isolation, shared rather than copied. Some of
// its helpers (`own`, the file accessors) serve only library tests.
#[allow(dead_code)]
#[path = "../../../tests/support/git_home.rs"]
pub(crate) mod git_home;
