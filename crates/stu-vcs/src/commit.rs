use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// Hash du commit parent (`None` pour le premier commit).
    pub parent: Option<Hash>,
    /// Hash du Tree associé à ce commit.
    pub tree: Hash,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

impl Commit {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("Commit est toujours sérialisable")
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}
