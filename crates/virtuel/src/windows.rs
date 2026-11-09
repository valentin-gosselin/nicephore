//! Windows: the virtual folder as a Cloud Files sync root (the API behind
//! OneDrive's "online-only" files). Every photo is a placeholder with its
//! real size and date; opening it hydrates it from the originals cache;
//! "Free up space" in Explorer is allowed; deleting or renaming is refused
//! (read-only, like on Linux).
//!
//! Not tested on Windows yet: written against `cloud-filter` 0.0.7, which
//! calls itself experimental. Only type-checked from Linux.

use crate::arbre::{Arbre, Genre, Inodes, Rangement, RACINE};
use api_immich::AssetDistant;
use cloud_filter::error::{CResult, CloudErrorKind};
use cloud_filter::filter::{Request, SyncFilter, info, ticket};
use cloud_filter::metadata::Metadata;
use cloud_filter::placeholder_create_info::PlaceholderCreateInfo;
use cloud_filter::root::{
    Connection, HydrationPolicy, HydrationType, PopulationType, SecurityId, Session, SyncRootId, SyncRootIdBuilder,
    SyncRootInfo,
};
use cloud_filter::utility::{FileTime, WriteAt};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Gives the path of an original on disk, downloading it first if needed
/// (blocking: called from a Cloud Files thread).
pub trait Source: Send + Sync + 'static {
    fn original(&self, asset: &AssetDistant) -> Result<PathBuf, String>;
}

/// Data is handed over in 4 MB pieces (offsets stay 4 KB aligned).
const MORCEAU: u64 = 4 * 1024 * 1024;

struct Fournisseur {
    source: Arc<dyn Source>,
    assets: Arc<RwLock<HashMap<String, AssetDistant>>>,
}

impl SyncFilter for Fournisseur {
    fn fetch_data(&self, request: Request, ticket: ticket::FetchData, info: info::FetchData) -> CResult<()> {
        let id = std::str::from_utf8(request.file_blob()).map_err(|_| CloudErrorKind::InvalidRequest)?.to_string();
        let asset = self.assets.read().expect("verrou des photos").get(&id).cloned().ok_or(CloudErrorKind::InvalidRequest)?;
        let chemin = self.source.original(&asset).map_err(|_| CloudErrorKind::NetworkUnavailable)?;
        let mut fichier = std::fs::File::open(chemin).map_err(|_| CloudErrorKind::Unsuccessful)?;
        let taille = fichier.metadata().map_err(|_| CloudErrorKind::Unsuccessful)?.len();
        let plage = info.required_file_range();
        let fin = plage.end.min(taille);
        let mut position = plage.start;
        fichier.seek(SeekFrom::Start(position)).map_err(|_| CloudErrorKind::Unsuccessful)?;
        while position < fin {
            let longueur = (fin - position).min(MORCEAU) as usize;
            let mut tampon = vec![0u8; longueur];
            fichier.read_exact(&mut tampon).map_err(|_| CloudErrorKind::Unsuccessful)?;
            ticket.write_at(&tampon, position).map_err(|_| CloudErrorKind::Unsuccessful)?;
            position += longueur as u64;
            let _ = ticket.report_progress(fin - plage.start, position - plage.start);
        }
        Ok(())
    }

    /// "Free up space": the content goes, the placeholder stays.
    fn dehydrate(&self, _request: Request, ticket: ticket::Dehydrate, _info: info::Dehydrate) -> CResult<()> {
        ticket.pass().map_err(|_| CloudErrorKind::Unsuccessful)
    }

    fn delete(&self, _request: Request, _ticket: ticket::Delete, _info: info::Delete) -> CResult<()> {
        Err(CloudErrorKind::AccessDenied)
    }

    fn rename(&self, _request: Request, _ticket: ticket::Rename, _info: info::Rename) -> CResult<()> {
        Err(CloudErrorKind::AccessDenied)
    }
}

/// A connected virtual folder; disconnected when dropped (placeholders and
/// downloaded contents stay; they open again at the next connection).
pub struct MontageWindows {
    connexion: Option<Connection<Fournisseur>>,
    id: SyncRootId,
    assets: Arc<RwLock<HashMap<String, AssetDistant>>>,
    inodes: std::sync::Mutex<Inodes>,
    rangement: Rangement,
    point: PathBuf,
}

fn identite(point: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    point.hash(&mut h);
    format!("Nicephore!{:016x}", h.finish())
}

fn date(rfc3339: &str) -> Option<FileTime> {
    let t = humantime::parse_rfc3339_weak(rfc3339).ok()?;
    FileTime::try_from(t).ok()
}

