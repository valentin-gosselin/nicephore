//! Manual upload: files and folders dropped on the window (or picked with
//! the « Envoyer » button) sent to Immich, like the upload of Immich web.
//! Folders are walked (hidden files left out), files the server already
//! holds are skipped as duplicates (by their SHA-1, or the first version of
//! a photo turned by Nicéphore), the others are sent three at a time, then
//! everything goes into the album asked for, if any. Lots dropped while one
//! is running wait their turn; the progress is one event, `envoi-manuel`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, UNIX_EPOCH};

use api_immich::{ClientImmich, ElementVerification, MetaEnvoi, TypesMedias};
use futures_util::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::Etat;

/// Files fingerprinted then looked up together.
const LOT_VERIFICATION: usize = 200;
const ENVOIS_PARALLELES: usize = 3;
/// At most this many error lines kept for the panel.
const ERREURS_GARDEES: usize = 50;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErreurEnvoi {
    nom: String,
    message: String,
}

/// What the upload panel shows.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtatEnvoiManuel {
    /// A lot is being sent or waits.
    actif: bool,
    total: u64,
    traites: u64,
    envoyes: u64,
    doublons: u64,
    nombre_erreurs: u64,
    erreurs: Vec<ErreurEnvoi>,
    octets_total: u64,
    octets_faits: u64,
    fichier: Option<String>,
    /// Albums the photos go into, by name.
    albums: Vec<String>,
    annule: bool,
    /// Counts each finished lot: the views reload when it changes.
    lots_finis: u64,
    /// Lots sent or waiting.
    #[serde(skip)]
    lots: u64,
}

#[derive(Default)]
pub struct EnvoisManuels {
    etat: std::sync::Mutex<EtatEnvoiManuel>,
    /// One lot at a time, in the order they were dropped.
    file: tokio::sync::Mutex<()>,
    /// Bumped by « Annuler »: every lot started or waiting before stops.
    generation: AtomicU64,
    dernier_signal: std::sync::Mutex<Option<Instant>>,
}

impl EnvoisManuels {
    fn modifier(&self, app: &AppHandle, forcer: bool, f: impl FnOnce(&mut EtatEnvoiManuel)) {
        let copie = {
            let mut etat = self.etat.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut etat);
            etat.clone()
        };
        // Progress at most every 150 ms; starts, ends and errors at once.
        let mut dernier = self.dernier_signal.lock().unwrap_or_else(|e| e.into_inner());
        if forcer || dernier.is_none_or(|d| d.elapsed() >= Duration::from_millis(150)) {
            *dernier = Some(Instant::now());
            let _ = app.emit("envoi-manuel", copie);
        }
    }

    fn annule(&self, generation: u64) -> bool {
        self.generation.load(Ordering::SeqCst) != generation
    }
}

/// Album receiving a lot.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AlbumEnvoi {
    Existant { id: String, nom: String },
    Nouveau { nom: String },
}

impl AlbumEnvoi {
    fn nom(&self) -> &str {
        match self {
            AlbumEnvoi::Existant { nom, .. } | AlbumEnvoi::Nouveau { nom } => nom,
        }
    }
}

/// What was dropped, before asking where it goes: the folders by name.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Depot {
    dossiers: Vec<String>,
    fichiers: usize,
}

#[tauri::command]
pub fn decrire_depot(chemins: Vec<String>) -> Depot {
    let (dossiers, fichiers): (Vec<_>, Vec<_>) = chemins.iter().map(PathBuf::from).partition(|c| c.is_dir());
    Depot {
        dossiers: dossiers.iter().map(|d| d.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).collect(),
        fichiers: fichiers.len(),
    }
}

/// The photos and videos of what was dropped: folders walked, files kept
/// when the server takes their format. Returns them with the number of
/// files left out (formats the server does not take).
fn deplier(chemins: &[String], types: &TypesMedias) -> (Vec<PathBuf>, u64) {
    let mut fichiers = Vec::new();
    let mut ecartes = 0;
    for chemin in chemins.iter().map(PathBuf::from) {
        if chemin.is_dir() {
            match envoi::analyse::inventorier(&chemin, types) {
                Ok(inventaire) => fichiers.extend(inventaire.fichiers.iter().map(|f| envoi::analyse::absolu(&chemin, &f.chemin))),
                Err(e) => tracing::warn!("dossier illisible {} : {e}", chemin.display()),
            }
        } else if chemin.is_file() {
            let extension = chemin.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
            if types.accepte(&extension) {
                fichiers.push(chemin);
            } else {
                ecartes += 1;
            }
        }
    }
    fichiers.sort();
    fichiers.dedup();
    (fichiers, ecartes)
}

