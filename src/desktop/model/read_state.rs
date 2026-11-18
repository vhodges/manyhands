//! A page's read state, without list completeness or index metadata.

use manyhands::results::ResultCode;

use crate::desktop::services::ReadResult;

pub(crate) enum Read<T> {
    /// No selection or read requested yet.
    None,
    Loading,
    Ready(T),
    Busy,
    Failed(ResultCode),
}

impl<T> Read<T> {
    pub(crate) fn from_result<U>(result: ReadResult<U>, f: impl FnOnce(U) -> T) -> Self {
        match result {
            ReadResult::Ok(value) => Self::Ready(f(value)),
            ReadResult::Busy => Self::Busy,
            ReadResult::Failed(failure) => Self::Failed(failure.code),
        }
    }
}
