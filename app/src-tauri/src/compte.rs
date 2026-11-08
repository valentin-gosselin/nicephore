//! Remembered accounts. `comptes.json` (config directory) lists them with
//! the active one; each secret (session token or API key) lives in the
//! operating system keyring (Secret Service on Linux, Credential Manager on
//! Windows) or, in portable mode, sealed in that file with a passphrase.
//! The secret is never written to disk in clear text, and the password
//! never stored.
//!
//! The single-account `compte.json` of earlier versions is still read, and
//! converted once the account has connected again.

use api_immich::{Authentification, Utilisateur};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};

use crate::coffre::Coffre;

/// Keyring service name; the entry user is the account key (the server URL
/// for the old single-account format).
const SERVICE: &str = "nicephore";

/// Stable key of an account: server and user id. Also names its upload
/// database.
pub fn cle_compte(url: &str, utilisateur: &str) -> String {
    let empreinte = Sha1::digest(format!("{}\n{utilisateur}", url.trim_end_matches('/')).as_bytes());
    hex::encode(&empreinte[..8])
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Compte {
    cle: String,
    url_serveur: String,
    #[serde(default)]
    nom: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    couleur: Option<String>,
    /// `session` or `cle`.
    mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coffre: Option<Coffre>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comptes {
    actif: Option<String>,
    comptes: Vec<Compte>,
}

/// Single-account format of earlier versions.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AncienCompte {
    url_serveur: String,
    #[serde(default = "mode_cle")]
    mode: String,
    #[serde(default)]
    coffre: Option<Coffre>,
}

fn mode_cle() -> String {
    "cle".into()
}

/// What is remembered for the active account.
pub enum Memorise {
    Aucun,
    /// Ready to use (from the keyring). `ancien`: old format, to convert
    /// once connected.
    Pret { url: String, auth: Authentification, ancien: bool },
    /// Sealed with a passphrase: the server URL, to show while asking it.
    Verrouille(String),
}

/// An account as listed in the account menu.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompteVue {
    pub cle: String,
    pub url_serveur: String,
    pub nom: String,
    pub email: String,
    pub couleur: Option<String>,
    pub scelle: bool,
    pub actif: bool,
}

fn fichier(dossier: &Path) -> PathBuf {
    dossier.join("comptes.json")
}

fn ancien_fichier(dossier: &Path) -> PathBuf {
    dossier.join("compte.json")
}

