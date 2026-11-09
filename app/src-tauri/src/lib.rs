//! Tauri shell of Nicéphore: commands used by the Svelte UI, the
//! `vignette://` protocol serving cached images, and the local video proxy.

mod alimentation;
mod banc;
mod coffre;
mod commandes_envoi;
mod compte;
mod fichiers_locaux;
mod mise_a_jour;
mod portable;
mod prechargement;
mod proxy_video;
mod reglages;
#[cfg(target_os = "linux")]
mod service_virtuel;
mod telechargement;
mod virtuels;
mod zone_notification;

use api_immich::{
    Album, AssetResume, Authentification, BilanAlbum, ClientImmich, CriteresRecherche, DetailMois,
    FiltreChronologie, Lieu, Marqueur, ModesConnexion, ModificationAlbum, ModificationAssets, ModificationPersonne,
    Mois, PageResultats, Personne, Souvenir, StylesCarte, Tag, TailleImage, Utilisateur,
};
use cache_vignettes::{CacheVignettes, ErreurCache};
use serde::Serialize;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::http::{self, StatusCode};
use tauri::{Manager, State};

/// Disk budgets of the image cache, per rendering size: grid thumbnails
/// (about 15 KB each, all of them are preloaded) and viewer previews.
const BUDGET_VIGNETTES: u64 = 2 * 1024 * 1024 * 1024;
const BUDGET_APERCUS: u64 = 4 * 1024 * 1024 * 1024;

/// Client shared by the commands, the image protocol and the video proxy.
pub type ClientPartage = Arc<RwLock<Option<Arc<ClientImmich>>>>;

/// Where the current connection comes from.
#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Origine {
    /// `IMMICH_URL` / `IMMICH_API_KEY` (development).
    Environnement,
    /// Remembered in the OS keyring.
    Trousseau,
    /// Portable mode: kept next to the program, sealed with a passphrase.
    Coffre,
    /// Typed in this session, not remembered.
    Memoire,
}

pub struct Etat {
    client: ClientPartage,
    origine: RwLock<Origine>,
    /// The current connection is a session (revoked on logout).
    session: RwLock<bool>,
    /// Set once the remembered account has been looked up.
    compte_cherche: tokio::sync::Mutex<bool>,
    dossier_config: PathBuf,
    cache: Arc<CacheVignettes>,
    proxy: proxy_video::Proxy,
    prechargement: Arc<prechargement::Prechargement>,
    envoi: Arc<envoi::Moteur>,
    /// The upload database could not be opened (shown on the Envoi page).
    erreur_base_envoi: RwLock<Option<String>>,
    emplacements: portable::Emplacements,
    /// Account whose upload database is open.
    compte_envoi: RwLock<Option<String>>,
    reglages: Arc<RwLock<reglages::Reglages>>,
    /// Tray icon (absent when the desktop offers none).
    zone: Option<zone_notification::Zone>,
    /// Portable mode: the `donnees` folder next to the program.
    portable: Option<PathBuf>,
    virtuels: virtuels::Virtuels,
    /// Originals downloaded through the virtual folders.
    originaux: Arc<virtuel::CacheOriginaux>,
    app: tauri::AppHandle,
}

impl Etat {
    /// Closing the window hides it when uploads may go on in the tray.
    fn garder_en_arriere_plan(&self) -> bool {
        self.zone.is_some()
            && self.reglages.read().expect("verrou des réglages").fermer_en_arriere_plan
            && self.envoi.base().regles().is_ok_and(|r| r.iter().any(|r| r.actif))
    }
}

impl Etat {
    fn client(&self) -> Result<Arc<ClientImmich>, String> {
        self.client
            .read()
            .expect("verrou du client")
            .clone()
            .ok_or_else(|| "non connecté".to_string())
    }

    /// Starts preloading thumbnails for the current connection (if enabled).
    fn precharger(&self) {
        if let Ok(client) = self.client() {
            self.prechargement.lancer(client, self.cache.clone());
        }
    }

