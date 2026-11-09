//! Local HTTP proxy for video playback.
//!
//! The webview `<video>` element cannot add the `x-api-key` header, so it
//! reads `http://127.0.0.1:<port>/video/<id>?jeton=<token>` instead. The proxy
//! only listens on loopback, requires a random per-launch token, forwards the
//! `Range` header (seeking) and streams the body without buffering it.
//!
//! It also serves videos of this computer opened in the local viewer
//! (`/local/<n>`): WebKitGTK cannot play videos from Tauri's own protocols.
//! Only files registered by the viewer are served, by number, never by path.

use crate::ClientPartage;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

pub struct Proxy {
    port: u16,
    jeton: Arc<str>,
    locaux: Arc<Mutex<HashMap<u64, PathBuf>>>,
    suivant: AtomicU64,
}

#[derive(Clone)]
struct Contexte {
    client: ClientPartage,
    jeton: Arc<str>,
    locaux: Arc<Mutex<HashMap<u64, PathBuf>>>,
}

#[derive(Deserialize)]
struct Parametres {
    jeton: String,
}

impl Proxy {
    pub fn demarrer(client: ClientPartage) -> std::io::Result<Self> {
        let ecouteur = std::net::TcpListener::bind(("127.0.0.1", 0))?;
        ecouteur.set_nonblocking(true)?;
        let port = ecouteur.local_addr()?.port();
        let jeton: Arc<str> = format!("{:032x}", rand::random::<u128>()).into();

        let locaux: Arc<Mutex<HashMap<u64, PathBuf>>> = Arc::default();
        let routeur = axum::Router::new()
            .route("/video/{id}", get(servir_video))
            .route("/local/{n}", get(servir_local))
            .with_state(Contexte { client, jeton: jeton.clone(), locaux: Arc::clone(&locaux) });
        tauri::async_runtime::spawn(async move {
            match tokio::net::TcpListener::from_std(ecouteur) {
                Ok(ecouteur) => {
                    if let Err(e) = axum::serve(ecouteur, routeur).await {
                        tracing::error!("proxy vidéo arrêté : {e}");
                    }
                }
                Err(e) => tracing::error!("proxy vidéo impossible : {e}"),
            }
        });
        tracing::info!("proxy vidéo sur 127.0.0.1:{port}");
        Ok(Self { port, jeton, locaux, suivant: AtomicU64::new(1) })
    }

    /// URL of a local video opened in the viewer (registered here first).
    pub fn url_locale(&self, chemin: PathBuf) -> String {
        let mut locaux = self.locaux.lock().expect("verrou des vidéos locales");
        let n = match locaux.iter().find(|(_, c)| **c == chemin) {
            Some((n, _)) => *n,
            None => {
                let n = self.suivant.fetch_add(1, Ordering::Relaxed);
                locaux.insert(n, chemin);
                n
            }
        };
        format!("http://127.0.0.1:{}/local/{n}?jeton={}", self.port, self.jeton)
    }

    pub fn url_video(&self, id: &str) -> String {
        format!("http://127.0.0.1:{}/video/{id}?jeton={}", self.port, self.jeton)
    }
}

