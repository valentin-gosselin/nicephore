//! Actions of the viewers beyond display: image and text to the clipboard,
//! the file shown in the file manager or opened with another application,
//! the system trash (with undo), small thumbnails of a folder and the EXIF
//! details of a local file, and its rotation (in the file and on Immich).

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use api_immich::TailleImage;
use image::{DynamicImage, ImageDecoder, ImageReader};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tokio::sync::Semaphore;

use crate::Etat;

/// Extensions read by the `image` crate (thumbnails, copy).
const DECODABLES: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "tif", "tiff"];
const RAW: &[&str] = &["dng", "arw", "cr2", "cr3", "nef", "orf", "raf", "rw2", "srw", "pef"];

fn extension(chemin: &Path) -> String {
    chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Decodes a photo of this computer, upright (EXIF orientation applied).
/// A RAW gives its embedded preview, turned by the RAW's own orientation.
fn decoder(chemin: &Path) -> Result<DynamicImage, String> {
    let e = extension(chemin);
    if RAW.contains(&e.as_str()) {
        let donnees = std::fs::read(chemin).map_err(|e| e.to_string())?;
        let plage = crate::apercu_raw::plus_grand_jpeg(&donnees).ok_or("pas d'aperçu dans ce RAW")?;
        let image = image::load_from_memory(&donnees[plage]).map_err(|e| e.to_string())?;
        return Ok(match crate::apercu_raw::orientation_tiff(&donnees).unwrap_or(1) {
            3 => image.rotate180(),
            6 => image.rotate90(),
            8 => image.rotate270(),
            _ => image,
        });
    }
    if !DECODABLES.contains(&e.as_str()) {
        return Err(format!("format non pris en charge ({e})"));
    }
    let mut decodeur = ImageReader::open(chemin)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_decoder()
        .map_err(|e| e.to_string())?;
    let orientation = decodeur.orientation().map_err(|e| e.to_string())?;
    let mut image = DynamicImage::from_decoder(decodeur).map_err(|e| e.to_string())?;
    image.apply_orientation(orientation);
    Ok(image)
}

fn tourner(image: DynamicImage, quarts: u8) -> DynamicImage {
    match quarts % 4 {
        1 => image.rotate90(),
        2 => image.rotate180(),
        3 => image.rotate270(),
        _ => image,
    }
}

fn vers_presse_papiers(app: &AppHandle, image: DynamicImage) -> Result<(), String> {
    let rgba = image.into_rgba8();
    let (largeur, hauteur) = rgba.dimensions();
    app.clipboard()
        .write_image(&tauri::image::Image::new_owned(rgba.into_raw(), largeur, hauteur))
        .map_err(|e| e.to_string())
}

/// Copies a photo of this computer as an image, turned by `quarts` quarter
/// turns clockwise (the rotation shown in the viewer).
#[tauri::command]
pub async fn copier_image_locale(app: AppHandle, chemin: String, quarts: u8) -> Result<(), String> {
    let image = tokio::task::spawn_blocking(move || decoder(Path::new(&chemin)).map(|i| tourner(i, quarts)))
        .await
        .map_err(|e| e.to_string())??;
    vers_presse_papiers(&app, image)
}

/// Copies a photo of the library as an image (its full rendering, or the
/// preview when the server keeps none).
#[tauri::command]
pub async fn copier_image_immich(app: AppHandle, etat: State<'_, Etat>, id: String, version: String) -> Result<(), String> {
    let client = etat.client()?;
    let octets = match etat.cache.obtenir(&client, &id, TailleImage::Pleine, &version).await {
        Ok(image) => image.octets,
        Err(_) => etat.cache.obtenir(&client, &id, TailleImage::Apercu, &version).await.map_err(|e| e.to_string())?.octets,
    };
    let image = tokio::task::spawn_blocking(move || image::load_from_memory(&octets).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())??;
    vers_presse_papiers(&app, image)
}

#[tauri::command]
pub fn copier_texte(app: AppHandle, texte: String) -> Result<(), String> {
    app.clipboard().write_text(texte).map_err(|e| e.to_string())
}

/// Shows the file in the file manager, selected.
#[tauri::command]
pub async fn afficher_dans_dossier(chemin: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let uri = url::Url::from_file_path(&chemin).map_err(|_| "chemin invalide")?.to_string();
        let selection = async {
            let connexion = zbus::Connection::session().await?;
            connexion
                .call_method(
                    Some("org.freedesktop.FileManager1"),
                    "/org/freedesktop/FileManager1",
                    Some("org.freedesktop.FileManager1"),
                    "ShowItems",
                    &(vec![uri.as_str()], ""),
                )
                .await
                .map(|_| ())
        };
        if let Err(e) = selection.await {
            // No file manager on the bus: its folder, at least.
            tracing::info!("ShowItems indisponible ({e}), ouverture du dossier");
            let dossier = Path::new(&chemin).parent().ok_or("fichier sans dossier")?;
            std::process::Command::new("xdg-open").arg(dossier).spawn().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{chemin}\""))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = chemin;
        Err("non pris en charge sur ce système".into())
    }
}

