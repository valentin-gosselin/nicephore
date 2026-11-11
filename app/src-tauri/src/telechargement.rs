//! Download of original files to a local folder, one at a time, with
//! progress events. Every file is written to a hidden temporary name, then
//! renamed: an interrupted download never leaves a truncated photo behind.

use api_immich::ClientImmich;
use futures_util::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progression {
    fait: usize,
    total: usize,
    nom: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bilan {
    reussis: usize,
    echecs: Vec<String>,
    dossier: String,
}

pub async fn telecharger(app: AppHandle, client: &ClientImmich, ids: &[String], dossier: &Path) -> Bilan {
    let mut reussis = 0;
    let mut echecs = Vec::new();
    for (index, id) in ids.iter().enumerate() {
        match telecharger_un(client, id, dossier).await {
            Ok(nom) => {
                reussis += 1;
                let _ = app.emit("telechargement", Progression { fait: index + 1, total: ids.len(), nom });
            }
            Err(e) => {
                tracing::warn!("téléchargement de {id} : {e}");
                echecs.push(format!("{id} : {e}"));
            }
        }
    }
    Bilan { reussis, echecs, dossier: dossier.display().to_string() }
}

async fn telecharger_un(client: &ClientImmich, id: &str, dossier: &Path) -> Result<String, String> {
    let infos = client.infos_asset(id).await.map_err(|e| e.to_string())?;
    let nom = assainir(infos["originalFileName"].as_str().unwrap_or_default(), id);
    let destination = nom_libre(dossier, &nom);
    let temporaire = dossier.join(format!(".{nom}.{id}.partiel"));

    let reponse = client.flux_original(id).await.map_err(|e| e.to_string())?;
    let resultat: Result<(), String> = async {
        let mut fichier = tokio::fs::File::create(&temporaire).await.map_err(|e| e.to_string())?;
        let mut flux = reponse.bytes_stream();
        while let Some(morceau) = flux.next().await {
            fichier.write_all(&morceau.map_err(|e| e.to_string())?).await.map_err(|e| e.to_string())?;
        }
        fichier.sync_all().await.map_err(|e| e.to_string())
    }
    .await;
    if let Err(e) = resultat {
        let _ = tokio::fs::remove_file(&temporaire).await;
        return Err(e);
    }
    tokio::fs::rename(&temporaire, &destination).await.map_err(|e| e.to_string())?;
    Ok(destination.file_name().unwrap_or_default().to_string_lossy().into_owned())
}

/// Keeps only a file name valid on Windows and Linux.
fn assainir(nom: &str, id: &str) -> String {
    // The last component, cut by hand: `Path` on Windows reads "a:b.jpg" as
    // a drive and would drop "a:".
    let nom = nom.rsplit(['/', '\\']).next().unwrap_or_default();
    let propre: String = nom
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let propre = propre.trim_matches(|c| c == '.' || c == ' ').to_string();
    if propre.is_empty() {
        return id.to_string();
    }
    // Names Windows reserves for devices, whatever the extension.
    let base = propre.split('.').next().unwrap_or_default().to_ascii_uppercase();
    let reserve = matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4 && (base.starts_with("COM") || base.starts_with("LPT")) && base.as_bytes()[3].is_ascii_digit());
    if reserve { format!("_{propre}") } else { propre }
}

/// `photo.jpg`, then `photo (1).jpg`, `photo (2).jpg`... never overwrites.
fn nom_libre(dossier: &Path, nom: &str) -> PathBuf {
    let candidat = dossier.join(nom);
    if !candidat.exists() {
        return candidat;
    }
    let chemin = Path::new(nom);
    let base = chemin.file_stem().unwrap_or_default().to_string_lossy();
    let extension = chemin.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (1..)
        .map(|n| dossier.join(format!("{base} ({n}){extension}")))
        .find(|c| !c.exists())
        .expect("un nom libre finit par exister")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noms_assainis() {
        assert_eq!(assainir("IMG_0001.JPG", "x"), "IMG_0001.JPG");
        assert_eq!(assainir("../../etc/passwd", "x"), "passwd");
        assert_eq!(assainir("a:b?.jpg", "x"), "a_b_.jpg");
        assert_eq!(assainir("", "abc"), "abc");
        assert_eq!(assainir("C:\\Users\\x\\y.jpg", "x"), "y.jpg");
        assert_eq!(assainir("nul.jpg", "x"), "_nul.jpg");
        assert_eq!(assainir("COM1", "x"), "_COM1");
        assert_eq!(assainir("console.jpg", "x"), "console.jpg");
    }

    #[test]
    fn jamais_d_ecrasement() {
        let dossier = std::env::temp_dir().join(format!("nicephore-nom-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        std::fs::write(dossier.join("photo.jpg"), b"x").unwrap();
        std::fs::write(dossier.join("photo (1).jpg"), b"x").unwrap();
        assert_eq!(nom_libre(&dossier, "photo.jpg"), dossier.join("photo (2).jpg"));
        assert_eq!(nom_libre(&dossier, "autre.png"), dossier.join("autre.png"));
        std::fs::remove_dir_all(&dossier).unwrap();
    }
}
