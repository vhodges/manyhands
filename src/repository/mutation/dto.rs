//! The data shapes of the mutation boundary's JSON v1 results.
//!
//! As with the read DTOs, these are the only types here that serialize.
//! Every field is always present, and an absent value is `null`.

use serde::{Serialize, Serializer};

use crate::results::contract_enum;

contract_enum!(
    /// Where a request stands. `accepted` is a request that was begun and
    /// whose end is not recorded; `finished` is one whose result is stored
    /// and is returned to every retry.
    RequestState {
        Accepted => "accepted",
        Finished => "finished",
    }
);
