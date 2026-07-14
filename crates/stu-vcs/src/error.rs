use thiserror::Error;

#[derive(Debug, Error)]
pub enum VcsError {
    #[error("erreur I/O : {0}")]
    Io(#[from] std::io::Error),

    #[error("archive STU invalide : {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("données de dépôt corrompues : {0}")]
    Json(#[from] serde_json::Error),

    #[error("pas de dépôt ioflow trouvé (lancez 'ioflow init')")]
    NotARepo,

    #[error("objet introuvable : {0}")]
    ObjectNotFound(String),

    #[error("commit introuvable : {0}")]
    CommitNotFound(String),

    #[error("dépôt déjà initialisé dans {0}")]
    AlreadyInitialized(String),

    #[error("aucun commit — faites d'abord 'ioflow snapshot'")]
    NoCommits,
}
