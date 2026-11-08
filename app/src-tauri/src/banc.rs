//! Benchmark mode (development): enabled when `NICEPHORE_BANC_DOSSIER` points
//! to a directory receiving screenshots and measurements. The UI then runs a
//! scripted session and reports through these commands.

use std::sync::Mutex;
use std::time::Duration;

fn dossier_banc() -> Option<std::path::PathBuf> {
    std::env::var_os("NICEPHORE_BANC_DOSSIER").map(Into::into)
}

#[tauri::command]
pub fn mode_banc() -> bool {
    dossier_banc().is_some()
}

/// Scenario to run: empty for the read-only benchmark, `ecriture` for the
/// in-app write scenario (disposable server only, checked by the UI too).
#[tauri::command]
pub fn scenario_banc() -> String {
    std::env::var("NICEPHORE_BANC_SCENARIO").unwrap_or_default()
}

/// Folder of generated photos prepared by `scripts/banc.sh --jetable` for
/// the upload scenario.
#[tauri::command]
pub fn dossier_photos_banc() -> Option<String> {
    let dossier = dossier_banc()?.join("photos");
    dossier.is_dir().then(|| dossier.to_string_lossy().into_owned())
}

/// Asks the main window to close, like the title bar button would.
#[tauri::command]
pub fn fermer_fenetre(app: tauri::AppHandle) {
    use tauri::Manager;
    if mode_banc() && let Some(fenetre) = app.get_webview_window("main") {
        let _ = fenetre.close();
    }
}

#[tauri::command]
pub fn fenetre_visible(app: tauri::AppHandle) -> bool {
    use tauri::Manager;
    app.get_webview_window("main").and_then(|f| f.is_visible().ok()).unwrap_or(false)
}

/// What the tray's "Ouvrir Nicéphore" does.
#[tauri::command]
pub fn montrer_fenetre(app: tauri::AppHandle) {
    crate::zone_notification::montrer(&app);
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifiantsBanc {
    url: String,
    email: String,
    mot_de_passe: String,
    email2: String,
    mot_de_passe2: String,
}

/// Login of the disposable test server (`IMMICH_TEST_*`, written by its
/// seeding script), for the portable scenario. Local servers only.
#[tauri::command]
pub fn identifiants_banc() -> Option<IdentifiantsBanc> {
    dossier_banc()?;
    let url = std::env::var("IMMICH_TEST_URL").ok()?;
    if !(url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost")) {
        return None;
    }
    Some(IdentifiantsBanc {
        url,
        email: std::env::var("IMMICH_TEST_EMAIL").ok()?,
        mot_de_passe: std::env::var("IMMICH_TEST_PASSWORD").ok()?,
        email2: std::env::var("IMMICH_TEST_EMAIL_2").unwrap_or_default(),
        mot_de_passe2: std::env::var("IMMICH_TEST_PASSWORD_2").unwrap_or_default(),
    })
}

#[tauri::command]
pub fn journal(message: String) {
    tracing::info!(target: "banc", "{message}");
}

/// Takes a screenshot of the (virtual) screen with Spectacle.
#[tauri::command]
pub async fn capture(nom: String) -> Result<(), String> {
    let Some(dossier) = dossier_banc() else { return Err("mode banc inactif".into()) };
    if nom.is_empty() || !nom.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("nom de capture invalide".into());
    }
    let chemin = dossier.join(format!("{nom}.png"));
    let statut = tokio::process::Command::new("spectacle")
        .args(["--background", "--nonotify", "--fullscreen", "--output"])
        .arg(&chemin)
        .status()
        .await
        .map_err(|e| e.to_string())?;
    if statut.success() { Ok(()) } else { Err(format!("spectacle : {statut}")) }
}

#[tauri::command]
pub fn quitter(app: tauri::AppHandle) {
    if mode_banc() {
        app.exit(0);
    }
}

/// Count, total and worst time of the image protocol since the last read.
static STATS_IMAGES: Mutex<(u64, u64, u64)> = Mutex::new((0, 0, 0));

pub fn compter_image(duree: Duration) {
    let ms = duree.as_millis() as u64;
    let mut stats = STATS_IMAGES.lock().expect("verrou des stats");
    *stats = (stats.0 + 1, stats.1 + ms, stats.2.max(ms));
}

#[tauri::command]
pub fn stats_images() -> String {
    let mut stats = STATS_IMAGES.lock().expect("verrou des stats");
    let (nombre, total, max) = *stats;
    *stats = (0, 0, 0);
    let moyenne = total.checked_div(nombre).unwrap_or(0);
    format!("{nombre} images servies, {moyenne} ms en moyenne, {max} ms au pire")
}