fn lire_comptes(dossier: &Path) -> Comptes {
    std::fs::read_to_string(fichier(dossier)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn ecrire_comptes(dossier: &Path, comptes: &Comptes) -> Result<(), String> {
    std::fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
    let texte = serde_json::to_string_pretty(comptes).map_err(|e| e.to_string())?;
    let temporaire = fichier(dossier).with_extension("json.tmp");
    std::fs::write(&temporaire, texte).map_err(|e| e.to_string())?;
    std::fs::rename(&temporaire, fichier(dossier)).map_err(|e| e.to_string())
}

fn lire_ancien(dossier: &Path) -> Option<AncienCompte> {
    serde_json::from_str(&std::fs::read_to_string(ancien_fichier(dossier)).ok()?).ok()
}

fn entree(utilisateur: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, utilisateur).map_err(|e| format!("trousseau indisponible : {e}"))
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

fn lire_trousseau(utilisateur: &str) -> Option<String> {
    match entree(utilisateur).and_then(|e| e.get_password().map_err(|e| e.to_string())) {
        Ok(secret) => Some(secret),
        Err(e) => {
            tracing::warn!("identifiants mémorisés illisibles : {e}");
            None
        }
    }
}

/// The active account, if any. In portable mode the keyring is never
/// touched. Blocking (keyring may wait to be unlocked): call it from a
/// blocking task.
pub fn lire(dossier: &Path, portable: bool) -> Memorise {
    let comptes = lire_comptes(dossier);
    if let Some(compte) = comptes.actif.as_ref().and_then(|a| comptes.comptes.iter().find(|c| &c.cle == a)) {
        if compte.coffre.is_some() {
            return Memorise::Verrouille(compte.url_serveur.clone());
        }
        if portable {
            return Memorise::Aucun;
        }
        return match lire_trousseau(&compte.cle) {
            Some(secret) => Memorise::Pret {
                url: compte.url_serveur.clone(),
                auth: authentification(&compte.mode, secret),
                ancien: false,
            },
            None => Memorise::Aucun,
        };
    }
    // Earlier versions: one account in compte.json.
    match lire_ancien(dossier) {
        Some(ancien) if ancien.coffre.is_some() => Memorise::Verrouille(ancien.url_serveur),
        Some(ancien) if !portable => match lire_trousseau(&ancien.url_serveur) {
            Some(secret) => Memorise::Pret { url: ancien.url_serveur, auth: authentification(&ancien.mode, secret), ancien: true },
            None => Memorise::Aucun,
        },
        _ => Memorise::Aucun,
    }
}

/// Opens the active sealed account with its passphrase (slow on purpose:
/// about a quarter of a second). `ancien`: from the old format.
pub fn deverrouiller(dossier: &Path, phrase: &str) -> Result<(String, Authentification, bool), String> {
    let comptes = lire_comptes(dossier);
    if let Some(compte) = comptes.actif.as_ref().and_then(|a| comptes.comptes.iter().find(|c| &c.cle == a)) {
        let coffre = compte.coffre.as_ref().ok_or("connexion non chiffrée")?;
        return Ok((compte.url_serveur.clone(), authentification(&compte.mode, coffre.ouvrir(phrase)?), false));
    }
    let ancien = lire_ancien(dossier).ok_or("aucune connexion gardée")?;
    let coffre = ancien.coffre.ok_or("connexion non chiffrée")?;
    Ok((ancien.url_serveur, authentification(&ancien.mode, coffre.ouvrir(phrase)?), true))
}

/// Remembers an account and makes it the active one: the secret in the
/// keyring, or sealed with `phrase` (portable mode). Converts the old
/// single-account format on the way.
pub fn enregistrer(
    dossier: &Path,
    url: &str,
    utilisateur: &Utilisateur,
    auth: &Authentification,
    phrase: Option<&str>,
) -> Result<String, String> {
    let (mode, secret) = decomposer(auth);
    let cle = cle_compte(url, &utilisateur.id);
    let coffre = match phrase {
        Some(phrase) => Some(Coffre::sceller(secret, phrase)?),
        None => {
            entree(&cle)?.set_password(secret).map_err(|e| format!("trousseau : {e}"))?;
            None
        }
    };
    let mut comptes = lire_comptes(dossier);
    comptes.comptes.retain(|c| c.cle != cle);
    comptes.comptes.push(Compte {
        cle: cle.clone(),
        url_serveur: url.to_string(),
        nom: utilisateur.name.clone(),
        email: utilisateur.email.clone(),
        couleur: utilisateur.couleur_avatar.clone(),
        mode: mode.to_string(),
        coffre,
    });
    comptes.actif = Some(cle.clone());
    ecrire_comptes(dossier, &comptes)?;
    oublier_ancien(dossier);
    Ok(cle)
}

/// Removes the old single-account file and its keyring entry.
fn oublier_ancien(dossier: &Path) {
    let Some(ancien) = lire_ancien(dossier) else { return };
    if ancien.coffre.is_none()
        && let Ok(e) = entree(&ancien.url_serveur)
    {
        let _ = e.delete_credential();
    }
    let _ = std::fs::remove_file(ancien_fichier(dossier));
}

/// Converts an old-format account once it has connected (its user is known).
pub fn convertir_ancien(dossier: &Path, url: &str, utilisateur: &Utilisateur, auth: &Authentification) -> Result<(), String> {
    let scelle = lire_ancien(dossier).and_then(|a| a.coffre);
    match scelle {
        // A sealed old account keeps its seal: the passphrase is not known here.
        Some(coffre) => {
            let (mode, _) = decomposer(auth);
            let cle = cle_compte(url, &utilisateur.id);
            let mut comptes = lire_comptes(dossier);
            comptes.comptes.retain(|c| c.cle != cle);
            comptes.comptes.push(Compte {
                cle: cle.clone(),
                url_serveur: url.to_string(),
                nom: utilisateur.name.clone(),
                email: utilisateur.email.clone(),
                couleur: utilisateur.couleur_avatar.clone(),
                mode: mode.to_string(),
                coffre: Some(coffre),
            });
            comptes.actif = Some(cle);
            ecrire_comptes(dossier, &comptes)?;
            let _ = std::fs::remove_file(ancien_fichier(dossier));
            Ok(())
        }
        None => enregistrer(dossier, url, utilisateur, auth, None).map(|_| ()),
    }
}

pub fn lister(dossier: &Path) -> Vec<CompteVue> {
    let comptes = lire_comptes(dossier);
    comptes
        .comptes
        .iter()
        .map(|c| CompteVue {
            cle: c.cle.clone(),
            url_serveur: c.url_serveur.clone(),
            nom: c.nom.clone(),
            email: c.email.clone(),
            couleur: c.couleur.clone(),
            scelle: c.coffre.is_some(),
            actif: comptes.actif.as_deref() == Some(c.cle.as_str()),
        })
        .collect()
}

pub fn activer(dossier: &Path, cle: &str) -> Result<(), String> {
    let mut comptes = lire_comptes(dossier);
    if !comptes.comptes.iter().any(|c| c.cle == cle) {
        return Err("compte inconnu".into());
    }
    comptes.actif = Some(cle.to_string());
    ecrire_comptes(dossier, &comptes)
}

/// Forgets the active account (keyring entry, if any, and its line). The
/// next remembered account, if any, becomes the active one.
pub fn oublier_actif(dossier: &Path) -> Result<(), String> {
    oublier_ancien(dossier);
    let mut comptes = lire_comptes(dossier);
    let Some(actif) = comptes.actif.take() else { return Ok(()) };
    if let Some(compte) = comptes.comptes.iter().find(|c| c.cle == actif)
        && compte.coffre.is_none()
    {
        match entree(&compte.cle)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(format!("trousseau : {e}")),
        }
    }
    comptes.comptes.retain(|c| c.cle != actif);
    comptes.actif = comptes.comptes.first().map(|c| c.cle.clone());
    ecrire_comptes(dossier, &comptes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utilisateur(id: &str, nom: &str) -> Utilisateur {
        Utilisateur {
            id: id.into(),
            name: nom.into(),
            email: format!("{nom}@exemple.invalid"),
            image_profil: String::new(),
            couleur_avatar: Some("pink".into()),
            profil_modifie_le: None,
        }
    }

    #[test]
    fn plusieurs_comptes_scelles() {
        let dossier = tempfile::tempdir().unwrap();
        let d = dossier.path();
        assert!(matches!(lire(d, true), Memorise::Aucun));
        let a = enregistrer(d, "http://h:2283", &utilisateur("u1", "Valentin"), &Authentification::Session("j1".into()), Some("phrase un ok"))
            .unwrap();
        let b = enregistrer(d, "http://h:2283", &utilisateur("u2", "Test"), &Authentification::Session("j2".into()), Some("phrase deux ok"))
            .unwrap();
        assert_ne!(a, b);
        let liste = lister(d);
        assert_eq!(liste.len(), 2);
        assert!(liste.iter().any(|c| c.cle == b && c.actif && c.scelle));
        assert!(!std::fs::read_to_string(fichier(d)).unwrap().contains("\"j2\""));
        // The active one is the last added.
        assert!(deverrouiller(d, "phrase un ok").is_err());
        assert!(matches!(deverrouiller(d, "phrase deux ok").unwrap().1, Authentification::Session(j) if j == "j2"));
        activer(d, &a).unwrap();
        assert!(matches!(deverrouiller(d, "phrase un ok").unwrap().1, Authentification::Session(j) if j == "j1"));
        // Same server and user again: updated, not duplicated.
        enregistrer(d, "http://h:2283/", &utilisateur("u1", "Valentin G."), &Authentification::Session("j3".into()), Some("phrase un ok"))
            .unwrap();
        assert_eq!(lister(d).len(), 2);
        // Logging out of the active account keeps the other one.
        oublier_actif(d).unwrap();
        let reste = lister(d);
        assert_eq!(reste.len(), 1);
        assert!(reste[0].actif && reste[0].nom == "Test");
        oublier_actif(d).unwrap();
        assert!(lister(d).is_empty());
        assert!(matches!(lire(d, true), Memorise::Aucun));
    }

    #[test]
    fn ancien_format_scelle_converti() {
        let dossier = tempfile::tempdir().unwrap();
        let d = dossier.path();
        let coffre = Coffre::sceller("jeton", "phrase de passe").unwrap();
        let ancien = serde_json::json!({ "urlServeur": "http://h:2283", "mode": "session", "coffre": coffre });
        std::fs::write(ancien_fichier(d), ancien.to_string()).unwrap();
        assert!(matches!(lire(d, true), Memorise::Verrouille(url) if url == "http://h:2283"));
        let (url, auth, ancien) = deverrouiller(d, "phrase de passe").unwrap();
        assert!(ancien);
        convertir_ancien(d, &url, &utilisateur("u1", "Valentin"), &auth).unwrap();
        assert!(!ancien_fichier(d).exists());
        let liste = lister(d);
        assert_eq!((liste.len(), liste[0].scelle, liste[0].actif), (1, true, true));
        assert!(deverrouiller(d, "phrase de passe").is_ok());
    }

    #[test]
    fn cle_stable() {
        assert_eq!(cle_compte("http://h:2283/", "u"), cle_compte("http://h:2283", "u"));
        assert_ne!(cle_compte("http://h:2283", "u"), cle_compte("http://h:2283", "v"));
    }
}
