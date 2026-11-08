//! Application settings kept in `reglages.json` (config directory), and
//! start with the session.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Argument given by the session autostart: start hidden in the tray.
pub const ARG_ARRIERE_PLAN: &str = "--arriere-plan";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Reglages {
    /// Closing the window keeps Nicéphore in the tray while folders are sent.
    pub fermer_en_arriere_plan: bool,
    pub pause_sur_batterie: bool,
    pub pause_reseau_facture: bool,
    pub notifications: bool,
}

impl Default for Reglages {
    fn default() -> Self {
        Self { fermer_en_arriere_plan: true, pause_sur_batterie: true, pause_reseau_facture: true, notifications: true }
    }
}

fn fichier(dossier_config: &Path) -> PathBuf {
    dossier_config.join("reglages.json")
}

pub fn charger(dossier_config: &Path) -> Reglages {
    std::fs::read_to_string(fichier(dossier_config))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn enregistrer(dossier_config: &Path, reglages: &Reglages) -> Result<(), String> {
    std::fs::create_dir_all(dossier_config).map_err(|e| e.to_string())?;
    let texte = serde_json::to_string_pretty(reglages).map_err(|e| e.to_string())?;
    let temporaire = fichier(dossier_config).with_extension("json.tmp");
    std::fs::write(&temporaire, texte).map_err(|e| e.to_string())?;
    std::fs::rename(&temporaire, fichier(dossier_config)).map_err(|e| e.to_string())
}

/// Program to start with the session: the AppImage itself when running
/// from one (its mount point changes at every launch), else this binary.
fn programme() -> Result<PathBuf, String> {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(appimage));
    }
    std::env::current_exe().map_err(|e| e.to_string())
}

// ---- Linux: XDG autostart entry ----

#[cfg(not(windows))]
fn entree_autostart() -> Result<PathBuf, String> {
    let config = dirs::config_dir().ok_or("dossier de configuration introuvable")?;
    Ok(config.join("autostart").join("nicephore.desktop"))
}

#[cfg(not(windows))]
fn contenu_desktop(programme: &Path) -> String {
    // Exec quoting per the Desktop Entry spec.
    let chemin = programme.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"").replace('`', "\\`").replace('$', "\\$");
    format!(
        "[Desktop Entry]\nType=Application\nName=Nicéphore\nComment=Envoi des photos vers Immich\n\
         Exec=\"{chemin}\" {ARG_ARRIERE_PLAN}\nIcon=nicephore\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    )
}

#[cfg(not(windows))]
pub fn demarrage_auto() -> bool {
    entree_autostart().map(|e| e.is_file()).unwrap_or(false)
}

#[cfg(not(windows))]
pub fn definir_demarrage_auto(actif: bool) -> Result<(), String> {
    let entree = entree_autostart()?;
    if actif {
        std::fs::create_dir_all(entree.parent().expect("dossier autostart")).map_err(|e| e.to_string())?;
        std::fs::write(&entree, contenu_desktop(&programme()?)).map_err(|e| e.to_string())
    } else {
        match std::fs::remove_file(&entree) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

// ---- Windows: per-user Run key ----

#[cfg(windows)]
const CLE_RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(windows)]
pub fn demarrage_auto() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(CLE_RUN)
        .and_then(|cle| cle.get_value::<String, _>("Nicéphore"))
        .is_ok()
}

#[cfg(windows)]
pub fn definir_demarrage_auto(actif: bool) -> Result<(), String> {
    // Opened with full access by `create_subkey`.
    let (cle, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .create_subkey(CLE_RUN)
        .map_err(|e| e.to_string())?;
    if actif {
        let commande = format!("\"{}\" {ARG_ARRIERE_PLAN}", programme()?.display());
        cle.set_value("Nicéphore", &commande).map_err(|e| e.to_string())
    } else {
        match cle.delete_value("Nicéphore") {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglages_par_defaut_et_relus() {
        let dossier = tempfile::tempdir().unwrap();
        assert_eq!(charger(dossier.path()), Reglages::default());
        let r = Reglages { pause_sur_batterie: false, ..Default::default() };
        enregistrer(dossier.path(), &r).unwrap();
        assert_eq!(charger(dossier.path()), r);
        // Unknown or missing keys keep their defaults.
        std::fs::write(fichier(dossier.path()), r#"{"notifications": false, "inconnu": 1}"#).unwrap();
        assert_eq!(charger(dossier.path()), Reglages { notifications: false, ..Default::default() });
    }

    #[cfg(not(windows))]
    #[test]
    fn entree_desktop_echappee() {
        let texte = contenu_desktop(Path::new("/home/v/Applications/Nicéphore $1.AppImage"));
        assert!(texte.contains("Exec=\"/home/v/Applications/Nicéphore \\$1.AppImage\" --arriere-plan\n"), "{texte}");
    }
}
