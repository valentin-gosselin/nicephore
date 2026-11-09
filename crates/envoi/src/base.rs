//! Local SQLite database of the upload engine: rules, what is known of every
//! file (size, date, SHA-1, state), folder albums and the action log.
//!
//! The database is never replaced silently: an unreadable file is an error
//! shown to the user, and a copy is kept before every schema migration.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{Erreur, Resultat, maintenant_ms};

/// Current schema version (`PRAGMA user_version`).
const VERSION: i32 = 2;

const SCHEMA_V1: &str = "
CREATE TABLE regles (
    id INTEGER PRIMARY KEY,
    dossier TEXT NOT NULL UNIQUE,
    cible TEXT NOT NULL,
    album_id TEXT,
    nom_album TEXT,
    actif INTEGER NOT NULL DEFAULT 1,
    cree_le INTEGER NOT NULL
);
CREATE TABLE fichiers (
    regle INTEGER NOT NULL REFERENCES regles(id) ON DELETE CASCADE,
    chemin TEXT NOT NULL,
    taille INTEGER NOT NULL,
    modifie_ms INTEGER NOT NULL,
    sha1 TEXT,
    etat TEXT NOT NULL,
    asset_id TEXT,
    album_fait INTEGER NOT NULL DEFAULT 0,
    essais INTEGER NOT NULL DEFAULT 0,
    prochain_essai INTEGER NOT NULL DEFAULT 0,
    erreur TEXT,
    maj_le INTEGER NOT NULL,
    PRIMARY KEY (regle, chemin)
);
CREATE INDEX fichiers_etat ON fichiers (regle, etat);
CREATE TABLE albums_dossiers (
    regle INTEGER NOT NULL REFERENCES regles(id) ON DELETE CASCADE,
    dossier TEXT NOT NULL,
    album_id TEXT NOT NULL,
    PRIMARY KEY (regle, dossier)
);
CREATE TABLE journal (
    id INTEGER PRIMARY KEY,
    horodatage INTEGER NOT NULL,
    regle INTEGER,
    chemin TEXT,
    action TEXT NOT NULL,
    detail TEXT,
    asset_id TEXT
);
";

/// Version 2: rules get a direction, download rules their selection and
/// layout, and received files their own table.
const SCHEMA_V2: &str = "
ALTER TABLE regles ADD COLUMN sens TEXT NOT NULL DEFAULT 'montante';
ALTER TABLE regles ADD COLUMN selection TEXT;
ALTER TABLE regles ADD COLUMN rangement TEXT NOT NULL DEFAULT 'plat';
CREATE TABLE recus (
    regle INTEGER NOT NULL REFERENCES regles(id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL,
    chemin TEXT NOT NULL,
    sha1 TEXT NOT NULL,
    taille INTEGER NOT NULL DEFAULT 0,
    modifie_le TEXT NOT NULL DEFAULT '',
    etat TEXT NOT NULL,
    essais INTEGER NOT NULL DEFAULT 0,
    prochain_essai INTEGER NOT NULL DEFAULT 0,
    erreur TEXT,
    maj_le INTEGER NOT NULL,
    PRIMARY KEY (regle, asset_id)
);
CREATE UNIQUE INDEX recus_chemin ON recus (regle, chemin);
";

/// Which way a rule works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sens {
    /// Local folder sent to the server.
    Montante,
    /// A selection of the server copied into the local folder.
    Descendante,
    /// Folder and album kept alike both ways.
    Double,
}

impl Sens {
    fn code(self) -> &'static str {
        match self {
            Sens::Montante => "montante",
            Sens::Descendante => "descendante",
            Sens::Double => "double",
        }
    }
}

/// How received files are laid out in the folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Rangement {
    /// All in the folder itself.
    Plat,
    /// `2026/05/IMG_0001.jpg`, from the capture date.
    AnneeMois,
}

/// State of a received file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EtatRecu {
    /// Path reserved, to download.
    Attente,
    Recu,
    Echec,
    /// Deleted here by the user: not downloaded again.
    RetireIci,
    /// No longer in the selection on the server: the local copy stays.
    RetireServeur,
}

