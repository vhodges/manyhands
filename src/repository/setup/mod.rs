//! A repository's pending setup: onboarding or a setting change put on a
//! branch for a pull request (Wave 06 Cycle 02).
// The record is consumed by the setup operations of the later tasks.
#![cfg_attr(not(test), allow(dead_code))]
// The transport's typed error is large by design, as in `transport` and `remote`.
#![allow(clippy::result_large_err)]

pub(crate) mod channel;
pub(crate) mod commit;
pub(crate) mod complete;
mod lock;
pub(crate) mod publish;
pub(crate) mod record;

// The bindings of the later tasks name the trait through here.
#[allow(unused_imports)]
pub(crate) use channel::SetupChannel;
