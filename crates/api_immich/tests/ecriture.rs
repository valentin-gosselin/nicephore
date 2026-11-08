//! Write tests against a disposable Immich server (`scripts/immich-jetable.sh
//! tester`). Never run them against a real library: they favorite, archive,
//! trash, create and delete albums, and merge people.

use api_immich::*;

async fn client() -> ClientImmich {
    let url = std::env::var("IMMICH_TEST_URL").expect("IMMICH_TEST_URL (serveur jetable)");
    assert!(
        url.contains("127.0.0.1") || url.contains("localhost"),
        "les tests d'écriture ne visent qu'un serveur local jetable"
    );
    let session = ouvrir_session(
        &url,
        &std::env::var("IMMICH_TEST_EMAIL").unwrap(),
        &std::env::var("IMMICH_TEST_PASSWORD").unwrap(),
    )
    .await
    .unwrap();
    ClientImmich::nouveau(&url, &Authentification::Session(session.access_token)).unwrap()
}

async fn premiers(c: &ClientImmich, n: usize) -> Vec<String> {
    let page = c.rechercher(&CriteresRecherche { mode: ModeRecherche::FichierNom, ..Default::default() }, 1).await.unwrap();
    assert!(page.assets.len() >= n, "le serveur jetable doit contenir des photos");
    page.assets.iter().take(n).map(|a| a.id.clone()).collect()
}

