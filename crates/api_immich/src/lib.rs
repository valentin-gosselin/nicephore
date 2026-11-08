//! Minimal typed client for the Immich server API.
//!
//! Targets Immich 3.x. Authentication uses an API key sent in the `x-api-key`
//! header. Only the endpoints needed by the desktop app are implemented; the
//! shapes follow the official OpenAPI specification (`@immich/sdk`).

mod erreur;
mod types;

pub use erreur::{Erreur, Resultat};
pub use types::*;

use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;

/// Header carrying the API key on every request.
const ENTETE_CLE: &str = "x-api-key";

/// HTTP client bound to one Immich server and one API key.
#[derive(Clone)]
pub struct ClientImmich {
    http: reqwest::Client,
    /// Base URL ending with `/api/`.
    base: Url,
}

impl std::fmt::Debug for ClientImmich {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the API key.
        f.debug_struct("ClientImmich").field("base", &self.base.as_str()).finish()
    }
}

impl ClientImmich {
    /// Builds a client. `url_serveur` may be given with or without the
    /// trailing `/api` (e.g. `http://192.168.1.10:2283`).
    pub fn nouveau(url_serveur: &str, cle_api: &str) -> Resultat<Self> {
        let base = normaliser_url(url_serveur)?;
        let cle = cle_api.trim();
        if cle.is_empty() {
            return Err(Erreur::Configuration("clé API vide".into()));
        }
        let mut valeur = HeaderValue::from_str(cle)
            .map_err(|_| Erreur::Configuration("clé API invalide".into()))?;
        valeur.set_sensitive(true);
        let mut entetes = HeaderMap::new();
        entetes.insert(ENTETE_CLE, valeur);

        let http = reqwest::Client::builder()
            .default_headers(entetes)
            .user_agent(concat!("nicephore/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(32)
            // Never forward the API key to another host on redirects.
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self { http, base })
    }

    /// Server base URL (without the `/api/` suffix), for display.
    pub fn url_serveur(&self) -> String {
        let mut url = self.base.clone();
        url.set_path(self.base.path().trim_end_matches("api/"));
        url.as_str().trim_end_matches('/').to_string()
    }

    fn url(&self, chemin: &str) -> Url {
        // `chemin` never starts with `/`, so it is appended to `/api/`.
        self.base.join(chemin).expect("chemin d'API valide")
    }

    async fn get_json<T: DeserializeOwned>(
        &self,
        chemin: &str,
        parametres: &[(&str, &str)],
    ) -> Resultat<T> {
        let reponse = self
            .http
            .get(self.url(chemin))
            .query(parametres)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        let reponse = verifier(reponse).await?;
        Ok(reponse.json::<T>().await?)
    }

    /// `GET /server/ping`: cheap reachability probe.
    pub async fn ping(&self) -> Resultat<()> {
        let reponse: Ping = self.get_json("server/ping", &[]).await?;
        if reponse.res == "pong" {
            Ok(())
        } else {
            Err(Erreur::ReponseInattendue("ping sans pong".into()))
        }
    }

    /// `GET /server/version`.
    pub async fn version(&self) -> Resultat<VersionServeur> {
        self.get_json("server/version", &[]).await
    }

    /// `GET /users/me`: also validates the API key.
    pub async fn moi(&self) -> Resultat<Utilisateur> {
        self.get_json("users/me", &[]).await
    }

    /// `GET /timeline/buckets`: one entry per month with its asset count,
    /// newest first. Mirrors the main timeline of the webapp.
    pub async fn liste_mois(&self) -> Resultat<Vec<Mois>> {
        self.get_json(
            "timeline/buckets",
            &[("visibility", "timeline"), ("withStacked", "true"), ("withPartners", "true")],
        )
        .await
    }

    /// `GET /timeline/bucket`: columnar description of every asset of a month.
    pub async fn detail_mois(&self, mois: &str) -> Resultat<DetailMois> {
        self.get_json(
            "timeline/bucket",
            &[
                ("timeBucket", mois),
                ("visibility", "timeline"),
                ("withStacked", "true"),
                ("withPartners", "true"),
            ],
        )
        .await
    }

    /// `GET /assets/{id}`: full metadata (EXIF, people, tags...).
    pub async fn infos_asset(&self, id: &str) -> Resultat<serde_json::Value> {
        valider_id(id)?;
        self.get_json(&format!("assets/{id}"), &[]).await
    }

    /// `GET /assets/{id}/thumbnail?size=...`: rendered image (JPEG or WebP).
    pub async fn image(&self, id: &str, taille: TailleImage) -> Resultat<ImageBrute> {
        valider_id(id)?;
        let reponse = self
            .http
            .get(self.url(&format!("assets/{id}/thumbnail")))
            .query(&[("size", taille.parametre())])
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
        let reponse = verifier(reponse).await?;
        let type_mime = reponse
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let octets: Bytes = reponse.bytes().await?;
        Ok(ImageBrute { octets, type_mime })
    }

    /// `GET /assets/{id}/video/playback`, forwarding an optional `Range`
    /// header. The caller streams the body.
    pub async fn flux_video(&self, id: &str, plage: Option<&str>) -> Resultat<reqwest::Response> {
        valider_id(id)?;
        let mut requete = self.http.get(self.url(&format!("assets/{id}/video/playback")));
        if let Some(plage) = plage {
            requete = requete.header(reqwest::header::RANGE, plage);
        }
        let reponse = requete.send().await?;
        verifier(reponse).await
    }
}

/// Turns a non-success response into a typed error.
async fn verifier(reponse: reqwest::Response) -> Resultat<reqwest::Response> {
    let statut = reponse.status();
    if statut.is_success() {
        return Ok(reponse);
    }
    let message = reponse
        .json::<MessageErreur>()
        .await
        .map(|m| m.message_lisible())
        .unwrap_or_default();
    Err(match statut.as_u16() {
        401 => Erreur::CleRefusee,
        403 => Erreur::PermissionManquante(message),
        404 => Erreur::Introuvable,
        code => Erreur::Http { statut: code, message },
    })
}

fn normaliser_url(brute: &str) -> Resultat<Url> {
    let brute = brute.trim().trim_end_matches('/');
    let brute = brute.strip_suffix("/api").unwrap_or(brute);
    let mut url = Url::parse(brute)
        .map_err(|e| Erreur::Configuration(format!("adresse du serveur invalide : {e}")))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(Erreur::Configuration("l'adresse doit commencer par http:// ou https://".into()));
    }
    let chemin = format!("{}/api/", url.path().trim_end_matches('/'));
    url.set_path(&chemin);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

/// Asset ids are UUIDs; validating them prevents path injection.
pub fn valider_id(id: &str) -> Resultat<()> {
    let ok = id.len() == 36
        && id.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        });
    if ok { Ok(()) } else { Err(Erreur::Configuration(format!("identifiant invalide : {id}"))) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_les_adresses() {
        for brute in ["http://h:2283", "http://h:2283/", "http://h:2283/api", "http://h:2283/api/"] {
            assert_eq!(normaliser_url(brute).unwrap().as_str(), "http://h:2283/api/");
        }
        assert_eq!(
            normaliser_url("https://photos.exemple.fr/immich").unwrap().as_str(),
            "https://photos.exemple.fr/immich/api/"
        );
        assert!(normaliser_url("ftp://h").is_err());
    }

    #[test]
    fn valide_les_identifiants() {
        assert!(valider_id("0f3c5a4e-1b2c-4d5e-8f90-a1b2c3d4e5f6").is_ok());
        assert!(valider_id("../../server/config").is_err());
        assert!(valider_id("0f3c5a4e-1b2c-4d5e-8f90-a1b2c3d4e5fZ").is_err());
    }

    #[test]
    fn url_serveur_sans_api() {
        let c = ClientImmich::nouveau("http://h:2283/api", "cle").unwrap();
        assert_eq!(c.url_serveur(), "http://h:2283");
    }
}
