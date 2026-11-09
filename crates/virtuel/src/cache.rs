//! Cache of downloaded originals: one file per asset (`<dossier>/<id>`),
//! written through a temporary file and kept only if its SHA-1 matches the
//! server's. Two reads of the same asset share one download. Beyond the
//! budget, the least recently read files are removed.

use api_immich::{AssetDistant, ClientImmich};
use futures_util::StreamExt;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::io::AsyncWriteExt;
use tokio::sync::OnceCell;

type Resultat = Result<PathBuf, String>;

pub struct CacheOriginaux {
    dossier: PathBuf,
    budget: AtomicU64,
    en_cours: Mutex<HashMap<String, Arc<OnceCell<Resultat>>>>,
    telechargements: AtomicU64,
}

impl CacheOriginaux {
    pub fn nouveau(dossier: PathBuf, budget: u64) -> Self {
        Self { dossier, budget: AtomicU64::new(budget), en_cours: Mutex::new(HashMap::new()), telechargements: AtomicU64::new(0) }
    }

    pub fn definir_budget(&self, octets: u64) {
        self.budget.store(octets, Ordering::Relaxed);
    }

    /// Downloads made so far (for tests and statistics).
    pub fn telechargements(&self) -> u64 {
        self.telechargements.load(Ordering::Relaxed)
    }

    pub fn chemin(&self, id: &str) -> PathBuf {
        self.dossier.join(id)
    }

    /// The original on disk, downloaded first if needed.
    pub async fn obtenir(&self, client: &ClientImmich, asset: &AssetDistant) -> Resultat {
        let chemin = self.chemin(&asset.id);
        if chemin.exists() {
            toucher(&chemin);
            return Ok(chemin);
        }
        let cellule = Arc::clone(self.en_cours.lock().expect("verrou du cache").entry(asset.id.clone()).or_default());
        let resultat = cellule.get_or_init(|| self.telecharger(client, asset)).await.clone();
        self.en_cours.lock().expect("verrou du cache").remove(&asset.id);
        resultat
    }

    async fn telecharger(&self, client: &ClientImmich, asset: &AssetDistant) -> Resultat {
        api_immich::valider_id(&asset.id).map_err(|e| e.to_string())?;
        tokio::fs::create_dir_all(&self.dossier).await.map_err(|e| e.to_string())?;
        let partiel = self.dossier.join(format!(".{}.part", asset.id));
        let resultat = async {
            let reponse = client.flux_original(&asset.id).await.map_err(|e| e.to_string())?;
            let mut sortie = tokio::fs::File::create(&partiel).await.map_err(|e| e.to_string())?;
            let mut hacheur = Sha1::new();
            let mut flux = reponse.bytes_stream();
            while let Some(morceau) = flux.next().await {
                let morceau = morceau.map_err(|e| e.to_string())?;
                hacheur.update(&morceau);
                sortie.write_all(&morceau).await.map_err(|e| e.to_string())?;
            }
            sortie.flush().await.map_err(|e| e.to_string())?;
            let empreinte = hex::encode(hacheur.finalize());
            if !asset.sha1.is_empty() && empreinte != asset.sha1 {
                return Err("empreinte différente de celle du serveur".to_string());
            }
            Ok(())
        }
        .await;
        if let Err(e) = resultat {
            let _ = tokio::fs::remove_file(&partiel).await;
            tracing::warn!("original {} : {e}", asset.id);
            return Err(e);
        }
        let chemin = self.chemin(&asset.id);
        tokio::fs::rename(&partiel, &chemin).await.map_err(|e| e.to_string())?;
        self.telechargements.fetch_add(1, Ordering::Relaxed);
        let (dossier, budget, garde) = (self.dossier.clone(), self.budget.load(Ordering::Relaxed), chemin.clone());
        let _ = tokio::task::spawn_blocking(move || elaguer(&dossier, budget, &garde)).await;
        Ok(chemin)
    }

    /// Bytes and files in the cache.
    pub fn occupation(&self) -> (u64, u64) {
        let mut total = (0, 0);
        if let Ok(entrees) = std::fs::read_dir(&self.dossier) {
            for e in entrees.flatten() {
                if let Ok(m) = e.metadata()
                    && m.is_file()
                {
                    total = (total.0 + m.len(), total.1 + 1);
                }
            }
        }
        total
    }

    /// Empties the cache (the virtual folders download again on demand).
    pub fn vider(&self) -> std::io::Result<()> {
        match std::fs::remove_dir_all(&self.dossier) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// Marks a file as just read (its modification date drives eviction).
fn toucher(chemin: &Path) {
    if let Ok(f) = std::fs::File::options().write(true).open(chemin) {
        let _ = f.set_modified(SystemTime::now());
    }
}

/// Removes the least recently read files beyond the budget, never `garde`.
fn elaguer(dossier: &Path, budget: u64, garde: &Path) {
    let Ok(entrees) = std::fs::read_dir(dossier) else { return };
    let mut fichiers: Vec<(SystemTime, u64, PathBuf)> = entrees
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            (m.is_file() && !e.file_name().to_string_lossy().starts_with('.'))
                .then(|| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len(), e.path()))
        })
        .collect();
    let mut total: u64 = fichiers.iter().map(|f| f.1).sum();
    fichiers.sort_by_key(|f| f.0);
    for (_, taille, chemin) in fichiers {
        if total <= budget {
            break;
        }
        if chemin != garde && std::fs::remove_file(&chemin).is_ok() {
            total -= taille;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elagage_des_plus_anciens() {
        let dossier = tempfile::tempdir().unwrap();
        for (i, nom) in ["a", "b", "c"].iter().enumerate() {
            let chemin = dossier.path().join(nom);
            std::fs::write(&chemin, vec![0u8; 100]).unwrap();
            let date = SystemTime::now() - std::time::Duration::from_secs(1000 - i as u64 * 100);
            std::fs::File::options().write(true).open(&chemin).unwrap().set_modified(date).unwrap();
        }
        elaguer(dossier.path(), 200, &dossier.path().join("a"));
        // "a" is the oldest but kept; "b" goes, then the total fits.
        assert!(dossier.path().join("a").exists());
        assert!(!dossier.path().join("b").exists());
        assert!(dossier.path().join("c").exists());
    }
}
