//! The tree shown by a virtual folder, rebuilt from the selection. Inode
//! numbers stay the same across rebuilds (keyed by asset id or folder
//! path), so open files and the kernel's caches stay valid.

use api_immich::AssetDistant;
use std::collections::HashMap;

/// How files are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rangement {
    Plat,
    /// `2026/05/IMG_0001.jpg`.
    AnneeMois,
}

#[derive(Debug, Clone)]
pub enum Genre {
    Dossier { enfants: Vec<u64> },
    Fichier { asset: AssetDistant },
}

#[derive(Debug, Clone)]
pub struct Noeud {
    pub ino: u64,
    pub parent: u64,
    pub nom: String,
    pub genre: Genre,
}

/// Inode numbers handed out so far.
#[derive(Debug)]
pub struct Inodes {
    suivant: u64,
    par_cle: HashMap<String, u64>,
}

impl Default for Inodes {
    fn default() -> Self {
        // 1 is the root.
        Self { suivant: 2, par_cle: HashMap::new() }
    }
}

impl Inodes {
    fn pour(&mut self, cle: String) -> u64 {
        if let Some(&ino) = self.par_cle.get(&cle) {
            return ino;
        }
        let ino = self.suivant;
        self.suivant += 1;
        self.par_cle.insert(cle, ino);
        ino
    }
}

pub const RACINE: u64 = 1;

#[derive(Debug, Clone)]
pub struct Arbre {
    noeuds: HashMap<u64, Noeud>,
}

/// A file name valid on every system.
fn nom_sur(nom: &str, secours: &str) -> String {
    let propre: String = nom.chars().map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c }).collect();
    let propre = propre.trim().trim_end_matches(['.', ' ']).trim_start_matches('.').to_string();
    if propre.is_empty() { secours.to_string() } else { propre }
}

fn sous_dossier(rangement: Rangement, date_locale: &str) -> Option<(String, String)> {
    match rangement {
        Rangement::Plat => None,
        Rangement::AnneeMois => match (date_locale.get(0..4), date_locale.get(5..7)) {
            (Some(a), Some(m)) if a.chars().all(|c| c.is_ascii_digit()) && m.chars().all(|c| c.is_ascii_digit()) => {
                Some((a.to_string(), m.to_string()))
            }
            _ => Some(("sans date".into(), String::new())),
        },
    }
}

impl Arbre {
    pub fn construire(assets: &[AssetDistant], rangement: Rangement, inodes: &mut Inodes) -> Self {
        let mut arbre = Self { noeuds: HashMap::new() };
        arbre.noeuds.insert(
            RACINE,
            Noeud { ino: RACINE, parent: RACINE, nom: String::new(), genre: Genre::Dossier { enfants: Vec::new() } },
        );
        // Names taken per folder, without case (as on Windows).
        let mut pris: HashMap<u64, std::collections::HashSet<String>> = HashMap::new();
        for asset in assets {
            let mut parent = RACINE;
            if let Some((annee, mois)) = sous_dossier(rangement, &asset.date_locale) {
                parent = arbre.dossier(parent, &annee, &format!("d:{annee}"), inodes);
                if !mois.is_empty() {
                    parent = arbre.dossier(parent, &mois, &format!("d:{annee}/{mois}"), inodes);
                }
            }
            let nom = nom_sur(&asset.nom_fichier, &asset.id);
            let (base, extension) = match nom.rsplit_once('.') {
                Some((b, e)) if !b.is_empty() => (b.to_string(), format!(".{e}")),
                _ => (nom.clone(), String::new()),
            };
            let deja = pris.entry(parent).or_default();
            let mut n = 1;
            let libre = loop {
                let candidat = if n == 1 { nom.clone() } else { format!("{base} ({n}){extension}") };
                if deja.insert(candidat.to_lowercase()) {
                    break candidat;
                }
                n += 1;
            };
            let ino = inodes.pour(format!("a:{}", asset.id));
            arbre.noeuds.insert(ino, Noeud { ino, parent, nom: libre, genre: Genre::Fichier { asset: asset.clone() } });
            arbre.ajouter_enfant(parent, ino);
        }
        arbre
    }

    fn dossier(&mut self, parent: u64, nom: &str, cle: &str, inodes: &mut Inodes) -> u64 {
        let ino = inodes.pour(cle.to_string());
        if let std::collections::hash_map::Entry::Vacant(place) = self.noeuds.entry(ino) {
            place.insert(Noeud { ino, parent, nom: nom.to_string(), genre: Genre::Dossier { enfants: Vec::new() } });
            self.ajouter_enfant(parent, ino);
        }
        ino
    }

    fn ajouter_enfant(&mut self, parent: u64, enfant: u64) {
        if let Some(Noeud { genre: Genre::Dossier { enfants }, .. }) = self.noeuds.get_mut(&parent) {
            enfants.push(enfant);
        }
    }

    pub fn noeud(&self, ino: u64) -> Option<&Noeud> {
        self.noeuds.get(&ino)
    }

    pub fn enfant(&self, parent: u64, nom: &str) -> Option<&Noeud> {
        match &self.noeuds.get(&parent)?.genre {
            Genre::Dossier { enfants } => enfants.iter().filter_map(|i| self.noeuds.get(i)).find(|n| n.nom == nom),
            Genre::Fichier { .. } => None,
        }
    }

    pub fn enfants(&self, ino: u64) -> Vec<&Noeud> {
        match self.noeuds.get(&ino).map(|n| &n.genre) {
            Some(Genre::Dossier { enfants }) => enfants.iter().filter_map(|i| self.noeuds.get(i)).collect(),
            _ => Vec::new(),
        }
    }

    pub fn fichiers(&self) -> usize {
        self.noeuds.values().filter(|n| matches!(n.genre, Genre::Fichier { .. })).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(id: &str, nom: &str, date: &str) -> AssetDistant {
        AssetDistant {
            id: id.into(),
            nom_fichier: nom.into(),
            sha1: String::new(),
            taille: Some(10),
            date_locale: date.into(),
            modifie_le: String::new(),
        }
    }

    #[test]
    fn arbre_par_annee_et_mois_avec_homonymes() {
        let assets = [
            asset("a", "IMG.jpg", "2026-05-30T10:00:00"),
            asset("b", "img.JPG", "2026-05-30T11:00:00"),
            asset("c", "x.jpg", "2025-01-02T10:00:00"),
        ];
        let mut inodes = Inodes::default();
        let arbre = Arbre::construire(&assets, Rangement::AnneeMois, &mut inodes);
        let annees: Vec<_> = arbre.enfants(RACINE).iter().map(|n| n.nom.clone()).collect();
        assert_eq!(annees, ["2026", "2025"]);
        let a2026 = arbre.enfant(RACINE, "2026").unwrap().ino;
        let mai = arbre.enfant(a2026, "05").unwrap().ino;
        let noms: Vec<_> = arbre.enfants(mai).iter().map(|n| n.nom.clone()).collect();
        assert_eq!(noms, ["IMG.jpg", "img (2).JPG"]);
        assert_eq!(arbre.fichiers(), 3);
        // Same inodes after a rebuild.
        let ino_b = arbre.enfant(mai, "img (2).JPG").unwrap().ino;
        let encore = Arbre::construire(&assets, Rangement::AnneeMois, &mut inodes);
        assert_eq!(encore.enfant(mai, "img (2).JPG").unwrap().ino, ino_b);
    }
}
