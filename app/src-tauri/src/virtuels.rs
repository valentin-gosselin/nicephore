//! Virtual folders of the connected account: a selection of the server
//! shown as a folder, each original downloaded when a program opens it.
//! The list is kept per account (`virtuels-<account>.json`); folders are
//! mounted at connection, refreshed every 5 minutes, unmounted on exit.

use api_immich::Selection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};

use crate::Etat;

/// Default size of the originals cache.
pub const BUDGET_ORIGINAUX: u64 = 20 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DossierVirtuel {
    pub id: i64,
    pub point: String,
    pub selection: Selection,
    pub rangement: envoi::Rangement,
    /// RAW files left out.
    #[serde(default)]
    pub sans_raw: bool,
}

/// The assets of a folder, RAW files left out when asked.
pub(crate) fn filtrer(dossier: &DossierVirtuel, mut assets: Vec<api_immich::AssetDistant>) -> Vec<api_immich::AssetDistant> {
    if dossier.sans_raw {
        assets.retain(|a| !envoi::analyse::est_raw(&a.nom_fichier));
    }
    assets
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VueVirtuel {
    #[serde(flatten)]
    dossier: DossierVirtuel,
    monte: bool,
    fichiers: usize,
    erreur: Option<String>,
}

/// Mounted folders and their state, for the connected account.
#[derive(Default)]
pub struct Virtuels {
    compte: Mutex<Option<String>>,
    #[cfg(target_os = "linux")]
    montages: Mutex<HashMap<i64, Arc<virtuel::Montage>>>,
    #[cfg(windows)]
    montages: Mutex<HashMap<i64, virtuel::windows::MontageWindows>>,
    erreurs: Mutex<HashMap<i64, String>>,
}

fn fichier(dossier_config: &Path, compte: &str) -> PathBuf {
    dossier_config.join(format!("virtuels-{compte}.json"))
}

pub(crate) fn lire(dossier_config: &Path, compte: &str) -> Vec<DossierVirtuel> {
    std::fs::read_to_string(fichier(dossier_config, compte))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn ecrire(dossier_config: &Path, compte: &str, liste: &[DossierVirtuel]) -> Result<(), String> {
    std::fs::create_dir_all(dossier_config).map_err(|e| e.to_string())?;
    let chemin = fichier(dossier_config, compte);
    let temporaire = chemin.with_extension("json.tmp");
    std::fs::write(&temporaire, serde_json::to_string_pretty(liste).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&temporaire, chemin).map_err(|e| e.to_string())
}

pub(crate) fn rangement(r: envoi::Rangement) -> virtuel::Rangement {
    match r {
        envoi::Rangement::Plat => virtuel::Rangement::Plat,
        envoi::Rangement::AnneeMois => virtuel::Rangement::AnneeMois,
    }
}

/// Linux, installed mode: the folders are served by the service process
/// (`service_virtuel`), so they outlive the app. Portable mode and the
/// benchmark keep them in the app.
#[cfg(target_os = "linux")]
fn par_service(etat: &Etat) -> bool {
    // The write scenario tests the service too, in its private config.
    let banc = std::env::var_os("NICEPHORE_BANC_DOSSIER").is_some() && std::env::var_os("NICEPHORE_BANC_SERVICE").is_none();
    cfg!(target_os = "linux") && etat.portable.is_none() && !banc
}

/// Waits (up to `secondes`) for the service to report on folder `id`.
#[cfg(target_os = "linux")]
async fn attendre_service(etat: &Etat, id: i64, secondes: u64, retire: bool) -> Option<crate::service_virtuel::EtatDossier> {
    for _ in 0..secondes * 4 {
        let rapport = crate::service_virtuel::lire_etat(&etat.dossier_config).and_then(|e| e.dossiers.get(&id).cloned());
        match (&rapport, retire) {
            (None, true) => return None,
            (Some(r), false) if r.monte || r.erreur.is_some() => return rapport,
            _ => {}
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    crate::service_virtuel::lire_etat(&etat.dossier_config).and_then(|e| e.dossiers.get(&id).cloned())
}

impl Virtuels {
    /// Unmounts everything (account change, logout, exit).
    pub fn demonter_tout(&self) {
        #[cfg(target_os = "linux")]
        for (_, montage) in self.montages.lock().expect("verrou des montages").drain() {
            montage.demonter();
        }
        // Windows: disconnecting keeps the placeholders (they open again at
        // the next connection).
        #[cfg(windows)]
        self.montages.lock().expect("verrou des montages").clear();
    }

    /// Logout: everything unmounted, no account.
    pub fn oublier(&self) {
        *self.compte.lock().expect("verrou du compte") = None;
        self.demonter_tout();
    }

    /// Mounts the folders of `compte` (after a connection).
    pub fn activer(&self, app: &AppHandle, compte: &str) {
        {
            let mut actuel = self.compte.lock().expect("verrou du compte");
            if actuel.as_deref() == Some(compte) {
                return;
            }
            *actuel = Some(compte.to_string());
        }
        self.demonter_tout();
        self.erreurs.lock().expect("verrou des erreurs").clear();
        #[cfg(target_os = "linux")]
        {
            let etat = app.state::<Etat>();
            if par_service(&etat) {
                // The service follows the active account; it only has to run.
                if !lire(&etat.dossier_config, compte).is_empty() {
                    if let Err(e) = crate::service_virtuel::demarrer() {
                        tracing::warn!("{e}");
                    }
                    let _ = crate::service_virtuel::definir_demarrage(true);
                }
                return;
            }
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let etat = app.state::<Etat>();
            let Some(compte) = etat.virtuels.compte.lock().expect("verrou du compte").clone() else { return };
            for dossier in lire(&etat.dossier_config, &compte) {
                etat.virtuels.monter(&etat, &dossier).await;
            }
        });
    }

    async fn monter(&self, etat: &Etat, dossier: &DossierVirtuel) {
        let resultat = self.essayer_monter(etat, dossier).await;
        let mut erreurs = self.erreurs.lock().expect("verrou des erreurs");
        match resultat {
            Ok(()) => {
                erreurs.remove(&dossier.id);
            }
            Err(e) => {
                tracing::warn!("dossier virtuel {} : {e}", dossier.point);
                erreurs.insert(dossier.id, e);
            }
        }
    }

    #[cfg(target_os = "linux")]
    async fn essayer_monter(&self, etat: &Etat, dossier: &DossierVirtuel) -> Result<(), String> {
        let client = etat.client()?;
        let assets = filtrer(dossier, client.assets_de_selection(&dossier.selection).await.map_err(|e| e.to_string())?);
        let point = PathBuf::from(&dossier.point);
        let cache = Arc::clone(&etat.originaux);
        let runtime = tokio::runtime::Handle::current();
        let r = rangement(dossier.rangement);
        let montage = tauri::async_runtime::spawn_blocking(move || virtuel::monter(&point, &assets, r, cache, client, runtime))
            .await
            .map_err(|e| e.to_string())??;
        self.montages.lock().expect("verrou des montages").insert(dossier.id, Arc::new(montage));
        Ok(())
    }

    #[cfg(windows)]
    async fn essayer_monter(&self, etat: &Etat, dossier: &DossierVirtuel) -> Result<(), String> {
        let client = etat.client()?;
        let assets = filtrer(dossier, client.assets_de_selection(&dossier.selection).await.map_err(|e| e.to_string())?);
        let source = Arc::new(virtuel::SourceCache {
            cache: Arc::clone(&etat.originaux),
            client,
            runtime: tokio::runtime::Handle::current(),
        });
        let point = PathBuf::from(&dossier.point);
        let r = rangement(dossier.rangement);
        let montage = tauri::async_runtime::spawn_blocking(move || virtuel::windows::monter(&point, &assets, r, source))
            .await
            .map_err(|e| e.to_string())??;
        self.montages.lock().expect("verrou des montages").insert(dossier.id, montage);
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    async fn essayer_monter(&self, _etat: &Etat, _dossier: &DossierVirtuel) -> Result<(), String> {
        Err("les fichiers virtuels ne sont pas pris en charge sur ce système".into())
    }

    /// Refreshes the content of every mounted folder.
    pub async fn actualiser(&self, etat: &Etat) {
        let Some(compte) = self.compte.lock().expect("verrou du compte").clone() else { return };
        let Ok(client) = etat.client() else { return };
        for dossier in lire(&etat.dossier_config, &compte) {
            #[cfg(target_os = "linux")]
            {
                let montage = self.montages.lock().expect("verrou des montages").get(&dossier.id).cloned();
                match (montage, client.assets_de_selection(&dossier.selection).await) {
                    (Some(m), Ok(assets)) => m.actualiser(&filtrer(&dossier, assets)),
                    (None, _) => self.monter(etat, &dossier).await,
                    (Some(_), Err(e)) => tracing::info!("actualisation de {} : {e}", dossier.point),
                }
            }
            #[cfg(windows)]
            {
                let deja = self.montages.lock().expect("verrou des montages").contains_key(&dossier.id);
                match client.assets_de_selection(&dossier.selection).await {
                    Ok(assets) if deja => {
                        if let Some(m) = self.montages.lock().expect("verrou des montages").get(&dossier.id)
                            && let Err(e) = m.actualiser(&filtrer(&dossier, assets))
                        {
                            tracing::info!("actualisation de {} : {e}", dossier.point);
                        }
                    }
                    _ if !deja => self.monter(etat, &dossier).await,
                    Ok(_) => {}
                    Err(e) => tracing::info!("actualisation de {} : {e}", dossier.point),
                }
            }
            #[cfg(not(any(target_os = "linux", windows)))]
            let _ = (&client, dossier);
        }
    }
}

/// Every 5 minutes, the mounted folders follow the server.
pub fn actualiser_regulierement(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            let etat = app.state::<Etat>();
            etat.virtuels.actualiser(&etat).await;
        }
    });
}

fn compte_actif(etat: &Etat) -> Result<String, String> {
    etat.virtuels.compte.lock().expect("verrou du compte").clone().ok_or_else(|| "non connecté".into())
}

#[tauri::command(async)]
pub fn virtuels(etat: State<'_, Etat>) -> Result<Vec<VueVirtuel>, String> {
    let compte = compte_actif(&etat)?;
    #[cfg(target_os = "linux")]
    if par_service(&etat) {
        let service = crate::service_virtuel::lire_etat(&etat.dossier_config).filter(|e| e.compte.as_deref() == Some(compte.as_str()));
        return Ok(lire(&etat.dossier_config, &compte)
            .into_iter()
            .map(|dossier| {
                let rapport = service.as_ref().and_then(|e| e.dossiers.get(&dossier.id).cloned());
                VueVirtuel {
                    monte: rapport.as_ref().is_some_and(|r| r.monte),
                    fichiers: rapport.as_ref().map_or(0, |r| r.fichiers),
                    erreur: match (&service, rapport) {
                        (None, _) => Some("service des dossiers virtuels arrêté : relancez Nicéphore".into()),
                        (_, Some(r)) => r.erreur,
                        _ => None,
                    },
                    dossier,
                }
            })
            .collect());
    }
    let erreurs = etat.virtuels.erreurs.lock().expect("verrou des erreurs").clone();
    Ok(lire(&etat.dossier_config, &compte)
        .into_iter()
        .map(|dossier| {
            #[cfg(target_os = "linux")]
            let montage = etat.virtuels.montages.lock().expect("verrou des montages").get(&dossier.id).cloned();
            #[cfg(target_os = "linux")]
            let (monte, fichiers) = montage.map_or((false, 0), |m| (true, m.fichiers()));
            #[cfg(windows)]
            let (monte, fichiers) = etat
                .virtuels
                .montages
                .lock()
                .expect("verrou des montages")
                .get(&dossier.id)
                .map_or((false, 0), |m| (true, m.fichiers()));
            #[cfg(not(any(target_os = "linux", windows)))]
            let (monte, fichiers) = (false, 0);
            VueVirtuel { erreur: erreurs.get(&dossier.id).cloned(), dossier, monte, fichiers }
        })
        .collect())
}

/// Adds a virtual folder on `point` (an empty folder) and mounts it.
#[tauri::command]
pub async fn ajouter_virtuel(
    etat: State<'_, Etat>,
    point: String,
    selection: Selection,
    rangement: envoi::Rangement,
    sans_raw: Option<bool>,
) -> Result<(), String> {
    if !cfg!(any(target_os = "linux", windows)) {
        return Err("les fichiers virtuels ne sont pas pris en charge sur ce système".into());
    }
    let compte = compte_actif(&etat)?;
    let chemin = Path::new(&point);
    if !chemin.is_absolute() || !chemin.is_dir() {
        return Err("choisissez un dossier existant".into());
    }
    if std::fs::read_dir(chemin).map_err(|e| e.to_string())?.next().is_some() {
        return Err("choisissez un dossier vide : ses fichiers seraient masqués".into());
    }
    let mut liste = lire(&etat.dossier_config, &compte);
    if liste.iter().any(|d| Path::new(&d.point) == chemin) {
        return Err("ce dossier montre déjà une sélection".into());
    }
    let dossier = DossierVirtuel { id: chrono_id(), point, selection, rangement, sans_raw: sans_raw.unwrap_or(false) };
    liste.push(dossier.clone());
    ecrire(&etat.dossier_config, &compte, &liste)?;
    #[cfg(target_os = "linux")]
    if par_service(&etat) {
        crate::service_virtuel::demarrer()?;
        crate::service_virtuel::definir_demarrage(true)?;
        return match attendre_service(&etat, dossier.id, 30, false).await {
            Some(r) if r.monte => Ok(()),
            Some(r) => Err(r.erreur.unwrap_or_else(|| "montage impossible".into())),
            None => Err("le service des dossiers virtuels ne répond pas".into()),
        };
    }
    etat.virtuels.monter(&etat, &dossier).await;
    match etat.virtuels.erreurs.lock().expect("verrou des erreurs").get(&dossier.id) {
        Some(e) => Err(e.clone()),
        None => Ok(()),
    }
}

fn chrono_id() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Unmounts and forgets a virtual folder (the cached originals stay).
#[tauri::command]
pub async fn retirer_virtuel(etat: State<'_, Etat>, id: i64) -> Result<(), String> {
    let compte = compte_actif(&etat)?;
    #[cfg(target_os = "linux")]
    if par_service(&etat) {
        let liste: Vec<DossierVirtuel> = lire(&etat.dossier_config, &compte).into_iter().filter(|d| d.id != id).collect();
        ecrire(&etat.dossier_config, &compte, &liste)?;
        if liste.is_empty() {
            crate::service_virtuel::definir_demarrage(false)?;
        }
        attendre_service(&etat, id, 10, true).await;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    if let Some(m) = etat.virtuels.montages.lock().expect("verrou des montages").remove(&id) {
        m.demonter();
    }
    // Windows: the folder leaves Explorer's cloud folders.
    #[cfg(windows)]
    if let Some(m) = etat.virtuels.montages.lock().expect("verrou des montages").remove(&id)
        && let Err(e) = m.retirer()
    {
        tracing::warn!("retrait du dossier virtuel : {e}");
    }
    let liste: Vec<DossierVirtuel> = lire(&etat.dossier_config, &compte).into_iter().filter(|d| d.id != id).collect();
    ecrire(&etat.dossier_config, &compte, &liste)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OccupationOriginaux {
    octets: u64,
    fichiers: u64,
    budget: u64,
}

#[tauri::command(async)]
pub fn cache_originaux(etat: State<'_, Etat>) -> Result<OccupationOriginaux, String> {
    let (octets, fichiers) = etat.originaux.occupation();
    Ok(OccupationOriginaux { octets, fichiers, budget: BUDGET_ORIGINAUX })
}

#[tauri::command(async)]
pub fn vider_cache_originaux(etat: State<'_, Etat>) -> Result<(), String> {
    etat.originaux.vider().map_err(|e| e.to_string())
}
