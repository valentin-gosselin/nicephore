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
    erreurs: Mutex<HashMap<i64, String>>,
}

fn fichier(dossier_config: &Path, compte: &str) -> PathBuf {
    dossier_config.join(format!("virtuels-{compte}.json"))
}

fn lire(dossier_config: &Path, compte: &str) -> Vec<DossierVirtuel> {
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

fn rangement(r: envoi::Rangement) -> virtuel::Rangement {
    match r {
        envoi::Rangement::Plat => virtuel::Rangement::Plat,
        envoi::Rangement::AnneeMois => virtuel::Rangement::AnneeMois,
    }
}

impl Virtuels {
    /// Unmounts everything (account change, logout, exit).
    pub fn demonter_tout(&self) {
        #[cfg(target_os = "linux")]
        for (_, montage) in self.montages.lock().expect("verrou des montages").drain() {
            montage.demonter();
        }
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
        let assets = client.assets_de_selection(&dossier.selection).await.map_err(|e| e.to_string())?;
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

    #[cfg(not(target_os = "linux"))]
    async fn essayer_monter(&self, _etat: &Etat, _dossier: &DossierVirtuel) -> Result<(), String> {
        Err("les fichiers virtuels ne sont pas encore pris en charge sur ce système".into())
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
                    (Some(m), Ok(assets)) => m.actualiser(&assets),
                    (None, _) => self.monter(etat, &dossier).await,
                    (Some(_), Err(e)) => tracing::info!("actualisation de {} : {e}", dossier.point),
                }
            }
            #[cfg(not(target_os = "linux"))]
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

#[tauri::command]
pub fn virtuels(etat: State<'_, Etat>) -> Result<Vec<VueVirtuel>, String> {
    let compte = compte_actif(&etat)?;
    let erreurs = etat.virtuels.erreurs.lock().expect("verrou des erreurs").clone();
    Ok(lire(&etat.dossier_config, &compte)
        .into_iter()
        .map(|dossier| {
            #[cfg(target_os = "linux")]
            let montage = etat.virtuels.montages.lock().expect("verrou des montages").get(&dossier.id).cloned();
            #[cfg(target_os = "linux")]
            let (monte, fichiers) = montage.map_or((false, 0), |m| (true, m.fichiers()));
            #[cfg(not(target_os = "linux"))]
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
) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("les fichiers virtuels ne sont pas encore pris en charge sur ce système".into());
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
    let dossier = DossierVirtuel { id: chrono_id(), point, selection, rangement };
    liste.push(dossier.clone());
    ecrire(&etat.dossier_config, &compte, &liste)?;
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
pub fn retirer_virtuel(etat: State<'_, Etat>, id: i64) -> Result<(), String> {
    let compte = compte_actif(&etat)?;
    #[cfg(target_os = "linux")]
    if let Some(m) = etat.virtuels.montages.lock().expect("verrou des montages").remove(&id) {
        m.demonter();
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

#[tauri::command]
pub fn cache_originaux(etat: State<'_, Etat>) -> OccupationOriginaux {
    let (octets, fichiers) = etat.originaux.occupation();
    OccupationOriginaux { octets, fichiers, budget: BUDGET_ORIGINAUX }
}

#[tauri::command]
pub fn vider_cache_originaux(etat: State<'_, Etat>) -> Result<(), String> {
    etat.originaux.vider().map_err(|e| e.to_string())
}
