//! The engine loop: passes over every active rule, woken by folder changes,
//! by the user, by due retries, or every 15 minutes.

use api_immich::{ClientImmich, ElementVerification, Erreur as ErreurApi, MetaEnvoi, TypesMedias};
use futures_util::{StreamExt, TryStreamExt};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::{Notify, watch};

use crate::analyse::{self, FichierLocal};
use crate::base::{AEnvoyer, Base, Cible, Regle};
use crate::{Erreur, Resultat, maintenant_ms};

/// Files sent at the same time.
const ENVOIS_PARALLELES: usize = 3;
/// Files looked up per `bulk-upload-check` request.
const LOT_VERIFICATION: usize = 500;
/// Assets added to an album per request.
const LOT_ALBUM: usize = 1_000;
/// Pass even without any event.
const PERIODE: Duration = Duration::from_secs(15 * 60);
/// Quiet time after a folder event before a pass starts.
const CALME: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    #[default]
    Inactif,
    Analyse,
    Empreintes,
    Verification,
    Envoi,
    Albums,
}

/// What the engine is doing, for the interface.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EtatEnvoi {
    pub phase: Phase,
    pub regle: Option<i64>,
    pub fichier: Option<String>,
    pub faits: u64,
    pub total: u64,
    pub octets_faits: u64,
    pub octets_total: u64,
    pub en_pause: bool,
    pub connecte: bool,
    /// Blocking problem (session expired, missing right...).
    pub erreur: Option<String>,
    /// Problem per rule (folder missing, album deleted...).
    pub erreurs_regles: BTreeMap<i64, String>,
    pub dernier_passage: Option<i64>,
    /// Why sending waits (on battery, metered network...).
    pub attente: Option<String>,
    /// Files sent by the last finished pass.
    pub envoyes_passage: u64,
}

/// Tells whether uploads must wait, and why.
pub type Condition = Arc<dyn Fn() -> Option<String> + Send + Sync>;

pub struct Moteur {
    base: Mutex<Base>,
    client: Mutex<Option<Arc<ClientImmich>>>,
    etat: watch::Sender<EtatEnvoi>,
    reveil: Arc<Notify>,
    dernier_evenement: Arc<AtomicI64>,
    pause: AtomicBool,
    /// How long a file must stay unchanged before it is sent.
    stabilite_ms: AtomicI64,
    surveillance: Mutex<Option<notify::RecommendedWatcher>>,
    condition: Mutex<Option<Condition>>,
    envoyes_passage: AtomicU64,
}

/// Outcome of one file upload.
enum Issue {
    Envoye,
    Saute,
}

impl Moteur {
    pub fn nouveau(base: Base) -> Arc<Self> {
        Arc::new(Self {
            base: Mutex::new(base),
            client: Mutex::new(None),
            etat: watch::Sender::new(EtatEnvoi::default()),
            reveil: Arc::new(Notify::new()),
            dernier_evenement: Arc::new(AtomicI64::new(0)),
            pause: AtomicBool::new(false),
            stabilite_ms: AtomicI64::new(analyse::DELAI_STABILITE.as_millis() as i64),
            surveillance: Mutex::new(None),
            condition: Mutex::new(None),
            envoyes_passage: AtomicU64::new(0),
        })
    }

