//! Background download of every grid thumbnail, so the whole timeline
//! scrolls instantly and stays browsable without the server. Low priority:
//! a few parallel requests, paused while the user scrolls, cancelled when
//! the setting is turned off or the user logs out.

use api_immich::{ClientImmich, FiltreChronologie, TailleImage};
use cache_vignettes::{CacheVignettes, ErreurCache};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Parallel downloads while preloading (the visible grid uses up to 16).
const PARALLELE: usize = 4;
/// Pause after the last scroll signal from the UI.
const PAUSE_DEFILEMENT: Duration = Duration::from_secs(2);

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct EtatPrechargement {
    pub actif: bool,
    pub en_cours: bool,
    pub termine: bool,
    /// Assets of the main timeline and the archive.
    pub total: u64,
    /// Thumbnails now on disk.
    pub presentes: u64,
    pub erreurs: u64,
    /// The last finished pass found every thumbnail on disk (remembered
    /// across launches): the grid can then read them even while flinging.
    pub complet: bool,
}

#[derive(Serialize, Deserialize)]
struct Reglage {
    prechargement: bool,
    #[serde(default)]
    complet: bool,
}

pub struct Prechargement {
    actif: AtomicBool,
    /// Bumped to cancel the running pass.
    generation: AtomicU64,
    pause_jusqu_a: Mutex<Instant>,
    etat: Mutex<EtatPrechargement>,
    fichier_reglage: PathBuf,
    app: AppHandle,
}

impl Prechargement {
    pub fn nouveau(dossier_config: &Path, app: AppHandle) -> Self {
        let fichier_reglage = dossier_config.join("prechargement.json");
        let reglage = std::fs::read_to_string(&fichier_reglage)
            .ok()
            .and_then(|t| serde_json::from_str::<Reglage>(&t).ok())
            .unwrap_or(Reglage { prechargement: true, complet: false });
        let actif = reglage.prechargement;
        Self {
            actif: AtomicBool::new(actif),
            generation: AtomicU64::new(0),
            pause_jusqu_a: Mutex::new(Instant::now()),
            etat: Mutex::new(EtatPrechargement { actif, complet: reglage.complet, ..Default::default() }),
            fichier_reglage,
            app,
        }
    }

    pub fn etat(&self) -> EtatPrechargement {
        let mut etat = self.etat.lock().expect("verrou du préchargement").clone();
        etat.actif = self.actif.load(Ordering::Relaxed);
        etat
    }

    pub fn est_actif(&self) -> bool {
        self.actif.load(Ordering::Relaxed)
    }

    /// Turns the setting on or off (remembered); turning it off stops the pass.
    pub fn definir_actif(&self, actif: bool) -> Result<(), String> {
        self.actif.store(actif, Ordering::Relaxed);
        let complet = self.etat.lock().expect("verrou du préchargement").complet;
        self.enregistrer(actif, complet)?;
        if !actif {
            self.arreter();
        }
        self.emettre();
        Ok(())
    }

    fn enregistrer(&self, actif: bool, complet: bool) -> Result<(), String> {
        if let Some(dossier) = self.fichier_reglage.parent() {
            std::fs::create_dir_all(dossier).map_err(|e| e.to_string())?;
        }
        let texte = serde_json::to_string(&Reglage { prechargement: actif, complet }).map_err(|e| e.to_string())?;
        std::fs::write(&self.fichier_reglage, texte).map_err(|e| e.to_string())
    }

    /// The cache was emptied: thumbnails are no longer all on disk.
    pub fn oublier_complet(&self) {
        self.etat.lock().expect("verrou du préchargement").complet = false;
        let _ = self.enregistrer(self.est_actif(), false);
    }

    /// Stops the running pass (logout, setting off).
    pub fn arreter(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        let mut etat = self.etat.lock().expect("verrou du préchargement");
        etat.en_cours = false;
    }

    /// The user is scrolling: give the network to the visible grid.
    pub fn signaler_activite(&self) {
        *self.pause_jusqu_a.lock().expect("verrou de la pause") = Instant::now() + PAUSE_DEFILEMENT;
    }

