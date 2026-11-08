//! Remembered account: the server URL lives in a small JSON file in the
//! config directory, the secret (session token or API key) in the operating
//! system keyring (Secret Service on Linux, Credential Manager on Windows).
//! The secret is never written to disk in clear text, and the password is
//! never stored at all.

use api_immich::Authentification;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Keyring service name; the entry user is the server URL.
const SERVICE: &str = "nicephore";

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompteMemorise {
    url_serveur: String,
    /// `session` or `cle`.
    #[serde(default = "mode_cle")]
    mode: String,
}

fn mode_cle() -> String {
    "cle".into()
}

fn fichier(dossier_config: &Path) -> PathBuf {
    dossier_config.join("compte.json")
}

fn entree(url: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, url).map_err(|e| format!("trousseau indisponible : {e}"))
}

/// Remembered URL and credentials, if any. Blocking: may wait for the
/// keyring to be unlocked, so call it from a blocking task.
pub fn charger(dossier_config: &Path) -> Option<(String, Authentification)> {
    let texte = std::fs::read_to_string(fichier(dossier_config)).ok()?;
    let compte: CompteMemorise = serde_json::from_str(&texte).ok()?;
    match entree(&compte.url_serveur).and_then(|e| e.get_password().map_err(|e| e.to_string())) {
        Ok(secret) => {
            let auth = if compte.mode == "session" {
                Authentification::Session(secret)
            } else {
                Authentification::CleApi(secret)
            };
            Some((compte.url_serveur, auth))
        }
        Err(e) => {
            tracing::warn!("identifiants mémorisés illisibles : {e}");
            None
        }
    }
}

/// Stores the secret in the keyring first, then the URL (atomically).
pub fn enregistrer(dossier_config: &Path, url: &str, auth: &Authentification) -> Result<(), String> {
    let (mode, secret) = match auth {
        Authentification::Session(jeton) => ("session", jeton),
        Authentification::CleApi(cle) => ("cle", cle),
    };
    entree(url)?.set_password(secret).map_err(|e| format!("trousseau : {e}"))?;
    std::fs::create_dir_all(dossier_config).map_err(|e| e.to_string())?;
    let compte = CompteMemorise { url_serveur: url.to_string(), mode: mode.to_string() };
    let texte = serde_json::to_string_pretty(&compte).map_err(|e| e.to_string())?;
    let temporaire = fichier(dossier_config).with_extension("json.tmp");
    std::fs::write(&temporaire, texte).map_err(|e| e.to_string())?;
    std::fs::rename(&temporaire, fichier(dossier_config)).map_err(|e| e.to_string())
}

/// Removes the remembered account (keyring entry and URL file).
pub fn oublier(dossier_config: &Path) -> Result<(), String> {
    if let Ok(texte) = std::fs::read_to_string(fichier(dossier_config))
        && let Ok(compte) = serde_json::from_str::<CompteMemorise>(&texte)
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