/// Upload details of a local file: its name, size, fingerprint and dates
/// (creation: the oldest of creation and modification).
pub fn meta_envoi(chemin: &Path, sha1: String) -> std::io::Result<MetaEnvoi> {
    let meta = std::fs::metadata(chemin)?;
    let ms = |t: std::io::Result<std::time::SystemTime>| t.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64);
    let modifie = ms(meta.modified());
    let cree = ms(meta.created()).min(modifie).max(0);
    Ok(MetaEnvoi {
        nom_fichier: chemin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        taille: meta.len(),
        sha1,
        cree_le: envoi::analyse::date_rfc3339(if cree > 0 { cree } else { modifie }),
        modifie_le: envoi::analyse::date_rfc3339(modifie),
    })
}

fn nom(chemin: &Path) -> String {
    chemin.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// What a drop holds: photos and videos sent, files left out (formats
/// the server does not take).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LotPrevu {
    medias: u64,
    ecartes: u64,
}

/// Sends files and folders to Immich (into an album when given). Returns
/// once the files are counted; the upload goes on in the background.
#[tauri::command]
pub async fn envoyer_lot(
    app: AppHandle,
    etat: State<'_, Etat>,
    envois: State<'_, Arc<EnvoisManuels>>,
    chemins: Vec<String>,
    album: Option<AlbumEnvoi>,
) -> Result<LotPrevu, String> {
    let client = etat.client()?;
    let types = client.types_medias().await.map_err(|e| e.to_string())?;
    let (fichiers, ecartes) = tokio::task::spawn_blocking(move || deplier(&chemins, &types)).await.map_err(|e| e.to_string())?;
    let nombre = fichiers.len() as u64;
    if nombre == 0 {
        return Ok(LotPrevu { medias: 0, ecartes });
    }
    let octets: u64 = fichiers.iter().filter_map(|f| f.metadata().ok()).map(|m| m.len()).sum();
    let envois = Arc::clone(&envois);
    let generation = envois.generation.load(Ordering::SeqCst);
    envois.modifier(&app, true, |e| {
        if !e.actif {
            // A new round: the counts of the last one are cleared.
            let lots_finis = e.lots_finis;
            *e = EtatEnvoiManuel { lots_finis, ..Default::default() };
        }
        e.actif = true;
        e.annule = false;
        e.lots += 1;
        e.total += nombre;
        e.octets_total += octets;
        if let Some(a) = &album {
            e.albums.push(a.nom().to_string());
        }
    });
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let _tour = envois.file.lock().await;
        let debut = envois.etat.lock().unwrap_or_else(|e| e.into_inner()).traites;
        let issue = envoyer(&app2, &client, &envois, generation, fichiers, album).await;
        if let Err(e) = issue {
            envois.modifier(&app2, true, |etat| {
                etat.nombre_erreurs += 1;
                if etat.erreurs.len() < ERREURS_GARDEES {
                    etat.erreurs.push(ErreurEnvoi { nom: String::new(), message: e });
                }
            });
        }
        envois.modifier(&app2, true, |e| {
            // A lot stopped early (album refused...) counts its files done.
            e.traites = e.traites.max(debut + nombre);
            e.lots_finis += 1;
            e.lots -= 1;
            e.actif = e.lots > 0;
            e.fichier = None;
        });
    });
    Ok(LotPrevu { medias: nombre, ecartes })
}

