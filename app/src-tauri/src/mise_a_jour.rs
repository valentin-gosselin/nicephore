//! In-app updates, signed (minisign) and checked against the public key
//! given at release time. Only the AppImage (Linux) and the installed
//! Windows version replace themselves; an rpm, a local build or the
//! portable Windows exe only learn that a new version exists.

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::Etat;

/// Where releases are published, for versions that cannot update themselves.
pub const PAGE_DES_VERSIONS: &str = "https://github.com/valentin-gosselin/nicephore/releases";

#[derive(Default)]
pub struct MisesAJour {
    trouvee: Mutex<Option<Update>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfosMiseAJour {
    pub version_actuelle: String,
    /// Signed updates are set up in this build (public key given at release).
    pub possibles: bool,
    /// This copy can replace itself (AppImage, installed Windows version).
    pub installable: bool,
    pub page_des_versions: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDisponible {
    pub version: String,
    pub notes: Option<String>,
    pub date: Option<String>,
}

fn possibles(app: &AppHandle) -> bool {
    app.config()
        .plugins
        .0
        .get("updater")
        .and_then(|u| u.get("pubkey"))
        .and_then(|k| k.as_str())
        .is_some_and(|k| !k.trim().is_empty())
}

fn installable(app: &AppHandle) -> bool {
    if app.try_state::<Etat>().is_some_and(|e| e.portable.is_some()) {
        // The portable copy is replaced by hand (an installer would put
        // Nicéphore in the profile).
        return false;
    }
    if cfg!(target_os = "linux") {
        std::env::var_os("APPIMAGE").is_some()
    } else {
        cfg!(windows)
    }
}

#[tauri::command]
pub fn infos_mise_a_jour(app: AppHandle) -> InfosMiseAJour {
    InfosMiseAJour {
        version_actuelle: app.package_info().version.to_string(),
        possibles: possibles(&app),
        installable: installable(&app),
        page_des_versions: PAGE_DES_VERSIONS,
    }
}

/// Looks for a newer version (signature checked at install time).
#[tauri::command]
pub async fn chercher_mise_a_jour(app: AppHandle, etat: State<'_, MisesAJour>) -> Result<Option<VersionDisponible>, String> {
    if !possibles(&app) {
        return Err("mises à jour non configurées dans cette version (compilée localement)".into());
    }
    let mise_a_jour = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    let disponible = mise_a_jour.as_ref().map(|m| VersionDisponible {
        version: m.version.clone(),
        notes: m.body.clone(),
        date: m.date.map(|d| d.to_string()),
    });
    *etat.trouvee.lock().expect("verrou de la mise à jour") = mise_a_jour;
    Ok(disponible)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progression {
    recus: u64,
    total: Option<u64>,
}

/// Downloads, checks the signature, replaces this copy (the previous
/// AppImage is kept beside it as `.ancienne`), then restarts.
#[tauri::command]
pub async fn installer_mise_a_jour(app: AppHandle, etat: State<'_, MisesAJour>) -> Result<(), String> {
    if !installable(&app) {
        return Err(format!("cette copie de Nicéphore se met à jour à la main : {PAGE_DES_VERSIONS}"));
    }
    let mise_a_jour = etat.trouvee.lock().expect("verrou de la mise à jour").take().ok_or("aucune mise à jour trouvée")?;
    garder_l_ancienne();
    let mut recus = 0u64;
    let emetteur = app.clone();
    mise_a_jour
        .download_and_install(
            move |morceau, total| {
                recus += morceau as u64;
                let _ = emetteur.emit("mise-a-jour", Progression { recus, total });
            },
            || {},
        )
        .await
        .map_err(|e| match e {
            tauri_plugin_updater::Error::Minisign(_) | tauri_plugin_updater::Error::SignatureUtf8(_) => {
                "signature invalide : mise à jour refusée, rien n'a été modifié".to_string()
            }
            e if e.to_string().contains("signature") => {
                "signature invalide : mise à jour refusée, rien n'a été modifié".to_string()
            }
            e => e.to_string(),
        })?;
    tracing::info!("mise à jour {} installée, redémarrage", mise_a_jour.version);
    app.restart();
}

/// AppImage: copy of the running version, to go back by hand if needed.
fn garder_l_ancienne() {
    if let Some(appimage) = std::env::var_os("APPIMAGE") {
        let chemin = std::path::PathBuf::from(appimage);
        let ancienne = chemin.with_extension("AppImage.ancienne");
        if let Err(e) = std::fs::copy(&chemin, &ancienne) {
            tracing::warn!("copie de l'ancienne version impossible : {e}");
        }
    }
}

/// Startup check (once, a little after launch): tells the interface when a
/// version is waiting.
pub fn verifier_au_demarrage(app: AppHandle) {
    if !possibles(&app) || std::env::var_os("NICEPHORE_BANC_DOSSIER").is_some() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        let etat = app.state::<MisesAJour>();
        match chercher_mise_a_jour(app.clone(), etat).await {
            Ok(Some(version)) => {
                let _ = app.emit("version-disponible", version);
            }
            Ok(None) => {}
            Err(e) => tracing::info!("recherche de mise à jour : {e}"),
        }
    });
}
