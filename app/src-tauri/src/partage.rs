//! Sharing, like Immich web: public links to an album or to some photos
//! (with their address ready to copy), users an album is shared with, and
//! the download of a whole album.

use std::path::PathBuf;

use api_immich::{ClientImmich, LienPartage, MembresAlbum, Selection, Utilisateur};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::Etat;

/// A shared link and the address to give out.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LienVue {
    #[serde(flatten)]
    lien: LienPartage,
    url: String,
}

/// Base of the addresses: the external domain set in Immich, otherwise the
/// address Nicéphore uses.
async fn base(client: &ClientImmich) -> String {
    let domaine = client.domaine_externe().await.unwrap_or_default();
    if domaine.is_empty() { client.url_serveur().trim_end_matches('/').to_string() } else { domaine }
}

/// `/s/<slug>` for a custom address, `/share/<key>` otherwise.
fn vue(base: &str, lien: LienPartage) -> LienVue {
    let url = match lien.slug.as_deref().filter(|s| !s.is_empty()) {
        Some(slug) => format!("{base}/s/{slug}"),
        None => format!("{base}/share/{}", lien.key),
    };
    LienVue { lien, url }
}

/// Every link of the user, newest first, or those of an album.
#[tauri::command]
pub async fn liens_partages(etat: State<'_, Etat>, album: Option<String>) -> Result<Vec<LienVue>, String> {
    let client = etat.client()?;
    let mut liens = client.liens_partages(album.as_deref()).await.map_err(|e| e.to_string())?;
    liens.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let base = base(&client).await;
    Ok(liens.into_iter().map(|l| vue(&base, l)).collect())
}

/// Creates a link. `lien` holds the fields as Immich names them (`type`,
/// `albumId` or `assetIds`, `expiresAt`, `password`, `allowDownload`...).
#[tauri::command]
pub async fn creer_lien(etat: State<'_, Etat>, lien: serde_json::Value) -> Result<LienVue, String> {
    let client = etat.client()?;
    let cree = client.creer_lien(&lien).await.map_err(|e| e.to_string())?;
    Ok(vue(&base(&client).await, cree))
}

#[tauri::command]
pub async fn modifier_lien(etat: State<'_, Etat>, id: String, modification: serde_json::Value) -> Result<LienVue, String> {
    let client = etat.client()?;
    let lien = client.modifier_lien(&id, &modification).await.map_err(|e| e.to_string())?;
    Ok(vue(&base(&client).await, lien))
}

#[tauri::command]
pub async fn supprimer_lien(etat: State<'_, Etat>, id: String) -> Result<(), String> {
    etat.client()?.supprimer_lien(&id).await.map_err(|e| e.to_string())
}

/// The users of the server (to share an album with).
#[tauri::command]
pub async fn utilisateurs_serveur(etat: State<'_, Etat>) -> Result<Vec<Utilisateur>, String> {
    etat.client()?.utilisateurs().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn membres_album(etat: State<'_, Etat>, id: String) -> Result<MembresAlbum, String> {
    etat.client()?.membres_album(&id).await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NouveauMembre {
    utilisateur: String,
    role: String,
}

#[tauri::command]
pub async fn ajouter_membres(etat: State<'_, Etat>, id: String, membres: Vec<NouveauMembre>) -> Result<(), String> {
    let membres: Vec<(String, String)> = membres.into_iter().map(|m| (m.utilisateur, m.role)).collect();
    etat.client()?.ajouter_membres(&id, &membres).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn changer_role(etat: State<'_, Etat>, id: String, utilisateur: String, role: String) -> Result<(), String> {
    etat.client()?.changer_role(&id, &utilisateur, &role).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn retirer_membre(etat: State<'_, Etat>, id: String, utilisateur: String) -> Result<(), String> {
    etat.client()?.retirer_membre(&id, &utilisateur).await.map_err(|e| e.to_string())
}

/// Characters a folder name cannot hold on Windows (and `/` on Linux).
fn nom_de_dossier(nom: &str) -> String {
    let propre: String = nom.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    let propre = propre.trim().trim_end_matches('.').to_string();
    if propre.is_empty() { "Album".into() } else { propre }
}

/// Downloads the originals of a whole album into a folder of its name,
/// inside `dossier`.
#[tauri::command]
pub async fn telecharger_album(
    app: tauri::AppHandle,
    etat: State<'_, Etat>,
    id: String,
    nom: String,
    dossier: PathBuf,
) -> Result<crate::telechargement::Bilan, String> {
    if !dossier.is_dir() {
        return Err(format!("dossier introuvable : {}", dossier.display()));
    }
    let client = etat.client()?;
    let assets = client.assets_de_selection(&Selection::Album { id, nom: nom.clone() }).await.map_err(|e| e.to_string())?;
    let ids: Vec<String> = assets.into_iter().map(|a| a.id).collect();
    let cible = dossier.join(nom_de_dossier(&nom));
    std::fs::create_dir_all(&cible).map_err(|e| e.to_string())?;
    Ok(crate::telechargement::telecharger(app, &client, &ids, &cible).await)
}

#[cfg(test)]
mod tests {
    use super::nom_de_dossier;

    #[test]
    fn noms_de_dossier_surs() {
        assert_eq!(nom_de_dossier("Vacances : été 2026"), "Vacances _ été 2026");
        assert_eq!(nom_de_dossier("a/b\\c"), "a_b_c");
        assert_eq!(nom_de_dossier(" ... "), "Album");
    }
}