/// Asks the system which application should open the file.
#[tauri::command]
pub async fn ouvrir_avec(chemin: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use std::collections::HashMap;
        use std::os::fd::AsFd;
        use zbus::zvariant::{Fd, Value};
        // The desktop portal, with its application chooser ("ask").
        let fichier = std::fs::File::open(&chemin).map_err(|e| e.to_string())?;
        let connexion = zbus::Connection::session().await.map_err(|e| e.to_string())?;
        let options: HashMap<&str, Value> = HashMap::from([("ask", Value::from(true))]);
        connexion
            .call_method(
                Some("org.freedesktop.portal.Desktop"),
                "/org/freedesktop/portal/desktop",
                Some("org.freedesktop.portal.OpenURI"),
                "OpenFile",
                &("", Fd::from(fichier.as_fd()), options),
            )
            .await
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("rundll32.exe")
            .raw_arg(format!("shell32.dll,OpenAs_RunDLL {chemin}"))
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = chemin;
        Err("non pris en charge sur ce système".into())
    }
}

/// Moves a file of this computer to the system trash.
#[tauri::command]
pub async fn corbeille_locale(chemin: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || trash::delete(&chemin).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

/// Puts back the file last trashed from `chemin`.
#[tauri::command]
pub async fn restaurer_corbeille_locale(chemin: String) -> Result<(), String> {
    #[cfg(any(target_os = "linux", windows))]
    {
        tokio::task::spawn_blocking(move || {
            let cible = PathBuf::from(&chemin);
            let element = trash::os_limited::list()
                .map_err(|e| e.to_string())?
                .into_iter()
                .filter(|e| e.original_path() == cible)
                .max_by_key(|e| e.time_deleted)
                .ok_or("introuvable dans la corbeille")?;
            trash::os_limited::restore_all([element]).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = chemin;
        Err("non pris en charge sur ce système".into())
    }
}

/// At most two thumbnails decoded at once: the folder strip stays light.
/// Formats whose file can be turned: JPEG by its EXIF orientation alone,
/// the others encoded again without loss (PNG, lossless WebP, TIFF, BMP).
/// RAW files and GIF (animations) are only shown turned.
const PIVOTABLES: &[&str] = &["jpg", "jpeg", "png", "webp", "bmp", "tif", "tiff"];

/// Turns the file itself by `quarts` quarter turns clockwise. The new
/// content is written beside it then renamed over it (never a half-written
/// photo), with its rights and its date (two seconds later, so the
/// synchronisation sees the change).
fn pivoter_sur_disque(chemin: &Path, quarts: u8) -> Result<(), String> {
    use std::io::Write;
    let meta = std::fs::metadata(chemin).map_err(|e| e.to_string())?;
    if meta.permissions().readonly() {
        return Err("fichier en lecture seule".into());
    }
    let e = extension(chemin);
    let octets = if e == "jpg" || e == "jpeg" {
        let jpeg = std::fs::read(chemin).map_err(|e| e.to_string())?;
        let orientation = crate::orientation::lire(&jpeg)?;
        crate::orientation::ecrire(&jpeg, crate::orientation::tournee(orientation, quarts))?
    } else {
        let format = image::ImageFormat::from_extension(&e).ok_or_else(|| format!("format non pris en charge ({e})"))?;
        let mut image = tourner(decoder(chemin)?, quarts);
        if matches!(format, image::ImageFormat::WebP | image::ImageFormat::Bmp) {
            image = if image.color().has_alpha() {
                DynamicImage::ImageRgba8(image.to_rgba8())
            } else {
                DynamicImage::ImageRgb8(image.to_rgb8())
            };
        }
        let mut tampon = Cursor::new(Vec::new());
        image.write_to(&mut tampon, format).map_err(|e| e.to_string())?;
        tampon.into_inner()
    };
    let nom = chemin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let temporaire = chemin.with_file_name(format!(".{nom}.nicephore"));
    let ecrire = || -> std::io::Result<()> {
        let mut f = std::fs::File::create(&temporaire)?;
        f.write_all(&octets)?;
        f.set_permissions(meta.permissions())?;
        if let Ok(date) = meta.modified() {
            let _ = f.set_modified(date + std::time::Duration::from_secs(2));
        }
        f.sync_all()?;
        std::fs::rename(&temporaire, chemin)
    };
    ecrire().map_err(|e| {
        let _ = std::fs::remove_file(&temporaire);
        e.to_string()
    })
}

/// What turning a photo of this computer did.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rotation {
    /// The file itself was turned (otherwise only shown turned: RAW, GIF).
    fichier: bool,
    /// The photo is on Immich and was turned there too.
    immich: bool,
    /// Why it could not be turned on Immich, when it is there.
    erreur_immich: Option<String>,
}

/// Turns a photo of this computer: the file itself, and the same photo on
/// Immich by its editor. The new fingerprint is linked to the first one, so
/// the photo is still known as the one on the server (never sent again).
#[tauri::command]
pub async fn pivoter_fichier_local(etat: State<'_, Etat>, chemin: String, quarts: u8) -> Result<Rotation, String> {
    let quarts = quarts % 4;
    let fichier = PathBuf::from(&chemin);
    let e = extension(&fichier);
    let modifiable = PIVOTABLES.contains(&e.as_str());
    if quarts == 0 {
        return Ok(Rotation { fichier: modifiable, immich: false, erreur_immich: None });
    }
    let c = fichier.clone();
    let avant = tokio::task::spawn_blocking(move || envoi::analyse::empreinte(&c)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    let origine = etat.envoi.base().origine(&avant).ok().flatten();
    // Looked up before the file changes: its fingerprint then is the one
    // the server may hold.
    let client = etat.client().ok();
    let sur_serveur = match &client {
        Some(c) => crate::fichiers_locaux::asset_par_empreintes(c, &avant, origine.as_deref()).await.map(|t| t.map(|(id, _)| id)),
        None => Ok(None),
    };
    if modifiable {
        let c = fichier.clone();
        let apres = tokio::task::spawn_blocking(move || {
            pivoter_sur_disque(&c, quarts)?;
            envoi::analyse::empreinte(&c).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        etat.envoi.base().lier_empreinte(&apres, &avant).map_err(|e| e.to_string())?;
    }
    let (immich, erreur_immich) = match (client, sur_serveur) {
        (Some(client), Ok(Some(id))) => match client.pivoter(&id, quarts).await {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        },
        (_, Err(e)) => (false, Some(e)),
        _ => (false, None),
    };
    Ok(Rotation { fichier: modifiable, immich, erreur_immich })
}

static VIGNETTES: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));
const COTE_VIGNETTE: u32 = 240;

/// Small thumbnail of a photo of this computer, made once into the cache
/// (path of a JPEG), or `None` when its format cannot be read here.
#[tauri::command]
pub async fn vignette_locale(app: AppHandle, chemin: String) -> Result<Option<String>, String> {
    let fichier = PathBuf::from(&chemin);
    let e = extension(&fichier);
    if !DECODABLES.contains(&e.as_str()) && !RAW.contains(&e.as_str()) {
        return Ok(None);
    }
    let dossier = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("vignettes-locales");
    app.asset_protocol_scope().allow_directory(&dossier, false).map_err(|e| e.to_string())?;
    let _jeton = VIGNETTES.acquire().await.map_err(|e| e.to_string())?;
    tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
        use sha1::{Digest, Sha1};
        let meta = std::fs::metadata(&fichier).map_err(|e| e.to_string())?;
        let cle = Sha1::digest(format!("{}\n{}\n{:?}", fichier.display(), meta.len(), meta.modified().ok()).as_bytes());
        let sortie = dossier.join(format!("{}.jpg", hex::encode(&cle[..10])));
        if sortie.exists() {
            return Ok(Some(sortie.to_string_lossy().into_owned()));
        }
        let Ok(image) = decoder(&fichier) else { return Ok(None) };
        let petite = image.thumbnail(COTE_VIGNETTE, COTE_VIGNETTE).into_rgb8();
        std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
        let temporaire = sortie.with_extension("part");
        let mut tampon = Cursor::new(Vec::new());
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut tampon, 82).encode_image(&petite).map_err(|e| e.to_string())?;
        std::fs::write(&temporaire, tampon.into_inner()).map_err(|e| e.to_string())?;
        std::fs::rename(&temporaire, &sortie).map_err(|e| e.to_string())?;
        Ok(Some(sortie.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// EXIF details of a local photo, for the info panel.
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InfosLocales {
    /// "YYYY-MM-DDTHH:MM:SS", with its offset when the camera wrote one.
    date_prise: Option<String>,
    appareil: Option<String>,
    objectif: Option<String>,
    focale: Option<f64>,
    ouverture: Option<f64>,
    /// Exposure time in seconds.
    exposition: Option<f64>,
    iso: Option<u32>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

fn lire_exif(octets: &[u8]) -> Option<exif::Exif> {
    exif::Reader::new().read_from_container(&mut Cursor::new(octets)).ok()
}

fn infos_depuis_exif(exif: &exif::Exif) -> InfosLocales {
    use exif::{In, Tag, Value};
    let champ = |tag| exif.get_field(tag, In::PRIMARY);
    let texte = |tag| {
        champ(tag).and_then(|c| match &c.value {
            Value::Ascii(v) => v.first().map(|o| String::from_utf8_lossy(o).trim().trim_end_matches('\0').to_string()),
            _ => None,
        })
        .filter(|s| !s.is_empty())
    };
    let rationnel = |tag| {
        champ(tag).and_then(|c| match &c.value {
            Value::Rational(v) => v.first().filter(|r| r.denom != 0).map(|r| r.to_f64()),
            _ => None,
        })
    };
    let date = texte(Tag::DateTimeOriginal).or_else(|| texte(Tag::DateTime)).and_then(|d| {
        // "2024:07:14 18:32:05" (EXIF) to ISO 8601.
        let (jour, heure) = d.split_once(' ')?;
        let iso = format!("{}T{heure}", jour.replace(':', "-"));
        Some(match texte(Tag::OffsetTimeOriginal) {
            Some(decalage) => format!("{iso}{decalage}"),
            None => iso,
        })
    });
    let marque = texte(Tag::Make);
    let modele = texte(Tag::Model);
    // Most models already start with the brand ("Canon EOS R6").
    let appareil = match (marque, modele) {
        (Some(m), Some(n)) if n.to_lowercase().starts_with(&m.to_lowercase().split_whitespace().next().unwrap_or("").to_string()) => Some(n),
        (Some(m), Some(n)) => Some(format!("{m} {n}")),
        (m, n) => n.or(m),
    };
    let coordonnee = |tag, reference, negatif: &[u8]| {
        let valeur = champ(tag).and_then(|c| match &c.value {
            Value::Rational(v) if v.len() >= 3 && v.iter().all(|r| r.denom != 0) => Some(v[0].to_f64() + v[1].to_f64() / 60.0 + v[2].to_f64() / 3600.0),
            _ => None,
        })?;
        let signe = champ(reference).and_then(|c| match &c.value {
            Value::Ascii(v) => v.first().and_then(|o| o.first().copied()),
            _ => None,
        });
        Some(if signe.is_some_and(|s| negatif.contains(&s)) { -valeur } else { valeur })
    };
    let iso = champ(Tag::PhotographicSensitivity).and_then(|c| c.value.get_uint(0));
    InfosLocales {
        date_prise: date,
        appareil,
        objectif: texte(Tag::LensModel),
        focale: rationnel(Tag::FocalLength),
        ouverture: rationnel(Tag::FNumber),
        exposition: rationnel(Tag::ExposureTime),
        iso,
        latitude: coordonnee(Tag::GPSLatitude, Tag::GPSLatitudeRef, b"Ss"),
        longitude: coordonnee(Tag::GPSLongitude, Tag::GPSLongitudeRef, b"Ww"),
    }
}

/// Reads the EXIF of a local photo (of a RAW's embedded preview when the
/// RAW itself cannot be read).
#[tauri::command]
pub async fn infos_fichier_local(chemin: String) -> Result<InfosLocales, String> {
    tokio::task::spawn_blocking(move || {
        let fichier = PathBuf::from(&chemin);
        let octets = std::fs::read(&fichier).map_err(|e| e.to_string())?;
        let exif = lire_exif(&octets).or_else(|| {
            RAW.contains(&extension(&fichier).as_str())
                .then(|| crate::apercu_raw::plus_grand_jpeg(&octets))
                .flatten()
                .and_then(|plage| lire_exif(&octets[plage]))
        });
        Ok(exif.map(|e| infos_depuis_exif(&e)).unwrap_or_default())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Place of a point by the server (city, region, country), when connected.
#[tauri::command]
pub async fn lieu_du_point(etat: State<'_, Etat>, latitude: f64, longitude: f64) -> Result<Option<String>, String> {
    let Ok(client) = etat.client() else { return Ok(None) };
    let lieux = client.lieu_inverse(latitude, longitude).await.map_err(|e| e.to_string())?;
    Ok(lieux.into_iter().next().map(|l| {
        [l.city, l.state, l.country].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ")
    }).filter(|s| !s.is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_par_quarts() {
        let image = DynamicImage::new_rgb8(4, 2);
        assert_eq!((tourner(image.clone(), 1).width(), tourner(image.clone(), 1).height()), (2, 4));
        assert_eq!(tourner(image.clone(), 2).width(), 4);
        assert_eq!(tourner(image, 7).width(), 2);
    }

    #[test]
    fn sans_exif_rien() {
        assert!(lire_exif(b"pas une image").is_none());
    }
}
