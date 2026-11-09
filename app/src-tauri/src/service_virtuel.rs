//! Linux: the virtual folders served by a small process without window
//! (`nicephore --dossiers-virtuels`), so they stay usable when Nicéphore is
//! closed. It follows the active account, mounts and unmounts the folders
//! listed by the app (`virtuels-<account>.json`), refreshes them every
//! 5 minutes, and writes its state for the app (`virtuels-etat.json`).
//! One instance at a time (lock file); it stops by itself when the active
//! account has no virtual folder left.

use api_immich::{Authentification, ClientImmich};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::virtuels::{DossierVirtuel, lire, rangement};
use crate::compte;

/// Argument that starts the service instead of the app.
pub const ARG_SERVICE: &str = "--dossiers-virtuels";
const IDENTIFIANT: &str = "io.github.valentin-gosselin.nicephore";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EtatDossier {
    pub monte: bool,
    pub fichiers: usize,
    pub erreur: Option<String>,
}

/// What the service publishes for the app.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EtatService {
    pub pid: u32,
    pub compte: Option<String>,
    pub dossiers: BTreeMap<i64, EtatDossier>,
}

pub fn dossier_config() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(IDENTIFIANT))
}

fn dossier_cache() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join(IDENTIFIANT))
}

pub fn fichier_etat(config: &Path) -> PathBuf {
    config.join("virtuels-etat.json")
}

/// The service's state, if one is running.
pub fn lire_etat(config: &Path) -> Option<EtatService> {
    let etat: EtatService = serde_json::from_str(&std::fs::read_to_string(fichier_etat(config)).ok()?).ok()?;
    // A state left by a crashed service does not count.
    Path::new(&format!("/proc/{}", etat.pid)).exists().then_some(etat)
}

/// Starts the service unless one runs already. Detached from the app: it
/// survives it. An AppImage starts the AppImage itself (its mount point
/// goes away with the app).
pub fn demarrer() -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    if let Some(config) = dossier_config()
        && lire_etat(&config).is_some()
    {
        return Ok(());
    }
    let programme = std::env::var_os("APPIMAGE").map(PathBuf::from).map_or_else(std::env::current_exe, Ok).map_err(|e| e.to_string())?;
    let journal = dossier_cache()
        .map(|d| {
            let _ = std::fs::create_dir_all(&d);
            d.join("dossiers-virtuels.log")
        })
        .and_then(|c| std::fs::File::create(c).ok());
    let mut commande = std::process::Command::new(programme);
    commande.arg(ARG_SERVICE).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null());
    match journal {
        Some(f) => commande.stderr(f),
        None => commande.stderr(std::process::Stdio::null()),
    };
    // Its own session: closing the app (or its terminal) does not stop it.
    unsafe {
        commande.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    commande.spawn().map(|_| ()).map_err(|e| format!("service des dossiers virtuels : {e}"))
}

/// Session autostart of the service (only while virtual folders exist).
pub fn definir_demarrage(actif: bool) -> Result<(), String> {
    let Some(config) = dirs::config_dir() else { return Ok(()) };
    let entree = config.join("autostart").join("nicephore-dossiers-virtuels.desktop");
    if !actif {
        return match std::fs::remove_file(&entree) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        };
    }
    let programme = std::env::var_os("APPIMAGE").map(PathBuf::from).map_or_else(std::env::current_exe, Ok).map_err(|e| e.to_string())?;
    let chemin = programme.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"").replace('`', "\\`").replace('$', "\\$");
    std::fs::create_dir_all(entree.parent().expect("dossier autostart")).map_err(|e| e.to_string())?;
    std::fs::write(
        &entree,
        format!(
            "[Desktop Entry]\nType=Application\nName=Nicéphore, dossiers virtuels\n\
             Comment=Garde les dossiers virtuels de Nicéphore accessibles\n\
             Exec=\"{chemin}\" {ARG_SERVICE}\nIcon=nicephore\nNoDisplay=true\nTerminal=false\n"
        ),
    )
    .map_err(|e| e.to_string())
}

