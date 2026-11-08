//! Upload engine against the disposable Immich server
//! (`scripts/immich-jetable.sh tester`). Never against a real library.

use api_immich::*;
use envoi::{Base, Cible, Moteur};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

async fn client() -> ClientImmich {
    let url = std::env::var("IMMICH_TEST_URL").expect("IMMICH_TEST_URL (serveur jetable)");
    assert!(url.contains("127.0.0.1") || url.contains("localhost"), "serveur local jetable uniquement");
    let session = ouvrir_session(
        &url,
        &std::env::var("IMMICH_TEST_EMAIL").unwrap(),
        &std::env::var("IMMICH_TEST_PASSWORD").unwrap(),
    )
    .await
    .unwrap();
    ClientImmich::nouveau(&url, &Authentification::Session(session.access_token)).unwrap()
}

/// A photo already on the server (seeded by the disposable server script).
fn photo_du_serveur() -> Vec<u8> {
    let donnees = std::env::var("NICEPHORE_IMMICH_JETABLE").map(PathBuf::from).unwrap_or_else(|_| {
        PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/nicephore-immich-jetable")
    });
    std::fs::read(donnees.join("images/essai-00.jpg")).expect("images du serveur jetable")
}

/// The same JPEG with a comment segment right after SOI: a valid image
/// with another checksum.
fn variante(jpeg: &[u8], texte: &str) -> Vec<u8> {
    let commentaire = texte.as_bytes();
    let longueur = (commentaire.len() + 2) as u16;
    let mut sortie = jpeg[..2].to_vec();
    sortie.extend_from_slice(&[0xFF, 0xFE]);
    sortie.extend_from_slice(&longueur.to_be_bytes());
    sortie.extend_from_slice(commentaire);
    sortie.extend_from_slice(&jpeg[2..]);
    sortie
}

/// Writes a file dated in the past (so it counts as complete).
fn ecrire(chemin: &Path, contenu: &[u8], il_y_a: u64) {
    std::fs::create_dir_all(chemin.parent().unwrap()).unwrap();
    std::fs::write(chemin, contenu).unwrap();
    let date = SystemTime::now() - Duration::from_secs(il_y_a);
    std::fs::File::options().write(true).open(chemin).unwrap().set_modified(date).unwrap();
}

/// Engine with a short stability delay (2 s instead of 10).
fn moteur() -> std::sync::Arc<Moteur> {
    let moteur = Moteur::nouveau(Base::en_memoire().unwrap());
    moteur.regler_stabilite(Duration::from_secs(2));
    moteur
}

/// Lets the files just written become stable.
async fn patienter() {
    tokio::time::sleep(Duration::from_millis(2_500)).await;
}

fn compter(moteur: &Moteur, regle: i64) -> envoi::Bilan {
    moteur.base().bilans().unwrap().remove(&regle).unwrap_or_default()
}

