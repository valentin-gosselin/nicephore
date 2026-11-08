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

    /// One page of search results: smart search (`/search/smart`) for the
    /// image content, metadata search (`/search/metadata`) otherwise.
    pub async fn rechercher(&self, criteres: &CriteresRecherche, page: u32) -> Resultat<PageResultats> {
        const TAILLE_PAGE: u32 = 150;
        let mut corps = serde_json::json!({ "page": page.max(1), "size": TAILLE_PAGE, "withExif": true });
        let texte = criteres.texte.trim();
        let intelligente = criteres.mode == ModeRecherche::Contenu && !texte.is_empty();
        if intelligente {
            corps["query"] = texte.into();
        } else {
            corps["order"] = "desc".into();
            match criteres.mode {
                _ if texte.is_empty() => {}
                ModeRecherche::Contenu | ModeRecherche::FichierNom => corps["originalFileName"] = texte.into(),
                ModeRecherche::Description => corps["description"] = texte.into(),
                ModeRecherche::Texte => corps["ocr"] = texte.into(),
            }
        }
        if let Some(t) = &criteres.type_media {
            corps["type"] = match t.as_str() {
                "video" => "VIDEO",
                _ => "IMAGE",
            }
            .into();
        }
        if !criteres.personnes.is_empty() {
            criteres.personnes.iter().try_for_each(|id| valider_id(id))?;
            corps["personIds"] = serde_json::json!(criteres.personnes);
        }
        if let Some(ville) = &criteres.ville {
            corps["city"] = ville.as_str().into();
        }
        if let Some(pays) = &criteres.pays {
            corps["country"] = pays.as_str().into();
        }
        if let Some(du) = &criteres.du {
            corps["takenAfter"] = format!("{du}T00:00:00.000Z").into();
        }
        if let Some(au) = &criteres.au {
            corps["takenBefore"] = format!("{au}T23:59:59.999Z").into();
        }
        if criteres.favoris {
            corps["isFavorite"] = true.into();
        }
        corps["visibility"] = if criteres.archive { "archive" } else { "timeline" }.into();

        let chemin = if intelligente { "search/smart" } else { "search/metadata" };
        let reponse = self
            .http
            .post(self.url(chemin))
            .json(&corps)
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
        let valeur: serde_json::Value = verifier(reponse).await?.json().await?;
        let resultats = &valeur["assets"];
        Ok(PageResultats {
            assets: resultats["items"].as_array().into_iter().flatten().filter_map(AssetResume::depuis).collect(),
            page_suivante: resultats["nextPage"].as_str().and_then(|p| p.parse().ok()),
            total: resultats["total"].as_u64().unwrap_or(0),
        })
    }

    /// `GET /search/explore`: one photo per city.
    pub async fn lieux(&self) -> Resultat<Vec<Lieu>> {
        let valeur: serde_json::Value = self.get_json("search/explore", &[]).await?;
        let mut lieux = Vec::new();
        for groupe in valeur.as_array().into_iter().flatten() {
            if groupe["fieldName"].as_str() != Some("exifInfo.city") {
                continue;
            }
            for element in groupe["items"].as_array().into_iter().flatten() {
                if let (Some(ville), Some(asset)) = (element["value"].as_str(), AssetResume::depuis(&element["data"])) {
                    lieux.push(Lieu { ville: ville.to_string(), asset });
                }
            }
        }
        Ok(lieux)
    }

    /// `GET /people/{id}`.
    pub async fn personne(&self, id: &str) -> Resultat<Personne> {
        valider_id(id)?;
        self.get_json(&format!("people/{id}"), &[]).await
    }

    /// People whose faces look most like this person's, closest first
    /// (`GET /people?closestPersonId=`), to suggest merges.
    pub async fn personnes_proches(&self, id: &str) -> Resultat<Vec<Personne>> {
        valider_id(id)?;
        let valeur: serde_json::Value = self
            .get_json("people", &[("closestPersonId", id), ("size", "40"), ("withHidden", "false")])
            .await?;
        let mut proches: Vec<Personne> = serde_json::from_value(valeur["people"].clone())
            .map_err(|e| Erreur::ReponseInattendue(e.to_string()))?;
        proches.retain(|p| p.id != id);
        Ok(proches)
    }

    /// `POST /people`: a person without faces (tests, manual tagging).
    pub async fn creer_personne(&self, nom: &str) -> Resultat<Personne> {
        let reponse = self
            .http
            .post(self.url("people"))
            .json(&serde_json::json!({ "name": nom }))
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        Ok(verifier(reponse).await?.json().await?)
    }

    /// `PUT /people/{id}`: name, favorite, hidden.
    pub async fn modifier_personne(&self, id: &str, modification: &ModificationPersonne) -> Resultat<Personne> {
        valider_id(id)?;
        let reponse = self
            .http
            .put(self.url(&format!("people/{id}")))
            .json(modification)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        Ok(verifier(reponse).await?.json().await?)
    }

    /// `POST /people/{cible}/merge`: folds `autres` into `cible`, which keeps
    /// its name. Irreversible on the server.
    pub async fn fusionner_personnes(&self, cible: &str, autres: &[String]) -> Resultat<u64> {
        valider_id(cible)?;
        valider_ids(autres)?;
        if autres.iter().any(|id| id == cible) {
            return Err(Erreur::Configuration("une personne ne peut pas fusionner avec elle-même".into()));
        }
        let reponse = self
            .http
            .post(self.url(&format!("people/{cible}/merge")))
            .json(&serde_json::json!({ "ids": autres }))
            .timeout(Duration::from_secs(120))
            .send()
            .await?;
        let resultats: serde_json::Value = verifier(reponse).await?.json().await?;
        // The response also lists the target itself: count the merged ones only.
        Ok(resultats
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["success"].as_bool() == Some(true) && r["id"].as_str() != Some(cible))
            .count() as u64)
    }

    /// Albums owned by the user, then albums shared with them.
    pub async fn albums(&self) -> Resultat<Vec<Album>> {
        let mut albums: Vec<Album> = self.get_json("albums", &[("isOwned", "true")]).await?;
        for album in &mut albums {
            album.possede = true;
        }
        let partages: Vec<Album> = self.get_json("albums", &[("isOwned", "false")]).await?;
        albums.extend(partages);
        Ok(albums)
    }

    /// `GET /albums/{id}` (the owner flag is not filled here).
    pub async fn album(&self, id: &str) -> Resultat<Album> {
        valider_id(id)?;
        self.get_json(&format!("albums/{id}"), &[("withoutAssets", "true")]).await
    }

    /// `POST /albums`, optionally with initial assets.
    pub async fn creer_album(&self, nom: &str, description: &str, ids: &[String]) -> Resultat<Album> {
        if nom.trim().is_empty() {
            return Err(Erreur::Configuration("nom d'album vide".into()));
        }
        if !ids.is_empty() {
            valider_ids(ids)?;
        }
        let reponse = self
            .http
            .post(self.url("albums"))
            .json(&serde_json::json!({ "albumName": nom.trim(), "description": description, "assetIds": ids }))
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
        let mut album: Album = verifier(reponse).await?.json().await?;
        album.possede = true;
        Ok(album)
    }

    /// `PATCH /albums/{id}`: name, description.
    pub async fn modifier_album(&self, id: &str, modification: &ModificationAlbum) -> Resultat<()> {
        valider_id(id)?;
        let corps = serde_json::to_value(modification).map_err(|e| Erreur::Configuration(e.to_string()))?;
        self.envoyer(reqwest::Method::PATCH, &format!("albums/{id}"), &corps).await
    }

    /// `DELETE /albums/{id}`: deletes the album, never its photos.
    pub async fn supprimer_album(&self, id: &str) -> Resultat<()> {
        valider_id(id)?;
        let reponse = self.http.delete(self.url(&format!("albums/{id}"))).timeout(Duration::from_secs(30)).send().await?;
        verifier(reponse).await.map(|_| ())
    }

    /// `PUT /albums/{id}/assets`.
    pub async fn ajouter_a_l_album(&self, id: &str, ids: &[String]) -> Resultat<BilanAlbum> {
        valider_id(id)?;
        valider_ids(ids)?;
        let reponse = self
            .http
            .put(self.url(&format!("albums/{id}/assets")))
            .json(&serde_json::json!({ "ids": ids }))
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
        let resultats: serde_json::Value = verifier(reponse).await?.json().await?;
        let mut bilan = BilanAlbum::default();
        for r in resultats.as_array().into_iter().flatten() {
            match (r["success"].as_bool(), r["error"].as_str()) {
                (Some(true), _) => bilan.ajoutes += 1,
                (_, Some("duplicate")) => bilan.deja_presents += 1,
                _ => bilan.echecs += 1,
            }
        }
        Ok(bilan)
    }

    /// `DELETE /albums/{id}/assets`: removes from the album, keeps the photos.
    pub async fn retirer_de_l_album(&self, id: &str, ids: &[String]) -> Resultat<()> {
        valider_id(id)?;
        valider_ids(ids)?;
        let corps = serde_json::json!({ "ids": ids });
        let reponse = self
            .http
            .delete(self.url(&format!("albums/{id}/assets")))
            .json(&corps)
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
        verifier(reponse).await.map(|_| ())
    }

    /// `GET /map/markers`: every geotagged asset of the timeline.
    pub async fn marqueurs(&self) -> Resultat<Vec<Marqueur>> {
        self.get_json("map/markers", &[("withPartners", "true")]).await
    }

    /// Map style URLs from `GET /server/config` (Immich's tile server by default).
    pub async fn styles_carte(&self) -> Resultat<StylesCarte> {
        self.get_json("server/config", &[]).await
    }

    /// `GET /memories?for=YYYY-MM-DD`: the memories shown on that day.
    pub async fn souvenirs(&self, jour: &str) -> Resultat<Vec<Souvenir>> {
        let valide = jour.len() == 10 && jour.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
        if !valide {
            return Err(Erreur::Configuration(format!("date invalide : {jour}")));
        }
        let valeur: serde_json::Value = self.get_json("memories", &[("for", jour)]).await?;
        Ok(valeur.as_array().into_iter().flatten().filter_map(Souvenir::depuis).collect())
    }

    /// `GET /memories/{id}`.
    pub async fn souvenir(&self, id: &str) -> Resultat<Souvenir> {
        valider_id(id)?;
        let valeur: serde_json::Value = self.get_json(&format!("memories/{id}"), &[]).await?;
        Souvenir::depuis(&valeur).ok_or_else(|| Erreur::ReponseInattendue("souvenir illisible".into()))
    }

    /// Grid descriptions of a set of assets (map selection), 8 requests at a
    /// time, in the given order; unreadable ones are left out.
    pub async fn assets_par_ids(&self, ids: &[String]) -> Resultat<Vec<AssetResume>> {
        use futures_util::StreamExt;
        valider_ids(ids)?;
        // Owned values: the futures must not borrow across the stream.
        let taches: Vec<_> = ids
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, id)| {
                let client = self.clone();
                async move { client.infos_asset(&id).await.ok().and_then(|v| AssetResume::depuis(&v)).map(|a| (i, a)) }
            })
            .collect();
        let mut lus: Vec<(usize, AssetResume)> = futures_util::stream::iter(taches)
            .buffer_unordered(8)
            .filter_map(|r| async move { r })
            .collect()
            .await;
        lus.sort_by_key(|(i, _)| *i);
        Ok(lus.into_iter().map(|(_, a)| a).collect())
    }

    /// `GET /people/{id}/statistics`: number of assets where the person appears.
    pub async fn nombre_photos_personne(&self, id: &str) -> Resultat<u64> {
        valider_id(id)?;
        let valeur: serde_json::Value = self.get_json(&format!("people/{id}/statistics"), &[]).await?;
        Ok(valeur["assets"].as_u64().unwrap_or(0))
    }

    /// `POST /search/statistics` filtered on a city: number of assets taken there.
    pub async fn nombre_photos_ville(&self, ville: &str) -> Resultat<u64> {
        let reponse = self
            .http
            .post(self.url("search/statistics"))
            .json(&serde_json::json!({ "city": ville, "visibility": "timeline" }))
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        let valeur: serde_json::Value = verifier(reponse).await?.json().await?;
        Ok(valeur["total"].as_u64().unwrap_or(0))
    }

    /// `GET /people`: every person, all pages (hidden ones on request).
    pub async fn personnes(&self, avec_masquees: bool) -> Resultat<Vec<Personne>> {
        let masquees = if avec_masquees { "true" } else { "false" };
        let mut personnes = Vec::new();
        for page in 1..=50u32 {
            let page_texte = page.to_string();
            let valeur: serde_json::Value = self
                .get_json("people", &[("withHidden", masquees), ("size", "500"), ("page", &page_texte)])
                .await?;
            let lot: Vec<Personne> = serde_json::from_value(valeur["people"].clone())
                .map_err(|e| Erreur::ReponseInattendue(e.to_string()))?;
            let vide = lot.is_empty();
            personnes.extend(lot);
            if vide || !valeur["hasNextPage"].as_bool().unwrap_or(false) {
                break;
            }
        }
        Ok(personnes)
    }

    /// `PUT /assets`: same change applied to many assets (favorite, archive...).
    pub async fn modifier_assets(&self, ids: &[String], modification: &ModificationAssets) -> Resultat<()> {
        valider_ids(ids)?;
        let mut corps = serde_json::to_value(modification).map_err(|e| Erreur::Configuration(e.to_string()))?;
        corps["ids"] = serde_json::json!(ids);
        self.envoyer(reqwest::Method::PUT, "assets", &corps).await
    }

    /// `GET /tags`: every tag of the user.
    pub async fn tags(&self) -> Resultat<Vec<Tag>> {
        self.get_json("tags", &[]).await
    }

    /// `PUT /tags` (upsert): the tag of that name, created if missing.
    pub async fn tag_nomme(&self, nom: &str) -> Resultat<Tag> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err(Erreur::Configuration("nom de tag vide".into()));
        }
        let reponse = self
            .http
            .put(self.url("tags"))
            .json(&serde_json::json!({ "tags": [nom] }))
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        let tags: Vec<Tag> = verifier(reponse).await?.json().await?;
        tags.into_iter().next().ok_or_else(|| Erreur::ReponseInattendue("tag non créé".into()))
    }

    /// `PUT /tags/{id}/assets` or `DELETE` for `retirer`.
    pub async fn etiqueter(&self, tag: &str, ids: &[String], retirer: bool) -> Resultat<()> {
        valider_id(tag)?;
        valider_ids(ids)?;
        let methode = if retirer { reqwest::Method::DELETE } else { reqwest::Method::PUT };
        self.envoyer(methode, &format!("tags/{tag}/assets"), &serde_json::json!({ "ids": ids })).await
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

    /// `GET /server/media-types`: extensions the server accepts.
    pub async fn types_medias(&self) -> Resultat<TypesMedias> {
        self.get_json("server/media-types", &[]).await
    }

    /// `POST /assets/bulk-upload-check`: which files the server already
    /// holds (by SHA-1), at most 1 000 at once.
    pub async fn verifier_envois(&self, elements: &[ElementVerification]) -> Resultat<Vec<ResultatVerification>> {
        if elements.len() > 1_000 {
            return Err(Erreur::Configuration("au plus 1 000 fichiers par vérification".into()));
        }
        #[derive(serde::Deserialize)]
        struct Reponse {
            results: Vec<ResultatVerification>,
        }
        let reponse = self
            .http
            .post(self.url("assets/bulk-upload-check"))
            .json(&serde_json::json!({ "assets": elements }))
            .timeout(Duration::from_secs(60))
            .send()
            .await?;
        Ok(verifier(reponse).await?.json::<Reponse>().await?.results)
    }

    /// `POST /assets`: sends one file, streamed from `corps`. No overall
    /// time limit for big videos on slow links, only a generous one
    /// proportional to the size (at least 64 kB/s).
    pub async fn envoyer_asset(&self, meta: &MetaEnvoi, corps: reqwest::Body) -> Resultat<ReponseEnvoi> {
        let partie = reqwest::multipart::Part::stream_with_length(corps, meta.taille)
            .file_name(meta.nom_fichier.clone())
            .mime_str("application/octet-stream")
            .expect("type MIME valide");
        let formulaire = reqwest::multipart::Form::new()
            .text("fileCreatedAt", meta.cree_le.clone())
            .text("fileModifiedAt", meta.modifie_le.clone())
            .text("filename", meta.nom_fichier.clone())
            .part("assetData", partie);
        let delai = Duration::from_secs(120 + meta.taille / 65_536);
        let reponse = self
            .http
            .post(self.url("assets"))
            .header("x-immich-checksum", &meta.sha1)
            .multipart(formulaire)
            .timeout(delai)
            .send()
            .await?;
        Ok(verifier(reponse).await?.json().await?)
    }

    /// `GET /assets/{id}`: full metadata (EXIF, people, tags...).
    pub async fn infos_asset(&self, id: &str) -> Resultat<serde_json::Value> {
        valider_id(id)?;
        self.get_json(&format!("assets/{id}"), &[]).await
    }

    /// `GET /assets/{id}/thumbnail?size=...`: rendered image (JPEG or WebP),
    /// or `GET /people/{id}/thumbnail` for a face.
    pub async fn image(&self, id: &str, taille: TailleImage) -> Resultat<ImageBrute> {
        valider_id(id)?;
        let requete = match taille {
            TailleImage::Personne => self.http.get(self.url(&format!("people/{id}/thumbnail"))),
            TailleImage::Profil => self.http.get(self.url(&format!("users/{id}/profile-image"))),
            _ => self.http.get(self.url(&format!("assets/{id}/thumbnail"))).query(&[("size", taille.parametre())]),
        };
        let reponse = requete.timeout(Duration::from_secs(60)).send().await?;
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
        // Immich answers 400 "Not found or no ... access" or "Album not
        // found" for missing items.
        400 if message.to_ascii_lowercase().contains("not found") => Erreur::Introuvable,
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
        let m = ModificationAssets { favori: Some(true), ..Default::default() };
        assert_eq!(serde_json::to_value(&m).unwrap(), serde_json::json!({ "isFavorite": true }));
        assert!(valider_ids(&[]).is_err());
        // From the UI: French names in, Immich names out; a null rating is
        // kept (it removes the stars), an absent one is left out.
        let m: ModificationAssets =
            serde_json::from_value(serde_json::json!({ "note": null, "date": "2024-05-01T12:00:00.000+02:00", "fuseau": "Europe/Paris" }))
                .unwrap();
        assert_eq!(
            serde_json::to_value(&m).unwrap(),
            serde_json::json!({ "rating": null, "dateTimeOriginal": "2024-05-01T12:00:00.000+02:00", "timeZone": "Europe/Paris" })
        );
        let sans_note: ModificationAssets = serde_json::from_value(serde_json::json!({ "description": "x" })).unwrap();
        assert_eq!(serde_json::to_value(&sans_note).unwrap(), serde_json::json!({ "description": "x" }));
    }

    #[test]
    fn modification_d_album_lue_depuis_l_interface() {
        // The UI sends `nom`; Immich expects `albumName` (bug caught in situ).
        let m: ModificationAlbum = serde_json::from_value(serde_json::json!({ "nom": "Vacances" })).unwrap();
        assert_eq!(m.nom.as_deref(), Some("Vacances"));
        assert_eq!(serde_json::to_value(&m).unwrap(), serde_json::json!({ "albumName": "Vacances" }));
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

    fn client() -> ClientImmich {
        let cle = std::env::var("IMMICH_API_KEY").or_else(|_| std::env::var("IMMICH_KEY")).expect("clé");
        ClientImmich::nouveau(&url(), &Authentification::CleApi(cle)).unwrap()
    }

    #[tokio::test]
    #[ignore = "needs a reachable Immich server and an API key"]
    async fn recherche_explorer_et_personnes() {
        let c = client();
        let contenu = CriteresRecherche { texte: "concert".into(), ..Default::default() };
        let page = c.rechercher(&contenu, 1).await.unwrap();
        println!("contenu « concert » : {} résultats, page suivante {:?}", page.assets.len(), page.page_suivante);
        assert!(!page.assets.is_empty());
        let videos = CriteresRecherche { type_media: Some("video".into()), ..Default::default() };
        let page = c.rechercher(&videos, 1).await.unwrap();
        println!("vidéos : {} sur la première page", page.assets.len());
        assert!(page.assets.iter().all(|a| !a.is_image));
        let lieux = c.lieux().await.unwrap();
        println!("lieux : {}", lieux.len());
        let personnes = c.personnes(false).await.unwrap();
        println!("personnes : {}, dont nommées : {}", personnes.len(), personnes.iter().filter(|p| !p.name.is_empty()).count());
        if let Some(p) = personnes.first() {
            let filtre = FiltreChronologie { personne: Some(p.id.clone()), ..Default::default() };
            let mois = c.liste_mois(&filtre).await.unwrap();
            let total: u64 = mois.iter().map(|m| u64::from(m.count)).sum();
            let stats = c.nombre_photos_personne(&p.id).await.unwrap();
            println!("premier visage : {} mois, {total} éléments, statistiques {stats}", mois.len());
        }
        if let Some(l) = lieux.first() {
            println!("ville {} : {} photos", l.ville, c.nombre_photos_ville(&l.ville).await.unwrap());
        }
        if let Some(p) = personnes.iter().find(|p| !p.name.is_empty()) {
            let proches = c.personnes_proches(&p.id).await.unwrap();
            println!("proches de {} : {} (dont nommées {})", p.name, proches.len(), proches.iter().filter(|x| !x.name.is_empty()).count());
        }
        let marqueurs = c.marqueurs().await.unwrap();
        println!("carte : {} marqueurs", marqueurs.len());
        let styles = c.styles_carte().await.unwrap();
        assert!(styles.map_light_style_url.starts_with("http"));
        let ids: Vec<String> = marqueurs.iter().take(5).map(|m| m.id.clone()).collect();
        if !ids.is_empty() {
            let lus = c.assets_par_ids(&ids).await.unwrap();
            println!("assets par ids : {} sur {}", lus.len(), ids.len());
            assert_eq!(lus.first().map(|a| a.id.clone()), ids.first().cloned());
        }
        let jour = "2026-11-08";
        let souvenirs = c.souvenirs(jour).await.unwrap();
        println!("souvenirs du {jour} : {}", souvenirs.iter().map(|s| format!("{} ({} éléments)", s.annee, s.assets.len())).collect::<Vec<_>>().join(", "));
        if let Some(s) = souvenirs.first() {
            assert_eq!(c.souvenir(&s.id).await.unwrap().assets.len(), s.assets.len());
        }
        let albums = c.albums().await.unwrap();
        println!("albums : {} dont {} possédés", albums.len(), albums.iter().filter(|a| a.possede).count());
        if let Some(a) = albums.first() {
            let filtre = FiltreChronologie { album: Some(a.id.clone()), ..Default::default() };
            let total: u64 = c.liste_mois(&filtre).await.unwrap().iter().map(|m| u64::from(m.count)).sum();
            println!("album {} : {} annoncés, {} dans la chronologie", a.album_name, a.asset_count, total);
        }
    }

    #[tokio::test]
    #[ignore = "needs a reachable Immich server"]
    async fn refuse_un_mauvais_mot_de_passe() {
        let erreur = ouvrir_session(&url(), "compte-inexistant@nicephore.invalid", "pas-le-bon").await.unwrap_err();
        assert!(matches!(erreur, Erreur::IdentifiantsRefuses), "{erreur:?}");
    }
}