    /// The database, for the commands of the interface (short operations).
    pub fn base(&self) -> MutexGuard<'_, Base> {
        self.base.lock().expect("verrou de la base d'envoi")
    }

    pub fn etat(&self) -> watch::Receiver<EtatEnvoi> {
        self.etat.subscribe()
    }

    pub fn instantane(&self) -> EtatEnvoi {
        self.etat.borrow().clone()
    }

    /// Server connection used for sending (`None` when logged out).
    pub fn definir_client(&self, client: Option<Arc<ClientImmich>>) {
        let connecte = client.is_some();
        *self.client.lock().expect("verrou du client") = client;
        self.etat.send_modify(|e| {
            e.connecte = connecte;
            e.erreur = None;
        });
        self.reveiller();
    }

    pub fn reveiller(&self) {
        self.reveil.notify_one();
    }

    pub fn mettre_en_pause(&self, pause: bool) {
        self.pause.store(pause, Ordering::SeqCst);
        self.etat.send_modify(|e| e.en_pause = pause);
        if !pause {
            self.reveiller();
        }
    }

    /// Switches to another database (another account): its rules are
    /// watched instead of the previous ones.
    pub fn remplacer_base(&self, base: Base) {
        *self.base.lock().expect("verrou de la base d'envoi") = base;
        self.etat.send_modify(|e| e.erreurs_regles.clear());
        self.regles_changees();
    }

    /// Changes how long a file must stay unchanged before it is sent
    /// (10 s by default).
    pub fn regler_stabilite(&self, delai: Duration) {
        self.stabilite_ms.store(delai.as_millis() as i64, Ordering::SeqCst);
    }

    /// Uploads wait while `condition` gives a reason (checked before every
    /// pass and every file).
    pub fn definir_condition(&self, condition: Condition) {
        *self.condition.lock().expect("verrou de la condition") = Some(condition);
        self.reveiller();
    }

    async fn raison_d_attendre(&self) -> Option<String> {
        let condition = self.condition.lock().expect("verrou de la condition").clone()?;
        tokio::task::spawn_blocking(move || condition()).await.ok().flatten()
    }

    /// Call after adding, removing or switching rules.
    pub fn regles_changees(&self) {
        self.surveiller();
        self.reveiller();
    }

    /// Starts watching the folders and returns the background loop, to be
    /// spawned on the caller's runtime.
    pub fn lancer(self: &Arc<Self>) -> impl std::future::Future<Output = ()> + Send + 'static {
        self.surveiller();
        let moteur = Arc::clone(self);
        async move { moteur.boucle().await }
    }

    async fn boucle(&self) {
        loop {
            let mut attente = PERIODE;
            if !self.pause.load(Ordering::SeqCst) {
                match self.passage().await {
                    Ok(Some(revoir)) => attente = attente.min(revoir),
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("passage d'envoi interrompu : {e}");
                        self.etat.send_modify(|etat| etat.erreur = Some(e.to_string()));
                    }
                }
            }
            let _ = tokio::time::timeout(attente, self.reveil.notified()).await;
            // Let a burst of folder events settle (a card being copied...).
            loop {
                let calme = maintenant_ms() - self.dernier_evenement.load(Ordering::SeqCst);
                if calme >= CALME.as_millis() as i64 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }

    /// Watches the folders of the active rules.
    fn surveiller(&self) {
        use notify::Watcher;
        let regles = self.base().regles().unwrap_or_default();
        let reveil = Arc::clone(&self.reveil);
        let dernier = Arc::clone(&self.dernier_evenement);
        let veilleur = notify::recommended_watcher(move |evenement: notify::Result<notify::Event>| {
            let Ok(evenement) = evenement else { return };
            if matches!(evenement.kind, notify::EventKind::Create(_) | notify::EventKind::Modify(_)) {
                dernier.store(maintenant_ms(), Ordering::SeqCst);
                reveil.notify_one();
            }
        });
        let mut veilleur = match veilleur {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("surveillance des dossiers impossible : {e}");
                return;
            }
        };
        for regle in regles.iter().filter(|r| r.actif) {
            if let Err(e) = veilleur.watch(Path::new(&regle.dossier), notify::RecursiveMode::Recursive) {
                tracing::warn!("surveillance de {} impossible : {e}", regle.dossier);
            }
        }
        *self.surveillance.lock().expect("verrou de la surveillance") = Some(veilleur);
    }

    /// One pass over every active rule. Returns when the next pass is
    /// wanted earlier than usual (files still being written, retries due).
    pub async fn passage(&self) -> Resultat<Option<Duration>> {
        let client = self.client.lock().expect("verrou du client").clone().ok_or(Erreur::SansConnexion)?;
        if let Some(raison) = self.raison_d_attendre().await {
            tracing::info!("envoi en attente : {raison}");
            self.etat.send_modify(|e| e.attente = Some(raison));
            return Ok(Some(Duration::from_secs(60)));
        }
        let debut = std::time::Instant::now();
        self.envoyes_passage.store(0, Ordering::SeqCst);
        self.etat.send_modify(|e| e.attente = None);
        let types = client.types_medias().await?;
        let regles: Vec<Regle> = self.base().regles()?.into_iter().filter(|r| r.actif).collect();
        let mut revoir: Option<Duration> = None;
        let mut erreurs = BTreeMap::new();
        for regle in &regles {
            if self.pause.load(Ordering::SeqCst) {
                break;
            }
            match self.passage_regle(&client, &types, regle).await {
                Ok(Some(d)) => revoir = Some(revoir.map_or(d, |r: Duration| r.min(d))),
                Ok(None) => {}
                // Lost session or missing right: stop everything.
                Err(Erreur::Api(e @ (ErreurApi::CleRefusee | ErreurApi::PermissionManquante(_)))) => {
                    self.fin_passage(erreurs);
                    return Err(Erreur::Api(e));
                }
                Err(e) => {
                    tracing::warn!("règle {} : {e}", regle.dossier);
                    erreurs.insert(regle.id, e.to_string());
                }
            }
        }
        // Stopped halfway by a condition: look again in a minute.
        if self.etat.borrow().attente.is_some() {
            let minute = Duration::from_secs(60);
            revoir = Some(revoir.map_or(minute, |r| r.min(minute)));
        }
        if let Some(prochain) = self.base().prochain_essai(maintenant_ms())? {
            let d = Duration::from_millis((prochain - maintenant_ms()).max(1_000) as u64);
            revoir = Some(revoir.map_or(d, |r| r.min(d)));
        }
        tracing::info!(
            "passage d'envoi : {} règles en {} ms, {} envoyés, revoir dans {:?}",
            regles.len(),
            debut.elapsed().as_millis(),
            self.envoyes_passage.load(Ordering::SeqCst),
            revoir
        );
        self.fin_passage(erreurs);
        Ok(revoir)
    }

    fn fin_passage(&self, erreurs: BTreeMap<i64, String>) {
        let envoyes = self.envoyes_passage.swap(0, Ordering::SeqCst);
        self.etat.send_modify(|e| {
            *e = EtatEnvoi {
                en_pause: e.en_pause,
                connecte: e.connecte,
                attente: e.attente.take(),
                erreurs_regles: erreurs,
                dernier_passage: Some(maintenant_ms()),
                envoyes_passage: envoyes,
                ..Default::default()
            }
        });
    }

    fn phase(&self, regle: i64, phase: Phase, total: u64) {
        self.etat.send_modify(|e| {
            e.phase = phase;
            e.regle = Some(regle);
            e.fichier = None;
            e.faits = 0;
            e.total = total;
            e.octets_faits = 0;
            e.octets_total = 0;
            e.erreur = None;
        });
    }

    async fn passage_regle(&self, client: &ClientImmich, types: &TypesMedias, regle: &Regle) -> Resultat<Option<Duration>> {
        let racine = PathBuf::from(&regle.dossier);

        // 1. Walk the folder. An unreadable root stops the rule: nothing is
        // concluded from a missing disk.
        self.phase(regle.id, Phase::Analyse, 0);
        let (r, t) = (racine.clone(), types.clone());
        let inventaire = tokio::task::spawn_blocking(move || analyse::inventorier(&r, &t))
            .await
            .map_err(|e| Erreur::Regle(e.to_string()))?
            .map_err(|e| Erreur::Regle(format!("dossier illisible ({e})")))?;

        // 2. New or changed files, once stable, get a fingerprint.
        let connus = self.base().fichiers_connus(regle.id)?;
        let maintenant = maintenant_ms();
        let delai = Duration::from_millis(self.stabilite_ms.load(Ordering::SeqCst).max(0) as u64);
        let mut revoir = None;
        tracing::debug!("{} : {} fichiers, {} connus", regle.dossier, inventaire.fichiers.len(), connus.len());
        let a_empreinter: Vec<FichierLocal> = inventaire
            .fichiers
            .into_iter()
            .filter(|f| connus.get(&f.chemin).is_none_or(|c| c.taille != f.taille || c.modifie_ms != f.modifie_ms))
            .filter(|f| {
                let stable = analyse::stable(f, maintenant, delai);
                if !stable {
                    revoir = Some(delai + Duration::from_secs(1));
                }
                stable
            })
            .collect();
        self.phase(regle.id, Phase::Empreintes, a_empreinter.len() as u64);
        for fichier in a_empreinter {
            self.etat.send_modify(|e| e.fichier = Some(fichier.chemin.clone()));
            let chemin = analyse::absolu(&racine, &fichier.chemin);
            let resultat = tokio::task::spawn_blocking(move || {
                let empreinte = analyse::empreinte(&chemin)?;
                // Changed while hashed: looked at again next pass.
                let meta = std::fs::metadata(&chemin)?;
                let modifie = meta.modified().map(analyse_ms).unwrap_or(0);
                Ok::<_, std::io::Error>((empreinte, meta.len() == fichier.taille && modifie == fichier.modifie_ms, fichier))
            })
            .await
            .map_err(|e| Erreur::Regle(e.to_string()))?;
            match resultat {
                Ok((empreinte, true, fichier)) => {
                    self.base().noter_empreinte(regle.id, &fichier.chemin, fichier.taille, fichier.modifie_ms, &empreinte)?
                }
                Ok((_, false, _)) => revoir = Some(delai + Duration::from_secs(1)),
                Err(e) => tracing::warn!("empreinte impossible : {e}"),
            }
            self.etat.send_modify(|e| e.faits += 1);
        }

        // 3. Look the fingerprints up on the server: known files are not sent.
        let a_verifier = self.base().a_verifier(regle.id)?;
        self.phase(regle.id, Phase::Verification, a_verifier.len() as u64);
        for lot in a_verifier.chunks(LOT_VERIFICATION) {
            let elements: Vec<ElementVerification> = lot
                .iter()
                .enumerate()
                .map(|(i, (_, sha1))| ElementVerification { id: i.to_string(), checksum: sha1.clone() })
                .collect();
            let resultats = client.verifier_envois(&elements).await?;
            let base = self.base();
            for resultat in resultats {
                let Some((chemin, _)) = resultat.id.parse::<usize>().ok().and_then(|i| lot.get(i)) else { continue };
                match (resultat.action.as_str(), resultat.reason.as_deref(), resultat.asset_id.as_deref()) {
                    ("accept", _, _) => base.marquer_attente(regle.id, chemin)?,
                    (_, Some("duplicate"), Some(id)) => {
                        base.marquer_present(regle.id, chemin, id)?;
                        if resultat.is_trashed {
                            // In the server's trash: left there, not put back in an album.
                            base.marquer_album_fait(regle.id, std::slice::from_ref(chemin))?;
                            base.journaliser(Some(regle.id), Some(chemin), "dans_la_corbeille", None, Some(id))?;
                        }
                    }
                    (_, raison, _) => {
                        let raison = raison.unwrap_or("refusé par le serveur");
                        base.marquer_ignore(regle.id, chemin, raison)?;
                        base.journaliser(Some(regle.id), Some(chemin), "ignore", Some(raison), None)?;
                    }
                }
            }
            self.etat.send_modify(|e| e.faits += lot.len() as u64);
        }

        // 4. Send the unknown files, a few at a time, streamed from disk.
        let a_envoyer = self.base().a_envoyer(regle.id, maintenant_ms())?;
        self.phase(regle.id, Phase::Envoi, a_envoyer.len() as u64);
        let octets_total: u64 = a_envoyer.iter().map(|f| f.taille).sum();
        self.etat.send_modify(|e| e.octets_total = octets_total);
        let octets = Arc::new(AtomicU64::new(0));
        let mut envois = futures_util::stream::iter(a_envoyer)
            .map(|fichier| {
                let octets = Arc::clone(&octets);
                let racine = racine.clone();
                async move {
                    let issue = self.envoyer(client, regle, &racine, &fichier, &octets).await;
                    (fichier, issue)
                }
            })
            .buffer_unordered(ENVOIS_PARALLELES);
        while let Some((fichier, issue)) = envois.next().await {
            match issue {
                Ok(Issue::Envoye) | Ok(Issue::Saute) => {}
                Err(e @ Erreur::Api(ErreurApi::CleRefusee | ErreurApi::PermissionManquante(_))) => return Err(e),
                Err(e) => tracing::warn!("{} : {e}", fichier.chemin),
            }
            self.etat.send_modify(|e| {
                e.faits += 1;
                e.fichier = Some(fichier.chemin.clone());
            });
        }
        drop(envois);

        // 5. Albums.
        self.associer_albums(client, regle).await?;
        Ok(revoir)
    }

    async fn envoyer(
        &self,
        client: &ClientImmich,
        regle: &Regle,
        racine: &Path,
        fichier: &AEnvoyer,
        octets: &Arc<AtomicU64>,
    ) -> Resultat<Issue> {
        // A condition appeared (unplugged, hotspot): the rest waits.
        if let Some(raison) = self.raison_d_attendre().await {
            self.etat.send_modify(|e| e.attente = Some(raison));
            return Ok(Issue::Saute);
        }
        let chemin = analyse::absolu(racine, &fichier.chemin);
        // Changed or gone since its fingerprint: the next pass decides.
        let meta = match tokio::fs::metadata(&chemin).await {
            Ok(m) => m,
            Err(_) => return Ok(Issue::Saute),
        };
        let modifie = meta.modified().map(analyse_ms).unwrap_or(0);
        if meta.len() != fichier.taille || modifie != fichier.modifie_ms {
            return Ok(Issue::Saute);
        }
        let cree = meta.created().map(analyse_ms).ok().filter(|&c| c > 0).map_or(modifie, |c| c.min(modifie));
        let nom = chemin.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let meta_envoi = MetaEnvoi {
            nom_fichier: nom,
            taille: fichier.taille,
            sha1: fichier.sha1.clone(),
            cree_le: analyse::date_rfc3339(cree),
            modifie_le: analyse::date_rfc3339(modifie),
        };
        let resultat = match tokio::fs::File::open(&chemin).await {
            Ok(f) => {
                let compteur = Arc::clone(octets);
                let etat = self.etat.clone();
                let flux = tokio_util::io::ReaderStream::new(f).inspect_ok(move |morceau| {
                    let faits = compteur.fetch_add(morceau.len() as u64, Ordering::Relaxed) + morceau.len() as u64;
                    etat.send_modify(|e| e.octets_faits = faits);
                });
                client.envoyer_asset(&meta_envoi, reqwest::Body::wrap_stream(flux)).await
            }
            Err(e) => Err(ErreurApi::Configuration(format!("lecture impossible : {e}"))),
        };
        let base = self.base();
        match resultat {
            Ok(reponse) if reponse.status == "duplicate" => {
                base.marquer_present(regle.id, &fichier.chemin, &reponse.id)?;
                Ok(Issue::Envoye)
            }
            Ok(reponse) => {
                base.marquer_envoye(regle.id, &fichier.chemin, &reponse.id)?;
                self.envoyes_passage.fetch_add(1, Ordering::SeqCst);
                base.journaliser(Some(regle.id), Some(&fichier.chemin), "envoye", None, Some(&reponse.id))?;
                Ok(Issue::Envoye)
            }
            Err(e @ (ErreurApi::CleRefusee | ErreurApi::PermissionManquante(_))) => Err(Erreur::Api(e)),
            Err(e) => {
                // Network or overload: up to 8 tries over about a day.
                // Anything else (refused, too big...): 3 tries.
                let essais = fichier.essais + 1;
                let definitif = if e.est_passagere() { essais >= 8 } else { essais >= 3 };
                let delai = 30_000_i64.saturating_mul(1 << essais.min(10)).min(6 * 3_600_000);
                base.marquer_essai_rate(regle.id, &fichier.chemin, &e.to_string(), maintenant_ms() + delai, definitif)?;
                if definitif {
                    base.journaliser(Some(regle.id), Some(&fichier.chemin), "abandon", Some(&e.to_string()), None)?;
                }
                Err(Erreur::Api(e))
            }
        }
    }

    async fn associer_albums(&self, client: &ClientImmich, regle: &Regle) -> Resultat<()> {
        let sans_album = self.base().sans_album(regle.id)?;
        if sans_album.is_empty() {
            return Ok(());
        }
        self.phase(regle.id, Phase::Albums, sans_album.len() as u64);
        match &regle.cible {
            Cible::Bibliotheque => {
                let chemins: Vec<String> = sans_album.into_iter().map(|(c, _)| c).collect();
                self.base().marquer_album_fait(regle.id, &chemins)?;
            }
            Cible::Album { id, nom } => {
                if let Err(e) = self.ajouter(client, regle, id, &sans_album).await {
                    if matches!(e, Erreur::Api(ErreurApi::Introuvable)) {
                        return Err(Erreur::Regle(format!("l'album « {nom} » n'existe plus sur le serveur")));
                    }
                    return Err(e);
                }
            }
            Cible::AlbumParDossier => {
                let mut par_dossier: HashMap<String, Vec<(String, String)>> = HashMap::new();
                for (chemin, asset) in sans_album {
                    par_dossier.entry(analyse::dossier_de(&chemin).to_string()).or_default().push((chemin, asset));
                }
                for (dossier, elements) in par_dossier {
                    let connu = self.base().album_dossier(regle.id, &dossier)?;
                    match connu {
                        Some(album) => match self.ajouter(client, regle, &album, &elements).await {
                            Ok(()) => {}
                            Err(Erreur::Api(ErreurApi::Introuvable)) => {
                                // Deleted on the server by the user: not created again.
                                let chemins: Vec<String> = elements.into_iter().map(|(c, _)| c).collect();
                                let base = self.base();
                                base.marquer_album_fait(regle.id, &chemins)?;
                                base.journaliser(Some(regle.id), Some(&dossier), "album_disparu", None, Some(&album))?;
                            }
                            Err(e) => return Err(e),
                        },
                        None => {
                            let nom = nom_album(&regle.dossier, &dossier);
                            let (chemins, ids): (Vec<String>, Vec<String>) = elements.into_iter().unzip();
                            let premiers = &ids[..ids.len().min(LOT_ALBUM)];
                            let album = client.creer_album(&nom, "", premiers).await?;
                            {
                                let base = self.base();
                                base.noter_album_dossier(regle.id, &dossier, &album.id)?;
                                base.journaliser(Some(regle.id), Some(&dossier), "album_cree", Some(&nom), Some(&album.id))?;
                            }
                            let reste: Vec<(String, String)> =
                                chemins.iter().cloned().zip(ids.iter().cloned()).skip(LOT_ALBUM).collect();
                            self.base().marquer_album_fait(regle.id, &chemins[..premiers.len()])?;
                            if !reste.is_empty() {
                                self.ajouter(client, regle, &album.id, &reste).await?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn ajouter(&self, client: &ClientImmich, regle: &Regle, album: &str, elements: &[(String, String)]) -> Resultat<()> {
        for lot in elements.chunks(LOT_ALBUM) {
            let ids: Vec<String> = lot.iter().map(|(_, a)| a.clone()).collect();
            client.ajouter_a_l_album(album, &ids).await?;
            let chemins: Vec<String> = lot.iter().map(|(c, _)| c.clone()).collect();
            self.base().marquer_album_fait(regle.id, &chemins)?;
            self.etat.send_modify(|e| e.faits += lot.len() as u64);
        }
        Ok(())
    }
}

fn analyse_ms(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Album name for a folder of a rule: the folder's own name, or the rule's
/// folder name for files at its root.
fn nom_album(racine: &str, dossier: &str) -> String {
    let source = if dossier.is_empty() { racine } else { dossier };
    source.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(source).to_string()
}

#[cfg(test)]
mod tests {
    use super::nom_album;

    #[test]
    fn noms_des_albums_par_dossier() {
        assert_eq!(nom_album("/home/v/Images/Appareil", ""), "Appareil");
        assert_eq!(nom_album("/home/v/Images", "2024/Vacances à Groix"), "Vacances à Groix");
        assert_eq!(nom_album("C:\\Users\\v\\Pictures", ""), "Pictures");
    }
}
