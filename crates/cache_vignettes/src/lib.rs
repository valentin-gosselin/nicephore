//! Disk cache for rendered Immich images (thumbnails and previews).
//!
//! - Concurrent requests for the same image share one download.
//! - Small and large images use separate concurrency limits, so a burst of
//!   grid thumbnails never starves the viewer and vice versa.
//! - Files are written atomically (temporary file then rename).
//! - The cache key includes a version derived from the asset thumbhash: when
//!   an asset is edited on the server its thumbhash changes, so stale files
//!   are never served.
//! - `elaguer` removes the least recently used files above a size budget.

use api_immich::{ClientImmich, Erreur, TailleImage};
use bytes::Bytes;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::sync::{OnceCell, Semaphore};

/// Image ready to be served to the webview.
#[derive(Clone, Debug)]
pub struct Image {
    pub octets: Bytes,
    pub type_mime: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurCache {
    /// The asset or its rendering does not exist (yet) on the server.
    Introuvable,
    Autre(String),
}

impl std::fmt::Display for ErreurCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErreurCache::Introuvable => write!(f, "image introuvable"),
            ErreurCache::Autre(m) => write!(f, "{m}"),
        }
    }
}

type Partage = Arc<OnceCell<Result<Image, ErreurCache>>>;

pub struct CacheVignettes {
    dossier: PathBuf,
    en_vol: Mutex<HashMap<String, Partage>>,
    petites: Semaphore,
    grandes: Semaphore,
}

impl CacheVignettes {
    pub fn nouveau(dossier: PathBuf) -> Self {
        Self {
            dossier,
            en_vol: Mutex::new(HashMap::new()),
            petites: Semaphore::new(16),
            grandes: Semaphore::new(6),
        }
    }

    pub fn dossier(&self) -> &Path {
        &self.dossier
    }

    /// Returns the image from disk, or downloads it once even if many callers
    /// ask for it at the same time.
    pub async fn obtenir(
        &self,
        client: &ClientImmich,
        id: &str,
        taille: TailleImage,
        version: &str,
    ) -> Result<Image, ErreurCache> {
        api_immich::valider_id(id).map_err(|e| ErreurCache::Autre(e.to_string()))?;
        let chemin = self.chemin(id, taille, version);
        let cle = chemin.to_string_lossy().into_owned();

        let partage = {
            let mut en_vol = self.en_vol.lock().expect("verrou du cache");
            en_vol.entry(cle.clone()).or_default().clone()
        };
        let resultat = partage
            .get_or_init(|| self.charger(client, id, taille, chemin.clone()))
            .await
            .clone();
        self.en_vol.lock().expect("verrou du cache").remove(&cle);
        resultat
    }

    async fn charger(
        &self,
        client: &ClientImmich,
        id: &str,
        taille: TailleImage,
        chemin: PathBuf,
    ) -> Result<Image, ErreurCache> {
        let sur_disque = taille != TailleImage::Pleine;
        if sur_disque && let Ok(octets) = tokio::fs::read(&chemin).await {
            rafraichir_date(&chemin);
            let octets = Bytes::from(octets);
            return Ok(Image { type_mime: deviner_type(&octets), octets });
        }

        let semaphore = match taille {
            TailleImage::Vignette => &self.petites,
            _ => &self.grandes,
        };
        let _jeton = semaphore.acquire().await.map_err(|e| ErreurCache::Autre(e.to_string()))?;
        let brute = client.image(id, taille).await.map_err(|e| match e {
            Erreur::Introuvable => ErreurCache::Introuvable,
            autre => ErreurCache::Autre(autre.to_string()),
        })?;

        if sur_disque && let Err(e) = ecrire_atomique(&chemin, &brute.octets).await {
            tracing::warn!("écriture du cache impossible ({}) : {e}", chemin.display());
        }
        Ok(Image { type_mime: deviner_type(&brute.octets), octets: brute.octets })
    }

    fn chemin(&self, id: &str, taille: TailleImage, version: &str) -> PathBuf {
        self.dossier
            .join(taille.nom())
            .join(&id[..2])
            .join(format!("{id}-{}.img", empreinte_version(version)))
    }

    /// Total size and number of cached files.
    pub fn occupation(&self) -> (u64, usize) {
        let mut fichiers = Vec::new();
        let _ = lister(&self.dossier, &mut fichiers);
        (fichiers.iter().map(|(_, taille, _)| taille).sum(), fichiers.len())
    }

