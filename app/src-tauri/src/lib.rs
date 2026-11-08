//! Tauri shell of the desktop app: commands used by the Svelte UI, the
//! `vignette://` protocol serving cached images, and the local video proxy.

mod proxy_video;

use api_immich::{ClientImmich, DetailMois, Mois, TailleImage, Utilisateur};
use cache_vignettes::{CacheVignettes, ErreurCache};
use serde::Serialize;
use std::sync::{Arc, RwLock};
use tauri::http::{self, StatusCode};
use tauri::{Manager, State};

/// Size budget of the on-disk image cache.
const BUDGET_CACHE: u64 = 4 * 1024 * 1024 * 1024;

/// Client shared by the commands, the image protocol and the video proxy.
pub type ClientPartage = Arc<RwLock<Option<Arc<ClientImmich>>>>;

pub struct Etat {
    client: ClientPartage,
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Connexion {
    url_serveur: String,
    version: String,
    utilisateur: Utilisateur,
}

async fn verifier_connexion(client: &ClientImmich) -> Result<Connexion, String> {
    client.ping().await.map_err(|e| e.to_string())?;
    let version = client.version().await.map_err(|e| e.to_string())?;
    let utilisateur = client.moi().await.map_err(|e| e.to_string())?;
    Ok(Connexion { url_serveur: client.url_serveur(), version: version.to_string(), utilisateur })
}

/// Returns the current connection, if any, after checking it still works.
#[tauri::command]
async fn etat_connexion(etat: State<'_, Etat>) -> Result<Option<Connexion>, String> {
    let Ok(client) = etat.client() else { return Ok(None) };
    verifier_connexion(&client).await.map(Some)
}

/// Connects with a server URL and an API key. The key stays in memory only
/// (keyring storage comes with the settings screen).
#[tauri::command]
async fn se_connecter(etat: State<'_, Etat>, url: String, cle: String) -> Result<Connexion, String> {
    let client = Arc::new(ClientImmich::nouveau(&url, &cle).map_err(|e| e.to_string())?);
    let connexion = verifier_connexion(&client).await?;
    *etat.client.write().expect("verrou du client") = Some(client);
    Ok(connexion)
}

#[tauri::command]
async fn liste_mois(etat: State<'_, Etat>) -> Result<Vec<Mois>, String> {
    etat.client()?.liste_mois().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn detail_mois(etat: State<'_, Etat>, mois: String) -> Result<DetailMois, String> {
    etat.client()?.detail_mois(&mois).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn infos_asset(etat: State<'_, Etat>, id: String) -> Result<serde_json::Value, String> {
    etat.client()?.infos_asset(&id).await.map_err(|e| e.to_string())
}

/// URL of the local proxy streaming a video (the `<video>` element cannot
/// send the API key itself).
#[tauri::command]
fn url_video(etat: State<'_, Etat>, id: String) -> Result<String, String> {
    api_immich::valider_id(&id).map_err(|e| e.to_string())?;
    Ok(etat.proxy.url_video(&id))
}

/// Benchmark mode (development): enabled when `NICEPHORE_BANC_DOSSIER` points
/// to a directory receiving screenshots and measurements.
fn dossier_banc() -> Option<std::path::PathBuf> {
    std::env::var_os("NICEPHORE_BANC_DOSSIER").map(Into::into)
}

#[tauri::command]
fn mode_banc() -> bool {
    dossier_banc().is_some()
}

#[tauri::command]
fn journal(message: String) {
    tracing::info!(target: "banc", "{message}");
}

/// Takes a screenshot of the (virtual) screen with Spectacle.
#[tauri::command]
async fn capture(nom: String) -> Result<(), String> {
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
fn quitter(app: tauri::AppHandle) {
    app.exit(0);
}

/// Timing of the image protocol, read by the benchmark.
static STATS_IMAGES: std::sync::Mutex<(u64, u64, u64)> = std::sync::Mutex::new((0, 0, 0));

#[tauri::command]
fn stats_images() -> String {
    let mut stats = STATS_IMAGES.lock().expect("verrou des stats");
    let (nombre, total, max) = *stats;
    *stats = (0, 0, 0);
    let moyenne = total.checked_div(nombre).unwrap_or(0);
    format!("{nombre} images servies, {moyenne} ms en moyenne, {max} ms au pire")
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
    match ClientImmich::nouveau(&url, &cle) {
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
        .setup(|app| {
            let dossier_cache = app.path().app_cache_dir()?.join("images");
            let cache = Arc::new(CacheVignettes::nouveau(dossier_cache));
            let client: ClientPartage = Arc::new(RwLock::new(client_depuis_environnement()));
            let proxy = proxy_video::Proxy::demarrer(client.clone())?;

            let a_elaguer = cache.clone();
            tauri::async_runtime::spawn_blocking(move || match a_elaguer.elaguer(BUDGET_CACHE) {
                Ok(0) => {}
                Ok(libere) => tracing::info!("cache élagué : {} Mo libérés", libere / 1_048_576),
                Err(e) => tracing::warn!("élagage du cache impossible : {e}"),
            });

            app.manage(Etat { client, cache, proxy });
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
                let ms = debut.elapsed().as_millis() as u64;
                let mut stats = STATS_IMAGES.lock().expect("verrou des stats");
                *stats = (stats.0 + 1, stats.1 + ms, stats.2.max(ms));
            });
        })
        .invoke_handler(tauri::generate_handler![
            etat_connexion,
            se_connecter,
            liste_mois,
            detail_mois,
            infos_asset,
            url_video,
            mode_banc,
            journal,
            capture,
            quitter,
            stats_images
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de l'application");
}
