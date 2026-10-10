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

/// The `data` of a mutation's envelope: the shape of the command's family.
/// It may accompany any outcome and never holds a body or a source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum MutationDataDto {
    Ticket(TicketMutationDto),
}

/// What `ticket create` and `ticket save` report about the ticket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TicketMutationDto {
    pub id: String,
    /// The ticket's file, relative to its context, with forward slashes.
    pub path: String,
    /// The short code the ticket's file holds once the request has
    /// written or found it. Null until then, and for a ticket without one.
    pub slug: Option<String>,
    /// The dependencies and parent this request sets that nothing the
    /// index holds has the ID of, in ID order, each once. They are
    /// accepted.
    pub unresolved: Vec<String>,
    /// Empty unless the request was rejected for its relationships. Then,
    /// in ID order: for `relationship_cycle` the tickets of the cycle, the
    /// ticket itself included, and for `invalid_relationship` the IDs that
    /// are a document's or a comment's.
    pub rejected: Vec<String>,
}

impl TicketMutationDto {
    /// The data as a finished record stored it.
    pub(crate) fn from_stored(stored: &serde_json::Value) -> Option<Self> {
        let strings = |key: &str| -> Option<Vec<String>> {
            stored
                .get(key)?
                .as_array()?
                .iter()
                .map(|value| value.as_str().map(str::to_owned))
                .collect()
        };
        Some(Self {
            id: stored.get("id")?.as_str()?.to_owned(),
            path: stored.get("path")?.as_str()?.to_owned(),
            slug: match stored.get("slug")? {
                serde_json::Value::Null => None,
                slug => Some(slug.as_str()?.to_owned()),
            },
            unresolved: strings("unresolved")?,
            rejected: strings("rejected")?,
        })
    }
}

impl MutationDataDto {
    /// The data a finished record of `command` stored, in the shape of
    /// that command's family. `None` for data that is not of that shape.
    pub(crate) fn from_stored(command: &str, stored: &serde_json::Value) -> Option<Self> {
        use super::bind::ticket::{TICKET_CREATE, TICKET_SAVE};

        if command == TICKET_CREATE || command == TICKET_SAVE {
            TicketMutationDto::from_stored(stored).map(Self::Ticket)
        } else {
            None
        }
    }
}