async fn total(c: &ClientImmich, filtre: &FiltreChronologie) -> u64 {
    c.liste_mois(filtre).await.unwrap().iter().map(|m| u64::from(m.count)).sum()
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn favoris_archive_et_corbeille() {
    let c = client().await;
    let ids = premiers(&c, 3).await;
    let principal = total(&c, &FiltreChronologie::default()).await;

    let favoris = FiltreChronologie { favori: true, ..Default::default() };
    let favoris_avant = total(&c, &favoris).await;
    c.modifier_assets(&ids, &ModificationAssets { favori: Some(false), ..Default::default() }).await.unwrap();
    let base = total(&c, &favoris).await;
    c.modifier_assets(&ids, &ModificationAssets { favori: Some(true), ..Default::default() }).await.unwrap();
    assert_eq!(total(&c, &favoris).await, base + 3);
    c.modifier_assets(&ids, &ModificationAssets { favori: Some(false), ..Default::default() }).await.unwrap();
    assert_eq!(total(&c, &favoris).await, base);
    let _ = favoris_avant;

    // Counts are compared before and after: earlier runs may leave items.
    let archive = FiltreChronologie { visibilite: Some("archive".into()), ..Default::default() };
    let archive_avant = total(&c, &archive).await;
    c.modifier_assets(&ids[..2], &ModificationAssets { visibilite: Some("archive".into()), ..Default::default() }).await.unwrap();
    assert_eq!(total(&c, &archive).await, archive_avant + 2);
    assert_eq!(total(&c, &FiltreChronologie::default()).await, principal - 2);
    c.modifier_assets(&ids[..2], &ModificationAssets { visibilite: Some("timeline".into()), ..Default::default() }).await.unwrap();
    assert_eq!(total(&c, &archive).await, archive_avant);

    let corbeille = FiltreChronologie { corbeille: true, ..Default::default() };
    let corbeille_avant = total(&c, &corbeille).await;
    c.mettre_a_la_corbeille(&ids[..1]).await.unwrap();
    assert_eq!(total(&c, &corbeille).await, corbeille_avant + 1);
    c.restaurer(&ids[..1]).await.unwrap();
    assert_eq!(total(&c, &corbeille).await, corbeille_avant);
    assert_eq!(total(&c, &FiltreChronologie::default()).await, principal);
    println!("favoris, archive, corbeille et restauration : ok sur {principal} éléments");
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn albums_de_bout_en_bout() {
    let c = client().await;
    let ids = premiers(&c, 4).await;
    let photos_avant = total(&c, &FiltreChronologie::default()).await;

    let album = c.creer_album("Essai Nicéphore", "créé par les tests", &ids[..2]).await.unwrap();
    assert!(album.possede);
    assert_eq!(c.album(&album.id).await.unwrap().asset_count, 2);

    let bilan = c.ajouter_a_l_album(&album.id, &ids[1..4]).await.unwrap();
    assert_eq!((bilan.ajoutes, bilan.deja_presents, bilan.echecs), (2, 1, 0));
    let filtre = FiltreChronologie { album: Some(album.id.clone()), ..Default::default() };
    assert_eq!(total(&c, &filtre).await, 4);

    c.retirer_de_l_album(&album.id, &ids[..1]).await.unwrap();
    assert_eq!(total(&c, &filtre).await, 3);

    let modification = ModificationAlbum { nom: Some("Essai renommé".into()), description: Some("nouvelle".into()) };
    c.modifier_album(&album.id, &modification).await.unwrap();
    let relu = c.album(&album.id).await.unwrap();
    assert_eq!((relu.album_name.as_str(), relu.description.as_str()), ("Essai renommé", "nouvelle"));
    assert!(c.albums().await.unwrap().iter().any(|a| a.id == album.id && a.possede));

    c.supprimer_album(&album.id).await.unwrap();
    assert!(!c.albums().await.unwrap().iter().any(|a| a.id == album.id));
    // Deleting an album never deletes its photos.
    assert_eq!(total(&c, &FiltreChronologie::default()).await, photos_avant);
    println!("albums : création, ajout (doublon compris), retrait, renommage, suppression : ok");
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn gestion_des_personnes() {
    let c = client().await;
    let a = c.creer_personne("Essai A").await.unwrap();
    let b = c.creer_personne("Essai B").await.unwrap();
    let d = c.creer_personne("").await.unwrap();

    let renommee = c
        .modifier_personne(&d.id, &ModificationPersonne { name: Some("Essai D".into()), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(renommee.name, "Essai D");
    let favorite = c
        .modifier_personne(&a.id, &ModificationPersonne { is_favorite: Some(true), ..Default::default() })
        .await
        .unwrap();
    assert!(favorite.is_favorite);
    let masquee = c
        .modifier_personne(&b.id, &ModificationPersonne { is_hidden: Some(true), ..Default::default() })
        .await
        .unwrap();
    assert!(masquee.is_hidden);
    // `/people` only lists people with enough faces, and these test people
    // have none (no machine learning here): check the person itself.
    assert!(c.personne(&b.id).await.unwrap().is_hidden);

    assert!(c.fusionner_personnes(&a.id, std::slice::from_ref(&a.id)).await.is_err());
    let fusionnees = c.fusionner_personnes(&a.id, &[b.id.clone(), d.id.clone()]).await.unwrap();
    assert_eq!(fusionnees, 2);
    assert!(matches!(c.personne(&b.id).await, Err(Erreur::Introuvable)));
    assert_eq!(c.personne(&a.id).await.unwrap().name, "Essai A");
    println!("personnes : nommer, favori, masquer, fusion (la cible garde son nom) : ok");
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn original_et_vignettes() {
    let c = client().await;
    let id = premiers(&c, 1).await.remove(0);
    let octets = c.flux_original(&id).await.unwrap().bytes().await.unwrap();
    assert_eq!(&octets[..2], &[0xFF, 0xD8], "un JPEG d'origine");
    let vignette = c.image(&id, TailleImage::Vignette).await.unwrap();
    assert!(!vignette.octets.is_empty());
    println!("original de {} octets, vignette de {} octets : ok", octets.len(), vignette.octets.len());
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn edition_des_infos_et_tags() {
    let c = client().await;
    let ids = premiers(&c, 2).await;
    let id = ids[0].clone();

    let infos = ModificationAssets {
        description: Some("Écrit par les tests".into()),
        note: Some(Some(4)),
        latitude: Some(48.1113),
        longitude: Some(-1.68),
        date: Some("2021-07-14T10:30:00.000+02:00".into()),
        fuseau: Some("Europe/Paris".into()),
        ..Default::default()
    };
    c.modifier_assets(std::slice::from_ref(&id), &infos).await.unwrap();
    // Location and date are applied by a background job on the server.
    let mut lu = serde_json::Value::Null;
    for _ in 0..20 {
        lu = c.infos_asset(&id).await.unwrap();
        if lu["exifInfo"]["latitude"].as_f64().is_some() && lu["exifInfo"]["dateTimeOriginal"].as_str().unwrap_or("").starts_with("2021-07-14") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    assert_eq!(lu["exifInfo"]["description"], "Écrit par les tests");
    assert_eq!(lu["exifInfo"]["rating"], 4);
    assert!((lu["exifInfo"]["latitude"].as_f64().unwrap() - 48.1113).abs() < 0.001, "{}", lu["exifInfo"]);
    assert!(lu["exifInfo"]["dateTimeOriginal"].as_str().unwrap().starts_with("2021-07-14"), "{}", lu["exifInfo"]);

    let sans_note = ModificationAssets { note: Some(None), ..Default::default() };
    c.modifier_assets(std::slice::from_ref(&id), &sans_note).await.unwrap();
    assert!(c.infos_asset(&id).await.unwrap()["exifInfo"]["rating"].is_null());

    let tag = c.tag_nomme("Essai Nicéphore").await.unwrap();
    assert_eq!(c.tag_nomme("Essai Nicéphore").await.unwrap().id, tag.id, "upsert sans doublon");
    c.etiqueter(&tag.id, &ids, false).await.unwrap();
    let tags = |v: serde_json::Value| v["tags"].as_array().cloned().unwrap_or_default().iter().filter_map(|t| t["id"].as_str().map(str::to_string)).collect::<Vec<_>>();
    assert!(tags(c.infos_asset(&id).await.unwrap()).contains(&tag.id));
    c.etiqueter(&tag.id, &ids, true).await.unwrap();
    assert!(!tags(c.infos_asset(&id).await.unwrap()).contains(&tag.id));
    assert!(c.tags().await.unwrap().iter().any(|t| t.id == tag.id));
    println!("édition : description, note, lieu, date, retrait de note, tags : ok");
}
