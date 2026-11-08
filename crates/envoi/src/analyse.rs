//! Local side: walking a rule's folder, filtering media files, fingerprints.

use api_immich::TypesMedias;
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A file seen in a rule's folder. `chemin` is relative to the folder, with
/// `/` separators on every system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichierLocal {
    pub chemin: String,
    pub taille: u64,
    pub modifie_ms: i64,
    pub cree_ms: i64,
    /// Latest of every date the system keeps (modification, creation of
    /// this copy, status change): a copy that keeps the original
    /// modification date still shows as recent here.
    pub vu_change_ms: i64,
}

/// Result of a walk: the files, and the folders that could not be read
/// (never taken as "deleted").
#[derive(Debug, Default)]
pub struct Inventaire {
    pub fichiers: Vec<FichierLocal>,
    pub illisibles: Vec<String>,
}

/// A file modified less than this long ago may still be written (copy from
/// a card, download...): it waits for the next pass.
pub const DELAI_STABILITE: Duration = Duration::from_secs(10);

/// Suffixes of files being written by other programs.
const SUFFIXES_TEMPORAIRES: [&str; 6] = [".part", ".tmp", ".crdownload", ".download", ".partial", "~"];

fn en_ms(t: SystemTime) -> i64 {
    t.duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// True for a file or folder name skipped by the walk.
fn ignore(nom: &str) -> bool {
    nom.starts_with('.') || SUFFIXES_TEMPORAIRES.iter().any(|s| nom.to_ascii_lowercase().ends_with(s))
}

/// Walks `racine` (symbolic links not followed) and lists the media files
/// accepted by the server.
pub fn inventorier(racine: &Path, types: &TypesMedias) -> std::io::Result<Inventaire> {
    // The root itself must be readable: otherwise nothing is concluded.
    std::fs::read_dir(racine)?;
    let mut inventaire = Inventaire::default();
    let parcours = walkdir::WalkDir::new(racine)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !ignore(&e.file_name().to_string_lossy()));
    for entree in parcours {
        let entree = match entree {
            Ok(e) => e,
            Err(e) => {
                let chemin = e.path().map(|p| relatif(racine, p)).unwrap_or_default();
                tracing::warn!("illisible : {chemin} ({e})");
                inventaire.illisibles.push(chemin);
                continue;
            }
        };
        if !entree.file_type().is_file() {
            continue;
        }
        let extension = entree.path().extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        if !types.accepte(&extension) {
            continue;
        }
        let Ok(meta) = entree.metadata() else {
            inventaire.illisibles.push(relatif(racine, entree.path()));
            continue;
        };
        let modifie = meta.modified().map(en_ms).unwrap_or(0);
        let naissance = meta.created().map(en_ms).ok().filter(|&c| c > 0);
        // Creation time is not available everywhere: the oldest of both.
        let cree = naissance.map_or(modifie, |c| c.min(modifie));
        inventaire.fichiers.push(FichierLocal {
            chemin: relatif(racine, entree.path()),
            taille: meta.len(),
            modifie_ms: modifie,
            cree_ms: cree,
            vu_change_ms: modifie.max(naissance.unwrap_or(0)).max(changement_ms(&meta)),
        });
    }
    inventaire.fichiers.sort_by(|a, b| a.chemin.cmp(&b.chemin));
    Ok(inventaire)
}

/// Status change time (Unix only: it cannot be set back by a copy).
#[cfg(unix)]
fn changement_ms(meta: &std::fs::Metadata) -> i64 {
    use std::os::unix::fs::MetadataExt;
    meta.ctime() * 1_000 + meta.ctime_nsec() / 1_000_000
}

#[cfg(not(unix))]
fn changement_ms(_meta: &std::fs::Metadata) -> i64 {
    0
}

fn relatif(racine: &Path, chemin: &Path) -> String {
    let relatif = chemin.strip_prefix(racine).unwrap_or(chemin);
    relatif.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/")
}

/// Absolute path of a file of a rule.
pub fn absolu(racine: &Path, chemin: &str) -> PathBuf {
    chemin.split('/').fold(racine.to_path_buf(), |p, morceau| p.join(morceau))
}

/// Folder holding a relative path (`""` at the root).
pub fn dossier_de(chemin: &str) -> &str {
    chemin.rsplit_once('/').map_or("", |(d, _)| d)
}

