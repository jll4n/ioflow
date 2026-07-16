use std::fs;
use std::path::{Path, PathBuf};

use crate::commit::Commit;
use crate::error::VcsError;
use crate::hash::Hash;
use crate::objects::ObjectStore;
use crate::tree::Tree;

pub struct Repo {
    /// Répertoire contenant `.ioflow/` (racine du projet).
    pub root: PathBuf,
    /// Chemin vers `.ioflow/`.
    pub ioflow: PathBuf,
    pub objects: ObjectStore,
}

impl Repo {
    /// Initialise un nouveau dépôt dans `path`.
    pub fn init(path: &Path) -> Result<Self, VcsError> {
        let ioflow = path.join(".ioflow");
        if ioflow.exists() {
            return Err(VcsError::AlreadyInitialized(ioflow.display().to_string()));
        }

        fs::create_dir_all(ioflow.join("refs").join("heads"))?;
        fs::write(ioflow.join("HEAD"), "ref: refs/heads/main\n")?;
        fs::write(ioflow.join("config.toml"), "[user]\nname = \"\"\n")?;

        let objects = ObjectStore::new(&ioflow);
        objects.init()?;

        Ok(Self {
            root: path.to_owned(),
            ioflow,
            objects,
        })
    }

    /// Ouvre un dépôt existant en cherchant `.ioflow/` depuis `start`
    /// et en remontant l'arborescence.
    pub fn open(start: &Path) -> Result<Self, VcsError> {
        let ioflow = find_ioflow(start)?;
        let root = ioflow
            .parent()
            .expect(".ioflow a toujours un parent")
            .to_owned();
        Ok(Self {
            objects: ObjectStore::new(&ioflow),
            root,
            ioflow,
        })
    }

    /// Retourne le hash du commit pointé par HEAD, ou `None` si aucun commit.
    pub fn head(&self) -> Result<Option<Hash>, VcsError> {
        let head = fs::read_to_string(self.ioflow.join("HEAD"))?;
        let head = head.trim();

        if let Some(ref_path) = head.strip_prefix("ref: ") {
            let ref_file = self.ioflow.join(ref_path);
            if !ref_file.exists() {
                return Ok(None);
            }
            Ok(Some(fs::read_to_string(ref_file)?.trim().to_string()))
        } else {
            Ok(Some(head.to_string()))
        }
    }

    /// Met à jour HEAD (et la ref pointée) vers `hash`.
    pub fn set_head(&self, hash: &Hash) -> Result<(), VcsError> {
        let head = fs::read_to_string(self.ioflow.join("HEAD"))?;
        let head = head.trim();

        if let Some(ref_path) = head.strip_prefix("ref: ") {
            let ref_file = self.ioflow.join(ref_path);
            if let Some(parent) = ref_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(ref_file, format!("{hash}\n"))?;
        } else {
            fs::write(self.ioflow.join("HEAD"), format!("{hash}\n"))?;
        }
        Ok(())
    }

    pub fn read_commit(&self, hash: &str) -> Result<Commit, VcsError> {
        let data = self.objects.read(hash)?;
        Commit::from_bytes(&data).map_err(VcsError::Json)
    }

    pub fn read_tree(&self, hash: &str) -> Result<Tree, VcsError> {
        let data = self.objects.read(hash)?;
        Tree::from_bytes(&data).map_err(VcsError::Json)
    }

    /// Écrit le nom de l'auteur dans `.ioflow/config.toml`.
    pub fn set_author(&self, name: &str) -> Result<(), VcsError> {
        let config_path = self.ioflow.join("config.toml");
        let content = format!("[user]\nname = \"{}\"\n", name);
        fs::write(config_path, content)?;
        Ok(())
    }

    /// Auteur lu depuis `.ioflow/config.toml`, ou variable d'environnement, ou "unknown".
    pub fn author(&self) -> String {
        if let Ok(content) = fs::read_to_string(self.ioflow.join("config.toml")) {
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("name = ") {
                    let name = val.trim().trim_matches('"');
                    if !name.is_empty() {
                        return name.to_string();
                    }
                }
            }
        }
        std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string())
    }
}

fn find_ioflow(start: &Path) -> Result<PathBuf, VcsError> {
    let mut current = start.to_owned();
    loop {
        let candidate = current.join(".ioflow");
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !current.pop() {
            return Err(VcsError::NotARepo);
        }
    }
}
