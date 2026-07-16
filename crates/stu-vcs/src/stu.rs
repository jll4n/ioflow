use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::ZipArchive;

use crate::error::VcsError;

/// Contenu extrait d'une archive `.stu` (zip).
/// La clé est le chemin relatif dans l'archive (ex : `"Project_Settings.xso"`).
pub struct StuArchive {
    pub files: HashMap<String, Vec<u8>>,
}

impl StuArchive {
    /// Ouvre et extrait tous les fichiers d'un `.stu`.
    pub fn open(path: &Path) -> Result<Self, VcsError> {
        let file = std::fs::File::open(path)?;
        let mut archive =
            ZipArchive::new(file).map_err(|_| VcsError::InvalidStu(path.display().to_string()))?;
        let mut files = HashMap::new();

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.is_file() {
                let name = entry.name().to_string();
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                files.insert(name, data);
            }
        }

        Ok(Self { files })
    }

    /// Recrée un fichier `.stu` depuis un ensemble de fichiers (pour `restore`).
    pub fn write(files: &HashMap<String, Vec<u8>>, dest: &Path) -> Result<(), VcsError> {
        use std::io::Write;

        let file = std::fs::File::create(dest)?;
        let mut archive = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // Ordre déterministe
        let mut names: Vec<&String> = files.keys().collect();
        names.sort();

        for name in names {
            archive.start_file(name, options)?;
            archive.write_all(&files[name])?;
        }

        archive.finish()?;
        Ok(())
    }
}