/// The active account and its client: `IMMICH_URL`/`IMMICH_API_KEY` first
/// (development, tests), else the remembered account (keyring).
async fn compte_actif(config: &Path) -> Option<(String, Arc<ClientImmich>)> {
    let (url, auth) = match (std::env::var("IMMICH_URL"), std::env::var("IMMICH_API_KEY")) {
        (Ok(url), Ok(cle)) if !url.is_empty() && !cle.is_empty() => (url, Authentification::CleApi(cle)),
        _ => {
            let config = config.to_path_buf();
            match tokio::task::spawn_blocking(move || compte::lire(&config, false)).await.ok()? {
                compte::Memorise::Pret { url, auth, .. } => (url, auth),
                _ => return None,
            }
        }
    };
    let client = Arc::new(ClientImmich::nouveau(&url, &auth).ok()?);
    let moi = client.moi().await.ok()?;
    Some((compte::cle_compte(&client.url_serveur(), &moi.id), client))
}

struct Montes {
    #[cfg(target_os = "linux")]
    montages: HashMap<i64, (DossierVirtuel, Arc<virtuel::Montage>)>,
    erreurs: HashMap<i64, String>,
}

impl Montes {
    fn tout_demonter(&mut self) {
        #[cfg(target_os = "linux")]
        for (_, (_, m)) in self.montages.drain() {
            m.demonter();
        }
        self.erreurs.clear();
    }
}

/// Runs the service (blocking, until stopped or not needed).
pub fn executer() {
    let _ = tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .try_init();
    let Some(config) = dossier_config() else { return };
    let _ = std::fs::create_dir_all(&config);
    // One service at a time.
    let verrou = match std::fs::File::create(config.join(".virtuels.lock")) {
        Ok(f) => f,
        Err(e) => return tracing::error!("verrou impossible : {e}"),
    };
    if verrou.try_lock().is_err() {
        tracing::info!("service des dossiers virtuels déjà lancé");
        return;
    }
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("environnement asynchrone");
    runtime.block_on(boucle(config));
}

