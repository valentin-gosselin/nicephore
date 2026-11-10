//! Commands of the "Envoi" page: upload rules, engine state, log. The
//! engine itself lives in the `envoi` crate.

use envoi::{Bilan, Cible, EntreeJournal, EtatEnvoi, FichierEnEchec, Moteur, Regle};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::Etat;
use crate::reglages::{self, Reglages};

/// Starts the engine and relays its state to the interface and the tray
/// (at most about five times a second), with a notification at the end of
/// a pass that sent files, or when sending stops on an error.
pub fn demarrer(moteur: &Arc<Moteur>, app: AppHandle) {
    tauri::async_runtime::spawn(moteur.lancer());
    let mut etat = moteur.etat();
    tauri::async_runtime::spawn(async move {
        let mut dernier_passage = None;
        let mut derniere_erreur: Option<String> = None;
        while etat.changed().await.is_ok() {
            let instantane = etat.borrow_and_update().clone();
            let _ = app.emit("envoi", instantane.clone());
            if let Some(etat_app) = app.try_state::<Etat>() {
                if let Some(zone) = &etat_app.zone {
                    zone.mettre_a_jour(&instantane);
                }
                let notifier = etat_app.reglages.read().expect("verrou des réglages").notifications && !fenetre_au_premier_plan(&app);
                if instantane.dernier_passage != dernier_passage {
                    dernier_passage = instantane.dernier_passage;
                    if notifier && instantane.envoyes_passage > 0 {
                        let n = instantane.envoyes_passage;
                        let texte = if n > 1 { format!("{n} photos et vidéos envoyées vers Immich") } else { "1 photo envoyée vers Immich".into() };
                        notification(&app, &texte);
                    }
                }
                if instantane.erreur != derniere_erreur {
                    derniere_erreur = instantane.erreur.clone();
                    if notifier && let Some(erreur) = &derniere_erreur {
                        notification(&app, &format!("Envoi arrêté : {erreur}"));
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });
}

fn fenetre_au_premier_plan(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|f| f.is_visible().unwrap_or(false) && f.is_focused().unwrap_or(false))
}

/// Shows a desktop notification without waiting: the D-Bus call can hang
/// until its timeout when no notification server answers.
fn notification(app: &AppHandle, texte: &str) {
    use tauri_plugin_notification::NotificationExt;
    let (app, texte) = (app.clone(), texte.to_string());
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(e) = app.notification().builder().title("Nicéphore").body(texte).show() {
            tracing::warn!("notification impossible : {e}");
        }
    });
}

/// First time the window is closed into the tray: say so once.
pub fn prevenir_arriere_plan(app: &AppHandle) {
    static DEJA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if DEJA.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    if app.try_state::<Etat>().is_some_and(|e| e.reglages.read().expect("verrou des réglages").notifications) {
        notification(app, "Nicéphore continue d’envoyer vos dossiers depuis la zone de notification.");
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReglagesVue {
    #[serde(flatten)]
    reglages: Reglages,
    demarrage_auto: bool,
    /// A tray icon could be created (otherwise closing quits).
    zone_disponible: bool,
    /// Portable mode: no start with the session.
    portable: bool,
}

#[tauri::command]
pub fn reglages_app(etat: State<'_, Etat>) -> ReglagesVue {
    ReglagesVue {
        reglages: etat.reglages.read().expect("verrou des réglages").clone(),
        demarrage_auto: reglages::demarrage_auto(),
        zone_disponible: etat.zone.is_some(),
        portable: etat.portable.is_some(),
    }
}

#[tauri::command(async)]
pub fn definir_reglages_app(etat: State<'_, Etat>, reglages: Reglages) -> Result<(), String> {
    reglages::enregistrer(&etat.dossier_config, &reglages)?;
    *etat.reglages.write().expect("verrou des réglages") = reglages;
    // A condition may have just been lifted.
    etat.envoi.reveiller();
    Ok(())
}

#[tauri::command(async)]
pub fn definir_demarrage_auto(etat: State<'_, Etat>, actif: bool) -> Result<(), String> {
    if actif && etat.portable.is_some() {
        return Err("pas de démarrage avec la session en mode portable".into());
    }
    reglages::definir_demarrage_auto(actif)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegleVue {
    #[serde(flatten)]
    regle: Regle,
    bilan: Bilan,
    /// The folder is missing or unreadable right now.
    dossier_absent: bool,
}

fn erreur(e: envoi::Erreur) -> String {
    e.to_string()
}

#[tauri::command(async)]
pub fn regles_envoi(etat: State<'_, Etat>) -> Result<Vec<RegleVue>, String> {
    if let Some(e) = etat.erreur_base_envoi.read().expect("verrou de l'erreur").clone() {
        return Err(e);
    }
    let base = etat.envoi.base();
    let mut bilans = base.bilans().map_err(erreur)?;
    Ok(base
        .regles()
        .map_err(erreur)?
        .into_iter()
        .map(|regle| RegleVue {
            bilan: bilans.remove(&regle.id).unwrap_or_default(),
            dossier_absent: !Path::new(&regle.dossier).is_dir(),
            regle,
        })
        .collect())
}

#[tauri::command(async)]
pub fn ajouter_regle_envoi(etat: State<'_, Etat>, dossier: String, cible: Cible, sans_raw: Option<bool>) -> Result<Regle, String> {
    if let Some(e) = etat.erreur_base_envoi.read().expect("verrou de l'erreur").clone() {
        return Err(e);
    }
    let chemin = Path::new(&dossier);
    if !chemin.is_absolute() || !chemin.is_dir() {
        return Err("choisissez un dossier existant".into());
    }
    let regle = etat.envoi.base().ajouter_regle(chemin, &cible).map_err(erreur)?;
    let regle = appliquer_sans_raw(&etat, regle, sans_raw)?;
    etat.envoi.regles_changees();
    Ok(regle)
}

/// Adds a download rule: a selection of the server copied into a folder.
#[tauri::command(async)]
pub fn ajouter_reception(
    etat: State<'_, Etat>,
    dossier: String,
    selection: api_immich::Selection,
    rangement: envoi::Rangement,
    sans_raw: Option<bool>,
) -> Result<Regle, String> {
    if let Some(e) = etat.erreur_base_envoi.read().expect("verrou de l'erreur").clone() {
        return Err(e);
    }
    let chemin = Path::new(&dossier);
    if !chemin.is_absolute() || !chemin.is_dir() {
        return Err("choisissez un dossier existant".into());
    }
    let regle = etat.envoi.base().ajouter_reception(chemin, &selection, rangement).map_err(erreur)?;
    let regle = appliquer_sans_raw(&etat, regle, sans_raw)?;
    etat.envoi.regles_changees();
    Ok(regle)
}

/// Adds a two-way rule between a folder and an album: an existing one
/// (`album`), or a new album named `nouvel_album`. The marker file is put
/// in the folder.
#[tauri::command]
pub async fn ajouter_double(
    etat: State<'_, Etat>,
    dossier: String,
    album: Option<String>,
    nouvel_album: Option<String>,
    miroir: bool,
    sans_raw: Option<bool>,
) -> Result<Regle, String> {
    if let Some(e) = etat.erreur_base_envoi.read().expect("verrou de l'erreur").clone() {
        return Err(e);
    }
    let chemin = Path::new(&dossier);
    if !chemin.is_absolute() || !chemin.is_dir() {
        return Err("choisissez un dossier existant".into());
    }
    let client = etat.client()?;
    let (id, nom) = match (album, nouvel_album) {
        (Some(id), _) => {
            let a = client.album(&id).await.map_err(|e| e.to_string())?;
            (a.id, a.album_name)
        }
        (None, Some(nom)) if !nom.trim().is_empty() => {
            let a = client.creer_album(nom.trim(), "", &[]).await.map_err(|e| e.to_string())?;
            (a.id, a.album_name)
        }
        _ => return Err("choisissez un album".into()),
    };
    envoi::poser_temoin(chemin).map_err(|e| format!("fichier témoin impossible à poser : {e}"))?;
    let regle = etat.envoi.base().ajouter_double(chemin, &id, &nom, miroir).map_err(erreur)?;
    let regle = appliquer_sans_raw(&etat, regle, sans_raw)?;
    etat.envoi.regles_changees();
    Ok(regle)
}

/// Adds a two-way rule between a folder and the library (or a period of
/// it): the folder is sent (`cible`: library or one album per folder), and
/// the photos of `selection` found nowhere in the folder come down into the
/// sub-folder `dossier_recus` (empty: the folder itself), by year and month.
#[tauri::command(async)]
pub fn ajouter_double_bibliotheque(
    etat: State<'_, Etat>,
    dossier: String,
    cible: Cible,
    selection: api_immich::Selection,
    dossier_recus: String,
    sans_raw: Option<bool>,
) -> Result<Regle, String> {
    if let Some(e) = etat.erreur_base_envoi.read().expect("verrou de l'erreur").clone() {
        return Err(e);
    }
    let chemin = Path::new(&dossier);
    if !chemin.is_absolute() || !chemin.is_dir() {
        return Err("choisissez un dossier existant".into());
    }
    let regle = etat.envoi.base().ajouter_double_bibliotheque(chemin, &cible, &selection, &dossier_recus).map_err(erreur)?;
    let regle = appliquer_sans_raw(&etat, regle, sans_raw)?;
    etat.envoi.regles_changees();
    Ok(regle)
}

/// RAW files left out from the start (before the first pass).
fn appliquer_sans_raw(etat: &Etat, regle: Regle, sans_raw: Option<bool>) -> Result<Regle, String> {
    if sans_raw != Some(true) {
        return Ok(regle);
    }
    let base = etat.envoi.base();
    base.definir_sans_raw(regle.id, true).map_err(erreur)?;
    base.regle(regle.id).map_err(erreur)
}

/// Includes or leaves out RAW files (DNG, ARW, CR2...) for a rule.
#[tauri::command(async)]
pub fn definir_sans_raw(etat: State<'_, Etat>, regle: i64, sans_raw: bool) -> Result<(), String> {
    etat.envoi.base().definir_sans_raw(regle, sans_raw).map_err(erreur)?;
    etat.envoi.regles_changees();
    Ok(())
}

/// Turns mirrored deletions on or off for a two-way rule.
#[tauri::command(async)]
pub fn definir_miroir(etat: State<'_, Etat>, regle: i64, miroir: bool) -> Result<(), String> {
    let base = etat.envoi.base();
    let r = base.regle(regle).map_err(erreur)?;
    if miroir {
        envoi::poser_temoin(Path::new(&r.dossier)).map_err(|e| format!("fichier témoin impossible à poser : {e}"))?;
    }
    base.definir_miroir(regle, miroir).map_err(erreur)?;
    drop(base);
    etat.envoi.regles_changees();
    Ok(())
}

#[tauri::command(async)]
pub fn suppressions_en_attente(etat: State<'_, Etat>, regle: i64) -> Result<Vec<envoi::Suppression>, String> {
    etat.envoi.base().suppressions(regle).map_err(erreur)
}

#[tauri::command(async)]
pub fn confirmer_suppressions(etat: State<'_, Etat>, regle: i64) -> Result<usize, String> {
    let n = etat.envoi.base().confirmer_suppressions(regle).map_err(erreur)?;
    etat.envoi.reveiller();
    Ok(n)
}

/// Refused deletions: what was deleted here comes back from the album, what
/// left the album goes back into it.
#[tauri::command(async)]
pub fn refuser_suppressions(etat: State<'_, Etat>, regle: i64) -> Result<usize, String> {
    let n = etat.envoi.base().refuser_suppressions(regle).map_err(erreur)?;
    etat.envoi.reveiller();
    Ok(n)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimationReception {
    fichiers: u64,
    octets: u64,
    /// Free space where the files would land.
    libre: Option<u64>,
}

/// What a selection would bring into a folder, shown before adding it.
#[tauri::command]
pub async fn estimer_reception(
    etat: State<'_, Etat>,
    selection: api_immich::Selection,
    dossier: Option<String>,
) -> Result<EstimationReception, String> {
    let assets = etat.client()?.assets_de_selection(&selection).await.map_err(|e| e.to_string())?;
    Ok(EstimationReception {
        fichiers: assets.len() as u64,
        octets: assets.iter().filter_map(|a| a.taille).sum(),
        libre: dossier.and_then(|d| fs4::available_space(d).ok()),
    })
}

/// Downloads again what was deleted here or given up.
#[tauri::command(async)]
pub fn recevoir_de_nouveau(etat: State<'_, Etat>, regle: i64) -> Result<usize, String> {
    let n = etat.envoi.base().recevoir_de_nouveau(regle).map_err(erreur)?;
    etat.envoi.reveiller();
    Ok(n)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimation {
    fichiers: u64,
    octets: u64,
    illisibles: u64,
}

/// What a folder would send (photos and videos the server accepts, before
/// knowing which are already there), shown before the folder is added.
#[tauri::command]
pub async fn estimer_dossier(etat: State<'_, Etat>, dossier: String) -> Result<Estimation, String> {
    let client = etat.client()?;
    let types = client.types_medias().await.map_err(|e| e.to_string())?;
    let inventaire = tauri::async_runtime::spawn_blocking(move || envoi::analyse::inventorier(Path::new(&dossier), &types))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("dossier illisible : {e}"))?;
    Ok(Estimation {
        fichiers: inventaire.fichiers.len() as u64,
        octets: inventaire.fichiers.iter().map(|f| f.taille).sum(),
        illisibles: inventaire.illisibles.len() as u64,
    })
}

#[tauri::command(async)]
pub fn activer_regle_envoi(etat: State<'_, Etat>, id: i64, actif: bool) -> Result<(), String> {
    etat.envoi.base().activer_regle(id, actif).map_err(erreur)?;
    etat.envoi.regles_changees();
    Ok(())
}

/// Forgets a rule: neither the local files nor the photos on the server
/// are touched.
#[tauri::command(async)]
pub fn supprimer_regle_envoi(etat: State<'_, Etat>, id: i64) -> Result<(), String> {
    etat.envoi.base().supprimer_regle(id).map_err(erreur)?;
    etat.envoi.regles_changees();
    Ok(())
}

#[tauri::command]
pub fn etat_envoi(etat: State<'_, Etat>) -> EtatEnvoi {
    etat.envoi.instantane()
}

#[tauri::command]
pub fn lancer_envoi(etat: State<'_, Etat>) {
    etat.envoi.reveiller();
}

#[tauri::command]
pub fn pause_envoi(etat: State<'_, Etat>, pause: bool) {
    etat.envoi.mettre_en_pause(pause);
}

#[tauri::command(async)]
pub fn journal_envoi(etat: State<'_, Etat>, limite: u32) -> Result<Vec<EntreeJournal>, String> {
    etat.envoi.base().journal(limite.min(1_000)).map_err(erreur)
}

#[tauri::command(async)]
pub fn echecs_envoi(etat: State<'_, Etat>, regle: i64) -> Result<Vec<FichierEnEchec>, String> {
    etat.envoi.base().echecs(regle).map_err(erreur)
}

#[tauri::command(async)]
pub fn reessayer_envoi(etat: State<'_, Etat>, regle: i64) -> Result<usize, String> {
    let n = etat.envoi.base().reessayer(regle).map_err(erreur)?;
    etat.envoi.reveiller();
    Ok(n)
}