    /// Opens the upload database of the connected account: each account has
    /// its own folders, so a folder set for one account never goes to
    /// another. The database of earlier versions goes to the first account
    /// connected after the update.
    fn ouvrir_base_envoi(&self, connexion: &Connexion) {
        let cle = compte::cle_compte(&connexion.url_serveur, &connexion.utilisateur.id);
        if self.compte_envoi.read().expect("verrou du compte d'envoi").as_deref() == Some(cle.as_str()) {
            return;
        }
        let chemin = match std::env::var_os("NICEPHORE_BANC_DOSSIER") {
            // Benchmark: a private database, whatever the account.
            Some(dossier) if self.portable.is_none() => PathBuf::from(dossier).join("envoi.sqlite"),
            _ => self.emplacements.donnees.join(format!("envoi-{cle}.sqlite")),
        };
        let ancienne = self.emplacements.donnees.join("envoi.sqlite");
        if !chemin.exists() && ancienne.exists() && chemin.parent() == ancienne.parent() {
            for suffixe in ["", "-wal", "-shm"] {
                let de = PathBuf::from(format!("{}{suffixe}", ancienne.display()));
                if de.exists()
                    && let Err(e) = std::fs::rename(&de, format!("{}{suffixe}", chemin.display()))
                {
                    tracing::warn!("reprise de la base d'envoi impossible : {e}");
                }
            }
        }
        match envoi::Base::ouvrir(&chemin) {
            Ok(base) => {
                self.envoi.remplacer_base(base);
                *self.erreur_base_envoi.write().expect("verrou de l'erreur") = None;
            }
            Err(e) => {
                tracing::error!("base d'envoi {} : {e}", chemin.display());
                self.envoi.remplacer_base(envoi::Base::en_memoire().expect("base en mémoire"));
                *self.erreur_base_envoi.write().expect("verrou de l'erreur") = Some(e.to_string());
            }
        }
        self.virtuels.activer(&self.app, &cle);
        *self.compte_envoi.write().expect("verrou du compte d'envoi") = Some(cle);
    }

