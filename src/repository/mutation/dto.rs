//! The data shapes of the mutation boundary's JSON v1 results.
//!
//! As with the read DTOs, these are the only types here that serialize.
//! Every field is always present, and an absent value is `null`.

use serde::{Serialize, Serializer};

use crate::{
    repository::OperationFamily,
    results::{Effects, Outcome, ResultCode, contract_enum},
};

contract_enum!(
    /// Where a request stands. `accepted` is a request that was begun and
    /// whose end is not recorded; `finished` is one whose result is stored
    /// and is returned to every retry.
    RequestState {
        Accepted => "accepted",
        Finished => "finished",
    }
);

/// A domain operation a request began, and the journal that holds it:
/// `operation show` reads it there.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RequestOperationDto {
    pub family: OperationFamily,
    pub operation_id: String,
}

/// What a finished request returned, as its envelope carried it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RequestResultDto {
    pub outcome: Outcome,
    pub code: ResultCode,
    /// The code's fixed message.
    pub message: String,
    pub effects: Effects,
    /// The envelope's `data`, whose shape is the command's own. Null when
    /// the envelope carried none.
    pub data: Option<serde_json::Value>,
}

/// What the records hold for one request ID. Nothing is observed again:
/// this is not what Git, the file system or a journal holds now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RequestDto {
    pub request_id: String,
    pub state: RequestState,
    /// The command the request was made with, as its envelope names it.
    pub command: String,
    pub accepted_at: String,
    /// Null until the request is finished.
    pub finished_at: Option<String>,
    /// The request's operations, in the order it runs them.
    pub operations: Vec<RequestOperationDto>,
    /// Set exactly when `state` is `finished`.
    pub result: Option<RequestResultDto>,
}