async fn servir_video(
    State(contexte): State<Contexte>,
    Path(id): Path<String>,
    Query(parametres): Query<Parametres>,
    entetes: HeaderMap,
) -> Response {
    if parametres.jeton.as_bytes() != contexte.jeton.as_bytes() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(client) = contexte.client.read().expect("verrou du client").clone() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let plage = entetes.get(header::RANGE).and_then(|v| v.to_str().ok());
    let amont = match client.flux_video(&id, plage).await {
        Ok(reponse) => reponse,
        Err(api_immich::Erreur::Introuvable) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::warn!("vidéo {id} : {e}");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let mut reponse = Response::builder().status(amont.status().as_u16());
    for nom in [
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
        header::ETAG,
        header::LAST_MODIFIED,
    ] {
        if let Some(valeur) = amont.headers().get(nom.as_str()) {
            reponse = reponse.header(nom, valeur.as_bytes());
        }
    }
    reponse
        .body(Body::from_stream(amont.bytes_stream()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// Type announced for a local video, from its extension.
fn type_video(chemin: &std::path::Path) -> &'static str {
    match chemin.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",
        Some("mkv") => "video/x-matroska",
        Some("avi") => "video/x-msvideo",
        Some("3gp") => "video/3gpp",
        _ => "video/mp4",
    }
}

/// `bytes=a-b`, `bytes=a-` or `bytes=-n` against a file of `taille` bytes:
/// the inclusive range to send, or `None` when it cannot be satisfied.
fn plage_demandee(valeur: &str, taille: u64) -> Option<(u64, u64)> {
    let (debut, fin) = valeur.strip_prefix("bytes=")?.split(',').next()?.trim().split_once('-')?;
    let (debut, fin) = match (debut.trim(), fin.trim()) {
        ("", n) => {
            let n: u64 = n.parse().ok()?;
            (taille.saturating_sub(n), taille.checked_sub(1)?)
        }
        (a, "") => (a.parse().ok()?, taille.checked_sub(1)?),
        (a, b) => (a.parse().ok()?, b.parse::<u64>().ok()?.min(taille.checked_sub(1)?)),
    };
    (debut <= fin && fin < taille).then_some((debut, fin))
}

async fn servir_local(
    State(contexte): State<Contexte>,
    Path(n): Path<u64>,
    Query(parametres): Query<Parametres>,
    entetes: HeaderMap,
) -> Response {
    if parametres.jeton.as_bytes() != contexte.jeton.as_bytes() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(chemin) = contexte.locaux.lock().expect("verrou des vidéos locales").get(&n).cloned() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(mut fichier) = tokio::fs::File::open(&chemin).await else { return StatusCode::NOT_FOUND.into_response() };
    let Ok(taille) = fichier.metadata().await.map(|m| m.len()) else { return StatusCode::NOT_FOUND.into_response() };
    let demande = entetes.get(header::RANGE).and_then(|v| v.to_str().ok());
    let (statut, debut, fin) = match demande {
        None => (StatusCode::OK, 0, taille.saturating_sub(1)),
        Some(valeur) => match plage_demandee(valeur, taille) {
            Some((d, f)) => (StatusCode::PARTIAL_CONTENT, d, f),
            None => {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header(header::CONTENT_RANGE, format!("bytes */{taille}"))
                    .body(Body::empty())
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
            }
        },
    };
    if fichier.seek(std::io::SeekFrom::Start(debut)).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    let longueur = if taille == 0 { 0 } else { fin - debut + 1 };
    let flux = tokio_util::io::ReaderStream::new(fichier.take(longueur));
    let mut reponse = Response::builder()
        .status(statut)
        .header(header::CONTENT_TYPE, type_video(&chemin))
        .header(header::CONTENT_LENGTH, longueur)
        .header(header::ACCEPT_RANGES, "bytes");
    if statut == StatusCode::PARTIAL_CONTENT {
        reponse = reponse.header(header::CONTENT_RANGE, format!("bytes {debut}-{fin}/{taille}"));
    }
    reponse.body(Body::from_stream(flux)).unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[cfg(test)]
mod tests {
    use super::plage_demandee;

    #[test]
    fn plages() {
        assert_eq!(plage_demandee("bytes=0-", 100), Some((0, 99)));
        assert_eq!(plage_demandee("bytes=10-19", 100), Some((10, 19)));
        assert_eq!(plage_demandee("bytes=90-200", 100), Some((90, 99)));
        assert_eq!(plage_demandee("bytes=-10", 100), Some((90, 99)));
        assert_eq!(plage_demandee("bytes=100-", 100), None);
        assert_eq!(plage_demandee("octets=0-1", 100), None);
    }
}
