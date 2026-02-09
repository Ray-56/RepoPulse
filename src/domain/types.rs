use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoId {
    owner: String,
    name: String,
}

impl RepoId {
    pub fn parse(s: &str) -> Result<Self, RepoIdError> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(RepoIdError::InvalidFormat(s.to_string()));
        }
        Ok(Self {
            owner: parts[0].to_string(),
            name: parts[1].to_string(),
        })
    }

    pub fn as_str(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }
}

#[derive(Clone, Debug, thiserror::Error)]
pub enum RepoIdError {
    #[error("invalid repo id format: {0} (expected owner/repo)")]
    InvalidFormat(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    GitHub,
    Npm,
    WhatsAppWeb,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::GitHub => write!(f, "github"),
            Source::Npm => write!(f, "npm"),
            Source::WhatsAppWeb => write!(f, "whatsapp-web"),
        }
    }
}
