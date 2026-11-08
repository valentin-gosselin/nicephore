// Thin wrappers around the Tauri commands exposed by the Rust core.
import { invoke } from '@tauri-apps/api/core';

export type Utilisateur = { id: string; name: string; email: string };
/** Where the connection comes from: development variables, OS keyring, or this session only. */
export type Origine = 'environnement' | 'trousseau' | 'memoire';
export type Connexion = {
  urlServeur: string;
  version: string;
  utilisateur: Utilisateur;
  origine: Origine;
  /** Session opened with email and password (rather than an API key). */
  session: boolean;
};

/** Login methods offered by a server, read before logging in. */
export type ModesConnexion = { passwordLogin: boolean; oauth: boolean; urlServeur: string };

/** Which assets a timeline shows. */
export type Filtre = { visibilite?: 'timeline' | 'archive'; favori?: boolean; corbeille?: boolean; personne?: string };

/** Asset of a search result, reduced to what the grid needs. */
export type AssetResume = {
  id: string;
  ratio: number;
  isImage: boolean;
  isFavorite: boolean;
  thumbhash: string | null;
  duration: number | null;
  fileCreatedAt: string;
  localDateTime: string;
};
export type PageResultats = { assets: AssetResume[]; pageSuivante: number | null; total: number };

/** contenu: smart search on what the image shows; the others match text fields. */
export type ModeRecherche = 'contenu' | 'fichierNom' | 'description' | 'texte';
export type CriteresRecherche = {
  texte?: string;
  mode?: ModeRecherche;
  typeMedia?: 'image' | 'video';
  personnes?: string[];
  ville?: string;
  pays?: string;
  du?: string;
  au?: string;
  favoris?: boolean;
  archive?: boolean;
};
export type Lieu = { ville: string; asset: AssetResume };
export type Personne = { id: string; name: string; isFavorite: boolean; updatedAt: string | null };
export type Mois = { timeBucket: string; count: number };

/** Columnar month description: index `i` of every array describes asset `i`. */
export type DetailMois = {
  id: string[];
  ratio: number[];
  isImage: boolean[];
  isFavorite: boolean[];
  thumbhash: (string | null)[];
  duration: (number | string | null)[];
  fileCreatedAt: string[];
  localOffsetHours: number[];
  livePhotoVideoId: (string | null)[];
  stack?: (string[] | null)[] | null;
};

export const etatConnexion = () => invoke<Connexion | null>('etat_connexion');
export const modesConnexion = (url: string) => invoke<ModesConnexion>('modes_connexion', { url });
export const seConnecterMotDePasse = (url: string, email: string, motDePasse: string, memoriser: boolean) =>
  invoke<Connexion>('se_connecter_mot_de_passe', { url, email, motDePasse, memoriser });
export const seConnecter = (url: string, cle: string, memoriser: boolean) =>
  invoke<Connexion>('se_connecter', { url, cle, memoriser });
export const seDeconnecter = () => invoke<void>('se_deconnecter');
export const listeMois = (filtre: Filtre) => invoke<Mois[]>('liste_mois', { filtre });
export const detailMois = (mois: string, filtre: Filtre) => invoke<DetailMois>('detail_mois', { mois, filtre });
export const definirFavori = (ids: string[], favori: boolean) => invoke<void>('definir_favori', { ids, favori });
export const definirArchive = (ids: string[], archive: boolean) => invoke<void>('definir_archive', { ids, archive });
export const mettreCorbeille = (ids: string[]) => invoke<void>('mettre_corbeille', { ids });
export const restaurerCorbeille = (ids: string[]) => invoke<void>('restaurer_corbeille', { ids });

export type Bilan = { reussis: number; echecs: string[]; dossier: string };
export const telecharger = (ids: string[], dossier: string) => invoke<Bilan>('telecharger', { ids, dossier });

export type OccupationCache = { octets: number; fichiers: number; budget: number };
export type InfosCache = { vignettes: OccupationCache; apercus: OccupationCache; dossier: string };
export const infosCache = () => invoke<InfosCache>('infos_cache');
export const viderCache = () => invoke<void>('vider_cache');

/** Background download of every grid thumbnail. */
export type EtatPrechargement = {
  actif: boolean;
  enCours: boolean;
  termine: boolean;
  total: number;
  presentes: number;
  erreurs: number;
  /** The last finished pass had every thumbnail on disk. */
  complet: boolean;
};
export const etatPrechargement = () => invoke<EtatPrechargement>('etat_prechargement');
export const definirPrechargement = (actif: boolean) => invoke<void>('definir_prechargement', { actif });
export const signalerDefilement = () => invoke<void>('signaler_defilement');
export const rechercher = (criteres: CriteresRecherche, page: number) =>
  invoke<PageResultats>('rechercher', { criteres, page });
export const lieux = () => invoke<Lieu[]>('lieux');
export const personnes = () => invoke<Personne[]>('personnes');
export const nombresPersonnes = (ids: string[]) => invoke<Record<string, number>>('nombres_personnes', { ids });
export const nombresVilles = (villes: string[]) => invoke<Record<string, number>>('nombres_villes', { villes });
export const infosAsset = (id: string) => invoke<Record<string, any>>('infos_asset', { id });
export const urlVideo = (id: string) => invoke<string>('url_video', { id });

export type TailleImage = 'vignette' | 'apercu' | 'pleine' | 'personne';

// Custom protocols are exposed as `http://<scheme>.localhost` on Windows.
const baseImages = navigator.userAgent.includes('Windows')
  ? 'http://vignette.localhost'
  : 'vignette://localhost';

/** URL of a cached image; `version` (the thumbhash) busts the cache after edits. */
export function urlImage(taille: TailleImage, id: string, version: string | null): string {
  return `${baseImages}/${taille}/${id}?v=${encodeURIComponent(version ?? '')}`;
}
