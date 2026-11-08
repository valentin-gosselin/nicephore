use bytes::Bytes;
use serde::{Deserialize, Deserializer, Serialize};

/// Tells "absent" (`None`) from "present and null" (`Some(None)`).
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<i32>>, D::Error> {
    Ok(Some(Option::<i32>::deserialize(d)?))
}

#[derive(Debug, Deserialize)]
pub(crate) struct Ping {
    pub res: String,
}

/// Error body returned by Immich (`message` may be a string or a list).
#[derive(Debug, Deserialize)]
pub(crate) struct MessageErreur {
    #[serde(default)]
    message: serde_json::Value,
}

impl MessageErreur {
    pub fn message_lisible(&self) -> String {
        match &self.message {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Array(liste) => liste
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(" ; "),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct VersionServeur {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for VersionServeur {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Utilisateur {
    pub id: String,
    pub name: String,
    pub email: String,
}

/// One month of the timeline (`timeBucket` is `YYYY-MM-DD`, first day).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mois {
    pub time_bucket: String,
    pub count: u32,
}

/// Columnar description of the assets of one month: every vector has the
/// same length, index `i` describes asset `i`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DetailMois {
    pub id: Vec<String>,
    pub ratio: Vec<f64>,
    pub is_image: Vec<bool>,
    #[serde(default)]
    pub is_favorite: Vec<bool>,
    #[serde(default)]
    pub thumbhash: Vec<Option<String>>,
    #[serde(default)]
    pub duration: Vec<Option<serde_json::Value>>,
    pub file_created_at: Vec<String>,
    #[serde(default)]
    pub local_offset_hours: Vec<f64>,
    #[serde(default)]
    pub live_photo_video_id: Vec<Option<String>>,
    #[serde(default)]
    pub stack: Option<Vec<Option<Vec<String>>>>,
}

/// Rendered sizes served by `/assets/{id}/thumbnail`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TailleImage {
    /// About 250 px, for grids.
    Vignette,
    /// About 1440 px, for the viewer.
    Apercu,
    /// Full resolution rendering (when enabled on the server).
    Pleine,
    /// Face thumbnail of a person (`/people/{id}/thumbnail`).
    Personne,
}

impl TailleImage {
    pub fn parametre(self) -> &'static str {
        match self {
            TailleImage::Vignette => "thumbnail",
            TailleImage::Apercu => "preview",
            TailleImage::Pleine => "fullsize",
            TailleImage::Personne => "thumbnail",
        }
    }

    pub fn depuis_nom(nom: &str) -> Option<Self> {
        match nom {
            "vignette" | "thumbnail" => Some(TailleImage::Vignette),
            "apercu" | "preview" => Some(TailleImage::Apercu),
            "pleine" | "fullsize" => Some(TailleImage::Pleine),
            "personne" => Some(TailleImage::Personne),
            _ => None,
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            TailleImage::Vignette => "vignette",
            TailleImage::Apercu => "apercu",
            TailleImage::Pleine => "pleine",
            TailleImage::Personne => "personne",
        }
    }
}

/// Image bytes as served by the server.
#[derive(Debug, Clone)]
pub struct ImageBrute {
    pub octets: Bytes,
    pub type_mime: Option<String>,
}

/// Which assets a timeline shows: the main timeline, favorites, archive or
/// trash. Mirrors the query parameters of `/timeline/buckets`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FiltreChronologie {
    /// `timeline` (default) or `archive`; ignored for the trash.
    #[serde(default)]
    pub visibilite: Option<String>,
    #[serde(default)]
    pub favori: bool,
    #[serde(default)]
    pub corbeille: bool,
    /// Only the assets where this person appears.
    #[serde(default)]
    pub personne: Option<String>,
    /// Only the assets of this album.
    #[serde(default)]
    pub album: Option<String>,
}

