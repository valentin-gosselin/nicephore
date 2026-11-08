//! Upload engine: watches local folders and sends their photos and videos
//! to Immich, with a persistent queue in SQLite.
//!
//! Safety rules (see docs/ARCHITECTURE.md): nothing is ever deleted, neither
//! locally nor on the server; an unreadable folder is never taken as empty;
//! albums are identified by id, never by name; a file that changes while it
//! is hashed or sent is looked at again on the next pass.

pub mod analyse;
pub mod base;
mod moteur;

pub use base::{Base, Bilan, Cible, EntreeJournal, FichierEnEchec, Regle};
pub use moteur::{EtatEnvoi, Moteur, Phase};

use std::time::{SystemTime, UNIX_EPOCH};

pub type Resultat<T> = Result<T, Erreur>;

#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("base locale : {0}")]
    Base(String),
    #[error("règle inconnue")]
    RegleInconnue,
    #[error("{0}")]
    Regle(String),
    #[error(transparent)]
    Api(#[from] api_immich::Erreur),
    #[error("pas de connexion au serveur")]
    SansConnexion,
}

impl From<rusqlite::Error> for Erreur {
    fn from(e: rusqlite::Error) -> Self {
        Erreur::Base(e.to_string())
    }
}

pub(crate) fn maintenant_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
