use similar::TextDiff;

use crate::hash::Hash;
use crate::tree::Tree;

/// Changement sur un fichier entre deux commits.
#[derive(Debug, Clone)]
pub enum FileChange {
    Added {
        path: String,
        hash: Hash,
    },
    Removed {
        path: String,
        hash: Hash,
    },
    Modified {
        path: String,
        old_hash: Hash,
        new_hash: Hash,
    },
    Unchanged {
        path: String,
    },
}

impl FileChange {
    pub fn path(&self) -> &str {
        match self {
            Self::Added { path, .. } => path,
            Self::Removed { path, .. } => path,
            Self::Modified { path, .. } => path,
            Self::Unchanged { path } => path,
        }
    }

    pub fn symbol(&self) -> char {
        match self {
            Self::Added { .. } => '+',
            Self::Removed { .. } => '-',
            Self::Modified { .. } => '~',
            Self::Unchanged { .. } => '=',
        }
    }
}

/// Compare deux trees et retourne la liste des changements, triée par chemin.
pub fn diff_trees(old: &Tree, new: &Tree) -> Vec<FileChange> {
    let mut changes = Vec::new();

    for (path, new_hash) in &new.files {
        match old.files.get(path) {
            Some(old_hash) if old_hash == new_hash => {
                changes.push(FileChange::Unchanged { path: path.clone() })
            }
            Some(old_hash) => changes.push(FileChange::Modified {
                path: path.clone(),
                old_hash: old_hash.clone(),
                new_hash: new_hash.clone(),
            }),
            None => changes.push(FileChange::Added {
                path: path.clone(),
                hash: new_hash.clone(),
            }),
        }
    }

    for (path, old_hash) in &old.files {
        if !new.files.contains_key(path) {
            changes.push(FileChange::Removed {
                path: path.clone(),
                hash: old_hash.clone(),
            });
        }
    }

    changes.sort_by(|a, b| a.path().cmp(b.path()));
    changes
}

/// Retourne `true` si le fichier peut faire l'objet d'un diff texte ligne à ligne.
pub fn is_text_diffable(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".xso") || lower.ends_with(".asm")
}

/// Produit un diff unifié (format patch) entre deux versions d'un fichier texte.
/// Retourne `None` si l'un des buffers n'est pas de l'UTF-8 valide ou si le contenu
/// est identique.
pub fn text_diff(old: &[u8], new: &[u8], path: &str) -> Option<String> {
    let old_str = std::str::from_utf8(old).ok()?;
    let new_str = std::str::from_utf8(new).ok()?;

    let diff = TextDiff::from_lines(old_str, new_str);
    if diff.ratio() >= 1.0 {
        return None; // identiques
    }
    Some(
        diff.unified_diff()
            .header(&format!("a/{path}"), &format!("b/{path}"))
            .to_string(),
    )
}

/// Étiquette human-friendly selon l'extension du fichier.
pub fn file_label(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".xso") {
        "XML paramètres"
    } else if lower.ends_with(".xpdf") {
        "XML chiffré Schneider"
    } else if lower.ends_with(".db") {
        "base propriétaire eXc"
    } else if lower.ends_with(".asm") {
        "assembleur généré"
    } else if lower.ends_with(".apb") || lower.ends_with(".apd") || lower.ends_with(".apx") {
        "binaire compilé"
    } else if lower.ends_with(".bmp") {
        "image"
    } else if lower.ends_with(".ctx") {
        "contexte binaire"
    } else if lower.ends_with(".odb") {
        "base objets"
    } else {
        "binaire"
    }
}
