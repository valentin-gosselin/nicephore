//! Photos and videos of this computer opened with Nicéphore (default
//! viewer): the file and its neighbours in the folder, whether it is on the
//! server already (by SHA-1, like the mobile app's cloud badge), and sending
//! it right away. Only the folder of an opened file is made readable by the
//! interface.

use api_immich::{ElementVerification, MetaEnvoi};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::Etat;

const IMAGES: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "avif", "bmp", "tif", "tiff", "heic", "heif"];
const RAW: &[&str] = &["dng", "arw", "cr2", "cr3", "nef", "orf", "raf", "rw2", "srw", "pef"];
const VIDEOS: &[&str] = &["mp4", "mov", "m4v", "webm", "mkv", "avi", "3gp", "mts", "m2ts"];
/// Formats the web view displays itself.
/// Videos are all tried: the system's decoders decide, and an unreadable
/// one shows a message.
const AFFICHABLES: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "avif", "bmp", "mp4", "m4v", "webm", "mov", "mkv", "avi", "3gp", "mts", "m2ts",
];

fn extension(chemin: &Path) -> String {
    chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

fn est_media(chemin: &Path) -> bool {
    let e = extension(chemin);
    IMAGES.contains(&e.as_str()) || RAW.contains(&e.as_str()) || VIDEOS.contains(&e.as_str())
}

/// Files named on a command line (the program and options skipped).
pub fn fichiers_dans_args(args: &[String]) -> Vec<String> {
    args.iter()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .map(|a| a.strip_prefix("file://").map(|p| p.to_string()).unwrap_or_else(|| a.clone()))
        .filter(|a| Path::new(a).is_file() && est_media(Path::new(a)))
        .collect()
}

/// Files given at launch, handed once to the interface.
#[derive(Default)]
pub struct FichiersInitiaux(pub Mutex<Option<Vec<String>>>);

#[tauri::command]
pub fn fichiers_au_demarrage(initiaux: State<'_, FichiersInitiaux>) -> Vec<String> {
    initiaux.0.lock().expect("verrou des fichiers").take().unwrap_or_default()
}

/// A second launch (file opened while Nicéphore runs): the running
/// instance shows the file.
pub fn seconde_instance(app: &AppHandle, args: Vec<String>) {
    let fichiers = fichiers_dans_args(&args);
    if fichiers.is_empty() {
        crate::zone_notification::montrer(app);
    } else {
        // The page sizes the window to the photo, then shows it.
        let _ = app.emit("ouvrir-fichiers", fichiers);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FichierOuvert {
    chemin: String,
    nom: String,
    taille: u64,
    modifie_le: i64,
    /// The web view can display it (otherwise only its details and sending).
    affichable: bool,
    video: bool,
    /// A RAW file: its embedded preview can be shown (`apercu_local`).
    raw: bool,
    /// Photos and videos of the same folder, by name, this one included.
    voisins: Vec<String>,
    index: usize,
}

/// Opens a local file: its details and neighbours, and its folder made
/// readable by the interface (not its sub-folders).
#[tauri::command]
pub fn ouvrir_fichier_local(app: AppHandle, chemin: String) -> Result<FichierOuvert, String> {
    let fichier = PathBuf::from(&chemin);
    let meta = std::fs::metadata(&fichier).map_err(|e| format!("{chemin} : {e}"))?;
    if !meta.is_file() || !est_media(&fichier) {
        return Err("ce n'est pas une photo ni une vidéo".into());
    }
    let dossier = fichier.parent().ok_or("fichier sans dossier")?.to_path_buf();
    app.asset_protocol_scope().allow_directory(&dossier, false).map_err(|e| e.to_string())?;
    let mut voisins: Vec<String> = std::fs::read_dir(&dossier)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && est_media(p) && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    voisins.sort_by_key(|v| v.to_lowercase());
    let index = voisins.iter().position(|v| Path::new(v) == fichier).unwrap_or(0);
    let e = extension(&fichier);
    Ok(FichierOuvert {
        nom: fichier.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        taille: meta.len(),
        modifie_le: meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64),
        affichable: AFFICHABLES.contains(&e.as_str()),
        video: VIDEOS.contains(&e.as_str()),
        raw: RAW.contains(&e.as_str()),
        chemin,
        voisins,
        index,
    })
}

/// URL to play a video of this computer (through the local relay: the web
/// view cannot play videos from Tauri's own protocols on Linux).
#[tauri::command]
pub fn url_video_locale(etat: State<'_, Etat>, chemin: String) -> Result<String, String> {
    let fichier = PathBuf::from(&chemin);
    if !fichier.is_file() || !VIDEOS.contains(&extension(&fichier).as_str()) {
        return Err("ce n'est pas une vidéo".into());
    }
    Ok(etat.proxy.url_locale(fichier))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApercuLocal {
    chemin: String,
    largeur: u32,
    hauteur: u32,
    /// EXIF orientation of the RAW (the embedded preview is not rotated).
    orientation: u16,
}

/// Preview of a RAW file: the camera's embedded JPEG, extracted once into
/// the cache. `None` when the file carries none.
#[tauri::command]
pub async fn apercu_local(app: AppHandle, chemin: String) -> Result<Option<ApercuLocal>, String> {
    let fichier = PathBuf::from(&chemin);
    if !RAW.contains(&extension(&fichier).as_str()) {
        return Ok(None);
    }
    let dossier = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("apercus-raw");
    app.asset_protocol_scope().allow_directory(&dossier, false).map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || -> Result<Option<ApercuLocal>, String> {
        use sha1::{Digest, Sha1};
        let meta = std::fs::metadata(&fichier).map_err(|e| e.to_string())?;
        let cle = Sha1::digest(format!("{}\n{}\n{:?}", fichier.display(), meta.len(), meta.modified().ok()).as_bytes());
        let sortie = dossier.join(format!("{}.jpg", hex::encode(&cle[..10])));
        // Already extracted: the small JPEG gives the size, the head of the
        // RAW its orientation (its first directory sits at the start), with
        // no full read of the RAW.
        if let Ok(jpeg) = std::fs::read(&sortie)
            && let Some((largeur, hauteur)) = crate::apercu_raw::dimensions_jpeg(&jpeg)
        {
            use std::io::Read;
            let mut tete = Vec::with_capacity(1 << 20);
            std::fs::File::open(&fichier).and_then(|f| f.take(1 << 20).read_to_end(&mut tete)).map_err(|e| e.to_string())?;
            return Ok(Some(ApercuLocal {
                chemin: sortie.to_string_lossy().into_owned(),
                largeur,
                hauteur,
                orientation: crate::apercu_raw::orientation_tiff(&tete).unwrap_or(1),
            }));
        }
        let donnees = std::fs::read(&fichier).map_err(|e| e.to_string())?;
        let Some(plage) = crate::apercu_raw::plus_grand_jpeg(&donnees) else { return Ok(None) };
        let (largeur, hauteur) = crate::apercu_raw::dimensions_jpeg(&donnees[plage.clone()]).unwrap_or((0, 0));
        if !sortie.exists() {
            std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
            let temporaire = sortie.with_extension("part");
            std::fs::write(&temporaire, &donnees[plage]).map_err(|e| e.to_string())?;
            std::fs::rename(&temporaire, &sortie).map_err(|e| e.to_string())?;
        }
        Ok(Some(ApercuLocal {
            chemin: sortie.to_string_lossy().into_owned(),
            largeur,
            hauteur,
            orientation: crate::apercu_raw::orientation_tiff(&donnees).unwrap_or(1),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtatLocal {
    connecte: bool,
    sur_serveur: bool,
    asset_id: Option<String>,
    dans_la_corbeille: bool,
}

/// Is this file on the server already? (by its SHA-1, like the uploads)
#[tauri::command]
pub async fn etat_fichier_local(etat: State<'_, Etat>, chemin: String) -> Result<EtatLocal, String> {
    let Ok(client) = etat.client() else {
        return Ok(EtatLocal { connecte: false, sur_serveur: false, asset_id: None, dans_la_corbeille: false });
    };
    let c = PathBuf::from(&chemin);
    let sha1 = tokio::task::spawn_blocking(move || envoi::analyse::empreinte(&c)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    let resultats = client
        .verifier_envois(&[ElementVerification { id: "0".into(), checksum: sha1 }])
        .await
        .map_err(|e| e.to_string())?;
    let r = resultats.into_iter().next();
    let present = r.as_ref().is_some_and(|r| r.action != "accept" && r.reason.as_deref() == Some("duplicate"));
    Ok(EtatLocal {
        connecte: true,
        sur_serveur: present,
        asset_id: r.as_ref().and_then(|r| r.asset_id.clone()),
        dans_la_corbeille: r.is_some_and(|r| r.is_trashed),
    })
}

/// Sends this file to the server now (library, no album).
#[tauri::command]
pub async fn envoyer_fichier_local(etat: State<'_, Etat>, chemin: String) -> Result<String, String> {
    let client = etat.client()?;
    let fichier = PathBuf::from(&chemin);
    let meta = tokio::fs::metadata(&fichier).await.map_err(|e| e.to_string())?;
    let c = fichier.clone();
    let sha1 = tokio::task::spawn_blocking(move || envoi::analyse::empreinte(&c)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    let ms = |t: std::io::Result<std::time::SystemTime>| t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64);
    let modifie = ms(meta.modified());
    let cree = ms(meta.created()).min(modifie).max(0);
    let meta_envoi = MetaEnvoi {
        nom_fichier: fichier.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        taille: meta.len(),
        sha1,
        cree_le: envoi::analyse::date_rfc3339(if cree > 0 { cree } else { modifie }),
        modifie_le: envoi::analyse::date_rfc3339(modifie),
    };
    let f = tokio::fs::File::open(&fichier).await.map_err(|e| e.to_string())?;
    let reponse = client.envoyer_asset(&meta_envoi, reqwest::Body::from(f)).await.map_err(|e| e.to_string())?;
    Ok(reponse.id)
}

/// Types offered to the desktop for "open with" and the default viewer.
#[cfg(target_os = "linux")]
const TYPES_MIME: &[&str] = &[
    "image/jpeg", "image/png", "image/gif", "image/webp", "image/avif", "image/bmp", "image/tiff", "image/heic", "image/heif",
    "video/mp4", "video/quicktime", "video/webm", "video/x-matroska",
];

/// Makes Nicéphore the default viewer for photos and videos (on request).
/// Linux: `xdg-mime default` with Nicéphore's desktop entry. Windows does
/// not let an app do it: its "Default apps" settings page is opened.
#[tauri::command]
pub fn definir_visionneuse_par_defaut() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let entree = entree_desktop().ok_or("lanceur de Nicéphore introuvable (scripts/installer-lanceur.sh)")?;
        let statut = std::process::Command::new("xdg-mime")
            .arg("default")
            .arg(&entree)
            .args(TYPES_MIME)
            .status()
            .map_err(|e| format!("xdg-mime : {e}"))?;
        if !statut.success() {
            return Err(format!("xdg-mime : {statut}"));
        }
        Ok(format!("Nicéphore ouvre désormais les photos et vidéos ({entree})."))
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd").args(["/C", "start", "ms-settings:defaultapps"]).spawn().map_err(|e| e.to_string())?;
        Ok("Choisissez Nicéphore pour les photos dans la page des applications par défaut.".into())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    Err("non pris en charge sur ce système".into())
}

/// Desktop entry of Nicéphore (development launcher, rpm or AppImage).
#[cfg(target_os = "linux")]
fn entree_desktop() -> Option<String> {
    let mut dossiers: Vec<PathBuf> = dirs::data_dir().into_iter().map(|d| d.join("applications")).collect();
    dossiers.extend(["/usr/share/applications", "/usr/local/share/applications"].iter().map(PathBuf::from));
    for dossier in dossiers {
        let Ok(entrees) = std::fs::read_dir(&dossier) else { continue };
        for e in entrees.flatten() {
            let nom = e.file_name().to_string_lossy().into_owned();
            if nom.ends_with(".desktop")
                && std::fs::read_to_string(e.path()).is_ok_and(|t| t.lines().any(|l| l == "Name=Nicéphore"))
            {
                return Some(nom);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichiers_de_la_ligne_de_commande() {
        let dossier = tempfile::tempdir().unwrap();
        let photo = dossier.path().join("a.JPG");
        std::fs::write(&photo, b"x").unwrap();
        std::fs::write(dossier.path().join("b.txt"), b"x").unwrap();
        let args = vec![
            "nicephore".to_string(),
            "--arriere-plan".to_string(),
            photo.to_string_lossy().into_owned(),
            dossier.path().join("b.txt").to_string_lossy().into_owned(),
            format!("file://{}", photo.display()),
            "/nexiste/pas.jpg".to_string(),
        ];
        let p = photo.to_string_lossy().into_owned();
        assert_eq!(fichiers_dans_args(&args), vec![p.clone(), p]);
    }
}
