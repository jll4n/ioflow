use std::fs;
use std::path::{Path, PathBuf};

use crate::error::VcsError;
use crate::hash::{hash_bytes, Hash};

/// Store contenu-adressé : chaque objet est stocké sous
/// `.ioflow/objects/<2 premiers chars du hash>/<62 chars restants>`.
pub struct ObjectStore {
    dir: PathBuf,
}

impl ObjectStore {
    pub fn new(ioflow_dir: &Path) -> Self {
        Self {
            dir: ioflow_dir.join("objects"),
        }
    }

    pub fn init(&self) -> Result<(), VcsError> {
        fs::create_dir_all(&self.dir)?;
        Ok(())
    }

    /// Stocke `data` et retourne son hash. Idempotent (ne réécrit pas si déjà présent).
    pub fn write(&self, data: &[u8]) -> Result<Hash, VcsError> {
        let hash = hash_bytes(data);
        let (prefix, suffix) = hash.split_at(2);
        let dir = self.dir.join(prefix);
        fs::create_dir_all(&dir)?;
        let path = dir.join(suffix);
        if !path.exists() {
            fs::write(path, data)?;
        }
        Ok(hash)
    }

    /// Lit le contenu d'un objet par son hash.
    pub fn read(&self, hash: &str) -> Result<Vec<u8>, VcsError> {
        if hash.len() < 3 {
            return Err(VcsError::ObjectNotFound(hash.to_string()));
        }
        let (prefix, suffix) = hash.split_at(2);
        let path = self.dir.join(prefix).join(suffix);
        fs::read(&path).map_err(|_| VcsError::ObjectNotFound(hash.to_string()))
    }
}