async fn boucle(config: PathBuf) {
    use tokio::signal::unix::{SignalKind, signal};
    let cache_dir = dossier_cache().unwrap_or_else(|| config.join("cache")).join("originaux");
    let cache = Arc::new(virtuel::CacheOriginaux::nouveau(cache_dir, crate::virtuels::BUDGET_ORIGINAUX));
    let mut arret = signal(SignalKind::terminate()).expect("signal d'arrêt");
    let mut interruption = signal(SignalKind::interrupt()).expect("signal d'interruption");
    let mut montes = Montes {
        #[cfg(target_os = "linux")]
        montages: HashMap::new(),
        erreurs: HashMap::new(),
    };
    let mut compte: Option<(String, Arc<ClientImmich>)> = None;
    let mut verifie_compte = Instant::now() - Duration::from_secs(3600);
    let mut actualise = Instant::now();
    let mut sans_dossier_depuis: Option<Instant> = None;
    tracing::info!("service des dossiers virtuels lancé");
    loop {
        // The active account, checked again every 30 seconds.
        if verifie_compte.elapsed() > Duration::from_secs(30) {
            verifie_compte = Instant::now();
            let nouveau = compte_actif(&config).await;
            if nouveau.as_ref().map(|c| &c.0) != compte.as_ref().map(|c| &c.0) {
                montes.tout_demonter();
                tracing::info!("compte actif : {}", nouveau.as_ref().map_or("aucun", |c| c.0.as_str()));
                compte = nouveau;
            }
        }
        let liste = compte.as_ref().map(|(cle, _)| lire(&config, cle)).unwrap_or_default();

        #[cfg(target_os = "linux")]
        if let Some((_, client)) = &compte {
            // Removed from the list: unmounted.
            let voulus: HashMap<i64, &DossierVirtuel> = liste.iter().map(|d| (d.id, d)).collect();
            let retires: Vec<i64> = montes
                .montages
                .iter()
                .filter(|(id, (d, _))| voulus.get(id).is_none_or(|v| v.point != d.point || v.selection != d.selection))
                .map(|(id, _)| *id)
                .collect();
            for id in retires {
                if let Some((_, m)) = montes.montages.remove(&id) {
                    m.demonter();
                }
            }
            montes.erreurs.retain(|id, _| voulus.contains_key(id));
            // New in the list: mounted (a failed one is tried again with the refresh).
            for d in &liste {
                if montes.montages.contains_key(&d.id) || (montes.erreurs.contains_key(&d.id) && actualise.elapsed() < Duration::from_secs(300)) {
                    continue;
                }
                match monter(d, Arc::clone(client), Arc::clone(&cache)).await {
                    Ok(m) => {
                        montes.erreurs.remove(&d.id);
                        montes.montages.insert(d.id, (d.clone(), m));
                    }
                    Err(e) => {
                        tracing::warn!("{} : {e}", d.point);
                        montes.erreurs.insert(d.id, e);
                    }
                }
            }
            // Every 5 minutes, the folders follow the server.
            if actualise.elapsed() > Duration::from_secs(300) {
                actualise = Instant::now();
                for (d, m) in montes.montages.values() {
                    match client.assets_de_selection(&d.selection).await {
                        Ok(assets) => m.actualiser(&assets),
                        Err(e) => tracing::info!("actualisation de {} : {e}", d.point),
                    }
                }
            }
        }
        publier(&config, &compte, &liste, &montes);

        // Not needed any more: stops after a minute without folder.
        if liste.is_empty() && compte.is_some() {
            let depuis = *sans_dossier_depuis.get_or_insert_with(Instant::now);
            if depuis.elapsed() > Duration::from_secs(60) {
                tracing::info!("plus de dossier virtuel : arrêt du service");
                break;
            }
        } else {
            sans_dossier_depuis = None;
        }

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(2)) => {}
            _ = arret.recv() => break,
            _ = interruption.recv() => break,
        }
    }
    montes.tout_demonter();
    let _ = std::fs::remove_file(fichier_etat(&config));
    tracing::info!("service des dossiers virtuels arrêté");
}

#[cfg(target_os = "linux")]
async fn monter(d: &DossierVirtuel, client: Arc<ClientImmich>, cache: Arc<virtuel::CacheOriginaux>) -> Result<Arc<virtuel::Montage>, String> {
    let assets = client.assets_de_selection(&d.selection).await.map_err(|e| e.to_string())?;
    let point = PathBuf::from(&d.point);
    let r = rangement(d.rangement);
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || virtuel::monter(&point, &assets, r, cache, client, runtime))
        .await
        .map_err(|e| e.to_string())?
        .map(Arc::new)
}

fn publier(config: &Path, compte: &Option<(String, Arc<ClientImmich>)>, liste: &[DossierVirtuel], montes: &Montes) {
    let mut etat = EtatService { pid: std::process::id(), compte: compte.as_ref().map(|c| c.0.clone()), dossiers: BTreeMap::new() };
    for d in liste {
        #[cfg(target_os = "linux")]
        let (monte, fichiers) = montes.montages.get(&d.id).map_or((false, 0), |(_, m)| (true, m.fichiers()));
        #[cfg(not(target_os = "linux"))]
        let (monte, fichiers) = (false, 0);
        etat.dossiers.insert(d.id, EtatDossier { monte, fichiers, erreur: montes.erreurs.get(&d.id).cloned() });
    }
    let chemin = fichier_etat(config);
    let temporaire = chemin.with_extension("json.tmp");
    if std::fs::write(&temporaire, serde_json::to_string(&etat).unwrap_or_default()).is_ok() {
        let _ = std::fs::rename(&temporaire, &chemin);
    }
}
