//! Portable mode: a file named `portable` next to the executable (or next
//! to the AppImage) keeps everything in a `donnees` folder beside it:
//! settings, upload database, image cache, web view profile. Nothing goes
//! to the user profile, the registry or the system keyring.

use std::path::{Path, PathBuf};

/// Where Nicéphore keeps its files.
#[derive(Debug, Clone)]
pub struct Emplacements {
    pub config: PathBuf,
    pub cache: PathBuf,
    pub donnees: PathBuf,
    /// Portable mode: the `donnees` folder; the web view profile goes there too.
    pub portable: Option<PathBuf>,
}

impl Emplacements {
    pub fn portables(racine: &Path) -> Self {
        Self {
            config: racine.join("config"),
            cache: racine.join("cache"),
            donnees: racine.join("envoi"),
            portable: Some(racine.to_path_buf()),
        }
    }

    pub fn profil_web(&self) -> Option<PathBuf> {
        self.portable.as_ref().map(|r| r.join("webview"))
    }
}

/// The `donnees` folder when running in portable mode.
/// `NICEPHORE_PORTABLE` (a folder) forces it, for tests.
pub fn detecter() -> Option<PathBuf> {
    if let Some(dossier) = std::env::var_os("NICEPHORE_PORTABLE") {
        return Some(PathBuf::from(dossier));
    }
    let programme = std::env::var_os("APPIMAGE").map(PathBuf::from).or_else(|| std::env::current_exe().ok())?;
    a_cote(programme.parent()?)
}

fn a_cote(dossier: &Path) -> Option<PathBuf> {
    dossier.join("portable").is_file().then(|| dossier.join("donnees"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichier_portable_a_cote() {
        let dossier = tempfile::tempdir().unwrap();
        assert_eq!(a_cote(dossier.path()), None);
        std::fs::write(dossier.path().join("portable"), "").unwrap();
        assert_eq!(a_cote(dossier.path()), Some(dossier.path().join("donnees")));
        let e = Emplacements::portables(&dossier.path().join("donnees"));
        assert!(e.config.starts_with(dossier.path()) && e.cache.starts_with(dossier.path()));
        assert_eq!(e.profil_web(), Some(dossier.path().join("donnees").join("webview")));
    }
}
