use thiserror::Error;

pub type Resultat<T> = Result<T, Erreur>;

/// Errors returned by the Immich client. Messages are user facing (French
/// for now, they will move to translation files).
#[derive(Debug, Error)]
pub enum Erreur {
    #[error("configuration : {0}")]
    Configuration(String),
    #[error("serveur injoignable : {0}")]
    Reseau(String),
    #[error("la clé API est refusée par le serveur")]
    CleRefusee,
    #[error("droit manquant sur la clé API : {0}")]
    PermissionManquante(String),
    #[error("élément introuvable")]
    Introuvable,
    #[error("erreur du serveur ({statut}) : {message}")]
    Http { statut: u16, message: String },
    #[error("réponse inattendue : {0}")]
    ReponseInattendue(String),
}

impl Erreur {
    /// True when retrying later may succeed (network or server overload).
    pub fn est_passagere(&self) -> bool {
        match self {
            Erreur::Reseau(_) => true,
            Erreur::Http { statut, .. } => matches!(statut, 429 | 500 | 502 | 503 | 504),
            _ => false,
        }
    }
}

impl From<reqwest::Error> for Erreur {
    fn from(e: reqwest::Error) -> Self {
        if e.is_decode() {
            Erreur::ReponseInattendue(e.to_string())
        } else {
            // Strip the URL from the message: it is noise for the user.
            Erreur::Reseau(e.without_url().to_string())
        }
    }
}
