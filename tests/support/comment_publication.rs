//! Explicit caller-owned session adapter for existing local-authoring regressions.
use manyhands::repository::{keys::*, *};

struct Unavailable;
impl SessionCredentialProvider for Unavailable {
    fn request_passphrase(&mut self, _: &UnlockRequest) -> PassphraseResponse {
        PassphraseResponse::Unavailable
    }
}
pub trait TestCommentSession {
    fn submit_comment_with_test_session(
        &self,
        request: SubmitCommentRequest,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError>;
    fn submit_comment_with_test_identity(
        &self,
        request: SubmitCommentRequest,
        config: &git2::Config,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError>;
    /// The publication-only retry of `operation_id`, with no approval,
    /// identity or restart.
    fn retry_comment_publication_with_test_session(
        &self,
        root: &std::path::Path,
        operation_id: OperationId,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError>;
}
impl TestCommentSession for RepositoryService {
    fn submit_comment_with_test_session(
        &self,
        request: SubmitCommentRequest,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError> {
        self.submit_comment(request.into(), &mut SessionCredentials::new(Unavailable))
    }
    fn submit_comment_with_test_identity(
        &self,
        request: SubmitCommentRequest,
        config: &git2::Config,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError> {
        self.submit_comment_with_identity_config_for_testing(
            request.into(),
            config,
            &mut SessionCredentials::new(Unavailable),
        )
    }
    fn retry_comment_publication_with_test_session(
        &self,
        root: &std::path::Path,
        operation_id: OperationId,
    ) -> Result<CommentSubmissionOutcome, CommentSubmissionError> {
        self.retry_comment_publication(
            RetryCommentPublicationRequest {
                root: root.to_owned(),
                operation_id,
                approval: None,
                confirmed_identity: None,
                restart: false,
            },
            &mut SessionCredentials::new(Unavailable),
        )
    }
}
