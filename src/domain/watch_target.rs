use serde::{Deserialize, Serialize};

use super::{RepoId, Source};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchTarget {
    pub id: String, // stable id, e.g. "github:owner/repo:release"
    pub enabled: bool,
    pub labels: Vec<String>,
    pub kind: WatchKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchKind {
    GitHubRelease { repo: RepoId },
    GitHubBranch { repo: RepoId, branch: String },
    NpmLatest { package: String },
    WhatsAppWebVersion {}, // v1 reserved
}

impl WatchKind {
    pub fn source(&self) -> Source {
        match self {
            WatchKind::GitHubRelease { .. } => Source::GitHub,
            WatchKind::GitHubBranch { .. } => Source::GitHub,
            WatchKind::NpmLatest { .. } => Source::Npm,
            WatchKind::WhatsAppWebVersion { .. } => Source::WhatsAppWeb,
        }
    }

    pub fn subject(&self) -> String {
        match self {
            WatchKind::GitHubRelease { repo } => repo.as_str(),
            WatchKind::GitHubBranch { repo, branch } => format!("{}#{}", repo.as_str(), branch),
            WatchKind::NpmLatest { package } => package.clone(),
            WatchKind::WhatsAppWebVersion { .. } => "whatsapp-web".to_string(),
        }
    }
}
