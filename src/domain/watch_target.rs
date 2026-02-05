use super::{RepoId, Source};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatchTarget {
    pub id: String, // stable id, e.g. "github:owner/repo:release"
    pub enabled: bool,
    pub labels: Vec<String>,
    pub kind: WatchKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchKind {
    GitHubRelease { repo: RepoId },
    GitHubBranch { repo: RepoId, branch: String },
    NpmLatest { package: String },
    WhatsappWebVersion {}, // v1 reserved
}

impl WatchKind {
    pub fn source(&self) -> Source {
        match self {
            WatchKind::GitHubRelease { .. } => Source::GitHub,
            WatchKind::GitHubBranch { .. } => Source::GitHub,
            WatchKind::NpmLatest { .. } => Source::Npm,
            WatchKind::WhatsappWebVersion { .. } => Source::WhatsappWeb,
        }
    }

    pub fn subject(&self) -> String {
        match self {
            WatchKind::GitHubRelease { repo } => repo.as_str(),
            WatchKind::GitHubBranch { repo, branch } => format!("{}#{}", repo.as_str(), branch),
            WatchKind::NpmLatest { package } => package.clone(),
            WatchKind::WhatsappWebVersion { .. } => "whatsapp-web".to_string(),
        }
    }
}
