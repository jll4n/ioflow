use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("XML invalide : {0}")]
    Xml(#[from] roxmltree::Error),

    #[error("attribut '{attr}' manquant sur <{element}>")]
    MissingAttr {
        element: &'static str,
        attr: &'static str,
    },

    #[error("élément <{0}> manquant")]
    MissingElement(&'static str),

    #[error("type de POU inconnu : '{0}' (attendu : program | functionBlock | function)")]
    UnknownPouType(String),

    #[error("type de donnée inconnu : '{0}'")]
    UnknownDataType(String),

    #[error("le POU '{0}' n'a pas de corps (<body>)")]
    NoBody(String),

    #[error("valeur entière invalide pour '{attr}' : '{value}'")]
    InvalidInt { attr: &'static str, value: String },
}
