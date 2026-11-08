//! Secret sealed with a passphrase, for portable mode where no system
//! keyring may be used: Argon2id derives the key, XChaCha20-Poly1305
//! encrypts and authenticates. A wrong passphrase fails cleanly.

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};

/// Shortest accepted passphrase.
pub const LONGUEUR_MINIMALE: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coffre {
    /// Argon2id parameters, kept so they can be raised later.
    memoire_kio: u32,
    passes: u32,
    sel: String,
    nonce: String,
    chiffre: String,
}

fn cle(phrase: &str, sel: &[u8], memoire_kio: u32, passes: u32) -> Result<[u8; 32], String> {
    let parametres = Params::new(memoire_kio, passes, 1, Some(32)).map_err(|e| e.to_string())?;
    let mut cle = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, parametres)
        .hash_password_into(phrase.as_bytes(), sel, &mut cle)
        .map_err(|e| e.to_string())?;
    Ok(cle)
}

fn aleatoire<const N: usize>() -> Result<[u8; N], String> {
    let mut octets = [0u8; N];
    getrandom::fill(&mut octets).map_err(|e| e.to_string())?;
    Ok(octets)
}

impl Coffre {
    pub fn sceller(secret: &str, phrase: &str) -> Result<Self, String> {
        if phrase.chars().count() < LONGUEUR_MINIMALE {
            return Err(format!("la phrase de passe doit faire au moins {LONGUEUR_MINIMALE} caractères"));
        }
        let (memoire_kio, passes) = (64 * 1024, 3);
        let sel: [u8; 16] = aleatoire()?;
        let nonce: [u8; 24] = aleatoire()?;
        let chiffreur = XChaCha20Poly1305::new(&cle(phrase, &sel, memoire_kio, passes)?.into());
        let chiffre = chiffreur
            .encrypt(&XNonce::from(nonce), secret.as_bytes())
            .map_err(|_| "chiffrement impossible".to_string())?;
        Ok(Self { memoire_kio, passes, sel: hex::encode(sel), nonce: hex::encode(nonce), chiffre: hex::encode(chiffre) })
    }

    pub fn ouvrir(&self, phrase: &str) -> Result<String, String> {
        let decoder = |t: &str| hex::decode(t).map_err(|_| "coffre abîmé".to_string());
        let nonce: [u8; 24] = decoder(&self.nonce)?.try_into().map_err(|_| "coffre abîmé".to_string())?;
        let chiffreur = XChaCha20Poly1305::new(&cle(phrase, &decoder(&self.sel)?, self.memoire_kio, self.passes)?.into());
        let clair = chiffreur
            .decrypt(&XNonce::from(nonce), decoder(&self.chiffre)?.as_slice())
            .map_err(|_| "phrase de passe incorrecte".to_string())?;
        String::from_utf8(clair).map_err(|_| "coffre abîmé".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scelle_et_rouvre() {
        let coffre = Coffre::sceller("jeton-de-session", "une phrase assez longue").unwrap();
        assert!(!coffre.chiffre.contains(&hex::encode("jeton-de-session")));
        assert_eq!(coffre.ouvrir("une phrase assez longue").unwrap(), "jeton-de-session");
        assert_eq!(coffre.ouvrir("une autre phrase").unwrap_err(), "phrase de passe incorrecte");
        // Two seals of the same secret never look alike.
        assert_ne!(Coffre::sceller("jeton-de-session", "une phrase assez longue").unwrap(), coffre);
    }

    #[test]
    fn phrase_trop_courte() {
        assert!(Coffre::sceller("x", "court").is_err());
    }
}
