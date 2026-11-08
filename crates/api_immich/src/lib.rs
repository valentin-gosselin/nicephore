//! Minimal typed client for the Immich server API.
//!
//! Targets Immich 3.x. Two ways to authenticate: a session opened with the
//! user's email and password (like the web and mobile apps, sent as a bearer
//! token), or an API key (`x-api-key`). Only the endpoints needed by the
//! desktop app are implemented; the shapes follow the official OpenAPI
//! specification (`@immich/sdk`).

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

/// How requests are authenticated.
#[derive(Clone)]
pub enum Authentification {
    /// Session token returned by `POST /auth/login`.
    Session(String),
    /// API key created in the Immich account settings.
    CleApi(String),
}

fn client_http(entetes: HeaderMap) -> Resultat<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .default_headers(entetes)
        .user_agent(concat!("Nicephore/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(5))
        .pool_max_idle_per_host(32)
        // Never forward credentials to another host on redirects.
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}

/// Login methods offered by a server, read before any authentication.
pub async fn modes_connexion(url_serveur: &str) -> Resultat<ModesConnexion> {
    let base = normaliser_url(url_serveur)?;
    let http = client_http(HeaderMap::new())?;
    let reponse = http
        .get(base.join("server/features").expect("chemin valide"))
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let mut modes: ModesConnexion = verifier(reponse).await?.json().await?;
    modes.url_serveur = sans_api(&base);
    Ok(modes)
}

/// Opens a session with the user's email and password. The password is
/// only sent to the server, never kept.
pub async fn ouvrir_session(url_serveur: &str, email: &str, mot_de_passe: &str) -> Resultat<SessionOuverte> {
    let base = normaliser_url(url_serveur)?;
    let http = client_http(HeaderMap::new())?;
    let reponse = http
        .post(base.join("auth/login").expect("chemin valide"))
        .json(&serde_json::json!({ "email": email.trim(), "password": mot_de_passe }))
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    match verifier(reponse).await {
        Ok(reponse) => Ok(reponse.json().await?),
        Err(Erreur::CleRefusee) | Err(Erreur::Http { statut: 400, .. }) => Err(Erreur::IdentifiantsRefuses),
        Err(e) => Err(e),
    }
}

/// Server address without the `/api/` suffix.
fn sans_api(base: &Url) -> String {
    let mut url = base.clone();
    url.set_path(base.path().trim_end_matches("api/"));
    url.as_str().trim_end_matches('/').to_string()
}

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
    pub fn nouveau(url_serveur: &str, authentification: &Authentification) -> Resultat<Self> {
        let base = normaliser_url(url_serveur)?;
        let (nom, valeur) = match authentification {
            Authentification::CleApi(cle) => (ENTETE_CLE, cle.trim().to_string()),
            Authentification::Session(jeton) => ("authorization", format!("Bearer {}", jeton.trim())),
        };
        if valeur.is_empty() || valeur == "Bearer " {
            return Err(Erreur::Configuration("identifiant de connexion vide".into()));
        }
        let mut valeur = HeaderValue::from_str(&valeur)
            .map_err(|_| Erreur::Configuration("identifiant de connexion invalide".into()))?;
        valeur.set_sensitive(true);
        let mut entetes = HeaderMap::new();
        entetes.insert(nom, valeur);
        Ok(Self { http: client_http(entetes)?, base })
    }

    /// `POST /auth/logout`: revokes the session on the server.
    pub async fn fermer_session(&self) -> Resultat<()> {
        self.envoyer(reqwest::Method::POST, "auth/logout", &serde_json::json!({})).await
    }

    /// Server base URL (without the `/api/` suffix), for display.
    pub fn url_serveur(&self) -> String {
        sans_api(&self.base)
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
    /// newest first, for the given view (main timeline, favorites...).
    pub async fn liste_mois(&self, filtre: &FiltreChronologie) -> Resultat<Vec<Mois>> {
        let parametres = filtre.parametres()?;
        self.get_json("timeline/buckets", &parametres.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>())
            .await
    }

    /// `GET /timeline/bucket`: columnar description of every asset of a month.
    pub async fn detail_mois(&self, mois: &str, filtre: &FiltreChronologie) -> Resultat<DetailMois> {
        let mut parametres = filtre.parametres()?;
        parametres.push(("timeBucket", mois.to_string()));
        self.get_json("timeline/bucket", &parametres.iter().map(|(k, v)| (*k, v.as_str())).collect::<Vec<_>>())
            .await
    }

    /// `PUT /assets`: same change applied to many assets (favorite, archive...).
    pub async fn modifier_assets(&self, ids: &[String], modification: &ModificationAssets) -> Resultat<()> {
        valider_ids(ids)?;
        let mut corps = serde_json::to_value(modification).map_err(|e| Erreur::Configuration(e.to_string()))?;
        corps["ids"] = serde_json::json!(ids);
        self.envoyer(reqwest::Method::PUT, "assets", &corps).await
    }

    /// `DELETE /assets` without `force`: moves assets to the Immich trash
    /// (recoverable). Permanent deletion is deliberately not exposed.
    pub async fn mettre_a_la_corbeille(&self, ids: &[String]) -> Resultat<()> {
        valider_ids(ids)?;
        let corps = serde_json::json!({ "ids": ids, "force": false });
        self.envoyer(reqwest::Method::DELETE, "assets", &corps).await
    }

    /// `POST /trash/restore/assets`: takes assets out of the trash.
    pub async fn restaurer(&self, ids: &[String]) -> Resultat<()> {
        valider_ids(ids)?;
        let corps = serde_json::json!({ "ids": ids });
        self.envoyer(reqwest::Method::POST, "trash/restore/assets", &corps).await
    }

    /// `GET /assets/{id}/original`: the original file, streamed by the caller.
    pub async fn flux_original(&self, id: &str) -> Resultat<reqwest::Response> {
        valider_id(id)?;
        let reponse = self.http.get(self.url(&format!("assets/{id}/original"))).send().await?;
        verifier(reponse).await
    }

    async fn envoyer(&self, methode: reqwest::Method, chemin: &str, corps: &serde_json::Value) -> Resultat<()> {
        let reponse = self
            .http
            .request(methode, self.url(chemin))
            .json(corps)
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
        verifier(reponse).await.map(|_| ())
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

/// Validates every id of a bulk request (at most 5 000 at once).
fn valider_ids(ids: &[String]) -> Resultat<()> {
    if ids.is_empty() || ids.len() > 5_000 {
        return Err(Erreur::Configuration(format!("{} éléments : il en faut entre 1 et 5 000", ids.len())));
    }
    ids.iter().try_for_each(|id| valider_id(id))
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
    fn filtres_de_vue() {
        let principal = FiltreChronologie::default().parametres().unwrap();
        assert!(principal.contains(&("withPartners", "true".into())));
        let favoris = FiltreChronologie { favori: true, ..Default::default() }.parametres().unwrap();
        assert!(favoris.contains(&("isFavorite", "true".into())));
        assert!(!favoris.iter().any(|(k, _)| *k == "withPartners"));
        let corbeille = FiltreChronologie { corbeille: true, ..Default::default() }.parametres().unwrap();
        assert!(corbeille.contains(&("isTrashed", "true".into())));
        assert!(!corbeille.iter().any(|(k, _)| *k == "visibility"));
        let inconnue = FiltreChronologie { visibilite: Some("locked".into()), ..Default::default() };
        assert!(inconnue.parametres().is_err());
    }

    #[test]
    fn modification_serialisee_au_format_immich() {
        let m = ModificationAssets { favori: Some(true), visibilite: None };
        assert_eq!(serde_json::to_value(&m).unwrap(), serde_json::json!({ "isFavorite": true }));
        assert!(valider_ids(&[]).is_err());
    }

    #[test]
    fn url_serveur_sans_api() {
        let c = ClientImmich::nouveau("http://h:2283/api", &Authentification::CleApi("cle".into())).unwrap();
        assert!(ClientImmich::nouveau("http://h", &Authentification::Session(" ".into())).is_err());
        assert_eq!(c.url_serveur(), "http://h:2283");
    }
}

/// Against a real server: `IMMICH_URL=... cargo test -p api_immich -- --ignored`.
#[cfg(test)]
mod tests_serveur {
    use super::*;

    fn url() -> String {
        std::env::var("IMMICH_URL").expect("IMMICH_URL")
    }

    #[tokio::test]
    #[ignore = "needs a reachable Immich server"]
    async fn lit_les_modes_de_connexion() {
        let modes = modes_connexion(&url()).await.unwrap();
        assert!(modes.password_login || modes.oauth);
        assert!(!modes.url_serveur.ends_with('/'));
    }

    #[tokio::test]
    #[ignore = "needs a reachable Immich server"]
    async fn refuse_un_mauvais_mot_de_passe() {
        let erreur = ouvrir_session(&url(), "compte-inexistant@nicephore.invalid", "pas-le-bon").await.unwrap_err();
        assert!(matches!(erreur, Erreur::IdentifiantsRefuses), "{erreur:?}");
    }
}
