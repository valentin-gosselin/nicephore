//! Commands of the "Envoi" page: upload rules, engine state, log. The
//! engine itself lives in the `envoi` crate.

use envoi::{Bilan, Cible, EntreeJournal, EtatEnvoi, FichierEnEchec, Moteur, Regle};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

use crate::Etat;

/// Starts the engine and relays its state to the interface (at most about
/// five times a second).
pub fn demarrer(moteur: &Arc<Moteur>, app: AppHandle) {
    tauri::async_runtime::spawn(moteur.lancer());
    let mut etat = moteur.etat();
    tauri::async_runtime::spawn(async move {
        while etat.changed().await.is_ok() {
            let instantane = etat.borrow_and_update().clone();
            let _ = app.emit("envoi", instantane);
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });
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
    if let Some(e) = &etat.erreur_base_envoi {
        return Err(e.clone());
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
    if let Some(e) = &etat.erreur_base_envoi {
        return Err(e.clone());
    }
    let chemin = Path::new(&dossier);
    if !chemin.is_absolute() || !chemin.is_dir() {
        return Err("choisissez un dossier existant".into());
    }
    let regle = etat.envoi.base().ajouter_regle(chemin, &cible).map_err(erreur)?;
    etat.envoi.regles_changees();
    Ok(regle)
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
