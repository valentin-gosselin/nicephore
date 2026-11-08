//! Remembered account: the server URL lives in a small JSON file in the
//! config directory, the secret (session token or API key) in the operating
//! system keyring (Secret Service on Linux, Credential Manager on Windows),
//! or, in portable mode, sealed in that file with a passphrase. The secret
//! is never written to disk in clear text, and the password never stored.

use api_immich::Authentification;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::coffre::Coffre;

/// Keyring service name; the entry user is the server URL.
const SERVICE: &str = "nicephore";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompteMemorise {
    url_serveur: String,
    /// `session` or `cle`.
    #[serde(default = "mode_cle")]
    mode: String,
    /// Portable mode: the secret sealed with the passphrase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coffre: Option<Coffre>,
}

fn mode_cle() -> String {
    "cle".into()
}

/// What is remembered.
pub enum Memorise {
    Aucun,
    /// Ready to use (from the keyring).
    Pret(String, Authentification),
    /// Sealed with a passphrase: the server URL, to show while asking it.
    Verrouille(String),
}

fn fichier(dossier_config: &Path) -> PathBuf {
    dossier_config.join("compte.json")
}

fn lire_fichier(dossier_config: &Path) -> Option<CompteMemorise> {
    serde_json::from_str(&std::fs::read_to_string(fichier(dossier_config)).ok()?).ok()
}

fn entree(url: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, url).map_err(|e| format!("trousseau indisponible : {e}"))
}

fn authentification(mode: &str, secret: String) -> Authentification {
    if mode == "session" { Authentification::Session(secret) } else { Authentification::CleApi(secret) }
}

fn decomposer(auth: &Authentification) -> (&'static str, &str) {
    match auth {
        Authentification::Session(jeton) => ("session", jeton),
        Authentification::CleApi(cle) => ("cle", cle),
    }
}

/// Remembered account, if any. In portable mode the keyring is never
/// touched. Blocking (keyring may wait to be unlocked): call it from a
/// blocking task.
pub fn lire(dossier_config: &Path, portable: bool) -> Memorise {
    let Some(compte) = lire_fichier(dossier_config) else { return Memorise::Aucun };
    if compte.coffre.is_some() {
        return Memorise::Verrouille(compte.url_serveur);
    }
    if portable {
        return Memorise::Aucun;
    }
    match entree(&compte.url_serveur).and_then(|e| e.get_password().map_err(|e| e.to_string())) {
        Ok(secret) => Memorise::Pret(compte.url_serveur, authentification(&compte.mode, secret)),
        Err(e) => {
            tracing::warn!("identifiants mémorisés illisibles : {e}");
            Memorise::Aucun
        }
    }
}

/// Opens the sealed account with its passphrase (slow on purpose: about a
/// quarter of a second).
pub fn deverrouiller(dossier_config: &Path, phrase: &str) -> Result<(String, Authentification), String> {
    let compte = lire_fichier(dossier_config).ok_or("aucune connexion gardée")?;
    let coffre = compte.coffre.ok_or("connexion non chiffrée")?;
    Ok((compte.url_serveur, authentification(&compte.mode, coffre.ouvrir(phrase)?)))
}

fn ecrire(dossier_config: &Path, compte: &CompteMemorise) -> Result<(), String> {
    std::fs::create_dir_all(dossier_config).map_err(|e| e.to_string())?;
    let texte = serde_json::to_string_pretty(compte).map_err(|e| e.to_string())?;
    let temporaire = fichier(dossier_config).with_extension("json.tmp");
    std::fs::write(&temporaire, texte).map_err(|e| e.to_string())?;
    std::fs::rename(&temporaire, fichier(dossier_config)).map_err(|e| e.to_string())
}

/// Stores the secret in the keyring first, then the URL (atomically).
pub fn enregistrer(dossier_config: &Path, url: &str, auth: &Authentification) -> Result<(), String> {
    let (mode, secret) = decomposer(auth);
    entree(url)?.set_password(secret).map_err(|e| format!("trousseau : {e}"))?;
    ecrire(dossier_config, &CompteMemorise { url_serveur: url.to_string(), mode: mode.to_string(), coffre: None })
}

/// Portable mode: seals the secret with the passphrase, in the file only.
pub fn enregistrer_scelle(dossier_config: &Path, url: &str, auth: &Authentification, phrase: &str) -> Result<(), String> {
    let (mode, secret) = decomposer(auth);
    let coffre = Coffre::sceller(secret, phrase)?;
    ecrire(dossier_config, &CompteMemorise { url_serveur: url.to_string(), mode: mode.to_string(), coffre: Some(coffre) })
}

/// Removes the remembered account (keyring entry, if any, and the file).
pub fn oublier(dossier_config: &Path) -> Result<(), String> {
    if let Some(compte) = lire_fichier(dossier_config)
        && compte.coffre.is_none()
    {
        match entree(&compte.url_serveur)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(format!("trousseau : {e}")),
        }
    }
    match std::fs::remove_file(fichier(dossier_config)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compte_scelle_sans_trousseau() {
        let dossier = tempfile::tempdir().unwrap();
        assert!(matches!(lire(dossier.path(), true), Memorise::Aucun));
        let auth = Authentification::Session("jeton".into());
        enregistrer_scelle(dossier.path(), "http://h:2283", &auth, "phrase de passe").unwrap();
        let texte = std::fs::read_to_string(fichier(dossier.path())).unwrap();
        assert!(!texte.contains("jeton\""), "{texte}");
        assert!(matches!(lire(dossier.path(), true), Memorise::Verrouille(url) if url == "http://h:2283"));
        assert!(deverrouiller(dossier.path(), "mauvaise phrase").is_err());
        let (url, auth) = deverrouiller(dossier.path(), "phrase de passe").unwrap();
        assert_eq!(url, "http://h:2283");
        assert!(matches!(auth, Authentification::Session(j) if j == "jeton"));
        oublier(dossier.path()).unwrap();
        assert!(matches!(lire(dossier.path(), true), Memorise::Aucun));
    }
}