/// Creates the placeholders missing under `point` (existing names are kept).
fn creer_espaces(point: &Path, arbre: &Arbre) -> Result<(), String> {
    let mut pile = vec![(RACINE, point.to_path_buf())];
    while let Some((ino, dossier)) = pile.pop() {
        for enfant in arbre.enfants(ino) {
            let chemin = dossier.join(&enfant.nom);
            match &enfant.genre {
                Genre::Dossier { .. } => {
                    if !chemin.exists() {
                        PlaceholderCreateInfo::new(&enfant.nom)
                            .metadata(Metadata::directory())
                            .blob(b"dossier".to_vec())
                            .mark_in_sync()
                            .create(&dossier)
                            .map_err(|e| format!("{} : {e}", chemin.display()))?;
                    }
                    pile.push((enfant.ino, chemin));
                }
                Genre::Fichier { asset } => {
                    if chemin.exists() {
                        continue;
                    }
                    let mut meta = Metadata::file().size(asset.taille.unwrap_or(0));
                    if let Some(t) = date(&asset.modifie_le) {
                        meta = meta.written(t);
                    }
                    PlaceholderCreateInfo::new(&enfant.nom)
                        .metadata(meta)
                        .blob(asset.id.as_bytes().to_vec())
                        .mark_in_sync()
                        .create(&dossier)
                        .map_err(|e| format!("{} : {e}", chemin.display()))?;
                }
            }
        }
    }
    Ok(())
}

impl MontageWindows {
    pub fn point(&self) -> &Path {
        &self.point
    }

    pub fn fichiers(&self) -> usize {
        self.assets.read().expect("verrou des photos").len()
    }

    /// New photos of the selection get their placeholder (removed ones stay).
    pub fn actualiser(&self, assets: &[AssetDistant]) -> Result<(), String> {
        let arbre = Arbre::construire(assets, self.rangement, &mut self.inodes.lock().expect("verrou des inodes"));
        *self.assets.write().expect("verrou des photos") = assets.iter().map(|a| (a.id.clone(), a.clone())).collect();
        creer_espaces(&self.point, &arbre)
    }

    pub fn demonter(&mut self) {
        self.connexion.take();
    }

    /// Removes the folder from Explorer's cloud folders; online-only
    /// placeholders go with it, downloaded files stay.
    pub fn retirer(mut self) -> Result<(), String> {
        self.demonter();
        self.id.unregister().map_err(|e| e.to_string())
    }
}

impl Drop for MontageWindows {
    fn drop(&mut self) {
        self.demonter();
    }
}

/// Registers `point` (empty the first time) as a cloud folder showing
/// `assets`, and connects to it.
pub fn monter(point: &Path, assets: &[AssetDistant], rangement: Rangement, source: Arc<dyn Source>) -> Result<MontageWindows, String> {
    let utilisateur = SecurityId::current_user().map_err(|e| e.to_string())?;
    let id = SyncRootIdBuilder::new(identite(point)).user_security_id(utilisateur).build();
    if !id.is_registered().map_err(|e| e.to_string())? {
        if std::fs::read_dir(point).map_err(|e| e.to_string())?.next().is_some() {
            return Err("le dossier doit être vide : ses fichiers seraient masqués".into());
        }
        let info = SyncRootInfo::default()
            .with_display_name("Nicéphore")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_hydration_type(HydrationType::Full)
            .with_hydration_policy(HydrationPolicy::AutoDehydrationAllowed)
            .with_population_type(PopulationType::AlwaysFull)
            .with_allow_pinning(true)
            .with_path(point)
            .map_err(|e| e.to_string())?;
        id.register(info).map_err(|e| e.to_string())?;
    }
    let assets_partages: Arc<RwLock<HashMap<String, AssetDistant>>> =
        Arc::new(RwLock::new(assets.iter().map(|a| (a.id.clone(), a.clone())).collect()));
    let fournisseur = Fournisseur { source, assets: Arc::clone(&assets_partages) };
    let connexion = Session::new().connect(point, fournisseur).map_err(|e| e.to_string())?;
    let mut inodes = Inodes::default();
    let arbre = Arbre::construire(assets, rangement, &mut inodes);
    creer_espaces(point, &arbre)?;
    Ok(MontageWindows {
        connexion: Some(connexion),
        id,
        assets: assets_partages,
        inodes: std::sync::Mutex::new(inodes),
        rangement,
        point: point.to_path_buf(),
    })
}
