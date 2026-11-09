//! The engine loop: passes over every active rule, woken by folder changes,
//! by the user, by due retries, or every 15 minutes.

use api_immich::{AssetDistant, ClientImmich, ElementVerification, Erreur as ErreurApi, MetaEnvoi, TypesMedias};
use futures_util::{StreamExt, TryStreamExt};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::{Notify, watch};

use crate::analyse::{self, FichierLocal};
use crate::base::{AEnvoyer, Base, Cible, EtatRecu, Regle};
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
/// Mirrored deletions beyond this many files (or 5 % of the folder) wait
/// for the user's confirmation.
const SEUIL_SUPPRESSIONS: usize = 20;
/// Hidden file marking a two-way folder: without it (unplugged disk,
/// folder replaced), no deletion is ever mirrored.
pub const TEMOIN: &str = ".nicephore-dossier";

/// Puts the marker file in a two-way folder.
pub fn poser_temoin(dossier: &Path) -> std::io::Result<()> {
    std::fs::write(dossier.join(TEMOIN), "Dossier synchronisé par Nicéphore avec un album Immich.\n")
}

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
    Reception,
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
    /// Two-way rules whose mirrored deletions wait for a confirmation.
    pub confirmations: BTreeMap<i64, u64>,
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
    /// Bumped by a pause or any rule change: the running pass stops at once
    /// (between two files, and during an upload, which is cancelled).
    arret: watch::Sender<u64>,
    /// Delay before a mirrored deletion is applied (10 minutes).
    grace_ms: AtomicI64,
    confirmations: Mutex<BTreeMap<i64, u64>>,
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
            arret: watch::Sender::new(0),
            grace_ms: AtomicI64::new(10 * 60_000),
            confirmations: Mutex::new(BTreeMap::new()),
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
        if pause {
            self.interrompre();
        } else {
            self.reveiller();
        }
    }

    /// Stops the running pass right away.
    fn interrompre(&self) {
        self.arret.send_modify(|g| *g += 1);
    }

    /// The pass started at generation `generation` must stop.
    fn interrompu(&self, generation: u64) -> bool {
        self.pause.load(Ordering::SeqCst) || *self.arret.borrow() != generation
    }

    /// Switches to another database (another account): its rules are
    /// watched instead of the previous ones.
    pub fn remplacer_base(&self, base: Base) {
        *self.base.lock().expect("verrou de la base d'envoi") = base;
        self.etat.send_modify(|e| e.erreurs_regles.clear());
        self.regles_changees();
    }

    /// Changes the delay before a mirrored deletion is applied.
    pub fn regler_delai_de_grace(&self, delai: Duration) {
        self.grace_ms.store(delai.as_millis() as i64, Ordering::SeqCst);
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

    /// Call after adding, removing or switching rules: the running pass
    /// stops (a removed or suspended folder sends nothing more) and a new
    /// one starts with the rules as they are now.
    pub fn regles_changees(&self) {
        self.interrompre();
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
        // Received folders are not watched: the server is their source, and
        // our own writes would wake the engine for nothing.
        for regle in regles.iter().filter(|r| r.actif && r.sens != crate::Sens::Descendante) {
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
        let generation = *self.arret.borrow();
        let types = client.types_medias().await?;
        let regles: Vec<Regle> = self.base().regles()?.into_iter().filter(|r| r.actif).collect();
        let mut revoir: Option<Duration> = None;
        let mut erreurs = BTreeMap::new();
        for regle in &regles {
            if self.interrompu(generation) {
                tracing::info!("passage d'envoi interrompu (pause ou règles changées)");
                break;
            }
            let resultat = match regle.sens {
                crate::Sens::Descendante => self.passage_reception(&client, regle, generation).await,
                crate::Sens::Double => self.passage_double(&client, &types, regle, generation).await,
                crate::Sens::Montante => self.passage_regle(&client, &types, regle, generation).await,
            };
            match resultat {
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
        let confirmations = self.confirmations.lock().expect("verrou des confirmations").clone();
        self.etat.send_modify(|e| {
            *e = EtatEnvoi {
                en_pause: e.en_pause,
                connecte: e.connecte,
                attente: e.attente.take(),
                erreurs_regles: erreurs,
                dernier_passage: Some(maintenant_ms()),
                envoyes_passage: envoyes,
                confirmations,
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

    async fn passage_regle(
        &self,
        client: &ClientImmich,
        types: &TypesMedias,
        regle: &Regle,
        generation: u64,
    ) -> Resultat<Option<Duration>> {
        let racine = PathBuf::from(&regle.dossier);

        // 1. Walk the folder. An unreadable root stops the rule: nothing is
        // concluded from a missing disk.
        self.phase(regle.id, Phase::Analyse, 0);
        let (r, t) = (racine.clone(), types.clone());
        let inventaire = tokio::task::spawn_blocking(move || analyse::inventorier(&r, &t))
            .await
            .map_err(|e| Erreur::Regle(e.to_string()))?
            .map_err(|e| Erreur::Regle(format!("dossier illisible ({e})")))?;

        if self.interrompu(generation) {
            return Ok(None);
        }

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
            if self.interrompu(generation) {
                return Ok(None);
            }
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
            if self.interrompu(generation) {
                return Ok(None);
            }
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

        if self.interrompu(generation) {
            return Ok(None);
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
                    let issue = self.envoyer(client, regle, &racine, &fichier, &octets, generation).await;
                    (fichier, issue)
                }
            })
            .buffer_unordered(ENVOIS_PARALLELES);
        while let Some((fichier, issue)) = envois.next().await {
            if self.interrompu(generation) {
                // Dropping the stream cancels the uploads still running.
                return Ok(None);
            }
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
        if self.interrompu(generation) {
            return Ok(None);
        }

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
        generation: u64,
    ) -> Resultat<Issue> {
        if self.interrompu(generation) {
            return Ok(Issue::Saute);
        }
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
                // A pause or a removed folder cancels the upload under way.
                let mut arret = self.arret.subscribe();
                tokio::select! {
                    resultat = client.envoyer_asset(&meta_envoi, reqwest::Body::wrap_stream(flux)) => resultat,
                    _ = async {
                        while !self.interrompu(generation) {
                            if arret.changed().await.is_err() {
                                std::future::pending::<()>().await;
                            }
                        }
                    } => return Ok(Issue::Saute),
                }
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

impl Moteur {
    /// Download rule: copies the selection into the folder. Files deleted
    /// here are not downloaded again, files gone from the server stay here;
    /// nothing local is ever deleted or overwritten.
    async fn passage_reception(&self, client: &ClientImmich, regle: &Regle, generation: u64) -> Resultat<Option<Duration>> {
        self.reception(client, regle, generation, false).await.map(|(revoir, _)| revoir)
    }

    /// Two-way rule: the folder is sent into the album, the album received
    /// into the folder; with `miroir`, deletions on one side are repeated on
    /// the other, after a grace delay and with a confirmation above a
    /// threshold, never when the marker file is missing.
    async fn passage_double(
        &self,
        client: &ClientImmich,
        types: &TypesMedias,
        regle: &Regle,
        generation: u64,
    ) -> Resultat<Option<Duration>> {
        let racine = PathBuf::from(&regle.dossier);
        if std::fs::read_dir(&racine).is_err() {
            return Err(Erreur::Regle("dossier introuvable ou illisible (disque débranché ?)".into()));
        }
        if regle.miroir && !racine.join(TEMOIN).exists() {
            return Err(Erreur::Regle(format!(
                "fichier témoin {TEMOIN} absent : dossier remplacé ou débranché ? Rien n'est synchronisé en attendant"
            )));
        }
        let revoir_envoi = self.passage_regle(client, types, regle, generation).await?;
        if self.interrompu(generation) {
            return Ok(None);
        }
        // Sent (or found on the server) files are the local copies of their
        // album photos.
        {
            let base = self.base();
            for (asset_id, chemin) in base.fichiers_lies(regle.id)? {
                base.lier_recu(regle.id, &asset_id, &chemin)?;
            }
        }
        let (revoir_reception, candidats) = self.reception(client, regle, generation, regle.miroir).await?;
        if self.interrompu(generation) {
            return Ok(None);
        }
        let revoir_suppressions = if regle.miroir {
            self.appliquer_suppressions(client, regle, &racine, candidats).await?
        } else {
            None
        };
        Ok([revoir_envoi, revoir_reception, revoir_suppressions].into_iter().flatten().min())
    }

    /// Records the deletions seen, then applies those confirmed or past
    /// their grace delay. Removing from the album keeps the photo in the
    /// library; a local file goes to the system trash.
    async fn appliquer_suppressions(
        &self,
        client: &ClientImmich,
        regle: &Regle,
        racine: &Path,
        candidats: Vec<(String, String, String)>,
    ) -> Resultat<Option<Duration>> {
        let Cible::Album { id: album, .. } = &regle.cible else { return Ok(None) };
        let (attente, total) = {
            let base = self.base();
            base.noter_suppressions(regle.id, &candidats)?;
            (base.suppressions(regle.id)?, base.recus(regle.id)?.len())
        };
        let mut confirmations = self.confirmations.lock().expect("verrou des confirmations").clone();
        confirmations.remove(&regle.id);
        if attente.is_empty() {
            *self.confirmations.lock().expect("verrou des confirmations") = confirmations;
            return Ok(None);
        }
        let grace = self.grace_ms.load(Ordering::SeqCst);
        let maintenant = maintenant_ms();
        let depasse = attente.len() > SEUIL_SUPPRESSIONS || attente.len() * 20 > total;
        let a_faire: Vec<&crate::Suppression> = attente
            .iter()
            .filter(|s| s.confirme || (!depasse && maintenant - s.vu_le >= grace))
            .collect();
        if depasse && attente.iter().any(|s| !s.confirme) {
            confirmations.insert(regle.id, attente.iter().filter(|s| !s.confirme).count() as u64);
        }
        *self.confirmations.lock().expect("verrou des confirmations") = confirmations;

        // Server side: out of the album, in one request.
        let hors_album: Vec<&crate::Suppression> = a_faire.iter().copied().filter(|s| s.cote == "serveur").collect();
        if !hors_album.is_empty() {
            let ids: Vec<String> = hors_album.iter().map(|s| s.asset_id.clone()).collect();
            client.retirer_de_l_album(album, &ids).await?;
            let base = self.base();
            for s in hors_album {
                base.changer_etat_recu(regle.id, &s.asset_id, EtatRecu::RetireIci)?;
                base.oublier_suppression(regle.id, &s.cote, &s.asset_id)?;
                base.journaliser(Some(regle.id), Some(&s.chemin), "retire_de_l_album", None, Some(&s.asset_id))?;
            }
        }
        // Local side: to the system trash, one by one.
        for s in a_faire.iter().filter(|s| s.cote == "ici") {
            let chemin = analyse::absolu(racine, &s.chemin);
            let resultat = tokio::task::spawn_blocking(move || if chemin.exists() { trash::delete(&chemin).map_err(|e| e.to_string()) } else { Ok(()) })
                .await
                .map_err(|e| Erreur::Regle(e.to_string()))?;
            let base = self.base();
            match resultat {
                Ok(()) => {
                    base.changer_etat_recu(regle.id, &s.asset_id, EtatRecu::RetireServeur)?;
                    base.oublier_suppression(regle.id, &s.cote, &s.asset_id)?;
                    base.journaliser(Some(regle.id), Some(&s.chemin), "mis_a_la_corbeille", None, Some(&s.asset_id))?;
                }
                Err(e) => base.journaliser(Some(regle.id), Some(&s.chemin), "corbeille_impossible", Some(&e), Some(&s.asset_id))?,
            }
        }
        // The rest waits for its delay (or for a confirmation).
        let prochaine = attente
            .iter()
            .filter(|s| !s.confirme && !depasse)
            .map(|s| s.vu_le + grace - maintenant)
            .filter(|&ms| ms > 0)
            .min();
        Ok(prochaine.map(|ms| Duration::from_millis(ms as u64 + 1_000)))
    }

    async fn reception(
        &self,
        client: &ClientImmich,
        regle: &Regle,
        generation: u64,
        miroir: bool,
    ) -> Resultat<(Option<Duration>, Vec<(String, String, String)>)> {
        let racine = PathBuf::from(&regle.dossier);
        self.phase(regle.id, Phase::Analyse, 0);
        // A missing folder (unplugged disk) stops the rule: nothing is
        // concluded, nothing is written elsewhere.
        if std::fs::read_dir(&racine).is_err() {
            return Err(Erreur::Regle("dossier introuvable ou illisible (disque débranché ?)".into()));
        }
        // Temporary files left by an abrupt stop (crash, power cut).
        let r = racine.clone();
        let _ = tokio::task::spawn_blocking(move || nettoyer_partiels(&r)).await;
        let selection = regle.selection.clone().ok_or_else(|| Erreur::Regle("sélection manquante".into()))?;
        let distants = client.assets_de_selection(&selection).await?;
        if self.interrompu(generation) {
            return Ok((None, Vec::new()));
        }
        let connus = self.base().recus(regle.id)?;
        let ids: std::collections::HashSet<&str> = distants.iter().map(|a| a.id.as_str()).collect();

        // What changed since the last pass. With mirrored deletions, a file
        // gone on one side becomes a candidate deletion on the other.
        let mut candidats: Vec<(String, String, String)> = Vec::new();
        {
            let base = self.base();
            for (asset_id, recu) in &connus {
                let present_ici = analyse::absolu(&racine, &recu.chemin).exists();
                let sur_serveur = ids.contains(asset_id.as_str());
                match recu.etat {
                    EtatRecu::Recu if miroir && !sur_serveur && present_ici => {
                        candidats.push(("ici".into(), asset_id.clone(), recu.chemin.clone()));
                    }
                    EtatRecu::Recu if miroir && sur_serveur && !present_ici => {
                        candidats.push(("serveur".into(), asset_id.clone(), recu.chemin.clone()));
                    }
                    EtatRecu::Recu if !ids.contains(asset_id.as_str()) => {
                        base.changer_etat_recu(regle.id, asset_id, EtatRecu::RetireServeur)?;
                        base.journaliser(Some(regle.id), Some(&recu.chemin), "retire_du_serveur", None, Some(asset_id))?;
                    }
                    EtatRecu::Recu if !present_ici => {
                        base.changer_etat_recu(regle.id, asset_id, EtatRecu::RetireIci)?;
                        base.journaliser(Some(regle.id), Some(&recu.chemin), "retire_ici", None, Some(asset_id))?;
                    }
                    EtatRecu::Attente | EtatRecu::Echec if !ids.contains(asset_id.as_str()) => {
                        base.oublier_recu(regle.id, asset_id)?;
                    }
                    EtatRecu::RetireServeur if ids.contains(asset_id.as_str()) && present_ici => {
                        base.changer_etat_recu(regle.id, asset_id, EtatRecu::Recu)?;
                    }
                    _ => {}
                }
            }
        }
        // New on the server. A file already here under its name with the
        // same content is adopted; otherwise a free local name is reserved.
        let nouveaux: Vec<&AssetDistant> = distants.iter().filter(|a| !connus.contains_key(&a.id)).collect();
        let homonymes: Vec<(usize, PathBuf, String)> = nouveaux
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let relatif = chemin_naturel(regle.rangement, a);
                let absolu = analyse::absolu(&racine, &relatif);
                let meme_taille = std::fs::metadata(&absolu).is_ok_and(|m| a.taille.is_none_or(|t| t == m.len()));
                meme_taille.then_some((i, absolu, relatif))
            })
            .collect();
        let adoptes: HashMap<usize, String> = tokio::task::spawn_blocking({
            let attendus: Vec<String> = homonymes.iter().map(|(i, _, _)| nouveaux[*i].sha1.clone()).collect();
            move || {
                homonymes
                    .into_iter()
                    .zip(attendus)
                    .filter(|((_, absolu, _), sha1)| analyse::empreinte(absolu).is_ok_and(|e| &e == sha1))
                    .map(|((i, _, relatif), _)| (i, relatif))
                    .collect()
            }
        })
        .await
        .map_err(|e| Erreur::Regle(e.to_string()))?;
        {
            let base = self.base();
            let mut pris: std::collections::HashSet<String> = connus.values().map(|r| r.chemin.to_lowercase()).collect();
            for (i, asset) in nouveaux.iter().enumerate() {
                if let Some(relatif) = adoptes.get(&i) {
                    pris.insert(relatif.to_lowercase());
                    base.noter_recu_present(regle.id, asset, relatif)?;
                    continue;
                }
                let dossier = sous_dossier(regle.rangement, &asset.date_locale);
                let chemin = nom_libre(&racine, &dossier, &asset.nom_fichier, &asset.id, &mut pris);
                base.reserver_recu(regle.id, asset, &chemin)?;
            }
        }
        if self.interrompu(generation) {
            return Ok((None, Vec::new()));
        }

        let a_recevoir = self.base().a_recevoir(regle.id, maintenant_ms())?;
        if a_recevoir.is_empty() {
            return Ok((None, candidats));
        }
        // Room on the disk for everything, plus a margin.
        let besoin: u64 = a_recevoir.iter().map(|f| f.taille).sum();
        if let Ok(libre) = fs4::available_space(&racine) {
            let marge = 1 << 30;
            if besoin + marge > libre {
                return Err(Erreur::Regle(format!(
                    "place insuffisante : {} Mo à recevoir, {} Mo libres",
                    besoin / 1_048_576,
                    libre / 1_048_576
                )));
            }
        }
        self.phase(regle.id, Phase::Reception, a_recevoir.len() as u64);
        self.etat.send_modify(|e| e.octets_total = besoin);
        let octets = Arc::new(AtomicU64::new(0));
        let mut receptions = futures_util::stream::iter(a_recevoir)
            .map(|fichier| {
                let octets = Arc::clone(&octets);
                let racine = racine.clone();
                async move {
                    let issue = self.recevoir(client, regle, &racine, &fichier, &octets, generation).await;
                    (fichier, issue)
                }
            })
            .buffer_unordered(ENVOIS_PARALLELES);
        // On interruption the queue is drained, not dropped: every download
        // under way stops by itself and removes its temporary file.
        while let Some((fichier, issue)) = receptions.next().await {
            if self.interrompu(generation) {
                continue;
            }
            match issue {
                Ok(()) => {}
                Err(e @ Erreur::Api(ErreurApi::CleRefusee | ErreurApi::PermissionManquante(_))) => return Err(e),
                Err(e) => tracing::warn!("{} : {e}", fichier.chemin),
            }
            self.etat.send_modify(|e| {
                e.faits += 1;
                e.fichier = Some(fichier.chemin.clone());
            });
        }
        Ok((None, candidats))
    }

    /// Downloads one original into a hidden temporary file, checks its
    /// SHA-1, then moves it into place without ever replacing a file.
    async fn recevoir(
        &self,
        client: &ClientImmich,
        regle: &Regle,
        racine: &Path,
        fichier: &crate::base::ARecevoir,
        octets: &Arc<AtomicU64>,
        generation: u64,
    ) -> Resultat<()> {
        use sha1::{Digest, Sha1};
        use tokio::io::AsyncWriteExt;
        if self.interrompu(generation) {
            return Ok(());
        }
        let cible = analyse::absolu(racine, &fichier.chemin);
        let dossier = cible.parent().map(Path::to_path_buf).unwrap_or_else(|| racine.to_path_buf());
        let echec = |e: String, essais: u32| -> Resultat<()> {
            let essais = essais + 1;
            let definitif = essais >= 5;
            let delai = 30_000_i64.saturating_mul(1 << essais.min(10)).min(6 * 3_600_000);
            let base = self.base();
            base.marquer_recu_rate(regle.id, &fichier.asset_id, &e, maintenant_ms() + delai, definitif)?;
            if definitif {
                base.journaliser(Some(regle.id), Some(&fichier.chemin), "abandon", Some(&e), Some(&fichier.asset_id))?;
            }
            Ok(())
        };
        if let Err(e) = tokio::fs::create_dir_all(&dossier).await {
            return echec(format!("dossier impossible à créer : {e}"), fichier.essais);
        }
        // The reserved name got taken meanwhile: same content means done,
        // otherwise another name, never an overwrite.
        if cible.exists() {
            let c = cible.clone();
            let meme = tokio::task::spawn_blocking(move || analyse::empreinte(&c)).await.ok().and_then(Result::ok);
            if meme.as_deref() == Some(fichier.sha1.as_str()) {
                self.base().changer_etat_recu(regle.id, &fichier.asset_id, EtatRecu::Recu)?;
                return Ok(());
            }
            let nom = cible.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let parent = analyse::dossier_de(&fichier.chemin).to_string();
            let mut pris = std::collections::HashSet::new();
            let autre = nom_libre(racine, &parent, &nom, &fichier.asset_id, &mut pris);
            self.base().deplacer_recu(regle.id, &fichier.asset_id, &autre)?;
            return Ok(());
        }
        let partiel = dossier.join(format!(".nicephore-{}.part", fichier.asset_id));
        let mut arret = self.arret.subscribe();
        let telechargement = async {
            let reponse = client.flux_original(&fichier.asset_id).await.map_err(|e| e.to_string())?;
            let mut sortie = tokio::fs::File::create(&partiel).await.map_err(|e| e.to_string())?;
            let mut hacheur = Sha1::new();
            let mut flux = reponse.bytes_stream();
            while let Some(morceau) = flux.next().await {
                let morceau = morceau.map_err(|e| e.to_string())?;
                hacheur.update(&morceau);
                sortie.write_all(&morceau).await.map_err(|e| e.to_string())?;
                let faits = octets.fetch_add(morceau.len() as u64, Ordering::Relaxed) + morceau.len() as u64;
                self.etat.send_modify(|e| e.octets_faits = faits);
            }
            sortie.flush().await.map_err(|e| e.to_string())?;
            sortie.sync_all().await.map_err(|e| e.to_string())?;
            Ok::<String, String>(hacheur.finalize().iter().map(|o| format!("{o:02x}")).collect())
        };
        let resultat = tokio::select! {
            r = telechargement => Some(r),
            _ = async {
                while !self.interrompu(generation) {
                    if arret.changed().await.is_err() {
                        std::future::pending::<()>().await;
                    }
                }
            } => None,
        };
        let empreinte = match resultat {
            None => {
                let _ = tokio::fs::remove_file(&partiel).await;
                return Ok(());
            }
            Some(Err(e)) => {
                let _ = tokio::fs::remove_file(&partiel).await;
                return echec(e, fichier.essais);
            }
            Some(Ok(e)) => e,
        };
        if empreinte != fichier.sha1 {
            let _ = tokio::fs::remove_file(&partiel).await;
            return echec("empreinte différente de celle du serveur".into(), fichier.essais);
        }
        // Original modification date, then into place (never replacing).
        if let Ok(date) = humantime::parse_rfc3339_weak(&fichier.modifie_le)
            && let Ok(f) = std::fs::File::options().write(true).open(&partiel)
        {
            let _ = f.set_modified(date);
        }
        if cible.exists() {
            let _ = tokio::fs::remove_file(&partiel).await;
            return Ok(());
        }
        if let Err(e) = tokio::fs::rename(&partiel, &cible).await {
            let _ = tokio::fs::remove_file(&partiel).await;
            return echec(format!("mise en place impossible : {e}"), fichier.essais);
        }
        let base = self.base();
        base.changer_etat_recu(regle.id, &fichier.asset_id, EtatRecu::Recu)?;
        base.journaliser(Some(regle.id), Some(&fichier.chemin), "recu", None, Some(&fichier.asset_id))?;
        Ok(())
    }
}

/// Removes our `.nicephore-*.part` temporary files under `racine`.
fn nettoyer_partiels(racine: &Path) {
    for entree in walkdir::WalkDir::new(racine).follow_links(false).into_iter().flatten() {
        let nom = entree.file_name().to_string_lossy();
        if entree.file_type().is_file() && nom.starts_with(".nicephore-") && nom.ends_with(".part") {
            let _ = std::fs::remove_file(entree.path());
        }
    }
}

/// Where an asset goes when its name is free.
fn chemin_naturel(rangement: crate::Rangement, asset: &AssetDistant) -> String {
    let dossier = sous_dossier(rangement, &asset.date_locale);
    let nom = nom_sur(&asset.nom_fichier, &asset.id);
    if dossier.is_empty() { nom } else { format!("{dossier}/{nom}") }
}

/// Sub-folder of a received file: none, or `YYYY/MM` from the capture date.
fn sous_dossier(rangement: crate::Rangement, date_locale: &str) -> String {
    match rangement {
        crate::Rangement::Plat => String::new(),
        crate::Rangement::AnneeMois => {
            let (annee, mois) = (date_locale.get(0..4), date_locale.get(5..7));
            match (annee, mois) {
                (Some(a), Some(m)) if a.chars().all(|c| c.is_ascii_digit()) && m.chars().all(|c| c.is_ascii_digit()) => format!("{a}/{m}"),
                _ => "sans date".into(),
            }
        }
    }
}

/// A file name valid on every system (Windows forbids more characters).
fn nom_sur(nom: &str, secours: &str) -> String {
    let propre: String = nom
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    let propre = propre.trim().trim_end_matches(['.', ' ']).trim_start_matches('.').to_string();
    if propre.is_empty() { secours.to_string() } else { propre }
}

/// A relative path free both on disk and among the reserved ones (names
/// compared without case, as on Windows): `IMG.jpg`, then `IMG (2).jpg`...
fn nom_libre(
    racine: &Path,
    dossier: &str,
    nom: &str,
    secours: &str,
    pris: &mut std::collections::HashSet<String>,
) -> String {
    let nom = nom_sur(nom, secours);
    let (base, extension) = match nom.rsplit_once('.') {
        Some((b, e)) if !b.is_empty() => (b.to_string(), format!(".{e}")),
        _ => (nom.clone(), String::new()),
    };
    for n in 1.. {
        let candidat = if n == 1 { nom.clone() } else { format!("{base} ({n}){extension}") };
        let chemin = if dossier.is_empty() { candidat } else { format!("{dossier}/{candidat}") };
        if !pris.contains(&chemin.to_lowercase()) && !analyse::absolu(racine, &chemin).exists() {
            pris.insert(chemin.to_lowercase());
            return chemin;
        }
    }
    unreachable!()
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
    fn noms_et_dossiers_des_fichiers_recus() {
        use super::{nom_libre, nom_sur, sous_dossier};
        let dossier = tempfile::tempdir().unwrap();
        std::fs::write(dossier.path().join("IMG.jpg"), b"x").unwrap();
        let mut pris = std::collections::HashSet::new();
        assert_eq!(nom_libre(dossier.path(), "", "IMG.jpg", "id", &mut pris), "IMG (2).jpg");
        assert_eq!(nom_libre(dossier.path(), "", "img (2).JPG", "id", &mut pris), "img (2) (2).JPG");
        assert_eq!(nom_libre(dossier.path(), "2026/05", "a:b?.jpg", "id", &mut pris), "2026/05/a_b_.jpg");
        assert_eq!(nom_sur("...", "secours"), "secours");
        assert_eq!(sous_dossier(crate::Rangement::AnneeMois, "2026-05-30T14:00:00"), "2026/05");
        assert_eq!(sous_dossier(crate::Rangement::AnneeMois, ""), "sans date");
        assert_eq!(sous_dossier(crate::Rangement::Plat, "2026-05-30T14:00:00"), "");
    }

    #[test]
    fn noms_des_albums_par_dossier() {
        assert_eq!(nom_album("/home/v/Images/Appareil", ""), "Appareil");
        assert_eq!(nom_album("/home/v/Images", "2024/Vacances à Groix"), "Vacances à Groix");
        assert_eq!(nom_album("C:\\Users\\v\\Pictures", ""), "Pictures");
    }
}