impl EtatRecu {
    fn code(self) -> &'static str {
        match self {
            Self::Attente => "attente",
            Self::Recu => "recu",
            Self::Echec => "echec",
            Self::RetireIci => "retire_ici",
            Self::RetireServeur => "retire_serveur",
        }
    }

    fn depuis(code: &str) -> Self {
        match code {
            "recu" => Self::Recu,
            "echec" => Self::Echec,
            "retire_ici" => Self::RetireIci,
            "retire_serveur" => Self::RetireServeur,
            _ => Self::Attente,
        }
    }
}

/// A received (or reserved) file.
#[derive(Debug, Clone)]
pub struct Recu {
    pub chemin: String,
    pub sha1: String,
    pub etat: EtatRecu,
}

/// A file to download.
#[derive(Debug, Clone)]
pub struct ARecevoir {
    pub asset_id: String,
    pub chemin: String,
    pub sha1: String,
    pub taille: u64,
    pub modifie_le: String,
    pub essais: u32,
}

/// Where the photos of a rule go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(tag = "genre", rename_all = "camelCase")]
pub enum Cible {
    /// The library only.
    Bibliotheque,
    /// One album, identified by its id (never by its name).
    Album { id: String, nom: String },
    /// One album per folder, named after the folder holding the file.
    AlbumParDossier,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Regle {
    pub id: i64,
    pub dossier: String,
    pub sens: Sens,
    /// Where sent photos go (upload and two-way rules).
    pub cible: Cible,
    /// What is received (download rules).
    pub selection: Option<api_immich::Selection>,
    pub rangement: Rangement,
    pub actif: bool,
    pub cree_le: i64,
}

/// State of a file in the queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EtatFichier {
    /// Hashed, to look up on the server.
    AVerifier,
    /// Unknown to the server: to send.
    Attente,
    /// Sent by Nicéphore.
    Envoye,
    /// Already on the server (sent before, by another app or device).
    Present,
    /// Gave up after several attempts, or refused for good.
    Echec,
    /// Not sent on purpose (format refused by the server...).
    Ignore,
}

impl EtatFichier {
    fn code(self) -> &'static str {
        match self {
            Self::AVerifier => "a_verifier",
            Self::Attente => "attente",
            Self::Envoye => "envoye",
            Self::Present => "present",
            Self::Echec => "echec",
            Self::Ignore => "ignore",
        }
    }

    fn depuis(code: &str) -> Self {
        match code {
            "attente" => Self::Attente,
            "envoye" => Self::Envoye,
            "present" => Self::Present,
            "echec" => Self::Echec,
            "ignore" => Self::Ignore,
            _ => Self::AVerifier,
        }
    }
}

/// What the database knows of a local file.
#[derive(Debug, Clone)]
pub struct FichierConnu {
    pub taille: u64,
    pub modifie_ms: i64,
    pub etat: EtatFichier,
}

/// A file waiting to be sent.
#[derive(Debug, Clone)]
pub struct AEnvoyer {
    pub chemin: String,
    pub taille: u64,
    pub modifie_ms: i64,
    pub sha1: String,
    pub essais: u32,
}

