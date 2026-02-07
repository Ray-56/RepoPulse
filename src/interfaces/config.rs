use serde::Deserialize;

use crate::domain::{RepoId, WatchKind, WatchTarget};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub poll_interval_seconds: u64,
    pub cooldown_seconds: Option<u64>,
    pub targets: Vec<TargetCfg>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TargetCfg {
    #[serde(rename = "github_release")]
    GitHubRelease {
        repo: String,
        id: Option<String>,
        enabled: Option<bool>,
        labels: Option<Vec<String>>,
    },

    #[serde(rename = "github_branch")]
    GitHubBranch {
        repo: String,
        branch: String,
        id: Option<String>,
        enabled: Option<bool>,
        labels: Option<Vec<String>>,
    },

    #[serde(rename = "npm_latest")]
    NpmLatest {
        package: String,
        id: Option<String>,
        enabled: Option<bool>,
        labels: Option<Vec<String>>,
    },
}

impl Config {
    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let raw = expand_env(&raw);
        let cfg: Config = serde_yaml::from_str(&raw)?;
        Ok(cfg)
    }

    pub fn to_watch_targets(&self) -> anyhow::Result<Vec<WatchTarget>> {
        let mut out = Vec::new();

        for t in &self.targets {
            match t {
                TargetCfg::GitHubRelease {
                    repo,
                    id,
                    enabled,
                    labels,
                } => {
                    let repo_id = RepoId::parse(repo)?;
                    let target_id = id
                        .clone()
                        .unwrap_or_else(|| format!("github:{}:release", repo));
                    out.push(WatchTarget {
                        id: target_id,
                        enabled: enabled.unwrap_or(true),
                        labels: labels.clone().unwrap_or_default(),
                        kind: WatchKind::GitHubRelease { repo: repo_id },
                    })
                }
                TargetCfg::GitHubBranch {
                    repo,
                    branch,
                    id,
                    enabled,
                    labels,
                } => {
                    let repo_id = RepoId::parse(repo)?;
                    let target_id = id
                        .clone()
                        .unwrap_or_else(|| format!("github:{}:branch", repo));
                    out.push(WatchTarget {
                        id: target_id,
                        enabled: enabled.unwrap_or(true),
                        labels: labels.clone().unwrap_or_default(),
                        kind: WatchKind::GitHubBranch {
                            repo: repo_id,
                            branch: branch.clone(),
                        },
                    })
                }
                TargetCfg::NpmLatest {
                    package,
                    id,
                    enabled,
                    labels,
                } => {
                    let target_id = id
                        .clone()
                        .unwrap_or_else(|| format!("npm:{}:latest", package));
                    out.push(WatchTarget {
                        id: target_id,
                        enabled: enabled.unwrap_or(true),
                        labels: labels.clone().unwrap_or_default(),
                        kind: WatchKind::NpmLatest {
                            package: package.clone(),
                        },
                    })
                }
            }
        }
        Ok(out)
    }
}

/// very small ${VAR} expansion to keep config simple
fn expand_env(s: &str) -> String {
    let mut out = s.to_string();
    for (k, v) in std::env::vars() {
        out = out.replace(&format!("${{{}}}", k), &v);
    }
    out
}
