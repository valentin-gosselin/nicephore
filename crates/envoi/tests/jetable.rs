//! Upload engine against the disposable Immich server
//! (`scripts/immich-jetable.sh tester`). Never against a real library.

use api_immich::*;
use envoi::{Base, Cible, Moteur, poser_temoin};
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

/// A bigger variant (about 2 MB) so uploads take a little time.
fn grosse_variante(jpeg: &[u8], texte: &str) -> Vec<u8> {
    let mut sortie = variante(jpeg, texte);
    let remplissage = vec![b'x'; 65_000];
    for _ in 0..30 {
        let mut segment = vec![0xFF, 0xFE];
        segment.extend_from_slice(&((remplissage.len() + 2) as u16).to_be_bytes());
        segment.extend_from_slice(&remplissage);
        sortie.splice(2..2, segment);
    }
    sortie
}

async fn dossier_de_60(nom: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join(nom);
    let jpeg = photo_du_serveur();
    for i in 0..60 {
        ecrire(&racine.join(format!("p{i:02}.jpg")), &grosse_variante(&jpeg, &format!("{nom} {i} {jeton}")), 3_600);
    }
    patienter().await;
    (temporaire, racine)
}

async fn attendre_un_envoi(moteur: &Moteur) {
    for _ in 0..600 {
        if journal(moteur, "envoye") > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("aucun envoi");
}

async fn nettoyer(c: &ClientImmich, moteur: &Moteur) {
    let ids: Vec<String> = moteur.base().journal(1_000).unwrap().into_iter().filter_map(|e| e.asset_id).collect();
    if !ids.is_empty() {
        c.mettre_a_la_corbeille(&ids).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "serveur Immich jetable"]
async fn la_pause_arrete_tout_de_suite() {
    let c = client().await;
    let (_temporaire, racine) = dossier_de_60("pause").await;
    let moteur = moteur();
    moteur.base().ajouter_regle(&racine, &Cible::Bibliotheque).unwrap();
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    let boucle = tokio::spawn(moteur.lancer());

    attendre_un_envoi(&moteur).await;
    moteur.mettre_en_pause(true);
    let au_moment = journal(&moteur, "envoye");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let apres = journal(&moteur, "envoye");
    println!("pause : {au_moment} envoyés au moment de la pause, {apres} trois secondes après");
    assert_eq!(apres, au_moment, "plus rien ne part après la pause");
    assert!(apres < 60, "la pause doit couper le passage en cours");

    // Resuming sends the rest.
    moteur.mettre_en_pause(false);
    for _ in 0..300 {
        if journal(&moteur, "envoye") >= 60 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(journal(&moteur, "envoye"), 60, "la reprise envoie le reste");
    boucle.abort();
    nettoyer(&c, &moteur).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "serveur Immich jetable"]
async fn retirer_le_dossier_arrete_tout_de_suite() {
    let c = client().await;
    let (_temporaire, racine) = dossier_de_60("retrait").await;
    let moteur = moteur();
    let regle = moteur.base().ajouter_regle(&racine, &Cible::Bibliotheque).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    let boucle = tokio::spawn(moteur.lancer());

    attendre_un_envoi(&moteur).await;
    let ids_avant: Vec<String> = moteur.base().journal(1_000).unwrap().into_iter().filter_map(|e| e.asset_id).collect();
    moteur.base().supprimer_regle(regle).unwrap();
    moteur.regles_changees();
    let au_moment = journal(&moteur, "envoye");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let apres = journal(&moteur, "envoye");
    println!("retrait : {au_moment} envoyés au moment du retrait, {apres} trois secondes après");
    assert_eq!(apres, au_moment, "plus rien ne part après le retrait");
    assert!(apres < 60);
    boucle.abort();
    let ids: Vec<String> = moteur.base().journal(1_000).unwrap().into_iter().filter_map(|e| e.asset_id).collect();
    assert!(ids.len() >= ids_avant.len());
    nettoyer(&c, &moteur).await;
}

async fn ids_essais(c: &ClientImmich, noms: &[&str]) -> Vec<String> {
    let mut ids = Vec::new();
    for nom in noms {
        let page = c
            .rechercher(&CriteresRecherche { texte: nom.to_string(), mode: ModeRecherche::FichierNom, ..Default::default() }, 1)
            .await
            .unwrap();
        ids.push(page.assets.first().unwrap_or_else(|| panic!("{nom} absent du serveur jetable")).id.clone());
    }
    ids
}

fn sha1_fichier(chemin: &Path) -> String {
    envoi::analyse::empreinte(chemin).unwrap()
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn reception_d_un_album() {
    let c = client().await;
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let noms = ["essai-10.jpg", "essai-11.jpg", "essai-12.jpg", "essai-13.jpg"];
    let ids = ids_essais(&c, &noms).await;
    let album = c.creer_album(&format!("Réception {jeton}"), "", &ids).await.unwrap();
    let images = PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/nicephore-immich-jetable/images");

    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join("recu");
    // Already here, same content: adopted. Same name, other content: kept,
    // the photo arrives beside it.
    ecrire(&racine.join("essai-11.jpg"), &std::fs::read(images.join("essai-11.jpg")).unwrap(), 3_600);
    ecrire(&racine.join("essai-12.jpg"), b"mon fichier a moi", 3_600);

    let moteur = moteur();
    let selection = Selection::Album { id: album.id.clone(), nom: album.album_name.clone() };
    let regle = moteur.base().ajouter_reception(&racine, &selection, envoi::Rangement::Plat).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    moteur.passage().await.unwrap();

    let bilan = compter(&moteur, regle);
    assert_eq!((bilan.recus, bilan.a_recevoir, bilan.echecs_reception), (4, 0, 0), "{bilan:?}");
    assert_eq!(journal(&moteur, "recu"), 3, "la copie identique n'est pas retéléchargée");
    for nom in ["essai-10.jpg", "essai-11.jpg", "essai-13.jpg"] {
        assert_eq!(sha1_fichier(&racine.join(nom)), sha1_fichier(&images.join(nom)), "{nom}");
    }
    assert_eq!(std::fs::read(racine.join("essai-12.jpg")).unwrap(), b"mon fichier a moi", "jamais écrasé");
    assert_eq!(sha1_fichier(&racine.join("essai-12 (2).jpg")), sha1_fichier(&images.join("essai-12.jpg")));
    assert!(std::fs::read_dir(&racine).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".part")));

    // Deleted here: not downloaded again. Removed from the album: kept here.
    std::fs::remove_file(racine.join("essai-10.jpg")).unwrap();
    c.retirer_de_l_album(&album.id, &[ids[3].clone()]).await.unwrap();
    moteur.passage().await.unwrap();
    let bilan = compter(&moteur, regle);
    assert_eq!((bilan.retires_ici, bilan.retires_serveur), (1, 1), "{bilan:?}");
    assert!(!racine.join("essai-10.jpg").exists());
    assert!(racine.join("essai-13.jpg").exists(), "la copie locale reste");

    // On request, what was deleted here comes back.
    moteur.base().recevoir_de_nouveau(regle).unwrap();
    moteur.passage().await.unwrap();
    assert!(racine.join("essai-10.jpg").exists());
    c.supprimer_album(&album.id).await.unwrap();
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn reception_d_une_periode_par_annee_et_mois() {
    let c = client().await;
    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join("periode");
    std::fs::create_dir_all(&racine).unwrap();
    let moteur = moteur();
    // The seeded photos of February 2025 (essai-06 to essai-11, minus any
    // moved by other tests).
    let selection = Selection::Periode { du: Some("2025-02-01".into()), au: Some("2025-02-28".into()) };
    let attendus = c.assets_de_selection(&selection).await.unwrap();
    assert!(!attendus.is_empty());
    let regle = moteur.base().ajouter_reception(&racine, &selection, envoi::Rangement::AnneeMois).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    moteur.passage().await.unwrap();
    assert_eq!(compter(&moteur, regle).recus as usize, attendus.len());
    let fevrier: Vec<_> = std::fs::read_dir(racine.join("2025/02")).unwrap().flatten().collect();
    assert_eq!(fevrier.len(), attendus.len());
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "serveur Immich jetable"]
async fn la_pause_arrete_une_reception() {
    let c = client().await;
    // 60 big photos sent into an album, then received elsewhere.
    let (_temporaire, source) = dossier_de_60("a-recevoir").await;
    let album = c.creer_album(&format!("Grosse réception {}", source.display().to_string().len()), "", &[]).await.unwrap();
    let envoi = moteur();
    envoi.base().ajouter_regle(&source, &Cible::Album { id: album.id.clone(), nom: album.album_name.clone() }).unwrap();
    envoi.definir_client(Some(std::sync::Arc::new(c.clone())));
    envoi.passage().await.unwrap();
    assert_eq!(journal(&envoi, "envoye"), 60);

    let destination = tempfile::tempdir().unwrap();
    let racine = destination.path().join("recu");
    std::fs::create_dir_all(&racine).unwrap();
    let moteur = moteur();
    let selection = Selection::Album { id: album.id.clone(), nom: album.album_name.clone() };
    moteur.base().ajouter_reception(&racine, &selection, envoi::Rangement::Plat).unwrap();
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    let boucle = tokio::spawn(moteur.lancer());
    for _ in 0..600 {
        if journal(&moteur, "recu") > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    moteur.mettre_en_pause(true);
    let au_moment = journal(&moteur, "recu");
    tokio::time::sleep(Duration::from_secs(3)).await;
    let apres = journal(&moteur, "recu");
    let fichiers = std::fs::read_dir(&racine).unwrap().flatten().count();
    println!("réception : {au_moment} reçus à la pause, {apres} trois secondes après, {fichiers} fichiers");
    assert_eq!(apres, au_moment);
    assert!(apres < 60);
    assert_eq!(fichiers, apres, "aucun fichier partiel laissé");
    boucle.abort();
    nettoyer(&c, &envoi).await;
    c.supprimer_album(&album.id).await.unwrap();
}

/// A private trash for the tests: deleted files never reach the user's.
fn corbeille_privee(dossier: &Path) -> PathBuf {
    let donnees = dossier.join("donnees-xdg");
    std::fs::create_dir_all(&donnees).unwrap();
    // Tests run on one thread (--test-threads=1).
    unsafe { std::env::set_var("XDG_DATA_HOME", &donnees) };
    donnees.join("Trash/files")
}

async fn nombre_dans_l_album(c: &ClientImmich, id: &str) -> u64 {
    c.album(id).await.unwrap().asset_count
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn double_sens_sans_miroir() {
    let c = client().await;
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let ids = ids_essais(&c, &["essai-14.jpg", "essai-15.jpg"]).await;
    let album = c.creer_album(&format!("Deux sens {jeton}"), "", &ids).await.unwrap();
    let temporaire = tempfile::tempdir().unwrap();
    let racine = temporaire.path().join("deux-sens");
    ecrire(&racine.join("locale.jpg"), &variante(&photo_du_serveur(), &format!("locale {jeton}")), 3_600);
    poser_temoin(&racine).unwrap();
    patienter().await;

    let moteur = moteur();
    let regle = moteur.base().ajouter_double(&racine, &album.id, &album.album_name, false).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 3, "la photo locale monte dans l'album");
    for nom in ["locale.jpg", "essai-14.jpg", "essai-15.jpg"] {
        assert!(racine.join(nom).exists(), "{nom}");
    }
    assert_eq!(journal(&moteur, "recu"), 2, "seules les photos de l'album descendent");

    // The downloaded files are recognised, not sent back.
    patienter().await;
    moteur.passage().await.unwrap();
    assert_eq!((journal(&moteur, "envoye"), journal(&moteur, "recu")), (1, 2));

    // Without mirroring, deletions stay on their side.
    std::fs::remove_file(racine.join("essai-14.jpg")).unwrap();
    let envoyee = moteur.base().journal(100).unwrap().into_iter().find(|e| e.action == "envoye").unwrap().asset_id.unwrap();
    c.retirer_de_l_album(&album.id, std::slice::from_ref(&envoyee)).await.unwrap();
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 2, "essai-14 reste dans l'album");
    assert!(racine.join("locale.jpg").exists(), "le fichier local reste");
    let bilan = compter(&moteur, regle);
    assert_eq!((bilan.retires_ici, bilan.retires_serveur), (1, 1), "{bilan:?}");

    c.mettre_a_la_corbeille(&[envoyee]).await.unwrap();
    c.supprimer_album(&album.id).await.unwrap();
}

#[tokio::test]
#[ignore = "serveur Immich jetable"]
async fn double_sens_avec_miroir_et_garde_fous() {
    let c = client().await;
    let jeton = format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
    let ids = ids_essais(&c, &["essai-16.jpg", "essai-17.jpg"]).await;
    let album = c.creer_album(&format!("Miroir {jeton}"), "", &ids).await.unwrap();
    let temporaire = tempfile::tempdir().unwrap();
    let corbeille = corbeille_privee(temporaire.path());
    let racine = temporaire.path().join("miroir");
    let jpeg = photo_du_serveur();
    // 40 local photos: deleting one is below the 5 % threshold, 25 above.
    for i in 0..40 {
        ecrire(&racine.join(format!("m{i:02}.jpg")), &variante(&jpeg, &format!("miroir {i} {jeton}")), 3_600);
    }
    poser_temoin(&racine).unwrap();
    patienter().await;

    let moteur = moteur();
    moteur.regler_delai_de_grace(Duration::from_secs(2));
    let regle = moteur.base().ajouter_double(&racine, &album.id, &album.album_name, true).unwrap().id;
    moteur.definir_client(Some(std::sync::Arc::new(c.clone())));
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 42);
    patienter().await;
    moteur.passage().await.unwrap();

    // Deleted here: out of the album after the grace delay, still in the library.
    std::fs::remove_file(racine.join("essai-16.jpg")).unwrap();
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 42, "pas avant le délai de grâce");
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 41, "retirée de l'album");
    assert_eq!(ids_essais(&c, &["essai-16.jpg"]).await.len(), 1, "toujours dans la photothèque");

    // Removed from the album: the local file goes to the trash.
    let m00 = moteur
        .base()
        .journal(1_000)
        .unwrap()
        .into_iter()
        .find(|e| e.action == "envoye" && e.chemin.as_deref() == Some("m00.jpg"))
        .unwrap()
        .asset_id
        .unwrap();
    c.retirer_de_l_album(&album.id, &[m00]).await.unwrap();
    moteur.passage().await.unwrap();
    assert!(racine.join("m00.jpg").exists(), "pas avant le délai de grâce");
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    moteur.passage().await.unwrap();
    assert!(!racine.join("m00.jpg").exists());
    assert!(corbeille.join("m00.jpg").exists(), "dans la corbeille, pas effacé");

    // Above the threshold: nothing until confirmed.
    for i in 10..35 {
        std::fs::remove_file(racine.join(format!("m{i:02}.jpg"))).unwrap();
    }
    moteur.passage().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 40, "25 suppressions attendent une confirmation");
    assert_eq!(moteur.instantane().confirmations.get(&regle), Some(&25));
    moteur.base().confirmer_suppressions(regle).unwrap();
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 15, "appliquées une fois confirmées");

    // Marker file gone: nothing at all.
    std::fs::remove_file(racine.join(envoi::TEMOIN)).unwrap();
    std::fs::remove_file(racine.join("m01.jpg")).unwrap();
    moteur.passage().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2_500)).await;
    moteur.passage().await.unwrap();
    assert_eq!(nombre_dans_l_album(&c, &album.id).await, 15);
    assert!(moteur.instantane().erreurs_regles.get(&regle).is_some_and(|e| e.contains("témoin")));

    let envoyes: Vec<String> = moteur.base().journal(1_000).unwrap().into_iter().filter(|e| e.action == "envoye").filter_map(|e| e.asset_id).collect();
    c.mettre_a_la_corbeille(&envoyes).await.unwrap();
    c.supprimer_album(&album.id).await.unwrap();
}
