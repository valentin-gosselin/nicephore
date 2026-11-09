//! Virtual folder mounted for real (FUSE) on the disposable Immich server.
//! Needs `fusermount3`: build in the toolbox with `--no-run`, run on the host.

#![cfg(target_os = "linux")]

use api_immich::*;
use std::sync::Arc;
use virtuel::{CacheOriginaux, Rangement, monter};

async fn client() -> ClientImmich {
    let url = std::env::var("IMMICH_TEST_URL").expect("IMMICH_TEST_URL (serveur jetable)");
    assert!(url.contains("127.0.0.1") || url.contains("localhost"));
    let session = ouvrir_session(&url, &std::env::var("IMMICH_TEST_EMAIL").unwrap(), &std::env::var("IMMICH_TEST_PASSWORD").unwrap())
        .await
        .unwrap();
    ClientImmich::nouveau(&url, &Authentification::Session(session.access_token)).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "serveur Immich jetable et FUSE"]
async fn dossier_virtuel_d_une_periode() {
    let c = client().await;
    let selection = Selection::Periode { du: Some("2025-02-01".into()), au: Some("2025-02-28".into()) };
    let assets = c.assets_de_selection(&selection).await.unwrap();
    assert!(!assets.is_empty());
    let temporaire = tempfile::tempdir().unwrap();
    let point = temporaire.path().join("montage");
    std::fs::create_dir_all(&point).unwrap();
    let cache = Arc::new(CacheOriginaux::nouveau(temporaire.path().join("cache"), 1 << 30));

    // A folder with files in it is refused.
    let plein = temporaire.path().join("plein");
    std::fs::create_dir_all(&plein).unwrap();
    std::fs::write(plein.join("a.txt"), "x").unwrap();
    let refus = monter(&plein, &assets, Rangement::Plat, Arc::clone(&cache), Arc::new(c.clone()), tokio::runtime::Handle::current());
    assert!(refus.is_err());

    let montage = monter(&point, &assets, Rangement::AnneeMois, Arc::clone(&cache), Arc::new(c.clone()), tokio::runtime::Handle::current())
        .unwrap();
    let (p, attendus) = (point.clone(), assets.clone());
    let cache_test = Arc::clone(&cache);
    tokio::task::spawn_blocking(move || {
        let fevrier = p.join("2025").join("02");
        let mut noms: Vec<String> = std::fs::read_dir(&fevrier).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        noms.sort();
        assert_eq!(noms.len(), attendus.len(), "{noms:?}");
        let premier = attendus.iter().find(|a| a.nom_fichier == noms[0]).unwrap();
        let chemin = fevrier.join(&noms[0]);
        assert_eq!(std::fs::metadata(&chemin).unwrap().len(), premier.taille.unwrap(), "taille réelle annoncée");
        assert_eq!(cache_test.telechargements(), 0, "rien n'est téléchargé pour lister");

        // Reading downloads the original once, checked.
        let contenu = std::fs::read(&chemin).unwrap();
        use sha1::Digest;
        assert_eq!(hex::encode(sha1::Sha1::digest(&contenu)), premier.sha1);
        assert_eq!(cache_test.telechargements(), 1);
        let encore = std::fs::read(&chemin).unwrap();
        assert_eq!(encore, contenu);
        assert_eq!(cache_test.telechargements(), 1, "la deuxième lecture vient du cache");

        // Read only.
        let ecriture = std::fs::File::create(fevrier.join("nouveau.jpg"));
        assert!(ecriture.is_err(), "écriture refusée");
        assert!(std::fs::remove_file(&chemin).is_err(), "suppression refusée");
    })
    .await
    .unwrap();

    montage.demonter();
    assert_eq!(std::fs::read_dir(&point).unwrap().count(), 0, "dossier vide une fois démonté");
}
