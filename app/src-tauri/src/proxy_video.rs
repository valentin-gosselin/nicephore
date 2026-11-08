//! Local HTTP proxy for video playback.
//!
//! The webview `<video>` element cannot add the `x-api-key` header, so it
//! reads `http://127.0.0.1:<port>/video/<id>?jeton=<token>` instead. The proxy
//! only listens on loopback, requires a random per-launch token, forwards the
//! `Range` header (seeking) and streams the body without buffering it.

use crate::ClientPartage;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Deserialize;
use std::sync::Arc;

pub struct Proxy {
    port: u16,
    jeton: Arc<str>,
}

#[derive(Clone)]
struct Contexte {
    client: ClientPartage,
    jeton: Arc<str>,
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

        let routeur = axum::Router::new()
            .route("/video/{id}", get(servir_video))
            .with_state(Contexte { client, jeton: jeton.clone() });
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
        Ok(Self { port, jeton })
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