    /// Starts a pass if enabled and none is running.
    pub fn lancer(self: &Arc<Self>, client: Arc<ClientImmich>, cache: Arc<CacheVignettes>) {
        if !self.est_actif() || self.etat.lock().expect("verrou du préchargement").en_cours {
            return;
        }
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut etat = self.etat.lock().expect("verrou du préchargement");
            *etat = EtatPrechargement { actif: true, en_cours: true, complet: etat.complet, ..Default::default() };
        }
        let moi = self.clone();
        tauri::async_runtime::spawn(async move {
            let debut = Instant::now();
            moi.parcourir(generation, &client, &cache).await;
            if moi.annule(generation) {
                return;
            }
            let etat = {
                let mut etat = moi.etat.lock().expect("verrou du préchargement");
                etat.en_cours = false;
                etat.termine = true;
                // Missing renderings on the server (errors) do not count against it.
                etat.complet = etat.total > 0 && etat.presentes + etat.erreurs >= etat.total;
                etat.clone()
            };
            let _ = moi.enregistrer(true, etat.complet);
            tracing::info!(
                "préchargement terminé en {} s : {} vignettes sur {}, {} erreurs",
                debut.elapsed().as_secs(),
                etat.presentes,
                etat.total,
                etat.erreurs
            );
            moi.emettre();
        });
    }

    fn annule(&self, generation: u64) -> bool {
        self.generation.load(Ordering::SeqCst) != generation
    }

    fn compter(&self, presentes: u64, erreurs: u64) {
        let mut etat = self.etat.lock().expect("verrou du préchargement");
        etat.presentes += presentes;
        etat.erreurs += erreurs;
    }

    fn emettre(&self) {
        let _ = self.app.emit("prechargement", self.etat());
    }

    async fn attendre_pause(&self, generation: u64) {
        loop {
            let reste = self.pause_jusqu_a.lock().expect("verrou de la pause").saturating_duration_since(Instant::now());
            if reste.is_zero() || self.annule(generation) {
                return;
            }
            tokio::time::sleep(reste.min(Duration::from_millis(500))).await;
        }
    }

    async fn parcourir(&self, generation: u64, client: &ClientImmich, cache: &CacheVignettes) {
        // Favorites are part of the main timeline; the trash is skipped.
        let vues = [
            FiltreChronologie::default(),
            FiltreChronologie { visibilite: Some("archive".into()), ..Default::default() },
        ];
        let mut mois_par_vue = Vec::new();
        for filtre in &vues {
            match client.liste_mois(filtre).await {
                Ok(mois) => {
                    self.etat.lock().expect("verrou du préchargement").total +=
                        mois.iter().map(|m| u64::from(m.count)).sum::<u64>();
                    mois_par_vue.push((filtre, mois));
                }
                Err(e) => tracing::warn!("préchargement : liste des mois impossible : {e}"),
            }
        }
        self.emettre();

        for (filtre, mois) in mois_par_vue {
            for m in mois {
                if self.annule(generation) {
                    return;
                }
                let detail = match client.detail_mois(&m.time_bucket, filtre).await {
                    Ok(detail) => detail,
                    Err(e) => {
                        tracing::warn!("préchargement du mois {} : {e}", m.time_bucket);
                        self.compter(0, u64::from(m.count));
                        continue;
                    }
                };
                let mut a_telecharger = Vec::new();
                let mut deja = 0;
                for (i, id) in detail.id.iter().enumerate() {
                    let version = detail.thumbhash.get(i).cloned().flatten().unwrap_or_default();
                    if cache.est_present(id, TailleImage::Vignette, &version) {
                        deja += 1;
                    } else {
                        a_telecharger.push((id.clone(), version));
                    }
                }
                self.compter(deja, 0);

                let mut derniere_emission = Instant::now();
                let mut flux = futures_util::stream::iter(a_telecharger)
                    .map(|(id, version)| async move {
                        self.attendre_pause(generation).await;
                        if self.annule(generation) {
                            return None;
                        }
                        Some(cache.obtenir(client, &id, TailleImage::Vignette, &version).await)
                    })
                    .buffer_unordered(PARALLELE);
                while let Some(resultat) = flux.next().await {
                    match resultat {
                        None => return,
                        Some(Ok(_)) => self.compter(1, 0),
                        // Not rendered yet on the server: retried on the next pass.
                        Some(Err(ErreurCache::Introuvable)) => self.compter(0, 1),
                        Some(Err(ErreurCache::Autre(e))) => {
                            tracing::debug!("préchargement : {e}");
                            self.compter(0, 1);
                        }
                    }
                    if derniere_emission.elapsed() > Duration::from_millis(300) {
                        self.emettre();
                        derniere_emission = Instant::now();
                    }
                }
                self.emettre();
            }
        }
    }
}
