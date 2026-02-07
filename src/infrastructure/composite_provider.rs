use async_trait::async_trait;

use crate::application::{AppError, AppResult, WatchProvider};
use crate::domain::{WatchKind, WatchTarget};

pub struct CompositeWatchProvider {
    github_release: Box<dyn WatchProvider>,
    github_branch: Box<dyn WatchProvider>,
    npm_latest: Box<dyn WatchProvider>,
}

impl CompositeWatchProvider {
    pub fn new(
        github_release: Box<dyn WatchProvider>,
        github_branch: Box<dyn WatchProvider>,
        npm_latest: Box<dyn WatchProvider>,
    ) -> Self {
        Self {
            github_release,
            github_branch,
            npm_latest,
        }
    }
}

#[async_trait]
impl WatchProvider for CompositeWatchProvider {
    async fn check(&self, target: &WatchTarget) -> AppResult<Option<crate::domain::Event>> {
        match &target.kind {
            WatchKind::GitHubRelease { .. } => self.github_release.check(target).await,
            WatchKind::GitHubBranch { .. } => self.github_branch.check(target).await,
            WatchKind::NpmLatest { .. } => self.npm_latest.check(target).await,
            WatchKind::WhatsappWebVersion { .. } => Err(AppError::Provider(
                "WhatsAppWebVersion provider not implemented".into(),
            )),
        }
    }
}
