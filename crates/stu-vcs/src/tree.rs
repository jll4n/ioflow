use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::hash::Hash;

/// Snapshot de tous les fichiers d'une archive STU à un instant donné.
/// `BTreeMap` pour un ordre déterministe (important pour le hash du tree).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Tree {
    pub files: BTreeMap<String, Hash>,
}

impl Tree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: String, hash: Hash) {
        self.files.insert(path, hash);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("Tree est toujours sérialisable")
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}
