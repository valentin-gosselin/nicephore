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

#[tauri::command]
pub fn definir_reglages_app(etat: State<'_, Etat>, reglages: Reglages) -> Result<(), String> {
    reglages::enregistrer(&etat.dossier_config, &reglages)?;
    *etat.reglages.write().expect("verrou des réglages") = reglages;
    // A condition may have just been lifted.
    etat.envoi.reveiller();
    Ok(())
}

#[tauri::command]
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

#[tauri::command]
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

#[tauri::command]
pub fn ajouter_regle_envoi(etat: State<'_, Etat>, dossier: String, cible: Cible) -> Result<Regle, String> {
    if let Some(e) = etat.erreur_base_envoi.read().expect("verrou de l'erreur").clone() {
        return Err(e);
    }
    let chemin = Path::new(&dossier);
    if !chemin.is_absolute() || !chemin.is_dir() {
        return Err("choisissez un dossier existant".into());
    }
    let regle = etat.envoi.base().ajouter_regle(chemin, &cible).map_err(erreur)?;
    etat.envoi.regles_changees();
    Ok(regle)
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

#[tauri::command]
pub fn activer_regle_envoi(etat: State<'_, Etat>, id: i64, actif: bool) -> Result<(), String> {
    etat.envoi.base().activer_regle(id, actif).map_err(erreur)?;
    etat.envoi.regles_changees();
    Ok(())
}

/// Forgets a rule: neither the local files nor the photos on the server
/// are touched.
#[tauri::command]
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

#[tauri::command]
pub fn journal_envoi(etat: State<'_, Etat>, limite: u32) -> Result<Vec<EntreeJournal>, String> {
    etat.envoi.base().journal(limite.min(1_000)).map_err(erreur)
}

#[tauri::command]
pub fn echecs_envoi(etat: State<'_, Etat>, regle: i64) -> Result<Vec<FichierEnEchec>, String> {
    etat.envoi.base().echecs(regle).map_err(erreur)
}

#[tauri::command]
pub fn reessayer_envoi(etat: State<'_, Etat>, regle: i64) -> Result<usize, String> {
    let n = etat.envoi.base().reessayer(regle).map_err(erreur)?;
    etat.envoi.reveiller();
    Ok(n)
}
