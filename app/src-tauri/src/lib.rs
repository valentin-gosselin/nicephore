//! Tauri shell of Nicéphore: commands used by the Svelte UI, the
//! `vignette://` protocol serving cached images, and the local video proxy.

mod banc;
mod compte;
mod proxy_video;
mod telechargement;

use api_immich::{
    Authentification, ClientImmich, DetailMois, FiltreChronologie, ModesConnexion, ModificationAssets, Mois,
    TailleImage, Utilisateur,
};
use cache_vignettes::{CacheVignettes, ErreurCache};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::http::{self, StatusCode};
use tauri::{Manager, State};

/// Size budget of the on-disk image cache.
const BUDGET_CACHE: u64 = 4 * 1024 * 1024 * 1024;

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
}

impl Etat {
    fn client(&self) -> Result<Arc<ClientImmich>, String> {
        self.client
            .read()
            .expect("verrou du client")
            .clone()
            .ok_or_else(|| "non connecté".to_string())
    }

    fn definir(&self, client: Option<Arc<ClientImmich>>, origine: Origine, session: bool) {
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
            if let Ok(Some((url, auth))) = tokio::task::spawn_blocking(move || compte::charger(&dossier)).await {
                let session = matches!(auth, Authentification::Session(_));
                match ClientImmich::nouveau(&url, &auth) {
                    Ok(client) => etat.definir(Some(Arc::new(client)), Origine::Trousseau, session),
                    Err(e) => tracing::warn!("compte mémorisé inutilisable : {e}"),
                }
            }
        }
        *cherche = true;
    }
    let Ok(client) = etat.client() else { return Ok(None) };
    let origine = *etat.origine.read().expect("verrou de l'origine");
    let session = *etat.session.read().expect("verrou de la session");
    verifier_connexion(&client, origine, session).await.map(Some)
}

/// Login methods of a server, before any authentication.
#[tauri::command]
async fn modes_connexion(url: String) -> Result<ModesConnexion, String> {
    api_immich::modes_connexion(&url).await.map_err(|e| e.to_string())
}

/// Connects (and optionally remembers) with ready-made credentials.
async fn connecter(etat: &Etat, url: &str, auth: Authentification, memoriser: bool) -> Result<Connexion, String> {
    let client = Arc::new(ClientImmich::nouveau(url, &auth).map_err(|e| e.to_string())?);
    let origine = if memoriser { Origine::Trousseau } else { Origine::Memoire };
    let session = matches!(auth, Authentification::Session(_));
    let connexion = verifier_connexion(&client, origine, session).await?;
    if memoriser {
        let dossier = etat.dossier_config.clone();
        let url = client.url_serveur();
        tokio::task::spawn_blocking(move || compte::enregistrer(&dossier, &url, &auth))
            .await
            .map_err(|e| e.to_string())??;
    }
    etat.definir(Some(client), origine, session);
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
) -> Result<Connexion, String> {
    let session = api_immich::ouvrir_session(&url, &email, &mot_de_passe).await.map_err(|e| e.to_string())?;
    connecter(&etat, &url, Authentification::Session(session.access_token), memoriser).await
}

/// Connects with a server URL and an API key (advanced option).
#[tauri::command]
async fn se_connecter(etat: State<'_, Etat>, url: String, cle: String, memoriser: bool) -> Result<Connexion, String> {
    connecter(&etat, &url, Authentification::CleApi(cle), memoriser).await
}

/// Disconnects: revokes the session on the server (best effort), then
/// forgets the remembered account.
#[tauri::command]
async fn se_deconnecter(etat: State<'_, Etat>) -> Result<(), String> {
    if *etat.session.read().expect("verrou de la session")
        && let Ok(client) = etat.client()
        && let Err(e) = client.fermer_session().await
    {
        tracing::warn!("révocation de la session impossible : {e}");
    }
    let dossier = etat.dossier_config.clone();
    tokio::task::spawn_blocking(move || compte::oublier(&dossier)).await.map_err(|e| e.to_string())??;
    etat.definir(None, Origine::Memoire, false);
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
struct InfosCache {
    octets: u64,
    fichiers: usize,
    budget: u64,
    dossier: String,
}

#[tauri::command]
async fn infos_cache(etat: State<'_, Etat>) -> Result<InfosCache, String> {
    let cache = etat.cache.clone();
    let (octets, fichiers) = tokio::task::spawn_blocking(move || cache.occupation()).await.map_err(|e| e.to_string())?;
    Ok(InfosCache { octets, fichiers, budget: BUDGET_CACHE, dossier: etat.cache.dossier().display().to_string() })
}

#[tauri::command]
async fn vider_cache(etat: State<'_, Etat>) -> Result<(), String> {
    let cache = etat.cache.clone();
    tokio::task::spawn_blocking(move || cache.vider()).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tao=warn,wry=warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dossier_cache = app.path().app_cache_dir()?.join("images");
            let cache = Arc::new(CacheVignettes::nouveau(dossier_cache));
            let depuis_env = client_depuis_environnement();
            let origine = if depuis_env.is_some() { Origine::Environnement } else { Origine::Memoire };
            let client: ClientPartage = Arc::new(RwLock::new(depuis_env));
            let proxy = proxy_video::Proxy::demarrer(client.clone())?;

            let a_elaguer = cache.clone();
            tauri::async_runtime::spawn_blocking(move || match a_elaguer.elaguer(BUDGET_CACHE) {
                Ok(0) => {}
                Ok(libere) => tracing::info!("cache élagué : {} Mo libérés", libere / 1_048_576),
                Err(e) => tracing::warn!("élagage du cache impossible : {e}"),
            });

            app.manage(Etat {
                client,
                origine: RwLock::new(origine),
                session: RwLock::new(false),
                compte_cherche: tokio::sync::Mutex::new(false),
                dossier_config: app.path().app_config_dir()?,
                cache,
                proxy,
            });
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
            modes_connexion,
            se_connecter_mot_de_passe,
            se_connecter,
            se_deconnecter,
            liste_mois,
            detail_mois,
            infos_asset,
            definir_favori,
            definir_archive,
            mettre_corbeille,
            restaurer_corbeille,
            telecharger,
            url_video,
            infos_cache,
            vider_cache,
            banc::mode_banc,
            banc::journal,
            banc::capture,
            banc::quitter,
            banc::stats_images
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
