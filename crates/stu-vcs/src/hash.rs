use sha2::{Digest, Sha256};

/// Hash SHA-256 encodé en hexadécimal (64 caractères).
pub type Hash = String;

pub fn hash_bytes(data: &[u8]) -> Hash {
    hex::encode(Sha256::digest(data))
}

/// Retourne un préfixe court (7 chars) pour l'affichage.
pub fn short(hash: &str) -> &str {
    &hash[..7.min(hash.len())]
}