/// True when the file has not changed for `delai` and is likely complete.
pub fn stable(fichier: &FichierLocal, maintenant_ms: i64, delai: Duration) -> bool {
    maintenant_ms - fichier.vu_change_ms >= delai.as_millis() as i64
}

/// SHA-1 of a file (Immich's own checksum), in hex.
pub fn empreinte(chemin: &Path) -> std::io::Result<String> {
    let mut fichier = std::fs::File::open(chemin)?;
    let mut hacheur = Sha1::new();
    let mut tampon = vec![0u8; 1 << 20];
    loop {
        let n = fichier.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        hacheur.update(&tampon[..n]);
    }
    Ok(hex::encode(hacheur.finalize()))
}

/// RFC 3339 date (UTC) for the upload form.
pub fn date_rfc3339(ms: i64) -> String {
    let t = UNIX_EPOCH + Duration::from_millis(ms.max(0) as u64);
    humantime::format_rfc3339_millis(t).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn types() -> TypesMedias {
        TypesMedias { image: vec![".jpg".into(), ".heic".into()], video: vec![".mp4".into()], sidecar: vec![".xmp".into()] }
    }

    #[test]
    fn inventaire_filtre_et_relativise() {
        let dossier = tempfile::tempdir().unwrap();
        let racine = dossier.path();
        std::fs::create_dir_all(racine.join("Vacances 2024")).unwrap();
        std::fs::create_dir_all(racine.join(".cache")).unwrap();
        for nom in ["a.JPG", "Vacances 2024/b.mp4", "Vacances 2024/c.txt", "d.jpg.part", ".e.jpg", ".cache/f.jpg", "g.xmp"] {
            std::fs::write(racine.join(nom), b"x").unwrap();
        }
        let inventaire = inventorier(racine, &types()).unwrap();
        let chemins: Vec<_> = inventaire.fichiers.iter().map(|f| f.chemin.as_str()).collect();
        assert_eq!(chemins, ["Vacances 2024/b.mp4", "a.JPG"]);
        assert!(inventaire.illisibles.is_empty());
        assert_eq!(absolu(racine, "Vacances 2024/b.mp4"), racine.join("Vacances 2024").join("b.mp4"));
        assert_eq!(dossier_de("Vacances 2024/b.mp4"), "Vacances 2024");
        assert_eq!(dossier_de("a.JPG"), "");
    }

    #[cfg(unix)]
    #[test]
    fn copie_antidatee_vue_recente() {
        let dossier = tempfile::tempdir().unwrap();
        let chemin = dossier.path().join("a.jpg");
        std::fs::write(&chemin, b"x").unwrap();
        let vieux = SystemTime::now() - Duration::from_secs(3_600);
        std::fs::File::options().write(true).open(&chemin).unwrap().set_modified(vieux).unwrap();
        let f = &inventorier(dossier.path(), &types()).unwrap().fichiers[0];
        let maintenant = en_ms(SystemTime::now());
        assert!(maintenant - f.modifie_ms > 3_000_000);
        assert!(!stable(f, maintenant, DELAI_STABILITE));
    }

    #[test]
    fn racine_absente_en_erreur() {
        assert!(inventorier(Path::new("/nexiste/pas/du/tout"), &types()).is_err());
    }

    #[test]
    fn empreinte_sha1() {
        let dossier = tempfile::tempdir().unwrap();
        let chemin = dossier.path().join("a");
        std::fs::write(&chemin, b"abc").unwrap();
        assert_eq!(empreinte(&chemin).unwrap(), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn dates_et_stabilite() {
        assert_eq!(date_rfc3339(0), "1970-01-01T00:00:00.000Z");
        let f = FichierLocal { chemin: "a".into(), taille: 1, modifie_ms: 1_000, cree_ms: 1_000, vu_change_ms: 1_000 };
        assert!(!stable(&f, 5_000, DELAI_STABILITE));
        assert!(stable(&f, 11_000, DELAI_STABILITE));
        // A copy keeping an old modification date is judged on its newest date.
        let copie = FichierLocal { vu_change_ms: 10_000, ..f };
        assert!(!stable(&copie, 11_000, DELAI_STABILITE));
    }
}