impl FiltreChronologie {
    pub(crate) fn parametres(&self) -> crate::Resultat<Vec<(&'static str, String)>> {
        let mut parametres = vec![("withStacked", "true".to_string())];
        if self.corbeille {
            parametres.push(("isTrashed", "true".into()));
            return Ok(parametres);
        }
        let visibilite = self.visibilite.as_deref().unwrap_or("timeline");
        if !matches!(visibilite, "timeline" | "archive") {
            return Err(crate::Erreur::Configuration(format!("visibilité inconnue : {visibilite}")));
        }
        parametres.push(("visibility", visibilite.into()));
        if self.favori {
            parametres.push(("isFavorite", "true".into()));
        }
        if let Some(personne) = &self.personne {
            crate::valider_id(personne)?;
            parametres.push(("personId", personne.clone()));
        }
        if let Some(album) = &self.album {
            crate::valider_id(album)?;
            parametres.push(("albumId", album.clone()));
        }
        // Partner assets are only allowed on the plain main timeline.
        if visibilite == "timeline" && !self.favori && self.personne.is_none() && self.album.is_none() {
            parametres.push(("withPartners", "true".into()));
        }
        Ok(parametres)
    }
}

/// Fields changed by a bulk update (`PUT /assets`); unset fields are untouched.
/// Field names are French on the UI side and Immich's on the server side.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModificationAssets {
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "isFavorite"))]
    pub favori: Option<bool>,
    /// `timeline` or `archive`.
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "visibility"))]
    pub visibilite: Option<String>,
    /// Capture date with its offset, e.g. `2024-05-01T12:00:00.000+02:00`.
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "dateTimeOriginal"))]
    pub date: Option<String>,
    /// IANA time zone of the capture (`Europe/Paris`).
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "timeZone"))]
    pub fuseau: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// 1 to 5 stars, -1 rejected; `Some(None)` removes the rating.
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "rating"), default, deserialize_with = "present")]
    pub note: Option<Option<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A tag (`/tags`); `value` is the full path of nested tags (`Voyages/Italie`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub color: Option<String>,
}

/// Login methods of a server (`GET /server/features`, public).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModesConnexion {
    #[serde(default)]
    pub password_login: bool,
    #[serde(default)]
    pub oauth: bool,
    /// Normalised server address, filled by the client.
    #[serde(default)]
    pub url_serveur: String,
}

/// Session opened by `POST /auth/login`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOuverte {
    pub access_token: String,
    pub name: String,
    pub user_email: String,
}

/// Asset as returned by searches, reduced to what the grid needs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetResume {
    pub id: String,
    pub ratio: f64,
    pub is_image: bool,
    pub is_favorite: bool,
    pub thumbhash: Option<String>,
    /// Milliseconds, for videos.
    pub duration: Option<f64>,
    pub file_created_at: String,
    /// Capture date in the photographer's time zone, without offset.
    pub local_date_time: String,
}

impl AssetResume {
    pub(crate) fn depuis(valeur: &serde_json::Value) -> Option<Self> {
        let largeur = valeur["width"].as_f64().or_else(|| valeur["exifInfo"]["exifImageWidth"].as_f64());
        let hauteur = valeur["height"].as_f64().or_else(|| valeur["exifInfo"]["exifImageHeight"].as_f64());
        let ratio = match (largeur, hauteur) {
            (Some(l), Some(h)) if l > 0.0 && h > 0.0 => l / h,
            _ => 1.0,
        };
        Some(Self {
            id: valeur["id"].as_str()?.to_string(),
            ratio,
            is_image: valeur["type"].as_str() != Some("VIDEO"),
            is_favorite: valeur["isFavorite"].as_bool().unwrap_or(false),
            thumbhash: valeur["thumbhash"].as_str().map(str::to_string),
            duration: valeur["duration"].as_f64(),
            file_created_at: valeur["fileCreatedAt"].as_str().unwrap_or_default().to_string(),
            local_date_time: valeur["localDateTime"].as_str().unwrap_or_default().to_string(),
        })
    }
}

/// How the search text is matched.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ModeRecherche {
    /// Smart search on the image content (CLIP), e.g. "beach at sunset".
    #[default]
    Contenu,
    FichierNom,
    Description,
    /// Text read in the images (OCR).
    Texte,
}

/// Search text and filters, all optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CriteresRecherche {
    #[serde(default)]
    pub texte: String,
    #[serde(default)]
    pub mode: ModeRecherche,
    /// `image` or `video`.
    #[serde(default)]
    pub type_media: Option<String>,
    #[serde(default)]
    pub personnes: Vec<String>,
    #[serde(default)]
    pub ville: Option<String>,
    #[serde(default)]
    pub pays: Option<String>,
    /// ISO dates (YYYY-MM-DD), inclusive.
    #[serde(default)]
    pub du: Option<String>,
    #[serde(default)]
    pub au: Option<String>,
    #[serde(default)]
    pub favoris: bool,
    #[serde(default)]
    pub archive: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResultats {
    pub assets: Vec<AssetResume>,
    pub page_suivante: Option<u32>,
    pub total: u64,
}