    /// Empties the cache (images are downloaded again when needed).
    pub fn vider(&self) -> std::io::Result<()> {
        match std::fs::remove_dir_all(&self.dossier) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// Removes the least recently used files until the cache fits in
    /// `budget_octets`. Meant to run in a background task.
    pub fn elaguer(&self, budget_octets: u64) -> std::io::Result<u64> {
        let mut fichiers = Vec::new();
        lister(&self.dossier, &mut fichiers)?;
        let total: u64 = fichiers.iter().map(|(_, taille, _)| taille).sum();
        if total <= budget_octets {
            return Ok(0);
        }
        // Oldest first.
        fichiers.sort_by_key(|(_, _, date)| *date);
        let mut a_liberer = total - budget_octets;
        let mut libere = 0;
        for (chemin, taille, _) in fichiers {
            if a_liberer == 0 {
                break;
            }
            if std::fs::remove_file(&chemin).is_ok() {
                libere += taille;
                a_liberer = a_liberer.saturating_sub(taille);
            }
        }
        Ok(libere)
    }
}

/// Short, filesystem safe fingerprint of the version string.
fn empreinte_version(version: &str) -> String {
    if version.is_empty() {
        return "0".into();
    }
    let hash = Sha1::digest(version.as_bytes());
    hex::encode(&hash[..6])
}

fn deviner_type(octets: &[u8]) -> &'static str {
    match octets {
        [0xFF, 0xD8, ..] => "image/jpeg",
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "image/webp",
        [_, _, _, _, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f', ..] => "image/avif",
        _ => "application/octet-stream",
    }
}

async fn ecrire_atomique(chemin: &Path, octets: &[u8]) -> std::io::Result<()> {
    let parent = chemin.parent().expect("chemin du cache avec parent");
    tokio::fs::create_dir_all(parent).await?;
    let temporaire = chemin.with_extension(format!("tmp{}", std::process::id()));
    tokio::fs::write(&temporaire, octets).await?;
    tokio::fs::rename(&temporaire, chemin).await
}

/// Marks a file as recently used, at most once a day to limit writes.
fn rafraichir_date(chemin: &Path) {
    let Ok(fichier) = std::fs::File::options().write(true).open(chemin) else { return };
    let recent = fichier
        .metadata()
        .and_then(|m| m.modified())
        .map(|date| date.elapsed().unwrap_or_default() < Duration::from_secs(86_400))
        .unwrap_or(true);
    if !recent {
        let _ = fichier.set_modified(SystemTime::now());
    }
}

fn lister(dossier: &Path, sortie: &mut Vec<(PathBuf, u64, SystemTime)>) -> std::io::Result<()> {
    let Ok(entrees) = std::fs::read_dir(dossier) else { return Ok(()) };
    for entree in entrees.flatten() {
        let meta = entree.metadata()?;
        if meta.is_dir() {
            lister(&entree.path(), sortie)?;
        } else {
            sortie.push((entree.path(), meta.len(), meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnait_les_formats() {
        assert_eq!(deviner_type(&[0xFF, 0xD8, 0xFF]), "image/jpeg");
        assert_eq!(deviner_type(b"RIFF\0\0\0\0WEBPVP8 "), "image/webp");
        assert_eq!(deviner_type(b"xx"), "application/octet-stream");
    }

    #[test]
    fn empreinte_stable_et_sure() {
        let a = empreinte_version("1QcSHQRnh493V4dIh4eXh1h4kJUI+A==");
        assert_eq!(a.len(), 12);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, empreinte_version("1QcSHQRnh493V4dIh4eXh1h4kJUI+A=="));
        assert_eq!(empreinte_version(""), "0");
    }

    #[test]
    fn elague_les_plus_anciens() {
        let dossier = std::env::temp_dir().join(format!("cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dossier);
        std::fs::create_dir_all(dossier.join("a")).unwrap();
        let vieux = dossier.join("a/vieux.img");
        let neuf = dossier.join("a/neuf.img");
        std::fs::write(&vieux, vec![0u8; 600]).unwrap();
        std::fs::write(&neuf, vec![0u8; 600]).unwrap();
        std::fs::File::options()
            .write(true)
            .open(&vieux)
            .unwrap()
            .set_modified(SystemTime::now() - Duration::from_secs(3600))
            .unwrap();
        let cache = CacheVignettes::nouveau(dossier.clone());
        assert_eq!(cache.elaguer(1000).unwrap(), 600);
        assert!(!vieux.exists());
        assert!(neuf.exists());
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