    fn definir(&self, client: Option<Arc<ClientImmich>>, origine: Origine, session: bool) {
        self.envoi.definir_client(client.clone());
        *self.client.write().expect("verrou du client") = client;
        *self.origine.write().expect("verrou de l'origine") = origine;
        *self.session.write().expect("verrou de la session") = session;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Connexion {
    url_serveur: String,
    version: String,
    utilisateur: Utilisateur,
    origine: Origine,
    /// Session (email and password) rather than an API key.
    session: bool,
}

async fn verifier_connexion(client: &ClientImmich, origine: Origine, session: bool) -> Result<Connexion, String> {
    client.ping().await.map_err(|e| e.to_string())?;
    let version = client.version().await.map_err(|e| e.to_string())?;
    let utilisateur = client.moi().await.map_err(|e| e.to_string())?;
    Ok(Connexion { url_serveur: client.url_serveur(), version: version.to_string(), utilisateur, origine, session })
}

/// Current connection, if any, after checking it still works. On the first
/// call, looks for a remembered account in the keyring.
#[tauri::command]
async fn etat_connexion(etat: State<'_, Etat>) -> Result<Option<Connexion>, String> {
    {
        let mut cherche = etat.compte_cherche.lock().await;
        if !*cherche && etat.client().is_err() {
            let dossier = etat.dossier_config.clone();
            let portable = etat.portable.is_some();
            if let Ok(compte::Memorise::Pret { url, auth, ancien }) =
                tokio::task::spawn_blocking(move || compte::lire(&dossier, portable)).await
            {
                let session = matches!(auth, Authentification::Session(_));
                match ClientImmich::nouveau(&url, &auth) {
                    Ok(client) => {
                        let client = Arc::new(client);
                        // Old single-account format: converted once its user is known.
                        if ancien && let Ok(moi) = client.moi().await {
                            let (dossier, url) = (etat.dossier_config.clone(), client.url_serveur());
                            let _ = tokio::task::spawn_blocking(move || compte::convertir_ancien(&dossier, &url, &moi, &auth)).await;
                        }
                        etat.definir(Some(client), Origine::Trousseau, session);
                    }
                    Err(e) => tracing::warn!("compte mémorisé inutilisable : {e}"),
                }
            }
        }
        *cherche = true;
    }
    let Ok(client) = etat.client() else { return Ok(None) };
    let origine = *etat.origine.read().expect("verrou de l'origine");
    let session = *etat.session.read().expect("verrou de la session");
    let connexion = verifier_connexion(&client, origine, session).await?;
    etat.ouvrir_base_envoi(&connexion);
    etat.precharger();
    Ok(Some(connexion))
}

/// Remembered accounts, for the account menu.
#[tauri::command]
async fn comptes(etat: State<'_, Etat>) -> Result<Vec<compte::CompteVue>, String> {
    let dossier = etat.dossier_config.clone();
    tokio::task::spawn_blocking(move || compte::lister(&dossier)).await.map_err(|e| e.to_string())
}

/// Switches to another remembered account. `None`: it is sealed and waits
/// for its passphrase (the interface then shows the unlock screen).
#[tauri::command]
async fn changer_compte(etat: State<'_, Etat>, cle: String) -> Result<Option<Connexion>, String> {
    let dossier = etat.dossier_config.clone();
    tokio::task::spawn_blocking(move || compte::activer(&dossier, &cle)).await.map_err(|e| e.to_string())??;
    etat.prechargement.arreter();
    etat.definir(None, Origine::Memoire, false);
    *etat.compte_cherche.lock().await = false;
    etat_connexion(etat).await
}

/// Portable mode: the server of the sealed account waiting for its
/// passphrase, if any (and not connected yet).
#[tauri::command]
async fn compte_verrouille(etat: State<'_, Etat>) -> Result<Option<String>, String> {
    if etat.client().is_ok() {
        return Ok(None);
    }
    let dossier = etat.dossier_config.clone();
    let portable = etat.portable.is_some();
    Ok(match tokio::task::spawn_blocking(move || compte::lire(&dossier, portable)).await.map_err(|e| e.to_string())? {
        compte::Memorise::Verrouille(url) => Some(url),
        _ => None,
    })
}

/// Opens the sealed account with its passphrase.
#[tauri::command]
async fn deverrouiller(etat: State<'_, Etat>, phrase: String) -> Result<Connexion, String> {
    let dossier = etat.dossier_config.clone();
    let (url, auth, ancien) = tokio::task::spawn_blocking(move || compte::deverrouiller(&dossier, &phrase))
        .await
        .map_err(|e| e.to_string())??;
    let client = Arc::new(ClientImmich::nouveau(&url, &auth).map_err(|e| e.to_string())?);
    let session = matches!(auth, Authentification::Session(_));
    let connexion = verifier_connexion(&client, Origine::Coffre, session).await?;
    if ancien {
        let (dossier, url, moi) = (etat.dossier_config.clone(), connexion.url_serveur.clone(), connexion.utilisateur.clone());
        tokio::task::spawn_blocking(move || compte::convertir_ancien(&dossier, &url, &moi, &auth))
            .await
            .map_err(|e| e.to_string())??;
    }
    etat.definir(Some(client), Origine::Coffre, session);
    etat.ouvrir_base_envoi(&connexion);
    etat.precharger();
    Ok(connexion)
}

/// Portable mode: its `donnees` folder.
#[tauri::command]
fn infos_portable(etat: State<'_, Etat>) -> Option<String> {
    etat.portable.as_ref().map(|d| d.to_string_lossy().into_owned())
}

/// Login methods of a server, before any authentication.
#[tauri::command]
async fn modes_connexion(url: String) -> Result<ModesConnexion, String> {
    api_immich::modes_connexion(&url).await.map_err(|e| e.to_string())
}

/// Connects (and optionally remembers) with ready-made credentials. In
/// portable mode, remembering needs a passphrase that seals the secret.
async fn connecter(
    etat: &Etat,
    url: &str,
    auth: Authentification,
    memoriser: bool,
    phrase: Option<String>,
) -> Result<Connexion, String> {
    let portable = etat.portable.is_some();
    if memoriser && portable && phrase.as_deref().is_none_or(|p| p.chars().count() < coffre::LONGUEUR_MINIMALE) {
        return Err(format!("phrase de passe d'au moins {} caractères requise", coffre::LONGUEUR_MINIMALE));
    }
    let client = Arc::new(ClientImmich::nouveau(url, &auth).map_err(|e| e.to_string())?);
    let origine = match (memoriser, portable) {
        (false, _) => Origine::Memoire,
        (true, false) => Origine::Trousseau,
        (true, true) => Origine::Coffre,
    };
    let session = matches!(auth, Authentification::Session(_));
    let connexion = verifier_connexion(&client, origine, session).await?;
    if memoriser {
        let dossier = etat.dossier_config.clone();
        let (url, moi) = (client.url_serveur(), connexion.utilisateur.clone());
        tokio::task::spawn_blocking(move || {
            compte::enregistrer(&dossier, &url, &moi, &auth, phrase.as_deref().filter(|_| portable))
        })
        .await
        .map_err(|e| e.to_string())??;
    }
    etat.prechargement.arreter();
    etat.definir(Some(client), origine, session);
    etat.ouvrir_base_envoi(&connexion);
    etat.precharger();
    Ok(connexion)
}

/// Email and password login, like the web app. Only the session token
/// returned by the server is kept (in the keyring when `memoriser` is set).
#[tauri::command]
async fn se_connecter_mot_de_passe(
    etat: State<'_, Etat>,
    url: String,
    email: String,
    mot_de_passe: String,
    memoriser: bool,
    phrase: Option<String>,
) -> Result<Connexion, String> {
    let session = api_immich::ouvrir_session(&url, &email, &mot_de_passe).await.map_err(|e| e.to_string())?;
    connecter(&etat, &url, Authentification::Session(session.access_token), memoriser, phrase).await
}

/// Connects with a server URL and an API key (advanced option).
#[tauri::command]
async fn se_connecter(
    etat: State<'_, Etat>,
    url: String,
    cle: String,
    memoriser: bool,
    phrase: Option<String>,
) -> Result<Connexion, String> {
    connecter(&etat, &url, Authentification::CleApi(cle), memoriser, phrase).await
}

/// Disconnects: revokes the session on the server (best effort), then
/// forgets this account; another remembered account becomes the active one.
#[tauri::command]
async fn se_deconnecter(etat: State<'_, Etat>) -> Result<(), String> {
    if *etat.session.read().expect("verrou de la session")
        && let Ok(client) = etat.client()
        && let Err(e) = client.fermer_session().await
    {
        tracing::warn!("révocation de la session impossible : {e}");
    }
    etat.prechargement.arreter();
    let dossier = etat.dossier_config.clone();
    tokio::task::spawn_blocking(move || compte::oublier_actif(&dossier)).await.map_err(|e| e.to_string())??;
    // The virtual folders and upload database of this account close too.
    etat.virtuels.oublier();
    *etat.compte_envoi.write().expect("verrou du compte d'envoi") = None;
    etat.definir(None, Origine::Memoire, false);
    *etat.compte_cherche.lock().await = false;
    Ok(())
}

#[tauri::command]
async fn liste_mois(etat: State<'_, Etat>, filtre: FiltreChronologie) -> Result<Vec<Mois>, String> {
    etat.client()?.liste_mois(&filtre).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn detail_mois(etat: State<'_, Etat>, mois: String, filtre: FiltreChronologie) -> Result<DetailMois, String> {
    etat.client()?.detail_mois(&mois, &filtre).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn rechercher(etat: State<'_, Etat>, criteres: CriteresRecherche, page: u32) -> Result<PageResultats, String> {
    etat.client()?.rechercher(&criteres, page).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn lieux(etat: State<'_, Etat>) -> Result<Vec<Lieu>, String> {
    etat.client()?.lieux().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn personnes(etat: State<'_, Etat>, avec_masquees: Option<bool>) -> Result<Vec<Personne>, String> {
    etat.client()?.personnes(avec_masquees.unwrap_or(false)).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn personne(etat: State<'_, Etat>, id: String) -> Result<Personne, String> {
    etat.client()?.personne(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn personnes_proches(etat: State<'_, Etat>, id: String) -> Result<Vec<Personne>, String> {
    etat.client()?.personnes_proches(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn modifier_personne(
    etat: State<'_, Etat>,
    id: String,
    modification: ModificationPersonne,
) -> Result<Personne, String> {
    etat.client()?.modifier_personne(&id, &modification).await.map_err(|e| e.to_string())
}

/// Folds `autres` into `cible` (irreversible): the UI asks for confirmation.
#[tauri::command]
async fn fusionner_personnes(etat: State<'_, Etat>, cible: String, autres: Vec<String>) -> Result<u64, String> {
    etat.client()?.fusionner_personnes(&cible, &autres).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn albums(etat: State<'_, Etat>) -> Result<Vec<Album>, String> {
    etat.client()?.albums().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn album(etat: State<'_, Etat>, id: String) -> Result<Album, String> {
    etat.client()?.album(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn creer_album(
    etat: State<'_, Etat>,
    nom: String,
    description: Option<String>,
    ids: Vec<String>,
) -> Result<Album, String> {
    etat.client()?.creer_album(&nom, description.as_deref().unwrap_or(""), &ids).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn modifier_album(etat: State<'_, Etat>, id: String, modification: ModificationAlbum) -> Result<(), String> {
    etat.client()?.modifier_album(&id, &modification).await.map_err(|e| e.to_string())
}

/// Deletes the album only; its photos stay in the library.
#[tauri::command]
async fn supprimer_album(etat: State<'_, Etat>, id: String) -> Result<(), String> {
    etat.client()?.supprimer_album(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn ajouter_a_l_album(etat: State<'_, Etat>, id: String, ids: Vec<String>) -> Result<BilanAlbum, String> {
    etat.client()?.ajouter_a_l_album(&id, &ids).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn retirer_de_l_album(etat: State<'_, Etat>, id: String, ids: Vec<String>) -> Result<(), String> {
    etat.client()?.retirer_de_l_album(&id, &ids).await.map_err(|e| e.to_string())
}

/// Number of photos per person, 8 requests at a time; failures are left out.
#[tauri::command]
async fn nombres_personnes(etat: State<'_, Etat>, ids: Vec<String>) -> Result<HashMap<String, u64>, String> {
    let client = etat.client()?;
    Ok(futures_util::stream::iter(ids)
        .map(|id| {
            let client = client.clone();
            async move { client.nombre_photos_personne(&id).await.ok().map(|n| (id, n)) }
        })
        .buffer_unordered(8)
        .filter_map(|r| async move { r })
        .collect()
        .await)
}

/// Number of photos per city, 8 requests at a time; failures are left out.
#[tauri::command]
async fn nombres_villes(etat: State<'_, Etat>, villes: Vec<String>) -> Result<HashMap<String, u64>, String> {
    let client = etat.client()?;
    Ok(futures_util::stream::iter(villes)
        .map(|ville| {
            let client = client.clone();
            async move { client.nombre_photos_ville(&ville).await.ok().map(|n| (ville, n)) }
        })
        .buffer_unordered(8)
        .filter_map(|r| async move { r })
        .collect()
        .await)
}

#[tauri::command]
async fn marqueurs(etat: State<'_, Etat>) -> Result<Vec<Marqueur>, String> {
    etat.client()?.marqueurs().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn styles_carte(etat: State<'_, Etat>) -> Result<StylesCarte, String> {
    etat.client()?.styles_carte().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn souvenirs(etat: State<'_, Etat>, jour: String) -> Result<Vec<Souvenir>, String> {
    etat.client()?.souvenirs(&jour).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn souvenir(etat: State<'_, Etat>, id: String) -> Result<Souvenir, String> {
    etat.client()?.souvenir(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn assets_par_ids(etat: State<'_, Etat>, ids: Vec<String>) -> Result<Vec<AssetResume>, String> {
    etat.client()?.assets_par_ids(&ids).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn infos_asset(etat: State<'_, Etat>, id: String) -> Result<serde_json::Value, String> {
    etat.client()?.infos_asset(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn definir_favori(etat: State<'_, Etat>, ids: Vec<String>, favori: bool) -> Result<(), String> {
    let modification = ModificationAssets { favori: Some(favori), ..Default::default() };
    etat.client()?.modifier_assets(&ids, &modification).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn definir_archive(etat: State<'_, Etat>, ids: Vec<String>, archive: bool) -> Result<(), String> {
    let visibilite = if archive { "archive" } else { "timeline" };
    let modification = ModificationAssets { visibilite: Some(visibilite.into()), ..Default::default() };
    etat.client()?.modifier_assets(&ids, &modification).await.map_err(|e| e.to_string())
}

/// Description, rating, date, location of one or many assets.
#[tauri::command]
async fn modifier_infos(etat: State<'_, Etat>, ids: Vec<String>, modification: ModificationAssets) -> Result<(), String> {
    etat.client()?.modifier_assets(&ids, &modification).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn tags(etat: State<'_, Etat>) -> Result<Vec<Tag>, String> {
    etat.client()?.tags().await.map_err(|e| e.to_string())
}

/// Adds (or with `retirer` removes) a tag by name on assets; the tag is
/// created when missing.
#[tauri::command]
async fn etiqueter(etat: State<'_, Etat>, nom: String, ids: Vec<String>, retirer: bool) -> Result<Tag, String> {
    let client = etat.client()?;
    let tag = client.tag_nomme(&nom).await.map_err(|e| e.to_string())?;
    client.etiqueter(&tag.id, &ids, retirer).await.map_err(|e| e.to_string())?;
    Ok(tag)
}

#[tauri::command]
async fn mettre_corbeille(etat: State<'_, Etat>, ids: Vec<String>) -> Result<(), String> {
    etat.client()?.mettre_a_la_corbeille(&ids).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn restaurer_corbeille(etat: State<'_, Etat>, ids: Vec<String>) -> Result<(), String> {
    etat.client()?.restaurer(&ids).await.map_err(|e| e.to_string())
}

/// Downloads the original files into `dossier`, emitting `telechargement`
/// progress events.
#[tauri::command]
async fn telecharger(
    app: tauri::AppHandle,
    etat: State<'_, Etat>,
    ids: Vec<String>,
    dossier: PathBuf,
) -> Result<telechargement::Bilan, String> {
    if !dossier.is_dir() {
        return Err(format!("dossier introuvable : {}", dossier.display()));
    }
    let client = etat.client()?;
    Ok(telechargement::telecharger(app, &client, &ids, &dossier).await)
}

/// URL of the local proxy streaming a video (the `<video>` element cannot
/// send the API key itself).
#[tauri::command]
fn url_video(etat: State<'_, Etat>, id: String) -> Result<String, String> {
    api_immich::valider_id(&id).map_err(|e| e.to_string())?;
    Ok(etat.proxy.url_video(&id))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OccupationCache {
    octets: u64,
    fichiers: usize,
    budget: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InfosCache {
    vignettes: OccupationCache,
    apercus: OccupationCache,
    dossier: String,
}

#[tauri::command]
async fn infos_cache(etat: State<'_, Etat>) -> Result<InfosCache, String> {
    let cache = etat.cache.clone();
    let (vignettes, apercus) = tokio::task::spawn_blocking(move || {
        (cache.occupation(TailleImage::Vignette), cache.occupation(TailleImage::Apercu))
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(InfosCache {
        vignettes: OccupationCache { octets: vignettes.0, fichiers: vignettes.1, budget: BUDGET_VIGNETTES },
        apercus: OccupationCache { octets: apercus.0, fichiers: apercus.1, budget: BUDGET_APERCUS },
        dossier: etat.cache.dossier().display().to_string(),
    })
}

#[tauri::command]
fn etat_prechargement(etat: State<'_, Etat>) -> prechargement::EtatPrechargement {
    etat.prechargement.etat()
}

/// Turns thumbnail preloading on or off (remembered across launches).
#[tauri::command]
fn definir_prechargement(etat: State<'_, Etat>, actif: bool) -> Result<(), String> {
    etat.prechargement.definir_actif(actif)?;
    if actif {
        etat.precharger();
    }
    Ok(())
}

/// The user is scrolling: preloading pauses for a moment.
#[tauri::command]
fn signaler_defilement(etat: State<'_, Etat>) {
    etat.prechargement.signaler_activite();
}

#[tauri::command]
async fn vider_cache(etat: State<'_, Etat>) -> Result<(), String> {
    etat.prechargement.arreter();
    etat.prechargement.oublier_complet();
    let cache = etat.cache.clone();
    tokio::task::spawn_blocking(move || cache.vider()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
    // Thumbnails are fetched again in the background when preloading is on.
    etat.precharger();
    Ok(())
}

/// Serves `vignette://localhost/<taille>/<id>?v=<thumbhash>`.
async fn servir_image(
    client: Option<Arc<ClientImmich>>,
    cache: Arc<CacheVignettes>,
    uri: http::Uri,
) -> http::Response<Vec<u8>> {
    let reponse = |statut: StatusCode| {
        http::Response::builder()
            .status(statut)
            .header("Access-Control-Allow-Origin", "*")
            .body(Vec::new())
            .expect("réponse valide")
    };
    let Some(client) = client else { return reponse(StatusCode::SERVICE_UNAVAILABLE) };
    let mut morceaux = uri.path().trim_start_matches('/').split('/');
    let (Some(taille), Some(id)) = (morceaux.next().and_then(TailleImage::depuis_nom), morceaux.next())
    else {
        return reponse(StatusCode::BAD_REQUEST);
    };
    let version = uri
        .query()
        .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("v=")))
        .unwrap_or("");

    match cache.obtenir(&client, id, taille, version).await {
        Ok(image) => http::Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", image.type_mime)
            .header("Cache-Control", "max-age=31536000, immutable")
            .header("Access-Control-Allow-Origin", "*")
            .body(image.octets.to_vec())
            .expect("réponse valide"),
        Err(ErreurCache::Introuvable) => reponse(StatusCode::NOT_FOUND),
        Err(ErreurCache::Autre(message)) => {
            tracing::warn!("image {id} : {message}");
            reponse(StatusCode::BAD_GATEWAY)
        }
    }
}

/// Builds a client from `IMMICH_URL` and `IMMICH_API_KEY` (development aid).
fn client_depuis_environnement() -> Option<Arc<ClientImmich>> {
    let url = std::env::var("IMMICH_URL").ok()?;
    let cle = std::env::var("IMMICH_API_KEY").ok()?;
    match ClientImmich::nouveau(&url, &Authentification::CleApi(cle)) {
        Ok(client) => Some(Arc::new(client)),
        Err(e) => {
            tracing::warn!("IMMICH_URL / IMMICH_API_KEY ignorés : {e}");
            None
        }
    }
}

pub fn lancer() {
    // The virtual folders service runs without window.
    #[cfg(target_os = "linux")]
    if std::env::args().any(|a| a == service_virtuel::ARG_SERVICE) {
        return service_virtuel::executer();
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tao=warn,wry=warn".into()),
        )
        .init();

    // Photos opened from the file manager (default viewer).
    let fichiers_initiaux = fichiers_locaux::fichiers_dans_args(&std::env::args().collect::<Vec<_>>());

    tauri::Builder::default()
        // A second launch (another photo opened) goes to the running window.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _dossier| fichiers_locaux::seconde_instance(app, argv)))
        .manage(fichiers_locaux::FichiersInitiaux(std::sync::Mutex::new(Some(fichiers_initiaux))))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(mise_a_jour::MisesAJour::default())
        .on_window_event(|fenetre, evenement| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = evenement
                && fenetre.label() == "main"
                && fenetre.app_handle().try_state::<Etat>().is_some_and(|e| e.garder_en_arriere_plan())
            {
                api.prevent_close();
                let _ = fenetre.hide();
                commandes_envoi::prevenir_arriere_plan(fenetre.app_handle());
            }
        })
        .setup(|app| {
            let portable = portable::detecter();
            let emplacements = match &portable {
                Some(racine) => portable::Emplacements::portables(racine),
                None => portable::Emplacements {
                    config: app.path().app_config_dir()?,
                    cache: app.path().app_cache_dir()?,
                    donnees: app.path().app_data_dir()?,
                    portable: None,
                },
            };
            if let Some(racine) = &portable {
                tracing::info!("mode portable : tout dans {}", racine.display());
            }
            // The window is created here (not from the configuration) so its
            // web view profile follows portable mode.
            let mut fenetre = tauri::WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?;
            if let Some(profil) = emplacements.profil_web() {
                fenetre = fenetre.data_directory(profil);
            }
            fenetre.build()?;
            let dossier_cache = emplacements.cache.join("images");
            let cache = Arc::new(CacheVignettes::nouveau(dossier_cache));
            let depuis_env = client_depuis_environnement();
            let origine = if depuis_env.is_some() { Origine::Environnement } else { Origine::Memoire };
            let client: ClientPartage = Arc::new(RwLock::new(depuis_env));
            let proxy = proxy_video::Proxy::demarrer(client.clone())?;

            let a_elaguer = cache.clone();
            tauri::async_runtime::spawn_blocking(move || {
                for (taille, budget) in [(TailleImage::Vignette, BUDGET_VIGNETTES), (TailleImage::Apercu, BUDGET_APERCUS)] {
                    match a_elaguer.elaguer(taille, budget) {
                        Ok(0) => {}
                        Ok(libere) => tracing::info!("cache {} élagué : {} Mo libérés", taille.nom(), libere / 1_048_576),
                        Err(e) => tracing::warn!("élagage du cache impossible : {e}"),
                    }
                }
            });
            let dossier_config = emplacements.config.clone();
            let dossier_originaux = emplacements.cache.join("originaux");
            let prechargement = Arc::new(prechargement::Prechargement::nouveau(&dossier_config, app.handle().clone()));

            // Each account has its upload database, opened once connected.
            let base = envoi::Base::en_memoire().expect("base en mémoire");
            let reglages = Arc::new(RwLock::new(reglages::charger(&dossier_config)));
            let moteur = envoi::Moteur::nouveau(base);
            moteur.definir_client(client.read().expect("verrou du client").clone());
            let pour_condition = Arc::clone(&reglages);
            moteur.definir_condition(Arc::new(move || {
                let r = pour_condition.read().expect("verrou des réglages").clone();
                if r.pause_sur_batterie && alimentation::sur_batterie() {
                    Some("sur batterie".into())
                } else if r.pause_reseau_facture && alimentation::reseau_facture() {
                    Some("réseau facturé".into())
                } else {
                    None
                }
            }));
            commandes_envoi::demarrer(&moteur, app.handle().clone());
            let zone = zone_notification::creer(app.handle())
                .map_err(|e| tracing::warn!("pas d'icône dans la zone de notification : {e}"))
                .ok();

            app.manage(Etat {
                client,
                origine: RwLock::new(origine),
                session: RwLock::new(false),
                compte_cherche: tokio::sync::Mutex::new(false),
                dossier_config,
                cache,
                proxy,
                prechargement,
                envoi: moteur,
                erreur_base_envoi: RwLock::new(None),
                emplacements,
                compte_envoi: RwLock::new(None),
                reglages,
                zone,
                portable,
                virtuels: virtuels::Virtuels::default(),
                originaux: Arc::new(virtuel::CacheOriginaux::nouveau(dossier_originaux, virtuels::BUDGET_ORIGINAUX)),
                app: app.handle().clone(),
            });
            virtuels::actualiser_regulierement(app.handle().clone());
            mise_a_jour::verifier_au_demarrage(app.handle().clone());
            // Started with the session: stay in the tray if uploads can
            // go on there, else show the window as usual.
            let etat = app.state::<Etat>();
            let arriere_plan = std::env::args().any(|a| a == reglages::ARG_ARRIERE_PLAN) && etat.garder_en_arriere_plan();
            if !arriere_plan && let Some(fenetre) = app.get_webview_window("main") {
                fenetre.show()?;
            }
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("vignette", |ctx, requete, repondeur| {
            let etat = ctx.app_handle().state::<Etat>();
            let client = etat.client.read().expect("verrou du client").clone();
            let cache = etat.cache.clone();
            let uri = requete.uri().clone();
            tauri::async_runtime::spawn(async move {
                let debut = std::time::Instant::now();
                repondeur.respond(servir_image(client, cache, uri).await);
                banc::compter_image(debut.elapsed());
            });
        })
        .invoke_handler(tauri::generate_handler![
            etat_connexion,
            comptes,
            changer_compte,
            compte_verrouille,
            deverrouiller,
            infos_portable,
            modes_connexion,
            se_connecter_mot_de_passe,
            se_connecter,
            se_deconnecter,
            liste_mois,
            detail_mois,
            rechercher,
            lieux,
            personnes,
            personne,
            personnes_proches,
            modifier_personne,
            fusionner_personnes,
            albums,
            album,
            creer_album,
            modifier_album,
            supprimer_album,
            ajouter_a_l_album,
            retirer_de_l_album,
            nombres_personnes,
            nombres_villes,
            marqueurs,
            styles_carte,
            souvenirs,
            souvenir,
            assets_par_ids,
            infos_asset,
            definir_favori,
            definir_archive,
            mettre_corbeille,
            modifier_infos,
            tags,
            etiqueter,
            restaurer_corbeille,
            telecharger,
            url_video,
            infos_cache,
            vider_cache,
            etat_prechargement,
            definir_prechargement,
            signaler_defilement,
            mise_a_jour::infos_mise_a_jour,
            mise_a_jour::chercher_mise_a_jour,
            mise_a_jour::installer_mise_a_jour,
            fichiers_locaux::fichiers_au_demarrage,
            fichiers_locaux::ouvrir_fichier_local,
            fichiers_locaux::etat_fichier_local,
            fichiers_locaux::envoyer_fichier_local,
            fichiers_locaux::definir_visionneuse_par_defaut,
            banc::lancer_seconde_instance,
            virtuels::virtuels,
            virtuels::ajouter_virtuel,
            virtuels::retirer_virtuel,
            virtuels::cache_originaux,
            virtuels::vider_cache_originaux,
            commandes_envoi::regles_envoi,
            commandes_envoi::estimer_dossier,
            commandes_envoi::ajouter_reception,
            commandes_envoi::estimer_reception,
            commandes_envoi::recevoir_de_nouveau,
            commandes_envoi::ajouter_double,
            commandes_envoi::definir_miroir,
            commandes_envoi::suppressions_en_attente,
            commandes_envoi::confirmer_suppressions,
            commandes_envoi::refuser_suppressions,
            commandes_envoi::reglages_app,
            commandes_envoi::definir_reglages_app,
            commandes_envoi::definir_demarrage_auto,
            commandes_envoi::ajouter_regle_envoi,
            commandes_envoi::activer_regle_envoi,
            commandes_envoi::supprimer_regle_envoi,
            commandes_envoi::etat_envoi,
            commandes_envoi::lancer_envoi,
            commandes_envoi::pause_envoi,
            commandes_envoi::journal_envoi,
            commandes_envoi::echecs_envoi,
            commandes_envoi::reessayer_envoi,
            banc::dossier_photos_banc,
            banc::dossier_banc_vide,
            banc::identifiants_banc,
            banc::fermer_fenetre,
            banc::fenetre_visible,
            banc::montrer_fenetre,
            banc::mode_banc,
            banc::scenario_banc,
            banc::journal,
            banc::capture,
            banc::quitter,
            banc::stats_images
        ])
        .build(tauri::generate_context!())
        .expect("erreur au lancement de l'application")
        .run(|app, evenement| {
            // Virtual folders are unmounted cleanly on the way out.
            if let tauri::RunEvent::Exit = evenement
                && let Some(etat) = app.try_state::<Etat>()
            {
                etat.virtuels.demonter_tout();
            }
        });
}