/// Counts per state for one rule.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bilan {
    pub envoyes: u64,
    pub presents: u64,
    pub en_attente: u64,
    pub echecs: u64,
    pub ignores: u64,
    pub recus: u64,
    pub a_recevoir: u64,
    pub retires_ici: u64,
    pub retires_serveur: u64,
    pub echecs_reception: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntreeJournal {
    pub id: i64,
    pub horodatage: i64,
    pub regle: Option<i64>,
    pub chemin: Option<String>,
    pub action: String,
    pub detail: Option<String>,
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FichierEnEchec {
    pub chemin: String,
    pub erreur: Option<String>,
    pub essais: u32,
}

pub struct Base {
    cnx: Connection,
}

impl Base {
    /// Opens (or creates) the database, migrating it if needed.
    pub fn ouvrir(chemin: &Path) -> Resultat<Self> {
        if let Some(parent) = chemin.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Erreur::Base(format!("{}: {e}", parent.display())))?;
        }
        let cnx = Connection::open(chemin)?;
        cnx.pragma_update(None, "journal_mode", "WAL")?;
        cnx.pragma_update(None, "foreign_keys", true)?;
        cnx.busy_timeout(std::time::Duration::from_secs(5))?;
        let version: i32 = cnx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > VERSION {
            return Err(Erreur::Base(format!(
                "base créée par une version plus récente de Nicéphore (schéma {version}) : rien n'est modifié"
            )));
        }
        if version > 0 && version < VERSION {
            sauvegarder(&cnx, chemin, version)?;
        }
        let base = Self { cnx };
        base.migrer(version)?;
        Ok(base)
    }

    /// In-memory database, for tests.
    pub fn en_memoire() -> Resultat<Self> {
        let cnx = Connection::open_in_memory()?;
        cnx.pragma_update(None, "foreign_keys", true)?;
        let base = Self { cnx };
        base.migrer(0)?;
        Ok(base)
    }

    fn migrer(&self, depuis: i32) -> Resultat<()> {
        if depuis < 1 {
            self.cnx.execute_batch(&format!("BEGIN; {SCHEMA_V1} PRAGMA user_version = 1; COMMIT;"))?;
        }
        if depuis < 2 {
            self.cnx.execute_batch(&format!("BEGIN; {SCHEMA_V2} PRAGMA user_version = 2; COMMIT;"))?;
        }
        Ok(())
    }

    // ---- Rules ----

    pub fn regles(&self) -> Resultat<Vec<Regle>> {
        let mut requete = self
            .cnx
            .prepare("SELECT id, dossier, cible, album_id, nom_album, actif, cree_le, sens, selection, rangement FROM regles ORDER BY dossier")?;
        let lignes = requete.query_map([], |r| {
            let cible = match r.get::<_, String>(2)?.as_str() {
                "album" => Cible::Album {
                    id: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                    nom: r.get::<_, Option<String>>(4)?.unwrap_or_default(),
                },
                "album_dossier" => Cible::AlbumParDossier,
                _ => Cible::Bibliotheque,
            };
            let sens = match r.get::<_, String>(7)?.as_str() {
                "descendante" => Sens::Descendante,
                "double" => Sens::Double,
                _ => Sens::Montante,
            };
            let selection = r.get::<_, Option<String>>(8)?.and_then(|t| serde_json::from_str(&t).ok());
            let rangement = if r.get::<_, String>(9)? == "annee_mois" { Rangement::AnneeMois } else { Rangement::Plat };
            Ok(Regle { id: r.get(0)?, dossier: r.get(1)?, sens, cible, selection, rangement, actif: r.get(5)?, cree_le: r.get(6)? })
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    pub fn regle(&self, id: i64) -> Resultat<Regle> {
        self.regles()?.into_iter().find(|r| r.id == id).ok_or(Erreur::RegleInconnue)
    }

    /// Adds an upload rule. A folder already covered by a rule (itself, a
    /// parent or a child) is refused: a file must belong to one rule only.
    pub fn ajouter_regle(&self, dossier: &Path, cible: &Cible) -> Resultat<Regle> {
        self.ajouter(dossier, Sens::Montante, cible, None, Rangement::Plat)
    }

    /// Adds a download rule: `selection` copied into `dossier`.
    pub fn ajouter_reception(&self, dossier: &Path, selection: &api_immich::Selection, rangement: Rangement) -> Resultat<Regle> {
        self.ajouter(dossier, Sens::Descendante, &Cible::Bibliotheque, Some(selection), rangement)
    }

    /// Adds a two-way rule between a folder and an album.
    pub fn ajouter_double(&self, dossier: &Path, album_id: &str, nom_album: &str) -> Resultat<Regle> {
        let cible = Cible::Album { id: album_id.to_string(), nom: nom_album.to_string() };
        let selection = api_immich::Selection::Album { id: album_id.to_string(), nom: nom_album.to_string() };
        self.ajouter(dossier, Sens::Double, &cible, Some(&selection), Rangement::Plat)
    }

    fn ajouter(
        &self,
        dossier: &Path,
        sens: Sens,
        cible: &Cible,
        selection: Option<&api_immich::Selection>,
        rangement: Rangement,
    ) -> Resultat<Regle> {
        let dossier = dossier.to_string_lossy().trim_end_matches(['/', '\\']).to_string();
        if dossier.is_empty() {
            return Err(Erreur::Regle("dossier vide".into()));
        }
        for existante in self.regles()? {
            let a = Path::new(&existante.dossier);
            let b = Path::new(&dossier);
            if a.starts_with(b) || b.starts_with(a) {
                return Err(Erreur::Regle(format!("ce dossier recoupe une règle existante ({})", existante.dossier)));
            }
        }
        let (code, album_id, nom_album) = match cible {
            Cible::Bibliotheque => ("bibliotheque", None, None),
            Cible::Album { id, nom } => {
                api_immich::valider_id(id).map_err(|e| Erreur::Regle(e.to_string()))?;
                ("album", Some(id.clone()), Some(nom.clone()))
            }
            Cible::AlbumParDossier => ("album_dossier", None, None),
        };
        let selection = selection.map(|s| serde_json::to_string(s).expect("sélection sérialisable"));
        let rangement = match rangement {
            Rangement::Plat => "plat",
            Rangement::AnneeMois => "annee_mois",
        };
        self.cnx.execute(
            "INSERT INTO regles (dossier, cible, album_id, nom_album, actif, cree_le, sens, selection, rangement)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8)",
            params![dossier, code, album_id, nom_album, maintenant_ms(), sens.code(), selection, rangement],
        )?;
        let id = self.cnx.last_insert_rowid();
        self.journaliser(Some(id), None, "regle_ajoutee", Some(&dossier), None)?;
        self.regle(id)
    }

    pub fn activer_regle(&self, id: i64, actif: bool) -> Resultat<()> {
        let n = self.cnx.execute("UPDATE regles SET actif = ?2 WHERE id = ?1", params![id, actif])?;
        if n == 0 { Err(Erreur::RegleInconnue) } else { Ok(()) }
    }

    /// Forgets a rule and what it knew. Neither local files nor assets on
    /// the server are touched.
    pub fn supprimer_regle(&self, id: i64) -> Resultat<()> {
        let regle = self.regle(id)?;
        self.cnx.execute("DELETE FROM regles WHERE id = ?1", params![id])?;
        self.journaliser(None, None, "regle_supprimee", Some(&regle.dossier), None)
    }

    // ---- Files ----

    pub fn fichiers_connus(&self, regle: i64) -> Resultat<HashMap<String, FichierConnu>> {
        let mut requete = self.cnx.prepare("SELECT chemin, taille, modifie_ms, etat FROM fichiers WHERE regle = ?1")?;
        let lignes = requete.query_map(params![regle], |r| {
            Ok((
                r.get::<_, String>(0)?,
                FichierConnu {
                    taille: r.get::<_, i64>(1)? as u64,
                    modifie_ms: r.get(2)?,
                    etat: EtatFichier::depuis(&r.get::<_, String>(3)?),
                },
            ))
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    /// Records a new or changed file with its SHA-1: it will be looked up
    /// on the server.
    pub fn noter_empreinte(&self, regle: i64, chemin: &str, taille: u64, modifie_ms: i64, sha1: &str) -> Resultat<()> {
        self.cnx.execute(
            "INSERT INTO fichiers (regle, chemin, taille, modifie_ms, sha1, etat, maj_le)
             VALUES (?1, ?2, ?3, ?4, ?5, 'a_verifier', ?6)
             ON CONFLICT (regle, chemin) DO UPDATE SET taille = ?3, modifie_ms = ?4, sha1 = ?5,
                 etat = 'a_verifier', asset_id = NULL, album_fait = 0, essais = 0, prochain_essai = 0,
                 erreur = NULL, maj_le = ?6",
            params![regle, chemin, taille as i64, modifie_ms, sha1, maintenant_ms()],
        )?;
        Ok(())
    }

    /// Files to look up on the server: (path, SHA-1).
    pub fn a_verifier(&self, regle: i64) -> Resultat<Vec<(String, String)>> {
        let mut requete =
            self.cnx.prepare("SELECT chemin, sha1 FROM fichiers WHERE regle = ?1 AND etat = 'a_verifier' AND sha1 IS NOT NULL")?;
        let lignes = requete.query_map(params![regle], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    fn changer_etat(&self, regle: i64, chemin: &str, etat: EtatFichier, asset_id: Option<&str>) -> Resultat<()> {
        self.cnx.execute(
            "UPDATE fichiers SET etat = ?3, asset_id = COALESCE(?4, asset_id), erreur = NULL, maj_le = ?5
             WHERE regle = ?1 AND chemin = ?2",
            params![regle, chemin, etat.code(), asset_id, maintenant_ms()],
        )?;
        Ok(())
    }

    pub fn marquer_attente(&self, regle: i64, chemin: &str) -> Resultat<()> {
        self.changer_etat(regle, chemin, EtatFichier::Attente, None)
    }

    pub fn marquer_present(&self, regle: i64, chemin: &str, asset_id: &str) -> Resultat<()> {
        self.changer_etat(regle, chemin, EtatFichier::Present, Some(asset_id))
    }

    pub fn marquer_envoye(&self, regle: i64, chemin: &str, asset_id: &str) -> Resultat<()> {
        self.changer_etat(regle, chemin, EtatFichier::Envoye, Some(asset_id))
    }

    pub fn marquer_ignore(&self, regle: i64, chemin: &str, raison: &str) -> Resultat<()> {
        self.cnx.execute(
            "UPDATE fichiers SET etat = 'ignore', erreur = ?3, maj_le = ?4 WHERE regle = ?1 AND chemin = ?2",
            params![regle, chemin, raison, maintenant_ms()],
        )?;
        Ok(())
    }

    /// Records a failed attempt: retried at `prochain_essai`, or given up
    /// when `definitif`.
    pub fn marquer_essai_rate(
        &self,
        regle: i64,
        chemin: &str,
        erreur: &str,
        prochain_essai: i64,
        definitif: bool,
    ) -> Resultat<()> {
        self.cnx.execute(
            "UPDATE fichiers SET essais = essais + 1, erreur = ?3, prochain_essai = ?4,
                 etat = CASE WHEN ?5 THEN 'echec' ELSE etat END, maj_le = ?6
             WHERE regle = ?1 AND chemin = ?2",
            params![regle, chemin, erreur, prochain_essai, definitif, maintenant_ms()],
        )?;
        Ok(())
    }

    /// Files due for sending now, oldest attempts first.
    pub fn a_envoyer(&self, regle: i64, maintenant: i64) -> Resultat<Vec<AEnvoyer>> {
        let mut requete = self.cnx.prepare(
            "SELECT chemin, taille, modifie_ms, sha1, essais FROM fichiers
             WHERE regle = ?1 AND etat = 'attente' AND prochain_essai <= ?2 ORDER BY essais, chemin",
        )?;
        let lignes = requete.query_map(params![regle, maintenant], |r| {
            Ok(AEnvoyer {
                chemin: r.get(0)?,
                taille: r.get::<_, i64>(1)? as u64,
                modifie_ms: r.get(2)?,
                sha1: r.get(3)?,
                essais: r.get(4)?,
            })
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    /// Assets of the rule not yet added to their album: (path, asset id).
    pub fn sans_album(&self, regle: i64) -> Resultat<Vec<(String, String)>> {
        let mut requete = self.cnx.prepare(
            "SELECT chemin, asset_id FROM fichiers
             WHERE regle = ?1 AND etat IN ('envoye', 'present') AND album_fait = 0 AND asset_id IS NOT NULL",
        )?;
        let lignes = requete.query_map(params![regle], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    pub fn marquer_album_fait(&self, regle: i64, chemins: &[String]) -> Resultat<()> {
        let mut requete = self.cnx.prepare("UPDATE fichiers SET album_fait = 1 WHERE regle = ?1 AND chemin = ?2")?;
        for chemin in chemins {
            requete.execute(params![regle, chemin])?;
        }
        Ok(())
    }

    /// Earliest retry still in the future, over active rules.
    pub fn prochain_essai(&self, maintenant: i64) -> Resultat<Option<i64>> {
        Ok(self.cnx.query_row(
            "SELECT MIN(f.prochain_essai) FROM fichiers f JOIN regles r ON r.id = f.regle
             WHERE r.actif = 1 AND f.etat = 'attente' AND f.prochain_essai > ?1",
            params![maintenant],
            |r| r.get(0),
        )?)
    }

    /// Puts the given-up files of a rule back in the queue.
    pub fn reessayer(&self, regle: i64) -> Resultat<usize> {
        Ok(self.cnx.execute(
            "UPDATE fichiers SET etat = 'attente', essais = 0, prochain_essai = 0, erreur = NULL
             WHERE regle = ?1 AND etat = 'echec'",
            params![regle],
        )?)
    }

    pub fn echecs(&self, regle: i64) -> Resultat<Vec<FichierEnEchec>> {
        let mut requete = self.cnx.prepare(
            "SELECT chemin, erreur, essais FROM fichiers WHERE regle = ?1 AND etat IN ('echec', 'ignore') ORDER BY chemin",
        )?;
        let lignes = requete
            .query_map(params![regle], |r| Ok(FichierEnEchec { chemin: r.get(0)?, erreur: r.get(1)?, essais: r.get(2)? }))?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    pub fn bilans(&self) -> Resultat<HashMap<i64, Bilan>> {
        let mut requete = self.cnx.prepare("SELECT regle, etat, COUNT(*) FROM fichiers GROUP BY regle, etat")?;
        let mut bilans: HashMap<i64, Bilan> = HashMap::new();
        let lignes = requete.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)? as u64)))?;
        for ligne in lignes {
            let (regle, etat, n) = ligne?;
            let bilan = bilans.entry(regle).or_default();
            match EtatFichier::depuis(&etat) {
                EtatFichier::Envoye => bilan.envoyes += n,
                EtatFichier::Present => bilan.presents += n,
                EtatFichier::AVerifier | EtatFichier::Attente => bilan.en_attente += n,
                EtatFichier::Echec => bilan.echecs += n,
                EtatFichier::Ignore => bilan.ignores += n,
            }
        }
        let mut requete = self.cnx.prepare("SELECT regle, etat, COUNT(*) FROM recus GROUP BY regle, etat")?;
        let lignes = requete.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)? as u64)))?;
        for ligne in lignes {
            let (regle, etat, n) = ligne?;
            let bilan = bilans.entry(regle).or_default();
            match EtatRecu::depuis(&etat) {
                EtatRecu::Recu => bilan.recus += n,
                EtatRecu::Attente => bilan.a_recevoir += n,
                EtatRecu::RetireIci => bilan.retires_ici += n,
                EtatRecu::RetireServeur => bilan.retires_serveur += n,
                EtatRecu::Echec => bilan.echecs_reception += n,
            }
        }
        Ok(bilans)
    }

    // ---- Received files ----

    pub fn recus(&self, regle: i64) -> Resultat<HashMap<String, Recu>> {
        let mut requete = self.cnx.prepare("SELECT asset_id, chemin, sha1, etat FROM recus WHERE regle = ?1")?;
        let lignes = requete.query_map(params![regle], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Recu { chemin: r.get(1)?, sha1: r.get(2)?, etat: EtatRecu::depuis(&r.get::<_, String>(3)?) },
            ))
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    /// Reserves a local path for an asset to download.
    pub fn reserver_recu(&self, regle: i64, asset: &api_immich::AssetDistant, chemin: &str) -> Resultat<()> {
        self.cnx.execute(
            "INSERT OR REPLACE INTO recus (regle, asset_id, chemin, sha1, taille, modifie_le, etat, maj_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'attente', ?7)",
            params![regle, asset.id, chemin, asset.sha1, asset.taille.unwrap_or(0) as i64, asset.modifie_le, maintenant_ms()],
        )?;
        Ok(())
    }

    /// Records a file already present locally with the right content.
    pub fn noter_recu_present(&self, regle: i64, asset: &api_immich::AssetDistant, chemin: &str) -> Resultat<()> {
        self.reserver_recu(regle, asset, chemin)?;
        self.changer_etat_recu(regle, &asset.id, EtatRecu::Recu)
    }

    pub fn a_recevoir(&self, regle: i64, maintenant: i64) -> Resultat<Vec<ARecevoir>> {
        let mut requete = self.cnx.prepare(
            "SELECT asset_id, chemin, sha1, taille, modifie_le, essais FROM recus
             WHERE regle = ?1 AND etat = 'attente' AND prochain_essai <= ?2 ORDER BY essais, chemin",
        )?;
        let lignes = requete.query_map(params![regle, maintenant], |r| {
            Ok(ARecevoir {
                asset_id: r.get(0)?,
                chemin: r.get(1)?,
                sha1: r.get(2)?,
                taille: r.get::<_, i64>(3)? as u64,
                modifie_le: r.get(4)?,
                essais: r.get(5)?,
            })
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }

    pub fn changer_etat_recu(&self, regle: i64, asset_id: &str, etat: EtatRecu) -> Resultat<()> {
        self.cnx.execute(
            "UPDATE recus SET etat = ?3, erreur = NULL, maj_le = ?4 WHERE regle = ?1 AND asset_id = ?2",
            params![regle, asset_id, etat.code(), maintenant_ms()],
        )?;
        Ok(())
    }

    /// A path reserved for download is taken by another file: use another.
    pub fn deplacer_recu(&self, regle: i64, asset_id: &str, chemin: &str) -> Resultat<()> {
        self.cnx.execute("UPDATE recus SET chemin = ?3 WHERE regle = ?1 AND asset_id = ?2", params![regle, asset_id, chemin])?;
        Ok(())
    }

    /// Forgets a reserved file that was never received (gone from the selection).
    pub fn oublier_recu(&self, regle: i64, asset_id: &str) -> Resultat<()> {
        self.cnx.execute("DELETE FROM recus WHERE regle = ?1 AND asset_id = ?2", params![regle, asset_id])?;
        Ok(())
    }

    pub fn marquer_recu_rate(&self, regle: i64, asset_id: &str, erreur: &str, prochain_essai: i64, definitif: bool) -> Resultat<()> {
        self.cnx.execute(
            "UPDATE recus SET essais = essais + 1, erreur = ?3, prochain_essai = ?4,
                 etat = CASE WHEN ?5 THEN 'echec' ELSE etat END, maj_le = ?6
             WHERE regle = ?1 AND asset_id = ?2",
            params![regle, asset_id, erreur, prochain_essai, definitif, maintenant_ms()],
        )?;
        Ok(())
    }

    /// Downloads again what was deleted here or given up.
    pub fn recevoir_de_nouveau(&self, regle: i64) -> Resultat<usize> {
        Ok(self.cnx.execute(
            "UPDATE recus SET etat = 'attente', essais = 0, prochain_essai = 0, erreur = NULL
             WHERE regle = ?1 AND etat IN ('retire_ici', 'echec')",
            params![regle],
        )?)
    }

    // ---- Folder albums ----

    pub fn album_dossier(&self, regle: i64, dossier: &str) -> Resultat<Option<String>> {
        Ok(self
            .cnx
            .query_row(
                "SELECT album_id FROM albums_dossiers WHERE regle = ?1 AND dossier = ?2",
                params![regle, dossier],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn noter_album_dossier(&self, regle: i64, dossier: &str, album_id: &str) -> Resultat<()> {
        self.cnx.execute(
            "INSERT OR REPLACE INTO albums_dossiers (regle, dossier, album_id) VALUES (?1, ?2, ?3)",
            params![regle, dossier, album_id],
        )?;
        Ok(())
    }

    // ---- Log ----

    pub fn journaliser(
        &self,
        regle: Option<i64>,
        chemin: Option<&str>,
        action: &str,
        detail: Option<&str>,
        asset_id: Option<&str>,
    ) -> Resultat<()> {
        self.cnx.execute(
            "INSERT INTO journal (horodatage, regle, chemin, action, detail, asset_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![maintenant_ms(), regle, chemin, action, detail, asset_id],
        )?;
        Ok(())
    }

    /// Latest entries first.
    pub fn journal(&self, limite: u32) -> Resultat<Vec<EntreeJournal>> {
        let mut requete = self.cnx.prepare(
            "SELECT id, horodatage, regle, chemin, action, detail, asset_id FROM journal ORDER BY id DESC LIMIT ?1",
        )?;
        let lignes = requete.query_map(params![limite], |r| {
            Ok(EntreeJournal {
                id: r.get(0)?,
                horodatage: r.get(1)?,
                regle: r.get(2)?,
                chemin: r.get(3)?,
                action: r.get(4)?,
                detail: r.get(5)?,
                asset_id: r.get(6)?,
            })
        })?;
        Ok(lignes.collect::<Result<_, _>>()?)
    }
}

/// Copies the database before a migration (`envoi.sqlite.v1.sauvegarde`).
fn sauvegarder(cnx: &Connection, chemin: &Path, version: i32) -> Resultat<()> {
    let copie: PathBuf = chemin.with_extension(format!("sqlite.v{version}.sauvegarde"));
    cnx.execute("VACUUM INTO ?1", params![copie.to_string_lossy()])?;
    tracing::info!("base sauvegardée avant migration : {}", copie.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regles_qui_se_recoupent_refusees() {
        let base = Base::en_memoire().unwrap();
        base.ajouter_regle(Path::new("/photos/vacances"), &Cible::Bibliotheque).unwrap();
        assert!(base.ajouter_regle(Path::new("/photos"), &Cible::Bibliotheque).is_err());
        assert!(base.ajouter_regle(Path::new("/photos/vacances/2024"), &Cible::AlbumParDossier).is_err());
        assert!(base.ajouter_regle(Path::new("/photos/vacances-bis"), &Cible::AlbumParDossier).is_ok());
        assert_eq!(base.regles().unwrap().len(), 2);
    }

    #[test]
    fn cycle_de_vie_d_un_fichier() {
        let base = Base::en_memoire().unwrap();
        let r = base.ajouter_regle(Path::new("/p"), &Cible::Bibliotheque).unwrap().id;
        base.noter_empreinte(r, "a.jpg", 10, 1, "aa").unwrap();
        assert_eq!(base.a_verifier(r).unwrap(), vec![("a.jpg".to_string(), "aa".to_string())]);
        base.marquer_attente(r, "a.jpg").unwrap();
        assert_eq!(base.a_envoyer(r, 0).unwrap().len(), 1);
        base.marquer_essai_rate(r, "a.jpg", "réseau", 5_000, false).unwrap();
        assert!(base.a_envoyer(r, 0).unwrap().is_empty(), "attend son prochain essai");
        assert_eq!(base.a_envoyer(r, 5_000).unwrap()[0].essais, 1);
        base.marquer_envoye(r, "a.jpg", "id").unwrap();
        assert_eq!(base.bilans().unwrap()[&r].envoyes, 1);
        assert_eq!(base.sans_album(r).unwrap().len(), 1);
        base.marquer_album_fait(r, &["a.jpg".into()]).unwrap();
        assert!(base.sans_album(r).unwrap().is_empty());
        // A changed file starts over.
        base.noter_empreinte(r, "a.jpg", 11, 2, "bb").unwrap();
        assert_eq!(base.bilans().unwrap()[&r].en_attente, 1);
    }

    #[test]
    fn suppression_d_une_regle_oublie_ses_fichiers() {
        let base = Base::en_memoire().unwrap();
        let r = base.ajouter_regle(Path::new("/p"), &Cible::Bibliotheque).unwrap().id;
        base.noter_empreinte(r, "a.jpg", 10, 1, "aa").unwrap();
        base.supprimer_regle(r).unwrap();
        assert!(base.fichiers_connus(r).unwrap().is_empty());
        assert_eq!(base.journal(10).unwrap()[0].action, "regle_supprimee");
    }

    #[test]
    fn migration_de_v1_avec_sauvegarde() {
        let dossier = tempfile::tempdir().unwrap();
        let chemin = dossier.path().join("envoi.sqlite");
        {
            let cnx = Connection::open(&chemin).unwrap();
            cnx.execute_batch(&format!("{SCHEMA_V1} PRAGMA user_version = 1;")).unwrap();
            cnx.execute("INSERT INTO regles (dossier, cible, actif, cree_le) VALUES ('/p', 'bibliotheque', 1, 0)", []).unwrap();
        }
        let base = Base::ouvrir(&chemin).unwrap();
        let regles = base.regles().unwrap();
        assert_eq!((regles.len(), regles[0].sens), (1, Sens::Montante));
        assert!(dossier.path().join("envoi.sqlite.v1.sauvegarde").exists());
        let r = base
            .ajouter_reception(Path::new("/recu"), &api_immich::Selection::Favoris, Rangement::AnneeMois)
            .unwrap();
        assert_eq!((r.sens, r.rangement, r.selection), (Sens::Descendante, Rangement::AnneeMois, Some(api_immich::Selection::Favoris)));
    }

    #[test]
    fn base_plus_recente_refusee() {
        let dossier = tempfile::tempdir().unwrap();
        let chemin = dossier.path().join("envoi.sqlite");
        drop(Base::ouvrir(&chemin).unwrap());
        Connection::open(&chemin).unwrap().pragma_update(None, "user_version", 99).unwrap();
        assert!(Base::ouvrir(&chemin).is_err());
    }
}