async fn envoyer(
    app: &AppHandle,
    client: &ClientImmich,
    envois: &EnvoisManuels,
    generation: u64,
    fichiers: Vec<PathBuf>,
    album: Option<AlbumEnvoi>,
) -> Result<(), String> {
    if envois.annule(generation) {
        return Ok(());
    }
    let album_id = match &album {
        Some(AlbumEnvoi::Existant { id, .. }) => Some(id.clone()),
        Some(AlbumEnvoi::Nouveau { nom }) => Some(client.creer_album(nom, "", &[]).await.map_err(|e| format!("album « {nom} » : {e}"))?.id),
        None => None,
    };
    let mut pour_l_album: Vec<String> = Vec::new();
    for lot in fichiers.chunks(LOT_VERIFICATION) {
        if envois.annule(generation) {
            break;
        }
        // Fingerprints, then one lookup for the lot.
        let chemins = lot.to_vec();
        let empreintes: Vec<Option<String>> = tokio::task::spawn_blocking(move || chemins.iter().map(|c| envoi::analyse::empreinte(c).ok()).collect())
            .await
            .map_err(|e| e.to_string())?;
        let origines: Vec<Option<String>> = {
            let etat = app.state::<Etat>();
            let base = etat.envoi.base();
            empreintes.iter().map(|s| s.as_ref().and_then(|s| base.origine(s).ok().flatten())).collect()
        };
        let mut elements = Vec::new();
        for (i, sha1) in empreintes.iter().enumerate() {
            if let Some(sha1) = sha1 {
                elements.push(ElementVerification { id: i.to_string(), checksum: sha1.clone() });
                if let Some(o) = &origines[i] {
                    elements.push(ElementVerification { id: format!("{i}-origine"), checksum: o.clone() });
                }
            }
        }
        let resultats = client.verifier_envois(&elements).await.map_err(|e| e.to_string())?;
        let mut deja: Vec<Option<String>> = vec![None; lot.len()];
        for r in resultats {
            if r.action == "accept" || r.reason.as_deref() != Some("duplicate") {
                continue;
            }
            let Some(i) = r.id.trim_end_matches("-origine").parse::<usize>().ok() else { continue };
            if deja[i].is_none() {
                deja[i] = r.asset_id;
            }
        }
        let mut a_envoyer = Vec::new();
        for (i, chemin) in lot.iter().enumerate() {
            match (&empreintes[i], &deja[i]) {
                (None, _) => {
                    let n = nom(chemin);
                    envois.modifier(app, true, |e| {
                        e.traites += 1;
                        e.nombre_erreurs += 1;
                        e.octets_faits += chemin.metadata().map(|m| m.len()).unwrap_or(0);
                        if e.erreurs.len() < ERREURS_GARDEES {
                            e.erreurs.push(ErreurEnvoi { nom: n, message: "lecture impossible".into() });
                        }
                    });
                }
                (Some(_), Some(id)) => {
                    pour_l_album.push(id.clone());
                    let taille = chemin.metadata().map(|m| m.len()).unwrap_or(0);
                    envois.modifier(app, false, |e| {
                        e.traites += 1;
                        e.doublons += 1;
                        e.octets_faits += taille;
                    });
                }
                (Some(sha1), None) => a_envoyer.push((chemin.clone(), sha1.clone())),
            }
        }
        let mut flux = futures_util::stream::iter(a_envoyer)
            .map(|(chemin, sha1)| async move {
                let issue = envoyer_un(app, client, envois, generation, &chemin, sha1).await;
                (chemin, issue)
            })
            .buffer_unordered(ENVOIS_PARALLELES);
        while let Some((chemin, issue)) = flux.next().await {
            match issue {
                Ok(Some((id, doublon))) => {
                    pour_l_album.push(id);
                    envois.modifier(app, false, |e| {
                        e.traites += 1;
                        if doublon { e.doublons += 1 } else { e.envoyes += 1 }
                    });
                }
                // Cancelled.
                Ok(None) => envois.modifier(app, false, |e| e.traites += 1),
                Err(message) => {
                    let n = nom(&chemin);
                    envois.modifier(app, true, |e| {
                        e.traites += 1;
                        e.nombre_erreurs += 1;
                        if e.erreurs.len() < ERREURS_GARDEES {
                            e.erreurs.push(ErreurEnvoi { nom: n, message });
                        }
                    });
                }
            }
        }
    }
    if let Some(id) = album_id
        && !pour_l_album.is_empty()
    {
        for morceau in pour_l_album.chunks(500) {
            client.ajouter_a_l_album(&id, morceau).await.map_err(|e| format!("album : {e}"))?;
        }
    }
    Ok(())
}

/// Sends one file, streamed from disk. `Some((id, duplicate))` once on the
/// server, `None` when cancelled meanwhile.
async fn envoyer_un(
    app: &AppHandle,
    client: &ClientImmich,
    envois: &EnvoisManuels,
    generation: u64,
    chemin: &Path,
    sha1: String,
) -> Result<Option<(String, bool)>, String> {
    if envois.annule(generation) {
        return Ok(None);
    }
    envois.modifier(app, false, |e| e.fichier = Some(nom(chemin)));
    let meta = meta_envoi(chemin, sha1).map_err(|e| e.to_string())?;
    let f = tokio::fs::File::open(chemin).await.map_err(|e| e.to_string())?;
    let app2 = app.clone();
    let flux = tokio_util::io::ReaderStream::new(f).inspect_ok(move |morceau| {
        let n = morceau.len() as u64;
        if let Some(envois) = app2.try_state::<Arc<EnvoisManuels>>() {
            envois.modifier(&app2, false, |e| e.octets_faits += n);
        }
    });
    tokio::select! {
        reponse = client.envoyer_asset(&meta, reqwest::Body::wrap_stream(flux)) => {
            let reponse = reponse.map_err(|e| e.to_string())?;
            Ok(Some((reponse.id, reponse.status == "duplicate")))
        }
        _ = async {
            while !envois.annule(generation) {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        } => Ok(None),
    }
}

/// Stops every lot sent or waiting.
#[tauri::command]
pub fn annuler_envois(app: AppHandle, envois: State<'_, Arc<EnvoisManuels>>) {
    envois.generation.fetch_add(1, Ordering::SeqCst);
    envois.modifier(&app, true, |e| e.annule = true);
}

/// The panel closed: the counts of the finished round are cleared.
#[tauri::command]
pub fn effacer_envois(app: AppHandle, envois: State<'_, Arc<EnvoisManuels>>) {
    envois.modifier(&app, true, |e| {
        if !e.actif {
            let lots_finis = e.lots_finis;
            *e = EtatEnvoiManuel { lots_finis, ..Default::default() };
        }
    });
}

/// Current state, for a panel opened after the start.
#[tauri::command]
pub fn etat_envois(envois: State<'_, Arc<EnvoisManuels>>) -> EtatEnvoiManuel {
    envois.etat.lock().unwrap_or_else(|e| e.into_inner()).clone()
}