fn journal(moteur: &Moteur, action: &str) -> usize {
    moteur.base().journal(1_000).unwrap().iter().filter(|e| e.action == action).count()
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn envoi_d_un_dossier_avec_albums_par_dossier() {
    let c = client().await;
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join(format!("Appareil {jeton}"));
    let sortie = format!("Sortie vélo {jeton}");
    let jpeg = photo_du_serveur();

    ecrire(&racine.join("a.jpg"), &variante(&jpeg, &format!("a {jeton}")), 3_600);
    ecrire(&racine.join(&sortie).join("b.jpg"), &variante(&jpeg, &format!("b {jeton}")), 3_600);
    ecrire(&racine.join(&sortie).join("c.JPG"), &variante(&jpeg, &format!("c {jeton}")), 3_600);
    // Already on the server: not sent again.
    ecrire(&racine.join("copie.jpg"), &jpeg, 3_600);
    // Skipped: not a media file, hidden, being written.
    ecrire(&racine.join("notes.txt"), b"x", 3_600);
    ecrire(&racine.join(".cache/d.jpg"), &variante(&jpeg, "cache"), 3_600);
    ecrire(&racine.join("e.jpg.part"), b"x", 3_600);
    patienter().await;
    ecrire(&racine.join("recent.jpg"), &variante(&jpeg, &format!("recent {jeton}")), 0);

    let moteur = moteur();
    let regle = moteur.base().ajouter_regle(&racine, &Cible::AlbumParDossier).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));

    let revoir = moteur.passage().await.unwrap();
    assert!(revoir.is_some(), "le fichier récent demande un nouveau passage");
    let bilan = compter(&moteur, regle);
    assert_eq!((bilan.envoyes, bilan.presents, bilan.echecs), (3, 1, 0), "{bilan:?}");
    assert_eq!(journal(&moteur, "envoye"), 3);
    assert_eq!(journal(&moteur, "album_cree"), 2);

    let albums = c.albums().await.unwrap();
    let album_sortie = albums.iter().find(|a| a.album_name == sortie).expect("album du sous-dossier");
    assert_eq!(album_sortie.asset_count, 2);
    let album_racine = albums.iter().find(|a| a.album_name == format!("Appareil {jeton}")).expect("album de la racine");
    // a.jpg and the duplicate already on the server.
    assert_eq!(album_racine.asset_count, 2);

    // Nothing new: nothing sent.
    moteur.passage().await.unwrap();
    assert_eq!(journal(&moteur, "envoye"), 3);

    // The recent file, once complete, and a changed file are sent.
    ecrire(&racine.join("recent.jpg"), &variante(&jpeg, &format!("recent {jeton}")), 60);
    ecrire(&racine.join("a.jpg"), &variante(&jpeg, &format!("a modifiée {jeton}")), 120);
    patienter().await;
    moteur.passage().await.unwrap();
    assert_eq!(journal(&moteur, "envoye"), 5);
    assert_eq!(compter(&moteur, regle).envoyes, 4);
    assert_eq!(c.album(&album_racine.id).await.unwrap().asset_count, 4);

    // Clean up the disposable server.
    let ids: Vec<String> = moteur.base().journal(1_000).unwrap().into_iter().filter(|e| e.action == "envoye").filter_map(|e| e.asset_id).collect();
    c.mettre_a_la_corbeille(&ids).await.unwrap();
    for album in [album_sortie, album_racine] {
        c.supprimer_album(&album.id).await.unwrap();
    }
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn envoi_vers_un_album_choisi_et_album_disparu() {
    let c = client().await;
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join("choisi");
    let jpeg = photo_du_serveur();
    ecrire(&racine.join("x/y.jpg"), &variante(&jpeg, &format!("y {jeton}")), 3_600);
    patienter().await;

    let album = c.creer_album(&format!("Choisi {jeton}"), "", &[]).await.unwrap();
    let moteur = moteur();
    let cible = Cible::Album { id: album.id.clone(), nom: album.album_name.clone() };
    let regle = moteur.base().ajouter_regle(&racine, &cible).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    moteur.passage().await.unwrap();
    assert_eq!(compter(&moteur, regle).envoyes, 1);
    assert_eq!(c.album(&album.id).await.unwrap().asset_count, 1);

    // The album is deleted on the server: the rule reports it, never
    // recreates an album with the same name.
    c.supprimer_album(&album.id).await.unwrap();
    ecrire(&racine.join("z.jpg"), &variante(&jpeg, &format!("z {jeton}")), 3_600);
    patienter().await;
    moteur.passage().await.unwrap();
    let etat = moteur.instantane();
    assert!(etat.erreurs_regles.get(&regle).is_some_and(|e| e.contains("n'existe plus")), "{etat:?}");
    assert!(!c.albums().await.unwrap().iter().any(|a| a.album_name == album.album_name));

    let ids: Vec<String> = moteur.base().journal(100).unwrap().into_iter().filter_map(|e| e.asset_id).collect();
    c.mettre_a_la_corbeille(&ids).await.unwrap();
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn une_photo_qui_arrive_part_toute_seule() {
    let c = client().await;
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join("surveille");
    let jpeg = photo_du_serveur();
    ecrire(&racine.join("premiere.jpg"), &variante(&jpeg, &format!("premiere {jeton}")), 3_600);

    let moteur = moteur();
    let regle = moteur.base().ajouter_regle(&racine, &Cible::Bibliotheque).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    let boucle = tokio::spawn(moteur.lancer());

    let attendre = |n: u64| {
        let moteur = std::sync::Arc::clone(&moteur);
        async move {
            for _ in 0..60 {
                if compter(&moteur, regle).envoyes >= n {
                    return true;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            false
        }
    };
    assert!(attendre(1).await, "premier passage");

    // A photo copied into a sub-folder, dated in the past like a file from
    // a camera card: the folder event wakes the engine.
    ecrire(&racine.join("2026/arrivee.jpg"), &variante(&jpeg, &format!("arrivee {jeton}")), 3_600);
    assert!(attendre(2).await, "la photo arrivée doit partir sans action");
    boucle.abort();

    let ids: Vec<String> = moteur.base().journal(100).unwrap().into_iter().filter_map(|e| e.asset_id).collect();
    c.mettre_a_la_corbeille(&ids).await.unwrap();
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn attente_sur_batterie() {
    let c = client().await;
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join("batterie");
    ecrire(&racine.join("p.jpg"), &variante(&photo_du_serveur(), &format!("p {jeton}")), 3_600);
    patienter().await;

    let moteur = moteur();
    let regle = moteur.base().ajouter_regle(&racine, &Cible::Bibliotheque).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    let sur_batterie = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let drapeau = std::sync::Arc::clone(&sur_batterie);
    moteur.definir_condition(std::sync::Arc::new(move || {
        drapeau.load(std::sync::atomic::Ordering::SeqCst).then(|| "sur batterie".to_string())
    }));

    let revoir = moteur.passage().await.unwrap();
    assert_eq!(revoir, Some(Duration::from_secs(60)));
    assert_eq!(moteur.instantane().attente.as_deref(), Some("sur batterie"));
    assert_eq!(compter(&moteur, regle).envoyes, 0);

    sur_batterie.store(false, std::sync::atomic::Ordering::SeqCst);
    moteur.passage().await.unwrap();
    let etat = moteur.instantane();
    assert_eq!((etat.attente, etat.envoyes_passage), (None, 1));
    assert_eq!(compter(&moteur, regle).envoyes, 1);

    let ids: Vec<String> = moteur.base().journal(100).unwrap().into_iter().filter_map(|e| e.asset_id).collect();
    c.mettre_a_la_corbeille(&ids).await.unwrap();
}