/// A place of the Explore page: a city and one of its photos.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lieu {
    pub ville: String,
    pub asset: AssetResume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Personne {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub is_hidden: bool,
    #[serde(default)]
    pub birth_date: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Fields changed on a person (`PUT /people/{id}`); unset fields are untouched.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModificationPersonne {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_favorite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_hidden: Option<bool>,
}

/// An album as listed by `/albums`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub album_name: String,
    #[serde(default)]
    pub description: String,
    pub asset_count: u64,
    #[serde(default)]
    pub album_thumbnail_asset_id: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub shared: bool,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub last_modified_asset_timestamp: Option<String>,
    /// Owned by the current user (false: shared with them). Filled by the client.
    #[serde(default)]
    pub possede: bool,
}

/// Fields changed on an album (`PATCH /albums/{id}`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModificationAlbum {
    /// Sent to Immich as `albumName`, received from the UI as `nom`.
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "albumName"))]
    pub nom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Outcome of adding assets to an album.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BilanAlbum {
    pub ajoutes: u64,
    pub deja_presents: u64,
    pub echecs: u64,
}

/// A geotagged asset on the map (`/map/markers`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marqueur {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    #[serde(default)]
    pub city: Option<String>,
}

/// Map styles configured on the server (MapLibre style URLs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StylesCarte {
    pub map_light_style_url: String,
    pub map_dark_style_url: String,
}

/// A memory ("on this day" or birthday) with its assets.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Souvenir {
    pub id: String,
    /// `on_this_day` or `birthday`.
    pub genre: String,
    pub annee: i32,
    pub personne: Option<String>,
    pub date: String,
    pub assets: Vec<AssetResume>,
}

impl Souvenir {
    pub(crate) fn depuis(valeur: &serde_json::Value) -> Option<Self> {
        Some(Self {
            id: valeur["id"].as_str()?.to_string(),
            genre: valeur["type"].as_str().unwrap_or("on_this_day").to_string(),
            annee: valeur["data"]["year"].as_i64().unwrap_or(0) as i32,
            personne: valeur["data"]["personName"].as_str().map(str::to_string),
            date: valeur["memoryAt"].as_str().unwrap_or_default().to_string(),
            assets: valeur["assets"].as_array().into_iter().flatten().filter_map(AssetResume::depuis).collect(),
        })
    }
}

/// File extensions the server accepts, by kind (`GET /server/media-types`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypesMedias {
    #[serde(default)]
    pub image: Vec<String>,
    #[serde(default)]
    pub video: Vec<String>,
    #[serde(default)]
    pub sidecar: Vec<String>,
}

impl TypesMedias {
    /// True for an image or video extension (case insensitive, with or
    /// without the leading dot).
    pub fn accepte(&self, extension: &str) -> bool {
        let extension = format!(".{}", extension.trim_start_matches('.').to_ascii_lowercase());
        self.image.iter().chain(&self.video).any(|e| e.eq_ignore_ascii_case(&extension))
    }
}

/// One file to look up before sending it. `id` is chosen by the caller and
/// echoed back; `checksum` is the SHA-1 of the content (hex).
#[derive(Debug, Clone, Serialize)]
pub struct ElementVerification {
    pub id: String,
    pub checksum: String,
}

/// Answer of `POST /assets/bulk-upload-check` for one file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultatVerification {
    pub id: String,
    /// `accept` (unknown to the server) or `reject`.
    pub action: String,
    /// `duplicate` or `unsupported-format` when rejected.
    #[serde(default)]
    pub reason: Option<String>,
    /// Existing asset when it is a duplicate.
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub is_trashed: bool,
}

/// Metadata sent with a file.
#[derive(Debug, Clone)]
pub struct MetaEnvoi {
    pub nom_fichier: String,
    pub taille: u64,
    /// SHA-1 (hex): the server rejects the upload as a duplicate early.
    pub sha1: String,
    /// RFC 3339 dates.
    pub cree_le: String,
    pub modifie_le: String,
}

/// Answer of `POST /assets`.
#[derive(Debug, Clone, Deserialize)]
pub struct ReponseEnvoi {
    pub id: String,
    /// `created`, `replaced` or `duplicate`.
    pub status: String,
}
