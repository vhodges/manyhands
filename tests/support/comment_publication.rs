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
}
