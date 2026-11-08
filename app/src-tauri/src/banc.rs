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
